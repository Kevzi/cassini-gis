"""Sonde manuelle sous le vrai QGIS : polygonisation d'un raster (ticket 37, `raster.polygonize`).

À lancer avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`) :

    "C:\\Program Files\\QGIS 3.44.15\\bin\\python-qgis-ltr.bat" qgis-plugin/tests/probe_polygonize.py

Aucun noyau ni aucune donnée hors dépôt : la sonde fabrique elle-même, dans un dossier temporaire, un
petit raster de classes (GeoTIFF d'octets, 6 x 4 cellules de 10 m, EPSG:2154), sa variante à 3 bandes et
un GeoTIFF sans système, puis lance le vrai geste `raster.polygonize`. Affiche la version de QGIS, puis
termine par `VERDICT : OK` ou `VERDICT : ÉCHEC`.

Le raster de classes (lignes du haut vers le bas) :

    1 1 1 1 1 1
    1 5 1 2 2 1
    1 1 5 2 2 1
    1 1 1 1 1 1

Ce que la sonde constate (le plan interdit de le supposer) :
1. `gdal:polygonize` est dans le registre de traitement (sinon la commande ne serait pas annoncée).
2. Bande 1 : quatre polygones (la zone 1, avec ses deux trous ; les deux 5, qui ne se touchent que par un
   coin et restent donc deux zones à quatre voisins ; le bloc 2), champ `value` exact, aires exactes
   (1 800 m2, 100 m2, 100 m2, 400 m2), `feature_count` et SCR EPSG:2154.
3. Raster à 3 bandes, `band` 2 : les polygones de la bande 2, pas ceux de la bande 1.
4. Bande absente (4 sur 3) : `analysis.bad_params`, aucun fichier écrit.
5. Raster absent : `layer.not_found` ; non GeoTIFF : `layer.invalid_source` ; sans système :
   `analysis.bad_params` ; aucun fichier écrit.
6. Les rasters d'origine restent identiques (empreinte SHA-256) et aucun `.aux.xml` n'apparaît.
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

from qgis.core import Qgis, QgsApplication, QgsCoordinateReferenceSystem, QgsProject, QgsVectorLayer  # noqa: E402
from qgis.PyQt.QtCore import QCoreApplication, QTimer  # noqa: E402

app = QgsApplication([], False)
QgsApplication.initQgis()
# Le fournisseur GDAL du cadre de traitement, comme dans QGIS ouvert (extension Processing).
sys.path.append(os.path.join(QgsApplication.prefixPath(), "python", "plugins"))
from processing.core.Processing import Processing  # noqa: E402

Processing.initialize()
import analysis  # noqa: E402
from osgeo import gdal, osr  # noqa: E402

print("QGIS", Qgis.version())
work = tempfile.mkdtemp(prefix="cassini-polygonize-")
project = QgsProject.instance()
project.setCrs(QgsCoordinateReferenceSystem("EPSG:2154"))
checks = []
outcomes = {}

CLASSES = os.path.join(work, "classes.tif")
MULTI = os.path.join(work, "multi.tif")
NO_CRS = os.path.join(work, "sans-systeme.tif")
ASC = os.path.join(work, "classes.asc")
ORIGINALS = {}

BAND1 = [
    [1, 1, 1, 1, 1, 1],
    [1, 5, 1, 2, 2, 1],
    [1, 1, 5, 2, 2, 1],
    [1, 1, 1, 1, 1, 1],
]
# Bande 2 : un fond de 7 et une seule cellule de 8 (deux polygones).
BAND2 = [
    [7, 7, 7, 7, 7, 7],
    [7, 7, 7, 7, 8, 7],
    [7, 7, 7, 7, 7, 7],
    [7, 7, 7, 7, 7, 7],
]
BAND3 = [[9] * 6 for _ in range(4)]


def make_raster(path, bands, epsg=2154):
    driver = gdal.GetDriverByName("GTiff")
    dataset = driver.Create(path, 6, 4, len(bands), gdal.GDT_Byte)
    dataset.SetGeoTransform((652000.0, 10.0, 0.0, 6850040.0, 0.0, -10.0))
    if epsg:
        srs = osr.SpatialReference()
        srs.ImportFromEPSG(epsg)
        dataset.SetProjection(srs.ExportToWkt())
    for i, rows in enumerate(bands, start=1):
        dataset.GetRasterBand(i).WriteArray(__import__("numpy").array(rows, dtype="uint8"))
    dataset.FlushCache()
    dataset = None


make_raster(CLASSES, [BAND1])
make_raster(MULTI, [BAND1, BAND2, BAND3])
make_raster(NO_CRS, [BAND1], epsg=None)
with open(ASC, "w") as handle:
    handle.write("ncols 2\nnrows 2\nxllcorner 0\nyllcorner 0\ncellsize 1\n1 2\n3 4\n")


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


def run_gesture(params, label, then):
    gesture = analysis.ProcessingGesture(analysis.ALGORITHMS["raster.polygonize"])

    def done(result=None, error=None):
        outcomes[label] = (result, error)
        QTimer.singleShot(0, protect(then))

    outcomes["_g_" + label] = gesture  # gardé en vie pendant la tâche
    gesture.start(params, done)


def polygons_of(path):
    """(champs, [(value, aire)]) de la couche écrite."""
    layer = QgsVectorLayer(path, "lecture", "ogr")
    names = [f.name() for f in layer.fields()]
    found = sorted((feature["value"], round(feature.geometry().area(), 6)) for feature in layer.getFeatures())
    del layer
    return names, found


def aux_files():
    return [n for n in os.listdir(work) if n.endswith(".aux.xml")]


def step_band1():
    run_gesture({"raster": CLASSES, "out": out("1-raster.polygonize.gpkg")}, "b1", step_band1_done)


def step_band1_done():
    result, error = outcomes["b1"]
    check("bande 1 : pas d'erreur", error is None, error and (error.code, error.message))
    if result is None:
        finish()
        return
    check("bande 1 : SCR EPSG:2154, couche nommée par le noyau",
          result["crs"] == "EPSG:2154" and result["name"] == "1-raster.polygonize", result)
    names, found = polygons_of(out("1-raster.polygonize.gpkg"))
    print("       champs : %s ; polygones (value, aire) : %s" % (names, found))
    check("champ `value` présent", "value" in names, names)
    check("feature_count = nombre de polygones écrits", result["feature_count"] == len(found) == 4, (result, found))
    check("zones et aires exactes (quatre voisins : les deux 5 restent deux zones)",
          found == [(1, 1800.0), (2, 400.0), (5, 100.0), (5, 100.0)], found)
    check("raster d'origine intact (empreinte)", sha(CLASSES) == ORIGINALS[CLASSES])
    check("aucun .aux.xml à côté du raster", not aux_files(), os.listdir(work))
    step_band2()


def step_band2():
    run_gesture({"raster": MULTI, "band": 2, "out": out("2-raster.polygonize.gpkg")}, "b2", step_band2_done)


def step_band2_done():
    result, error = outcomes["b2"]
    check("bande 2 : pas d'erreur", error is None, error and (error.code, error.message))
    if result is None:
        finish()
        return
    _, found = polygons_of(out("2-raster.polygonize.gpkg"))
    check("bande 2 : les polygones de la bande 2, pas ceux de la 1",
          found == [(7, 2300.0), (8, 100.0)], found)
    check("bande 2 : raster d'origine intact", sha(MULTI) == ORIGINALS[MULTI])
    check("bande 2 : aucun .aux.xml", not aux_files(), os.listdir(work))
    run_gesture({"raster": MULTI, "out": out("3-raster.polygonize.gpkg")}, "default", step_default_done)


def step_default_done():
    result, error = outcomes["default"]
    check("raster à 3 bandes sans `band` : la première", error is None and result is not None, error)
    if result is not None:
        _, found = polygons_of(out("3-raster.polygonize.gpkg"))
        check("sans `band` : les polygones de la bande 1", len(found) == 4 and found[0][0] == 1, found)
    run_gesture({"raster": MULTI, "band": 4, "out": out("4-raster.polygonize.gpkg")}, "b4", step_b4_done)


def step_b4_done():
    _, error = outcomes["b4"]
    check("bande 4 sur 3 : analysis.bad_params", error is not None and error.code == "analysis.bad_params",
          error and error.code)
    check("bande 4 : aucun fichier écrit", not os.path.exists(out("4-raster.polygonize.gpkg")))
    run_gesture({"raster": out("absent.tif"), "out": out("5-raster.polygonize.gpkg")}, "absent", step_missing_done)


def step_missing_done():
    _, error = outcomes["absent"]
    check("raster absent : layer.not_found", error is not None and error.code == "layer.not_found", error and error.code)
    check("raster absent : aucun fichier écrit", not os.path.exists(out("5-raster.polygonize.gpkg")))
    run_gesture({"raster": ASC, "out": out("6-raster.polygonize.gpkg")}, "asc", step_asc_done)


def step_asc_done():
    _, error = outcomes["asc"]
    check("non GeoTIFF : layer.invalid_source",
          error is not None and error.code == "layer.invalid_source", error and error.code)
    check("non GeoTIFF : aucun fichier écrit", not os.path.exists(out("6-raster.polygonize.gpkg")))
    run_gesture({"raster": NO_CRS, "out": out("7-raster.polygonize.gpkg")}, "nocrs", step_nocrs_done)


def step_nocrs_done():
    _, error = outcomes["nocrs"]
    check("sans système : analysis.bad_params",
          error is not None and error.code == "analysis.bad_params", error and error.code)
    check("sans système : aucun fichier écrit", not os.path.exists(out("7-raster.polygonize.gpkg")))
    check("sans système : source intacte", sha(NO_CRS) == ORIGINALS[NO_CRS])
    check("fin : aucun .aux.xml", not aux_files(), os.listdir(work))
    finish()


check("gdal:polygonize est dans le registre de traitement",
      QgsApplication.processingRegistry().algorithmById("gdal:polygonize") is not None)
for path in (CLASSES, MULTI, NO_CRS):
    ORIGINALS[path] = sha(path)

QTimer.singleShot(0, protect(step_band1))
QTimer.singleShot(150000, lambda: (check("sonde terminée à temps", False), finish()))
app.exec()

failed = [label for label, ok in checks if not ok]
verdict = bool(checks) and not failed
print("VERDICT :", "OK" if verdict else "ÉCHEC (%s)" % ", ".join(failed))
QgsApplication.exitQgis()
shutil.rmtree(work, ignore_errors=True)
sys.exit(0 if verdict else 1)
