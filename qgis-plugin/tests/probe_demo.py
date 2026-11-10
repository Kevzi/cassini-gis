"""Sonde manuelle sous le vrai QGIS, le vrai noyau et le vrai modèle : la phrase de démo, jugée.

À lancer à la main avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`) :

    set CASSINI_CORE=<cassini.exe>
    set CASSINI_MODELS=<dossier qui ne contient que le modèle à montrer>
    "C:\\Program Files\\QGIS 3.44.15\\bin\\python-qgis-ltr.bat" qgis-plugin/tests/probe_demo.py [<projet.qgz>]

Sans argument, la sonde assemble un projet de démo dans un dossier temporaire, à partir des
fixtures synthétiques de `fixtures/demo/` (zonage du PLU en zones U et N, 16 parcelles, 2 gares,
en EPSG:2154), copiées : les fixtures ne sont jamais touchées. Avec un argument, elle ouvre ce
projet, sans le modifier ; les sorties vont dans `<projet>.cassini/`, à côté du projet.

Le noyau est cherché dans `CASSINI_CORE`, sinon `target/release/cassini.exe`. `CASSINI_MODELS` est
transmis tel quel au noyau, qui y prend la première entrée de `model.list` ; la sonde refuse de
partir sans lui, pour ne jamais prendre en silence le dossier des modèles du poste. Facultatifs :
`CASSINI_PROBE_PATH`, dossiers ajoutés en tête du `PATH` du noyau (par exemple les DLL de CUDA
d'un noyau construit avec `--features cassini-ai/cuda`, que le Python de QGIS ne voit pas) ;
`CASSINI_DEMO_FAIL_STEP=<id de commande>`, qui fait répondre une erreur au geste de cette
commande, pour voir la sonde rendre ECHEC.

La sonde propose la phrase de démo exacte de `CLAUDE.md`, valide le plan, l'exécute, puis juge
(`demo_verdict.py`) : la couche retenue (dernière analyse avant l'export) n'est pas vide, porte un
champ propre à la couche des parcelles, chacune de ses entités a son point intérieur en zone
`typezone = U` et est à 500 m au plus de la gare la plus proche, et le PDF commence par `%PDF-`.
Les gestes sont ceux de l'extension (`delegated.build_registry`), style compris. Elle imprime les durées, le plan,
le résultat de chaque étape, puis `VERDICT : OK` (code 0) ou `VERDICT : ECHEC — <raison>` (code 1).
"""

import json
import os
import re
import shutil
import sys
import tempfile
import time
import traceback

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "qgis-plugin", "cassini_qgis"))
sys.path.insert(0, HERE)
os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")

import demo_verdict  # noqa: E402

PHRASE = (
    "Trouve les parcelles constructibles du PLU à moins de 500 m d'une gare à "
    "Villeneuve-le-Roi et sors-moi la carte en PDF."
)
GIVE_UP_MS = 30 * 60 * 1000
DEMO_FIXTURES = os.path.join(ROOT, "fixtures", "demo")
# Fichier, nom de couche : les noms de l'inventaire de l'épreuve du modèle local.
DEMO_LAYERS = (
    ("plu-zonage-2154.geojson", "Zonage du PLU"),
    ("parcelles-2154.geojson", "Parcelles cadastrales"),
    ("gares-2154.geojson", "Gares"),
)
INJECTED = "qgis.demo_injected"


def check_phrase():
    """La phrase est celle de CLAUDE.md, à l'espace près (le fichier la coupe en lignes)."""
    with open(os.path.join(ROOT, "CLAUDE.md"), encoding="utf-8") as f:
        text = re.sub(r"\s+", " ", f.read())
    if PHRASE not in text:
        sys.exit("la phrase de démo n'est plus celle de CLAUDE.md")


def core_path():
    for path in (os.environ.get("CASSINI_CORE"), os.path.join(ROOT, "target", "release", "cassini.exe")):
        if path and os.path.isfile(path):
            return path
    sys.exit("noyau introuvable : poser CASSINI_CORE")


if len(sys.argv) > 2 or (len(sys.argv) == 2 and not os.path.isfile(sys.argv[1])):
    sys.exit("usage : probe_demo.py [<projet.qgz>]")
if not os.environ.get("CASSINI_MODELS"):
    sys.exit("CASSINI_MODELS absent : poser le dossier du modèle à montrer")
check_phrase()
CORE = core_path()
FAIL_STEP = os.environ.get("CASSINI_DEMO_FAIL_STEP") or None
if os.environ.get("CASSINI_PROBE_PATH"):
    os.environ["PATH"] = os.environ["CASSINI_PROBE_PATH"] + os.pathsep + os.environ.get("PATH", "")

from qgis.analysis import QgsNativeAlgorithms  # noqa: E402
from qgis.core import (  # noqa: E402
    Qgis,
    QgsApplication,
    QgsCoordinateReferenceSystem,
    QgsCoordinateTransform,
    QgsProject,
    QgsVectorLayer,
)
from qgis.PyQt.QtCore import QProcess, QTimer  # noqa: E402

app = QgsApplication([], False)
QgsApplication.initQgis()
QgsApplication.processingRegistry().addProvider(QgsNativeAlgorithms())

import client  # noqa: E402
import delegated  # noqa: E402
import gestures  # noqa: E402

project = QgsProject.instance()


def assemble_demo_project():
    """Projet de démo dans un dossier temporaire, sur des copies des fixtures."""
    work = tempfile.mkdtemp(prefix="cassini-demo-")
    project.setCrs(QgsCoordinateReferenceSystem("EPSG:2154"))
    for file, name in DEMO_LAYERS:
        copy = os.path.join(work, file)
        shutil.copyfile(os.path.join(DEMO_FIXTURES, file), copy)
        layer = QgsVectorLayer(copy, name, "ogr")
        if not layer.isValid():
            sys.exit("fixture illisible : %s" % copy)
        project.addMapLayer(layer)
    path = os.path.join(work, "demo.qgz").replace("\\", "/")
    if not project.write(path):
        sys.exit("projet de démo non enregistré : %s" % path)
    print("projet de démo assemblé : %s" % path, flush=True)
    return path


if len(sys.argv) == 2:
    PROJECT = os.path.abspath(sys.argv[1])
    if not project.read(PROJECT):
        sys.exit("projet illisible : %s" % PROJECT)
else:
    PROJECT = assemble_demo_project()
OUTPUT_DIR = os.path.splitext(PROJECT)[0] + ".cassini"


def reference_layers():
    """Couches de référence du contrôle, relevées avant l'exécution (les sorties du plan
    reprennent les champs du PLU et ne doivent pas être prises pour lui)."""
    zoning = stations = parcels = None
    for layer in project.mapLayers().values():
        if not isinstance(layer, QgsVectorLayer):
            continue
        name = layer.name().lower()
        kind = layer.geometryType()
        if zoning is None and layer.fields().indexFromName(demo_verdict.ZONE_FIELD) >= 0:
            zoning = layer
        elif parcels is None and kind == Qgis.GeometryType.Polygon and "parcelle" in name:
            parcels = layer
        if stations is None and kind == Qgis.GeometryType.Point and "gare" in name:
            stations = layer
    return zoning, stations, parcels


ZONING, STATIONS, PARCELS = reference_layers()
missing = demo_verdict.missing_layers_reason(
    ZONING is not None, STATIONS is not None, PARCELS is not None)
if missing is not None:
    print(demo_verdict.verdict_line(missing), flush=True)
    QgsApplication.exitQgis()
    sys.exit(1)

t0 = time.monotonic()
state = {"reason": None, "done": False, "marks": {}, "calls": []}


def now():
    return "%.2f" % (time.monotonic() - t0)


class Timed:
    """Geste enveloppé : chronomètre start → done, sans rien changer à sa réponse."""

    def __init__(self, command, inner):
        self.command, self.inner = command, inner

    def start(self, params, done):
        started = time.monotonic()

        def finish(result=None, error=None):
            seconds = time.monotonic() - started
            failure = "" if error is None else " ; erreur %s" % getattr(error, "code", error)
            print("[%s] geste %s : %.2f s%s" % (now(), self.command, seconds, failure), flush=True)
            done(result=result, error=error)

        self.inner.start(params, finish)

    def cancel(self):
        self.inner.cancel()


class Failing:
    """Geste qui répond une erreur sans rien faire (`CASSINI_DEMO_FAIL_STEP`)."""

    def __init__(self, command):
        self.command = command

    def start(self, params, done):
        error = gestures.GestureError(INJECTED, "Échec injecté par CASSINI_DEMO_FAIL_STEP.")
        QTimer.singleShot(0, lambda: done(error=error))

    def cancel(self):
        pass


class NoProbe:  # le geste de test du noyau n'est jamais dicté ici
    def start(self, params, done):
        done(result={})

    def cancel(self):
        pass


registry = delegated.build_registry(NoProbe)
plain_create = registry.create


def timed_create(command_id):
    """Les gestes de l'extension, chronométrés ; celui de `CASSINI_DEMO_FAIL_STEP` échoue."""
    inner = plain_create(command_id)
    if inner is not None and command_id == FAIL_STEP:
        inner = Failing(command_id)
    return None if inner is None else Timed(command_id, inner)


registry.create = timed_create


def finish(reason):
    if not state["done"]:
        state["done"], state["reason"] = True, reason
        print("[%s] %s" % (now(), demo_verdict.verdict_line(reason)), flush=True)
        QTimer.singleShot(0, app.quit)


def metric_crs():
    crs = project.crs()
    return QgsCoordinateReferenceSystem("EPSG:2154") if not crs.isValid() or crs.isGeographic() else crs


def geometries(layer, crs):
    """Géométries de la couche dans `crs`."""
    transform = QgsCoordinateTransform(layer.crs(), crs, project)
    out = []
    for feature in layer.getFeatures():
        geometry = feature.geometry()
        if geometry.isNull() or geometry.isEmpty():
            continue
        geometry.transform(transform)
        out.append((feature, geometry))
    return out


def feature_name(feature):
    names = feature.fields().names()
    for field in ("id_parcelle", "nom"):
        if field in names and feature[field] not in (None, ""):
            return "%s (id %d)" % (feature[field], feature.id())
    return "id %d" % feature.id()


def retained_features(layer_id):
    """Zone `typezone` au point intérieur et distance à la gare la plus proche, par entité."""
    layer = project.mapLayer(layer_id)
    if not isinstance(layer, QgsVectorLayer):
        return None
    crs = metric_crs()
    zones = [(f[demo_verdict.ZONE_FIELD], g) for f, g in geometries(ZONING, crs)]
    stations = [g for _f, g in geometries(STATIONS, crs)]
    facts = []
    for feature, geometry in geometries(layer, crs):
        inside = geometry.pointOnSurface()
        zone = next((value for value, area in zones if area.contains(inside)), None)
        distance = min((geometry.distance(s) for s in stations), default=None)
        facts.append({"name": feature_name(feature), "zone": zone, "distance": distance})
    return facts


def parcel_fields(layer_id):
    """Vrai si la couche retenue porte un champ des parcelles absent du zonage et des gares."""
    layer = project.mapLayer(layer_id)
    if not isinstance(layer, QgsVectorLayer):
        return False
    others = set(ZONING.fields().names()) | set(STATIONS.fields().names()) | {"fid"}
    own = set(PARCELS.fields().names()) - others
    return bool(own & set(layer.fields().names()))


def pdf_head(steps):
    file = demo_verdict.pdf_file(steps)
    if file is None:
        return None
    path = file if os.path.isabs(file) else os.path.join(OUTPUT_DIR, file)
    try:
        with open(path, "rb") as f:
            return f.read(5)
    except OSError:
        return None


def on_event(kind, payload):
    marks = state["marks"]
    if kind == client.READY:
        print("[%s] noyau prêt ; phrase : %s" % (now(), PHRASE), flush=True)
        if FAIL_STEP:
            print("échec injecté sur : %s" % FAIL_STEP, flush=True)
        marks["propose"] = time.monotonic()
        core.propose(PHRASE)
    elif kind == client.PLAN_PROPOSED:
        print("[%s] composition : %.2f s" % (now(), time.monotonic() - marks["propose"]), flush=True)
        print("plan :", json.dumps(payload["plan"], ensure_ascii=False, indent=1), flush=True)
        calls = payload["plan"].get("calls")
        state["calls"] = calls if isinstance(calls, list) else []
        marks["execute"] = time.monotonic()
        core.execute_plan(payload["plan_id"])
    elif kind == client.PLAN_DONE:
        print("[%s] exécution : %.2f s" % (now(), time.monotonic() - marks["execute"]), flush=True)
        steps = payload["steps"]
        for step in steps:
            print("  étape %s %s : %s" % (step.get("step"), step.get("command"),
                                          json.dumps(step.get("result"), ensure_ascii=False)))
        layer_id = demo_verdict.retained_layer_id(steps)
        features = retained_features(layer_id) if layer_id else None
        for fact in features or []:
            distance = fact["distance"]
            print("  retenue %s : zone %s, %s m de la gare la plus proche" % (
                fact["name"], fact["zone"], "?" if distance is None else "%.1f" % distance))
        facts = {
            "features": features,
            "parcel_fields": parcel_fields(layer_id) if layer_id else False,
            "pdf_head": pdf_head(steps),
        }
        finish(demo_verdict.judge_done(steps, facts, injected=FAIL_STEP))
    else:
        reason = demo_verdict.failure_reason(kind, payload, state["calls"])
        if reason is not None:
            print("[%s] %s %s" % (now(), kind, json.dumps(payload, ensure_ascii=False)), flush=True)
            finish(reason)


def guarded(kind, payload):
    """Une exception dans un rappel de Qt abattrait QGIS sans message : on la montre."""
    try:
        on_event(kind, payload)
    except Exception as error:
        traceback.print_exc()
        finish("exception dans la sonde : %s" % error)


process = QProcess()
core = client.CoreClient(process, guarded, registry)
core.start(CORE, PROJECT)
QTimer.singleShot(GIVE_UP_MS, lambda: finish("délai dépassé"))
app.exec_() if hasattr(app, "exec_") else app.exec()
core.stop()
QgsApplication.exitQgis()
sys.exit(0 if state["done"] and state["reason"] is None else 1)
