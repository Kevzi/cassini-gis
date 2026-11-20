"""Sonde manuelle sous le vrai QGIS : surface des entités (ticket 22, `analysis.area`).

À lancer avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`) :

    "C:\\Program Files\\QGIS 3.44.15\\bin\\python-qgis-ltr.bat" qgis-plugin/tests/probe_area.py

Aucun noyau ni aucune donnée hors dépôt : couches mémoire synthétiques, vrai geste
`analysis.area`. Termine par `VERDICT : OK` ou `VERDICT : ÉCHEC`.

Ce que la sonde constate (le plan interdit de le supposer) :
1. Carré de 100 m en Lambert-93 : `area_m2` = 10 000 en `m2`, `area_ha` = 1 en `ha`.
2. Couche en degrés (WGS84) avec `work_epsg` 2154 : surface en mètres carrés (à 1 % près), système de
   la sortie EPSG:2154.
3. Entité multipartie : la surface est la somme des parties.
4. Champ déjà présent : `area_m2` d'une couche qui le porte est remplacé sur place (un seul champ de
   ce nom, valeur recalculée) ; `area_ha` s'ajoute à côté sans toucher `area_m2`.
5. Géométrie nulle : surface nulle (NULL), pas d'erreur, l'entité est gardée.
6. Enchaînement : `analysis.area` puis `analysis.select_by_attribute` sur `area_m2 gt 2500` ne garde
   que les entités de plus de 2 500 m².
7. Lignes et points : refus avant lancement ; les originaux ne sont jamais modifiés.
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
    QgsCoordinateTransform,
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

work = tempfile.mkdtemp(prefix="cassini-area-")
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


def out(name):
    return os.path.join(work, name)


def run_gesture(command, params, label, then):
    gesture = analysis.ProcessingGesture(analysis.ALGORITHMS[command])

    def done(result=None, error=None):
        outcomes[label] = (result, error)
        QTimer.singleShot(0, protect(then))

    outcomes["_g_" + label] = gesture  # gardé en vie pendant la tâche
    gesture.start(params, done)


def ring(x, y=0, size=100):
    x0, y0 = 900000 + x, 6500000 + y
    return "%d %d,%d %d,%d %d,%d %d,%d %d" % (
        x0, y0, x0 + size, y0, x0 + size, y0 + size, x0, y0 + size, x0, y0
    )


def memory_layer(spec, name, rows, crs="EPSG:2154"):
    layer = QgsVectorLayer(spec + "&crs=" + crs, name, "memory")
    feats = []
    for attrs, wkt in rows:
        f = QgsFeature(layer.fields())
        f.setAttributes(attrs)
        if wkt is not None:
            f.setGeometry(QgsGeometry.fromWkt(wkt))
        feats.append(f)
    layer.dataProvider().addFeatures(feats)
    project.addMapLayer(layer)
    return layer


def read(path):
    layer = QgsVectorLayer(path, "lecture", "ogr")
    return layer, list(layer.getFeatures())


# Parcelles : carré de 100 m (10 000 m²), carré de 40 m (1 600 m²), multipartie de deux carrés de 100 m
# (20 000 m²), et une entité sans géométrie.
parcels = memory_layer(
    "MultiPolygon?field=nom:string",
    "parcelles",
    [
        (["p0"], "MULTIPOLYGON(((%s)))" % ring(0)),
        (["p1"], "MULTIPOLYGON(((%s)))" % ring(500, 0, 40)),
        (["p2"], "MULTIPOLYGON(((%s)),((%s)))" % (ring(1000), ring(1500))),
        (["p3"], None),
    ],
)
# Même couche mais portant déjà `area_m2` (valeur fausse, 1).
with_field = memory_layer(
    "MultiPolygon?field=nom:string&field=area_m2:double",
    "deja_surface",
    [(["q0", 1.0], "MULTIPOLYGON(((%s)))" % ring(0))],
)
# Un carré de 100 m de Lambert-93 passé en WGS84.
transform = QgsCoordinateTransform(
    QgsCoordinateReferenceSystem("EPSG:2154"), QgsCoordinateReferenceSystem("EPSG:4326"), project
)
square = QgsGeometry.fromWkt("MULTIPOLYGON(((%s)))" % ring(0))
square.transform(transform)
degrees = memory_layer("MultiPolygon?field=nom:string", "parcelles_wgs", [(["d0"], square.asWkt())], "EPSG:4326")
lines = memory_layer("LineString?field=nom:string", "lignes", [(["l0"], "LINESTRING(900000 6500000,900100 6500000)")])
points = memory_layer("Point?field=nom:string", "points", [(["g0"], "POINT(900000 6500000)")])


def area_of(feats, field, nom):
    return next(f[field] for f in feats if f["nom"] == nom)


def step_m2():
    run_gesture(
        "analysis.area",
        {"layer_id": parcels.id(), "unit": "m2", "out": out("1-analysis.area.gpkg")},
        "m2",
        step_m2_done,
    )


def step_m2_done():
    result, error = outcomes["m2"]
    check("m2 : pas d'erreur", error is None, error and (error.code, error.message))
    if result is None:
        finish()
        return
    check("m2 : résultat nommant le champ et l'unité",
          (result["field"], result["unit"], result["feature_count"]) == ("area_m2", "m2", 4), result)
    layer, feats = read(out("1-analysis.area.gpkg"))
    names = [f.name() for f in layer.fields()]
    print("       champs de la sortie : %s" % names)
    check("m2 : carré de 100 m = 10 000", round(area_of(feats, "area_m2", "p0"), 3) == 10000.0,
          area_of(feats, "area_m2", "p0"))
    check("m2 : carré de 40 m = 1 600", round(area_of(feats, "area_m2", "p1"), 3) == 1600.0)
    check("m2 : multipartie = somme des parties (20 000)", round(area_of(feats, "area_m2", "p2"), 3) == 20000.0)
    check("m2 : géométrie nulle -> surface NULL, entité gardée",
          area_of(feats, "area_m2", "p3") in (None,) or str(area_of(feats, "area_m2", "p3")) == "NULL",
          area_of(feats, "area_m2", "p3"))
    check("m2 : les originaux sont intacts",
          parcels.featureCount() == 4 and [f.name() for f in parcels.fields()] == ["nom"])
    step_ha()


def step_ha():
    run_gesture(
        "analysis.area",
        {"layer_id": parcels.id(), "unit": "ha", "out": out("2-analysis.area.gpkg")},
        "ha",
        step_ha_done,
    )


def step_ha_done():
    result, error = outcomes["ha"]
    check("ha : pas d'erreur", error is None, error and (error.code, error.message))
    if result is None:
        finish()
        return
    check("ha : résultat nommant le champ et l'unité", (result["field"], result["unit"]) == ("area_ha", "ha"), result)
    _, feats = read(out("2-analysis.area.gpkg"))
    check("ha : carré de 100 m = 1 ha", round(area_of(feats, "area_ha", "p0"), 6) == 1.0)
    check("ha : multipartie = 2 ha", round(area_of(feats, "area_ha", "p2"), 6) == 2.0)
    check("ha : carré de 40 m = 0,16 ha (aucun arrondi à l'unité)", round(area_of(feats, "area_ha", "p1"), 6) == 0.16)
    step_degrees()


def step_degrees():
    run_gesture(
        "analysis.area",
        {"layer_id": degrees.id(), "unit": "m2", "work_epsg": 2154, "out": out("3-analysis.area.gpkg")},
        "degres",
        step_degrees_done,
    )


def step_degrees_done():
    result, error = outcomes["degres"]
    check("degrés : pas d'erreur avec work_epsg", error is None, error and (error.code, error.message))
    if result is None:
        finish()
        return
    check("degrés : sortie en EPSG:2154", result["crs"] == "EPSG:2154", result)
    _, feats = read(out("3-analysis.area.gpkg"))
    value = area_of(feats, "area_m2", "d0")
    check("degrés : surface en mètres carrés (10 000 à 1 % près)", abs(value - 10000.0) < 100.0, value)
    run_gesture(
        "analysis.area",
        {"layer_id": degrees.id(), "unit": "m2", "out": out("3b-analysis.area.gpkg")},
        "degres_sans",
        step_existing,
    )


def step_existing():
    result, error = outcomes["degres_sans"]
    check("degrés sans work_epsg : analysis.crs_not_metric (le noyau le remplit en vrai)",
          error is not None and error.code == "analysis.crs_not_metric", error and error.code)
    run_gesture(
        "analysis.area",
        {"layer_id": with_field.id(), "unit": "m2", "out": out("4-analysis.area.gpkg")},
        "existant",
        step_existing_done,
    )


def step_existing_done():
    result, error = outcomes["existant"]
    check("champ déjà présent : pas d'erreur", error is None, error and (error.code, error.message))
    if result is None:
        finish()
        return
    layer, feats = read(out("4-analysis.area.gpkg"))
    names = [f.name() for f in layer.fields()]
    print("       champs de la sortie : %s" % names)
    check("champ déjà présent : un seul champ area_m2", names.count("area_m2") == 1, names)
    check("champ déjà présent : valeur recalculée (10 000, non 1)", round(area_of(feats, "area_m2", "q0"), 3) == 10000.0,
          area_of(feats, "area_m2", "q0"))
    check("champ déjà présent : l'original garde sa valeur", next(with_field.getFeatures())["area_m2"] == 1.0)
    run_gesture(
        "analysis.area",
        {"layer_id": with_field.id(), "unit": "ha", "out": out("5-analysis.area.gpkg")},
        "existant_ha",
        step_existing_ha_done,
    )


def step_existing_ha_done():
    result, error = outcomes["existant_ha"]
    check("area_ha à côté d'area_m2 : pas d'erreur", error is None, error and (error.code, error.message))
    if result is None:
        finish()
        return
    layer, feats = read(out("5-analysis.area.gpkg"))
    names = [f.name() for f in layer.fields()]
    check("area_ha à côté d'area_m2 : les deux champs, area_m2 intact (1)",
          "area_ha" in names and area_of(feats, "area_m2", "q0") == 1.0, names)
    step_chain()


def step_chain():
    layer_id = outcomes["m2"][0]["layer_id"]
    run_gesture(
        "analysis.select_by_attribute",
        {"layer_id": layer_id, "field": "area_m2", "operator": "gt", "value": "2500",
         "out": out("6-analysis.select_by_attribute.gpkg")},
        "chaine",
        step_chain_done,
    )


def step_chain_done():
    result, error = outcomes["chaine"]
    check("enchaînement : sélection area_m2 gt 2500 sans erreur", error is None, error and (error.code, error.message))
    if result is None:
        finish()
        return
    _, feats = read(out("6-analysis.select_by_attribute.gpkg"))
    check("enchaînement : seules p0 et p2 restent", sorted(f["nom"] for f in feats) == ["p0", "p2"],
          sorted(f["nom"] for f in feats))
    step_lines()


def step_lines():
    run_gesture(
        "analysis.area",
        {"layer_id": lines.id(), "unit": "m2", "out": out("7-analysis.area.gpkg")},
        "lignes",
        step_points,
    )


def step_points():
    _, error = outcomes["lignes"]
    check("lignes : refus avant lancement (analysis.bad_params)", error is not None and error.code == "analysis.bad_params",
          error and error.code)
    run_gesture(
        "analysis.area",
        {"layer_id": points.id(), "unit": "m2", "out": out("8-analysis.area.gpkg")},
        "points",
        step_last,
    )


def step_last():
    _, error = outcomes["points"]
    check("points : refus avant lancement (analysis.bad_params)", error is not None and error.code == "analysis.bad_params",
          error and error.code)
    check("refus : aucun fichier écrit", not os.path.exists(out("7-analysis.area.gpkg"))
          and not os.path.exists(out("8-analysis.area.gpkg")))
    finish()


QTimer.singleShot(0, protect(step_m2))
QTimer.singleShot(120000, lambda: (check("sonde terminée à temps", False), finish()))
app.exec()

failed = [label for label, ok in checks if not ok]
verdict = bool(checks) and not failed
print("VERDICT :", "OK" if verdict else "ÉCHEC (%s)" % ", ".join(failed))
QgsApplication.exitQgis()
sys.exit(0 if verdict else 1)
