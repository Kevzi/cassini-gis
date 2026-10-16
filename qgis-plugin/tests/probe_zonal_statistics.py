"""Sonde manuelle sous le vrai QGIS : `analysis.zonal_statistics`.

À lancer avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`) :

    "C:\Program Files\QGIS 3.44.15\bin\python-qgis-ltr.bat" qgis-plugin/tests/probe_zonal_statistics.py

Sur `../donnees-test-cassini/gares-2154.csv` : trois zones rectangulaires construites ici
(moitié ouest des gares, moitié est, une zone vide loin de tout), dont les décomptes
attendus sont faits à la main en Python sur le CSV, sans passer par QGIS ; puis une grille
de 120 zones pour la troncature à 100 ; puis une couche de points donnée comme zones (refus).
Le jeu de données vit hors du dépôt (voir CLAUDE.md §5).
"""

import csv
import os
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "qgis-plugin", "cassini_qgis"))
CSV = os.path.normpath(os.path.join(ROOT, "..", "donnees-test-cassini", "gares-2154.csv"))

from qgis.analysis import QgsNativeAlgorithms  # noqa: E402
from qgis.core import (  # noqa: E402
    QgsApplication,
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

uri = "file:///%s?delimiter=,&xField=X&yField=Y&crs=EPSG:2154" % CSV.replace("\\", "/")
gares = QgsVectorLayer(uri, "gares", "delimitedtext")
assert gares.isValid(), "CSV illisible"
QgsProject.instance().addMapLayer(gares)

with open(CSV, encoding="utf-8") as f:
    xs = [float(row["X"]) for row in csv.DictReader(f)]
x0, x1 = min(xs) - 10, max(xs) + 10
mid = sorted(xs)[len(xs) // 2 - 1] + 0.5  # coupe entre les gares, jamais sur l'une d'elles
west = sum(1 for x in xs if x < mid)
east = sum(1 for x in xs if x >= mid)
Y0, Y1 = 6000000.0, 7000000.0  # large bande verticale en Lambert-93


def zones_layer(name, rects):
    layer = QgsVectorLayer("Polygon?crs=EPSG:2154&field=nom:string", name, "memory")
    features = []
    for label, (xa, ya, xb, yb) in rects:
        feature = QgsFeature(layer.fields())
        feature["nom"] = label
        feature.setGeometry(
            QgsGeometry.fromWkt("POLYGON((%f %f,%f %f,%f %f,%f %f,%f %f))" % (xa, ya, xb, ya, xb, yb, xa, yb, xa, ya))
        )
        features.append(feature)
    layer.dataProvider().addFeatures(features)
    QgsProject.instance().addMapLayer(layer)
    return layer


three = zones_layer("trois", [("Ouest", (x0, Y0, mid, Y1)), ("Est", (mid, Y0, x1 + 10, Y1)), ("Vide", (0.0, 0.0, 10.0, 10.0))])
grid = zones_layer("grille", [("z%d" % i, (i * 10.0, 0.0, i * 10.0 + 5, 5.0)) for i in range(120)])

out_dir = tempfile.mkdtemp()
steps = [
    ("trois", {"layer_id": three.id(), "points_layer_id": gares.id(), "label_field": "nom"}),
    ("grille", {"layer_id": grid.id(), "points_layer_id": gares.id()}),
    ("points en zones", {"layer_id": gares.id(), "points_layer_id": three.id()}),
]
results = []
state = {"i": 0}


def check(label, result, error):
    if label == "trois":
        want = [{"label": "Ouest", "count": west}, {"label": "Est", "count": east}, {"label": "Vide", "count": 0}]
        return error is None and result["zones"] == want and not result["truncated"] and result["feature_count"] == 3
    if label == "grille":
        return (
            error is None
            and len(result["zones"]) == 100
            and result["truncated"]
            and result["feature_count"] == 120
            and all(z["count"] == 0 for z in result["zones"])
        )
    return error is not None and error.code == "analysis.bad_params"


def next_step():
    i = state["i"]
    if i >= len(steps):
        QCoreApplication.quit()
        return
    label, params = steps[i]
    params = dict(params, out=os.path.join(out_dir, "%d-analysis.zonal_statistics.gpkg" % (i + 1)))

    def done(result=None, error=None):
        results.append((label, result if error is None else error.code, check(label, result, error)))
        state["i"] += 1
        QTimer.singleShot(0, guarded)

    state["gesture"] = analysis.ProcessingGesture(analysis.ALGORITHMS["analysis.zonal_statistics"])
    state["gesture"].start(params, done)


def guarded():
    import traceback

    try:
        next_step()
    except Exception:
        traceback.print_exc()
        sys.stdout.flush()
        QCoreApplication.quit()


QTimer.singleShot(0, guarded)
QTimer.singleShot(60000, QCoreApplication.quit)
QCoreApplication.exec()

print("attendu à la main : Ouest=%d Est=%d Vide=0" % (west, east))
for row in results:
    print(row)
ok = len(results) == len(steps) and all(r[2] for r in results)
print("RÉSULTAT :", "OK" if ok else "ÉCHEC")
sys.exit(0 if ok else 1)
