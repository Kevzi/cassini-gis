"""Sonde manuelle sous le vrai QGIS : matrice de distances (entrée 10.36, `analysis.distance_matrix`).

À lancer avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`) :

    "C:\\Program Files\\QGIS 3.44.15\\bin\\python-qgis-ltr.bat" qgis-plugin/tests/probe_distance_matrix.py

Aucun noyau ni aucune donnée hors dépôt : des couches mémoire synthétiques autour d'Annecy. La chaîne
passe par `qgis:distancematrix` (fournisseur « qgis » du module Processing, chargé ici comme QGIS le
charge). Le geste reçoit `work_epsg` comme le noyau le lui donne. Termine par `RÉSULTAT : OK` ou `ÉCHEC`.

Ce que la sonde établit (les noms de colonne et les comportements ne sont jamais supposés) :
1. Départ ET cible en EPSG:4326, `work_epsg` = 2154 : une ligne par paire (`pair_count` = N x M), le
   tableau CSV porte `InputID`, `TargetID`, `Distance`, et chaque distance est en mètres, comparée à la
   distance ellipsoïdale (`QgsDistanceArea`, WGS84) à 5 m près plus 0,2 % (altération du Lambert-93).
2. Les lignes affichées sont triées par distance, départ après départ.
3. `neighbors` = 1 : une seule cible par départ, la plus proche.
4. Trois cibles à égale distance et `neighbors` = 1 : le nombre de lignes que QGIS rend (constat).
5. Départ en EPSG:2154 et cible en EPSG:4326, sans `work_epsg` : mêmes distances (QGIS transforme la cible).
6. Une couche de multipoints, une couche de polygones : refusées avant tout fichier écrit.
7. 400 x 300 = 120 000 paires sans `neighbors` : refusées avant tout lancement ; avec `neighbors`, acceptées.
8. 12 x 12 = 144 paires : 100 lignes dans le résultat, `truncated` vrai, le fichier garde les 144.
9. Le tableau n'est jamais ajouté au projet, et ne porte que les trois colonnes.
"""

import os
import sys
import tempfile
import traceback

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "qgis-plugin", "cassini_qgis"))

from qgis.analysis import QgsNativeAlgorithms  # noqa: E402
from qgis.core import (  # noqa: E402
    QgsApplication,
    QgsCoordinateReferenceSystem,
    QgsCoordinateTransform,
    QgsDistanceArea,
    QgsFeature,
    QgsGeometry,
    QgsPointXY,
    QgsProject,
    QgsVectorLayer,
)
from qgis.PyQt.QtCore import QCoreApplication, QTimer  # noqa: E402

app = QgsApplication([], False)
QgsApplication.initQgis()
QgsApplication.processingRegistry().addProvider(QgsNativeAlgorithms())
# Le fournisseur « qgis » (distancematrix) est celui du module Processing de QGIS.
plugins = os.path.join(QgsApplication.prefixPath(), "python", "plugins")
if os.path.isdir(plugins) and plugins not in sys.path:
    sys.path.append(plugins)
from processing.core.Processing import Processing  # noqa: E402

Processing.initialize()
import analysis  # noqa: E402

TOLERANCE_M = 5.0
ENTRY = analysis.ALGORITHMS["analysis.distance_matrix"]
# Départs et cibles autour d'Annecy (longitude, latitude).
STARTS = [("a", 6.1222, 45.9013), ("b", 6.1500, 45.9100), ("c", 5.9440, 45.8676)]
TARGETS = [("g1", 6.1262, 45.9013), ("g2", 6.1500, 45.9140)]

work = tempfile.mkdtemp(prefix="cassini-matrice-")
project = QgsProject.instance()
project.setCrs(QgsCoordinateReferenceSystem("EPSG:2154"))
checks = []
outcomes = {}


def check(label, condition, detail=""):
    checks.append((label, bool(condition)))
    print(("ok     " if condition else "ECHEC  ") + label + (" : %s" % (detail,) if detail and not condition else ""))


def finish():
    QTimer.singleShot(0, QCoreApplication.quit)


def protect(fn):
    def wrapper(*args, **kwargs):
        try:
            fn(*args, **kwargs)
        except Exception:
            traceback.print_exc()
            check("aucune exception dans la sonde", False)
            finish()

    return wrapper


def memory_layer(name, rows, epsg, kind="Point"):
    """Couche mémoire de points `name` ; `rows` : (nom, lon, lat), transformés vers `epsg`."""
    layer = QgsVectorLayer("%s?crs=EPSG:%d&field=name:string" % (kind, epsg), name, "memory")
    transform = None
    if epsg != 4326:
        transform = QgsCoordinateTransform(
            QgsCoordinateReferenceSystem("EPSG:4326"), QgsCoordinateReferenceSystem("EPSG:%d" % epsg), project
        )
    features = []
    for row in rows:
        point = QgsPointXY(row[1], row[2])
        if transform is not None:
            point = transform.transform(point)
        f = QgsFeature(layer.fields())
        f.setAttribute("name", row[0])
        geometry = QgsGeometry.fromPointXY(point)
        if kind == "MultiPoint":
            geometry = QgsGeometry.fromMultiPointXY([point])
        elif kind == "Polygon":
            geometry = QgsGeometry.fromPointXY(point).buffer(0.001, 4)
        f.setGeometry(geometry)
        features.append(f)
    layer.dataProvider().addFeatures(features)
    layer.updateExtents()
    project.addMapLayer(layer)
    return layer


def planar_layer(name, rows):
    """Couche mémoire de points en EPSG:2154 ; `rows` : (nom, x, y)."""
    layer = QgsVectorLayer("Point?crs=EPSG:2154&field=name:string", name, "memory")
    features = []
    for label, x, y in rows:
        f = QgsFeature(layer.fields())
        f.setAttribute("name", label)
        f.setGeometry(QgsGeometry.fromPointXY(QgsPointXY(x, y)))
        features.append(f)
    layer.dataProvider().addFeatures(features)
    project.addMapLayer(layer)
    return layer


def out(name):
    return os.path.join(work, name)


def gesture(params, label, then):
    def done(result=None, error=None):
        outcomes[label] = (result, error, params.get("out"))
        QTimer.singleShot(0, protect(then))

    g = analysis.ProcessingGesture(ENTRY)
    outcomes["_gesture_" + label] = g  # gardé en vie pendant la tâche
    g.start(params, done)


def base(start, target, **extra):
    params = {"layer_id": start.id(), "label_field": "name", "target_layer_id": target.id(),
              "target_label_field": "name"}
    params.update(extra)
    return params


ellipsoid = QgsDistanceArea()
ellipsoid.setSourceCrs(QgsCoordinateReferenceSystem("EPSG:4326"), project.transformContext())
ellipsoid.setEllipsoid("WGS84")


def expected_distance(start, target):
    s = next(r for r in STARTS if r[0] == start)
    t = next(r for r in TARGETS if r[0] == target)
    return ellipsoid.measureLine(QgsPointXY(s[1], s[2]), QgsPointXY(t[1], t[2]))


def table_of(label):
    """Tableau écrit par le geste `label`, relu du fichier : (colonnes, lignes (départ, cible, distance))."""
    _, _, path = outcomes[label]
    layer = QgsVectorLayer(path, "lecture", "ogr")
    columns = [f.name() for f in layer.fields()]
    rows = [(f[0], f[1], float(f[2])) for f in layer.getFeatures()]
    return columns, rows


def check_distances(label, rows):
    for start, target, distance in rows:
        want = expected_distance(start, target)
        check(
            "%s : distance %s -> %s en mètres (%.0f m attendus, à %.0f m + 0,2 %% près)"
            % (label, start, target, want, TOLERANCE_M),
            abs(distance - want) <= TOLERANCE_M + 0.002 * want,
            distance,
        )


layers_before = len(project.mapLayers())
starts_wgs = memory_layer("departs_wgs", STARTS, 4326)
targets_wgs = memory_layer("cibles_wgs", TARGETS, 4326)
starts_l93 = memory_layer("departs_l93", STARTS, 2154)


# -- 1, 2 et 9. Départ et cible en degrés, `work_epsg` = 2154 ----------------------------------
def step_degrees():
    gesture(base(starts_wgs, targets_wgs, work_epsg=2154, out=out("1-matrice.csv")), "degres", step_degrees_done)


def step_degrees_done():
    result, error, path = outcomes["degres"]
    check("degrés : pas d'erreur", error is None, error and error.message)
    check("degrés : le résultat est file, pair_count, rows, truncated",
          result and sorted(result) == ["file", "pair_count", "rows", "truncated"], result)
    check("degrés : file est le fichier du noyau", result and result["file"] == path, result)
    check("degrés : 3 x 2 = 6 paires, pas tronqué",
          result and result["pair_count"] == 6 and len(result["rows"]) == 6 and result["truncated"] is False, result)
    columns, rows = table_of("degres")
    check("degrés : colonnes du tableau écrit", columns == ["InputID", "TargetID", "Distance"], columns)
    check_distances("degrés", rows)
    labels = [r["label"] for r in result["rows"]] if result else []
    check("degrés : étiquettes « départ → cible »", all(" \u2192 " in label for label in labels), labels)
    check("degrés : unité des distances", result and all(r["values"]["distance"]["unit"] == "m" for r in result["rows"]), result)
    # Triées par distance, départ après départ.
    by_start = {}
    for r in result["rows"]:
        by_start.setdefault(r["label"].split(" \u2192 ")[0], []).append(r["values"]["distance"]["value"])
    check("degrés : lignes triées par distance pour chaque départ",
          all(values == sorted(values) for values in by_start.values()), by_start)
    check("tableau : jamais ajouté au projet", len(project.mapLayers()) == layers_before + 3, len(project.mapLayers()))
    step_neighbors()


# -- 3. Un seul voisin par départ ---------------------------------------------------------------
def step_neighbors():
    gesture(base(starts_wgs, targets_wgs, work_epsg=2154, neighbors=1, out=out("2-voisin.csv")), "voisin", step_neighbors_done)


def step_neighbors_done():
    result, error, _ = outcomes["voisin"]
    check("un voisin : pas d'erreur", error is None, error and error.message)
    columns, rows = table_of("voisin")
    check("un voisin : une ligne par départ", len(rows) == 3 and len({r[0] for r in rows}) == 3, rows)
    nearest = {s: min(TARGETS, key=lambda t: expected_distance(s, t[0]))[0] for s, *_ in STARTS}
    check("un voisin : la cible la plus proche de chaque départ",
          all(nearest[start] == target for start, target, _ in rows), (rows, nearest))
    check_distances("un voisin", rows)
    step_ties()


# -- 4. Égalités ---------------------------------------------------------------------------------
def step_ties():
    entries = planar_layer("entree", [("a", 900000, 6500000)])
    targets = planar_layer("equidistantes", [("c1", 900010, 6500000), ("c2", 899990, 6500000), ("c3", 900000, 6500010)])
    gesture(base(entries, targets, neighbors=1, out=out("3-egalite.csv")), "egalite", step_ties_done)


def step_ties_done():
    result, error, _ = outcomes["egalite"]
    check("égalité : pas d'erreur", error is None, error and error.message)
    _, rows = table_of("egalite")
    print("constat : trois cibles à égale distance, neighbors = 1 : %d ligne(s) rendue(s) par QGIS" % len(rows))
    check("égalité : au plus un voisin gardé, comme demandé", len(rows) <= 1, rows)
    step_metric()


# -- 5. Départ en mètres, cible en degrés, sans `work_epsg` ---------------------------------------
def step_metric():
    gesture(base(starts_l93, targets_wgs, out=out("4-metres.csv")), "metres", step_metric_done)


def step_metric_done():
    result, error, _ = outcomes["metres"]
    check("mètres : pas d'erreur", error is None, error and error.message)
    _, rows = table_of("metres")
    check("mètres : 6 paires", len(rows) == 6, rows)
    check_distances("mètres", rows)
    step_refusals()


# -- 6. Multipoints et polygones --------------------------------------------------------------------
def step_refusals():
    multi = memory_layer("multipoints", STARTS, 2154, kind="MultiPoint")
    polygons = memory_layer("polygones", STARTS, 2154, kind="Polygon")
    outcomes["_multi"], outcomes["_polygons"] = multi, polygons
    gesture(base(multi, targets_wgs, out=out("5-multi.csv")), "multi", step_refusals_polygons)


def step_refusals_polygons():
    result, error, path = outcomes["multi"]
    check("multipoints : refusés avant lancement", error is not None and error.code == "analysis.bad_params",
          error and (error.code, error.message))
    check("multipoints : le message nomme la couche", error is not None and "multipoints" in error.message, error and error.message)
    check("multipoints : aucun fichier écrit", not os.path.exists(path))
    gesture(base(outcomes["_polygons"], targets_wgs, out=out("6-polygones.csv")), "polygones", step_polygons_done)


def step_polygons_done():
    result, error, path = outcomes["polygones"]
    check("polygones : refusés avant lancement", error is not None and error.code == "analysis.bad_params",
          error and (error.code, error.message))
    check("polygones : aucun fichier écrit", not os.path.exists(path))
    step_ceiling()


# -- 7. Plafond de paires ----------------------------------------------------------------------------
def step_ceiling():
    big = planar_layer("grande", [("p%d" % i, 900000 + i, 6500000) for i in range(400)])
    wide = planar_layer("large", [("q%d" % i, 900000 + i, 6500100) for i in range(300)])
    outcomes["_big"], outcomes["_wide"] = big, wide
    gesture(base(big, wide, out=out("7-plafond.csv")), "plafond", step_ceiling_done)


def step_ceiling_done():
    result, error, path = outcomes["plafond"]
    check("plafond : 120 000 paires refusées avant lancement", error is not None and error.code == "analysis.bad_params",
          error and (error.code, error.message))
    check("plafond : le message demande `neighbors`", error is not None and "voisins" in error.message, error and error.message)
    check("plafond : aucun fichier écrit", not os.path.exists(path))
    gesture(base(outcomes["_big"], outcomes["_wide"], neighbors=2, out=out("8-plafond-voisins.csv")), "plafond_voisins", step_ceiling_neighbors_done)


def step_ceiling_neighbors_done():
    result, error, _ = outcomes["plafond_voisins"]
    check("plafond : avec `neighbors`, la matrice est faite", error is None and result and result["pair_count"] == 800,
          error and error.message or result and result["pair_count"])
    step_repeated()


# -- 7 bis. Étiquettes en double avec `neighbors` ------------------------------------------------------
def step_repeated():
    twins = planar_layer("jumeaux", [("x", 900000, 6500000), ("x", 900050, 6500000)])
    gesture(base(twins, outcomes["_wide"], neighbors=2, out=out("10-doubles.csv")), "doubles", step_repeated_done)


def step_repeated_done():
    result, error, path = outcomes["doubles"]
    check("doubles : refus avant lancement", error is not None and error.code == "analysis.bad_params",
          error and (error.code, error.message))
    check("doubles : aucun fichier écrit", not os.path.exists(path))
    step_truncated()


# -- 8. Tableau tronqué ----------------------------------------------------------------------------------
def step_truncated():
    left = planar_layer("gauche", [("l%d" % i, 900000 + 10 * i, 6500000) for i in range(12)])
    right = planar_layer("droite", [("r%d" % i, 900000 + 10 * i, 6500500) for i in range(12)])
    gesture(base(left, right, out=out("9-tronque.csv")), "tronque", step_truncated_done)


def step_truncated_done():
    result, error, _ = outcomes["tronque"]
    check("tronqué : pas d'erreur", error is None, error and error.message)
    check("tronqué : 100 lignes, 144 paires, tronqué vrai",
          result and len(result["rows"]) == 100 and result["pair_count"] == 144 and result["truncated"] is True,
          result and (len(result["rows"]), result["pair_count"], result["truncated"]))
    _, rows = table_of("tronque")
    check("tronqué : le fichier garde les 144 paires", len(rows) == 144, len(rows))
    finish()


QTimer.singleShot(0, protect(step_degrees))
QTimer.singleShot(180000, lambda: (check("sonde terminée à temps", False), finish()))
app.exec()

failed = [label for label, ok in checks if not ok]
print("RÉSULTAT : %s" % ("OK" if checks and not failed else "ÉCHEC (%s)" % ", ".join(failed)))
QgsApplication.exitQgis()
sys.exit(0 if checks and not failed else 1)
