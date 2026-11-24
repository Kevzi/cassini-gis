"""Sonde manuelle sous le vrai QGIS : alignement d'un raster sur un autre (ticket 38, `raster.align`).

À lancer avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`) :

    "C:\\Program Files\\QGIS 3.44.15\\bin\\python-qgis-ltr.bat" qgis-plugin/tests/probe_align.py

Le noyau est cherché dans `CASSINI_CORE`, sinon `target/release/`, `target/debug/` (la phase 2 en a besoin).
Aucune donnée hors dépôt : la sonde fabrique elle-même, avec GDAL, dans un dossier temporaire, deux GeoTIFF
Float32 d'une bande en EPSG:2154 :

- la référence : 20 x 20 cellules de 10 m, origine (652000, 6500200) ;
- la source : 30 x 30 cellules de 7 m, origine (651995.5, 6500210), valeurs en dégradé (aucune valeur
  n'est répétée, donc le plus proche voisin rend des valeurs de la source et le bilinéaire en invente) ;

plus une référence en EPSG:3857 couvrant la même zone, une référence sans recouvrement, un raster à trois
bandes, un raster sans système et un `.asc`. Elle lance le vrai geste `raster.align`, affiche la version de
QGIS, puis termine par `VERDICT : OK` ou `VERDICT : ÉCHEC`.

Ce que la sonde constate (le plan interdit de le supposer) :
1. `native:alignsingleraster` est dans le registre de traitement (sinon la commande ne serait pas annoncée).
2. `nearest` : la sortie a le système, la taille de cellule, l'origine, les colonnes et les lignes de la
   référence (20 x 20, pas les 21 lignes de la source sans `EXTENT`), une bande, et ne contient que des
   valeurs de la source ; `bilinear` : mêmes dimensions, des valeurs qui n'existent pas dans la source.
3. Référence en EPSG:3857, raster en 2154 : la sortie est en EPSG:3857, sur la grille de cette référence.
4. Les rasters d'origine sont identiques octet pour octet (SHA-256) et aucun `.aux.xml` n'apparaît.
5. Refus avant lancement, aucun fichier écrit : raster absent (`layer.not_found`), `.asc`
   (`layer.invalid_source`), trois bandes ou sans système, pour le raster comme pour la référence
   (`analysis.bad_params`).
6. Emprises sans recouvrement : ce que QGIS rend (échec nommé `analysis.failed`, ou fichier vide écarté).
7. Phase 2, par le noyau : la sortie est ouverte (`layer.open_raster` dicté) et `terrain.slope` la lit
   (`out:1.1`) : le noyau lit le GeoTIFF écrit par QGIS (AD-20) ; journal et rejeu identique.
"""

import hashlib
import json
import os
import shutil
import sys
import tempfile
import traceback

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "qgis-plugin", "cassini_qgis"))

from qgis.core import Qgis, QgsApplication, QgsCoordinateReferenceSystem, QgsProject  # noqa: E402
from qgis.PyQt.QtCore import QCoreApplication, QProcess, QTimer  # noqa: E402

app = QgsApplication([], False)
QgsApplication.initQgis()
# Le fournisseur GDAL du cadre de traitement, comme dans QGIS ouvert (extension Processing).
sys.path.append(os.path.join(QgsApplication.prefixPath(), "python", "plugins"))
from processing.core.Processing import Processing  # noqa: E402

Processing.initialize()
from osgeo import gdal, osr  # noqa: E402

gdal.UseExceptions()
import analysis  # noqa: E402
import client  # noqa: E402
import gestures  # noqa: E402
import raster_output  # noqa: E402

print("QGIS", Qgis.version())
work = tempfile.mkdtemp(prefix="cassini-align-")
project = QgsProject.instance()
project.setCrs(QgsCoordinateReferenceSystem("EPSG:2154"))
checks = []
outcomes = {}
state = {"client": None}

SOURCE = os.path.join(work, "source.tif")
REFERENCE = os.path.join(work, "reference.tif")
REFERENCE_3857 = os.path.join(work, "reference-3857.tif")
FAR = os.path.join(work, "loin.tif")
PARTIAL = os.path.join(work, "partiel.tif")
MULTI = os.path.join(work, "multi.tif")
NO_CRS = os.path.join(work, "sans-systeme.tif")
ASC = os.path.join(work, "source.asc")
ORIGINALS = {}


def srs_wkt(epsg):
    srs = osr.SpatialReference()
    srs.ImportFromEPSG(epsg)
    return srs.ExportToWkt()


def make_raster(path, width, height, origin, cell, epsg=2154, bands=1, value=None):
    import numpy

    dataset = gdal.GetDriverByName("GTiff").Create(path, width, height, bands, gdal.GDT_Float32)
    dataset.SetGeoTransform((origin[0], cell, 0.0, origin[1], 0.0, -cell))
    if epsg:
        dataset.SetProjection(srs_wkt(epsg))
    rows, cols = numpy.mgrid[0:height, 0:width]
    grid = (value or (lambda r, c: r * 1000.0 + c * 1.5 + 0.25))(rows, cols).astype("float32")
    for i in range(1, bands + 1):
        dataset.GetRasterBand(i).WriteArray(grid + i)
    dataset.FlushCache()
    dataset = None


def make_reference_3857():
    """Une référence en EPSG:3857 entièrement dans la source : 15 x 15 cellules de 14 m (environ 10 m au sol)."""
    to_3857 = osr.CoordinateTransformation(_srs(2154), _srs(3857))
    x, y, _ = to_3857.TransformPoint(652010.0, 6500190.0)
    make_raster(REFERENCE_3857, 15, 15, (round(x, 3), round(y, 3)), 14.0, epsg=3857)


def _srs(epsg):
    srs = osr.SpatialReference()
    srs.ImportFromEPSG(epsg)
    srs.SetAxisMappingStrategy(osr.OAMS_TRADITIONAL_GIS_ORDER)
    return srs


make_raster(SOURCE, 30, 30, (651995.5, 6500210.0), 7.0)
make_raster(REFERENCE, 20, 20, (652000.0, 6500200.0), 10.0, value=lambda r, c: r * 0.0 + c * 0.0 + 100.0)
make_reference_3857()
make_raster(FAR, 20, 20, (700000.0, 6600200.0), 10.0)
make_raster(MULTI, 30, 30, (651995.5, 6500210.0), 7.0, bands=3)
make_raster(NO_CRS, 30, 30, (651995.5, 6500210.0), 7.0, epsg=None)
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


def aux_files():
    return [n for n in os.listdir(work) if n.endswith(".aux.xml")]


def find_core():
    candidates = [os.environ.get("CASSINI_CORE")]
    for profile in ("release", "debug"):
        candidates.append(os.path.join(ROOT, "target", profile, "cassini.exe"))
        candidates.append(os.path.join(ROOT, "target", profile, "cassini"))
    return next((p for p in candidates if p and os.path.isfile(p)), None)


def grid_of(path):
    """(colonnes, lignes, géotransformation, bandes, EPSG, valeurs) d'un raster."""
    dataset = gdal.Open(path)
    srs = osr.SpatialReference(wkt=dataset.GetProjection()) if dataset.GetProjection() else None
    epsg = None
    if srs is not None:
        srs.AutoIdentifyEPSG()
        code = srs.GetAuthorityCode(None)
        epsg = int(code) if code else None
    found = (dataset.RasterXSize, dataset.RasterYSize, tuple(round(v, 6) for v in dataset.GetGeoTransform()),
             dataset.RasterCount, epsg, dataset.GetRasterBand(1).ReadAsArray().ravel().tolist())
    dataset = None
    return found


def run_gesture(params, label, then):
    gesture = analysis.ProcessingGesture(analysis.ALGORITHMS["raster.align"])

    def done(result=None, error=None):
        outcomes[label] = (result, error)
        QTimer.singleShot(0, protect(then))

    outcomes["_g_" + label] = gesture  # gardé en vie pendant la tâche
    gesture.start(params, done)


def step_nearest():
    run_gesture({"raster": SOURCE, "reference": REFERENCE, "out": out("1-raster.align.tif")}, "nearest",
                step_nearest_done)


def step_nearest_done():
    result, error = outcomes["nearest"]
    check("nearest : pas d'erreur", error is None, error and (error.code, error.message))
    if result is None:
        finish()
        return
    check("nearest : résultat `file` seul", result == {"file": out("1-raster.align.tif")}, result)
    ref, aligned, source = grid_of(REFERENCE), grid_of(out("1-raster.align.tif")), grid_of(SOURCE)
    print("       référence %s ; sortie %s" % (ref[:5], aligned[:5]))
    check("nearest : colonnes, lignes, origine et cellule de la référence", aligned[:3] == ref[:3], (aligned[:3], ref[:3]))
    check("nearest : une bande, EPSG:2154", aligned[3:5] == (1, 2154), aligned[3:5])
    check("nearest : seulement des valeurs de la source", set(aligned[5]) <= set(source[5]), None)
    check("nearest : aucune cellule vide (la source couvre la référence)", all(v == v for v in aligned[5]), None)
    check("sources intactes (empreinte)", sha(SOURCE) == ORIGINALS[SOURCE] and sha(REFERENCE) == ORIGINALS[REFERENCE])
    check("aucun .aux.xml", not aux_files(), os.listdir(work))
    run_gesture({"raster": SOURCE, "reference": REFERENCE, "resampling": "bilinear",
                 "out": out("2-raster.align.tif")}, "bilinear", step_bilinear_done)


def step_bilinear_done():
    result, error = outcomes["bilinear"]
    check("bilinear : pas d'erreur", error is None, error and (error.code, error.message))
    if result is None:
        finish()
        return
    ref, aligned, source = grid_of(REFERENCE), grid_of(out("2-raster.align.tif")), grid_of(SOURCE)
    check("bilinear : grille de la référence", aligned[:5] == ref[:5], (aligned[:5], ref[:5]))
    invented = [v for v in aligned[5] if v not in set(source[5])]
    check("bilinear : des valeurs interpolées, absentes de la source", len(invented) > 0, len(invented))
    check("aucun .aux.xml", not aux_files(), os.listdir(work))
    run_gesture({"raster": SOURCE, "reference": REFERENCE_3857, "out": out("3-raster.align.tif")}, "other",
                step_other_done)


def step_other_done():
    result, error = outcomes["other"]
    check("autre système : pas d'erreur", error is None, error and (error.code, error.message))
    if result is None:
        finish()
        return
    ref, aligned = grid_of(REFERENCE_3857), grid_of(out("3-raster.align.tif"))
    print("       référence 3857 %s ; sortie %s" % (ref[:5], aligned[:5]))
    check("autre système : sortie en EPSG:3857, grille de la référence", aligned[:5] == ref[:5], (aligned[:5], ref[:5]))
    check("autre système : des valeurs lues dans la source", any(v == v and v != 0 for v in aligned[5]), None)
    check("aucun .aux.xml", not aux_files(), os.listdir(work))
    # Recouvrement partiel : la référence dépasse la source de 5,5 m à l'ouest (constat, pas une exigence).
    make_raster(PARTIAL, 20, 20, (651990.0, 6500200.0), 10.0)
    ORIGINALS[PARTIAL] = sha(PARTIAL)
    run_gesture({"raster": SOURCE, "reference": PARTIAL, "out": out("5-raster.align.tif")}, "partial",
                step_partial_done)


def step_partial_done():
    result, error = outcomes["partial"]
    if result is not None:
        print("       recouvrement partiel : référence %s ; sortie %s"
              % (grid_of(PARTIAL)[:3], grid_of(out("5-raster.align.tif"))[:3]))
    else:
        print("       recouvrement partiel : erreur %s" % (error and (error.code, error.message),))
    os.path.exists(out("5-raster.align.tif")) and os.remove(out("5-raster.align.tif"))
    run_gesture({"raster": SOURCE, "reference": FAR, "out": out("4-raster.align.tif")}, "far", step_far_done)


def step_far_done():
    result, error = outcomes["far"]
    print("       sans recouvrement : résultat %s, erreur %s" % (result, error and (error.code, error.message)))
    check("sans recouvrement : analysis.failed", error is not None and error.code == "analysis.failed",
          error and (error.code, error.message))
    check("sans recouvrement : aucun fichier partiel", not os.path.exists(out("4-raster.align.tif")))
    refusals = [
        ("absent-r", {"raster": out("absent.tif"), "reference": REFERENCE}, "layer.not_found"),
        ("absent-ref", {"raster": SOURCE, "reference": out("absent.tif")}, "layer.not_found"),
        ("asc-r", {"raster": ASC, "reference": REFERENCE}, "layer.invalid_source"),
        ("asc-ref", {"raster": SOURCE, "reference": ASC}, "layer.invalid_source"),
        ("multi-r", {"raster": MULTI, "reference": REFERENCE}, "analysis.bad_params"),
        ("multi-ref", {"raster": SOURCE, "reference": MULTI}, "analysis.bad_params"),
        ("nocrs-r", {"raster": NO_CRS, "reference": REFERENCE}, "analysis.bad_params"),
        ("nocrs-ref", {"raster": SOURCE, "reference": NO_CRS}, "analysis.bad_params"),
    ]
    state["refusals"] = refusals
    refuse(0)


def refuse(i):
    refusals = state["refusals"]
    if i > 0:
        label, _, code = refusals[i - 1]
        _, error = outcomes[label]
        check("refus %s : %s" % (label, code), error is not None and error.code == code, error and error.code)
        check("refus %s : aucun fichier écrit" % label, not os.path.exists(out("9-raster.align.tif")))
    if i >= len(refusals):
        QTimer.singleShot(0, protect(after_direct))
        return
    label, params, _ = refusals[i]
    run_gesture(dict(params, out=out("9-raster.align.tif")), label, lambda: refuse(i + 1))


def after_direct():
    for path, digest in ORIGINALS.items():
        check("original inchangé : %s" % os.path.basename(path), sha(path) == digest)
    check("fin de la phase 1 : aucun .aux.xml", not aux_files(), os.listdir(work))
    core = find_core()
    if core is None:
        check("noyau trouvé (CASSINI_CORE)", False)
        finish()
        return
    state["core"] = core
    # Le projet du noyau : un dossier à part, avec les deux rasters recopiés (chemins relatifs au projet).
    base = os.path.join(work, "projet")
    os.makedirs(base)
    shutil.copyfile(SOURCE, os.path.join(base, "classes.tif"))
    shutil.copyfile(REFERENCE, os.path.join(base, "mnt.tif"))
    state["project_file"] = os.path.join(base, "essai.qgz").replace("\\", "/")
    state["cassini_dir"] = os.path.join(base, "essai.cassini")
    project.clear()
    project.setCrs(QgsCoordinateReferenceSystem("EPSG:2154"))
    assert project.write(state["project_file"]), "projet non enregistré"
    new_client(on_first)


# -- phase 2 : par le noyau --------------------------------------------------
def read_journal():
    with open(os.path.join(state["cassini_dir"], "journal.jsonl"), encoding="utf-8") as f:
        return [json.loads(line) for line in f if line.strip()]


def new_client(on_event):
    process = QProcess()
    registry = gestures.GestureRegistry()
    registry.register("layer.open_raster", raster_output.LayerOpenRasterGesture)
    analysis.register_all(registry)
    state["process"] = process
    state["client"] = client.CoreClient(process, protect(on_event), registry)
    state["client"].start(state["core"], state["project_file"])


STEPS = [
    ("raster.align", {"raster": "classes.tif", "reference": "mnt.tif"}),
    ("terrain.slope", {"dem": "out:1.1", "unit": "degrees"}),
]
results = []


def run_next():
    command, params = STEPS[len(results)]
    state["client"].run_command(command, params)


def on_first(kind, payload):
    if kind == client.READY:
        run_next()
    elif kind == client.RUN_DONE:
        results.append(payload["result"])
        if len(results) < len(STEPS):
            run_next()
        else:
            QTimer.singleShot(0, protect(between))
    elif kind in (client.RUN_FAILED, client.RUN_CANCELLED, client.FAILED, client.DIED):
        check("exécution par le noyau", False, "%s %s" % (kind, payload))
        finish()


def between():
    aligned, slope = results
    check("par le noyau : résultat `file`", list(aligned) == ["file"], aligned)
    check("la pente lit le GeoTIFF aligné par QGIS (AD-20)", isinstance(slope.get("max"), (int, float)), slope)
    rows = read_journal()
    ids = [r["id"] for r in rows]
    check("journal : alignement, ouverture dictée, pente, ouverture dictée",
          ids == ["raster.align", "layer.open_raster", "terrain.slope", "layer.open_raster"], ids)
    check("journal : les deux rasters relatifs, la pente sur out:1.1",
          rows[0]["params"].get("raster") == "classes.tif" and rows[0]["params"].get("reference") == "mnt.tif"
          and rows[2]["params"]["dem"] == "out:1.1", (rows[0]["params"], rows[2]["params"]))
    check("la sortie est ouverte dans le projet", len(project.mapLayersByName("1-raster.align")) == 1)
    state["journal"] = rows
    state["client"].stop()
    project.clear()
    check("le projet rouvre", project.read(state["project_file"]))
    new_client(on_second)


def on_second(kind, payload):
    if kind == client.READY:
        check("le rejeu part", state["client"].replay())
    elif kind == client.REPLAY_DONE:
        check("le rejeu rend identical: true", payload == {"steps": 4, "identical": True}, payload)
        check("le journal est inchangé par le rejeu", read_journal() == state["journal"])
        finish()
    elif kind in (client.REPLAY_FAILED, client.FAILED, client.DIED):
        check("rejeu nominal", False, "%s %s" % (kind, payload))
        finish()


check("native:alignsingleraster est dans le registre de traitement",
      QgsApplication.processingRegistry().algorithmById("native:alignsingleraster") is not None)
for path in (SOURCE, REFERENCE, REFERENCE_3857, FAR, MULTI, NO_CRS, ASC):
    ORIGINALS[path] = sha(path)

QTimer.singleShot(0, protect(step_nearest))
QTimer.singleShot(280000, lambda: (check("sonde terminée à temps", False), finish()))
QCoreApplication.exec()

if state["client"] is not None:
    state["client"].stop()
failed = [label for label, ok in checks if not ok]
verdict = len(checks) >= 25 and not failed
project.clear()
shutil.rmtree(work, ignore_errors=True)
print("VERDICT :", "OK" if verdict else "ÉCHEC (%s)" % ", ".join(failed))
sys.stdout.flush()
os._exit(0 if verdict else 1)
