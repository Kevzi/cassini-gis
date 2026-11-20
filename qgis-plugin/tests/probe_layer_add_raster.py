"""Sonde manuelle sous le vrai QGIS : ouverture d'un MNT GeoTIFF par `layer.add{raster}` (ticket 20).

À lancer avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`) :

    "C:\\Program Files\\QGIS 3.44.15\\bin\\python-qgis-ltr.bat" qgis-plugin/tests/probe_layer_add_raster.py

Aucun noyau ni donnée hors dépôt : les vrais gestes `layer.add` et `project.inventory` lisent un
GeoTIFF synthétique du dépôt (`fixtures/mnt-colline-2154.tif`) copié dans un dossier temporaire.
Termine par `VERDICT : OK` ou `VERDICT : ÉCHEC`.

Ce que la sonde constate :
1. Le MNT s'ouvre : `kind` = `raster`, `source_crs` = `EPSG:2154`, couche raster valide au projet.
2. `project.inventory` rend `kind` = `raster`, le SCR, `width` et `height` égaux à ceux de GDAL.
3. Le fichier est intact (empreinte SHA-256 avant et après) et aucun `.aux.xml` ni pyramide n'apparaît.
4. Un GeoTIFF sans SCR : refusé sans `crs` (`layer.unknown_crs`), ouvert avec `crs` (SCR attribué en
   mémoire, fichier intact), refusé si `crs` contredit celui d'un fichier qui en porte un.
5. Un `.asc` est refusé (`layer.unsupported_format`).
6. Après un rendu réel de la couche (QgsMapRendererSequentialJob), rien n'est écrit à côté du fichier.
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

from qgis.PyQt.QtCore import QSize  # noqa: E402
from osgeo import gdal  # noqa: E402
from qgis.core import QgsApplication, QgsCoordinateReferenceSystem, QgsMapRendererSequentialJob, QgsMapSettings, QgsProject, QgsRasterLayer  # noqa: E402

app = QgsApplication([], False)
QgsApplication.initQgis()
import inventory  # noqa: E402
import layer_add  # noqa: E402

FIXTURE = os.path.join(ROOT, "fixtures", "mnt-colline-2154.tif")
checks = []


def check(label, condition, detail=""):
    checks.append((label, bool(condition)))
    print(("ok     " if condition else "ECHEC  ") + label + (" : %s" % (detail,) if detail and not condition else ""))


def digest(path):
    with open(path, "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()


def run(gesture_class, params):
    calls = []
    gesture = gesture_class(None, lambda fn: fn())
    gesture.start(params, lambda result=None, error=None: calls.append((result, error)))
    assert len(calls) == 1, calls
    return calls[0]


def listing(folder):
    return sorted(os.listdir(folder))


def main(work):
    mnt = os.path.join(work, "mnt.tif")
    shutil.copy(FIXTURE, mnt)
    before, files = digest(mnt), listing(work)
    project = QgsProject.instance()
    project.setCrs(QgsCoordinateReferenceSystem("EPSG:2154"))

    result, error = run(layer_add.LayerAddGesture, {"raster": mnt})
    check("1. ouverture sans erreur", error is None, error)
    check("1. kind raster", (result or {}).get("kind") == "raster", result)
    check("1. SCR du fichier", (result or {}).get("source_crs") == "EPSG:2154", result)
    layer = project.mapLayer((result or {}).get("layer_id"))
    check("1. couche raster valide au projet", layer is not None and layer.isValid() and isinstance(layer, QgsRasterLayer))

    result, error = run(inventory.ProjectInventoryGesture, {})
    entry = (result or {}).get("layers", [{}])[0]
    ds = gdal.Open(mnt)
    check("2. inventaire kind raster", entry.get("kind") == "raster", entry)
    check("2. inventaire SCR", entry.get("crs") == "EPSG:2154", entry)
    check("2. largeur et hauteur de GDAL", (entry.get("width"), entry.get("height")) == (ds.RasterXSize, ds.RasterYSize), entry)
    print("       dimensions : %s x %s pixels" % (entry.get("width"), entry.get("height")))
    ds = None

    # 3 bis. Rendu : un rendu réel de la couche (rendu par défaut, statistiques de contraste)
    # ne doit écrire ni `.aux.xml` ni pyramide à côté du fichier.
    settings = QgsMapSettings()
    settings.setLayers([layer])
    settings.setDestinationCrs(layer.crs())
    settings.setExtent(layer.extent())
    settings.setOutputSize(QSize(200, 200))
    job = QgsMapRendererSequentialJob(settings)
    job.start()
    job.waitForFinished()
    image = job.renderedImage()
    check("3 bis. rendu produit une image", not image.isNull() and image.width() == 200)
    del job
    project.removeAllMapLayers()
    layer = None
    check("3 bis. fichier intact après rendu", digest(mnt) == before)
    check("3 bis. aucun fichier ajouté après rendu", listing(work) == files, listing(work))
    check("3. fichier intact (empreinte)", digest(mnt) == before)
    check("3. aucun fichier ajouté à côté", listing(work) == files, listing(work))

    # 4. Un GeoTIFF sans système.
    bare = os.path.join(work, "sans-scr.tif")
    src = gdal.Open(FIXTURE)
    out = gdal.GetDriverByName("GTiff").CreateCopy(bare, src)
    out.SetProjection("")
    out.FlushCache()
    out = None
    src = None
    probe = gdal.Open(bare)
    has_crs = bool(probe.GetProjection())
    probe = None
    print("       sans-scr.tif porte un SCR : %s" % has_crs)
    bare_before, files = digest(bare), listing(work)
    if not has_crs:
        _, error = run(layer_add.LayerAddGesture, {"raster": bare})
        check("4. sans SCR ni crs : layer.unknown_crs", error is not None and error.code == "layer.unknown_crs", error)
        result, error = run(layer_add.LayerAddGesture, {"raster": bare, "crs": "EPSG:2154"})
        check("4. sans SCR avec crs : ouvert", error is None and result["source_crs"] == "EPSG:2154", (result, error))
        check("4. fichier sans SCR intact", digest(bare) == bare_before)
        check("4. aucun .aux.xml", not [f for f in listing(work) if f.endswith(".aux.xml")], listing(work))
        project.removeAllMapLayers()
    else:
        check("4. fabrication d'un GeoTIFF sans SCR", False, "GDAL a gardé le SCR")
    _, error = run(layer_add.LayerAddGesture, {"raster": mnt, "crs": "EPSG:4326"})
    check("4. crs contraire : layer.crs_conflict", error is not None and error.code == "layer.crs_conflict", error)
    project.removeAllMapLayers()

    # 5. Un autre format.
    asc = os.path.join(work, "mnt.asc")
    with open(asc, "w") as f:
        f.write("ncols 1\nnrows 1\nxllcorner 0\nyllcorner 0\ncellsize 1\n1\n")
    _, error = run(layer_add.LayerAddGesture, {"raster": asc})
    check("5. .asc refusé", error is not None and error.code == "layer.unsupported_format", error)
    check("5. aucun .aux.xml final", not [f for f in listing(work) if f.endswith(".aux.xml")], listing(work))


work = tempfile.mkdtemp(prefix="cassini-sonde-raster-")
try:
    main(work)
except Exception:
    traceback.print_exc()
    checks.append(("exception", False))
finally:
    QgsProject.instance().clear()
    shutil.rmtree(work, ignore_errors=True)
    QgsApplication.exitQgis()
ok = all(c for _, c in checks)
print("VERDICT : %s" % ("OK" if ok else "ÉCHEC"))
sys.exit(0 if ok else 1)
