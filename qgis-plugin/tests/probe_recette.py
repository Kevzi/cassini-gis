"""Sonde manuelle : recette QGIS en phrases enchaînées, jugées contre leur plan attendu (5.45).

À lancer à la main avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`) :

    set CASSINI_CORE=<cassini.exe>
    set CASSINI_MODELS=<dossier qui ne contient que le modèle à mesurer>
    set QT_QPA_PLATFORM=windows
    "C:\\Program Files\\QGIS 3.44.15\\bin\\python-qgis-ltr.bat" qgis-plugin/tests/probe_recette.py ^
        <projet.qgz> qgis-plugin/tests/recette/annecy.json

Paramétrable : le projet et le fichier de phrases sont des arguments (le fichier d'Annecy, ou celui
d'un autre projet qui porte ses propres noms de couches, comme le projet de 60 couches de 5.50).
Le fichier est lu par `recette_verdict.py` (forme et règle de conformité y sont décrites) ; une
couche `@nom` absente du projet arrête la sonde avant le noyau (code 2).

La sonde ne touche jamais au projet donné : elle en pose une copie à côté de lui
(`<projet>-recette-<date>.qgz`, à côté pour garder ses chemins relatifs), et le noyau écrit
ses sorties et son journal dans `<copie>.cassini/`. Les phrases s'enchaînent dans une seule
session, comme un utilisateur : un plan conforme est exécuté, un plan non conforme est rejeté
(`plan.reject`), comme l'utilisateur le refuserait. Puis le journal est rejoué.

Le noyau est cherché dans `CASSINI_CORE`, sinon `target/release/cassini.exe`. `CASSINI_MODELS`
est transmis tel quel au noyau, qui prend la première entrée de la liste ; la sonde refuse de
partir sans lui, pour ne jamais prendre en silence le dossier des modèles du poste. Facultatifs :
`CASSINI_PROBE_PATH`, dossiers ajoutés en tête du `PATH` du noyau (DLL de CUDA) ;
`CASSINI_RECETTE_MODEL`, nom du modèle à demander (sinon le premier de la liste).

Sortie : par phrase, la durée de composition, le plan, `JUSTE` ou `RATE — <raison>` (avec le plan
attendu) ; puis `SCORE : N/total (barre B) : …`, la liste des ratés et le verdict du rejeu.
Code 0 si la barre est tenue et le rejeu identique, 1 sinon, 2 si la sonde n'a pas pu partir.
"""

import json
import os
import shutil
import sys
import time
import traceback

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "qgis-plugin", "cassini_qgis"))
sys.path.insert(0, HERE)
os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")

import recette_verdict as rv  # noqa: E402

GIVE_UP_MS = 90 * 60 * 1000


def stop(message):
    print("SONDE NON PARTIE : %s" % message, flush=True)
    sys.exit(2)


def core_path():
    for path in (os.environ.get("CASSINI_CORE"), os.path.join(ROOT, "target", "release", "cassini.exe")):
        if path and os.path.isfile(path):
            return path
    stop("noyau introuvable : poser CASSINI_CORE")


if len(sys.argv) != 3 or not os.path.isfile(sys.argv[1]) or not os.path.isfile(sys.argv[2]):
    stop("usage : probe_recette.py <projet.qgz> <phrases.json>")
if not os.environ.get("CASSINI_MODELS"):
    stop("CASSINI_MODELS absent : poser le dossier du modèle à mesurer")
CORE = core_path()
MODEL = os.environ.get("CASSINI_RECETTE_MODEL") or None
if os.environ.get("CASSINI_PROBE_PATH"):
    os.environ["PATH"] = os.environ["CASSINI_PROBE_PATH"] + os.pathsep + os.environ.get("PATH", "")

ORIGINAL = os.path.abspath(sys.argv[1])
stem, ext = os.path.splitext(ORIGINAL)
PROJECT = "%s-recette-%s%s" % (stem, time.strftime("%Y%m%d-%H%M%S"), ext)
shutil.copyfile(ORIGINAL, PROJECT)
with open(sys.argv[2], encoding="utf-8") as f:
    PHRASES_TEXT = f.read()

from qgis.analysis import QgsNativeAlgorithms  # noqa: E402
from qgis.core import QgsApplication, QgsProject  # noqa: E402
from qgis.PyQt.QtCore import QProcess, QTimer  # noqa: E402

app = QgsApplication([], False)
QgsApplication.initQgis()
QgsApplication.processingRegistry().addProvider(QgsNativeAlgorithms())

import client  # noqa: E402
import delegated  # noqa: E402

project = QgsProject.instance()
if not project.read(PROJECT):
    stop("projet illisible : %s" % PROJECT)
try:
    PHRASES = rv.load(PHRASES_TEXT, rv.layer_index(
        (layer_id, layer.name()) for layer_id, layer in project.mapLayers().items()))
except rv.RecetteError as error:
    stop(str(error))

print("projet : %s (copie de %s)" % (PROJECT, ORIGINAL), flush=True)
print("phrases : %s (%d) ; noyau : %s ; modèles : %s" % (
    sys.argv[2], len(PHRASES), CORE, os.environ["CASSINI_MODELS"]), flush=True)

t0 = time.monotonic()
state = {"i": 0, "phase": None, "since": 0.0, "right": 0, "missed": [], "done": False,
         "replay": None}


def now():
    return "%.1f" % (time.monotonic() - t0)


def current():
    return PHRASES[state["i"] - 1]


def dumps(value):
    return json.dumps(value, ensure_ascii=False)


def verdict(reason):
    """Consigne le verdict de la phrase en cours, puis passe à la suivante."""
    phrase = current()
    if reason is rv.RIGHT:
        state["right"] += 1
        print("  JUSTE", flush=True)
    else:
        state["missed"].append((phrase["id"], reason))
        print("  RATE — %s" % reason, flush=True)
        print("  attendu : %s" % ("une question" if rv.wants_question(phrase) else dumps(phrase["plan"])),
              flush=True)
    state["phase"] = None
    QTimer.singleShot(0, next_phrase)


def next_phrase():
    if state["i"] >= len(PHRASES):
        total = len(PHRASES)
        print("\n%s" % rv.score_line(state["right"], total), flush=True)
        print("ratés : %s" % (", ".join("%s (%s)" % m for m in state["missed"]) or "aucun"), flush=True)
        print("\n[%s] rejeu du journal" % now(), flush=True)
        state["phase"] = "replay"
        if not core.replay():
            finish("rejeu non parti")
        return
    state["i"] += 1
    phrase = current()
    print("\n[%s] == %s : %s" % (now(), phrase["id"], phrase["text"]), flush=True)
    state["phase"], state["since"] = "propose", time.monotonic()
    if not core.propose(phrase["text"], MODEL):
        finish("proposition non partie")


def finish(reason):
    if not state["done"]:
        state["done"] = True
        state["replay"] = state["replay"] or reason
        print("\n[%s] FIN : %s" % (now(), reason), flush=True)
        QTimer.singleShot(0, app.quit)


def on_event(kind, payload):
    phase = state["phase"]
    if kind == client.READY:
        next_phrase()
    elif kind == client.PLAN_PROPOSED and phase == "propose":
        print("  composition %.1f s ; plan : %s" % (time.monotonic() - state["since"], dumps(payload["plan"])),
              flush=True)
        reason = rv.on_proposed(current(), payload["plan"])
        if reason is rv.RIGHT:
            state["phase"], state["since"] = "execute", time.monotonic()
            core.execute_plan(payload["plan_id"])
        else:
            state["phase"], state["pending"] = "reject", reason
            core.reject_plan(payload["plan_id"])
    elif kind == client.PLAN_QUESTION and phase == "propose":
        print("  question (%.1f s) : %s" % (time.monotonic() - state["since"], dumps(payload)), flush=True)
        verdict(rv.on_question(current()))
    elif kind == client.PLAN_FAILED and phase == "propose":
        print("  refus (%.1f s) : %s" % (time.monotonic() - state["since"], dumps(payload)), flush=True)
        verdict(rv.on_failed(payload))
    elif kind in (client.PLAN_REJECTED, client.PLAN_FAILED) and phase == "reject":
        verdict(state.pop("pending"))
    elif kind == client.PLAN_DONE and phase == "execute":
        print("  exécution %.1f s" % (time.monotonic() - state["since"]), flush=True)
        for step in payload["steps"]:
            result = step.get("result")
            if isinstance(result, dict):
                result = {k: v for k, v in result.items() if k != "profile"}
            print("    étape %s %s -> %s" % (step.get("step"), step.get("command"), dumps(result)[:300]),
                  flush=True)
        verdict(rv.RIGHT)
    elif kind in (client.PLAN_EXEC_FAILED, client.PLAN_CANCELLED) and phase == "execute":
        print("  %s : %s" % (kind, dumps(payload)), flush=True)
        verdict(rv.on_exec_failed(payload))
    elif kind == client.REPLAY_DONE:
        print("rejeu : %s" % dumps(payload), flush=True)
        finish("rejeu identique" if payload.get("identical") else "rejeu NON identique")
    elif kind == client.REPLAY_FAILED:
        print("rejeu : %s" % dumps(payload), flush=True)
        finish("rejeu en échec : %s" % payload.get("code"))
    elif kind in (client.FAILED, client.DIED):
        print("[%s] %s : %s" % (now(), kind, dumps(payload)), flush=True)
        finish("noyau perdu (%s)" % kind)


def guarded(kind, payload):
    """Une exception dans un rappel de Qt abattrait QGIS sans message : on la montre."""
    try:
        on_event(kind, payload)
    except Exception as error:
        traceback.print_exc()
        finish("exception dans la sonde : %s" % error)


class NoProbe:  # le geste de test du noyau n'est jamais dicté ici
    def start(self, params, done):
        done(result={})

    def cancel(self):
        pass


process = QProcess()
core = client.CoreClient(process, guarded, delegated.build_registry(NoProbe))
core.start(CORE, PROJECT)
QTimer.singleShot(GIVE_UP_MS, lambda: finish("délai dépassé"))
app.exec_() if hasattr(app, "exec_") else app.exec()
core.stop()
QgsApplication.exitQgis()
held = state["right"] >= rv.bar(len(PHRASES))
sys.exit(0 if held and state["replay"] == "rejeu identique" else 1)
