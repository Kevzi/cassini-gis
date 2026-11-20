"""Sonde manuelle sous le vrai QGIS : différence de couches (ticket 21, `analysis.difference`).

À lancer avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`) :

    "C:\\Program Files\\QGIS 3.44.15\\bin\\python-qgis-ltr.bat" qgis-plugin/tests/probe_difference.py

Aucun noyau ni aucune donnée hors dépôt : couches mémoire synthétiques, vrai geste
`analysis.difference`. Termine par `VERDICT : OK` ou `VERDICT : ÉCHEC`.

Ce que la sonde constate (le plan interdit de le supposer) :
1. Recouvrement partiel : la partie commune est retranchée, l'entité non touchée reste telle quelle.
2. Recouvrement total : couche vide (0 entité), sans erreur, `feature_count` = 0.
3. Aucun recouvrement : la sortie a les mêmes entités et les mêmes surfaces que l'entrée.
4. Champs : la sortie garde les champs de l'entrée seuls, aucun champ de la couche retranchée.
5. `fid` : colonne du GeoPackage, renumérotée par QGIS ; les originaux ne sont pas modifiés.
6. Géométries : une entité multipartie reste multipartie ; une couche de lignes retranchée d'un
   polygone ne retire rien (surface inchangée).
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
    QgsFeature,
    QgsGeometry,
    QgsProject,
    QgsVectorLayer,
)
from qgis.PyQt.QtCore import QCoreApplication, QTimer  # noqa: E402

app = QgsApplication([], False)
QgsApplication.initQgis()
QgsApplication.processingRegistry().addProvider(QgsNativeAlgorithms())
import analysis  # noqa: E402

work = tempfile.mkdtemp(prefix="cassini-difference-")
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


def out(name):
    return os.path.join(work, name)


def run_gesture(gesture, params, label, then):
    def done(result=None, error=None):
        outcomes[label] = (result, error)
        QTimer.singleShot(0, protect(then))

    outcomes["_g_" + label] = gesture  # gardé en vie pendant la tâche
    gesture.start(params, done)


def difference(params, label, then):
    run_gesture(analysis.ProcessingGesture(analysis.ALGORITHMS["analysis.difference"]), params, label, then)


def ring(x, y=0, size=1000):
    x0, y0 = 900000 + x, 6500000 + y
    return "%d %d,%d %d,%d %d,%d %d,%d %d" % (
        x0, y0, x0 + size, y0, x0 + size, y0 + size, x0, y0 + size, x0, y0
    )


def memory_layer(spec, name, rows):
    layer = QgsVectorLayer(spec + "&crs=EPSG:2154", name, "memory")
    feats = []
    for attrs, wkt in rows:
        f = QgsFeature(layer.fields())
        f.setAttributes(attrs)
        f.setGeometry(QgsGeometry.fromWkt(wkt))
        feats.append(f)
    layer.dataProvider().addFeatures(feats)
    project.addMapLayer(layer)
    return layer


# Entrée : trois entités multiparties (x 0-1000, x 3000-4000, et deux carrés vers x 6000 et 8000).
parcels = memory_layer(
    "MultiPolygon?field=commune:string&field=nom:string",
    "parcelles",
    [
        (["A", "p0"], "MULTIPOLYGON(((%s)))" % ring(0)),
        (["B", "p1"], "MULTIPOLYGON(((%s)))" % ring(3000)),
        (["C", "p2"], "MULTIPOLYGON(((%s)),((%s)))" % (ring(6000), ring(8000))),
    ],
)
# Retranchés : partiel (x 500-1500), total (couvre tout), disjoint, et une couche de lignes.
mask_part = memory_layer("Polygon?field=zone:string", "partiel", [(["N"], "POLYGON((%s))" % ring(500))])
mask_all = memory_layer("Polygon?field=zone:string", "total", [(["N"], "POLYGON((%s))" % ring(-1000, -1000, 20000))])
mask_far = memory_layer("Polygon?field=zone:string", "disjoint", [(["N"], "POLYGON((%s))" % ring(50000))])
mask_line = memory_layer(
    "LineString?field=zone:string", "lignes", [(["N"], "LINESTRING(900000 6500500,909000 6500500)")]
)


def read(path):
    layer = QgsVectorLayer(path, "lecture", "ogr")
    return layer, list(layer.getFeatures())


def total_area(feats):
    return round(sum(f.geometry().area() for f in feats))


def step_partial():
    difference(
        {"layer_id": parcels.id(), "overlay_layer_id": mask_part.id(), "out": out("1-analysis.difference.gpkg")},
        "partiel",
        step_partial_done,
    )


def step_partial_done():
    result, error = outcomes["partiel"]
    check("partiel : pas d'erreur", error is None, error and (error.code, error.message))
    if result is None:
        finish()
        return
    check("partiel : feature_count = 3", result["feature_count"] == 3, result)
    check("partiel : sortie en EPSG:2154", result["crs"] == "EPSG:2154", result)
    layer, feats = read(out("1-analysis.difference.gpkg"))
    names = [f.name() for f in layer.fields()]
    print("       champs de la sortie : %s" % names)
    by = {f["nom"]: f for f in feats}
    check("partiel : trois entités lues", len(feats) == 3 and set(by) == {"p0", "p1", "p2"}, list(by))
    check("partiel : p0 perd la partie commune (surface 500 000)", round(by["p0"].geometry().area()) == 500000,
          round(by["p0"].geometry().area()))
    check("partiel : p1 est intacte (1 000 000)", round(by["p1"].geometry().area()) == 1000000)
    check("partiel : l'entité multipartie reste multipartie", by["p2"].geometry().isMultipart())
    check("partiel : champs de l'entrée seuls, aucun champ de la couche retranchée",
          "zone" not in names and {"commune", "nom"} <= set(names), names)
    check("partiel : la colonne fid est le seul ajout aux champs de l'entrée",
          [n for n in names if n not in ("commune", "nom")] == ["fid"], names)
    check("partiel : fid renumérotée 1, 2, 3", sorted(f.id() for f in feats) == [1, 2, 3], [f.id() for f in feats])
    check("partiel : les originaux sont intacts", parcels.featureCount() == 3 and mask_part.featureCount() == 1)
    step_total()


def step_total():
    difference(
        {"layer_id": parcels.id(), "overlay_layer_id": mask_all.id(), "out": out("2-analysis.difference.gpkg")},
        "total",
        step_total_done,
    )


def step_total_done():
    result, error = outcomes["total"]
    check("total : pas d'erreur", error is None, error and (error.code, error.message))
    if result is None:
        finish()
        return
    check("total : feature_count = 0 (couche vide)", result["feature_count"] == 0, result)
    check("total : la couche vide porte un nom", isinstance(result.get("name"), str) and result["name"], result)
    check("total : la couche vide est dans le projet", project.mapLayer(result["layer_id"]) is not None, result)
    layer, feats = read(out("2-analysis.difference.gpkg"))
    check("total : fichier lisible, sans entité", layer.isValid() and len(feats) == 0, (layer.isValid(), len(feats)))
    step_far()


def step_far():
    difference(
        {"layer_id": parcels.id(), "overlay_layer_id": mask_far.id(), "out": out("3-analysis.difference.gpkg")},
        "disjoint",
        step_far_done,
    )


def step_far_done():
    result, error = outcomes["disjoint"]
    check("disjoint : pas d'erreur", error is None, error and (error.code, error.message))
    if result is None:
        finish()
        return
    _, feats = read(out("3-analysis.difference.gpkg"))
    check("disjoint : feature_count = 3", result["feature_count"] == 3, result)
    check("disjoint : surface totale inchangée (4 000 000)", total_area(feats) == 4000000, total_area(feats))
    step_line()


def step_line():
    difference(
        {"layer_id": parcels.id(), "overlay_layer_id": mask_line.id(), "out": out("4-analysis.difference.gpkg")},
        "lignes",
        step_line_done,
    )


def step_line_done():
    result, error = outcomes["lignes"]
    check("lignes : géométries mixtes acceptées, pas d'erreur", error is None, error and (error.code, error.message))
    if result is None:
        finish()
        return
    _, feats = read(out("4-analysis.difference.gpkg"))
    check("lignes : rien n'est retiré à un polygone (surface 4 000 000)", total_area(feats) == 4000000,
          total_area(feats))
    step_same()


def step_same():
    difference(
        {"layer_id": parcels.id(), "overlay_layer_id": parcels.id(), "out": out("5-analysis.difference.gpkg")},
        "meme",
        step_last,
    )


def step_last():
    _, error = outcomes["meme"]
    check("même couche : analysis.bad_params", error is not None and error.code == "analysis.bad_params",
          error and error.code)
    check("même couche : aucun fichier écrit", not os.path.exists(out("5-analysis.difference.gpkg")))
    finish()


QTimer.singleShot(0, protect(step_partial))
QTimer.singleShot(120000, lambda: (check("sonde terminée à temps", False), finish()))
app.exec()

failed = [label for label, ok in checks if not ok]
verdict = bool(checks) and not failed
print("VERDICT :", "OK" if verdict else "ÉCHEC (%s)" % ", ".join(failed))
QgsApplication.exitQgis()
sys.exit(0 if verdict else 1)
