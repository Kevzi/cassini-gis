"""Sonde manuelle sous le vrai QGIS : la chaîne fixe d'une ligne de la table fermée (AD-24).

À lancer avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`), sur la série 3
(LTR 3.44) et sur la série 4 si elle est installée :

    "C:\\Program Files\\QGIS 3.44.15\\bin\\python-qgis-ltr.bat" qgis-plugin/tests/probe_chain.py

Elle n'a besoin d'aucune donnée : une couche de trois points en EPSG:2154 est écrite en GeoJSON dans un
dossier temporaire.

1. une ligne de test à deux étapes (`native:buffer` fusionné puis `native:convexhull`) : le geste rend une seule
   couche d'une entité, et le dossier de sortie ne contient que le fichier que le noyau a nommé (la
   première étape est un temporaire de QGIS, jamais écrit à côté) ;
2. la ligne `analysis.buffer` avec `work_epsg` (couche en degrés) : `native:reprojectlayer` puis
   `native:buffer`, toujours un seul fichier ;
3. annulation en cours de chaîne (10 000 points, tampon fusionné) : `qgis.cancelled`, aucun fichier.

Attendu : `RÉSULTAT : OK`.
"""

import json
import os
import sys
import tempfile
import traceback

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "qgis-plugin", "cassini_qgis"))

from qgis.analysis import QgsNativeAlgorithms  # noqa: E402
from qgis.core import Qgis, QgsApplication, QgsProject, QgsVectorLayer  # noqa: E402
from qgis.PyQt.QtCore import QCoreApplication, QTimer  # noqa: E402

app = QgsApplication([], False)
QgsApplication.initQgis()
QgsApplication.processingRegistry().addProvider(QgsNativeAlgorithms())
import analysis  # noqa: E402

GIVE_UP_MS = 30000
work = tempfile.mkdtemp()


def write_points(name, epsg, coords):
    path = os.path.join(work, name)
    collection = {
        "type": "FeatureCollection",
        "crs": {"type": "name", "properties": {"name": "urn:ogc:def:crs:EPSG::%d" % epsg}},
        "features": [
            {"type": "Feature", "properties": {"n": i}, "geometry": {"type": "Point", "coordinates": list(c)}}
            for i, c in enumerate(coords)
        ],
    }
    with open(path, "w", encoding="utf-8") as f:
        json.dump(collection, f)
    layer = QgsVectorLayer(path, name, "ogr")
    assert layer.isValid(), path
    QgsProject.instance().addMapLayer(layer)
    return layer


metric = write_points("points-2154.geojson", 2154, [(700000, 6600000), (700500, 6600000), (700000, 6600500)])
# Assez de points pour que le tampon fusionné dure plus que le délai d'annulation.
heavy = write_points(
    "lourd-2154.geojson", 2154, [(600000 + 700 * (i % 100), 6500000 + 700 * (i // 100)) for i in range(10000)]
)
degrees = write_points("points-4326.geojson", 4326, [(3.0, 46.5), (3.001, 46.5), (3.0, 46.501)])

# Ligne de test : tampon puis enveloppe convexe, la première étape en temporaire.
TEST_ROW = {
    "chain_version": 1,
    "chain": [
        {"algorithm": "native:buffer", "translate": analysis.translate_buffer, "output": "OUTPUT"},
        {
            "algorithm": "native:convexhull",
            "translate": lambda params, layer: {"INPUT": layer},
            "output": "OUTPUT",
        },
    ],
    "input": "layer_id",
    "metric": True,
}
DISTANCE = {"value": 50, "unit": "m"}

outputs = tempfile.mkdtemp()
results = []
state = {"i": 0}


def files_in(folder):
    return sorted(os.listdir(folder))


def check_two_steps(folder, out, result, error):
    return (
        error is None
        and result["feature_count"] == 1
        and files_in(folder) == [os.path.basename(out)]
    )


def check_work_crs(folder, out, result, error):
    return error is None and result["feature_count"] == 3 and files_in(folder) == [os.path.basename(out)]


def check_cancel(folder, out, result, error):
    return error is not None and error.code == "qgis.cancelled" and files_in(folder) == []


CASES = [
    ("deux étapes", TEST_ROW, lambda: {"layer_id": metric.id(), "distance": DISTANCE, "dissolve": True}, None, check_two_steps),
    (
        "reprojection puis tampon",
        analysis.ALGORITHMS["analysis.buffer"],
        lambda: {"layer_id": degrees.id(), "distance": {"value": 20, "unit": "m"}, "work_epsg": 32631},
        None,
        check_work_crs,
    ),
    (
        "annulation en cours de chaîne",
        TEST_ROW,
        lambda: {"layer_id": heavy.id(), "distance": {"value": 4, "unit": "km"}, "dissolve": True},
        200,
        check_cancel,
    ),
]


def next_case():
    try:
        _next_case()
    except Exception:
        traceback.print_exc()
        QCoreApplication.quit()


def _next_case():
    i = state["i"]
    if i >= len(CASES):
        QCoreApplication.quit()
        return
    label, row, make_params, cancel_after, check = CASES[i]
    folder = tempfile.mkdtemp(dir=outputs)
    out = os.path.join(folder, "%d-test.chain.gpkg" % (i + 1))
    params = make_params()
    params["out"] = out
    gesture = analysis.ProcessingGesture(row)
    state["gesture"] = gesture  # le geste vit tant que son travail dure

    def done(result=None, error=None):
        try:
            ok = bool(check(folder, out, result, error))
            detail = error.code if error is not None else {k: result[k] for k in ("feature_count", "crs")}
            results.append((label, ok, detail, files_in(folder)))
        except Exception:
            results.append((label, False, traceback.format_exc(), []))
        state["i"] += 1
        QTimer.singleShot(0, next_case)

    gesture.start(params, done)
    if cancel_after is not None:
        QTimer.singleShot(cancel_after, gesture.cancel)


QTimer.singleShot(0, next_case)
QTimer.singleShot(GIVE_UP_MS, QCoreApplication.quit)
QCoreApplication.exec()

print("QGIS %s" % Qgis.version())
for label, ok, detail, files in results:
    print("%s : %s  %s  %s" % ("OK " if ok else "ECHEC", label, detail, files))
good = len(results) == len(CASES) and all(ok for _, ok, _, _ in results)
print("RÉSULTAT : %s" % ("OK" if good else "ECHEC"))
QgsApplication.exitQgis()
sys.exit(0 if good else 1)
