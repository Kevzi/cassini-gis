"""Sonde manuelle sous le vrai QGIS : jointure d'un tableau (ticket 23, `analysis.join_table`).

À lancer avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`) :

    "C:\\Program Files\\QGIS 3.44.15\\bin\\python-qgis-ltr.bat" qgis-plugin/tests/probe_join_table.py

Aucun noyau ni aucune donnée hors dépôt : un CSV synthétique écrit dans un dossier temporaire, ouvert
par le vrai geste `layer.add` (paramètre `table`), puis joint à une couche mémoire par le vrai geste
`analysis.join_table`. Termine par `VERDICT : OK` ou `VERDICT : ÉCHEC`.

Ce que la sonde constate (le plan interdit de le supposer) :
1. `layer.add` ouvre le CSV sans géométrie, sans SCR, tous les champs en texte : le code `01001`
   garde son zéro de tête ; le fichier d'origine n'est pas modifié.
2. La jointure garde une ligne par entité (`feature_count` = nombre de communes), une clé en double
   dans la table donne la première ligne, une commune sans ligne reste avec des colonnes vides.
3. `unmatched_count` (sortie `UNJOINABLE_COUNT` de l'algorithme) compte les communes sans ligne et
   non les lignes du tableau sans commune.
4. Collision de noms : une colonne de la table qui porte le nom d'un champ de la couche est
   renommée par QGIS (`nom_2`) ; la clé de la table est recopiée aussi (`code_2`).
5. Un champ inconnu, d'un côté ou de l'autre, est refusé (`analysis.unknown_field`) avant tout lancement.
"""

import hashlib
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
    QgsWkbTypes,
)
from qgis.PyQt.QtCore import QCoreApplication, QTimer  # noqa: E402

app = QgsApplication([], False)
QgsApplication.initQgis()
QgsApplication.processingRegistry().addProvider(QgsNativeAlgorithms())
import analysis  # noqa: E402
import layer_add  # noqa: E402

work = tempfile.mkdtemp(prefix="cassini-jointure-")
project = QgsProject.instance()
project.setCrs(QgsCoordinateReferenceSystem("EPSG:2154"))
checks = []
outcomes = {}

CSV_PATH = os.path.join(work, "prix.csv")
CSV_TEXT = (
    "code,nom,prix\n"
    "01001,Premier,100\n"
    "01001,Doublon,200\n"  # clé en double : la première ligne est retenue
    "02002,Deuxieme,300\n"
    "74010,Annecy,400\n"
    "99999,Sans commune,1\n"  # ligne sans commune : n'est pas comptée
)
COMMUNES = [("01001", "A"), ("02002", "B"), ("03003", "C"), ("74010", "D")]  # 03003 : sans ligne


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


def is_null(value):
    return value is None or (hasattr(value, "isNull") and value.isNull())


def out(name):
    return os.path.join(work, name)


def run_gesture(gesture, params, label, then):
    def done(result=None, error=None):
        outcomes[label] = (result, error)
        QTimer.singleShot(0, protect(then))

    outcomes["_g_" + label] = gesture  # gardé en vie pendant la tâche
    gesture.start(params, done)


def join(params, label, then):
    run_gesture(analysis.ProcessingGesture(analysis.ALGORITHMS["analysis.join_table"]), params, label, then)


with open(CSV_PATH, "w", encoding="utf-8", newline="") as handle:
    handle.write(CSV_TEXT)
with open(CSV_PATH, "rb") as handle:
    CSV_DIGEST = hashlib.sha256(handle.read()).hexdigest()

communes = QgsVectorLayer("Polygon?crs=EPSG:2154&field=code:string&field=nom:string", "communes", "memory")
features = []
for index, (code, name) in enumerate(COMMUNES):
    f = QgsFeature(communes.fields())
    f.setAttributes([code, name])
    x = 900000 + index * 2000
    f.setGeometry(
        QgsGeometry.fromWkt(
            "POLYGON((%d 6500000,%d 6500000,%d 6501000,%d 6501000,%d 6500000))" % (x, x + 1000, x + 1000, x, x)
        )
    )
    features.append(f)
communes.dataProvider().addFeatures(features)
project.addMapLayer(communes)
state = {}


# -- 1. layer.add ouvre le CSV comme tableau ---------------------------------------------------
def step_open():
    run_gesture(layer_add.LayerAddGesture(), {"table": CSV_PATH}, "ouverture", step_open_done)


def step_open_done():
    result, error = outcomes["ouverture"]
    check("ouverture : pas d'erreur", error is None, error and (error.code, error.message))
    if result is None:
        finish()
        return
    check(
        "ouverture : source_crs vide, non reprojetée",
        result["source_crs"] == "" and result["reprojected"] is False,
        result,
    )
    table = project.mapLayer(result["layer_id"])
    state["table"] = table
    check("ouverture : couche vecteur valide", table is not None and table.isValid())
    check("ouverture : sans géométrie", table.wkbType() == QgsWkbTypes.NoGeometry, table.wkbType())
    types = [(f.name(), f.typeName()) for f in table.fields()]
    check("ouverture : tous les champs en texte", all(t == "text" for _, t in types), types)
    codes = [f["code"] for f in table.getFeatures()]
    check("ouverture : le zéro de tête de 01001 est conservé", "01001" in codes, codes)
    check("ouverture : 5 lignes lues", table.featureCount() == 5, table.featureCount())
    with open(CSV_PATH, "rb") as handle:
        digest = hashlib.sha256(handle.read()).hexdigest()
    check("ouverture : le fichier d'origine n'est pas modifié", digest == CSV_DIGEST)
    step_join()


# -- 2 à 4. La jointure ---------------------------------------------------------------------
def step_join():
    join(
        {
            "layer_id": communes.id(),
            "table_layer_id": state["table"].id(),
            "field": "code",
            "table_field": "code",
            "out": out("1-analysis.join_table.gpkg"),
        },
        "jointure",
        step_join_done,
    )


def step_join_done():
    result, error = outcomes["jointure"]
    check("jointure : pas d'erreur", error is None, error and (error.code, error.message))
    if result is None:
        finish()
        return
    check("jointure : feature_count = 4 communes", result["feature_count"] == 4, result)
    check("jointure : unmatched_count = 1 (la commune 03003)", result["unmatched_count"] == 1, result)
    check("jointure : sortie en EPSG:2154", result["crs"] == "EPSG:2154", result)
    layer = QgsVectorLayer(out("1-analysis.join_table.gpkg"), "lecture", "ogr")
    names = [f.name() for f in layer.fields()]
    print("       champs de la sortie : %s" % names)
    check("jointure : colonne prix ajoutée", "prix" in names, names)
    check("jointure : colonne `nom` de la table renommée nom_2 (collision)", "nom_2" in names, names)
    check("jointure : clé de la table recopiée (code_2)", "code_2" in names, names)
    rows = {f["code"]: f for f in layer.getFeatures()}
    check(
        "jointure : 01001 a la première ligne du tableau (prix 100)",
        rows["01001"]["prix"] == "100",
        rows["01001"]["prix"],
    )
    check("jointure : 74010 a son prix", rows["74010"]["prix"] == "400", rows["74010"]["prix"])
    check(
        "jointure : 03003 est gardée, colonnes vides",
        is_null(rows["03003"]["prix"]) and is_null(rows["03003"]["nom_2"]),
        (rows["03003"]["prix"], rows["03003"]["nom_2"]),
    )
    check("jointure : la ligne 99999 sans commune n'ajoute aucune entité", "99999" not in rows)
    step_unknown_field()


# -- 5. Champ inconnu --------------------------------------------------------------------------
def step_unknown_field():
    join(
        {
            "layer_id": communes.id(),
            "table_layer_id": state["table"].id(),
            "field": "code",
            "table_field": "absent",
            "out": out("2-analysis.join_table.gpkg"),
        },
        "inconnu_table",
        step_unknown_field_done,
    )


def step_unknown_field_done():
    _, error = outcomes["inconnu_table"]
    check(
        "champ inconnu côté table : analysis.unknown_field",
        error is not None and error.code == "analysis.unknown_field",
        error and error.code,
    )
    check("champ inconnu : aucun fichier écrit", not os.path.exists(out("2-analysis.join_table.gpkg")))
    join(
        {
            "layer_id": communes.id(),
            "table_layer_id": state["table"].id(),
            "field": "absent",
            "table_field": "code",
            "out": out("3-analysis.join_table.gpkg"),
        },
        "inconnu_couche",
        step_last,
    )


def step_last():
    _, error = outcomes["inconnu_couche"]
    check(
        "champ inconnu côté couche : analysis.unknown_field",
        error is not None and error.code == "analysis.unknown_field",
        error and error.code,
    )
    finish()


QTimer.singleShot(0, protect(step_open))
QTimer.singleShot(120000, lambda: (check("sonde terminée à temps", False), finish()))
app.exec()

failed = [label for label, ok in checks if not ok]
verdict = bool(checks) and not failed
print("VERDICT :", "OK" if verdict else "ÉCHEC (%s)" % ", ".join(failed))
QgsApplication.exitQgis()
sys.exit(0 if verdict else 1)
