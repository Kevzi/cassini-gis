"""Sonde manuelle sous le vrai QGIS : fusion par champ (ticket 24, `analysis.dissolve`).

À lancer avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`) :

    "C:\\Program Files\\QGIS 3.44.15\\bin\\python-qgis-ltr.bat" qgis-plugin/tests/probe_dissolve.py

Aucun noyau ni aucune donnée hors dépôt : une couche mémoire synthétique de sept « parcelles »,
fusionnée par le vrai geste `analysis.dissolve`. Termine par `VERDICT : OK` ou `VERDICT : ÉCHEC`.

Ce que la sonde constate (le plan interdit de le supposer) :
1. Un champ : une entité par valeur, surface = somme des parcelles du groupe ; `feature_count` juste.
2. Deux champs : une entité par couple de valeurs.
3. Valeurs nulles : les entités dont le champ est nul forment un groupe à part (une entité).
4. Multiparties : des parcelles disjointes d'un même groupe donnent une seule entité (multipartie),
   `SEPARATE_DISJOINT=False`.
5. Attributs et `fid` : la sortie garde tous les champs de l'entrée (valeur de la première entité du
   groupe) ; la colonne `fid` du GeoPackage est celle de QGIS, renumérotée.
6. Un champ inconnu, même en seconde position, est refusé (`analysis.unknown_field`) avant tout
   lancement, sans fichier écrit.
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

work = tempfile.mkdtemp(prefix="cassini-fusion-")
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


def dissolve(params, label, then):
    run_gesture(analysis.ProcessingGesture(analysis.ALGORITHMS["analysis.dissolve"]), params, label, then)


# (commune, zone, x) : des carrés de 1000 m de côté, contigus quand les x se suivent.
PARCELS = [
    ("A", "U", 0),
    ("A", "U", 1000),
    ("B", "U", 5000),
    ("B", "N", 6000),
    ("B", "N", 9000),  # disjointe des précédentes : multipartie
    (None, "U", 12000),
    (None, "N", 13000),
]

parcels = QgsVectorLayer(
    "Polygon?crs=EPSG:2154&field=commune:string&field=zone:string&field=nom:string", "parcelles", "memory"
)
features = []
for index, (commune, zone, x) in enumerate(PARCELS):
    f = QgsFeature(parcels.fields())
    f.setAttributes([commune, zone, "p%d" % index])
    x0 = 900000 + x
    f.setGeometry(
        QgsGeometry.fromWkt(
            "POLYGON((%d 6500000,%d 6500000,%d 6501000,%d 6501000,%d 6500000))" % (x0, x0 + 1000, x0 + 1000, x0, x0)
        )
    )
    features.append(f)
parcels.dataProvider().addFeatures(features)
project.addMapLayer(parcels)


def read(path):
    layer = QgsVectorLayer(path, "lecture", "ogr")
    return layer, list(layer.getFeatures())


def step_one():
    dissolve(
        {"layer_id": parcels.id(), "fields": ["commune"], "out": out("1-analysis.dissolve.gpkg")},
        "un_champ",
        step_one_done,
    )


def step_one_done():
    result, error = outcomes["un_champ"]
    check("un champ : pas d'erreur", error is None, error and (error.code, error.message))
    if result is None:
        finish()
        return
    check("un champ : feature_count = 3 (A, B et nul)", result["feature_count"] == 3, result)
    check("un champ : sortie en EPSG:2154", result["crs"] == "EPSG:2154", result)
    layer, feats = read(out("1-analysis.dissolve.gpkg"))
    names = [f.name() for f in layer.fields()]
    print("       champs de la sortie : %s" % names)
    by = {f["commune"] if not is_null(f["commune"]) else None: f for f in feats}
    check("un champ : trois groupes lus", len(feats) == 3 and set(by) == {"A", "B", None}, list(by))
    area = {k: round(f.geometry().area()) for k, f in by.items()}
    check("un champ : surface de A = 2 parcelles", area.get("A") == 2000000, area)
    check("un champ : surface de B = 3 parcelles", area.get("B") == 3000000, area)
    check("un champ : le groupe nul = 2 parcelles, une seule entité", area.get(None) == 2000000, area)
    check("un champ : B (parcelles disjointes) est une seule entité multipartie", by["B"].geometry().isMultipart())
    check("un champ : tous les champs de l'entrée sont conservés", {"commune", "zone", "nom"} <= set(names), names)
    check("un champ : attributs non groupés = valeur de la première entité du groupe (A : zone U, nom p0)",
          by["A"]["zone"] == "U" and by["A"]["nom"] == "p0", (by["A"]["zone"], by["A"]["nom"]))
    check("un champ : B garde la valeur de sa première entité (zone U, nom p2)",
          by["B"]["zone"] == "U" and by["B"]["nom"] == "p2", (by["B"]["zone"], by["B"]["nom"]))
    check("un champ : fid renumérotée 1, 2, 3 (celle de l'entrée n'est pas reportée)",
          sorted(f.id() for f in feats) == [1, 2, 3], [f.id() for f in feats])
    check("un champ : la colonne fid du GeoPackage est le seul ajout aux champs de l'entrée",
          [n for n in names if n not in ("commune", "zone", "nom")] == ["fid"], names)
    check("un champ : le fichier de sortie n'écrase pas l'entrée", parcels.featureCount() == 7)
    step_two()


def step_two():
    dissolve(
        {
            "layer_id": parcels.id(),
            "fields": ["commune", "zone"],
            "out": out("2-analysis.dissolve.gpkg"),
        },
        "deux_champs",
        step_two_done,
    )


def step_two_done():
    result, error = outcomes["deux_champs"]
    check("deux champs : pas d'erreur", error is None, error and (error.code, error.message))
    if result is None:
        finish()
        return
    # (A,U) (B,U) (B,N) (nul,U) (nul,N)
    check("deux champs : feature_count = 5 couples", result["feature_count"] == 5, result)
    layer, feats = read(out("2-analysis.dissolve.gpkg"))
    area = {
        (None if is_null(f["commune"]) else f["commune"], f["zone"]): round(f.geometry().area()) for f in feats
    }
    check("deux champs : (A, U) = 2 parcelles", area.get(("A", "U")) == 2000000, area)
    check("deux champs : (B, N) = 2 parcelles disjointes", area.get(("B", "N")) == 2000000, area)
    step_unknown()


def step_unknown():
    dissolve(
        {
            "layer_id": parcels.id(),
            "fields": ["commune", "absent"],
            "out": out("3-analysis.dissolve.gpkg"),
        },
        "inconnu",
        step_last,
    )


def step_last():
    _, error = outcomes["inconnu"]
    check(
        "champ inconnu en seconde position : analysis.unknown_field",
        error is not None and error.code == "analysis.unknown_field",
        error and error.code,
    )
    check("champ inconnu : aucun fichier écrit", not os.path.exists(out("3-analysis.dissolve.gpkg")))
    finish()


QTimer.singleShot(0, protect(step_one))
QTimer.singleShot(120000, lambda: (check("sonde terminée à temps", False), finish()))
app.exec()

failed = [label for label, ok in checks if not ok]
verdict = bool(checks) and not failed
print("VERDICT :", "OK" if verdict else "ÉCHEC (%s)" % ", ".join(failed))
QgsApplication.exitQgis()
sys.exit(0 if verdict else 1)
