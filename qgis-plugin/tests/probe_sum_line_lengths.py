"""Sonde manuelle sous le vrai QGIS : longueurs de lignes par polygone (ticket 11, `analysis.sum_line_lengths`).

À lancer avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`) :

    "C:\\Program Files\\QGIS 3.44.15\\bin\\python-qgis-ltr.bat" qgis-plugin/tests/probe_sum_line_lengths.py

Aucun noyau ni aucune donnée hors dépôt : des couches mémoire synthétiques autour d'Annecy. Le
geste reçoit `work_epsg` comme le noyau le lui donne. Termine par `RÉSULTAT : OK` ou `ÉCHEC`.

1. Polygones ET lignes en EPSG:4326, `work_epsg` = 2154 : les longueurs sont en mètres, comparées à la
   longueur ellipsoïdale (`QgsDistanceArea`, WGS84) de la partie de la ligne contenue dans chaque
   polygone, à 5 m près plus 0,3 % (l'altération linéaire du Lambert-93). Ce point tranche la question
   du plan : `native:sumlinelengths` transforme-t-il la couche de lignes vers le système des polygones ?
2. Une ligne qui sort du polygone n'est comptée que pour sa partie intérieure.
3. Un polygone sans ligne est présent, avec `length` à 0.
4. `unit: km` : valeurs divisées par 1000, unité `km`.
5. Polygones en EPSG:2154 et lignes en EPSG:4326, sans `work_epsg` : mêmes longueurs.
6. Couche de lignes en polygones : refusée avant tout lancement ; sans `work_epsg`, des polygones en
   degrés sont refusés (`analysis.crs_not_metric`).
7. Plus de 100 polygones : 100 lignes, `truncated` vrai, la couche produite garde tout.
8. Quand l'entrée porte déjà un champ `length_m`, QGIS le remplace sur place : le champ de longueur se
   lit par son nom, jamais par sa place.
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

work = tempfile.mkdtemp(prefix="cassini-longueurs-")
project = QgsProject.instance()
project.setCrs(QgsCoordinateReferenceSystem("EPSG:2154"))
checks = []
outcomes = {}
ENTRY = analysis.ALGORITHMS["analysis.sum_line_lengths"]

# Deux polygones autour d'Annecy (lon0, lat0, lon1, lat1) ; le troisième ne porte aucune ligne.
POLYGONS = [("Nord", 6.10, 45.90, 6.20, 45.95), ("Sud", 6.10, 45.80, 6.20, 45.85), ("Vide", 6.30, 45.80, 6.35, 45.85)]
# Lignes en WGS84 : l1 est entièrement dans Nord ; l2 sort de Sud par l'est ; l3 est hors de tous les polygones.
LINES = [
    ("l1", [(6.11, 45.92), (6.19, 45.92)]),
    ("l2", [(6.15, 45.82), (6.25, 45.82)]),
    ("l3", [(6.40, 45.50), (6.45, 45.50)]),
]

ellipsoid = QgsDistanceArea()
ellipsoid.setSourceCrs(QgsCoordinateReferenceSystem("EPSG:4326"), project.transformContext())
ellipsoid.setEllipsoid("WGS84")
EXPECTED = {
    "Nord": ellipsoid.measureLine(QgsPointXY(6.11, 45.92), QgsPointXY(6.19, 45.92)),
    "Sud": ellipsoid.measureLine(QgsPointXY(6.15, 45.82), QgsPointXY(6.20, 45.82)),  # partie intérieure seulement
    "Vide": 0.0,
}


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


def to(epsg, point):
    if epsg == 4326:
        return point
    transform = QgsCoordinateTransform(
        QgsCoordinateReferenceSystem("EPSG:4326"), QgsCoordinateReferenceSystem("EPSG:%d" % epsg), project
    )
    return transform.transform(point)


def polygon_layer(name, epsg, rows=POLYGONS, extra=""):
    layer = QgsVectorLayer("Polygon?crs=EPSG:%d&field=nom:string%s" % (epsg, extra), name, "memory")
    features = []
    for nom, x0, y0, x1, y1 in rows:
        corners = [to(epsg, QgsPointXY(x, y)) for x, y in ((x0, y0), (x1, y0), (x1, y1), (x0, y1), (x0, y0))]
        f = QgsFeature(layer.fields())
        f.setAttribute("nom", nom)
        f.setGeometry(QgsGeometry.fromPolygonXY([corners]))
        features.append(f)
    layer.dataProvider().addFeatures(features)
    project.addMapLayer(layer)
    return layer


def line_layer(name, epsg, rows=LINES):
    layer = QgsVectorLayer("LineString?crs=EPSG:%d&field=nom:string" % epsg, name, "memory")
    features = []
    for nom, points in rows:
        f = QgsFeature(layer.fields())
        f.setAttribute("nom", nom)
        f.setGeometry(QgsGeometry.fromPolylineXY([to(epsg, QgsPointXY(x, y)) for x, y in points]))
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


def lengths(result, unit_factor=1.0):
    return {row["label"]: row["values"]["length"]["value"] for row in result["rows"]}


def check_lengths(label, result, factor=1.0):
    got = lengths(result)
    for nom, want in EXPECTED.items():
        want = want / factor
        tolerance = (5.0 + 0.003 * want * factor) / factor
        check(
            "%s : longueur de %s (%.1f attendus, à %.3f près)" % (label, nom, want, tolerance),
            nom in got and abs(got[nom] - want) <= tolerance,
            got.get(nom),
        )


polygons_wgs = polygon_layer("communes_wgs", 4326)
lines_wgs = line_layer("routes_wgs", 4326)
polygons_l93 = polygon_layer("communes_l93", 2154)
lines_l93 = line_layer("routes_l93", 2154)


# -- 1, 2 et 3. Polygones et lignes en degrés, `work_epsg` = 2154 -----------------------------
def step_degrees():
    gesture(
        {"layer_id": polygons_wgs.id(), "lines_layer_id": lines_wgs.id(), "label_field": "nom",
         "work_epsg": 2154, "out": out("1-longueurs.gpkg")},
        "degres",
        step_degrees_done,
    )


def step_degrees_done():
    result, error, path = outcomes["degres"]
    check("degrés : pas d'erreur", error is None, error and error.message)
    check("degrés : feature_count = 3 (le polygone sans ligne y est)", result and result["feature_count"] == 3, result)
    check("degrés : sortie en EPSG:2154", result and result["crs"] == "EPSG:2154", result)
    check("degrés : une ligne par polygone, unité m",
          result and len(result["rows"]) == 3 and all(r["values"]["length"]["unit"] == "m" for r in result["rows"]), result)
    check("degrés : pas tronqué", result and result["truncated"] is False, result)
    check_lengths("degrés", result)
    check("degrés : le polygone sans ligne vaut 0", result and lengths(result).get("Vide") == 0, result)
    step_km()


# -- 4. Kilomètres ---------------------------------------------------------------------------
def step_km():
    gesture(
        {"layer_id": polygons_wgs.id(), "lines_layer_id": lines_wgs.id(), "label_field": "nom", "unit": "km",
         "work_epsg": 2154, "out": out("2-longueurs-km.gpkg")},
        "km",
        step_km_done,
    )


def step_km_done():
    result, error, _ = outcomes["km"]
    check("km : pas d'erreur", error is None, error and error.message)
    check("km : unité km", result and all(r["values"]["length"]["unit"] == "km" for r in result["rows"]), result)
    check_lengths("km", result, factor=1000.0)
    step_metric()


# -- 5. Polygones en mètres, lignes en degrés, sans `work_epsg` ----------------------------------
def step_metric():
    gesture(
        {"layer_id": polygons_l93.id(), "lines_layer_id": lines_wgs.id(), "label_field": "nom",
         "out": out("3-longueurs-metres.gpkg")},
        "metres",
        step_metric_done,
    )


def step_metric_done():
    result, error, _ = outcomes["metres"]
    check("mètres : pas d'erreur", error is None, error and error.message)
    check_lengths("mètres", result)
    step_mixed()


# -- 5 bis. Polygones en degrés reprojetés, lignes en Lambert-93 --------------------------------------
def step_mixed():
    gesture(
        {"layer_id": polygons_wgs.id(), "lines_layer_id": lines_l93.id(), "label_field": "nom",
         "work_epsg": 2154, "out": out("4-longueurs-mixte.gpkg")},
        "mixte",
        step_mixed_done,
    )


def step_mixed_done():
    result, error, _ = outcomes["mixte"]
    check("mixte : pas d'erreur", error is None, error and error.message)
    check_lengths("mixte", result)
    step_refusals()


# -- 6. Refus avant tout lancement ----------------------------------------------------------------------
def step_refusals():
    gesture(
        {"layer_id": polygons_wgs.id(), "lines_layer_id": polygons_l93.id(), "work_epsg": 2154,
         "out": out("5-refus-lignes.gpkg")},
        "refus_lignes",
        step_refusals_lines_done,
    )


def step_refusals_lines_done():
    result, error, path = outcomes["refus_lignes"]
    check("refus : une couche de polygones à la place des lignes", result is None and error is not None
          and error.code == analysis.BAD_PARAMS, error and error.code)
    check("refus : aucun fichier écrit", not os.path.exists(path))
    gesture(
        {"layer_id": polygons_wgs.id(), "lines_layer_id": lines_wgs.id(), "out": out("6-refus-degres.gpkg")},
        "refus_degres",
        step_refusals_degrees_done,
    )


def step_refusals_degrees_done():
    result, error, path = outcomes["refus_degres"]
    check("refus : polygones en degrés sans work_epsg", result is None and error is not None
          and error.code == analysis.CRS_NOT_METRIC, error and error.code)
    check("refus : aucun fichier écrit en degrés", not os.path.exists(path))
    step_many()


# -- 7. Plus de 100 polygones ------------------------------------------------------------------------------
def step_many():
    rows = [("p%03d" % i, 6.0 + i * 0.001, 45.0, 6.0005 + i * 0.001, 45.001) for i in range(150)]
    many = polygon_layer("beaucoup", 4326, rows)
    gesture(
        {"layer_id": many.id(), "lines_layer_id": lines_wgs.id(), "label_field": "nom", "work_epsg": 2154,
         "out": out("7-longueurs-beaucoup.gpkg")},
        "beaucoup",
        step_many_done,
    )


def step_many_done():
    result, error, path = outcomes["beaucoup"]
    check("150 polygones : pas d'erreur", error is None, error and error.message)
    check("150 polygones : 100 lignes, tronqué", result and len(result["rows"]) == 100 and result["truncated"] is True,
          result and (len(result["rows"]), result["truncated"]))
    check("150 polygones : la couche produite garde les 150", result and result["feature_count"] == 150, result)
    step_taken()


# -- 8. Un champ `length_m` déjà porté par l'entrée, avant d'autres champs ---------------------------------------------------------
def step_taken():
    taken = polygon_layer("pris", 2154, extra="&field=length_m:double&field=autre:string&field=line_count:integer")
    gesture(
        {"layer_id": taken.id(), "lines_layer_id": lines_l93.id(), "label_field": "nom", "out": out("8-pris.gpkg")},
        "pris",
        step_taken_done,
    )


def step_taken_done():
    result, error, path = outcomes["pris"]
    check("champ pris : pas d'erreur", error is None, error and error.message)
    layer = QgsVectorLayer(path, "lecture", "ogr")
    names = [f.name() for f in layer.fields()]
    check("champ pris : QGIS remplace le champ existant au lieu de le renommer",
          names.count("length_m") == 1 and names.index("length_m") < names.index("autre"), names)
    del layer
    check_lengths("champ pris", result)
    finish()


QTimer.singleShot(0, protect(step_degrees))
QTimer.singleShot(150000, lambda: (check("sonde terminée à temps", False), finish()))
app.exec()

failed = [label for label, ok in checks if not ok]
print("RÉSULTAT : %s" % ("OK" if checks and not failed else "ÉCHEC (%s)" % ", ".join(failed)))
QgsApplication.exitQgis()
sys.exit(0 if checks and not failed else 1)
