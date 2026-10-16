"""Sonde manuelle sous le vrai QGIS : `analysis.intersect` et `analysis.select_by_attribute`.

À lancer avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`) :

    "C:\\Program Files\\QGIS 3.44.15\\bin\\python-qgis-ltr.bat" qgis-plugin/tests/probe_intersect_select.py

Sur `../donnees-test-cassini/gares-2154.csv` (8 gares) : zone tampon de 500 m, puis
intersection des gares avec cette zone (attendu : 8), filtre `name` commence par « A »
(attendu : le décompte fait à la main sur le CSV), filtre sans résultat (attendu : 0).
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
from qgis.core import QgsApplication, QgsProject, QgsVectorLayer  # noqa: E402
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
    names = [row["name"] for row in csv.DictReader(f)]
expected_a = sum(1 for n in names if n.startswith("A"))

out_dir = tempfile.mkdtemp()
steps = [
    ("analysis.buffer", {"layer_id": gares.id(), "distance": {"value": 500, "unit": "m"}, "dissolve": True}, 1),  # fusion : une seule entité
    ("analysis.intersect", {"layer_id": gares.id(), "overlay_layer_id": "@buffer"}, len(names)),
    ("analysis.select_by_attribute", {"layer_id": gares.id(), "field": "name", "operator": "begins_with", "value": "A"}, expected_a),
    ("analysis.select_by_attribute", {"layer_id": gares.id(), "field": "name", "operator": "eq", "value": "zzz"}, 0),
    ("analysis.select_by_attribute", {"layer_id": gares.id(), "field": "absent", "operator": "eq", "value": "x"}, "analysis.unknown_field"),
]
results = []
state = {"i": 0, "buffer": None}


def next_step():
    i = state["i"]
    if i >= len(steps):
        QCoreApplication.quit()
        return
    command, params, expected = steps[i]
    params = dict(params)
    if params.get("overlay_layer_id") == "@buffer":
        params["overlay_layer_id"] = state["buffer"]
    params["out"] = os.path.join(out_dir, "%d-%s.gpkg" % (i + 1, command))

    def done(result=None, error=None):
        if command == "analysis.buffer" and result:
            state["buffer"] = result["layer_id"]
        got = error.code if error else result["feature_count"]
        results.append((command, expected, got, got == expected))
        state["i"] += 1
        QTimer.singleShot(0, guarded)

    state["gesture"] = analysis.ProcessingGesture(analysis.ALGORITHMS[command])
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

for row in results:
    print(row)
ok = len(results) == len(steps) and all(r[3] for r in results)
print("RÉSULTAT :", "OK" if ok else "ÉCHEC")
sys.exit(0 if ok else 1)
