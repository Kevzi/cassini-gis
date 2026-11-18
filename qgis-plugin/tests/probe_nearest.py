"""Sonde manuelle sous le vrai QGIS : plus proche voisin (entrée 10.10, `analysis.nearest`).

À lancer avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`) :

    "C:\\Program Files\\QGIS 3.44.15\\bin\\python-qgis-ltr.bat" qgis-plugin/tests/probe_nearest.py

Aucun noyau ni aucune donnée hors dépôt : des couches mémoire synthétiques autour d'Annecy. Le
geste reçoit `work_epsg` comme le noyau le lui donne. Termine par `RÉSULTAT : OK` ou `ÉCHEC`.

1. Couche d'entrée ET cible en EPSG:4326, `work_epsg` = 2154 : le champ `distance` est en mètres,
   comparé à la distance ellipsoïdale (`QgsDistanceArea`, WGS84) vers la cible attendue, à 5 m près
   plus 0,2 % (l'altération linéaire du Lambert-93 : 13 m à 15 km).
   Ce point tranche la question du plan : `native:joinbynearest` transforme-t-il la couche cible
   vers le système de l'entrée ? Sinon, la cible serait lue en degrés et les distances fausses.
2. Chaque entité garde son voisin (le nom de la gare porté par la couche cible), `feature_count` =
   nombre d'entrées, `unmatched_count` = 0 sans limite.
3. Limite de 500 m : les entités plus éloignées sont gardées, sans voisin ; elles sont comptées.
4. Couche cible vide : toutes les entités sans voisin, `unmatched_count` = `feature_count`.
5. Couche d'entrée en EPSG:2154 et cible en EPSG:4326, sans `work_epsg` : mêmes distances.
6. Trois cibles à égale distance d'une entité qui porte déjà un champ `distance` : une seule ligne
   en sortie, et `distance_field` désigne le champ renommé par QGIS (`distance_2`).
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
import analysis  # noqa: E402

TOLERANCE_M = 5.0
# Entrées autour d'Annecy (longitude, latitude) : la plus proche gare est dite pour chacune.
INPUTS = [("a", 6.1222, 45.9013, "g1"), ("b", 6.1500, 45.9100, "g2"), ("c", 5.9440, 45.8676, "g1")]
GARES = [("g1", 6.1262, 45.9013), ("g2", 6.1500, 45.9140)]
LIMIT = {"value": 500, "unit": "m"}

work = tempfile.mkdtemp(prefix="cassini-voisin-")
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


def memory_layer(name, rows, epsg):
    """Couche mémoire de points `name`/`x`/`y` (en WGS84) ; `rows` : (nom, lon, lat, ...)."""
    layer = QgsVectorLayer("Point?crs=EPSG:%d&field=name:string" % epsg, name, "memory")
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
        f.setGeometry(QgsGeometry.fromPointXY(point))
        features.append(f)
    layer.dataProvider().addFeatures(features)
    layer.updateExtents()
    project.addMapLayer(layer)
    return layer


def out(name):
    return os.path.join(work, name)


def gesture(params, label, then):
    def done(result=None, error=None):
        outcomes[label] = (result, error, params.get("out"))
        QTimer.singleShot(0, protect(then))

    g = analysis.ProcessingGesture(analysis.ALGORITHMS["analysis.nearest"])
    outcomes["_gesture_" + label] = g  # gardé en vie pendant la tâche
    g.start(params, done)


ellipsoid = QgsDistanceArea()
ellipsoid.setSourceCrs(QgsCoordinateReferenceSystem("EPSG:4326"), project.transformContext())
ellipsoid.setEllipsoid("WGS84")


def expected_distance(lon, lat, gare):
    g = next(r for r in GARES if r[0] == gare)
    return ellipsoid.measureLine(QgsPointXY(lon, lat), QgsPointXY(g[1], g[2]))


def rows_of(label):
    """Couche produite par le geste `label`, relue du fichier : nom d'entrée -> (voisin, distance)."""
    result, error, path = outcomes[label]
    layer = QgsVectorLayer(path, "lecture", "ogr")
    fields = [f.name() for f in layer.fields()]
    neighbour_field = [n for n in fields if n.startswith("name") and n != "name"]
    rows = {}
    for f in layer.getFeatures():
        rows[f["name"]] = (f[neighbour_field[0]] if neighbour_field else None, f["distance"])
    return result, error, rows, fields


def is_null(value):
    return value is None or (hasattr(value, "isNull") and value.isNull())


def check_distances(label, rows):
    for name, lon, lat, gare in INPUTS:
        near, distance = rows.get(name, (None, None))
        check("%s : %s a pour voisin %s" % (label, name, gare), near == gare, near)
        want = expected_distance(lon, lat, gare)
        check(
            "%s : distance de %s en mètres (%.0f m attendus, à %.0f m + 0,2 %% près)" % (label, name, want, TOLERANCE_M),
            not is_null(distance) and abs(float(distance) - want) <= TOLERANCE_M + 0.002 * want,
            distance,
        )


inputs_wgs = memory_layer("parcelles_wgs", INPUTS, 4326)
gares_wgs = memory_layer("gares_wgs", GARES, 4326)
inputs_l93 = memory_layer("parcelles_l93", INPUTS, 2154)
gares_empty = memory_layer("gares_vide", [], 4326)


# -- 1 et 2. Entrée et cible en degrés, `work_epsg` = 2154 -----------------------------------
def step_degrees():
    gesture(
        {"layer_id": inputs_wgs.id(), "target_layer_id": gares_wgs.id(), "work_epsg": 2154, "out": out("1-voisin.gpkg")},
        "degres",
        step_degrees_done,
    )


def step_degrees_done():
    result, error, rows, fields = rows_of("degres")
    check("degrés : pas d'erreur", error is None, error and error.message)
    check("degrés : champ distance écrit", "distance" in fields, fields)
    check("degrés : feature_count = 3", result and result["feature_count"] == 3, result)
    check("degrés : unmatched_count = 0 sans limite", result and result["unmatched_count"] == 0, result)
    check("degrés : unit = m, distance_field = distance",
          result and (result["unit"], result["distance_field"]) == ("m", "distance"), result)
    check("degrés : sortie en EPSG:2154", result and result["crs"] == "EPSG:2154", result)
    check_distances("degrés", rows)
    step_limit()


# -- 3. Limite de 500 m ----------------------------------------------------------------------
def step_limit():
    gesture(
        {"layer_id": inputs_wgs.id(), "target_layer_id": gares_wgs.id(), "work_epsg": 2154,
         "max_distance": LIMIT, "out": out("2-voisin-limite.gpkg")},
        "limite",
        step_limit_done,
    )


def step_limit_done():
    result, error, rows, _ = rows_of("limite")
    check("limite : pas d'erreur", error is None, error and error.message)
    check("limite : aucune entité écartée", result and result["feature_count"] == 3, result)
    within = [n for n, lon, lat, g in INPUTS if expected_distance(lon, lat, g) <= 500]
    far = [n for n, *_ in INPUTS if n not in within]
    check("limite : au moins une entité dans la limite et une hors limite", within and far, (within, far))
    check("limite : les entités hors limite sont comptées", result and result["unmatched_count"] == len(far), result)
    check("limite : les entités hors limite n'ont pas de voisin", all(is_null(rows[n][0]) for n in far), rows)
    check("limite : les entités dans la limite gardent leur voisin", all(not is_null(rows[n][0]) for n in within), rows)
    step_empty()


# -- 4. Cible vide ---------------------------------------------------------------------------
def step_empty():
    gesture(
        {"layer_id": inputs_wgs.id(), "target_layer_id": gares_empty.id(), "work_epsg": 2154,
         "out": out("3-voisin-vide.gpkg")},
        "vide",
        step_empty_done,
    )


def step_empty_done():
    result, error, _, _ = outcomes["vide"][0], outcomes["vide"][1], None, None
    check("cible vide : pas d'erreur", error is None, error and error.message)
    check("cible vide : toutes les entités sans voisin",
          result and result["unmatched_count"] == result["feature_count"] == 3, result)
    step_metric()


# -- 5. Entrée en mètres, cible en degrés, sans `work_epsg` ------------------------------------
def step_metric():
    gesture(
        {"layer_id": inputs_l93.id(), "target_layer_id": gares_wgs.id(), "out": out("4-voisin-metres.gpkg")},
        "metres",
        step_metric_done,
    )


def step_metric_done():
    result, error, rows, _ = rows_of("metres")
    check("mètres : pas d'erreur", error is None, error and error.message)
    check("mètres : unmatched_count = 0", result and result["unmatched_count"] == 0, result)
    check_distances("mètres", rows)
    step_ties()


# -- 6. Voisins à égale distance et champ `distance` déjà pris -----------------------------------
def planar_layer(name, fields, rows):
    layer = QgsVectorLayer("Point?crs=EPSG:2154&" + "&".join("field=%s" % f for f in fields), name, "memory")
    features = []
    for attributes, x, y in rows:
        f = QgsFeature(layer.fields())
        f.setAttributes(attributes)
        f.setGeometry(QgsGeometry.fromPointXY(QgsPointXY(x, y)))
        features.append(f)
    layer.dataProvider().addFeatures(features)
    project.addMapLayer(layer)
    return layer


def step_ties():
    entries = planar_layer("entrees", ["name:string", "distance:double"], [(["a", 1.0], 900000, 6500000)])
    targets = planar_layer(
        "cibles_equidistantes", ["name:string"],
        [(["c1"], 900010, 6500000), (["c2"], 899990, 6500000), (["c3"], 900000, 6500010)],
    )
    gesture({"layer_id": entries.id(), "target_layer_id": targets.id(), "out": out("5-voisin-egalite.gpkg")},
            "egalite", step_ties_done)


def step_ties_done():
    result, error, path = outcomes["egalite"]
    check("égalité : pas d'erreur", error is None, error and error.message)
    check("égalité : une ligne par entité (feature_count = 1)", result and result["feature_count"] == 1, result)
    check("égalité : le champ de distance est celui que QGIS a écrit (distance_2)",
          result and result["distance_field"] == "distance_2", result)
    check("égalité : unmatched_count = 0", result and result["unmatched_count"] == 0, result)
    layer = QgsVectorLayer(path, "lecture", "ogr")
    values = [f["distance_2"] for f in layer.getFeatures()]
    check("égalité : distance de 10 m", len(values) == 1 and abs(float(values[0]) - 10.0) < 0.01, values)
    finish()


QTimer.singleShot(0, protect(step_degrees))
QTimer.singleShot(120000, lambda: (check("sonde terminée à temps", False), finish()))
app.exec()

failed = [label for label, ok in checks if not ok]
print("RÉSULTAT : %s" % ("OK" if checks and not failed else "ÉCHEC (%s)" % ", ".join(failed)))
QgsApplication.exitQgis()
sys.exit(0 if checks and not failed else 1)
