"""Sonde manuelle de la démonstration de bout en bout : une phrase, le vrai modèle, le vrai QGIS.

À lancer avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`), de
préférence par `outils-cassini/scripts/demo-11-14.ps1` :

    set CASSINI_CORE=<cassini.exe>
    set CASSINI_MODELS=<dossier qui ne contient que le modèle à montrer>
    "C:\\Program Files\\QGIS 3.44.15\\bin\\python-qgis-ltr.bat" qgis-plugin/tests/probe_demo_chaine.py [<projet.qgz>]

Sans argument, le projet est `../donnees-test-cassini/annecy/projet-test-annecy.qgz` (hors dépôt).
Il n'est jamais modifié : la sonde travaille sur une copie posée à côté (`<projet>-demo-<date>.qgz`,
chemins relatifs gardés), et le noyau écrit son journal et ses sorties dans `<copie>.cassini/`.

Une seule phrase (`PHRASE`) : conversion d'un point MGRS pris sur le MNT d'Annecy en WGS84,
reprojection vers le système du MNT, champ de vision cumulé depuis ce point, carte en PDF A3 avec
un carroyage UTM de 1 km. La sonde juge (`demo_chaine_verdict.py`) :

1. le plan proposé : les quatre familles de commandes, dans l'ordre, et un export qui demande l'A3 et
   le carroyage de 1 km ; sinon ECHEC, le plan obtenu est imprimé (rien n'est exécuté) ;
2. le plan exécuté, puis le PDF : lisible, une page, A3, avec une grille UTM de 1 km active sur la
   carte (relevée sur la mise en page au moment de l'export) ;
3. le rejeu : projet fermé et rouvert, nouveau noyau sans modèle, `identical` et autant d'étapes que
   dans le journal ;
4. le réseau, comme `probe_demo.py` : un relevé `netstat -ano` du noyau chaque seconde ; toute
   connexion à une adresse non locale rend ECHEC.

Elle imprime le plan, le résultat de chaque étape, le fichier PDF, puis `VERDICT : OK` (code 0) ou
`VERDICT : ECHEC — <raison>` (code 1). Facultatif : `CASSINI_PROBE_PATH`, dossiers ajoutés en tête
du `PATH` du noyau (DLL de CUDA d'un noyau construit avec `--features cassini-ai/cuda`).
Le PDF est à ouvrir et à regarder : un verdict ne prouve pas le rendu.
"""

import json
import os
import shutil
import subprocess
import sys
import time
import traceback

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "qgis-plugin", "cassini_qgis"))
sys.path.insert(0, HERE)
# `offscreen` ne charge aucune police : le texte du PDF deviendrait des cases noires.
os.environ.setdefault("QT_QPA_PLATFORM", "windows" if sys.platform == "win32" else "offscreen")

import demo_chaine_verdict as chain  # noqa: E402
import demo_verdict  # noqa: E402

# Point réel du MNT d'Annecy (mnt-74-25m.tif, EPSG:2154) : Mont Veyrier, altitude 797 m au MNT.
PHRASE = (
    "Convertis le point 32T KR 81280 88316 (MGRS) en WGS84, reprojette-le vers le système du MNT, "
    "calcule le champ de vision cumulé depuis ce point et sors la carte en PDF A3 avec un "
    "carroyage UTM de 1 km."
)
DEFAULT_PROJECT = os.path.normpath(
    os.path.join(ROOT, "..", "donnees-test-cassini", "annecy", "projet-test-annecy.qgz"))
GIVE_UP_MS = 30 * 60 * 1000
POLL_MS = 1000


def stop(message):
    print(demo_verdict.verdict_line(message), flush=True)
    sys.exit(1)


def core_path():
    for path in (os.environ.get("CASSINI_CORE"), os.path.join(ROOT, "target", "release", "cassini.exe")):
        if path and os.path.isfile(path):
            return path
    stop("noyau introuvable : poser CASSINI_CORE")


ORIGINAL = os.path.abspath(sys.argv[1]) if len(sys.argv) == 2 else DEFAULT_PROJECT
if len(sys.argv) > 2 or not os.path.isfile(ORIGINAL):
    stop("usage : probe_demo_chaine.py [<projet.qgz>] (projet introuvable : %s)" % ORIGINAL)
if not os.environ.get("CASSINI_MODELS"):
    stop("CASSINI_MODELS absent : poser le dossier du modèle à montrer")
CORE = core_path()
if os.environ.get("CASSINI_PROBE_PATH"):
    os.environ["PATH"] = os.environ["CASSINI_PROBE_PATH"] + os.pathsep + os.environ.get("PATH", "")

stem, ext = os.path.splitext(ORIGINAL)
PROJECT = "%s-demo-%s%s" % (stem, time.strftime("%Y%m%d-%H%M%S"), ext)
shutil.copyfile(ORIGINAL, PROJECT)
OUTPUT_DIR = os.path.splitext(PROJECT)[0] + ".cassini"
print("projet copié : %s" % PROJECT, flush=True)

from qgis.analysis import QgsNativeAlgorithms  # noqa: E402
from qgis.core import QgsApplication, QgsLayoutExporter, QgsLayoutItemMap, QgsProject  # noqa: E402
from qgis.PyQt.QtCore import QProcess, QTimer  # noqa: E402

app = QgsApplication([], False)
QgsApplication.initQgis()
from qgis.PyQt.QtGui import QFontDatabase  # noqa: E402

_no_font = demo_verdict.fonts_reason(len(QFontDatabase().families()))
if _no_font is not None:
    stop(_no_font)
QgsApplication.processingRegistry().addProvider(QgsNativeAlgorithms())

import client  # noqa: E402
import delegated  # noqa: E402

project = QgsProject.instance()
if not project.read(PROJECT):
    stop("projet illisible : %s" % PROJECT)

# Grilles posées sur les cartes, relevées au moment de l'export (la mise en page est jetée après).
grids = []
_export_to_pdf = QgsLayoutExporter.exportToPdf


def _spy_export(self, *args, **kwargs):
    result = _export_to_pdf(self, *args, **kwargs)
    for item in self.layout().items():
        if isinstance(item, QgsLayoutItemMap):
            for g in item.grids().asList():
                grids.append((g.crs().authid(), g.intervalX(), g.intervalY(), g.enabled()))
    return result


QgsLayoutExporter.exportToPdf = _spy_export

t0 = time.monotonic()
state = {"reason": None, "done": False, "calls": [], "steps": [], "phase": "plan", "net": [],
         "net_keys": set(), "pids": set(), "polls": 0}


def now():
    return "%.2f" % (time.monotonic() - t0)


class NoProbe:  # le geste de test du noyau n'est jamais dicté ici
    def start(self, params, done):
        done(result={})

    def cancel(self):
        pass


registry = delegated.build_registry(NoProbe)


def finish(reason):
    if not state["done"]:
        state["done"], state["reason"] = True, reason
        print("[%s] %s" % (now(), demo_verdict.verdict_line(reason)), flush=True)
        QTimer.singleShot(0, app.quit)


def poll_network():
    """Un relevé de `netstat -ano` pour le PID du noyau en cours (voir `probe_demo.py`)."""
    process_id = int(process.processId() or 0)
    if process_id:
        state["pids"].add(process_id)
    try:
        out = subprocess.run(["netstat", "-ano"], capture_output=True, timeout=20,
                             check=True).stdout.decode("ascii", errors="replace")
    except (OSError, subprocess.SubprocessError):
        return
    rows = demo_verdict.parse_netstat(out)
    if not rows:
        return
    if state["pids"]:
        state["polls"] += 1
    for row in rows:
        key = (row["proto"], row["local"], row["remote"], row["state"], row["pid"])
        if row["pid"] in state["pids"] and key not in state["net_keys"]:
            state["net_keys"].add(key)
            state["net"].append(row)


def guarded_poll():
    try:
        poll_network()
    except Exception:
        traceback.print_exc()


def network_verdict():
    guarded_poll()
    print("  réseau : %d relevé(s), PID surveillés %s, %d connexion(s) vue(s)" % (
        state["polls"], sorted(state["pids"]), len(state["net"])), flush=True)
    return demo_verdict.judge_network(state["net"], state["pids"], state["polls"])


def pdf_bytes(steps):
    file = demo_verdict.pdf_file(steps)
    if file is None:
        return None
    path = file if os.path.isabs(file) else os.path.join(OUTPUT_DIR, file)
    print("  PDF : %s" % path, flush=True)
    try:
        with open(path, "rb") as f:
            return f.read()
    except OSError:
        return None


def journal_steps():
    """Étapes du journal : le rejeu les refait toutes (celles que le noyau dicte comprises)."""
    with open(os.path.join(OUTPUT_DIR, "journal.jsonl"), encoding="utf-8") as f:
        rows = [json.loads(line) for line in f if line.strip()]
    return [{"step": i + 1, "command": row.get("id")} for i, row in enumerate(rows)]


def start_replay():
    """Ferme le projet, le rouvre, lance un nouveau noyau sans modèle (rejeu : aucune IA)."""
    global core, process
    core.stop()
    project.clear()
    if not project.read(PROJECT):
        finish("projet illisible à la réouverture : %s" % PROJECT)
        return
    state["phase"] = "replay"
    os.environ.pop("CASSINI_MODELS", None)
    process = QProcess()
    core = client.CoreClient(process, guarded, registry)
    core.start(CORE, PROJECT)


def on_replay_event(kind, payload):
    if kind == client.READY:
        print("[%s] nouveau noyau prêt ; rejeu du journal" % now(), flush=True)
        guarded_poll()
        if not core.replay():
            finish("rejeu : l'extension n'a pas pu le lancer")
    elif kind in (client.REPLAY_DONE, client.REPLAY_FAILED):
        print("[%s] %s %s" % (now(), kind, json.dumps(payload, ensure_ascii=False)), flush=True)
        reason = demo_verdict.judge_replay(kind, payload, state["steps"])
        finish(reason if reason is not None else network_verdict())
    elif kind in (client.FAILED, client.DIED):
        finish("rejeu : noyau %s" % payload)


def on_event(kind, payload):
    if state["phase"] == "replay":
        on_replay_event(kind, payload)
        return
    if kind == client.READY:
        print("[%s] noyau prêt ; phrase : %s" % (now(), PHRASE), flush=True)
        guarded_poll()
        core.propose(PHRASE)
    elif kind == client.PLAN_PROPOSED:
        plan = payload["plan"]
        print("plan :", json.dumps(plan, ensure_ascii=False, indent=1), flush=True)
        calls = plan.get("calls")
        state["calls"] = calls if isinstance(calls, list) else []
        reason = chain.first_reason(chain.judge_plan(state["calls"]), chain.judge_requested(state["calls"]))
        if reason is not None:
            finish(reason)  # le plan obtenu est imprimé ci-dessus ; rien n'est exécuté
            return
        core.execute_plan(payload["plan_id"])
    elif kind == client.PLAN_DONE:
        steps = payload["steps"]
        for step in steps:
            print("  étape %s %s : %s" % (step.get("step"), step.get("command"),
                                          json.dumps(step.get("result"), ensure_ascii=False)))
        reason = chain.first_reason(
            chain.judge_steps(state["calls"], steps),
            chain.judge_pdf(pdf_bytes(steps), grids))
        print("  grilles posées : %s" % grids, flush=True)
        if reason is not None:
            finish(reason)
            return
        state["steps"] = journal_steps()
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
poll_timer = QTimer()
poll_timer.timeout.connect(guarded_poll)
poll_timer.start(POLL_MS)
QTimer.singleShot(GIVE_UP_MS, lambda: finish("délai dépassé"))
app.exec_() if hasattr(app, "exec_") else app.exec()
core.stop()
QgsApplication.exitQgis()
sys.exit(0 if state["done"] and state["reason"] is None else 1)
