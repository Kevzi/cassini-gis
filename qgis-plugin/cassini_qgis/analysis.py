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
UNKNOWN_CRS = "data.unknown_crs"
FORMAT_MISMATCH = "data.format_mismatch"
INVALID_SOURCE = "layer.invalid_source"

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


# Vitesses fixes du geste, en km/h (pas de champ de vitesse du réseau).
_SPEEDS_KMH = {"walk": 5.0, "car": 50.0}
_UNIT_TO_SECONDS = {"s": 1.0, "min": 60.0}
_COST_SECONDS_PER_UNIT = 3600.0  # le coût « plus rapide » de l'algorithme est en heures (vérifié par la sonde)
_ORIGIN_SNAP_METERS = 100.0  # distance maximale entre l'origine et le réseau (contrôlée avant de lancer)


def _duration_in_seconds(duration):
    """Durée `{value, unit}` en secondes ; le noyau a déjà vérifié qu'elle est positive."""
    try:
        seconds = float(duration["value"]) * _UNIT_TO_SECONDS[duration["unit"]]
    except (KeyError, TypeError, ValueError):
        raise GestureError(BAD_PARAMS, "La durée doit être de la forme {value, unit}, en s ou min.")
    if not seconds > 0:
        raise GestureError(BAD_PARAMS, "La durée doit être strictement positive.")
    return seconds


def translate_isochrone(params, layer):
    """Paramètres neutres de `analysis.isochrone` -> paramètres de `native:serviceareafrompoint`.

    Stratégie « plus rapide », vitesse du mode, coût en secondes ; l'origine porte son SCR,
    que QGIS reprojette dans celui du réseau. Aucun champ de sens ni de vitesse : tous les
    tronçons sont parcourus dans les deux sens, à la même vitesse.
    """
    origin = params.get("origin")
    try:
        x, y, epsg = float(origin["x"]), float(origin["y"]), int(origin["epsg"])
    except (KeyError, TypeError, ValueError):
        raise GestureError(BAD_PARAMS, "L'origine doit être de la forme {x, y, epsg}.")
    speed = _SPEEDS_KMH.get(params.get("mode"))
    if speed is None:
        raise GestureError(BAD_PARAMS, "Le mode doit être « walk » (à pied) ou « car » (en voiture).")
    return {
        "INPUT": layer,
        "STRATEGY": 1,  # plus rapide
        "DIRECTION_FIELD": "",
        "DEFAULT_DIRECTION": 2,  # deux sens
        "SPEED_FIELD": "",
        "DEFAULT_SPEED": speed,
        "TOLERANCE": 0,
        "START_POINT": "%r,%r [EPSG:%d]" % (x, y, epsg),
        "TRAVEL_COST2": _duration_in_seconds(params.get("duration")) / _COST_SECONDS_PER_UNIT,
        "INCLUDE_BOUNDS": False,
    }


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


# Champ de décompte écrit par `native:countpointsinpolygon` ; le noyau ne le connaît pas.
_COUNT_FIELD = "NUMPOINTS"
_MAX_ZONES = 100  # borne du schéma de sortie du noyau (AD-8)


def translate_zonal_statistics(params, layer):
    """Paramètres neutres de `analysis.zonal_statistics` -> `native:countpointsinpolygon`.

    La couche de points est ajoutée par le geste (clé `overlay` de la table).
    """
    return {"POLYGONS": layer, "WEIGHT": "", "CLASSFIELD": "", "FIELD": _COUNT_FIELD}


def summarize_zonal_statistics(backend, layer, params):
    """Décomptes des 100 premières zones de la couche produite (clé `summary` de la table)."""
    pairs, truncated = backend.counts_of(layer, _COUNT_FIELD, params.get("label_field") or None, _MAX_ZONES)
    return {"zones": [{"label": label, "count": count} for label, count in pairs], "truncated": truncated}


# Table fermée : id de commande -> algorithme et traduction.
#   algorithm : id de l'algorithme du cadre de traitement ;
#   translate : (params du noyau, couche d'entrée) -> paramètres de l'algorithme, sans la sortie ;
#   output    : nom du paramètre de sortie de l'algorithme ;
#   input     : nom du paramètre du noyau qui désigne la couche d'entrée ;
#   metric    : l'algorithme mesure en unités de la couche, donc le SCR doit être en mètres ;
#   overlay   : (facultatif) paramètre du noyau qui désigne une seconde couche, et clé de
#               l'algorithme qui la reçoit (`overlay_key`) ;
#   field     : (facultatif) paramètre du noyau qui nomme un champ de la couche d'entrée ;
#               avec `field_optional`, il peut être absent ;
#   geometry  : (facultatif) type de géométrie exigé de la couche d'entrée (ex. `Polygon`) ;
#   origin    : (facultatif) paramètre du noyau `{x, y, epsg}` qui doit tomber à moins de
#               `_ORIGIN_SNAP_METERS` d'une entité de la couche d'entrée (sinon
#               `analysis.origin_off_network`, avant tout lancement) ;
#   then      : (facultatif) étape suivante, sur la sortie temporaire de l'algorithme : dict
#               {algorithm, translate(params, sortie précédente), output, empty_error} ;
#               seule cette dernière étape écrit `out`, la première écrit en sortie
#               temporaire ; `empty_error` (code, message) si la sortie de la première
#               étape n'a aucune entité ;
#   summary   : (facultatif) (fond, couche produite, params) -> dict qui complète le résultat ;
#   target_crs: (facultatif) paramètre du noyau qui porte un code EPSG d'arrivée ; le fond
#               doit le connaître (`crs_is_known`), sinon `data.unknown_crs`, avant tout
#               lancement ;
#   file_result: (facultatif) paramètre du noyau qui nomme le format écrit : la sortie est un
#               fichier, relu à part pour compter ses entités mais jamais ajouté au projet ;
#               le résultat est `file`, `format`, `feature_count` ; le fichier relu doit
#               être du pilote du format (`data.format_mismatch` sinon) ; un Shapefile
#               demande une couche d'un seul type de géométrie, vérifié avant tout lancement ;
#   encoding  : (facultatif) encodage des textes écrits, posé sur le contexte de traitement
#               de cette seule commande ;
#   raster    : (facultatif) genre d'entrée `raster` (AD-24) : `input` nomme un `RasterPath`,
#               chemin absolu donné par le noyau (AD-9), et non un identifiant de couche ; le
#               fichier doit exister et se lire en raster, sinon `layer.not_found` ou
#               `layer.invalid_source` ; la sortie est un GeoTIFF, relu à part, jamais ajouté au
#               projet (le noyau dicte `layer.open_raster`) ; le résultat est `file` et `epsg` ;
#   assign_crs: (facultatif, avec `raster`) paramètre du noyau qui déclare le code EPSG d'une
#               source sans système : connu de QGIS (`data.unknown_crs` sinon), et refusé
#               (`analysis.bad_params`) si la source a déjà un autre système ;
#   work_crs  : (facultatif) paramètre du noyau qui porte le système de calcul en mètres, que le
#               noyau ne remplit que pour une couche en degrés (changement de cap du 2026-11-12,
#               amendement RA06 d'AD-24). Présent : inconnu de QGIS, `data.unknown_crs` ; pas en
#               mètres, `analysis.crs_not_metric`, avant tout lancement ; la chaîne commence alors
#               par `native:reprojectlayer` vers ce système, en sortie temporaire, et l'algorithme
#               de la ligne travaille sur la couche reprojetée, vide ou non. Absent : l'algorithme
#               seul, avec la garde `metric`, comme avant. Le choix est dicté par le paramètre du
#               noyau ; le geste ne calcule ni ne choisit aucun système (AD-5, AD-24).
ALGORITHMS = {
    "analysis.buffer": {
        "algorithm": "native:buffer",
        "translate": translate_buffer,
        "output": "OUTPUT",
        "input": "layer_id",
        "metric": True,
        "work_crs": "work_epsg",
    },
    "analysis.intersect": {
        "algorithm": "native:intersection",
        "translate": translate_intersect,
        "output": "OUTPUT",
        "input": "layer_id",
        "overlay": "overlay_layer_id",
        "overlay_key": "OVERLAY",
    },
    "analysis.select_by_attribute": {
        "algorithm": "native:extractbyattribute",
        "translate": translate_select_by_attribute,
        "output": "OUTPUT",
        "input": "layer_id",
        "field": "field",
    },
    "analysis.zonal_statistics": {
        "algorithm": "native:countpointsinpolygon",
        "translate": translate_zonal_statistics,
        "output": "OUTPUT",
        "input": "layer_id",
        "overlay": "points_layer_id",
        "overlay_key": "POINTS",
        "field": "label_field",
        "field_optional": True,
        "geometry": "Polygon",
        "summary": summarize_zonal_statistics,
    },
    "analysis.isochrone": {
        "algorithm": "native:serviceareafrompoint",
        "translate": translate_isochrone,
        "output": "OUTPUT_LINES",
        "input": "layer_id",
        "metric": True,
        "geometry": "Line",
        "origin": "origin",
        "then": {
            "algorithm": "native:convexhull",
            "translate": translate_convex_hull,
            "output": "OUTPUT",
            "empty_error": (ORIGIN_OFF_NETWORK, "Aucun tronçon du réseau n'est atteint depuis cette origine : "
                            "vérifiez qu'elle se trouve sur le réseau choisi."),
        },
    },
    "data.reproject": {
        "algorithm": "native:reprojectlayer",
        "translate": translate_reproject,
        "output": "OUTPUT",
        "input": "layer_id",
        "target_crs": "target_epsg",
    },
    "data.convert": {
        "algorithm": "native:extractbyexpression",
        "translate": translate_convert,
        "output": "OUTPUT",
        "input": "layer_id",
        "file_result": "format",
        "encoding": "UTF-8",
    },
    "data.convert_raster": {
        "algorithm": "gdal:warpreproject",
        "translate": translate_convert_raster,
        "output": "OUTPUT",
        "input": "raster",
        "raster": True,
        "assign_crs": "assign_epsg",
    },
}


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

    def has_feature_near(self, layer, x, y, epsg, radius):
        """Vrai si une entité de la couche passe à `radius` (unités de la couche) du point."""
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
        if source != layer.crs():
            point = QgsCoordinateTransform(source, layer.crs(), QgsProject.instance()).transform(point)
        around = QgsRectangle(point.x() - radius, point.y() - radius, point.x() + radius, point.y() + radius)
        target = QgsGeometry.fromPointXY(point)
        request = QgsFeatureRequest().setFilterRect(around)
        return any(f.geometry().distance(target) <= radius for f in layer.getFeatures(request))

    def temporary_layer(self, handle, reference):
        """Couche temporaire écrite par l'étape que `handle` vient de finir ; None si absente."""
        from qgis.core import QgsProcessingUtils

        return QgsProcessingUtils.mapLayerFromString(reference, handle.context)

    def counts_of(self, layer, count_field, label_field, limit):
        """Couples (étiquette, décompte) des `limit` premières entités, et si la couche en a plus.

        L'étiquette est la valeur de `label_field`, ou le numéro d'entité si le champ est
        absent, vide ou nul.
        """
        from qgis.core import QgsFeatureRequest

        request = QgsFeatureRequest()
        request.setLimit(limit)
        pairs = []
        for feature in layer.getFeatures(request):
            label = feature[label_field] if label_field else None
            text = "" if label is None else str(label).strip()
            if text in ("", "NULL"):
                text = str(feature.id())
            count = feature[count_field]
            pairs.append((text, int(count) if count is not None else 0))
        return pairs, self.feature_count(layer) > limit

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
# Stade de la reprojection qui ouvre une ligne `work_crs` ; 0 et 1 sont l'algorithme de la ligne
# et l'étape `then`.
_WORK_CRS_STAGE = "work_crs"
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
        self._stage = 0  # `_WORK_CRS_STAGE` : reprojection ; 0 : algorithme de la ligne ; 1 : `then`
        self._overlay_layer = None

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
            work_epsg = self._check_work_crs(work_epsg, layer)
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
        if entry.get("origin"):
            self._check_origin(params.get(entry["origin"]), layer)
        overlay = None
        if entry.get("overlay"):
            overlay = self._overlay(params, layer_id)
        if entry.get("field") and not (entry.get("field_optional") and not params.get(entry["field"])):
            self._check_field(params.get(entry["field"]), layer)
        # Traduits dès maintenant, sur la couche d'entrée : un paramètre invalide est refusé avant
        # tout lancement, même quand une reprojection ouvre la chaîne. `out` n'est à ce geste (et
        # nettoyé par lui) qu'une fois tous les refus passés.
        algorithm_params = self._line_parameters(layer, overlay, out)
        self._out = out
        if work_epsg is not None:
            self._overlay_layer = overlay
            self._stage = _WORK_CRS_STAGE
            reprojection = translate_work_crs(work_epsg, layer)
            reprojection["OUTPUT"] = _TEMPORARY
            return backend.run_algorithm(_WORK_CRS_ALGORITHM, reprojection, self._on_finished)
        return self._run_line(algorithm_params)

    def _line_parameters(self, source, overlay, out):
        """Paramètres de l'algorithme de la ligne, sur la couche `source`, vers `out`."""
        entry = self._entry
        algorithm_params = entry["translate"](self._params or {}, source)
        if overlay is not None:
            algorithm_params[entry["overlay_key"]] = overlay
        # Avec une étape suivante, la première écrit en sortie temporaire : un seul fichier, `out`.
        algorithm_params[entry["output"]] = _TEMPORARY if entry.get("then") else out
        return algorithm_params

    def _run_line(self, algorithm_params, previous=None):
        entry = self._entry
        extra = {"encoding": entry["encoding"]} if entry.get("encoding") else {}
        if previous is not None:
            extra["previous"] = previous
        return self._backend.run_algorithm(entry["algorithm"], algorithm_params, self._on_finished, **extra)

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
        # La source n'est plus tenue ouverte pendant le travail de QGIS (Windows garderait le
        # fichier d'origine verrouillé).
        source = None
        algorithm_params = entry["translate"](params, path)
        algorithm_params[entry["output"]] = out
        self._out = out
        return backend.run_algorithm(entry["algorithm"], algorithm_params, self._on_finished)

    def _check_origin(self, origin, layer):
        backend = self._backend
        try:
            x, y, epsg = float(origin["x"]), float(origin["y"]), int(origin["epsg"])
        except (KeyError, TypeError, ValueError):
            raise GestureError(BAD_PARAMS, "L'origine doit être de la forme {x, y, epsg}.")
        if not backend.has_feature_near(layer, x, y, epsg, _ORIGIN_SNAP_METERS):
            raise GestureError(
                ORIGIN_OFF_NETWORK,
                "L'origine est à plus de %d m du réseau « %s » : vérifiez ses coordonnées et son système."
                % (_ORIGIN_SNAP_METERS, backend.name_of(layer)),
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

    def _on_finished(self, ok, results):
        if self._done is None:  # annulé entre-temps : le geste a déjà répondu
            return
        if self._cancelling:  # la tâche est arrêtée : plus rien ne tient le fichier
            self._clean()
            self._finish(error=GestureError(CANCELLED, "Geste annulé."))
            return
        try:
            if not ok:
                raise GestureError(FAILED, "L'analyse a échoué dans QGIS.")
            if self._stage == _WORK_CRS_STAGE:
                self._after_reprojection(results)
                return
            then = self._entry.get("then")
            if then is not None and self._stage == 0:
                self._chain(then, results)
                return
            self._finish(result=self._load())
        except GestureError as exc:
            self._fail(exc)
        except Exception:
            self._fail(GestureError(FAILED, "Échec inattendu de l'analyse."))

    def _after_reprojection(self, results):
        """Lance l'algorithme de la ligne sur la couche reprojetée, vide ou non."""
        backend = self._backend
        reference = results.get("OUTPUT")
        temporary = backend.temporary_layer(self._handle, reference) if reference else None
        if temporary is None:
            raise GestureError(FAILED, "La reprojection de la couche d'entrée n'a rien rendu.")
        parameters = self._line_parameters(temporary, self._overlay_layer, self._out)
        self._stage = 0
        self._handle = self._run_line(parameters, previous=self._handle)

    def _chain(self, then, results):
        """Lance l'étape suivante sur la sortie temporaire de la première."""
        backend = self._backend
        reference = results.get(self._entry["output"])
        temporary = backend.temporary_layer(self._handle, reference) if reference else None
        if temporary is None or backend.feature_count(temporary) == 0:
            code, message = then["empty_error"]
            raise GestureError(code, message)
        parameters = then["translate"](self._params or {}, temporary)
        parameters[then["output"]] = self._out
        self._stage = 1
        self._handle = backend.run_algorithm(then["algorithm"], parameters, self._on_finished, self._handle)

    def _load(self):
        backend = self._backend
        if self._entry.get("file_result"):
            return self._written()
        if self._entry.get("raster"):
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
            result.update(summary(backend, layer, self._params or {}))
        return result

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
