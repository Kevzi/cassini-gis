"""Exécuteur générique des analyses : une commande du noyau, un algorithme du cadre de
traitement de QGIS.

Le noyau décide (AD-22 : ses commandes ont des noms neutres, jamais `native:buffer`) ;
ce module traduit. La table `ALGORITHMS` est fermée (AD-23) : seul un id de commande
présent dans la table peut lancer un algorithme, jamais un nom venu du modèle. Ajouter
une analyse = une ligne de table (+ sa fonction de traduction) ; `ProcessingGesture`
ne change pas.

Le noyau remplit `out` (AD-10) : le geste écrit exactement ce fichier, ne l'invente pas
et n'en dérive aucun autre. Le travail avance en tâche QGIS annulable
(`QgsProcessingAlgRunnerTask`), jamais en bloquant l'interface. L'entrée est lue seule.

Le fond `QgisBackend` est le seul endroit de ce module qui touche `qgis.core`.
"""

import contextlib
import os

try:  # importés à plat par les tests, en paquet par QGIS
    from .gestures import GestureError
    from .inventory import QgisBackend as _ReadBackend
except ImportError:  # pragma: no cover
    from gestures import GestureError
    from inventory import QgisBackend as _ReadBackend

NOT_FOUND = "layer.not_found"
NOT_VECTOR = "layer.not_vector"
CRS_NOT_METRIC = "analysis.crs_not_metric"
BAD_PARAMS = "analysis.bad_params"
UNKNOWN_FIELD = "analysis.unknown_field"
FAILED = "analysis.failed"
CANCELLED = "qgis.cancelled"
ORIGIN_OFF_NETWORK = "analysis.origin_off_network"
NO_PATH = "analysis.no_path"
UNKNOWN_CRS = "data.unknown_crs"
FORMAT_MISMATCH = "data.format_mismatch"
INVALID_SOURCE = "layer.invalid_source"
LAYER_EMPTY = "layer.empty"
LAYOUT_BAD_PARAMS = "layout.bad_params"
LAYOUT_EXPORT_FAILED = "layout.export_failed"
MAX_SOURCES = 50  # borne du schéma de sortie du noyau (AD-8), la même que `layout.export_pdf`

_UNIT_TO_METERS = {"m": 1.0, "km": 1000.0}


def _distance_in_meters(distance):
    """Distance `{value, unit}` en mètres ; le noyau a déjà vérifié qu'elle est positive."""
    try:
        meters = float(distance["value"]) * _UNIT_TO_METERS[distance["unit"]]
    except (KeyError, TypeError, ValueError):
        raise GestureError(BAD_PARAMS, "La distance doit être de la forme {value, unit}, en m ou km.")
    if not meters > 0:
        raise GestureError(BAD_PARAMS, "La distance doit être strictement positive.")
    return meters


def translate_buffer(params, layer):
    """Paramètres neutres de `analysis.buffer` -> paramètres de `native:buffer`."""
    return {
        "INPUT": layer,
        "DISTANCE": _distance_in_meters(params.get("distance")),
        "SEGMENTS": 5,
        "END_CAP_STYLE": 0,
        "JOIN_STYLE": 0,
        "MITER_LIMIT": 2,
        "DISSOLVE": bool(params.get("dissolve")),
    }


# Reprojection fixe qui ouvre la chaîne d'une ligne `work_crs` quand le noyau donne un système
# de calcul, c'est-à-dire pour une couche en degrés seulement (changement de cap du 2026-11-12,
# amendement RA06 d'AD-24) : vers le système que le noyau a choisi, sans autre choix.
_WORK_CRS_ALGORITHM = "native:reprojectlayer"


def translate_work_crs(work_epsg, layer):
    """Couche d'entrée -> paramètres de `native:reprojectlayer` vers EPSG:`work_epsg`, sans la
    sortie (temporaire). `OPERATION` est laissé de côté, comme pour `data.reproject`."""
    return {"INPUT": layer, "TARGET_CRS": "EPSG:%d" % work_epsg, "CONVERT_CURVED_GEOMETRIES": False}


# Paramètre du noyau qui porte le système de calcul (AD-19) : la condition de l'étape de reprojection.
WORK_EPSG = "work_epsg"


def translate_work_reprojection(params, layer):
    """Traduction de l'étape de reprojection vers le système de calcul, lu du paramètre du noyau."""
    return translate_work_crs(params[WORK_EPSG], layer)


def translate_intersect(params, layer):
    """Paramètres neutres de `analysis.intersect` -> paramètres de `native:intersection`.

    La couche de recouvrement est ajoutée par le geste (clé `overlay` de la table).
    """
    return {
        "INPUT": layer,
        "INPUT_FIELDS": [],  # liste vide : tous les champs
        "OVERLAY_FIELDS": [],
        "OVERLAY_FIELDS_PREFIX": "",
    }


def translate_difference(params, layer):
    """Paramètres neutres de `analysis.difference` -> paramètres de `native:difference`.

    La couche à retrancher est ajoutée par le geste (clé `overlay_key` de la table) ;
    `GRID_SIZE` reste à son défaut (aucune grille).
    """
    return {"INPUT": layer}


# Opérateur du noyau -> code de `native:extractbyattribute` (jamais d'expression).
_OPERATOR_CODES = {
    "eq": 0,
    "ne": 1,
    "gt": 2,
    "gte": 3,
    "lt": 4,
    "lte": 5,
    "begins_with": 6,
    "contains": 7,
}


def translate_select_by_attribute(params, layer):
    """Champ, opérateur et valeur -> paramètres séparés de `native:extractbyattribute`.

    Le texte de l'utilisateur n'entre jamais dans une expression : QGIS reçoit le nom du
    champ, le code de l'opérateur et la valeur comme trois paramètres distincts.
    """
    operator = params.get("operator")
    value = params.get("value")
    if operator not in _OPERATOR_CODES or not isinstance(value, str):
        raise GestureError(BAD_PARAMS, "Le filtre doit avoir un opérateur connu et une valeur en texte.")
    return {
        "INPUT": layer,
        "FIELD": params.get("field"),
        "OPERATOR": _OPERATOR_CODES[operator],
        "VALUE": value,
    }


_UNIT_TO_SECONDS = {"s": 1.0, "min": 60.0}
_COST_SECONDS_PER_UNIT = 3600.0  # le coût « plus rapide » de l'algorithme est en heures (vérifié par la sonde)
_DISTANCE_TO_METERS = {"m": 1.0, "km": 1000.0}


def _duration_in_seconds(duration):
    """Durée `{value, unit}` en secondes ; le noyau a déjà vérifié qu'elle est positive."""
    try:
        seconds = float(duration["value"]) * _UNIT_TO_SECONDS[duration["unit"]]
    except (KeyError, TypeError, ValueError):
        raise GestureError(BAD_PARAMS, "La durée doit être de la forme {value, unit}, en s ou min.")
    if not seconds > 0:
        raise GestureError(BAD_PARAMS, "La durée doit être strictement positive.")
    return seconds


def _network_of(params):
    """Contrat de réseau (AD-28) résolu par le noyau : vitesse et tolérance toujours présentes.

    Le noyau écrit les valeurs par défaut dans les paramètres (`NetworkOptions::resolve`) ;
    l'extension n'en connaît aucune, et refuse un plan qui ne les porte pas.
    """
    network = params.get("network")
    if not isinstance(network, dict):
        raise GestureError(BAD_PARAMS, "Le contrat de réseau (vitesse, tolérance) manque : le noyau le résout.")
    return network


def _network_speed(network):
    """Vitesse uniforme du réseau en km/h (repli quand un tronçon n'a pas de vitesse propre)."""
    speed = network.get("speed")
    try:
        value = float(speed["value"])
        unit = speed["unit"]
    except (KeyError, TypeError, ValueError):
        raise GestureError(BAD_PARAMS, "La vitesse doit être de la forme {value, unit}, en km/h.")
    if unit != "km/h" or not value > 0:
        raise GestureError(BAD_PARAMS, "La vitesse doit être strictement positive, en km/h.")
    return value


def _network_tolerance_meters(network):
    """Tolérance d'accrochage du réseau en mètres."""
    tolerance = network.get("tolerance")
    try:
        meters = float(tolerance["value"]) * _DISTANCE_TO_METERS[tolerance["unit"]]
    except (KeyError, TypeError, ValueError):
        raise GestureError(BAD_PARAMS, "La tolérance doit être de la forme {value, unit}, en m ou km.")
    if not meters > 0:
        raise GestureError(BAD_PARAMS, "La tolérance doit être strictement positive.")
    return meters


def translate_isochrone(params, layer, backend):
    """Paramètres neutres de `analysis.isochrone` -> paramètres de `native:serviceareafrompoint`.

    Stratégie « plus rapide », coût en secondes ; l'origine porte son SCR. Quand le noyau donne
    `work_epsg` (réseau en degrés, reprojeté en tête de chaîne), l'origine est transformée par
    QGIS vers ce système : le réseau que lit l'algorithme est en mètres. Sinon QGIS la reprojette
    dans celui du réseau. Vitesse, champ de vitesse et sens viennent du contrat de réseau
    résolu par le noyau (`network`) : l'extension n'a aucune valeur par défaut.
    """
    origin = params.get("origin")
    try:
        x, y, epsg = float(origin["x"]), float(origin["y"]), int(origin["epsg"])
    except (KeyError, TypeError, ValueError):
        raise GestureError(BAD_PARAMS, "L'origine doit être de la forme {x, y, epsg}.")
    if params.get("mode") not in ("walk", "car"):
        raise GestureError(BAD_PARAMS, "Le mode doit être « walk » (à pied) ou « car » (en voiture).")
    network = _network_of(params)
    work_epsg = params.get(WORK_EPSG)
    if work_epsg is not None and work_epsg != epsg:
        x, y = backend.transform_point(x, y, epsg, work_epsg)
        epsg = work_epsg
    direction = network.get("direction")
    algorithm = {
        "INPUT": layer,
        "STRATEGY": 1,  # plus rapide
        "DIRECTION_FIELD": "",
        "DEFAULT_DIRECTION": 2,  # deux sens
        "SPEED_FIELD": network.get("speed_field") or "",
        "DEFAULT_SPEED": _network_speed(network),
        "TOLERANCE": 0,
        "START_POINT": "%r,%r [EPSG:%d]" % (x, y, epsg),
        "TRAVEL_COST2": _duration_in_seconds(params.get("duration")) / _COST_SECONDS_PER_UNIT,
        "INCLUDE_BOUNDS": False,
    }
    if direction:
        try:
            algorithm["DIRECTION_FIELD"] = direction["field"]
            algorithm["VALUE_FORWARD"] = direction["forward"]
            algorithm["VALUE_BACKWARD"] = direction["backward"]
        except (KeyError, TypeError):
            raise GestureError(BAD_PARAMS, "Le sens doit être de la forme {field, forward, backward}.")
    return algorithm


_POINT_LABELS = {"start": "Le départ", "end": "L'arrivée"}


def _point_of(params, key, backend, work_epsg):
    """Point `{x, y, epsg}` du paramètre `key`, en texte de QGIS ; transformé par QGIS vers `work_epsg`
    quand le noyau en donne un (réseau en degrés, reprojeté en tête de chaîne)."""
    point = params.get(key)
    try:
        x, y, epsg = float(point["x"]), float(point["y"]), int(point["epsg"])
    except (KeyError, TypeError, ValueError):
        raise GestureError(BAD_PARAMS, "%s doit être de la forme {x, y, epsg}." % _POINT_LABELS[key])
    if work_epsg is not None and work_epsg != epsg:
        x, y = backend.transform_point(x, y, epsg, work_epsg)
        epsg = work_epsg
    return "%r,%r [EPSG:%d]" % (x, y, epsg)


def translate_shortest_path(params, layer, backend):
    """Paramètres neutres de `network.shortest_path` -> paramètres de `native:shortestpathpointtopoint`.

    Même contrat de réseau que l'isochrone (`network`, résolu par le noyau : l'extension n'a aucune valeur
    par défaut) et même stratégie « plus rapide », qui donne la durée ; la longueur se lit à part, sur la
    géométrie. Départ et arrivée portent leur SCR et sont transformés par QGIS vers `work_epsg` quand le
    noyau en donne un. `TOLERANCE` (topologie du graphe) reste à 0 : l'accrochage des deux points a été
    vérifié avant le lancement."""
    if params.get("mode") not in ("walk", "car"):
        raise GestureError(BAD_PARAMS, "Le mode doit être « walk » (à pied) ou « car » (en voiture).")
    network = _network_of(params)
    work_epsg = params.get(WORK_EPSG)
    algorithm = {
        "INPUT": layer,
        "STRATEGY": 1,  # plus rapide
        "DIRECTION_FIELD": "",
        "DEFAULT_DIRECTION": 2,  # deux sens
        "SPEED_FIELD": network.get("speed_field") or "",
        "DEFAULT_SPEED": _network_speed(network),
        "TOLERANCE": 0,
        "START_POINT": _point_of(params, "start", backend, work_epsg),
        "END_POINT": _point_of(params, "end", backend, work_epsg),
    }
    direction = network.get("direction")
    if direction:
        try:
            algorithm["DIRECTION_FIELD"] = direction["field"]
            algorithm["VALUE_FORWARD"] = direction["forward"]
            algorithm["VALUE_BACKWARD"] = direction["backward"]
        except (KeyError, TypeError):
            raise GestureError(BAD_PARAMS, "Le sens doit être de la forme {field, forward, backward}.")
    return algorithm


def summarize_shortest_path(backend, layer, params, results):
    """Longueur (mètres, lue sur la géométrie dans le système en mètres du calcul) et durée (secondes,
    coût « plus rapide » de l'algorithme, en heures) du chemin ; une seule entité est exigée."""
    if backend.feature_count(layer) != 1:
        raise GestureError(FAILED, "Le calcul du chemin n'a pas rendu exactement une ligne.")
    cost = results.get("TRAVEL_COST") if isinstance(results, dict) else None
    try:
        hours = float(cost)
    except (TypeError, ValueError):
        raise GestureError(FAILED, "Le calcul du chemin n'a pas rendu son coût de parcours.")
    length = backend.total_length(layer)
    if not hours >= 0 or length is None:
        raise GestureError(FAILED, "Le calcul du chemin n'a pas rendu une longueur et une durée lisibles.")
    return {
        "length": {"value": float(length), "unit": "m"},
        "duration": {"value": hours * _COST_SECONDS_PER_UNIT, "unit": "s"},
    }


def translate_prepare_snap(params, layer):
    """Paramètres neutres de `network.prepare` -> `native:snapgeometries`, la couche contre elle-même.

    La tolérance vient du noyau (qui écrit son défaut, AD-28) et se lit en mètres : la couche est en mètres
    (ou reprojetée vers `work_epsg` en tête de chaîne). `BEHAVIOR` 4 : « Move end points only, prefer
    aligning nodes » (constat de la sonde) : seules les extrémités bougent, vers un nœud d'abord, sinon vers
    le segment le plus proche ; les sommets intermédiaires restent où ils sont."""
    return {
        "INPUT": layer,
        "REFERENCE_LAYER": layer,
        "TOLERANCE": _distance_in_meters(params.get("tolerance")),
        "BEHAVIOR": 4,
    }


def translate_prepare_split(params, layer):
    """Étape suivante : `native:splitwithlines` de la couche accrochée sur elle-même (nœud à chaque croisement)."""
    return {"INPUT": layer, "LINES": layer}


def translate_prepare_remove_null(params, layer):
    """Étape suivante : `native:removenullgeometries`, géométries nulles et vides retirées (la sortie des
    écartées, facultative, n'est pas demandée)."""
    return {"INPUT": layer, "REMOVE_EMPTY": True}


def translate_prepare_back(params, layer, backend):
    """Étape de retour : `native:reprojectlayer` vers le système de la couche d'origine, que le noyau n'a pas
    changé (la préparation se calcule dans `work_epsg`, le résultat reste dans le système de la couche)."""
    original = backend.layer(params.get("layer_id"))
    if original is None:
        raise GestureError(NOT_FOUND, "Aucune couche du projet ne porte cet identifiant.")
    return {"INPUT": layer, "TARGET_CRS": backend.crs_of(original), "CONVERT_CURVED_GEOMETRIES": False}


def translate_prepare_dedupe(params, layer):
    """Dernière étape : `native:deleteduplicategeometries` (égalité topologique : un tronçon à l'envers est un
    doublon). Écrit `out`."""
    return {"INPUT": layer}


def translate_convex_hull(params, previous):
    """Étape suivante : enveloppe convexe de la couche temporaire de l'étape précédente."""
    return {"INPUT": previous}


def _epsg_code(value):
    """Code EPSG entier strictement positif ; le noyau l'a déjà vérifié (u32)."""
    if isinstance(value, bool) or not isinstance(value, int) or value <= 0:
        raise GestureError(BAD_PARAMS, "Le système d'arrivée doit être un code EPSG entier positif.")
    return value


def translate_reproject(params, layer):
    """Paramètres neutres de `data.reproject` -> paramètres de `native:reprojectlayer`.

    `OPERATION` est laissé de côté : les réglages de transformation du projet s'appliquent
    (AD-19 : la reprojection est celle de QGIS, jamais du noyau).
    """
    return {
        "INPUT": layer,
        "TARGET_CRS": "EPSG:%d" % _epsg_code(params.get("target_epsg")),
        "CONVERT_CURVED_GEOMETRIES": False,
    }


# Formats de `data.convert` et extension du fichier que le noyau dicte pour chacun (AD-10).
_FORMAT_EXTENSIONS = {"gpkg": ".gpkg", "shp": ".shp", "geojson": ".geojson"}
# Pilote OGR (`storageType` du fournisseur) attendu du fichier relu, pour chaque format.
_FORMAT_DRIVERS = {"gpkg": "GPKG", "shp": "ESRI Shapefile", "geojson": "GeoJSON"}
# Un Shapefile ne porte qu'un seul type de géométrie.
_SINGLE_GEOMETRIES = ("Point", "Line", "Polygon")


def translate_convert(params, layer):
    """Paramètres neutres de `data.convert` -> paramètres de `native:extractbyexpression`.

    Une extraction sur l'expression `TRUE` écrit toutes les entités par le puits de sortie
    du cadre de traitement, qui choisit le pilote d'après l'extension du fichier ; les
    limites du format s'appliquent (Shapefile : noms de champ coupés à 10 caractères, textes
    à 254, dates sans heure). `native:savefeatures` ne convient pas : sous
    QGIS 3.44, il cherche le pilote d'après tout ce qui suit le premier point du nom
    (`convert.gpkg` pour `2-data.convert.gpkg`, le nom que dicte le noyau) et échoue
    (« OGR driver for '' not found »). Le format écrit est celui de l'extension du fichier
    `out`, que le noyau a tirée du `format` : le geste vérifie qu'elles s'accordent, il n'en
    choisit aucune.
    """
    ext = _FORMAT_EXTENSIONS.get(params.get("format"))
    if ext is None:
        raise GestureError(BAD_PARAMS, "Le format doit être gpkg, shp ou geojson.")
    out = params.get("out")
    if not isinstance(out, str) or not out.lower().endswith(ext):
        raise GestureError(BAD_PARAMS, "Le fichier de sortie du noyau ne porte pas l'extension du format.")
    return {"INPUT": layer, "EXPRESSION": "TRUE"}


# Sortie de `data.convert_raster`, fixée par la fiche du catalogue (epic 10, entrée 30) :
# GeoTIFF Float32, sans-donnée -9999, première bande seulement, compressé sans perte (DEFLATE,
# que le lecteur du noyau lit, AD-20). Index de l'énumération `DATA_TYPE` de
# `gdal:warpreproject` sous QGIS 3.44 (vérifié par la sonde) : 6 = Float32.
_RASTER_NODATA = -9999
_RASTER_FLOAT32 = 6
_RASTER_NEAREST = 0
_RASTER_BILINEAR = 1


# Rééchantillonnage choisi par le noyau (paramètre `resampling` de `raster.reproject_work`) -> index
# de l'énumération `RESAMPLING` de `gdal:warpreproject` sous QGIS 3.44.
_RESAMPLING = {"nearest": _RASTER_NEAREST, "bilinear": _RASTER_BILINEAR}


def translate_reproject_work(params, raster):
    """Paramètres neutres de `raster.reproject_work` -> paramètres de `gdal:warpreproject` vers le
    système en mètres que le noyau a choisi (`work_epsg`), ticket 33.

    Geste fixe, aucun choix laissé à l'appelant (AD-5, AD-24) : taille de cellule par défaut de
    gdalwarp (`TARGET_RESOLUTION` vide), emprise de la source, Float32 et sans-donnée -9999 comme
    `data.convert_raster`, première bande seulement. Le rééchantillonnage vient du noyau (`resampling`,
    bilinéaire sans lui) : l'extension ne le devine pas d'après le type de la bande.
    """
    name = params.get("resampling", "bilinear")
    if name not in _RESAMPLING:
        raise GestureError(BAD_PARAMS, "Le rééchantillonnage doit être « nearest » ou « bilinear ».")
    return {
        "INPUT": raster,
        "SOURCE_CRS": None,
        "TARGET_CRS": "EPSG:%d" % _epsg_code(params.get(WORK_EPSG)),
        "RESAMPLING": _RESAMPLING[name],
        "NODATA": _RASTER_NODATA,
        "TARGET_RESOLUTION": None,
        "CREATION_OPTIONS": "COMPRESS=DEFLATE",
        "DATA_TYPE": _RASTER_FLOAT32,
        "TARGET_EXTENT": None,
        "MULTITHREADING": False,
        "EXTRA": "-srcband 1 -dstband 1",
    }


def translate_convert_raster(params, raster):
    """Paramètres neutres de `data.convert_raster` -> paramètres de `gdal:warpreproject`.

    `gdal:warpreproject` et non `gdal:translate` : seul gdalwarp réécrit les cellules
    sans-donnée de la source (le -99999 d'un ASCII Grid de l'IGN) en -9999 ; gdal_translate
    changerait l'étiquette sans toucher les valeurs, et les anciennes cellules vides
    deviendraient des altitudes. Sans `TARGET_CRS`, ni résolution ni emprise imposées, la grille
    de sortie est celle de la source, au plus proche voisin : aucune reprojection (AD-19).
    `SOURCE_CRS` n'est posé que pour déclarer le système d'une source qui n'en a pas ; le geste
    a déjà refusé une source qui en a un autre.
    """
    assign = params.get("assign_epsg")
    return {
        "INPUT": raster,
        "SOURCE_CRS": "EPSG:%d" % _epsg_code(assign) if assign is not None else None,
        "TARGET_CRS": None,
        "RESAMPLING": _RASTER_NEAREST,
        "NODATA": _RASTER_NODATA,
        "TARGET_RESOLUTION": None,
        "CREATION_OPTIONS": "COMPRESS=DEFLATE",
        "DATA_TYPE": _RASTER_FLOAT32,
        "TARGET_EXTENT": None,
        "MULTITHREADING": False,
        "EXTRA": "-srcband 1 -dstband 1",
    }


# Champs écrits par `native:countpointsinpolygon` ; le noyau ne les connaît pas. Avec `WEIGHT`,
# l'algorithme écrit la somme pondérée à la place du décompte dans son champ : le décompte et la
# somme sont donc deux passes, chacune avec son propre champ.
_COUNT_FIELD = "NUMPOINTS"
_WEIGHT_RESULT_FIELD = "SUMWEIGHT"
_MAX_ROWS = 100  # borne du schéma de sortie du noyau (AD-8)
_NUMERIC_MARKERS = ("int", "real", "double", "float", "decimal", "numeric", "long", "short")


def is_numeric_type(type_name):
    """Vrai si le nom de type d'un champ (celui que QGIS rend) désigne un nombre."""
    name = str(type_name).lower()
    return "list" not in name and any(marker in name for marker in _NUMERIC_MARKERS)


def translate_count_points(params, layer):
    """Paramètres neutres de `analysis.count_points_in_polygon` -> `native:countpointsinpolygon`,
    passe du décompte, toujours la dernière (celle qui écrit `out`). La couche de points est ajoutée
    par le geste (clé `overlay_key`)."""
    return {"POLYGONS": layer, "WEIGHT": "", "CLASSFIELD": "", "FIELD": _COUNT_FIELD}


def translate_count_points_weighted(params, layer):
    """Première passe, seulement avec `weight_field` : la somme du champ, dans son propre champ ;
    la passe du décompte, qui suit, ajoute le sien (la dernière étape ne peut pas être conditionnelle)."""
    return {"POLYGONS": layer, "WEIGHT": params["weight_field"], "CLASSFIELD": "", "FIELD": _WEIGHT_RESULT_FIELD}


def summarize_count_points(backend, layer, params):
    """Lignes du résumé des 100 premières zones de la couche produite (clé `summary` de la table) :
    `count` en points, et `weighted_sum` sans unité quand `weight_field` est donné."""
    weight = params.get("weight_field")
    triples, truncated = backend.counts_of(
        layer, _COUNT_FIELD, params.get("label_field") or None, _MAX_ROWS, _WEIGHT_RESULT_FIELD if weight else None
    )
    rows = []
    for label, count, weighted in triples:
        values = {"count": {"value": count, "unit": "points"}}
        if weight:
            values["weighted_sum"] = {"value": weighted, "unit": None}
        rows.append({"label": label, "values": values})
    return {"rows": rows, "truncated": truncated}


# Champs écrits par `native:zonalstatisticsfb` : préfixe de colonne `_` + nom de la statistique (vérifié
# par la sonde). Les trois statistiques demandées : 2 = moyenne, 5 = minimum, 6 = maximum.
_ZONAL_PREFIX = "_"
_ZONAL_STATISTICS = (("mean", 2), ("min", 5), ("max", 6))
_RASTER_SUFFIXES = (".tif", ".tiff")
_UNIT_NAMES = {
    "m": "m", "metre": "m", "metres": "m", "meter": "m", "meters": "m",
    "degree": "°", "degrees": "°", "deg": "°", "°": "°",
    "percent": "%", "%": "%",
}


def normalize_unit(text):
    """Unité d'un raster écrite à la manière de GDAL (`metre`, `degrees`) -> forme du résumé (`m`, `°`) ;
    un nom inconnu est gardé tel quel, vide ou absent : None."""
    if not isinstance(text, str) or not text.strip():
        return None
    name = text.strip()
    return _UNIT_NAMES.get(name.lower(), name)


_pam = {"holders": 0, "previous": None}


def _pam_off():
    """Demande à GDAL de ne pas écrire de `.aux.xml` à côté des rasters qu'il ouvre (`GDAL_PAM_ENABLED=NO`) :
    QGIS calcule les statistiques d'un raster à l'ouverture et GDAL les y consignerait, ce que le fichier
    d'origine ne doit pas recevoir (AD-10). Le réglage se lit à l'ouverture du jeu de données ; il tient
    donc le temps du geste, le dernier à le rendre rétablit la valeur d'avant. Rend True si pris."""
    try:
        from osgeo import gdal
    except ImportError:  # GDAL accompagne toujours QGIS ; sans lui, rien à régler
        return False
    if _pam["holders"] == 0:
        _pam["previous"] = gdal.GetConfigOption("GDAL_PAM_ENABLED")
        _pam["previous_env"] = os.environ.get("GDAL_PAM_ENABLED")
        gdal.SetConfigOption("GDAL_PAM_ENABLED", "NO")
        # Les algorithmes `gdal:*` lancent un programme GDAL à part, qui hérite de l'environnement du
        # processus et non des options de configuration de celui-ci.
        os.environ["GDAL_PAM_ENABLED"] = "NO"
    _pam["holders"] += 1
    return True


def _pam_restore():
    from osgeo import gdal

    _pam["holders"] = max(0, _pam["holders"] - 1)
    if _pam["holders"] == 0:
        gdal.SetConfigOption("GDAL_PAM_ENABLED", _pam["previous"])
        if _pam.get("previous_env") is None:
            os.environ.pop("GDAL_PAM_ENABLED", None)
        else:
            os.environ["GDAL_PAM_ENABLED"] = _pam["previous_env"]


def translate_raster_zonal(params, layer):
    """Paramètres neutres de `raster.zonal_statistics` -> `native:zonalstatisticsfb`, qui écrit une
    nouvelle couche (jamais `qgis:zonalstatistics`, qui écrit dans la couche d'entrée). Le raster est le
    chemin absolu donné par le noyau ; une seule bande, la première."""
    return {
        "INPUT": layer,
        "INPUT_RASTER": params["raster"],
        "RASTER_BAND": 1,
        "COLUMN_PREFIX": _ZONAL_PREFIX,
        "STATISTICS": [code for _, code in _ZONAL_STATISTICS],
    }


# Courbes de niveau (`gdal:contour`) : champ d'altitude écrit, bande unique. L'équidistance est verticale,
# en mètres (le noyau a normalisé `km`). Noms des paramètres de QGIS 3.44 et 4.x : `INPUT`, `BAND`,
# `INTERVAL`, `FIELD_NAME`, `CREATE_3D` ; la sortie est `OUTPUT`.
_CONTOUR_FIELD = "elevation"


def translate_raster_contour(params, raster):
    """Paramètres neutres de `raster.contour` -> paramètres de `gdal:contour`, qui écrit une couche de
    lignes nouvelle (jamais le raster). L'équidistance du noyau est une `Distance` : ramenée en mètres ici
    si elle arrive en kilomètres."""
    interval = params.get("interval")
    try:
        value = float(interval["value"])
        factor = {"m": 1.0, "km": 1000.0}[interval["unit"]]
    except (KeyError, TypeError, ValueError):
        raise GestureError(BAD_PARAMS, "L'équidistance doit être de la forme {value, unit} en m ou km.")
    metres = value * factor
    if not (metres > 0) or metres == float("inf"):
        raise GestureError(BAD_PARAMS, "L'équidistance doit être strictement positive.")
    return {
        "INPUT": raster,
        "BAND": 1,
        "INTERVAL": metres,
        "FIELD_NAME": _CONTOUR_FIELD,
        "CREATE_3D": False,
        "IGNORE_NODATA": False,
        "NODATA": None,
        "OFFSET": 0.0,
        "EXTRA": None,
    }


def summarize_raster_zonal(backend, layer, params):
    """Lignes du résumé des 100 premiers polygones de la couche produite : `mean`, `min` et `max` avec
    l'unité du raster (déclarée par `unit`, sinon lue du fichier, à défaut le mètre) ; un polygone qui ne
    recouvre aucune cellule rend une ligne sans valeur."""
    unit = normalize_unit(params.get("unit")) or normalize_unit(backend.raster_unit(params.get("raster"))) or "m"
    fields = [(name, _ZONAL_PREFIX + name) for name, _ in _ZONAL_STATISTICS]
    pairs, truncated = backend.stats_of(layer, fields, params.get("label_field") or None, _MAX_ROWS)
    rows = []
    for label, stats in pairs:
        values = {name: {"value": value, "unit": unit} for name, value in stats.items() if value is not None}
        rows.append({"label": label, "values": values})
    return {"rows": rows, "truncated": truncated}


def translate_nearest(params, layer):
    """Paramètres neutres de `analysis.nearest` -> paramètres de `native:joinbynearest`.

    Un seul voisin, tous les champs de la cible copiés, aucune entité écartée : une entité sans
    voisin (au-delà de la limite, ou cible vide) reste, avec des valeurs nulles. La limite est en
    mètres, comme le champ de distance : l'algorithme mesure dans le système de la couche d'entrée,
    que la chaîne a mis en mètres. La couche cible est ajoutée par le geste (clé `overlay`).
    """
    limit = params.get("max_distance")
    return {
        "INPUT": layer,
        "FIELDS_TO_COPY": [],  # liste vide : tous les champs de la cible
        "DISCARD_NONMATCHING": False,
        "PREFIX": "",
        "NEIGHBORS": 1,
        "MAX_DISTANCE": _distance_in_meters(limit) if limit is not None else None,
    }


# `native:joinbynearest` ajoute en queue six champs : rang du voisin, distance, puis les quatre
# coordonnées. QGIS les renomme (`distance_2`...) quand un nom existe déjà dans l'entrée ou la
# cible : on les lit donc par leur place, jamais par un nom supposé.
_NEAREST_TAIL = 6


def _nearest_fields(backend, layer):
    """(champ du rang du voisin, champ de la distance) de la couche rendue par la jointure."""
    names = [name for name, _ in backend.fields_of(layer)]
    if len(names) < _NEAREST_TAIL:
        raise GestureError(FAILED, "La jointure au plus proche voisin n'a pas rendu les champs attendus.")
    rank, distance = names[-_NEAREST_TAIL], names[-_NEAREST_TAIL + 1]
    if not (rank.startswith("n") and distance.startswith("distance")):
        raise GestureError(FAILED, "La jointure au plus proche voisin n'a pas rendu les champs attendus.")
    return rank, distance


def translate_nearest_first(params, layer, backend):
    """Garde le premier voisin de chaque entité (`native:extractbyexpression`).

    À égale distance, `native:joinbynearest` rend une ligne par voisin, de rang 1, 2, 3... : on ne
    garde que le rang 1 (déterministe), et les entités sans voisin (rang nul). Il reste ainsi une
    ligne par entité de la couche d'entrée.
    """
    rank, _ = _nearest_fields(backend, layer)
    quoted = '"%s"' % rank.replace('"', '""')
    return {"INPUT": layer, "EXPRESSION": "%s = 1 OR %s IS NULL" % (quoted, quoted)}


def summarize_nearest(backend, layer, params):
    """Champ de distance (lu sur la couche produite), mètres, entités sans voisin (clé `summary`)."""
    _, distance = _nearest_fields(backend, layer)
    return {
        "distance_field": distance,
        "unit": "m",
        "unmatched_count": backend.null_count(layer, distance),
    }


def translate_join_table(params, layer):
    """Paramètres neutres de `analysis.join_table` -> paramètres de `native:joinattributestable`.

    Tous les champs de la table sont copiés (liste vide), `METHOD=1` : une seule correspondance par
    entité, la première de la table (déterministe, `feature_count` inchangé) ; aucune entité écartée :
    une entité sans ligne reste, colonnes vides. Un nom de colonne déjà pris est renommé par QGIS
    (`_2`), la clé de la table est recopiée aussi (sonde `probe_join_table.py`). La table est la
    seconde couche, ajoutée par le geste (clé `overlay`) ; ses champs sont des textes, car `layer.add`
    l'ouvre sans détection de types.
    """
    return {
        "INPUT": layer,
        "FIELD": params["field"],
        "FIELD_2": params["table_field"],
        "FIELDS_TO_COPY": [],  # liste vide : tous les champs de la table
        "METHOD": 1,  # première correspondance
        "DISCARD_NONMATCHING": False,
        "PREFIX": "",
    }


def translate_dissolve(params, layer):
    """Paramètres neutres de `analysis.dissolve` -> paramètres de `native:dissolve`.

    Les champs de regroupement sont ceux du noyau, dans l'ordre. `SEPARATE_DISJOINT=False` : une
    entité par groupe, même si ses morceaux sont disjoints (multipartie). Aucun champ n'est agrégé :
    ce que QGIS garde des autres attributs est constaté par `probe_dissolve.py`.
    """
    return {
        "INPUT": layer,
        "FIELD": list(params["fields"]),
        "SEPARATE_DISJOINT": False,
    }


_UNJOINED_COUNT = "UNJOINABLE_COUNT"  # sortie de `native:joinattributestable` : entités sans ligne


def summarize_join_table(backend, layer, params, results):
    """Nombre d'entités sans ligne correspondante, lu sur la sortie de l'algorithme (clé `summary`)."""
    count = results.get(_UNJOINED_COUNT) if isinstance(results, dict) else None
    if isinstance(count, bool) or not isinstance(count, int) or count < 0:
        raise GestureError(FAILED, "La jointure n'a pas rendu le nombre d'entités sans correspondance.")
    return {"unmatched_count": count}


# Champ temporaire qui porte le champ à sommer, nuls ramenés à 0 : `qgis:statisticsbycategories` ignore
# les entités dont la valeur est nulle, ce qui fausserait le décompte. Il n'existe que dans la chaîne.
_SUM_PREPARED_FIELD = "cassini_sum"
_STATS_COUNT_COLUMN = 1  # table de l'algorithme : `cat,count,unique,min,max,range,sum,...`
_STATS_SUM_COLUMN = 6


def translate_sum_prepared(params, layer):
    """Première étape, seulement avec `sum_field` : `native:fieldcalculator` copie le champ à sommer
    dans `cassini_sum`, nuls ramenés à 0 ; le nom du champ est cité, jamais mis en expression brute."""
    quoted = '"%s"' % params["sum_field"].replace('"', '""')
    return {
        "INPUT": layer,
        "FIELD_NAME": _SUM_PREPARED_FIELD,
        "FIELD_TYPE": 0,  # décimal
        "FIELD_LENGTH": 0,
        "FIELD_PRECISION": 0,
        "FORMULA": "coalesce(%s, 0)" % quoted,
    }


def translate_statistics_by_category(params, layer):
    """Paramètres neutres de `analysis.statistics_by_category` -> `qgis:statisticsbycategories`.
    Sans `sum_field`, la table ne porte que le décompte ; avec lui, les statistiques du champ préparé."""
    parameters = {"INPUT": layer, "CATEGORIES_FIELD_NAME": [params["category_field"]]}
    if params.get("sum_field"):
        parameters["VALUES_FIELD_NAME"] = _SUM_PREPARED_FIELD
    return parameters


def summarize_statistics_by_category(backend, out, params):
    """Résultat de la table écrite (clé `table` de la table) : `file` et les lignes des 100 premières
    catégories, `count` en entités et `sum` sans unité quand `sum_field` est donné. Les colonnes se
    lisent par leur place, jamais par un nom : la catégorie porte le nom du champ de l'utilisateur."""
    summed = bool(params.get("sum_field"))
    header, rows, total = backend.table_rows(out, _MAX_ROWS)
    need = (_STATS_SUM_COLUMN if summed else _STATS_COUNT_COLUMN) + 1
    if len(header) < need or header[_STATS_COUNT_COLUMN] != "count" or (summed and header[_STATS_SUM_COLUMN] != "sum"):
        raise GestureError(FAILED, "Les statistiques par catégorie n'ont pas rendu les colonnes attendues.")
    summary = []
    for rank, row in enumerate(rows, start=1):
        label = "" if row[0] is None else str(row[0]).strip()
        values = {"count": {"value": _as_number(row[_STATS_COUNT_COLUMN]), "unit": "entités"}}
        if summed:
            values["sum"] = {"value": _as_number(row[_STATS_SUM_COLUMN]), "unit": None}
        summary.append({"label": label or str(rank), "values": values})
    return {"file": out, "rows": summary, "truncated": total > len(rows)}


def _as_number(text):
    """Nombre lu dans une cellule de la table écrite ; une cellule vide vaut 0."""
    try:
        return float(text) if text not in (None, "") else 0.0
    except (TypeError, ValueError):
        raise GestureError(FAILED, "La table des statistiques contient une valeur qui n'est pas un nombre.")


# `qgis:distancematrix` (fournisseur « qgis » du module Processing, pas `native:`), matrice « linéaire »
# (type 0) : une ligne par paire, colonnes `InputID`, `TargetID`, `Distance`. Les types « standard » et
# « résumé » ne sont jamais demandés : le premier est faux quand il y a moins de voisins que de cibles.
_MATRIX_LINEAR = 0
_MATRIX_MAX_PAIRS = 100000  # au-delà, sans `neighbors`, le tableau n'a plus de sens : refus avant lancement
_MATRIX_COLUMNS = ("InputID", "TargetID", "Distance")


def _matrix_neighbors(params):
    """Nombre de voisins demandé (entier au moins égal à 1), ou None : toutes les cibles."""
    neighbors = params.get("neighbors")
    if neighbors is not None and (isinstance(neighbors, bool) or not isinstance(neighbors, int) or neighbors < 1):
        raise GestureError(BAD_PARAMS, "Le nombre de voisins doit être un entier au moins égal à 1.")
    return neighbors


def translate_distance_matrix(params, layer, backend):
    """Paramètres neutres de `analysis.distance_matrix` -> paramètres de `qgis:distancematrix`.

    Refus avant tout lancement, ici parce que la traduction est appelée avant l'algorithme : une couche
    multipartie (QGIS la refuse, sans le dire clairement) et trop de paires sans `neighbors` (le tableau
    aurait des millions de lignes). La couche cible est ajoutée par le geste (clé `overlay`) ; QGIS la
    transforme lui-même vers le système de la couche de départ, que la chaîne a mis en mètres."""
    target = backend.layer(params.get("target_layer_id"))
    for one in (layer, target):
        if one is not None and backend.is_multipart(one):
            raise GestureError(
                BAD_PARAMS,
                "La couche « %s » est une couche de multipoints : cette commande ne lit que des points simples."
                % backend.name_of(one),
            )
    neighbors = _matrix_neighbors(params)
    if neighbors is not None and backend.has_repeated_values(layer, params["label_field"]):
        # Le tri des voisins regroupe les lignes par étiquette de départ : deux départs de même nom
        # (ou sans nom) se partageraient les k voisins, en silence.
        raise GestureError(
            BAD_PARAMS,
            "Le champ « %s » de la couche « %s » a des valeurs vides ou en double : avec un nombre de voisins, "
            "choisissez un champ dont chaque valeur est unique." % (params["label_field"], backend.name_of(layer)),
        )
    if neighbors is None and target is not None:
        pairs = backend.feature_count(layer) * backend.feature_count(target)
        if pairs > _MATRIX_MAX_PAIRS:
            raise GestureError(
                BAD_PARAMS,
                "%d paires de points : trop pour un tableau lisible (au plus %d). "
                "Donnez le nombre de voisins à garder par point de départ." % (pairs, _MATRIX_MAX_PAIRS),
            )
    return {
        "INPUT": layer,
        "INPUT_FIELD": params["label_field"],
        "TARGET_FIELD": params["target_label_field"],
        "MATRIX_TYPE": _MATRIX_LINEAR,
        "NEAREST_POINTS": neighbors if neighbors is not None else 0,  # 0 : toutes les cibles
    }


def translate_distance_matrix_nearest(params, layer):
    """Seconde étape, `native:extractbyexpression` : garde les `neighbors` premières cibles de chaque
    départ. QGIS garde toutes les cibles à égale distance de la k-ième (sonde) : on classe donc chaque
    départ par distance, puis par ordre d'écriture, et on ne garde que les k premières. Sans `neighbors`,
    l'expression est vraie pour toute ligne (la dernière étape d'une chaîne n'est jamais conditionnelle)."""
    neighbors = _matrix_neighbors(params)
    if neighbors is None:
        expression = "true"
    else:
        expression = 'array_find(array_agg($id, group_by:="InputID", order_by:="Distance"), $id) < %d' % neighbors
    return {"INPUT": layer, "EXPRESSION": expression}


def summarize_distance_matrix(backend, out, params):
    """Résultat du tableau écrit (clé `table`) : `file`, le nombre de paires et les 100 premières lignes,
    « départ → cible » et distance en mètres. QGIS ne trie pas les cibles d'un même départ : les lignes
    affichées le sont, par distance croissante, départ après départ (le fichier garde l'ordre de QGIS)."""
    header, rows, total = backend.table_rows(out, _MAX_ROWS)
    if tuple(header[: len(_MATRIX_COLUMNS)]) != _MATRIX_COLUMNS:
        raise GestureError(FAILED, "La matrice de distances n'a pas rendu les colonnes attendues.")
    first_seen = {}
    for row in rows:
        first_seen.setdefault(row[0], len(first_seen))
    rows = sorted(rows, key=lambda row: (first_seen[row[0]], _as_number(row[2])))
    summary = []
    for rank, row in enumerate(rows, start=1):
        start = "" if row[0] is None else str(row[0]).strip()
        target = "" if row[1] is None else str(row[1]).strip()
        label = "%s → %s" % (start, target) if start and target else str(rank)
        summary.append({"label": label, "values": {"distance": {"value": _as_number(row[2]), "unit": "m"}}})
    return {"file": out, "pair_count": total, "rows": summary, "truncated": total > len(rows)}


# Champs écrits par `native:sumlinelengths` : la longueur, puis le nombre de lignes. Si l'entrée porte
# déjà un champ de ce nom, QGIS le remplace sur place (sonde) : on lit donc la longueur par son nom.
_LENGTH_FIELD = "length_m"
_LINE_COUNT_FIELD = "line_count"


def translate_sum_line_lengths(params, layer):
    """Paramètres neutres de `analysis.sum_line_lengths` -> `native:sumlinelengths`. La longueur est
    en mètres, dans le système de la couche de polygones, que la chaîne a mis en mètres ; la couche de
    lignes est ajoutée par le geste (clé `overlay_key`)."""
    return {
        "POLYGONS": layer,
        "LEN_FIELD": _LENGTH_FIELD,
        "COUNT_FIELD": _LINE_COUNT_FIELD,
    }


def summarize_sum_line_lengths(backend, layer, params):
    """Longueur par polygone des 100 premiers polygones de la couche produite (clé `summary`), dans
    l'unité demandée ; un polygone sans ligne vaut 0."""
    if _LENGTH_FIELD not in [name for name, _ in backend.fields_of(layer)]:
        raise GestureError(FAILED, "La somme des longueurs n'a pas rendu le champ attendu.")
    unit = params.get("unit") or "m"
    factor = _UNIT_TO_METERS[unit]
    pairs, truncated = backend.numbers_of(layer, _LENGTH_FIELD, params.get("label_field") or None, _MAX_ROWS)
    rows = [{"label": label, "values": {"length": {"value": value / factor, "unit": unit}}} for label, value in pairs]
    return {"rows": rows, "truncated": truncated}


_AREA_UNITS = {"m2": 1.0, "ha": 10000.0}


def _area_field(params):
    """Nom fixe du champ de surface : il porte l'unité (`area_m2`, `area_ha`), pour que la sélection
    numérique qui suit compare dans la bonne unité."""
    return "area_%s" % params["unit"]


def translate_area(params, layer):
    """Paramètres neutres de `analysis.area` -> `native:fieldcalculator`. Formule fixe de la ligne de
    table, jamais écrite par le modèle : `area($geometry)` est plane, dans le système de la couche (mis
    en mètres par la chaîne), donc indépendante de l'ellipsoïde du projet (jamais `$area`). Un champ du
    même nom est remplacé."""
    formula = "area($geometry)"
    if _AREA_UNITS[params["unit"]] != 1.0:
        formula += " / %d" % _AREA_UNITS[params["unit"]]
    return {
        "INPUT": layer,
        "FIELD_NAME": _area_field(params),
        "FIELD_TYPE": 0,  # décimal
        "FIELD_LENGTH": 0,
        "FIELD_PRECISION": 0,
        "FORMULA": formula,
    }


def summarize_area(backend, layer, params):
    """Champ de surface (lu sur la couche produite) et son unité (clé `summary`)."""
    field = _area_field(params)
    if field not in [name for name, _ in backend.fields_of(layer)]:
        raise GestureError(FAILED, "Le calcul des surfaces n'a pas rendu le champ attendu.")
    return {"field": field, "unit": params["unit"]}


def translate_atlas(params, layer):
    """Paramètres neutres de `layout.export_atlas` -> paramètres de `native:atlaslayouttopdf`.

    La couverture est la couche d'entrée ; le nom de la mise en page et les couches locales à
    dessiner viennent du genre `layout` (clés `name_key` et `layers_key` de la table). Ni
    `FILTER_EXPRESSION` ni `SORTBY_EXPRESSION` : aucune expression n'est émise (AD-23)."""
    return {"COVERAGE_LAYER": layer}


# Table fermée : id de commande -> chaîne fixe d'algorithmes (AD-24).
#
# Une ligne est une chaîne fixe d'étapes plus des attributs de la ligne :
#   chain_version : entier, version de la chaîne (AD-11), incrémenté à chaque changement de la
#                   ligne ; 1 pour toutes les lignes à l'origine de la version de chaîne ;
#   chain         : liste d'étapes, exécutées dans l'ordre. Chaque étape travaille sur la couche
#                   d'entrée (la première) ou sur la sortie temporaire de la précédente ; seule la
#                   dernière écrit `out`, dicté par le noyau (AD-10), les autres écrivent des
#                   temporaires de QGIS, jamais nommés ni journalisés ;
#   input     : nom du paramètre du noyau qui désigne la couche d'entrée ;
#   metric    : l'algorithme mesure en unités de la couche, donc le SCR doit être en mètres ;
#   overlay   : (facultatif) paramètre du noyau qui désigne une seconde couche ; l'étape qui la reçoit
#               porte `overlay_key`, la clé de l'algorithme ;
#   overlay_numeric_field : (facultatif, avec `overlay`) paramètre du noyau qui nomme un champ
#               numérique de la seconde couche ; absent, rien n'est vérifié ; présent, un champ
#               inconnu est `analysis.unknown_field` et un champ non numérique `analysis.bad_params`,
#               avant tout lancement ;
#   field     : (facultatif) paramètre du noyau qui nomme un champ de la couche d'entrée ;
#               avec `field_optional`, il peut être absent ;
#   geometry  : (facultatif) type de géométrie exigé de la couche d'entrée (ex. `Polygon`) ;
#   origin    : (facultatif) paramètre du noyau `{x, y, epsg}` qui doit tomber à moins de
#               `network.tolerance` (résolue par le noyau, AD-28) d'une entité de la couche d'entrée (sinon
#               `analysis.origin_off_network`, avant tout lancement) ;
#   points    : (facultatif, avec `network`) noms de paramètres du noyau (départ, arrivée), chacun un point
#               `{x, y, epsg}` qui doit tomber, comme `origin`, à moins de `network.tolerance` d'une entité
#               de la couche d'entrée, sinon `analysis.origin_off_network` (le message nomme le point),
#               avant tout lancement ;
#   summary   : (facultatif) (fond, couche produite, params) -> dict qui complète le résultat ;
#   table     : (facultatif) genre de sortie `table` (AD-24) : `out` est un tableau sans géométrie (CSV),
#               jamais ajouté au projet ; (fond, `out`, params) -> dict qui est le résultat (`file`,
#               `rows`, `truncated`) ;
#   numeric_field : (facultatif) paramètre du noyau qui nomme un champ numérique de la couche d'entrée ;
#               absent, rien n'est vérifié ; présent, un champ inconnu est `analysis.unknown_field` et un
#               champ non numérique `analysis.bad_params`, avant tout lancement ;
#   overlay_field : (facultatif, avec `overlay`) paramètre du noyau qui nomme un champ de la seconde
#               couche, de n'importe quel type ; un champ inconnu est `analysis.unknown_field`, avant
#               tout lancement ;
#   summary_results : (facultatif, avec `summary`) le résumé reçoit aussi les sorties de l'algorithme
#               de la dernière étape : `summary(fond, couche, params, résultats)` ;
#   fields_list : (facultatif) paramètre du noyau qui nomme une liste de champs de la couche d'entrée ;
#               chacun est vérifié, un champ inconnu est `analysis.unknown_field`, avant tout lancement ;
#   overlay_geometry : (facultatif, avec `overlay`) type de géométrie exigé de la seconde couche ;
#   target_crs: (facultatif) paramètre du noyau qui porte un code EPSG d'arrivée ; le fond
#               doit le connaître (`crs_is_known`), sinon `data.unknown_crs`, avant tout
#               lancement ;
#   file_result: (facultatif) paramètre du noyau qui nomme le format écrit : la sortie est un
#               fichier, relu à part pour compter ses entités mais jamais ajouté au projet ;
#               le résultat est `file`, `format`, `feature_count` ; le fichier relu doit
#               être du pilote du format (`data.format_mismatch` sinon) ; un Shapefile
#               demande une couche d'un seul type de géométrie, vérifié avant tout lancement ;
#   encoding  : (facultatif) encodage des textes écrits, posé sur le contexte de traitement
#               de cette seule commande (étape finale) ;
#   raster    : (facultatif) genre d'entrée `raster` (AD-24) : `input` nomme un `RasterPath`,
#               chemin absolu donné par le noyau (AD-9), et non un identifiant de couche ; le
#               fichier doit exister et se lire en raster, sinon `layer.not_found` ou
#               `layer.invalid_source` ; la sortie est un GeoTIFF, relu à part, jamais ajouté au
#               projet (le noyau dicte `layer.open_raster`) ; le résultat est `file` et `epsg` ;
#   raster_input : (facultatif) genre d'entrée mixte : `input` reste une couche de polygones et ce paramètre du
#               noyau nomme un `RasterPath` lu en entrée (chemin absolu donné par le noyau, AD-9) ; le fichier
#               doit exister, être un GeoTIFF lisible d'une seule bande et avoir un système de coordonnées
#               (`layer.not_found`, `layer.invalid_source`, `analysis.bad_params`), avant tout lancement ;
#               un système différent de celui des polygones est admis : QGIS reprojette les polygones ;
#               il est libéré avant l'algorithme et jamais modifié ; la sortie reste une couche ;
#   layer_output : (facultatif, avec `raster`) la sortie est une couche de lignes ou de polygones ajoutée au
#               projet (résultat `layer_id`, `name`, `feature_count`, `crs`), et non un GeoTIFF ; le raster
#               d'entrée est contrôlé comme un `raster_input` (GeoTIFF d'une bande, avec système) ;
#   assign_crs: (facultatif, avec `raster`) paramètre du noyau qui déclare le code EPSG d'une
#               source sans système : connu de QGIS (`data.unknown_crs` sinon), et refusé
#               (`analysis.bad_params`) si la source a déjà un autre système ;
#   reproject_to : (facultatif, avec `raster`) paramètre du noyau qui porte le système en mètres de la
#               reprojection (ticket 33) : connu de QGIS et en mètres, jamais le système du raster ;
#   layout    : (facultatif) genre `layout` (AD-24) : l'algorithme exporte une mise en page nommée du
#               gestionnaire du projet. Le geste monte la mise en page des couches locales visibles
#               (`backend.mount_atlas`), l'y ajoute le temps de l'appel et la retire dans tous les cas
#               (succès, échec, annulation) ; `{"name_key": clé de l'algorithme qui reçoit son nom,
#               "layers_key": clé qui reçoit les couches locales, "title": paramètre du noyau du titre}` ;
#               `input` est la couche de couverture (une page par entité, vide : `layer.empty`) et
#               `field` (facultatif) le champ ajouté au titre, vérifié contre les champs de la couche ;
#               la sortie est un fichier PDF, jamais ajouté au projet ; le résultat est `file`,
#               `page_count`, `layer_count`, `skipped_layers`, `sources`, `truncated` ;
#   work_crs  : (facultatif) paramètre du noyau qui porte le système de calcul en mètres, que le
#               noyau ne remplit que pour une couche en degrés (changement de cap du 2026-11-12,
#               amendement RA06 d'AD-24). Rempli : inconnu de QGIS, `data.unknown_crs` ; pas en
#               mètres, `analysis.crs_not_metric`, avant tout lancement. Il va avec l'étape de
#               reprojection `_REPROJECT_TO_WORK`, qui porte `when` : étape exécutée seulement si ce
#               paramètre est rempli. Absent : la chaîne sans elle, avec la garde `metric`, comme avant.
#               Le choix est dicté par le paramètre du noyau ; le geste ne calcule ni ne choisit
#               aucun système (AD-5, AD-24).
#
# Une étape :
#   algorithm : id de l'algorithme du cadre de traitement ;
#   translate : (params du noyau, couche d'entrée de l'étape[, fond]) -> paramètres de
#               l'algorithme, sans la sortie ;
#   output    : nom du paramètre de sortie de l'algorithme (`OUTPUT` par défaut) ;
#   when      : (facultatif) paramètre du noyau : l'étape n'est exécutée que s'il est rempli ;
#   guard     : (facultatif) `{"empty": (code, message)}` : erreur nommée si la couche d'entrée de
#               l'étape (sortie de la précédente) n'a aucune entité ;
#   with_backend : (facultatif) la traduction reçoit aussi le fond (lecture des champs) ;
#   on_failure : (facultatif) `(code, message)` : erreur nommée quand l'algorithme de cette étape échoue
#               (sans être annulé), au lieu de `analysis.failed` (ex. aucun chemin) ;
#   overlay_key  : (facultatif) clé de l'algorithme qui reçoit la seconde couche de la ligne ;
#   nothing   : (facultatif) message si l'étape ne rend aucune sortie temporaire.
# Aucune étape ne choisit entre deux suites : chaque valeur vient des paramètres du noyau.
_REPROJECT_TO_WORK = {
    "algorithm": _WORK_CRS_ALGORITHM,
    "translate": translate_work_reprojection,
    "output": "OUTPUT",
    "when": WORK_EPSG,
    "nothing": "La reprojection de la couche d'entrée n'a rien rendu.",
}

ALGORITHMS = {
    "analysis.buffer": {
        "chain_version": 1,
        "chain": [
            _REPROJECT_TO_WORK,
            {"algorithm": "native:buffer", "translate": translate_buffer, "output": "OUTPUT"},
        ],
        "input": "layer_id",
        "metric": True,
        "work_crs": WORK_EPSG,
    },
    "analysis.intersect": {
        "chain_version": 1,
        "chain": [
            {
                "algorithm": "native:intersection",
                "translate": translate_intersect,
                "output": "OUTPUT",
                "overlay_key": "OVERLAY",
            },
        ],
        "input": "layer_id",
        "overlay": "overlay_layer_id",
    },
    "analysis.difference": {
        "chain_version": 1,
        "chain": [
            {
                "algorithm": "native:difference",
                "translate": translate_difference,
                "output": "OUTPUT",
                "overlay_key": "OVERLAY",
            },
        ],
        "input": "layer_id",
        "overlay": "overlay_layer_id",
    },
    "analysis.select_by_attribute": {
        "chain_version": 1,
        "chain": [
            {"algorithm": "native:extractbyattribute", "translate": translate_select_by_attribute, "output": "OUTPUT"},
        ],
        "input": "layer_id",
        "field": "field",
    },
    "analysis.count_points_in_polygon": {
        "chain_version": 1,
        "chain": [
            {
                "algorithm": "native:countpointsinpolygon",
                "translate": translate_count_points_weighted,
                "output": "OUTPUT",
                "overlay_key": "POINTS",
                "when": "weight_field",
            },
            {
                "algorithm": "native:countpointsinpolygon",
                "translate": translate_count_points,
                "output": "OUTPUT",
                "overlay_key": "POINTS",
            },
        ],
        "input": "layer_id",
        "overlay": "points_layer_id",
        "overlay_numeric_field": "weight_field",
        "field": "label_field",
        "field_optional": True,
        "geometry": "Polygon",
        "summary": summarize_count_points,
    },
    "raster.zonal_statistics": {
        "chain_version": 1,
        "chain": [
            {"algorithm": "native:zonalstatisticsfb", "translate": translate_raster_zonal, "output": "OUTPUT"},
        ],
        "input": "layer_id",
        "raster_input": "raster",
        "field": "label_field",
        "field_optional": True,
        "geometry": "Polygon",
        "summary": summarize_raster_zonal,
    },
    "raster.contour": {
        "chain_version": 1,
        "chain": [
            {"algorithm": "gdal:contour", "translate": translate_raster_contour, "output": "OUTPUT"},
        ],
        "input": "raster",
        "raster": True,
        "layer_output": True,
    },
    "analysis.nearest": {
        "chain_version": 1,
        "chain": [
            _REPROJECT_TO_WORK,
            {
                "algorithm": "native:joinbynearest",
                "translate": translate_nearest,
                "output": "OUTPUT",
                "overlay_key": "INPUT_2",
            },
            {
                "algorithm": "native:extractbyexpression",
                "translate": translate_nearest_first,
                "output": "OUTPUT",
                "with_backend": True,
            },
        ],
        "input": "layer_id",
        "metric": True,
        "work_crs": WORK_EPSG,
        "overlay": "target_layer_id",
        "summary": summarize_nearest,
    },
    "analysis.join_table": {
        "chain_version": 1,
        "chain": [
            {
                "algorithm": "native:joinattributestable",
                "translate": translate_join_table,
                "output": "OUTPUT",
                "overlay_key": "INPUT_2",
            },
        ],
        "input": "layer_id",
        "overlay": "table_layer_id",
        "overlay_field": "table_field",
        "field": "field",
        "summary": summarize_join_table,
        "summary_results": True,
    },
    "analysis.dissolve": {
        "chain_version": 1,
        "chain": [
            {
                "algorithm": "native:dissolve",
                "translate": translate_dissolve,
                "output": "OUTPUT",
            },
        ],
        "input": "layer_id",
        "fields_list": "fields",
    },
    "analysis.statistics_by_category": {
        "chain_version": 1,
        "chain": [
            {
                "algorithm": "native:fieldcalculator",
                "translate": translate_sum_prepared,
                "output": "OUTPUT",
                "when": "sum_field",
            },
            {
                "algorithm": "qgis:statisticsbycategories",
                "translate": translate_statistics_by_category,
                "output": "OUTPUT",
            },
        ],
        "input": "layer_id",
        "field": "category_field",
        "numeric_field": "sum_field",
        "table": summarize_statistics_by_category,
    },
    "analysis.distance_matrix": {
        "chain_version": 1,
        "chain": [
            _REPROJECT_TO_WORK,
            {
                "algorithm": "qgis:distancematrix",
                "translate": translate_distance_matrix,
                "output": "OUTPUT",
                "overlay_key": "TARGET",
                "with_backend": True,
            },
            {
                "algorithm": "native:extractbyexpression",
                "translate": translate_distance_matrix_nearest,
                "output": "OUTPUT",
            },
        ],
        "input": "layer_id",
        "metric": True,
        "work_crs": WORK_EPSG,
        "overlay": "target_layer_id",
        "overlay_geometry": "Point",
        "overlay_field": "target_label_field",
        "field": "label_field",
        "geometry": "Point",
        "table": summarize_distance_matrix,
    },
    "analysis.sum_line_lengths": {
        "chain_version": 1,
        "chain": [
            _REPROJECT_TO_WORK,
            {
                "algorithm": "native:sumlinelengths",
                "translate": translate_sum_line_lengths,
                "output": "OUTPUT",
                "overlay_key": "LINES",
            },
        ],
        "input": "layer_id",
        "metric": True,
        "work_crs": WORK_EPSG,
        "overlay": "lines_layer_id",
        "overlay_geometry": "Line",
        "field": "label_field",
        "field_optional": True,
        "geometry": "Polygon",
        "summary": summarize_sum_line_lengths,
    },
    "analysis.area": {
        "chain_version": 1,
        "chain": [
            _REPROJECT_TO_WORK,
            {"algorithm": "native:fieldcalculator", "translate": translate_area, "output": "OUTPUT"},
        ],
        "input": "layer_id",
        "metric": True,
        "work_crs": WORK_EPSG,
        "geometry": "Polygon",
        "summary": summarize_area,
    },
    "analysis.isochrone": {
        # Version 2 : reprojection du réseau vers `work_epsg` en tête de chaîne (entrée 10.32).
        "chain_version": 2,
        "chain": [
            _REPROJECT_TO_WORK,
            {
                "algorithm": "native:serviceareafrompoint",
                "translate": translate_isochrone,
                "output": "OUTPUT_LINES",
                "with_backend": True,
            },
            {
                "algorithm": "native:convexhull",
                "translate": translate_convex_hull,
                "output": "OUTPUT",
                "guard": {
                    "empty": (
                        ORIGIN_OFF_NETWORK,
                        "Aucun tronçon du réseau n'est atteint depuis cette origine : "
                        "vérifiez qu'elle se trouve sur le réseau choisi.",
                    )
                },
            },
        ],
        "input": "layer_id",
        "metric": True,
        "work_crs": WORK_EPSG,
        "geometry": "Line",
        "origin": "origin",
        "network": "network",
    },
    "network.shortest_path": {
        "chain_version": 1,
        "chain": [
            _REPROJECT_TO_WORK,
            {
                "algorithm": "native:shortestpathpointtopoint",
                "translate": translate_shortest_path,
                "output": "OUTPUT",
                "with_backend": True,
                "on_failure": (
                    NO_PATH,
                    "Aucun chemin ne relie le départ à l'arrivée sur ce réseau : il n'est pas connecté entre "
                    "ces deux points, un sens interdit les sépare, ou ils tombent au même endroit.",
                ),
            },
        ],
        "input": "layer_id",
        "metric": True,
        "work_crs": WORK_EPSG,
        "geometry": "Line",
        "network": "network",
        "points": ("start", "end"),
        "summary": summarize_shortest_path,
        "summary_results": True,
    },
    "network.prepare": {
        "chain_version": 1,
        "chain": [
            _REPROJECT_TO_WORK,
            {"algorithm": "native:snapgeometries", "translate": translate_prepare_snap, "output": "OUTPUT"},
            {"algorithm": "native:splitwithlines", "translate": translate_prepare_split, "output": "OUTPUT"},
            {
                "algorithm": "native:removenullgeometries",
                "translate": translate_prepare_remove_null,
                "output": "OUTPUT",
            },
            {
                "algorithm": "native:reprojectlayer",
                "translate": translate_prepare_back,
                "output": "OUTPUT",
                "when": WORK_EPSG,
                "with_backend": True,
                "nothing": "Le retour au système de la couche n'a rien rendu.",
            },
            {"algorithm": "native:deleteduplicategeometries", "translate": translate_prepare_dedupe, "output": "OUTPUT"},
        ],
        "input": "layer_id",
        "metric": True,
        "work_crs": WORK_EPSG,
        "geometry": "Line",
    },
    "layout.export_atlas": {
        "chain_version": 1,
        "chain": [
            {
                "algorithm": "native:atlaslayouttopdf",
                "translate": translate_atlas,
                "output": "OUTPUT",
                "on_failure": (LAYOUT_EXPORT_FAILED, "L'export de l'atlas a échoué : aucun fichier n'a été écrit."),
            },
        ],
        "input": "coverage_layer_id",
        "field": "label_field",
        "field_optional": True,
        "layout": {"name_key": "LAYOUT", "layers_key": "LAYERS", "title": "title"},
    },
    "data.reproject": {
        "chain_version": 1,
        "chain": [
            {"algorithm": "native:reprojectlayer", "translate": translate_reproject, "output": "OUTPUT"},
        ],
        "input": "layer_id",
        "target_crs": "target_epsg",
    },
    "data.convert": {
        "chain_version": 1,
        "chain": [
            {"algorithm": "native:extractbyexpression", "translate": translate_convert, "output": "OUTPUT"},
        ],
        "input": "layer_id",
        "file_result": "format",
        "encoding": "UTF-8",
    },
    "data.convert_raster": {
        "chain_version": 1,
        "chain": [
            {"algorithm": "gdal:warpreproject", "translate": translate_convert_raster, "output": "OUTPUT"},
        ],
        "input": "raster",
        "raster": True,
        "assign_crs": "assign_epsg",
    },
    # Ticket 33 : reprojection d'un MNT en degrés, dictée par le noyau avant une commande de terrain.
    "raster.reproject_work": {
        "chain_version": 1,
        "chain": [
            {
                "algorithm": "gdal:warpreproject",
                "translate": translate_reproject_work,
                "output": "OUTPUT",
            },
        ],
        "input": "raster",
        "raster": True,
        "reproject_to": WORK_EPSG,
    },
}


for _row in ALGORITHMS.values():
    # Lecture seule, dérivée de la chaîne : l'algorithme de la première étape inconditionnelle, pour les
    # lecteurs de la table qui ne regardent qu'un algorithme par ligne.
    _row["algorithm"] = next(s for s in _row["chain"] if "when" not in s)["algorithm"]
del _row


def chain_of(entry):
    """Étapes d'une ligne. Une ligne à l'ancienne forme (`algorithm`, `then`, `work_crs` au premier
    niveau, sans `chain`) est lue comme la chaîne qu'elle décrit : le même geste les exécute."""
    if "chain" in entry:
        return list(entry["chain"])
    steps = []
    if entry.get("work_crs"):
        param = entry["work_crs"]
        steps.append(
            {
                "algorithm": _WORK_CRS_ALGORITHM,
                "translate": lambda params, layer: translate_work_crs(params[param], layer),
                "output": "OUTPUT",
                "when": param,
                "nothing": _REPROJECT_TO_WORK["nothing"],
            }
        )
    main = {"algorithm": entry["algorithm"], "translate": entry["translate"], "output": entry["output"]}
    if entry.get("overlay"):
        main["overlay_key"] = entry["overlay_key"]
    steps.append(main)
    then = entry.get("then")
    if then is not None:
        step = {"algorithm": then["algorithm"], "translate": then["translate"], "output": then["output"]}
        if then.get("with_backend"):
            step["with_backend"] = True
        if then.get("empty_error"):
            step["guard"] = {"empty": then["empty_error"]}
        steps.append(step)
    return steps


# Algorithmes de traitement que le geste sait interrompre (AD-12) : tous passent par
# `QgsProcessingAlgRunnerTask` et son retour d'information annulable. Une commande n'est annulable que si
# chaque algorithme de sa chaîne est ici ; un algorithme qui n'accepte pas d'objet de retour annulable
# n'y entre pas, et rend sa commande non annulable.
CANCELLABLE_ALGORITHMS = frozenset(
    {
        "native:buffer",
        "native:intersection",
        "native:difference",
        "native:extractbyattribute",
        "native:countpointsinpolygon",
        "native:zonalstatisticsfb",
        "gdal:contour",
        "native:joinbynearest",
        "native:joinattributestable",
        "native:dissolve",
        "native:extractbyexpression",
        "native:fieldcalculator",
        "qgis:statisticsbycategories",
        "qgis:distancematrix",
        "native:sumlinelengths",
        "native:serviceareafrompoint",
        "native:shortestpathpointtopoint",
        "native:convexhull",
        "native:snapgeometries",
        "native:splitwithlines",
        "native:removenullgeometries",
        "native:deleteduplicategeometries",
        "native:reprojectlayer",
        "gdal:warpreproject",
    }
)

# Algorithmes qui accèdent au réseau ou ouvrent une page (AD-14, AD-24) : aucune ligne n'en cite un. La
# comparaison porte sur le nom sans le fournisseur, pour qu'un autre préfixe ne la contourne pas.
NETWORK_BLACKLIST = frozenset(
    {
        "native:filedownloader",
        "native:ht" "tprequest",  # écrit en deux morceaux : la garde de source refuse ce mot partout
        "native:downloadvectortiles",
        "native:openurl",
        "native:batchnominatimgeocoder",
    }
)
_NETWORK_NAMES = frozenset(name.split(":", 1)[-1] for name in NETWORK_BLACKLIST)


def required_algorithms(entry):
    """Algorithmes dont une ligne a besoin : ceux de toutes ses étapes, les étapes `when` comprises
    (AD-25). Vide pour un geste fixe sans algorithme. Lève `KeyError`/`TypeError` sur une ligne illisible."""
    if "gesture" in entry:
        return []
    return [step["algorithm"] for step in chain_of(entry)]


def available_chains(table, backend):
    """Lignes de la table dont toute la chaîne existe dans le registre de traitement du QGIS hôte :
    id de commande -> `chain_version` (AD-25). Règle unique pour toute ligne, `data.reproject`
    comprise ; une ligne illisible n'est pas annoncée. Un geste fixe sans algorithme l'est toujours."""
    available = {}
    for command_id, entry in table.items():
        try:
            present = all(backend.algorithm_exists(a) for a in required_algorithms(entry))
            version = entry["chain_version"]
        except (KeyError, TypeError, AttributeError):
            present = False
        if present:
            available[command_id] = version
    return available


def derived_cancellable(table):
    """Annulabilité de chaque ligne, déduite de sa chaîne : id -> bool. Une ligne de geste fixe
    (`gesture`), une chaîne vide ou une ligne illisible ne sont pas annulables."""
    result = {}
    for command_id, entry in table.items():
        try:
            chain = [] if "gesture" in entry else chain_of(entry)
            result[command_id] = bool(chain) and all(s["algorithm"] in CANCELLABLE_ALGORITHMS for s in chain)
        except (KeyError, TypeError, AttributeError):
            result[command_id] = False
    return result


def validate_table(table=None):
    """Invariants de la table fermée (AD-24) : liste de messages, vide si la table est valide ; ne lève jamais.

    Chaque ligne porte une `chain_version` entière, soit une chaîne non vide d'étapes, soit un geste
    fixe d'entrée `project` et de sortie `none` ; aucune étape ne cite un algorithme de la liste noire
    du réseau ; une garde est une paire (code, message), jamais sur la première étape ; la dernière étape
    n'est jamais conditionnelle et la chaîne a une étape inconditionnelle ; le `work_crs` de la ligne et le
    `when` de l'étape de reprojection vont ensemble.
    """
    errors = []
    for command_id, entry in (ALGORITHMS if table is None else table).items():
        try:
            _validate_row(command_id, entry, errors)
        except Exception as exc:  # une ligne illisible est un message, jamais une trace
            errors.append("%s : ligne illisible (%s)" % (command_id, type(exc).__name__))
    return errors


def _validate_row(command_id, entry, errors):
    version = entry.get("chain_version")
    if isinstance(version, bool) or not isinstance(version, int) or version < 1:
        errors.append("%s : chain_version absente ou invalide (un entier au moins égal à 1)" % command_id)
    if "gesture" in entry:
        if "chain" in entry:
            errors.append("%s : ligne de geste fixe qui porte aussi une chaîne" % command_id)
        if entry.get("input") != "project" or entry.get("output") != "none":
            errors.append("%s : un geste fixe est d'entrée project et de sortie none" % command_id)
        return
    if "chain" not in entry and "algorithm" not in entry:
        errors.append("%s : ligne sans chaîne ni algorithme" % command_id)
        return
    chain = chain_of(entry)
    if not chain:
        errors.append("%s : chaîne vide" % command_id)
        return
    for index, step in enumerate(chain):
        algorithm = step.get("algorithm")
        if not isinstance(algorithm, str) or not algorithm:
            errors.append("%s : étape %d sans algorithme" % (command_id, index + 1))
            continue
        lowered = algorithm.lower()
        if lowered in NETWORK_BLACKLIST or lowered.split(":", 1)[-1] in _NETWORK_NAMES:
            errors.append(
                "%s : l'algorithme « %s » accède au réseau ou ouvre une page, il est interdit (AD-14, AD-24)"
                % (command_id, algorithm)
            )
        if "guard" in step:
            guard = step["guard"]
            empty = guard.get("empty") if isinstance(guard, dict) else None
            if not (isinstance(empty, tuple) and len(empty) == 2):
                errors.append(
                    "%s : étape %d, la garde est {\"empty\": (code, message)}" % (command_id, index + 1)
                )
            if index == 0:
                errors.append("%s : la première étape ne peut pas porter de garde (rien à contrôler)" % command_id)
        if "on_failure" in step:
            failure = step["on_failure"]
            if not (isinstance(failure, tuple) and len(failure) == 2 and all(isinstance(t, str) for t in failure)):
                errors.append("%s : étape %d, on_failure est (code, message)" % (command_id, index + 1))
        if "when" in step and index == len(chain) - 1:
            errors.append("%s : la dernière étape ne peut pas être conditionnelle" % command_id)
    if all("when" in s for s in chain):
        errors.append("%s : la chaîne n'a aucune étape inconditionnelle" % command_id)
    whens = {s["when"] for s in chain if "when" in s}
    work = entry.get("work_crs")
    if work is not None and work not in whens:
        errors.append("%s : work_crs « %s » sans étape dont `when` est ce paramètre" % (command_id, work))
    if WORK_EPSG in whens and work != WORK_EPSG:
        errors.append("%s : une étape dépend de %s mais la ligne n'a pas de work_crs" % (command_id, WORK_EPSG))


def _is_null(value):
    """Vrai pour une valeur nulle de QGIS (`None` ou `QVariant` nul)."""
    return value is None or (hasattr(value, "isNull") and value.isNull())


class QgisBackend(_ReadBackend):
    """Accès réel à QGIS. Seul ce type touche `qgis.core`."""

    def crs_is_metric(self, layer):
        from qgis.core import Qgis

        return layer.crs().mapUnits() == Qgis.DistanceUnit.Meters

    def crs_is_known(self, epsg):
        """Vrai si QGIS connaît le code EPSG `epsg`."""
        from qgis.core import QgsCoordinateReferenceSystem

        return QgsCoordinateReferenceSystem("EPSG:%d" % epsg).isValid()

    def epsg_is_metric(self, epsg):
        """Vrai si le système EPSG:`epsg` a le mètre pour unité, selon QGIS."""
        from qgis.core import Qgis, QgsCoordinateReferenceSystem

        crs = QgsCoordinateReferenceSystem("EPSG:%d" % epsg)
        return crs.isValid() and crs.mapUnits() == Qgis.DistanceUnit.Meters

    def algorithm_exists(self, algorithm_id):
        """Vrai si le registre de traitement du QGIS hôte connaît l'algorithme (AD-25)."""
        from qgis.core import QgsApplication

        return QgsApplication.processingRegistry().algorithmById(algorithm_id) is not None

    def qgis_version(self):
        """Version du QGIS hôte, annoncée à `session.hello` (AD-25)."""
        from qgis.core import Qgis

        return str(Qgis.version())

    def run_algorithm(self, algorithm_id, parameters, finished, previous=None, encoding=None):
        """Lance l'algorithme en tâche annulable ; `finished(ok, results)` à la fin.

        `encoding` : encodage des textes écrits par le puits de sortie (sinon celui du système,
        cp1252 sous Windows pour un Shapefile).

        Rend un objet à `cancel()`, qui garde la tâche, son contexte et son retour
        référencés tant que le travail dure. `previous` : le retour de l'étape précédente,
        gardé référencé tant que celle-ci porte la couche temporaire passée en entrée.
        """
        from qgis.core import (
            QgsApplication,
            QgsProcessingAlgRunnerTask,
            QgsProcessingContext,
            QgsProcessingFeedback,
            QgsProject,
        )

        algorithm = QgsApplication.processingRegistry().algorithmById(algorithm_id)
        if algorithm is None:
            raise GestureError(FAILED, "Le cadre de traitement de QGIS ne fournit pas cet algorithme.")
        context = QgsProcessingContext()
        context.setProject(QgsProject.instance())
        if encoding:
            context.setDefaultEncoding(encoding)
        feedback = QgsProcessingFeedback()
        task = QgsProcessingAlgRunnerTask(algorithm, dict(parameters), context, feedback)
        handle = _TaskHandle(task, context, feedback, previous)
        task.executed.connect(lambda ok, results: finished(bool(ok), results or {}))
        QgsApplication.taskManager().addTask(task)
        return handle

    def mount_atlas(self, coverage, title, label_field=None):
        """Monte la mise en page d'atlas des couches locales visibles et l'ajoute au gestionnaire du
        projet (genre `layout`) ; rend un objet à `remove()`, `name`, `layers`, `skipped`, `names`,
        `page_count`. `GestureError` (`layout.empty`) sans couche locale visible."""
        try:  # le module de mise en page ne se charge que pour ce genre
            from .layout_export import mount_atlas
        except ImportError:  # pragma: no cover
            from layout_export import mount_atlas

        return mount_atlas(coverage, title, label_field)

    def transform_point(self, x, y, epsg, target_epsg):
        """Point (x, y) en EPSG:`epsg` transformé par QGIS vers EPSG:`target_epsg`."""
        from qgis.core import QgsCoordinateReferenceSystem, QgsCoordinateTransform, QgsPointXY, QgsProject

        source = QgsCoordinateReferenceSystem("EPSG:%d" % epsg)
        target = QgsCoordinateReferenceSystem("EPSG:%d" % target_epsg)
        if not source.isValid() or not target.isValid():
            raise GestureError(BAD_PARAMS, "Le code EPSG de l'origine est inconnu.")
        point = QgsCoordinateTransform(source, target, QgsProject.instance()).transform(QgsPointXY(x, y))
        return point.x(), point.y()

    def has_feature_near(self, layer, x, y, epsg, radius, work_epsg=None):
        """Vrai si une entité de la couche passe à `radius` du point : en unités de la couche, ou,
        avec `work_epsg` (système en mètres), en mètres dans ce système, point et entités transformés."""
        from qgis.core import (
            QgsCoordinateReferenceSystem,
            QgsCoordinateTransform,
            QgsFeatureRequest,
            QgsGeometry,
            QgsPointXY,
            QgsProject,
            QgsRectangle,
        )

        point = QgsPointXY(x, y)
        source = QgsCoordinateReferenceSystem("EPSG:%d" % epsg)
        if not source.isValid():
            raise GestureError(BAD_PARAMS, "Le code EPSG de l'origine est inconnu.")
        if work_epsg is not None:
            work = QgsCoordinateReferenceSystem("EPSG:%d" % work_epsg)
            if not work.isValid():
                raise GestureError(BAD_PARAMS, "Le code EPSG du système de calcul est inconnu.")
            context = QgsProject.instance()
            if source != work:
                point = QgsCoordinateTransform(source, work, context).transform(point)
            around = QgsRectangle(point.x() - radius, point.y() - radius, point.x() + radius, point.y() + radius)
            to_layer = None
            if work != layer.crs():
                to_layer = QgsCoordinateTransform(work, layer.crs(), context)
                around = to_layer.transformBoundingBox(around)
            to_work = QgsCoordinateTransform(layer.crs(), work, context) if work != layer.crs() else None
            target = QgsGeometry.fromPointXY(point)
            for feature in layer.getFeatures(QgsFeatureRequest().setFilterRect(around)):
                geometry = QgsGeometry(feature.geometry())
                if to_work is not None:
                    geometry.transform(to_work)
                if geometry.distance(target) <= radius:
                    return True
            return False
        if source != layer.crs():
            point = QgsCoordinateTransform(source, layer.crs(), QgsProject.instance()).transform(point)
        around = QgsRectangle(point.x() - radius, point.y() - radius, point.x() + radius, point.y() + radius)
        target = QgsGeometry.fromPointXY(point)
        request = QgsFeatureRequest().setFilterRect(around)
        return any(f.geometry().distance(target) <= radius for f in layer.getFeatures(request))

    def total_length(self, layer):
        """Somme des longueurs des géométries de la couche, dans son propre système (la chaîne l'a mis
        en mètres) ; None si une entité n'a pas de géométrie."""
        total = 0.0
        for feature in layer.getFeatures():
            geometry = feature.geometry()
            if geometry is None or geometry.isNull():
                return None
            total += geometry.length()
        return total

    def temporary_layer(self, handle, reference):
        """Couche temporaire écrite par l'étape que `handle` vient de finir ; None si absente."""
        from qgis.core import QgsProcessingUtils

        return QgsProcessingUtils.mapLayerFromString(reference, handle.context)

    def counts_of(self, layer, count_field, label_field, limit, weight_field=None):
        """Triplets (étiquette, décompte, somme pondérée) des `limit` premières entités, et si la
        couche en a plus. La somme est None sans `weight_field`, 0 si elle est nulle.

        L'étiquette est la valeur de `label_field`, ou le rang de l'entité (à partir de 1) si le
        champ est absent, vide ou nul : jamais l'identifiant d'entité.
        """
        from qgis.core import QgsFeatureRequest

        request = QgsFeatureRequest()
        request.setLimit(limit)
        triples = []
        for rank, feature in enumerate(layer.getFeatures(request), start=1):
            label = feature[label_field] if label_field else None
            text = "" if label is None else str(label).strip()
            if text in ("", "NULL"):
                text = str(rank)
            count = feature[count_field]
            weighted = None
            if weight_field:
                raw = feature[weight_field]
                try:
                    weighted = float(raw) if raw is not None else 0.0
                except (TypeError, ValueError):
                    weighted = 0.0
            triples.append((text, int(count) if count is not None else 0, weighted))
        return triples, self.feature_count(layer) > limit

    def numbers_of(self, layer, field, label_field, limit):
        """Paires (étiquette, nombre) des `limit` premières entités, et si la couche en a plus. Le
        nombre est celui du champ `field` (0 s'il est nul) ; l'étiquette, la valeur de `label_field`,
        ou le rang de l'entité (à partir de 1) si le champ est absent, vide ou nul : jamais
        l'identifiant d'entité."""
        from qgis.core import QgsFeatureRequest

        request = QgsFeatureRequest()
        request.setLimit(limit)
        pairs = []
        for rank, feature in enumerate(layer.getFeatures(request), start=1):
            label = feature[label_field] if label_field else None
            text = "" if label is None else str(label).strip()
            if text in ("", "NULL"):
                text = str(rank)
            raw = feature[field]
            try:
                number = float(raw) if raw is not None else 0.0
            except (TypeError, ValueError):
                number = 0.0
            pairs.append((text, number))
        return pairs, self.feature_count(layer) > limit

    def has_repeated_values(self, layer, field):
        """Vrai si le champ a une valeur vide, ou une valeur portée par deux entités."""
        seen = set()
        for feature in layer.getFeatures():
            value = feature[field]
            text = "" if value is None else str(value).strip()
            if text in ("", "NULL") or text in seen:
                return True
            seen.add(text)
        return False

    def is_multipart(self, layer):
        """Vrai si la couche est de type multipartie (multipoints, multilignes, multipolygones)."""
        from qgis.core import QgsWkbTypes

        return QgsWkbTypes.isMultiType(layer.wkbType())

    def table_rows(self, path, limit):
        """(noms des colonnes, `limit` premières lignes en textes, nombre total de lignes) du tableau
        écrit, ouvert à part sans l'ajouter au projet ; `GestureError` s'il est illisible."""
        from qgis.core import QgsVectorLayer

        layer = QgsVectorLayer(path, "lecture", "ogr")
        try:
            if not layer.isValid():
                raise GestureError(FAILED, "Le tableau écrit est illisible.")
            header = [field.name() for field in layer.fields()]
            rows = []
            for feature in layer.getFeatures():
                if len(rows) >= limit:
                    break
                rows.append([None if _is_null(v) else str(v) for v in feature.attributes()])
            return header, rows, int(layer.featureCount())
        finally:
            del layer  # le fournisseur relâche le fichier

    def null_count(self, layer, field):
        """Nombre d'entités de la couche dont le champ `field` est nul ou vide."""
        from qgis.core import QgsFeatureRequest

        request = QgsFeatureRequest().setSubsetOfAttributes([field], layer.fields())
        total = 0
        for feature in layer.getFeatures(request):
            value = feature[field]
            if value is None or (hasattr(value, "isNull") and value.isNull()):
                total += 1
        return total

    def read_written(self, path):
        """(nombre d'entités, pilote) du fichier écrit, ouvert à part sans l'ajouter au
        projet ; None s'il est illisible."""
        from qgis.core import QgsVectorLayer

        layer = QgsVectorLayer(path, "lecture", "ogr")
        try:
            if not layer.isValid():
                return None
            return int(layer.featureCount()), layer.storageType()
        finally:
            del layer  # le fournisseur relâche le fichier

    def raster_source(self, path):
        """Le raster `path` ouvert à part, sans l'ajouter au projet ; None s'il est illisible."""
        from qgis.core import QgsRasterLayer

        layer = QgsRasterLayer(path, "source", "gdal")
        return layer if layer.isValid() else None

    def raster_crs(self, layer):
        """Le système du raster : None s'il n'en a pas, sinon son code (`EPSG:2154`, ou un
        texte descriptif quand QGIS ne lui trouve pas de code)."""
        crs = layer.crs()
        if not crs.isValid():
            return None
        return crs.authid() or crs.description() or "?"

    def raster_band_count(self, layer):
        """Nombre de bandes du raster ouvert à part."""
        return int(layer.bandCount())

    def raster_unit(self, path):
        """Unité de la première bande du GeoTIFF, telle que GDAL la porte (`metre`…) ; None si le fichier n'en
        déclare pas, si GDAL n'est pas joignable ou si le fichier est illisible. Lecture seule."""
        try:
            from osgeo import gdal
        except ImportError:
            return None
        dataset = None
        try:
            dataset = gdal.Open(path)
            if dataset is None:
                return None
            return dataset.GetRasterBand(1).GetUnitType() or None
        except Exception:
            return None
        finally:
            dataset = None  # relâche le fichier

    def stats_of(self, layer, fields, label_field, limit):
        """Paires (étiquette, {nom: nombre ou None}) des `limit` premières entités, et si la couche en a plus.
        `fields` : paires (nom de la statistique, champ de la couche). Un champ nul donne None (polygone sans
        cellule) ; l'étiquette est la valeur de `label_field`, ou le rang de l'entité (à partir de 1) si le
        champ est absent, vide ou nul : jamais l'identifiant d'entité."""
        from qgis.core import QgsFeatureRequest

        request = QgsFeatureRequest()
        request.setLimit(limit)
        pairs = []
        for rank, feature in enumerate(layer.getFeatures(request), start=1):
            label = feature[label_field] if label_field else None
            text = "" if label is None else str(label).strip()
            if text in ("", "NULL"):
                text = str(rank)
            stats = {}
            for name, field in fields:
                raw = feature[field]
                try:
                    stats[name] = None if _is_null(raw) else float(raw)
                except (TypeError, ValueError):
                    stats[name] = None
            pairs.append((text, stats))
        return pairs, self.feature_count(layer) > limit

    def same_crs(self, layer, epsg):
        """Vrai si le système du raster est EPSG:`epsg` : même code, ou même définition (un
        `.prj` ESRI du Lambert-93 que QGIS ne rattache pas au code EPSG:2154)."""
        from qgis.core import QgsCoordinateReferenceSystem

        crs = layer.crs()
        target = QgsCoordinateReferenceSystem("EPSG:%d" % epsg)
        if crs.authid().upper() == "EPSG:%d" % epsg or crs == target:
            return True
        proj = crs.toProj()
        return bool(proj) and proj == target.toProj()

    def read_written_raster(self, path):
        """Code EPSG du GeoTIFF écrit (None s'il n'a pas de système), relu à part sans l'ajouter
        au projet ; `False` s'il est illisible."""
        from qgis.core import QgsRasterLayer

        layer = QgsRasterLayer(path, "lecture", "gdal")
        try:
            if not layer.isValid():
                return False
            authid = layer.crs().authid() if layer.crs().isValid() else ""
            if authid.upper().startswith("EPSG:") and authid[5:].isdigit():
                return int(authid[5:])
            return None
        finally:
            del layer  # le fournisseur relâche le fichier

    def load_output(self, path, name):
        """Ouvre la couche écrite et l'ajoute au projet ; None si illisible."""
        from qgis.core import QgsProject, QgsVectorLayer

        layer = QgsVectorLayer(path, name, "ogr")
        if not layer.isValid():
            return None
        QgsProject.instance().addMapLayer(layer)
        return layer


class _TaskHandle:
    def __init__(self, task, context, feedback, previous=None):
        self._task, self._context, self._feedback = task, context, feedback
        self._previous = previous  # garde en vie les couches temporaires de l'étape d'avant

    @property
    def context(self):
        return self._context

    def cancel(self):
        self._feedback.cancel()
        self._task.cancel()


_TEMPORARY = "TEMPORARY_OUTPUT"
_GEOMETRY_LABELS = {"Polygon": "de polygones", "Line": "de lignes", "Point": "de points"}


_SHAPEFILE_COMPANIONS = (".shx", ".dbf", ".prj", ".cpg", ".qix")


def _remove(path):
    """Retire le fichier de sortie et ses fichiers annexes : ceux d'un GeoPackage, les
    compagnons d'un Shapefile (même radical), ou le `.aux.xml` que GDAL pose à côté d'un
    GeoTIFF."""
    paths = [path + suffix for suffix in ("", "-wal", "-shm", "-journal", ".aux.xml")]
    stem, ext = os.path.splitext(path)
    if ext.lower() == ".shp":
        paths += [stem + companion for companion in _SHAPEFILE_COMPANIONS]
    for candidate in paths:
        try:
            os.remove(candidate)
        except OSError:
            pass


def _file_name(path):
    """Le seul nom du fichier, jamais son dossier (AD-9, AD-21)."""
    if not isinstance(path, str):
        return "?"
    return os.path.basename(path.replace("\\", "/")) or "?"


def _singleshot(fn):
    from qgis.PyQt.QtCore import QTimer

    QTimer.singleShot(0, fn)


class ProcessingGesture:
    """Geste générique : valide, traduit, lance l'algorithme de la table, charge la sortie."""

    def __init__(self, entry, backend=None, schedule=None):
        self._entry = entry
        self._backend = backend if backend is not None else QgisBackend()
        self._schedule = schedule if schedule is not None else _singleshot
        self._done = None
        self._params = None
        self._out = None
        self._handle = None
        self._loaded = False
        self._cancelling = False
        self._steps = []  # étapes actives de la chaîne (celles dont `when` est rempli)
        self._index = 0  # étape en cours
        self._overlay_layer = None
        self._results = None
        self._pam = False  # réglage GDAL « pas de .aux.xml » pris par ce geste, à rendre à la fin
        self._mounted = None  # genre `layout` : mise en page ajoutée au projet, retirée à la fin

    def start(self, params, done):
        self._done = done
        self._params = params
        self._schedule(self._launch)

    def cancel(self):
        if self._done is None:  # déjà répondu : la sortie livrée n'est plus à nous
            return
        handle = self._handle
        if handle is not None:
            with contextlib.suppress(Exception):
                handle.cancel()
                # La tâche tient encore le fichier : on répond et on nettoie quand elle
                # rend la main (`_on_finished`), pas avant.
                self._cancelling = True
                return
        self._clean()
        self._finish(error=GestureError(CANCELLED, "Geste annulé."))

    def _launch(self):
        if self._done is None:  # déjà soldé (annulation)
            return
        try:
            self._handle = self._begin()
        except GestureError as exc:
            self._clean()
            self._finish(error=exc)
        except Exception:
            self._clean()
            self._finish(error=GestureError(FAILED, "Échec inattendu du lancement de l'analyse."))

    def _begin(self):
        params = self._params or {}
        entry, backend = self._entry, self._backend
        out = params.get("out")
        if not isinstance(out, str) or not out:
            raise GestureError(BAD_PARAMS, "Le noyau n'a pas donné de fichier de sortie.")
        if entry.get("raster"):
            return self._begin_raster(params, out)
        layer_id = params.get(entry["input"])
        layer = backend.layer(layer_id) if isinstance(layer_id, str) and layer_id else None
        if layer is None:
            raise GestureError(NOT_FOUND, "Aucune couche du projet ne porte cet identifiant.")
        if not backend.is_vector(layer):
            raise GestureError(
                NOT_VECTOR, "La couche « %s » n'est pas une couche vecteur." % backend.name_of(layer)
            )
        # Le système de calcul, quand le noyau en donne un (couche en degrés) : la garde porte sur
        # lui ; sinon la garde porte sur la couche, comme avant l'entrée 9.16.
        work_epsg = params.get(entry["work_crs"]) if entry.get("work_crs") else None
        if work_epsg is not None:
            self._check_work_crs(work_epsg, layer)
        elif entry.get("metric") and not backend.crs_is_metric(layer):
            raise GestureError(
                CRS_NOT_METRIC,
                "La couche « %s » est en %s, dont l'unité n'est pas le mètre : "
                "reprojetez-la d'abord dans un système en mètres (par exemple EPSG:2154)."
                % (backend.name_of(layer), backend.crs_of(layer)),
            )
        if entry.get("target_crs"):
            code = _epsg_code(params.get(entry["target_crs"]))
            if not backend.crs_is_known(code):
                raise GestureError(UNKNOWN_CRS, "Système inconnu de QGIS : EPSG:%d." % code)
        if entry.get("geometry") and backend.geometry_of(layer) != entry["geometry"]:
            raise GestureError(
                BAD_PARAMS,
                "La couche « %s » n'est pas une couche %s : choisissez une couche %s."
                % (backend.name_of(layer), _GEOMETRY_LABELS[entry["geometry"]], _GEOMETRY_LABELS[entry["geometry"]]),
            )
        if (
            entry.get("file_result")
            and params.get(entry["file_result"]) == "shp"
            and backend.geometry_of(layer) not in _SINGLE_GEOMETRIES
        ):
            raise GestureError(
                BAD_PARAMS,
                "La couche « %s » n'a pas un seul type de géométrie (points, lignes ou polygones) : "
                "un Shapefile en demande un. Choisissez le GeoPackage ou le GeoJSON."
                % backend.name_of(layer),
            )
        if entry.get("network"):
            self._check_network(params, layer)
        if entry.get("origin"):
            self._check_origin(params.get(entry["origin"]), layer, params.get(entry.get("network")), work_epsg)
        for key in entry.get("points", ()):
            self._check_origin(
                params.get(key), layer, params.get(entry.get("network")), work_epsg, label=_POINT_LABELS[key]
            )
        overlay = None
        if entry.get("overlay"):
            overlay = self._overlay(params, layer_id)
            if entry.get("overlay_numeric_field") and params.get(entry["overlay_numeric_field"]) is not None:
                self._check_numeric_field(params[entry["overlay_numeric_field"]], overlay)
            if entry.get("overlay_field"):
                self._check_field(params.get(entry["overlay_field"]), overlay)
            kind = entry.get("overlay_geometry")
            if kind and backend.geometry_of(overlay) != kind:
                raise GestureError(
                    BAD_PARAMS,
                    "La couche « %s » n'est pas une couche %s : choisissez une couche %s."
                    % (backend.name_of(overlay), _GEOMETRY_LABELS[kind], _GEOMETRY_LABELS[kind]),
                )
        if entry.get("field") and not (entry.get("field_optional") and not params.get(entry["field"])):
            self._check_field(params.get(entry["field"]), layer)
        if entry.get("layout"):
            self._begin_layout(params, layer)
        if entry.get("raster_input"):
            self._check_raster_input(params.get(entry["raster_input"]), layer)
        if entry.get("fields_list"):
            names = params.get(entry["fields_list"])
            if not isinstance(names, list) or not names:
                raise GestureError(BAD_PARAMS, "Nommez au moins un champ de regroupement.")
            for name in names:
                self._check_field(name, layer)
        if entry.get("numeric_field") and params.get(entry["numeric_field"]) is not None:
            self._check_numeric_field(params[entry["numeric_field"]], layer)
        # Traduits dès maintenant, sur la couche d'entrée : un paramètre invalide est refusé avant
        # tout lancement, même quand une reprojection ouvre la chaîne. `out` n'est à ce geste (et
        # nettoyé par lui) qu'une fois tous les refus passés.
        self._overlay_layer = overlay
        self._steps = [s for s in chain_of(entry) if s.get("when") is None or params.get(s["when"]) is not None]
        # La première étape inconditionnelle est traduite dès maintenant, sur la couche d'entrée.
        first = next(s for s in self._steps if s.get("when") is None)
        # Un paramètre invalide est refusé avant tout lancement, même quand une reprojection ouvre la
        # chaîne. `out` n'est à ce geste (et nettoyé par lui) qu'une fois tous les refus passés.
        self._step_parameters(first, layer, first is self._steps[-1], out)
        self._out = out
        self._index = 0
        return self._run_step(layer)

    def _step_parameters(self, step, source, final, out):
        """Paramètres de l'algorithme de l'étape, sur la couche `source` ; seule la dernière étape
        écrit `out`, les autres une sortie temporaire de QGIS."""
        extra = (self._backend,) if step.get("with_backend") else ()
        parameters = step["translate"](self._params or {}, source, *extra)
        if self._overlay_layer is not None and step.get("overlay_key"):
            parameters[step["overlay_key"]] = self._overlay_layer
        if self._mounted is not None:
            # Genre `layout` : la mise en page montée, par son nom, et les seules couches locales.
            spec = self._entry["layout"]
            parameters[spec["name_key"]] = self._mounted.name
            parameters[spec["layers_key"]] = list(self._mounted.layers)
        parameters[step.get("output", "OUTPUT")] = out if final else _TEMPORARY
        return parameters

    def _begin_layout(self, params, coverage):
        """Genre `layout` : refus avant tout lancement (titre vide, couverture sans entité), puis
        montage de la mise en page, ajoutée au projet jusqu'à `_finish`."""
        backend = self._backend
        title = params.get(self._entry["layout"]["title"])
        if not isinstance(title, str) or not title.strip():
            raise GestureError(LAYOUT_BAD_PARAMS, "Le titre de l'atlas ne doit pas être vide.")
        if backend.feature_count(coverage) == 0:
            raise GestureError(
                LAYER_EMPTY,
                "La couche de couverture « %s » n'a aucune entité : l'atlas n'aurait aucune page."
                % backend.name_of(coverage),
            )
        field = params.get(self._entry["field"]) if self._entry.get("field") else None
        self._mounted = backend.mount_atlas(coverage, title, field or None)

    def _run_step(self, source):
        """Lance l'étape `self._index` de la chaîne sur la couche `source`."""
        step = self._steps[self._index]
        final = self._index == len(self._steps) - 1
        parameters = self._step_parameters(step, source, final, self._out)
        extra = {}
        if final and self._entry.get("encoding"):
            extra["encoding"] = self._entry["encoding"]
        if self._handle is not None:
            extra["previous"] = self._handle  # garde en vie la couche temporaire de l'étape d'avant
        return self._backend.run_algorithm(step["algorithm"], parameters, self._on_finished, **extra)

    def _check_work_crs(self, value, layer):
        """Système de calcul donné par le noyau : entier, connu de QGIS, en mètres."""
        backend = self._backend
        if isinstance(value, bool) or not isinstance(value, int) or value <= 0:
            raise GestureError(BAD_PARAMS, "Le système de calcul doit être un code EPSG entier positif.")
        if not backend.crs_is_known(value):
            raise GestureError(UNKNOWN_CRS, "Système inconnu de QGIS : EPSG:%d." % value)
        if not backend.epsg_is_metric(value):
            raise GestureError(
                CRS_NOT_METRIC,
                "La couche « %s » est en %s et le calcul serait fait en EPSG:%d, dont l'unité n'est "
                "pas le mètre : reprojetez-la d'abord dans un système en mètres (par exemple EPSG:2154)."
                % (backend.name_of(layer), backend.crs_of(layer), value),
            )
        return value

    def _begin_raster(self, params, out):
        """Genre d'entrée `raster` : un fichier désigné par son chemin, jamais une couche."""
        entry, backend = self._entry, self._backend
        path = params.get(entry["input"])
        if entry.get("layer_output"):
            # Entrée raster, sortie couche : mêmes gardes qu'un raster lu par une analyse de polygones, et
            # avant toute ouverture du fichier (le réglage « pas de .aux.xml » doit précéder la première).
            self._check_raster_input(path, None)
        if not isinstance(path, str) or not path or not os.path.isfile(path):
            raise GestureError(NOT_FOUND, "Aucun fichier raster « %s »." % _file_name(path))
        source = backend.raster_source(path)
        if source is None:
            raise GestureError(
                INVALID_SOURCE, "QGIS ne sait pas lire « %s » comme un raster." % _file_name(path)
            )
        assign = params.get(entry["assign_crs"]) if entry.get("assign_crs") else None
        if assign is not None:
            if isinstance(assign, bool) or not isinstance(assign, int) or assign <= 0:
                raise GestureError(BAD_PARAMS, "Le système à déclarer doit être un code EPSG entier positif.")
            code = assign
            if not backend.crs_is_known(code):
                raise GestureError(UNKNOWN_CRS, "Système inconnu de QGIS : EPSG:%d." % code)
            current = backend.raster_crs(source)
            if current is not None and not backend.same_crs(source, code):
                raise GestureError(
                    BAD_PARAMS,
                    "« %s » a déjà un système (%s) : on ne déclare un système qu'à un raster qui "
                    "n'en a pas, et Cassini ne reprojette pas un raster." % (_file_name(path), current),
                )
        step = chain_of(entry)[-1]
        target = None
        if entry.get("reproject_to"):
            # Reprojection dictée par le noyau : vers un système en mètres, jamais par identité (RA06).
            target = params.get(entry["reproject_to"])
            if isinstance(target, bool) or not isinstance(target, int) or target <= 0:
                raise GestureError(BAD_PARAMS, "Le système de calcul doit être un code EPSG entier positif.")
            if not backend.crs_is_known(target):
                raise GestureError(UNKNOWN_CRS, "Système inconnu de QGIS : EPSG:%d." % target)
            if not backend.epsg_is_metric(target):
                raise GestureError(
                    CRS_NOT_METRIC, "Le système de calcul EPSG:%d n'a pas le mètre pour unité." % target
                )
            current = backend.raster_crs(source)
            if current is None:
                raise GestureError(
                    BAD_PARAMS,
                    "« %s » n'a pas de système de coordonnées : déclarez-le d'abord (data.convert_raster)."
                    % _file_name(path),
                )
            if backend.same_crs(source, target):
                raise GestureError(
                    BAD_PARAMS,
                    "« %s » est déjà en EPSG:%d : un raster n'est jamais reprojeté vers son propre système."
                    % (_file_name(path), target),
                )
        # La source n'est plus tenue ouverte pendant le travail de QGIS (Windows garderait le
        # fichier d'origine verrouillé).
        source = None
        algorithm_params = step["translate"](params, path)
        algorithm_params[step.get("output", "OUTPUT")] = out
        self._out = out
        return backend.run_algorithm(step["algorithm"], algorithm_params, self._on_finished)

    def _check_raster_input(self, path, layer):
        """Raster lu en entrée d'une analyse de polygones : fichier GeoTIFF existant et lisible, une seule
        bande, un système de coordonnées. Un système différent de celui de la couche n'est pas refusé : QGIS
        reprojette lui-même les polygones vers celui du raster (constat de la sonde, résultats exacts).
        Libéré aussitôt : QGIS le rouvrira seul pour l'algorithme."""
        backend = self._backend
        if not isinstance(path, str) or not path or not os.path.isfile(path):
            raise GestureError(NOT_FOUND, "Aucun fichier raster « %s »." % _file_name(path))
        if not self._pam:
            self._pam = _pam_off()  # avant toute ouverture du raster, ici comme dans l'algorithme
        if os.path.splitext(path)[1].lower() not in _RASTER_SUFFIXES:
            raise GestureError(
                INVALID_SOURCE, "« %s » n'est pas un GeoTIFF (.tif) : ouvrez un raster de ce format." % _file_name(path)
            )
        source = backend.raster_source(path)
        if source is None:
            raise GestureError(
                INVALID_SOURCE, "QGIS ne sait pas lire « %s » comme un raster." % _file_name(path)
            )
        try:
            if backend.raster_band_count(source) != 1:
                raise GestureError(
                    BAD_PARAMS,
                    "« %s » a plusieurs bandes : cette commande ne porte que sur un raster à une bande."
                    % _file_name(path),
                )
            current = backend.raster_crs(source)
            if current is None:
                raise GestureError(
                    BAD_PARAMS,
                    "« %s » n'a pas de système de coordonnées : déclarez-le d'abord (data.convert_raster)."
                    % _file_name(path),
                )
        finally:
            source = None  # le fichier d'origine n'est plus tenu ouvert (Windows le verrouillerait)

    def _check_network(self, params, layer):
        """Champ de vitesse numérique et champ de sens présents dans la couche, avant tout lancement."""
        network = _network_of(params)
        _network_speed(network)
        _network_tolerance_meters(network)
        if network.get("speed_field"):
            self._check_numeric_field(network["speed_field"], layer)
        direction = network.get("direction")
        if direction:
            self._check_field(direction.get("field") if isinstance(direction, dict) else None, layer)

    def _check_origin(self, origin, layer, network=None, work_epsg=None, label="L'origine"):
        backend = self._backend
        snap = _network_tolerance_meters(network if isinstance(network, dict) else {})
        try:
            x, y, epsg = float(origin["x"]), float(origin["y"]), int(origin["epsg"])
        except (KeyError, TypeError, ValueError):
            raise GestureError(BAD_PARAMS, "%s doit être de la forme {x, y, epsg}." % label)
        # Réseau en degrés : l'accrochage se vérifie en mètres, dans `work_epsg` (origine et
        # tronçons transformés par QGIS), jamais avec un rayon en mètres sur des degrés.
        extra = {"work_epsg": work_epsg} if work_epsg is not None else {}
        if not backend.has_feature_near(layer, x, y, epsg, snap, **extra):
            raise GestureError(
                ORIGIN_OFF_NETWORK,
                "%s est à plus de %d m du réseau « %s » : vérifiez ses coordonnées et son système."
                % (label, snap, backend.name_of(layer)),
            )

    def _overlay(self, params, layer_id):
        """Seconde couche vecteur, distincte de l'entrée."""
        backend = self._backend
        overlay_id = params.get(self._entry["overlay"])
        if overlay_id == layer_id:
            raise GestureError(BAD_PARAMS, "Les deux couches doivent être différentes.")
        overlay = backend.layer(overlay_id) if isinstance(overlay_id, str) and overlay_id else None
        if overlay is None:
            raise GestureError(NOT_FOUND, "Aucune couche du projet ne porte cet identifiant.")
        if not backend.is_vector(overlay):
            raise GestureError(
                NOT_VECTOR, "La couche « %s » n'est pas une couche vecteur." % backend.name_of(overlay)
            )
        return overlay

    def _check_field(self, name, layer):
        backend = self._backend
        if not isinstance(name, str) or name not in [n for n, _ in backend.fields_of(layer)]:
            raise GestureError(
                UNKNOWN_FIELD,
                "La couche « %s » n'a pas de champ « %s »." % (backend.name_of(layer), name),
            )

    def _check_numeric_field(self, name, layer):
        """Champ de `layer` qui doit exister et être numérique (pondération)."""
        backend = self._backend
        types = dict(backend.fields_of(layer))
        if not isinstance(name, str) or name not in types:
            raise GestureError(
                UNKNOWN_FIELD,
                "La couche « %s » n'a pas de champ « %s »." % (backend.name_of(layer), name),
            )
        if not is_numeric_type(types[name]):
            raise GestureError(
                BAD_PARAMS,
                "Le champ « %s » de la couche « %s » n'est pas numérique : choisissez un champ de nombres."
                % (name, backend.name_of(layer)),
            )

    def _on_finished(self, ok, results):
        if self._done is None:  # annulé entre-temps : le geste a déjà répondu
            return
        if self._cancelling:  # la tâche est arrêtée : plus rien ne tient le fichier
            self._clean()
            self._finish(error=GestureError(CANCELLED, "Geste annulé."))
            return
        try:
            if not ok:
                failure = self._steps[self._index].get("on_failure")
                if failure:
                    raise GestureError(*failure)
                raise GestureError(FAILED, "L'analyse a échoué dans QGIS.")
            if self._index + 1 < len(self._steps):
                self._advance(results)
                return
            self._results = results
            self._finish(result=self._load())
        except GestureError as exc:
            self._fail(exc)
        except Exception:
            self._fail(GestureError(FAILED, "Échec inattendu de l'analyse."))

    def _advance(self, results):
        """Lance l'étape suivante sur la sortie temporaire de celle qui vient de finir."""
        backend = self._backend
        current, following = self._steps[self._index], self._steps[self._index + 1]
        reference = results.get(current.get("output", "OUTPUT"))
        temporary = backend.temporary_layer(self._handle, reference) if reference else None
        if temporary is None:
            raise GestureError(FAILED, current.get("nothing", "L'étape intermédiaire de l'analyse n'a rien rendu."))
        empty = (following.get("guard") or {}).get("empty")
        if empty and backend.feature_count(temporary) == 0:
            code, message = empty
            raise GestureError(code, message)
        self._index += 1
        self._handle = self._run_step(temporary)

    def _load(self):
        backend = self._backend
        if self._entry.get("file_result"):
            return self._written()
        if self._entry.get("table"):
            return self._entry["table"](backend, self._out, self._params or {})
        if self._entry.get("layout"):
            return self._exported_layout()
        if self._entry.get("raster") and not self._entry.get("layer_output"):
            epsg = backend.read_written_raster(self._out)
            if epsg is False:
                raise GestureError(FAILED, "Le GeoTIFF écrit est illisible.")
            return {"file": self._out, "epsg": epsg}
        name = os.path.splitext(os.path.basename(self._out))[0]  # radical donné par le noyau (AD-10)
        layer = backend.load_output(self._out, name)
        if layer is None:
            raise GestureError(FAILED, "La couche produite est illisible.")
        self._loaded = True
        result = {
            "layer_id": backend.id_of(layer),
            "name": backend.name_of(layer),
            "feature_count": backend.feature_count(layer),
            "crs": backend.crs_of(layer),
        }
        summary = self._entry.get("summary")
        if summary is not None:
            extra = (self._results,) if self._entry.get("summary_results") else ()
            result.update(summary(backend, layer, self._params or {}, *extra))
        return result

    def _exported_layout(self):
        """Résultat du genre `layout` : le PDF écrit (jamais ajouté au projet) et ce que la mise en
        page a dessiné ; un fichier absent ou vide est un échec nommé."""
        mounted = self._mounted
        if not os.path.isfile(self._out) or os.path.getsize(self._out) == 0:
            raise GestureError(LAYOUT_EXPORT_FAILED, "L'export de l'atlas a échoué : aucun fichier n'a été écrit.")
        return {
            "file": self._out,
            "page_count": mounted.page_count,
            "layer_count": len(mounted.layers),
            "skipped_layers": list(mounted.skipped),
            "sources": list(mounted.names[:MAX_SOURCES]),
            "truncated": len(mounted.names) > MAX_SOURCES,
        }

    def _written(self):
        """Résultat d'une sortie fichier : relue à part, jamais ajoutée au projet."""
        fmt = (self._params or {}).get(self._entry["file_result"])
        read = self._backend.read_written(self._out)
        if read is None:
            raise GestureError(FAILED, "Le fichier écrit est illisible.")
        count, driver = read
        if driver != _FORMAT_DRIVERS.get(fmt):
            raise GestureError(
                FORMAT_MISMATCH,
                "Le fichier écrit n'est pas au format demandé (%s) : QGIS l'a lu comme %s." % (fmt, driver),
            )
        return {"file": self._out, "format": fmt, "feature_count": count}

    def _fail(self, error):
        # Une couche déjà ajoutée au projet garde son fichier.
        if not self._loaded:
            self._clean()
        self._finish(error=error)

    def _clean(self):
        # L'écriture est celle de ce geste : un échec ne laisse pas de fichier partiel.
        if self._out:
            _remove(self._out)

    def _finish(self, result=None, error=None):
        mounted, self._mounted = self._mounted, None
        if mounted is not None:
            with contextlib.suppress(Exception):
                mounted.remove()  # la mise en page n'est au projet que le temps de l'appel
        if self._pam:
            self._pam = False
            _pam_restore()
        done, self._done = self._done, None
        if done is not None:
            done(result=result, error=error)


def register_all(registry, table=None, backend_factory=None):
    """Enregistre chaque commande de la table dans le registre des gestes."""
    for command_id, entry in (ALGORITHMS if table is None else table).items():
        registry.register(
            command_id,
            lambda e=entry: ProcessingGesture(e, backend_factory() if backend_factory else None),
        )
