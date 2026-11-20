"""Sonde manuelle sous le vrai QGIS : courbes de niveau d'un MNT (ticket 26, `raster.contour`).

À lancer avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`) :

    "C:\\Program Files\\QGIS 3.44.15\\bin\\python-qgis-ltr.bat" qgis-plugin/tests/probe_contour.py

Aucun noyau ni aucune donnée hors dépôt : les MNT synthétiques de `fixtures/` (copiés dans un dossier
temporaire, pour comparer l'empreinte avant et après), le vrai geste `raster.contour`. Affiche la
version de QGIS, puis termine par `VERDICT : OK` ou `VERDICT : ÉCHEC`.

Ce que la sonde constate (le plan interdit de le supposer) :
1. `gdal:contour` est dans le registre de traitement (sinon la commande ne serait pas annoncée).
2. Plan incliné (valeur = 100 - (x - 652000) / 10 au centre des cellules), 5 m : une ligne droite et
   verticale par palier, `elevation` multiple de 5 et exacte, placée là où l'altitude vaut le palier.
3. Colline (50 à 150 m), 5 m : courbes fermées, `elevation` multiples de 5 entre 50 et 150.
4. Équidistance de 0,005 km : mêmes paliers et même nombre de courbes que 5 m.
5. Le raster d'origine reste identique (empreinte SHA-256) et aucun `.aux.xml` n'apparaît.
6. Raster absent : `layer.not_found` ; fichier non GeoTIFF (ASCII Grid sans système compris) :
   `layer.invalid_source` ; aucun fichier écrit.
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

print("QGIS", Qgis.version())
work = tempfile.mkdtemp(prefix="cassini-contour-")
project = QgsProject.instance()
project.setCrs(QgsCoordinateReferenceSystem("EPSG:2154"))
checks = []
outcomes = {}

SLOPE_SRC = os.path.join(ROOT, "fixtures", "mnt-plan-incline-2154.tif")
HILL_SRC = os.path.join(ROOT, "fixtures", "mnt-colline-2154.tif")
NO_CRS_SRC = os.path.join(ROOT, "fixtures", "mnt-colline-sans-systeme.asc")
SLOPE = os.path.join(work, "plan.tif")
HILL = os.path.join(work, "colline.tif")
NO_CRS = os.path.join(work, "sans-systeme.asc")
shutil.copy(SLOPE_SRC, SLOPE)
shutil.copy(HILL_SRC, HILL)
shutil.copy(NO_CRS_SRC, NO_CRS)
ORIGINALS = {}


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
    gesture = analysis.ProcessingGesture(analysis.ALGORITHMS["raster.contour"])

    def done(result=None, error=None):
        outcomes[label] = (result, error)
        QTimer.singleShot(0, protect(then))

    outcomes["_g_" + label] = gesture  # gardé en vie pendant la tâche
    gesture.start(params, done)


def lines_of(path):
    """(altitude, [sommets]) de chaque courbe de la couche écrite."""
    layer = QgsVectorLayer(path, "lecture", "ogr")
    names = [f.name() for f in layer.fields()]
    found = []
    for feature in layer.getFeatures():
        geometry = feature.geometry()
        parts = geometry.asMultiPolyline() if geometry.isMultipart() else [geometry.asPolyline()]
        found.append((feature["elevation"], [[(p.x(), p.y()) for p in part] for part in parts]))
    del layer
    return names, found


def aux_files():
    return [n for n in os.listdir(work) if n.endswith(".aux.xml")]


def step_slope():
    run_gesture({"raster": SLOPE, "interval": {"value": 5, "unit": "m"}, "out": out("1-raster.contour.gpkg")},
                "plan", step_slope_done)


def step_slope_done():
    result, error = outcomes["plan"]
    check("plan incliné : pas d'erreur", error is None, error and (error.code, error.message))
    if result is None:
        finish()
        return
    check("plan incliné : SCR EPSG:2154, couche nommée par le noyau",
          result["crs"] == "EPSG:2154" and result["name"] == "1-raster.contour", result)
    names, found = lines_of(out("1-raster.contour.gpkg"))
    print("       champs de la sortie : %s ; %d courbes ; altitudes %s" % (names, len(found), sorted(e for e, _ in found)))
    check("champ `elevation` présent", "elevation" in names, names)
    check("feature_count = nombre de courbes écrites", result["feature_count"] == len(found), (result, len(found)))
    elevations = sorted(e for e, _ in found)
    check("altitudes toutes multiples de 5", elevations and all(abs(e / 5 - round(e / 5)) < 1e-9 for e in elevations), elevations)
    check("une courbe par palier (pas de doublon)", len(set(elevations)) == len(elevations), elevations)
    # Le plan va de 60,5 à 99,5 : paliers 65 à 95.
    check("paliers 65 à 95", elevations == [65.0 + 5 * i for i in range(7)], elevations)
    straight = True
    placed = True
    for elevation, parts in found:
        points = [p for part in parts for p in part]
        xs = {round(x, 6) for x, _ in points}
        straight = straight and len(xs) == 1
        expected_x = 652000 + 10 * (100 - elevation)
        placed = placed and all(abs(x - expected_x) < 1e-3 for x in xs)
    check("chaque courbe est une droite verticale", straight)
    check("chaque courbe est à l'abscisse où l'altitude vaut son palier", placed,
          [(e, sorted({round(x, 3) for part in parts for x, _ in part})) for e, parts in found][:3])
    check("raster d'origine intact (empreinte)", sha(SLOPE) == ORIGINALS[SLOPE])
    check("aucun .aux.xml à côté du raster", not aux_files(), os.listdir(work))
    step_hill()


def step_hill():
    run_gesture({"raster": HILL, "interval": {"value": 5, "unit": "m"}, "out": out("2-raster.contour.gpkg")},
                "colline", step_hill_done)


def step_hill_done():
    result, error = outcomes["colline"]
    check("colline : pas d'erreur", error is None, error and (error.code, error.message))
    if result is None:
        finish()
        return
    _, found = lines_of(out("2-raster.contour.gpkg"))
    elevations = sorted({e for e, _ in found})
    print("       %d courbes, altitudes de %s à %s" % (len(found), elevations[:1], elevations[-1:]))
    check("colline : des courbes", len(found) > 0, result)
    check("colline : altitudes multiples de 5 entre 50 et 150",
          all(abs(e / 5 - round(e / 5)) < 1e-9 and 50 <= e <= 150 for e in elevations), elevations)
    def closed(parts):
        return all(len(part) > 3 and part[0] == part[-1] for part in parts)

    def on_border(parts):  # emprise du raster : x 652000 à 652500, y 6850000 à 6850500
        ends = [part[i] for part in parts for i in (0, -1)]
        return all(x in (652000.0, 652500.0) or y in (6850000.0, 6850500.0) for x, y in ends)

    shut = [e for e, parts in found if closed(parts)]
    check("colline : des courbes fermées", len(shut) > 0, (len(shut), len(found)))
    check("colline : toute courbe ouverte s'arrête au bord du raster",
          all(closed(parts) or on_border(parts) for _, parts in found),
          [e for e, parts in found if not closed(parts) and not on_border(parts)])
    outcomes["colline_count"] = (len(found), elevations)
    check("colline : raster d'origine intact", sha(HILL) == ORIGINALS[HILL])
    check("colline : aucun .aux.xml", not aux_files(), os.listdir(work))
    step_km()


def step_km():
    run_gesture({"raster": HILL, "interval": {"value": 0.005, "unit": "km"}, "out": out("3-raster.contour.gpkg")},
                "km", step_km_done)


def step_km_done():
    result, error = outcomes["km"]
    check("kilomètres : pas d'erreur", error is None, error and (error.code, error.message))
    if result is not None:
        _, found = lines_of(out("3-raster.contour.gpkg"))
        count, elevations = outcomes["colline_count"]
        check("0,005 km : mêmes courbes que 5 m", len(found) == count and sorted({e for e, _ in found}) == elevations,
              (len(found), count))
    step_missing()


def step_missing():
    run_gesture({"raster": out("absent.tif"), "interval": {"value": 5, "unit": "m"}, "out": out("4-raster.contour.gpkg")},
                "absent", step_missing_done)


def step_missing_done():
    _, error = outcomes["absent"]
    check("raster absent : layer.not_found", error is not None and error.code == "layer.not_found", error and error.code)
    check("raster absent : aucun fichier écrit", not os.path.exists(out("4-raster.contour.gpkg")))
    run_gesture({"raster": NO_CRS, "interval": {"value": 5, "unit": "m"}, "out": out("5-raster.contour.gpkg")},
                "asc", step_asc_done)


def step_asc_done():
    _, error = outcomes["asc"]
    check("ASCII Grid sans système : refus net (layer.invalid_source)",
          error is not None and error.code == "layer.invalid_source", error and error.code)
    check("ASCII Grid : aucun fichier écrit", not os.path.exists(out("5-raster.contour.gpkg")))
    check("ASCII Grid : source intacte", sha(NO_CRS) == ORIGINALS[NO_CRS])
    finish()


class _Backend:
    """Registre de traitement réel, pour la présence de l'algorithme."""


check("gdal:contour est dans le registre de traitement",
      QgsApplication.processingRegistry().algorithmById("gdal:contour") is not None)
for path in (SLOPE, HILL, NO_CRS):
    ORIGINALS[path] = sha(path)

QTimer.singleShot(0, protect(step_slope))
QTimer.singleShot(150000, lambda: (check("sonde terminée à temps", False), finish()))
app.exec()

failed = [label for label, ok in checks if not ok]
verdict = bool(checks) and not failed
print("VERDICT :", "OK" if verdict else "ÉCHEC (%s)" % ", ".join(failed))
QgsApplication.exitQgis()
shutil.rmtree(work, ignore_errors=True)
sys.exit(0 if verdict else 1)
