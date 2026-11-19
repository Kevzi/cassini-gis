"""Sonde manuelle sous le vrai QGIS : statistiques par catégorie (ticket 11, `analysis.statistics_by_category`).

À lancer avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`) :

    "C:\\Program Files\\QGIS 3.44.15\\bin\\python-qgis-ltr.bat" qgis-plugin/tests/probe_statistics_by_category.py

Aucun noyau ni aucune donnée hors dépôt : des couches mémoire synthétiques. La chaîne passe par
`qgis:statisticsbycategories` (fournisseur « qgis » du module Processing, chargé ici comme QGIS le
charge) et, avec `sum_field`, par `native:fieldcalculator`. Termine par `RÉSULTAT : OK` ou `ÉCHEC`.

Ce que la sonde établit (les noms d'algorithme et de colonne ne sont jamais supposés) :
1. Sans `sum_field` : la table CSV écrite porte la catégorie, puis `count` (colonne 2), une ligne par
   catégorie ; les entités à catégorie nulle forment une ligne, étiquetée par son rang.
2. Avec `sum_field` : `sum` est la colonne 7 ; les entités à valeur nulle sont comptées (le décompte
   est exact) et valent 0 dans la somme, grâce à la préparation du champ.
3. Un champ à sommer qui n'est pas numérique est refusé avant tout fichier écrit.
4. 150 catégories : 100 lignes dans le résultat, `truncated` vrai, le fichier garde les 150.
5. Le tableau n'est jamais ajouté au projet.
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
# Le fournisseur « qgis » (statisticsbycategories) est celui du module Processing de QGIS.
plugins = os.path.join(QgsApplication.prefixPath(), "python", "plugins")
if os.path.isdir(plugins) and plugins not in sys.path:
    sys.path.append(plugins)
from processing.core.Processing import Processing  # noqa: E402

Processing.initialize()
import analysis  # noqa: E402

work = tempfile.mkdtemp(prefix="cassini-stats-")
project = QgsProject.instance()
checks = []
outcomes = {}

ENTRY = analysis.ALGORITHMS["analysis.statistics_by_category"]
# (catégorie, valeur, entier) : la catégorie nulle et la valeur nulle sont voulues.
DATA = [("a", 1.5, 1), ("a", 2.0, 1), ("b", 3.0, 2), (None, 4.0, 2), ("c", None, 3)]


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


def points_layer(name, rows, extra="&field=cat:string&field=v:double&field=n:integer&field=texte:string"):
    layer = QgsVectorLayer("Point?crs=EPSG:2154" + extra, name, "memory")
    features = []
    for rank, (cat, value, number) in enumerate(rows):
        f = QgsFeature(layer.fields())
        f.setAttributes([cat, value, number, "t%d" % rank])
        f.setGeometry(QgsGeometry.fromPointXY(QgsPointXY(900000 + rank, 6500000)))
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


def by_label(result):
    return {row["label"]: row["values"] for row in result["rows"]}


layers_before = len(project.mapLayers())
simple = points_layer("p", DATA)


# -- 1. Décomptes seuls ----------------------------------------------------------------------
def step_counts():
    gesture({"layer_id": simple.id(), "category_field": "cat", "out": out("1-stats.csv")}, "counts", step_counts_done)


def step_counts_done():
    result, error, path = outcomes["counts"]
    check("décomptes : pas d'erreur", error is None, error and error.message)
    check("décomptes : le résultat est file, rows, truncated",
          result and sorted(result) == ["file", "rows", "truncated"], result)
    check("décomptes : file est le fichier du noyau", result and result["file"] == path, result)
    check("décomptes : le fichier existe", os.path.isfile(path))
    rows = by_label(result)
    # La catégorie nulle est étiquetée par son rang (4e ligne de la table : la 3e des catégories).
    counts = {label: v["count"]["value"] for label, v in rows.items()}
    check("décomptes : a=2, b=1, c=1 et une ligne pour la catégorie nulle",
          counts.get("a") == 2 and counts.get("b") == 1 and counts.get("c") == 1 and len(counts) == 4, counts)
    check("décomptes : unité des décomptes", all(v["count"]["unit"] == "entités" for v in rows.values()), rows)
    check("décomptes : sans somme, rien d'autre que count", all(list(v) == ["count"] for v in rows.values()), rows)
    check("décomptes : pas tronqué", result and result["truncated"] is False, result)
    step_sum()


# -- 2. Avec somme ----------------------------------------------------------------------------
def step_sum():
    gesture(
        {"layer_id": simple.id(), "category_field": "cat", "sum_field": "v", "out": out("2-stats-somme.csv")},
        "somme",
        step_sum_done,
    )


def step_sum_done():
    result, error, path = outcomes["somme"]
    check("somme : pas d'erreur", error is None, error and error.message)
    rows = by_label(result) if result else {}
    check("somme : a = 2 entités, somme 3,5", rows.get("a", {}).get("sum", {}).get("value") == 3.5
          and rows["a"]["count"]["value"] == 2, rows)
    check("somme : b = 1 entité, somme 3", rows.get("b", {}).get("sum", {}).get("value") == 3.0, rows)
    check("somme : l'entité à valeur nulle est comptée, somme 0",
          rows.get("c", {}).get("count", {}).get("value") == 1 and rows["c"]["sum"]["value"] == 0, rows)
    check("somme : la somme est sans unité", all(v["sum"]["unit"] is None for v in rows.values()), rows)
    with open(path, encoding="utf-8") as f:
        header = f.readline().strip().split(",")
    check("somme : colonnes de la table (catégorie, count, ..., sum en 7e)",
          header[0] == "cat" and header[1] == "count" and header[6] == "sum", header)
    step_integer()


# -- 3. Champ de catégorie entier, somme d'un champ entier ------------------------------------------
def step_integer():
    gesture(
        {"layer_id": simple.id(), "category_field": "n", "sum_field": "n", "out": out("3-stats-entier.csv")},
        "entier",
        step_integer_done,
    )


def step_integer_done():
    result, error, _ = outcomes["entier"]
    check("entier : pas d'erreur", error is None, error and error.message)
    rows = by_label(result) if result else {}
    check("entier : catégorie 2 = 2 entités, somme 4",
          rows.get("2", {}).get("count", {}).get("value") == 2 and rows["2"]["sum"]["value"] == 4, rows)
    step_text()


# -- 4. Champ à sommer non numérique ---------------------------------------------------------------
def step_text():
    gesture(
        {"layer_id": simple.id(), "category_field": "cat", "sum_field": "texte", "out": out("4-stats-texte.csv")},
        "texte",
        step_text_done,
    )


def step_text_done():
    result, error, path = outcomes["texte"]
    check("texte : refusé avant tout lancement", result is None and error is not None
          and error.code == analysis.BAD_PARAMS, error and error.code)
    check("texte : aucun fichier écrit", not os.path.exists(path))
    step_truncated()


# -- 5. Plus de 100 catégories ----------------------------------------------------------------------
def step_truncated():
    many = points_layer("beaucoup", [("c%03d" % i, 1.0, i) for i in range(150)])
    gesture({"layer_id": many.id(), "category_field": "cat", "out": out("5-stats-beaucoup.csv")}, "beaucoup", step_truncated_done)


def step_truncated_done():
    result, error, path = outcomes["beaucoup"]
    check("150 catégories : pas d'erreur", error is None, error and error.message)
    check("150 catégories : 100 lignes, tronqué", result and len(result["rows"]) == 100 and result["truncated"] is True,
          result and (len(result["rows"]), result["truncated"]))
    table = QgsVectorLayer(path, "lecture", "ogr")
    check("150 catégories : le fichier garde les 150", table.isValid() and table.featureCount() == 150,
          table.featureCount())
    del table
    check("le tableau n'est jamais ajouté au projet", len(project.mapLayers()) == layers_before + 2,
          (layers_before, len(project.mapLayers())))
    finish()


QTimer.singleShot(0, protect(step_counts))
QTimer.singleShot(120000, lambda: (check("sonde terminée à temps", False), finish()))
app.exec()

failed = [label for label, ok in checks if not ok]
print("RÉSULTAT : %s" % ("OK" if checks and not failed else "ÉCHEC (%s)" % ", ".join(failed)))
QgsApplication.exitQgis()
sys.exit(0 if checks and not failed else 1)
