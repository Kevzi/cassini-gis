"""Sonde manuelle sous le vrai QGIS : `data.transform_point`.

À lancer avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`) :

    "C:\\Program Files\\QGIS 3.44.15\\bin\\python-qgis-ltr.bat" qgis-plugin/tests/probe_point_transform.py

Sans projet ouvert ni jeu de données : reprojette par le vrai QGIS l'origine du Lambert-93
(φ0 = 46°30' N, λ0 = 3° E, qui vaut par définition E = 700 000 m, N = 6 600 000 m) et vérifie
le résultat à 1 cm près ; puis le retour en WGS84 à 1e-9° près ; puis un même système rend le
point tel quel ; puis un code cible ou source inconnu (EPSG:99999) rend `data.unknown_crs` en
nommant le code ; puis un point hors du domaine rend `data.transform_failed`. Chaque geste ne
répond qu'une fois.
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "qgis-plugin", "cassini_qgis"))

from qgis.core import QgsApplication  # noqa: E402
from qgis.PyQt.QtCore import QCoreApplication, QTimer  # noqa: E402

app = QgsApplication([], False)
QgsApplication.initQgis()
import point_transform  # noqa: E402

ORIGIN_WGS84 = {"x": 3.0, "y": 46.5, "epsg": 4326}
CASES = []  # (nom, paramètres, vérification(result, error) -> bool)


def near(result, x, y, epsg, tolerance):
    if result is None:
        return False
    p = result["point"]
    return p["epsg"] == epsg and abs(p["x"] - x) <= tolerance and abs(p["y"] - y) <= tolerance


def failed_with(code, named=None):
    def check(result, error):
        return result is None and error is not None and error.code == code and (
            named is None or named in error.message
        )

    return check


CASES.append(("origine du Lambert-93", {"point": ORIGIN_WGS84, "target_epsg": 2154},
              lambda r, e: e is None and near(r, 700000.0, 6600000.0, 2154, 0.01)))
CASES.append(("retour en WGS84", {"point": {"x": 700000.0, "y": 6600000.0, "epsg": 2154}, "target_epsg": 4326},
              lambda r, e: e is None and near(r, 3.0, 46.5, 4326, 1e-9)))
CASES.append(("même système", {"point": {"x": 652000.25, "y": 6853000.5, "epsg": 2154}, "target_epsg": 2154},
              lambda r, e: e is None and r == {"point": {"x": 652000.25, "y": 6853000.5, "epsg": 2154}}))
CASES.append(("cible inconnue", {"point": ORIGIN_WGS84, "target_epsg": 99999},
              failed_with("data.unknown_crs", "EPSG:99999")))
CASES.append(("source inconnue", {"point": {"x": 1.0, "y": 2.0, "epsg": 99999}, "target_epsg": 4326},
              failed_with("data.unknown_crs", "EPSG:99999")))
# Une latitude de 95° : QGIS (PROJ) la refuse (« Invalid coordinate »). Un point Lambert-93
# très lointain, lui, rend un résultat fini près du pôle : ce n'est pas un cas d'échec.
CASES.append(("hors domaine", {"point": {"x": 0.0, "y": 95.0, "epsg": 4326}, "target_epsg": 2154},
              failed_with("data.transform_failed")))

results = []
alive = []


def run(index):
    if index == len(CASES):
        QCoreApplication.quit()
        return
    name, params, check = CASES[index]
    answers = []

    def done(result=None, error=None):
        answers.append((result, error))
        if len(answers) == 1:
            # Laisser passer un tour de boucle : une seconde réponse serait vue.
            QTimer.singleShot(50, lambda: finish())

    def finish():
        result, error = answers[0]
        shown = error.code + " : " + error.message if error else result
        results.append((name, shown, len(answers) == 1 and check(result, error)))
        run(index + 1)

    gesture = point_transform.PointTransformGesture()
    alive.append(gesture)
    gesture.start(params, done)


QTimer.singleShot(0, lambda: run(0))
QTimer.singleShot(30000, QCoreApplication.quit)  # garde-fou
app.exec_() if hasattr(app, "exec_") else app.exec()

ok = len(results) == len(CASES)
for name, shown, passed in results:
    print("%s  %s : %s" % ("OK " if passed else "ÉCHEC", name, shown))
    ok = ok and passed
if len(results) != len(CASES):
    print("ÉCHEC  %d cas sur %d joués" % (len(results), len(CASES)))
print("SONDE %s" % ("RÉUSSIE" if ok else "ÉCHOUÉE"))
QgsApplication.exitQgis()
sys.exit(0 if ok else 1)
