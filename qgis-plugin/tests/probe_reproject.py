"""Sonde manuelle sous le vrai QGIS : `data.reproject` (epic 10, entrée 7).

À lancer avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`),
après `cargo build` (ou `cargo test`) :

    "C:\\Program Files\\QGIS 3.44.15\\bin\\python-qgis-ltr.bat" qgis-plugin/tests/probe_reproject.py

Le noyau est cherché dans `CASSINI_CORE`, sinon `target/release/`, `target/debug/`, puis
`C:\\ct\\debug\\` (dossier de compilation des essais sous Windows).

Dans un dossier temporaire, une couche synthétique de trois points en EPSG:4326 (GeoJSON
écrit par la sonde, aucune donnée hors dépôt) :

1. geste seul : reprojection en EPSG:2154 — couche ajoutée en EPSG:2154, trois entités, le
   point (3 ; 46,5) à moins de 1 cm de l'origine du Lambert-93 (700 000 ; 6 600 000) ;
   même système (2154 vers 2154) : copie ; code inconnu (99999) : `data.unknown_crs` qui
   nomme EPSG:99999, rien lancé, aucun fichier ; couche absente : `layer.not_found` ;
   annulation : `qgis.cancelled`, aucun fichier partiel ;
2. par le noyau : `layer.add` puis `data.reproject`, projet fermé et rouvert, rejeu du
   journal : `identical: true`, la sortie rejouée existe sous `replay/` ;
3. le fichier d'origine est inchangé (empreinte) et la couche d'origine reste en EPSG:4326.
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

from qgis.analysis import QgsNativeAlgorithms  # noqa: E402
from qgis.core import (  # noqa: E402
    QgsApplication,
    QgsCoordinateReferenceSystem,
    QgsProject,
    QgsVectorLayer,
)
from qgis.PyQt.QtCore import QCoreApplication, QProcess, QTimer  # noqa: E402

app = QgsApplication([], False)
QgsApplication.initQgis()
QgsApplication.processingRegistry().addProvider(QgsNativeAlgorithms())
import analysis  # noqa: E402
import client  # noqa: E402
import gestures  # noqa: E402
import layer_add  # noqa: E402

POINTS = [("origine", 3.0, 46.5), ("paris", 2.3522, 48.8566), ("annecy", 6.1294, 45.8992)]
LAMBERT_ORIGIN = (700000.0, 6600000.0)


def find_core():
    candidates = [os.environ.get("CASSINI_CORE")]
    for profile in ("release", "debug"):
        candidates.append(os.path.join(ROOT, "target", profile, "cassini.exe"))
        candidates.append(os.path.join(ROOT, "target", profile, "cassini"))
    candidates.append(os.path.join("C:\\ct", "debug", "cassini.exe"))
    for path in candidates:
        if path and os.path.isfile(path):
            return path
    return None


def sha256(path):
    with open(path, "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()


work = tempfile.mkdtemp(prefix="cassini-reproject-")
source = os.path.join(work, "gares-4326.geojson")
with open(source, "w", encoding="utf-8") as f:
    json.dump(
        {
            "type": "FeatureCollection",
            "crs": {"type": "name", "properties": {"name": "urn:ogc:def:crs:EPSG::4326"}},
            "features": [
                {"type": "Feature", "properties": {"name": n}, "geometry": {"type": "Point", "coordinates": [x, y]}}
                for n, x, y in POINTS
            ],
        },
        f,
    )
source_hash = sha256(source)
project_file = os.path.join(work, "essai.qgz").replace("\\", "/")
cassini_dir = os.path.join(work, "essai.cassini")
project = QgsProject.instance()
project.setCrs(QgsCoordinateReferenceSystem("EPSG:2154"))
assert project.write(project_file), "projet non enregistré"

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


# -- phase 1 : le geste seul ----------------------------------------------------
gares = QgsVectorLayer(source, "gares", "ogr")
assert gares.isValid(), "GeoJSON illisible"
project.addMapLayer(gares)
check("la couche d'origine est en EPSG:4326", gares.crs().authid() == "EPSG:4326", gares.crs().authid())

direct = os.path.join(work, "direct")
os.makedirs(direct)
CASES = [
    ("4326 vers 2154", {"layer_id": gares.id(), "target_epsg": 2154}),
    ("même système", {"layer_id": "@first", "target_epsg": 2154}),
    ("code inconnu", {"layer_id": gares.id(), "target_epsg": 99999}),
    ("couche absente", {"layer_id": "absente", "target_epsg": 2154}),
    ("annulation", {"layer_id": gares.id(), "target_epsg": 2154}),
]
outcomes = {}


def run_case(i=0):
    if i >= len(CASES):
        QTimer.singleShot(0, protect(after_direct))
        return
    label, params = CASES[i]
    params = dict(params)
    if params["layer_id"] == "@first":
        params["layer_id"] = outcomes["4326 vers 2154"][0]["layer_id"]
    params["out"] = os.path.join(direct, "%d-data.reproject.gpkg" % (i + 1))

    def done(result=None, error=None):
        outcomes[label] = (result, error, params["out"])
        QTimer.singleShot(0, protect(lambda: run_case(i + 1)))

    gesture = analysis.ProcessingGesture(analysis.ALGORITHMS["data.reproject"])
    state["gesture"] = gesture
    gesture.start(params, done)
    if label == "annulation":
        # Annulé tout de suite, avant que le minuteur de Qt ne lance `_launch` : la voie
        # « pas encore de tâche » (aucune tâche QGIS créée), pas l'annulation d'une tâche en
        # cours, que seuls les tests par fond doublure couvrent.
        gesture.cancel()


def after_direct():
    result, error, out = outcomes["4326 vers 2154"]
    check("4326 vers 2154 : pas d'erreur", error is None, error and error.code)
    if result:
        check("la couche ajoutée est en EPSG:2154", result["crs"] == "EPSG:2154", result)
        check("même nombre d'entités", result["feature_count"] == len(POINTS), result)
        layer = project.mapLayer(result["layer_id"])
        origin = [f for f in layer.getFeatures() if f["name"] == "origine"]
        if origin:
            p = origin[0].geometry().asPoint()
            gap = ((p.x() - LAMBERT_ORIGIN[0]) ** 2 + (p.y() - LAMBERT_ORIGIN[1]) ** 2) ** 0.5
            check("(3 ; 46,5) tombe à moins de 1 cm de (700000 ; 6600000)", gap < 0.01, (p.x(), p.y(), gap))
        else:
            check("le point d'origine est dans la couche reprojetée", False)
        check("la sortie est le fichier du noyau", os.path.isfile(out), out)
    result, error, _ = outcomes["même système"]
    check("même système : copie en EPSG:2154", error is None and result["crs"] == "EPSG:2154"
          and result["feature_count"] == len(POINTS), (result, error and error.code))
    result, error, out = outcomes["code inconnu"]
    check("code inconnu : data.unknown_crs qui nomme EPSG:99999",
          error is not None and error.code == "data.unknown_crs" and "EPSG:99999" in error.message,
          error and (error.code, error.message))
    check("code inconnu : aucun fichier", not os.path.exists(out), out)
    _, error, _ = outcomes["couche absente"]
    check("couche absente : layer.not_found", error is not None and error.code == "layer.not_found",
          error and error.code)
    _, error, out = outcomes["annulation"]
    check("annulation : qgis.cancelled, aucun fichier", error is not None and error.code == "qgis.cancelled"
          and not os.path.exists(out), error and error.code)
    check("la couche d'origine reste en EPSG:4326", gares.crs().authid() == "EPSG:4326")
    check("le fichier d'origine est inchangé (empreinte)", sha256(source) == source_hash)
    core = find_core()
    if core is None:
        check("noyau trouvé pour le rejeu (cargo build)", False)
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
    registry.register("layer.add", layer_add.LayerAddGesture)
    analysis.register_all(registry)
    state["process"] = process
    state["client"] = client.CoreClient(process, protect(on_event), registry)
    state["client"].start(state["core"], project_file)


results = []
STEPS = [
    ("layer.add", lambda: {"path": "gares-4326.geojson"}),
    ("data.reproject", lambda: {"layer_id": results[0]["layer_id"], "target_epsg": 2154}),
]


def run_next():
    command, make = STEPS[len(results)]
    state["client"].run_command(command, make())


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
    reprojected = results[1]
    check("par le noyau : couche en EPSG:2154, 3 entités",
          reprojected.get("crs") == "EPSG:2154" and reprojected.get("feature_count") == len(POINTS), reprojected)
    rows = read_journal()
    check("le journal porte les deux étapes", [r["id"] for r in rows] == [s for s, _ in STEPS], [r["id"] for r in rows])
    state["journal"] = rows
    state["client"].stop()
    project.clear()
    check("le projet rouvre", project.read(project_file))
    check("le projet rouvert est vide", len(project.mapLayers()) == 0)
    new_client(on_second)


def on_second(kind, payload):
    if kind == client.READY:
        check("le rejeu part", state["client"].replay())
    elif kind == client.REPLAY_DONE:
        check("le rejeu rend identical: true", payload == {"steps": len(STEPS), "identical": True}, payload)
        replayed = os.path.join(cassini_dir, "replay", "2-data.reproject.gpkg")
        check("la couche reprojetée est rejouée sous replay/", os.path.isfile(replayed)
              and os.path.getsize(replayed) > 0, replayed)
        check("le journal est inchangé par le rejeu", read_journal() == state["journal"])
        finish()
    elif kind in (client.REPLAY_FAILED, client.FAILED, client.DIED):
        check("rejeu nominal", False, "%s %s" % (kind, payload))
        finish()


QTimer.singleShot(0, protect(run_case))
QTimer.singleShot(120000, QCoreApplication.quit)
QCoreApplication.exec()

if state["client"] is not None:
    state["client"].stop()
check("le fichier d'origine est toujours inchangé", sha256(source) == source_hash)
ok = len(checks) >= 15 and all(v for _, v in checks)
project.clear()
shutil.rmtree(work, ignore_errors=True)
print("RÉSULTAT :", "OK" if ok else "ÉCHEC")
sys.exit(0 if ok else 1)
