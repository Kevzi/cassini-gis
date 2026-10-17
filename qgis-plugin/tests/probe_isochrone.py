r"""Sonde manuelle sous le vrai QGIS : `analysis.isochrone`.

À lancer avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`) :

    "C:\Program Files\QGIS 3.44.15\bin\python-qgis-ltr.bat" qgis-plugin/tests/probe_isochrone.py

Sur `fixtures/reseau-grille-2154.geojson` (rues tous les 100 m, 1 km de côté, origine au
centre) : à pied en 3 min (5 km/h) on atteint 250 m, soit une emprise d'environ 500 m de
côté ; en voiture en 3 min (50 km/h), toute la grille. Les emprises attendues sont calculées
ici à la main, sans passer par QGIS. Puis : durée en secondes, origine en WGS84 (reprojetée),
origine loin du réseau, réseau en degrés, couche de points.
"""

import os
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "qgis-plugin", "cassini_qgis"))
GRID = os.path.join(ROOT, "fixtures", "reseau-grille-2154.geojson")

from qgis.analysis import QgsNativeAlgorithms  # noqa: E402
from qgis.core import (  # noqa: E402
    QgsApplication,
    QgsCoordinateReferenceSystem,
    QgsCoordinateTransform,
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

grid = QgsVectorLayer(GRID, "reseau", "ogr")
assert grid.isValid(), "grille illisible"
QgsProject.instance().addMapLayer(grid)

# Même réseau, mais déclaré en degrés (refus attendu) ; des points (refus attendu).
degrees = QgsVectorLayer("LineString?crs=EPSG:4326", "reseau-degres", "memory")
QgsProject.instance().addMapLayer(degrees)
points = QgsVectorLayer("Point?crs=EPSG:2154", "points", "memory")
QgsProject.instance().addMapLayer(points)

CX, CY = 652500.0, 6850500.0
TOL = 5.0
wgs = QgsCoordinateTransform(
    QgsCoordinateReferenceSystem("EPSG:2154"), QgsCoordinateReferenceSystem("EPSG:4326"), QgsProject.instance()
)
centre_wgs = wgs.transform(QgsPointXY(CX, CY))


def base(**extra):
    p = {
        "layer_id": grid.id(),
        "origin": {"x": CX, "y": CY, "epsg": 2154},
        "mode": "walk",
        "duration": {"value": 3, "unit": "min"},
    }
    p.update(extra)
    return p


def extent_of(result):
    layer = QgsProject.instance().mapLayer(result["layer_id"])
    box = layer.extent()
    return box.xMinimum(), box.yMinimum(), box.xMaximum(), box.yMaximum()


def near(box, half):
    want = (CX - half, CY - half, CX + half, CY + half)
    return all(abs(a - b) <= TOL for a, b in zip(box, want))


state = {"walk": None}


def check_walk(result, error):
    if error is not None or result["feature_count"] != 1:
        return False
    state["walk"] = extent_of(result)
    return near(state["walk"], 250.0)


def check_car(result, error):
    if error is not None or result["feature_count"] != 1:
        return False
    box = extent_of(result)
    return state["walk"] is not None and (box[2] - box[0]) > (state["walk"][2] - state["walk"][0]) and near(box, 500.0)


def check_seconds(result, error):
    return error is None and state["walk"] is not None and all(
        abs(a - b) <= TOL for a, b in zip(extent_of(result), state["walk"])
    )


def check_wgs(result, error):
    return error is None and all(abs(a - b) <= 20.0 for a, b in zip(extent_of(result), state["walk"]))


def code(expected):
    return lambda result, error: error is not None and error.code == expected


steps = [
    ("marche", base(), check_walk),
    ("voiture", base(mode="car"), check_car),
    ("durée en s", base(duration={"value": 180, "unit": "s"}), check_seconds),
    ("origine en WGS84", base(origin={"x": centre_wgs.x(), "y": centre_wgs.y(), "epsg": 4326}), check_wgs),
    ("origine hors réseau", base(origin={"x": CX + 5000.0, "y": CY + 5000.0, "epsg": 2154}), code("analysis.origin_off_network")),
    ("réseau en degrés", base(layer_id=degrees.id()), code("analysis.crs_not_metric")),
    ("couche de points", base(layer_id=points.id()), code("analysis.bad_params")),
    ("réseau absent", base(layer_id="nope"), code("layer.not_found")),
]
results = []
state["i"] = 0
out_dir = tempfile.mkdtemp()


def next_step():
    i = state["i"]
    if i >= len(steps):
        QCoreApplication.quit()
        return
    label, params, check = steps[i]
    out = os.path.join(out_dir, "%d-analysis.isochrone.gpkg" % (i + 1))

    def done(result=None, error=None):
        ok = bool(check(result, error))
        detail = error.code if error is not None else (result["feature_count"], extent_of(result))
        # Un refus ne laisse aucun fichier ; un succès n'en laisse qu'un, jamais d'annexe.
        files = sorted(os.listdir(out_dir))
        expected = 1 if error is None else 0
        mine = [f for f in files if f.startswith("%d-" % (i + 1))]
        ok = ok and len(mine) == expected
        results.append((label, detail, ok))
        state["i"] += 1
        QTimer.singleShot(0, guarded)

    state["gesture"] = analysis.ProcessingGesture(analysis.ALGORITHMS["analysis.isochrone"])
    state["gesture"].start(dict(params, out=out), done)


def guarded():
    import traceback

    try:
        next_step()
    except Exception:
        traceback.print_exc()
        sys.stdout.flush()
        QCoreApplication.quit()


QTimer.singleShot(0, guarded)
QTimer.singleShot(90000, QCoreApplication.quit)
QCoreApplication.exec()

print("attendu à la main : marche = emprise de 500 m autour de (%.0f, %.0f), voiture = emprise de 1000 m" % (CX, CY))
for row in results:
    print(row)
ok = len(results) == len(steps) and all(r[2] for r in results)
print("RÉSULTAT :", "OK" if ok else "ÉCHEC")
sys.exit(0 if ok else 1)
