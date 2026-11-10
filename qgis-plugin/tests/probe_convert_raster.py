"""Sonde manuelle sous le vrai QGIS : `data.convert_raster` (epic 10, entrée 30).

À lancer avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`) :

    set QT_QPA_PLATFORM=windows
    "C:\\Program Files\\QGIS 3.44.15\\bin\\python-qgis-ltr.bat" qgis-plugin/tests/probe_convert_raster.py

Le noyau est cherché dans `CASSINI_CORE`, sinon `target/release/`, `target/debug/`. Un vrai
MNT d'un autre format (ASCII Grid de la BD ALTI, hors dépôt) est pris dans `CASSINI_ASC` s'il
est posé ; sans lui, seule la fixture synthétique sert.

1. Geste seul, fournisseur GDAL de QGIS chargé : `fixtures/mnt-colline-sans-systeme.asc`
   (sans `.prj`) avec `assign_epsg` 2154 — GeoTIFF Float32, une bande, sans-donnée -9999, même
   grille, les trois cellules à -99999 devenues -9999, valeurs identiques ailleurs, EPSG:2154 ;
   sans `assign_epsg` — GeoTIFF sans système (`epsg` null) ; une source en EPSG:2154 déclarée
   32632 — `analysis.bad_params`, rien lancé ; code inconnu — `data.unknown_crs` ; fichier
   absent — `layer.not_found` ; annulation avant lancement — `qgis.cancelled`, aucun fichier ;
   le vrai ASCII Grid, s'il est là, converti à l'identique (même grille, mêmes valeurs).
2. Par le noyau, sur la fixture : la conversion, l'ouverture dictée (`layer.open_raster`), puis
   `terrain.slope` sur la sortie de la conversion (`out:1.1`) — le noyau lit le GeoTIFF écrit
   par QGIS (AD-20) ; projet fermé et rouvert, rejeu du journal : `identical: true`.
3. Les originaux sont inchangés (empreinte).
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
FIXTURE = os.path.join(ROOT, "fixtures", "mnt-colline-sans-systeme.asc")
WITH_SYSTEM = os.path.join(ROOT, "fixtures", "mnt-colline-2154.tif")

from qgis.core import QgsApplication, QgsCoordinateReferenceSystem, QgsProject  # noqa: E402
from qgis.PyQt.QtCore import QCoreApplication, QProcess, QTimer  # noqa: E402

app = QgsApplication([], False)
QgsApplication.initQgis()
# Le fournisseur GDAL du cadre de traitement, comme dans QGIS ouvert (extension Processing).
sys.path.append(os.path.join(QgsApplication.prefixPath(), "python", "plugins"))
from processing.core.Processing import Processing  # noqa: E402

Processing.initialize()
from osgeo import gdal  # noqa: E402

gdal.UseExceptions()
import analysis  # noqa: E402
import client  # noqa: E402
import gestures  # noqa: E402
import raster_output  # noqa: E402

checks = []
state = {"client": None}


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


def find_core():
    candidates = [os.environ.get("CASSINI_CORE")]
    for profile in ("release", "debug"):
        candidates.append(os.path.join(ROOT, "target", profile, "cassini.exe"))
        candidates.append(os.path.join(ROOT, "target", profile, "cassini"))
    for path in candidates:
        if path and os.path.isfile(path):
            return path
    return None


def sha256(path):
    with open(path, "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()


def grid(path):
    """(largeur, hauteur, géotransformation, bandes, type, sans-donnée, valeurs) d'un raster."""
    ds = gdal.Open(path)
    band = ds.GetRasterBand(1)
    values = band.ReadAsArray().tolist()
    out = (ds.RasterXSize, ds.RasterYSize, tuple(round(v, 6) for v in ds.GetGeoTransform()), ds.RasterCount,
           gdal.GetDataTypeName(band.DataType), band.GetNoDataValue(), values)
    ds = None
    return out


work = tempfile.mkdtemp(prefix="cassini-convert-raster-")
os.makedirs(os.path.join(work, "mnt"))
source = os.path.join(work, "mnt", "colline.asc")
shutil.copyfile(FIXTURE, source)
with_system = os.path.join(work, "mnt", "colline-2154.tif")
shutil.copyfile(WITH_SYSTEM, with_system)
real = os.environ.get("CASSINI_ASC")
if real and os.path.isfile(real):
    real_copy = os.path.join(work, "mnt", os.path.basename(real))
    shutil.copyfile(real, real_copy)
else:
    real_copy = None
    print("(pas de CASSINI_ASC : seule la fixture synthétique est convertie)")
originals = {p: sha256(p) for p in (source, with_system, real_copy) if p}
project_file = os.path.join(work, "essai.qgz").replace("\\", "/")
cassini_dir = os.path.join(work, "essai.cassini")
project = QgsProject.instance()
project.setCrs(QgsCoordinateReferenceSystem("EPSG:2154"))
assert project.write(project_file), "projet non enregistré"

# -- phase 1 : le geste seul ----------------------------------------------------
direct = os.path.join(work, "direct")
os.makedirs(direct)
CASES = [
    ("sans système, déclaré 2154", {"raster": source, "assign_epsg": 2154}),
    ("sans système, rien déclaré", {"raster": source}),
    ("autre système déclaré", {"raster": with_system, "assign_epsg": 32632}),
    ("même système déclaré", {"raster": with_system, "assign_epsg": 2154}),
    ("code inconnu", {"raster": source, "assign_epsg": 99999}),
    ("fichier absent", {"raster": os.path.join(work, "mnt", "absent.asc"), "assign_epsg": 2154}),
    ("annulation", {"raster": source, "assign_epsg": 2154}),
]
if real_copy:
    CASES.append(("vrai ASCII Grid de la BD ALTI", {"raster": real_copy, "assign_epsg": 2154}))
outcomes = {}


def run_case(i=0):
    if i >= len(CASES):
        QTimer.singleShot(0, protect(after_direct))
        return
    label, params = CASES[i]
    params = dict(params, out=os.path.join(direct, "%d-data.convert_raster.tif" % (i + 1)))

    def done(result=None, error=None):
        outcomes[label] = (result, error, params["out"])
        QTimer.singleShot(0, protect(lambda: run_case(i + 1)))

    gesture = analysis.ProcessingGesture(analysis.ALGORITHMS["data.convert_raster"])
    state["gesture"] = gesture
    gesture.start(params, done)
    if label == "annulation":
        gesture.cancel()


def after_direct():
    result, error, out = outcomes["sans système, déclaré 2154"]
    check("déclaré 2154 : pas d'erreur", error is None, error and (error.code, error.message))
    if result:
        check("déclaré 2154 : epsg 2154 et le fichier du noyau", result == {"file": out, "epsg": 2154}, result)
        src, dst = grid(source), grid(out)
        check("même grille (taille, géotransformation)", src[:3] == dst[:3], (src[:3], dst[:3]))
        check("une bande Float32, sans-donnée -9999", dst[3:6] == (1, "Float32", -9999.0), dst[3:6])
        flat_src = [v for row in src[6] for v in row]
        flat_dst = [v for row in dst[6] for v in row]
        holes_src = [k for k, v in enumerate(flat_src) if v == -99999]
        holes_dst = [k for k, v in enumerate(flat_dst) if v == -9999]
        check("les 3 cellules -99999 deviennent -9999", len(holes_src) == 3 and holes_src == holes_dst,
              (holes_src, holes_dst))
        gap = max(abs(a - b) for k, (a, b) in enumerate(zip(flat_src, flat_dst)) if k not in holes_src)
        check("valeurs identiques ailleurs (< 1e-3)", gap < 1e-3, gap)
    result, error, _ = outcomes["sans système, rien déclaré"]
    check("rien déclaré : GeoTIFF sans système (epsg null)", error is None and result["epsg"] is None,
          (result, error and error.code))
    result, error, out = outcomes["autre système déclaré"]
    check("autre système : analysis.bad_params qui nomme EPSG:2154, sans dossier",
          error is not None and error.code == "analysis.bad_params" and "EPSG:2154" in error.message
          and work not in error.message, error and (error.code, error.message))
    check("autre système : aucun fichier", not os.path.exists(out), out)
    result, error, _ = outcomes["même système déclaré"]
    check("même système déclaré : accepté, EPSG:2154", error is None and result["epsg"] == 2154,
          (result, error and error.code))
    _, error, out = outcomes["code inconnu"]
    check("code inconnu : data.unknown_crs", error is not None and error.code == "data.unknown_crs"
          and not os.path.exists(out), error and error.code)
    _, error, _ = outcomes["fichier absent"]
    check("fichier absent : layer.not_found", error is not None and error.code == "layer.not_found",
          error and error.code)
    _, error, out = outcomes["annulation"]
    check("annulation : qgis.cancelled, aucun fichier", error is not None and error.code == "qgis.cancelled"
          and not os.path.exists(out), error and error.code)
    if real_copy:
        result, error, out = outcomes["vrai ASCII Grid de la BD ALTI"]
        check("BD ALTI : pas d'erreur", error is None, error and (error.code, error.message))
        if result:
            src, dst = grid(real_copy), grid(out)
            check("BD ALTI : même grille", src[:3] == dst[:3], (src[:3], dst[:3]))
            check("BD ALTI : Float32, -9999, EPSG:2154", dst[3:6] == (1, "Float32", -9999.0)
                  and result["epsg"] == 2154, (dst[3:6], result))
            flat_src = [v for row in src[6] for v in row]
            flat_dst = [v for row in dst[6] for v in row]
            holes = sum(1 for v in flat_src if v == -99999)
            gap = max(abs(a - b) for a, b in zip(flat_src, flat_dst) if a != -99999)
            check("BD ALTI : valeurs identiques (%d cellules sans donnée)" % holes, gap < 1e-3, gap)
            print("       BD ALTI : %d × %d cellules, %s" % (dst[0], dst[1], os.path.basename(real_copy)))
    for path, digest in originals.items():
        check("original inchangé : %s" % os.path.basename(path), sha256(path) == digest)
    core = find_core()
    if core is None:
        check("noyau trouvé (CASSINI_CORE)", False)
        finish()
        return
    state["core"] = core
    project.clear()
    project.setCrs(QgsCoordinateReferenceSystem("EPSG:2154"))
    assert project.write(project_file), "projet non réenregistré"
    new_client(on_first)


# -- phase 2 : par le noyau, puis rejeu après réouverture ---------------------
def read_journal():
    with open(os.path.join(cassini_dir, "journal.jsonl"), encoding="utf-8") as f:
        return [json.loads(line) for line in f if line.strip()]


def new_client(on_event):
    process = QProcess()
    registry = gestures.GestureRegistry()
    registry.register("layer.open_raster", raster_output.LayerOpenRasterGesture)
    analysis.register_all(registry)
    state["process"] = process
    state["client"] = client.CoreClient(process, protect(on_event), registry)
    state["client"].start(state["core"], project_file)


source_rel = "mnt/" + os.path.basename(source)  # la fixture : ses cellules -99999 passent par le noyau
results = []
STEPS = [
    ("data.convert_raster", {"raster": source_rel, "assign_epsg": 2154}),
    ("terrain.slope", {"dem": "out:1.1", "unit": "degrees"}),
]


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
    converted, slope = results
    check("par le noyau : GeoTIFF en EPSG:2154", converted.get("epsg") == 2154, converted)
    check("la pente lit le GeoTIFF de QGIS (AD-20)", isinstance(slope.get("max"), (int, float))
          and slope["max"] > 0, slope)
    # Les 3 cellules sans donnée de la fixture, réécrites en -9999, sont lues comme telles par le
    # noyau : la pente en compte au moins autant (une cellule sans voisin complet aussi).
    check("le noyau lit le sans-donnée -9999 (pente : nodata_cells ≥ 3)",
          isinstance(slope.get("nodata_cells"), int) and slope["nodata_cells"] >= 3, slope)
    print("       pente : de %s à %s degrés" % (slope.get("min"), slope.get("max")))
    rows = read_journal()
    ids = [r["id"] for r in rows]
    check("journal : conversion, ouverture dictée, pente, ouverture dictée",
          ids == ["data.convert_raster", "layer.open_raster", "terrain.slope", "layer.open_raster"], ids)
    check("journal : la source relative, la pente sur out:1.1",
          rows[0]["params"]["raster"] == source_rel and rows[2]["params"]["dem"] == "out:1.1",
          (rows[0]["params"], rows[2]["params"]))
    check("la conversion ouverte dans le projet",
          len(project.mapLayersByName("1-data.convert_raster")) == 1)
    state["journal"] = rows
    state["client"].stop()
    project.clear()
    check("le projet rouvre", project.read(project_file))
    new_client(on_second)


def on_second(kind, payload):
    if kind == client.READY:
        check("le rejeu part", state["client"].replay())
    elif kind == client.REPLAY_DONE:
        check("le rejeu rend identical: true", payload == {"steps": 4, "identical": True}, payload)
        replayed = os.path.join(cassini_dir, "replay", "1-data.convert_raster.tif")
        check("la conversion est rejouée sous replay/", os.path.isfile(replayed), replayed)
        check("le journal est inchangé par le rejeu", read_journal() == state["journal"])
        finish()
    elif kind in (client.REPLAY_FAILED, client.FAILED, client.DIED):
        check("rejeu nominal", False, "%s %s" % (kind, payload))
        finish()


QTimer.singleShot(0, protect(run_case))
QTimer.singleShot(300000, QCoreApplication.quit)
QCoreApplication.exec()

if state["client"] is not None:
    state["client"].stop()
for path, digest in originals.items():
    check("original toujours inchangé : %s" % os.path.basename(path), sha256(path) == digest)
ok = len(checks) >= 20 and all(v for _, v in checks)
project.clear()
shutil.rmtree(work, ignore_errors=True)
print("RÉSULTAT :", "OK" if ok else "ÉCHEC")
sys.stdout.flush()
os._exit(0 if ok else 1)
