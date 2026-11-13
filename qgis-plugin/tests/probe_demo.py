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
commande, pour voir la sonde rendre ECHEC ; `CASSINI_DEMO_TAMPER=1`, qui change le `result` d'une
étape d'analyse du journal avant le rejeu (refusé avec un projet donné en argument) ;
`CASSINI_DEMO_DROP_ITEM=<élément>` (title, legend, scalebar, northarrow, sources), qui retire
cet élément de la liste relevée sur la mise en page, pour voir la sonde rendre ECHEC.

Après un plan jugé juste, la sonde juge la mise en page (titre, légende, échelle, flèche du nord
et sources, d'après ce que le geste d'export déclare avoir posé, `items_placed`, et le résultat
de l'export ; PDF non vide), puis ferme le projet, le rouvre, lance un nouveau noyau sans
`CASSINI_MODELS` et rejoue le journal : `identical` et autant d'étapes que d'exécutées.

La sonde propose la phrase de démo exacte de `CLAUDE.md`, valide le plan, l'exécute, puis juge
(`demo_verdict.py`) : la couche retenue (dernière analyse avant l'export) n'est pas vide, porte un
champ propre à la couche des parcelles, chacune de ses entités a son point intérieur en zone
`typezone = U` et est à 500 m au plus de la gare la plus proche, et le PDF commence par `%PDF-`.
Les gestes sont ceux de l'extension (`delegated.build_registry`), style compris. Elle imprime les durées, le plan,
le résultat de chaque étape, puis `VERDICT : OK` (code 0) ou `VERDICT : ECHEC — <raison>` (code 1).
Avec un argument, elle ouvre le projet donné, par exemple celui d'Annecy (territoire de la démo,
décision du propriétaire du 2026-11-10 ; `donnees-test-cassini/annecy/projet-test-annecy.qgz`) dont
les gares sont en EPSG:4326 : le tampon est calculé dans le système du projet (entrée 9.16). Elle
ne modifie ni le projet ni les fichiers d'origine. Aucune donnée réelle n'est versionnée : le projet
vient de l'utilisateur (voir CLAUDE.md §5).
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
# `offscreen` ne charge aucune police : le texte du PDF deviendrait des cases noires. Sous Windows,
# la plateforme normale en a (essai du 2026-11-13 : 154 familles contre 0).
os.environ.setdefault("QT_QPA_PLATFORM", "windows" if sys.platform == "win32" else "offscreen")

import demo_verdict  # noqa: E402

PHRASE = (
    "Trouve les parcelles constructibles du PLU à moins de 500 m d'une gare à "
    "Annecy et sors-moi la carte en PDF."
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
TAMPER = bool(os.environ.get("CASSINI_DEMO_TAMPER"))
DROP_ITEM = os.environ.get("CASSINI_DEMO_DROP_ITEM") or None
_refused = demo_verdict.tamper_refused_reason(TAMPER, len(sys.argv) == 2)
_refused = _refused or demo_verdict.drop_item_refused_reason(DROP_ITEM)
if _refused is not None:
    print(demo_verdict.verdict_line(_refused), flush=True)
    sys.exit(1)
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
from qgis.PyQt.QtGui import QFontDatabase  # noqa: E402

_no_font = demo_verdict.fonts_reason(len(QFontDatabase().families()))
if _no_font is not None:
    print(demo_verdict.verdict_line(_no_font), flush=True)
    sys.exit(1)
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
state = {"reason": None, "done": False, "marks": {}, "calls": [], "steps": [], "phase": "plan",
         "export": None}


def now():
    return "%.2f" % (time.monotonic() - t0)


class Timed:
    """Geste enveloppé : chronomètre start → done, sans rien changer à sa réponse."""

    def __init__(self, command, inner):
        self.command, self.inner = command, inner
        if command == demo_verdict.EXPORT:
            state["export"] = inner  # sa liste `items_placed` est relevée après l'export

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


def pdf_facts(steps):
    """Cinq premiers octets et taille du PDF d'export, `(None, 0)` s'il n'est pas lisible."""
    file = demo_verdict.pdf_file(steps)
    if file is None:
        return None, 0
    path = file if os.path.isabs(file) else os.path.join(OUTPUT_DIR, file)
    try:
        with open(path, "rb") as f:
            head = f.read(5)
        return head, os.path.getsize(path)
    except OSError:
        return None, 0


def pdf_head(steps):
    return pdf_facts(steps)[0]


def layout_verdict(steps):
    """Éléments posés d'après le geste d'export, `CASSINI_DEMO_DROP_ITEM` appliqué."""
    gesture = state["export"]
    items = list(getattr(gesture, "items_placed", []) or [])
    if DROP_ITEM:
        items = [item for item in items if item != DROP_ITEM]
    print("  mise en page : éléments posés %s" % items, flush=True)
    result = next((s.get("result") for s in steps
                   if isinstance(s, dict) and s.get("command") == demo_verdict.EXPORT), None)
    result = result if isinstance(result, dict) else {}
    head, size = pdf_facts(steps)
    facts = {"sources": result.get("sources"), "page_count": result.get("page_count"),
             "pdf_head": head, "pdf_size": size}
    return demo_verdict.judge_layout(items, facts)


def tamper_journal():
    """Change le `result` de la première étape d'analyse du journal, avant le rejeu."""
    path = os.path.join(OUTPUT_DIR, "journal.jsonl")
    with open(path, encoding="utf-8") as f:
        rows = [json.loads(line) for line in f if line.strip()]
    for index, row in enumerate(rows):
        if str(row.get("id", "")).startswith(demo_verdict.ANSWER_PREFIX) and isinstance(row.get("result"), dict):
            result = row["result"]
            count = result.get("feature_count")
            result["feature_count"] = count + 1 if isinstance(count, int) else 999999
            break
    else:
        raise RuntimeError("aucune étape d'analyse à altérer dans le journal")
    with open(path, "w", encoding="utf-8") as f:
        for row in rows:
            f.write(json.dumps(row, ensure_ascii=False) + "\n")
    print("  journal altéré : étape %d %s" % (index + 1, rows[index]["id"]), flush=True)


def start_replay():
    """Ferme le projet, le rouvre, lance un nouveau noyau sans modèle (rejeu : aucune IA)."""
    global core, process
    core.stop()
    project.clear()
    if not project.read(PROJECT):
        finish("projet illisible à la réouverture : %s" % PROJECT)
        return
    if TAMPER:
        tamper_journal()
    state["phase"] = "replay"
    os.environ.pop("CASSINI_MODELS", None)
    process = QProcess()
    core = client.CoreClient(process, guarded, registry)
    core.start(CORE, PROJECT)


def on_replay_event(kind, payload):
    if kind == client.READY:
        print("[%s] nouveau noyau prêt ; rejeu du journal" % now(), flush=True)
        if not core.replay():
            finish("rejeu : l'extension n'a pas pu le lancer")
    elif kind in (client.REPLAY_DONE, client.REPLAY_FAILED):
        print("[%s] %s %s" % (now(), kind, json.dumps(payload, ensure_ascii=False)), flush=True)
        finish(demo_verdict.judge_replay(kind, payload, state["steps"]))
    elif kind in (client.FAILED, client.DIED):
        finish("rejeu : noyau %s" % payload)


def on_event(kind, payload):
    if state["phase"] == "replay":
        on_replay_event(kind, payload)
        return
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
        reason = demo_verdict.judge_done(steps, facts, injected=FAIL_STEP)
        if reason is None:
            reason = layout_verdict(steps)
        if reason is not None:
            finish(reason)
        else:
            state["steps"] = steps
            QTimer.singleShot(0, start_replay_guarded)
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


def start_replay_guarded():
    try:
        start_replay()
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
