"""Sonde manuelle sous le vrai QGIS : statistiques d'un raster par polygone (ticket 12,
`raster.zonal_statistics`).

À lancer avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`) :

    "C:\\Program Files\\QGIS 3.44.15\\bin\\python-qgis-ltr.bat" qgis-plugin/tests/probe_zonal_statistics.py

Aucun noyau ni aucune donnée hors dépôt : les MNT synthétiques de `fixtures/` (copiés dans un dossier
temporaire, pour comparer l'empreinte avant et après), des polygones en couche mémoire, le vrai geste
`raster.zonal_statistics`. Termine par `VERDICT : OK` ou `VERDICT : ÉCHEC`.

Ce que la sonde constate (le plan interdit de le supposer) :
1. Plan incliné (valeur = 99,5 - colonne, cellules de 10 m) : trois polygones alignés sur les cellules,
   moyenne, minimum et maximum exacts (calculés à la main), unité `m` par défaut.
2. Colline : un polygone aligné, comparé aux cellules lues une à une dans le raster par QGIS.
3. Unité déclarée (`unit`) : reprise telle quelle dans chaque valeur.
4. Un polygone entièrement hors de l'emprise du raster : une ligne sans valeur, les autres intactes.
5. Le fichier raster d'origine reste identique (empreinte SHA-256) et aucun `.aux.xml` n'apparaît.
6. Mauvais type de couche (points) : refus `analysis.bad_params`, aucun fichier écrit.
7. Raster absent : `layer.not_found` ; fichier non GeoTIFF : `layer.invalid_source`.
8. Systèmes différents (polygones en WGS84, raster en Lambert-93) : QGIS reprojette les polygones vers
   le système du raster, les statistiques sont exactes (celles du même carré en Lambert-93) et la couche
   produite reste en WGS84. Aucun refus.
"""

import hashlib
import os
import shutil
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
    QgsRasterLayer,
    QgsVectorLayer,
)
from qgis.PyQt.QtCore import QCoreApplication, QTimer  # noqa: E402

app = QgsApplication([], False)
QgsApplication.initQgis()
QgsApplication.processingRegistry().addProvider(QgsNativeAlgorithms())
import analysis  # noqa: E402

work = tempfile.mkdtemp(prefix="cassini-zonal-")
project = QgsProject.instance()
project.setCrs(QgsCoordinateReferenceSystem("EPSG:2154"))
checks = []
outcomes = {}

SLOPE_SRC = os.path.join(ROOT, "fixtures", "mnt-plan-incline-2154.tif")
HILL_SRC = os.path.join(ROOT, "fixtures", "mnt-colline-2154.tif")
SLOPE = os.path.join(work, "plan.tif")
HILL = os.path.join(work, "colline.tif")
shutil.copy(SLOPE_SRC, SLOPE)
shutil.copy(HILL_SRC, HILL)


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


def sha(path):
    with open(path, "rb") as handle:
        return hashlib.sha256(handle.read()).hexdigest()


def run_gesture(params, label, then, epsg_layer=None):
    gesture = analysis.ProcessingGesture(analysis.ALGORITHMS["raster.zonal_statistics"])

    def done(result=None, error=None):
        outcomes[label] = (result, error)
        QTimer.singleShot(0, protect(then))

    outcomes["_g_" + label] = gesture  # gardé en vie pendant la tâche
    gesture.start(params, done)


def box(x0, y0, x1, y1):
    return QgsGeometry.fromWkt("POLYGON((%s %s,%s %s,%s %s,%s %s,%s %s))" % (x0, y0, x1, y0, x1, y1, x0, y1, x0, y0))


def polygon_layer(name, boxes, crs="EPSG:2154"):
    layer = QgsVectorLayer("Polygon?crs=%s&field=nom:string" % crs, name, "memory")
    features = []
    for label, geometry in boxes:
        f = QgsFeature(layer.fields())
        f.setAttributes([label])
        f.setGeometry(geometry)
        features.append(f)
    layer.dataProvider().addFeatures(features)
    project.addMapLayer(layer)
    return layer


def hill_cells(x0, y0, x1, y1):
    """Valeurs des cellules de la colline dont le centre est dans le rectangle, lues par QGIS."""
    analysis._pam_off()  # la lecture de la sonde ne doit pas poser de .aux.xml elle non plus
    raster = QgsRasterLayer(HILL, "lecture", "gdal")
    extent = raster.extent()
    step = raster.rasterUnitsPerPixelX()
    block = raster.dataProvider().block(1, extent, raster.width(), raster.height())
    values = []
    for row in range(raster.height()):
        for col in range(raster.width()):
            cx = extent.xMinimum() + (col + 0.5) * step
            cy = extent.yMaximum() - (row + 0.5) * step
            if x0 <= cx <= x1 and y0 <= cy <= y1:
                values.append(block.value(row, col))
    del raster
    analysis._pam_restore()
    return values


# Plan incliné : x 652000 à 652400, y 6850000 à 6850300, cellules de 10 m, valeur 99,5 - colonne.
SLOPE_ZONES = [
    ("A", box(652000, 6850000, 652100, 6850100)),  # colonnes 0-9 : 99,5 à 90,5
    ("B", box(652100, 6850100, 652200, 6850300)),  # colonnes 10-19 : 89,5 à 80,5
    ("Loin", box(700000, 6900000, 700100, 6900100)),  # hors de l'emprise
]
slope_zones = polygon_layer("zones", SLOPE_ZONES)
# Colline : x 652000 à 652500, y 6850000 à 6850500.
HILL_BOX = (652100, 6850100, 652300, 6850300)
hill_zones = polygon_layer("zones-colline", [("Colline", box(*HILL_BOX))])
points = QgsVectorLayer("Point?crs=EPSG:2154&field=nom:string", "points", "memory")
project.addMapLayer(points)


def is_close(a, b, tol=1e-3):
    return a is not None and abs(a - b) <= tol


def values_of(row, name):
    cell = row["values"].get(name)
    return None if cell is None else cell["value"]


def step_slope():
    run_gesture(
        {"raster": SLOPE, "layer_id": slope_zones.id(), "label_field": "nom", "out": out("1-raster.zonal_statistics.gpkg")},
        "plan",
        step_slope_done,
    )


def step_slope_done():
    result, error = outcomes["plan"]
    check("plan incliné : pas d'erreur", error is None, error and (error.code, error.message))
    if result is None:
        finish()
        return
    check("plan incliné : feature_count = 3, SCR EPSG:2154", result["feature_count"] == 3 and result["crs"] == "EPSG:2154", result)
    check("plan incliné : pas tronqué", result["truncated"] is False, result)
    rows = {row["label"]: row for row in result["rows"]}
    check("plan incliné : trois lignes A, B, Loin", set(rows) == {"A", "B", "Loin"}, list(rows))
    a, b = rows.get("A"), rows.get("B")
    if a and b:
        check("A : moyenne 95, minimum 90,5, maximum 99,5",
              is_close(values_of(a, "mean"), 95.0) and is_close(values_of(a, "min"), 90.5)
              and is_close(values_of(a, "max"), 99.5), a)
        check("B : moyenne 85, minimum 80,5, maximum 89,5",
              is_close(values_of(b, "mean"), 85.0) and is_close(values_of(b, "min"), 80.5)
              and is_close(values_of(b, "max"), 89.5), b)
        check("unité `m` par défaut sur chaque valeur",
              all(cell["unit"] == "m" for r in (a, b) for cell in r["values"].values()), a)
    check("polygone hors emprise : une ligne, sans valeur", rows.get("Loin", {}).get("values") == {}, rows.get("Loin"))
    layer = QgsVectorLayer(out("1-raster.zonal_statistics.gpkg"), "lecture", "ogr")
    names = [f.name() for f in layer.fields()]
    print("       champs de la sortie : %s" % names)
    check("la sortie porte les champs _mean, _min, _max", {"_mean", "_min", "_max"} <= set(names), names)
    del layer
    check("fichier raster d'origine intact (empreinte)", sha(SLOPE) == sha(SLOPE_SRC))
    check("aucun .aux.xml à côté du raster", not os.path.exists(SLOPE + ".aux.xml") and not any(
        name.endswith(".aux.xml") for name in os.listdir(work)), os.listdir(work))
    check("la couche de polygones d'entrée garde ses champs", [f.name() for f in slope_zones.fields()] == ["nom"])
    step_hill()


def step_hill():
    run_gesture(
        {"raster": HILL, "layer_id": hill_zones.id(), "out": out("2-raster.zonal_statistics.gpkg"), "unit": "m"},
        "colline",
        step_hill_done,
    )


def step_hill_done():
    result, error = outcomes["colline"]
    check("colline : pas d'erreur", error is None, error and (error.code, error.message))
    if result is None:
        finish()
        return
    cells = hill_cells(*HILL_BOX)
    row = result["rows"][0]
    check("colline : étiquette = rang du polygone (pas de label_field)", row["label"] == "1", row["label"])
    check("colline : %d cellules lues" % len(cells), len(cells) == 400, len(cells))
    check("colline : moyenne exacte", is_close(values_of(row, "mean"), sum(cells) / len(cells)), (values_of(row, "mean"), sum(cells) / len(cells)))
    check("colline : minimum exact", is_close(values_of(row, "min"), min(cells)), (values_of(row, "min"), min(cells)))
    check("colline : maximum exact", is_close(values_of(row, "max"), max(cells)), (values_of(row, "max"), max(cells)))
    check("colline : unité déclarée reprise", all(c["unit"] == "m" for c in row["values"].values()), row)
    check("colline : fichier d'origine intact", sha(HILL) == sha(HILL_SRC))
    check("colline : aucun .aux.xml", not any(name.endswith(".aux.xml") for name in os.listdir(work)), os.listdir(work))
    step_declared_unit()


def step_declared_unit():
    run_gesture(
        {"raster": SLOPE, "layer_id": slope_zones.id(), "out": out("3-raster.zonal_statistics.gpkg"), "unit": "°"},
        "unite",
        step_declared_unit_done,
    )


def step_declared_unit_done():
    result, error = outcomes["unite"]
    check("unité déclarée « ° » : pas d'erreur", error is None, error and (error.code, error.message))
    if result is not None:
        units = {c["unit"] for r in result["rows"] for c in r["values"].values()}
        check("unité déclarée « ° » reprise telle quelle", units == {"°"}, units)
    step_points()


def step_points():
    run_gesture(
        {"raster": SLOPE, "layer_id": points.id(), "out": out("4-raster.zonal_statistics.gpkg")}, "points", step_points_done
    )


def step_points_done():
    _, error = outcomes["points"]
    check("couche de points : analysis.bad_params", error is not None and error.code == "analysis.bad_params", error and error.code)
    check("couche de points : aucun fichier écrit", not os.path.exists(out("4-raster.zonal_statistics.gpkg")))
    step_missing()


def step_missing():
    run_gesture(
        {"raster": out("absent.tif"), "layer_id": slope_zones.id(), "out": out("5-raster.zonal_statistics.gpkg")},
        "absent",
        step_missing_done,
    )


def step_missing_done():
    _, error = outcomes["absent"]
    check("raster absent : layer.not_found", error is not None and error.code == "layer.not_found", error and error.code)
    not_tif = out("table.csv")
    with open(not_tif, "w") as handle:
        handle.write("a,b\n1,2\n")
    run_gesture(
        {"raster": not_tif, "layer_id": slope_zones.id(), "out": out("6-raster.zonal_statistics.gpkg")},
        "pas_tif",
        step_not_tif_done,
    )


def step_not_tif_done():
    _, error = outcomes["pas_tif"]
    check("fichier non GeoTIFF : layer.invalid_source", error is not None and error.code == "layer.invalid_source", error and error.code)
    check("fichier non GeoTIFF : aucun fichier écrit", not os.path.exists(out("6-raster.zonal_statistics.gpkg")))
    step_wgs()


def step_wgs():
    # Le même carré A, exprimé en WGS84.
    transform = QgsCoordinateTransform(
        QgsCoordinateReferenceSystem("EPSG:2154"), QgsCoordinateReferenceSystem("EPSG:4326"), project
    )
    geometry = QgsGeometry(SLOPE_ZONES[0][1])
    geometry.transform(transform)
    wgs_zones = polygon_layer("zones-wgs84", [("A", geometry)], crs="EPSG:4326")
    outcomes["_wgs"] = wgs_zones
    run_gesture(
        {"raster": SLOPE, "layer_id": wgs_zones.id(), "label_field": "nom", "out": out("7-raster.zonal_statistics.gpkg")},
        "wgs",
        step_wgs_done,
    )


def step_wgs_done():
    result, error = outcomes["wgs"]
    check("SCR différents : pas d'erreur", error is None, error and (error.code, error.message))
    if result is not None:
        check("SCR différents : la couche produite reste en WGS84", result["crs"] == "EPSG:4326", result["crs"])
        row = result["rows"][0]
        check("SCR différents : statistiques exactes (95, 90,5, 99,5)",
              is_close(values_of(row, "mean"), 95.0, 1e-3) and is_close(values_of(row, "min"), 90.5)
              and is_close(values_of(row, "max"), 99.5), row)
        check("SCR différents : aucun .aux.xml", not any(n.endswith(".aux.xml") for n in os.listdir(work)), os.listdir(work))
        check("SCR différents : raster intact", sha(SLOPE) == sha(SLOPE_SRC))
    finish()


QTimer.singleShot(0, protect(step_slope))
QTimer.singleShot(150000, lambda: (check("sonde terminée à temps", False), finish()))
app.exec()

failed = [label for label, ok in checks if not ok]
verdict = bool(checks) and not failed
print("VERDICT :", "OK" if verdict else "ÉCHEC (%s)" % ", ".join(failed))
QgsApplication.exitQgis()
shutil.rmtree(work, ignore_errors=True)
sys.exit(0 if verdict else 1)
