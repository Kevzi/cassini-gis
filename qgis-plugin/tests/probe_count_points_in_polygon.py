"""Sonde manuelle sous le vrai QGIS : `analysis.count_points_in_polygon`.

À lancer avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`) :

    "C:\Program Files\QGIS 3.44.15\bin\python-qgis-ltr.bat" qgis-plugin/tests/probe_count_points_in_polygon.py

Sur `../donnees-test-cassini/gares-2154.csv` : trois zones rectangulaires construites ici
(moitié ouest des gares, moitié est, une zone vide loin de tout), dont les décomptes
attendus sont faits à la main en Python sur le CSV, sans passer par QGIS ; puis une grille
de 120 zones pour la troncature à 100 ; puis une couche de points donnée comme zones (refus) ; puis la somme pondérée d'un champ numérique
(2,5 par point : somme attendue = 2,5 x décompte), et le refus d'un champ texte.
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
    rows = [(float(row["X"]), float(row["Y"])) for row in csv.DictReader(f)]
xs = [x for x, _ in rows]
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


weighted = QgsVectorLayer("Point?crs=EPSG:2154&field=poids:double&field=nom:string", "ponderes", "memory")
points = []
for x, y in rows:
    feature = QgsFeature(weighted.fields())
    feature["poids"] = 2.5
    feature["nom"] = "g"
    feature.setGeometry(QgsGeometry.fromWkt("POINT(%f %f)" % (x, y)))
    points.append(feature)
weighted.dataProvider().addFeatures(points)
QgsProject.instance().addMapLayer(weighted)

three = zones_layer("trois", [("Ouest", (x0, Y0, mid, Y1)), ("Est", (mid, Y0, x1 + 10, Y1)), ("Vide", (0.0, 0.0, 10.0, 10.0))])
grid = zones_layer("grille", [("z%d" % i, (i * 10.0, 0.0, i * 10.0 + 5, 5.0)) for i in range(120)])

out_dir = tempfile.mkdtemp()
steps = [
    ("trois", {"layer_id": three.id(), "points_layer_id": gares.id(), "label_field": "nom"}),
    ("grille", {"layer_id": grid.id(), "points_layer_id": gares.id()}),
    ("points en zones", {"layer_id": gares.id(), "points_layer_id": three.id()}),
    ("pondere", {"layer_id": three.id(), "points_layer_id": weighted.id(), "label_field": "nom", "weight_field": "poids"}),
    ("champ texte", {"layer_id": three.id(), "points_layer_id": weighted.id(), "weight_field": "nom"}),
]
results = []
state = {"i": 0}


def check(label, result, error):
    if label == "trois":
        want = [
            {"label": label, "values": {"count": {"value": n, "unit": "points"}}}
            for label, n in (("Ouest", west), ("Est", east), ("Vide", 0))
        ]
        return error is None and result["rows"] == want and not result["truncated"] and result["feature_count"] == 3
    if label == "pondere":
        sums = [(r["label"], r["values"]["count"]["value"], r["values"]["weighted_sum"]) for r in result["rows"]] if error is None else []
        want = [("Ouest", west, 2.5 * west), ("Est", east, 2.5 * east), ("Vide", 0, 0.0)]
        return error is None and [(a, b, c["value"]) for a, b, c in sums] == want and all(c["unit"] is None for _, _, c in sums)
    if label == "grille":
        return (
            error is None
            and len(result["rows"]) == 100
            and result["truncated"]
            and result["feature_count"] == 120
            and all(r["values"]["count"]["value"] == 0 for r in result["rows"])
        )
    return error is not None and error.code == "analysis.bad_params"  # points en zones, champ texte


def next_step():
    i = state["i"]
    if i >= len(steps):
        QCoreApplication.quit()
        return
    label, params = steps[i]
    params = dict(params, out=os.path.join(out_dir, "%d-analysis.count_points_in_polygon.gpkg" % (i + 1)))

    def done(result=None, error=None):
        results.append((label, result if error is None else error.code, check(label, result, error)))
        state["i"] += 1
        QTimer.singleShot(0, guarded)

    state["gesture"] = analysis.ProcessingGesture(analysis.ALGORITHMS["analysis.count_points_in_polygon"])
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
