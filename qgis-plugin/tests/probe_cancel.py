"""Sonde manuelle sous le vrai QGIS : une analyse annulée ne laisse aucun fichier.

À lancer avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`) :

    "C:\Program Files\QGIS 3.44.15\bin\python-qgis-ltr.bat" qgis-plugin/tests/probe_cancel.py

Elle lance `analysis.buffer` (distance de 50 km, fusion) sur la plus grosse couche de
`../donnees-test-cassini/fr/villeneuve-le-roi-2154.gpkg`, annule après 1,5 s, attend la
réponse du geste, puis liste le dossier de sortie. Attendu : `qgis.cancelled` et dossier vide.
Le jeu de données vit hors du dépôt (voir CLAUDE.md §5).
"""

import os
import sys
import tempfile
import time

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "qgis-plugin", "cassini_qgis"))
DATA = os.path.normpath(os.path.join(ROOT, "..", "donnees-test-cassini", "fr", "villeneuve-le-roi-2154.gpkg"))

from qgis.analysis import QgsNativeAlgorithms  # noqa: E402
from qgis.core import QgsApplication, QgsProject, QgsVectorLayer  # noqa: E402
from qgis.PyQt.QtCore import QCoreApplication, QTimer  # noqa: E402

app = QgsApplication([], False)
QgsApplication.initQgis()
QgsApplication.processingRegistry().addProvider(QgsNativeAlgorithms())
import analysis  # noqa: E402

CANCEL_AFTER_MS = 1500
GIVE_UP_MS = 15000

container = QgsVectorLayer(DATA, "x", "ogr")  # garder la référence : le fournisseur en dépend
biggest = None
for sub in container.dataProvider().subLayers():
    name = sub.split("!!::!!")[1]
    layer = QgsVectorLayer("%s|layername=%s" % (DATA, name), name, "ogr")
    if layer.isValid() and (biggest is None or layer.featureCount() > biggest.featureCount()):
        biggest = layer
print("couche :", biggest.name(), biggest.featureCount())
QgsProject.instance().addMapLayer(biggest)

out_dir = tempfile.mkdtemp()
out = os.path.join(out_dir, "1-analysis.buffer.gpkg")
answer = {}
t0 = time.time()


def done(result=None, error=None):
    answer.update(result=result, error=error and (error.code, error.message), seconds=round(time.time() - t0, 1))
    QCoreApplication.quit()


gesture = analysis.ProcessingGesture(analysis.ALGORITHMS["analysis.buffer"])
gesture.start({"layer_id": biggest.id(), "distance": {"value": 50, "unit": "km"}, "dissolve": True, "out": out}, done)


def cancel():
    print("sortie présente avant annulation :", os.path.exists(out), "| réponse déjà donnée :", bool(answer))
    gesture.cancel()


QTimer.singleShot(CANCEL_AFTER_MS, cancel)
QTimer.singleShot(GIVE_UP_MS, QCoreApplication.quit)
QCoreApplication.exec()

files = sorted(os.listdir(out_dir))
print("réponse :", answer, "| fichiers restants :", files)
ok = answer.get("error", (None,))[0] == "qgis.cancelled" and not files
print("RÉSULTAT :", "OK" if ok else "ÉCHEC")
sys.exit(0 if ok else 1)
