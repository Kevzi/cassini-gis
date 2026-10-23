"""Sonde manuelle sous le vrai QGIS, le vrai noyau et le vrai modèle : la phrase de démo.

À lancer à la main avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`) :

    set CASSINI_CORE=<cassini.exe>
    set CASSINI_MODELS=<dossier qui ne contient que le modèle à montrer>
    "C:\\Program Files\\QGIS 3.44.15\\bin\\python-qgis-ltr.bat" qgis-plugin/tests/probe_demo.py <projet.qgz>

Le noyau est cherché dans `CASSINI_CORE`, sinon `target/release/cassini.exe`. `CASSINI_MODELS` est
transmis tel quel au noyau, qui y prend la première entrée de `model.list` ; la sonde refuse de
partir sans lui, pour ne jamais prendre en silence le dossier des modèles du poste. Facultatif :
`CASSINI_PROBE_PATH`, dossiers ajoutés en tête du `PATH` du noyau (par exemple les DLL de CUDA
d'un noyau construit avec `--features cassini-ai/cuda`, que le Python de QGIS ne voit pas).

La sonde ouvre le projet, lance `cassini serve` par `client.py` avec les vrais gestes de
l'extension, propose la phrase de démo exacte de `CLAUDE.md`, valide le plan, l'exécute et
imprime : la durée de `project.inventory`, la durée de la composition, le plan, le résultat de
chaque étape et le chemin du PDF (qui doit commencer par `%PDF`). Elle ne modifie ni le projet ni
les fichiers d'origine ; les sorties vont dans le dossier `<projet>.cassini/`, à côté du projet.
Aucune donnée n'est versionnée : le projet vient de l'utilisateur (voir CLAUDE.md §5).
"""

import json
import os
import re
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "qgis-plugin", "cassini_qgis"))
os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")

PHRASE = (
    "Trouve les parcelles constructibles du PLU à moins de 500 m d'une gare à "
    "Villeneuve-le-Roi et sors-moi la carte en PDF."
)
GIVE_UP_MS = 30 * 60 * 1000


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


if len(sys.argv) != 2 or not os.path.isfile(sys.argv[1]):
    sys.exit("usage : probe_demo.py <projet.qgz>")
if not os.environ.get("CASSINI_MODELS"):
    sys.exit("CASSINI_MODELS absent : poser le dossier du modèle à montrer")
check_phrase()
PROJECT = os.path.abspath(sys.argv[1])
CORE = core_path()
if os.environ.get("CASSINI_PROBE_PATH"):
    os.environ["PATH"] = os.environ["CASSINI_PROBE_PATH"] + os.pathsep + os.environ.get("PATH", "")

from qgis.analysis import QgsNativeAlgorithms  # noqa: E402
from qgis.core import QgsApplication, QgsProject  # noqa: E402
from qgis.PyQt.QtCore import QProcess, QTimer  # noqa: E402

app = QgsApplication([], False)
QgsApplication.initQgis()
QgsApplication.processingRegistry().addProvider(QgsNativeAlgorithms())

import analysis  # noqa: E402
import client  # noqa: E402
import gestures  # noqa: E402
import inventory  # noqa: E402
import layer_add  # noqa: E402
import layout_export  # noqa: E402

if not QgsProject.instance().read(PROJECT):
    sys.exit("projet illisible : %s" % PROJECT)

t0 = time.monotonic()
state = {"verdict": None, "marks": {}}


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


class TimedRegistry(gestures.GestureRegistry):
    def create(self, command_id):
        inner = super().create(command_id)
        return None if inner is None else Timed(command_id, inner)


registry = TimedRegistry()
registry.register("layer.add", layer_add.LayerAddGesture)
registry.register("project.inventory", inventory.ProjectInventoryGesture)
registry.register("layer.describe", inventory.LayerDescribeGesture)
analysis.register_all(registry)
registry.register("layout.export_pdf", layout_export.LayoutExportGesture)


def finish(verdict):
    if state["verdict"] is None:
        state["verdict"] = verdict
        print("[%s] FIN : %s" % (now(), verdict), flush=True)
        QTimer.singleShot(0, app.quit)


def pdf_path(steps):
    """Chemin du PDF de l'étape `layout.export_pdf`, dans le dossier de sortie du projet."""
    folder = os.path.splitext(PROJECT)[0] + ".cassini"
    for step in steps:
        if step.get("command") == "layout.export_pdf":
            return os.path.join(folder, step["result"]["file"])
    return None


def on_event(kind, payload):
    marks = state["marks"]
    if kind == client.READY:
        print("[%s] noyau prêt ; phrase : %s" % (now(), PHRASE), flush=True)
        marks["propose"] = time.monotonic()
        core.propose(PHRASE)
    elif kind == client.PLAN_PROPOSED:
        print("[%s] composition : %.2f s" % (now(), time.monotonic() - marks["propose"]), flush=True)
        print("plan :", json.dumps(payload["plan"], ensure_ascii=False, indent=1), flush=True)
        marks["execute"] = time.monotonic()
        core.execute_plan(payload["plan_id"])
    elif kind == client.PLAN_DONE:
        print("[%s] exécution : %.2f s" % (now(), time.monotonic() - marks["execute"]), flush=True)
        for step in payload["steps"]:
            print("  étape %s %s : %s" % (step["step"], step["command"], json.dumps(step["result"], ensure_ascii=False)))
        pdf = pdf_path(payload["steps"])
        if pdf is None:
            print("aucun PDF dans le plan")
            finish("plan exécuté, sans PDF")
            return
        with open(pdf, "rb") as f:
            ok = f.read(5) == b"%PDF-"
        print("PDF :", pdf, "(lisible)" if ok else "(PAS un PDF)")
        finish("réussite" if ok else "PDF illisible")
    elif kind in (client.PLAN_QUESTION, client.PLAN_FAILED, client.PLAN_EXEC_FAILED,
                  client.PLAN_CANCELLED, client.FAILED, client.DIED):
        print("[%s] %s %s" % (now(), kind, json.dumps(payload, ensure_ascii=False)), flush=True)
        finish("échec : %s" % kind)


process = QProcess()
core = client.CoreClient(process, on_event, registry)
core.start(CORE, PROJECT)
QTimer.singleShot(GIVE_UP_MS, lambda: finish("délai dépassé"))
app.exec_() if hasattr(app, "exec_") else app.exec()
core.stop()
QgsApplication.exitQgis()
sys.exit(0 if state["verdict"] == "réussite" else 1)
