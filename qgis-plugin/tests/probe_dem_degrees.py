"""Sonde manuelle sous le vrai QGIS : `raster.reproject_work`, MNT en degrés (ticket 33, seconde moitié).

À lancer avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`) :

    set QT_QPA_PLATFORM=windows
    set CASSINI_CORE=C:\\ct-5-57g\\release\\cassini.exe
    "C:\\Program Files\\QGIS 3.44.15\\bin\\python-qgis-ltr.bat" qgis-plugin/tests/probe_dem_degrees.py

Un MNT en EPSG:4326 est fabriqué depuis `fixtures/mnt-colline-2154.tif` (reprojeté avec GDAL), ainsi
qu'une copie entière (Int16) pour les classes.

1. Geste seul, fournisseur GDAL de QGIS chargé : MNT flottant 4326 vers 2154 — même grille et mêmes
   valeurs (tolérance) que `gdal.Warp` fait à la main (bilinéaire, Float32, -9999) ; raster entier —
   plus proche voisin, aucune valeur nouvelle (classes non mélangées) ; système du raster refusé
   (`analysis.bad_params`, aucun fichier) ; annulation — `qgis.cancelled`, aucun fichier ; originaux
   inchangés (empreinte).
2. Par le noyau : `terrain.slope` sur le MNT en degrés — le noyau dicte `raster.reproject_work` avant
   la pente, et la pente égale celle du MNT reprojeté à la main ; projet fermé et rouvert, rejeu du
   journal : `identical: true`. Le rééchantillonnage vient du noyau (`resampling`) : un MNT d'altitudes
   entières (Int16) reprojeté en « bilinear » égale le gdalwarp bilinéaire fait à la main.
3. Ticket 33c, suite : `terrain.viewshed`, `terrain.viewshed_cumulative`, `terrain.line_of_sight` et
   `terrain.peaks` sur le MNT en degrés, avec des points WGS84 — le noyau dicte la reprojection puis
   `data.transform_point` pour chaque point, et le résultat égale celui du MNT reprojeté à la main avec
   les points convertis à la main (Lambert-93).
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
FIXTURE = os.path.join(ROOT, "fixtures", "mnt-colline-2154.tif")

from qgis.core import (  # noqa: E402
    QgsApplication,
    QgsCoordinateReferenceSystem,
    QgsCoordinateTransform,
    QgsPointXY,
    QgsProject,
)
from qgis.PyQt.QtCore import QCoreApplication, QProcess, QTimer  # noqa: E402

app = QgsApplication([], False)
QgsApplication.initQgis()
sys.path.append(os.path.join(QgsApplication.prefixPath(), "python", "plugins"))
from processing.core.Processing import Processing  # noqa: E402

Processing.initialize()
from osgeo import gdal  # noqa: E402

gdal.UseExceptions()
import analysis  # noqa: E402
import client  # noqa: E402
import delegated  # noqa: E402

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
    """(largeur, hauteur, géotransformation, bandes, type, sans-donnée, valeurs, EPSG) d'un raster."""
    ds = gdal.Open(path)
    band = ds.GetRasterBand(1)
    srs = ds.GetSpatialRef()
    code = srs.GetAuthorityCode(None) if srs else None
    out = (ds.RasterXSize, ds.RasterYSize, tuple(ds.GetGeoTransform()), ds.RasterCount,
           gdal.GetDataTypeName(band.DataType), band.GetNoDataValue(), band.ReadAsArray().tolist(), code)
    ds = None
    return out


def flat(values):
    return [v for row in values for v in row]


def hand_warp(source, out, resample, out_type):
    """La reprojection « faite à la main » : gdalwarp vers 2154 avec les réglages fixés par la fiche."""
    gdal.Warp(out, source, dstSRS="EPSG:2154", resampleAlg=resample, outputType=out_type, dstNodata=-9999,
              srcBands=[1], dstBands=[1])
    return out


work = tempfile.mkdtemp(prefix="cassini-dem-degrees-")
os.makedirs(os.path.join(work, "mnt"))
metric = os.path.join(work, "mnt", "colline-2154.tif")
shutil.copyfile(FIXTURE, metric)
degrees = os.path.join(work, "mnt", "colline-4326.tif")
gdal.Warp(degrees, metric, dstSRS="EPSG:4326", resampleAlg="bilinear", outputType=gdal.GDT_Float32,
          dstNodata=-9999)
classes_src = os.path.join(work, "mnt", "classes-2154.tif")
gdal.Translate(classes_src, metric, outputType=gdal.GDT_Int16, scaleParams=[[0, 1000, 1, 5]])
classes = os.path.join(work, "mnt", "classes-4326.tif")
gdal.Warp(classes, classes_src, dstSRS="EPSG:4326", resampleAlg="near", outputType=gdal.GDT_Int16,
          dstNodata=-9999)
# Un MNT d'altitudes entières (mètres arrondis), en degrés : le type de la bande ne dit rien du rééchantillonnage.
int_src = os.path.join(work, "mnt", "alti-int-2154.tif")
gdal.Translate(int_src, metric, outputType=gdal.GDT_Int16)
int_dem = os.path.join(work, "mnt", "alti-int-4326.tif")
gdal.Warp(int_dem, int_src, dstSRS="EPSG:4326", resampleAlg="near", outputType=gdal.GDT_Int16, dstNodata=-9999)
originals = {p: sha256(p) for p in (metric, degrees, classes, int_dem)}
check("le MNT fabriqué est en degrés", grid(degrees)[7] == "4326")
project_file = os.path.join(work, "essai.qgz").replace("\\", "/")
cassini_dir = os.path.join(work, "essai.cassini")
project = QgsProject.instance()
project.setCrs(QgsCoordinateReferenceSystem("EPSG:2154"))
assert project.write(project_file), "projet non enregistré"

# -- phase 1 : le geste seul ----------------------------------------------------
direct = os.path.join(work, "direct")
os.makedirs(direct)
CASES = [
    ("flottant", {"raster": degrees, "work_epsg": 2154, "resampling": "bilinear"}),
    ("entier", {"raster": classes, "work_epsg": 2154, "resampling": "nearest"}),
    ("altitudes entières", {"raster": int_dem, "work_epsg": 2154, "resampling": "bilinear"}),
    ("identité", {"raster": metric, "work_epsg": 2154}),
    ("annulation", {"raster": degrees, "work_epsg": 2154}),
]
outcomes = {}


def run_case(i=0):
    if i >= len(CASES):
        QTimer.singleShot(0, protect(after_direct))
        return
    label, params = CASES[i]
    params = dict(params, out=os.path.join(direct, "%d-raster.reproject_work.tif" % (i + 1)))

    def done(result=None, error=None):
        outcomes[label] = (result, error, params["out"])
        QTimer.singleShot(0, protect(lambda: run_case(i + 1)))

    gesture = analysis.ProcessingGesture(analysis.ALGORITHMS["raster.reproject_work"])
    state["gesture"] = gesture
    gesture.start(params, done)
    if label == "annulation":
        gesture.cancel()


def after_direct():
    result, error, out = outcomes["flottant"]
    check("flottant : pas d'erreur", error is None, error and (error.code, error.message))
    if result:
        check("flottant : le fichier du noyau, en EPSG:2154", result == {"file": out, "epsg": 2154}, result)
        mine = grid(out)
        hand = grid(hand_warp(degrees, os.path.join(work, "main-flottant.tif"), "bilinear", gdal.GDT_Float32))
        check("flottant : même grille que la reprojection à la main", mine[:3] == hand[:3], (mine[:3], hand[:3]))
        check("flottant : une bande Float32, sans-donnée -9999", mine[3:6] == (1, "Float32", -9999.0), mine[3:6])
        pairs = list(zip(flat(mine[6]), flat(hand[6])))
        check("flottant : mêmes cellules sans donnée", all((a == -9999) == (b == -9999) for a, b in pairs))
        gap = max((abs(a - b) for a, b in pairs if a != -9999 and b != -9999), default=0)
        check("flottant : mêmes valeurs qu'à la main (< 1e-3)", gap < 1e-3, gap)
        print("       grille %d x %d, écart max %.2e" % (mine[0], mine[1], gap))
    result, error, out = outcomes["entier"]
    check("entier : pas d'erreur", error is None, error and (error.code, error.message))
    if result:
        mine = grid(out)
        hand = grid(hand_warp(classes, os.path.join(work, "main-entier.tif"), "near", gdal.GDT_Float32))
        source_values = {float(v) for v in flat(grid(classes)[6]) if v != -9999}
        written = {v for v in flat(mine[6]) if v != -9999}
        check("entier : classes non mélangées (aucune valeur nouvelle)", written <= source_values,
              (sorted(written), sorted(source_values)))
        check("entier : égal au plus proche voisin à la main", mine[6] == hand[6])
    result, error, out = outcomes["altitudes entières"]
    check("altitudes Int16 : pas d'erreur", error is None, error and (error.code, error.message))
    if result:
        mine = grid(out)
        hand = grid(hand_warp(int_dem, os.path.join(work, "main-int.tif"), "bilinear", gdal.GDT_Float32))
        check("altitudes Int16 bilinéaire : même grille qu'à la main", mine[:3] == hand[:3])
        pairs = list(zip(flat(mine[6]), flat(hand[6])))
        gap = max((abs(a - b) for a, b in pairs if a != -9999 and b != -9999), default=0)
        check("altitudes Int16 bilinéaire : mêmes valeurs qu'à la main (< 1e-3)", gap < 1e-3, gap)
        nearest = grid(hand_warp(int_dem, os.path.join(work, "main-int-near.tif"), "near", gdal.GDT_Float32))
        check("altitudes Int16 : ce n'est pas le plus proche voisin", mine[6] != nearest[6])
    _, error, out = outcomes["identité"]
    check("identité : analysis.bad_params, rien écrit",
          error is not None and error.code == "analysis.bad_params" and not os.path.exists(out)
          and work not in error.message, error and (error.code, error.message))
    _, error, out = outcomes["annulation"]
    check("annulation : qgis.cancelled, aucun fichier", error is not None and error.code == "qgis.cancelled"
          and not os.path.exists(out), error and error.code)
    for path, digest in originals.items():
        check("original inchangé : %s" % os.path.basename(path), sha256(path) == digest)
    # Le MNT reprojeté à la main, dans le projet, pour comparer la pente du noyau à la sienne.
    hand_warp(degrees, os.path.join(work, "mnt", "main-2154.tif"), "bilinear", gdal.GDT_Float32)
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


class NoProbe:  # le geste de test du noyau n'est jamais dicté ici
    def start(self, params, done):
        done(result={})

    def cancel(self):
        pass


def new_client(on_event):
    process = QProcess()
    registry = delegated.build_registry(NoProbe)
    state["process"] = process
    state["client"] = client.CoreClient(process, protect(on_event), registry)
    state["client"].start(state["core"], project_file)


results = []


def wgs84_of(x, y):
    """Le point Lambert-93 (x, y) en WGS84, fait à la main (QgsCoordinateTransform)."""
    tr = QgsCoordinateTransform(QgsCoordinateReferenceSystem("EPSG:2154"),
                                QgsCoordinateReferenceSystem("EPSG:4326"), project)
    p = tr.transform(QgsPointXY(x, y))
    return {"x": p.x(), "y": p.y(), "epsg": 4326}


def l93(x, y):
    return {"x": x, "y": y, "epsg": 2154}


RADIUS = {"value": 300, "unit": "m"}
OBS = (652250.0, 6850250.0)
OBS2 = (652100.0, 6850100.0)
TARGET = (652400.0, 6850250.0)
DEG, MAIN = "mnt/colline-4326.tif", "mnt/main-2154.tif"
STEPS = [
    ("terrain.slope", {"dem": DEG, "unit": "degrees"}),
    ("terrain.slope", {"dem": MAIN, "unit": "degrees"}),
    ("terrain.viewshed", {"dem": DEG, "observer": wgs84_of(*OBS), "radius": RADIUS}),
    ("terrain.viewshed", {"dem": MAIN, "observer": l93(*OBS), "radius": RADIUS}),
    ("terrain.viewshed_cumulative",
     {"dem": DEG, "observers": [wgs84_of(*OBS), wgs84_of(*OBS2)], "radius": RADIUS}),
    ("terrain.viewshed_cumulative",
     {"dem": MAIN, "observers": [l93(*OBS), l93(*OBS2)], "radius": RADIUS}),
    ("terrain.line_of_sight",
     {"dem": DEG, "observer": wgs84_of(*OBS), "target": wgs84_of(*TARGET),
      "observer_height": {"value": 2, "unit": "m"}, "target_height": {"value": 2, "unit": "m"}}),
    ("terrain.line_of_sight",
     {"dem": MAIN, "observer": l93(*OBS), "target": l93(*TARGET),
      "observer_height": {"value": 2, "unit": "m"}, "target_height": {"value": 2, "unit": "m"}}),
    ("terrain.peaks", {"dem": DEG, "min_prominence": {"value": 5, "unit": "m"}}),
    ("terrain.peaks", {"dem": MAIN, "min_prominence": {"value": 5, "unit": "m"}}),
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
    slope_deg, slope_hand = results[0], results[1]
    rows = read_journal()
    ids = [r["id"] for r in rows]
    print("       journal :", ids)
    check("le noyau dicte la reprojection avant la pente",
          "raster.reproject_work" in ids and ids.index("raster.reproject_work") < ids.index("terrain.slope"), ids)
    reproject = next((r for r in rows if r["id"] == "raster.reproject_work"), {"params": {}})
    check("journal : work_epsg 2154 et rééchantillonnage bilinéaire choisis par le noyau",
          reproject["params"].get("work_epsg") == 2154 and reproject["params"].get("resampling") == "bilinear",
          reproject["params"])
    mine, hand = grid(slope_deg["file"]), grid(slope_hand["file"])
    check("pente : en EPSG:2154", mine[7] == "2154", mine[7])
    check("pente : même grille que celle du MNT reprojeté à la main", mine[:3] == hand[:3], (mine[:3], hand[:3]))
    pairs = list(zip(flat(mine[6]), flat(hand[6])))
    check("pente : mêmes cellules sans donnée", all((x == -9999) == (y == -9999) for x, y in pairs))
    gap = max((abs(x - y) for x, y in pairs if x != -9999 and y != -9999), default=0)
    check("pente : égale à celle du MNT reprojeté à la main (< 1e-3)", gap < 1e-3, gap)
    print("       pente : écart max %.2e" % gap)
    check_points(rows)
    state["journal"] = rows
    state["client"].stop()
    project.clear()
    check("le projet rouvre", project.read(project_file))
    new_client(on_second)


def same_raster(label, a, b):
    mine, hand = grid(a["file"]), grid(b["file"])
    check("%s : en EPSG:2154" % label, mine[7] == "2154", mine[7])
    check("%s : même grille que celle du MNT reprojeté à la main" % label, mine[:3] == hand[:3],
          (mine[:3], hand[:3]))
    pairs = list(zip(flat(mine[6]), flat(hand[6])))
    check("%s : mêmes cellules sans donnée" % label, all((x == -9999) == (y == -9999) for x, y in pairs))
    gap = max((abs(x - y) for x, y in pairs if x != -9999 and y != -9999), default=0)
    check("%s : égal à celui du MNT reprojeté à la main (< 1e-3)" % label, gap < 1e-3, gap)
    return pairs


def check_points(rows):
    """Champ de vision, cumulé, ligne de vue, points hauts : degrés + points WGS84 contre la main."""
    ids = [r["id"] for r in rows]
    for command, count in (("terrain.viewshed", 1), ("terrain.viewshed_cumulative", 2), ("terrain.line_of_sight", 2)):
        at = [i for i, r in enumerate(rows) if r["id"] == command and r["params"].get("work_epsg") == 2154]
        check("%s : une ligne dictée avec work_epsg 2154" % command, len(at) == 1, ids)
        if len(at) != 1:
            continue
        i = at[0]
        before = ids[max(0, i - 1 - count):i]
        check("%s : reprojection puis %d conversion(s) de point juste avant" % (command, count),
              before == ["raster.reproject_work"] + ["data.transform_point"] * count, before)
        converts = [r for r in rows[i - count:i]]
        check("%s : conversions vers EPSG:2154" % command,
              all(r["params"].get("target_epsg") == 2154 for r in converts))
    peaks_at = [r for r in rows if r["id"] == "terrain.peaks" and r["params"].get("work_epsg") == 2154]
    check("terrain.peaks : une ligne dictée avec work_epsg 2154", len(peaks_at) == 1)
    transforms = [r for r in rows if r["id"] == "data.transform_point"]
    check("cinq points WGS84 convertis en tout (1 + 2 + 2)", len(transforms) == 5, len(transforms))
    # métrique : aucun point converti, aucune reprojection pour les calculs sur le MNT à la main
    check("MNT métrique : une seule reprojection par calcul en degrés",
          ids.count("raster.reproject_work") == 5, ids.count("raster.reproject_work"))
    same_raster("champ de vision", results[2], results[3])
    same_raster("champ de vision cumulé", results[4], results[5])
    seen = results[2]
    check("champ de vision : des cellules visibles", seen.get("visible_cells", 0) > 0, seen)
    a, b = results[6], results[7]
    check("ligne de vue : même verdict qu'à la main", a["visible"] == b["visible"], (a["visible"], b["visible"]))
    check("ligne de vue : même distance (< 1e-3 m)", abs(a["distance"] - b["distance"]) < 1e-3,
          (a["distance"], b["distance"]))
    oa, ob = a.get("obstruction"), b.get("obstruction")
    check("ligne de vue : même obstacle, en EPSG:2154",
          (oa is None and ob is None)
          or (oa is not None and ob is not None and oa["epsg"] == 2154
              and abs(oa["x"] - ob["x"]) < 1e-3 and abs(oa["y"] - ob["y"]) < 1e-3), (oa, ob))
    pa, pb = results[8], results[9]
    check("points hauts : au moins un sommet", pa["feature_count"] >= 1, pa)
    check("points hauts : mêmes sommets qu'à la main, en EPSG:2154",
          pa["feature_count"] == pb["feature_count"]
          and all(x["point"]["epsg"] == 2154 and abs(x["point"]["x"] - y["point"]["x"]) < 1e-3
                  and abs(x["point"]["y"] - y["point"]["y"]) < 1e-3
                  and abs(x["elevation"] - y["elevation"]) < 1e-3
                  for x, y in zip(pa["peaks"], pb["peaks"])), (pa["peaks"], pb["peaks"]))


def on_second(kind, payload):
    if kind == client.READY:
        check("le rejeu part", state["client"].replay())
    elif kind == client.REPLAY_DONE:
        check("le rejeu rend identical: true", payload.get("identical") is True, payload)
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
ok = len(checks) >= 40 and all(v for _, v in checks)
project.clear()
shutil.rmtree(work, ignore_errors=True)
print("RÉSULTAT :", "OK" if ok else "ÉCHEC")
print("VERDICT", "OK" if ok else "ECHEC")
sys.stdout.flush()
os._exit(0 if ok else 1)
