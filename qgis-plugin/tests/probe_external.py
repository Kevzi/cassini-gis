"""Sonde manuelle sous le vrai QGIS et le vrai noyau : une IA externe joue la phrase de démo.

À lancer à la main avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`) :

    set CASSINI_CORE=<cassini.exe>
    "C:\\Program Files\\QGIS 3.44.15\\bin\\python-qgis-ltr.bat" qgis-plugin/tests/probe_external.py <projet.qgz> <nom>

La sonde tient la session comme le ferait QGIS ouvert avec le panneau : elle enregistre une copie
du projet à côté de lui, `projet-sonde-5-27-<nom>.qgz` (mêmes chemins relatifs, le projet d'origine
et ses données ne changent pas), lance `cassini serve` par `client.py` avec les vrais gestes de
l'extension et allume l'interrupteur « Autoriser une IA externe », case « Me demander » éteinte.

On donne alors la phrase de démo de `CLAUDE.md` à un client MCP branché par sa fiche du README
(Gemini CLI, Codex CLI, Claude Code…). La sonde imprime chaque appel reçu : commande, durée,
issue. Quand le client a fini, on crée le fichier `projet-sonde-5-27-<nom>.stop` à côté de la
copie ; la sonde contrôle alors qu'au moins un appel a réussi, qu'une couche s'est ajoutée au
projet et qu'un PDF lisible est sorti, puis rouvre la copie dans un nouveau noyau et rejoue le
journal, qui doit rendre `identical: true`. Sans fichier d'arrêt, elle abandonne au bout de
`CASSINI_PROBE_MINUTES` minutes (30 par défaut). Code de sortie 0 si tout est vert.

Les sorties vont dans `projet-sonde-5-27-<nom>.cassini/`, vidé au départ. Aucune donnée n'est
versionnée : le projet vient de l'utilisateur (voir CLAUDE.md §5).
"""

import json
import os
import re
import shutil
import sys
import time
import traceback

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "qgis-plugin", "cassini_qgis"))
os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")

PHRASE = (
    "Trouve les parcelles constructibles du PLU à moins de 500 m d'une gare à "
    "Villeneuve-le-Roi et sors-moi la carte en PDF."
)
STOP_POLL_MS = 1000


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


if len(sys.argv) != 3 or not os.path.isfile(sys.argv[1]) or not re.fullmatch(r"[a-z0-9-]+", sys.argv[2]):
    sys.exit("usage : probe_external.py <projet.qgz> <nom en minuscules, chiffres et tirets>")
check_phrase()
CORE = core_path()
SOURCE = os.path.abspath(sys.argv[1])
BASE = os.path.join(os.path.dirname(SOURCE), "projet-sonde-5-27-%s" % sys.argv[2])
COPY = (BASE + ".qgz").replace("\\", "/")
CASSINI_DIR = BASE + ".cassini"
STOP_FILE = BASE + ".stop"
GIVE_UP_MS = int(os.environ.get("CASSINI_PROBE_MINUTES", "30")) * 60 * 1000

from qgis.analysis import QgsNativeAlgorithms  # noqa: E402
from qgis.core import QgsApplication, QgsProject  # noqa: E402
from qgis.PyQt.QtCore import QCoreApplication, QProcess, QTimer  # noqa: E402

app = QgsApplication([], False)
QgsApplication.initQgis()
QgsApplication.processingRegistry().addProvider(QgsNativeAlgorithms())
import analysis  # noqa: E402
import client  # noqa: E402
import gestures  # noqa: E402
import inventory  # noqa: E402
import layer_add  # noqa: E402
import layout_export  # noqa: E402
import point_transform  # noqa: E402

project = QgsProject.instance()
if not project.read(SOURCE):
    sys.exit("projet illisible : %s" % SOURCE)
# Copie à côté de l'original : les chemins relatifs des couches restent valables.
if not project.write(COPY):
    sys.exit("copie du projet non enregistrée : %s" % COPY)
shutil.rmtree(CASSINI_DIR, ignore_errors=True)
if os.path.exists(STOP_FILE):
    os.remove(STOP_FILE)
project.clear()
if not project.read(COPY):
    sys.exit("copie du projet illisible : %s" % COPY)

checks = []  # (libellé, vrai/faux)
state = {"client": None, "process": None, "calls": {}, "done": [], "in_flight": 0, "stopping": False}
BASE_LAYERS = set(project.mapLayers())


def now():
    return time.strftime("%H:%M:%S")


def check(label, condition, detail=""):
    checks.append((label, bool(condition)))
    print(("ok     " if condition else "ECHEC  ") + label + (" : %s" % (detail,) if detail and not condition else ""), flush=True)


def finish():
    QTimer.singleShot(0, QCoreApplication.quit)


def guarded(handler):
    """Une exception dans un rappel de Qt abattrait QGIS sans message : on la montre."""

    def wrapper(kind, payload):
        try:
            handler(kind, payload)
        except Exception:
            traceback.print_exc()
            check("aucune exception dans la sonde", False)
            finish()

    return wrapper


def read_journal():
    path = os.path.join(CASSINI_DIR, "journal.jsonl")
    if not os.path.isfile(path):
        return []
    with open(path, encoding="utf-8") as f:
        return [json.loads(line) for line in f if line.strip()]


def new_client(on_event):
    process = QProcess()
    registry = gestures.GestureRegistry()
    # Une partie des gestes de l'extension (table fermée : delegated.py), sans la commande de test.
    registry.register("layer.add", layer_add.LayerAddGesture)
    registry.register("project.inventory", inventory.ProjectInventoryGesture)
    registry.register("layer.describe", inventory.LayerDescribeGesture)
    analysis.register_all(registry)
    registry.register("layout.export_pdf", layout_export.LayoutExportGesture)
    registry.register("data.transform_point", point_transform.PointTransformGesture)
    state["process"] = process  # garder la référence
    state["client"] = client.CoreClient(process, guarded(on_event), registry)
    state["client"].start(CORE, COPY)


def short(value, limit=300):
    text = json.dumps(value, ensure_ascii=False)
    return text if len(text) <= limit else text[:limit] + "…"


# -- phase 1 : la session attend l'IA externe ---------------------------------
def on_first(kind, payload):
    if kind == client.READY:
        if not state["client"].set_external(True, False):
            check("l'interrupteur part vers le noyau", False)
            finish()
    elif kind == client.EXTERNAL_STATE:
        if not payload.get("enabled"):
            check("le noyau allume le canal d'IA externe", False, payload)
            finish()
        else:
            print("[%s] interrupteur allumé ; copie : %s" % (now(), COPY), flush=True)
            print("phrase à donner au client : %s" % PHRASE, flush=True)
            print("fichier d'arrêt, à créer quand le client a fini : %s" % STOP_FILE, flush=True)
            QTimer.singleShot(STOP_POLL_MS, poll_stop)
    elif kind == client.EXTERNAL_FAILED:
        check("le noyau ouvre le canal d'IA externe", False, payload)
        finish()
    elif kind == client.EXTERNAL_CALL_STARTED:
        state["in_flight"] += 1
        state["calls"][payload["call_id"]] = time.monotonic()
        print("[%s] → %s %s" % (now(), payload.get("command"), short(payload.get("params"))), flush=True)
    elif kind == client.EXTERNAL_CALL_FINISHED:
        state["in_flight"] = max(0, state["in_flight"] - 1)
        state["done"].append(payload)
        issue = "erreur %s" % short(payload["error"]) if payload.get("error") else short(payload.get("result"))
        print("[%s] ← %s (%s ms) %s" % (now(), payload.get("command"), payload.get("duration_ms"), issue), flush=True)
    elif kind in (client.FAILED, client.DIED):
        check("le noyau tient la session", False, "%s %s" % (kind, payload))
        finish()


def poll_stop():
    if state["stopping"]:
        return
    if os.path.exists(STOP_FILE) and state["in_flight"] == 0:
        state["stopping"] = True
        QTimer.singleShot(0, between)
        return
    QTimer.singleShot(STOP_POLL_MS, poll_stop)


def give_up():
    if not state["stopping"]:
        state["stopping"] = True
        print("[%s] délai dépassé" % now(), flush=True)
        between()


def pdf_files():
    if not os.path.isdir(CASSINI_DIR):
        return []
    return sorted(
        os.path.join(CASSINI_DIR, name) for name in os.listdir(CASSINI_DIR) if name.endswith(".pdf")
    )


def between():
    try:
        _between()
    except Exception:
        traceback.print_exc()
        check("aucune exception dans la sonde", False)
        finish()


def _between():
    succeeded = [c for c in state["done"] if not c.get("error")]
    commands = [c.get("command") for c in state["done"]]
    print("appels reçus : %d (%d réussis) : %s" % (len(state["done"]), len(succeeded), ", ".join(map(str, commands))), flush=True)
    check("au moins un appel externe a réussi", succeeded, commands)
    added = [project.mapLayer(i).name() for i in set(project.mapLayers()) - BASE_LAYERS]
    check("au moins une couche s'est ajoutée au projet", added)
    if added:
        print("couches ajoutées : %s" % ", ".join(added), flush=True)
    pdfs = pdf_files()
    readable = []
    for path in pdfs:
        with open(path, "rb") as f:
            if f.read(4) == b"%PDF":
                readable.append(os.path.basename(path))
    check("un PDF lisible est sorti", readable, pdfs)
    rows = read_journal()
    # Une étape en erreur ou annulée est journalisée aussi : on ne compte que les réussies.
    external = [
        r["id"] for r in rows
        if r.get("origin") == "external" and "error" not in r and not r.get("cancelled")
    ]
    check("le journal porte les appels réussis avec l'origine external", len(external) == len(succeeded), (external, len(succeeded)))
    state["client"].set_external(False)
    state["client"].stop()
    # Fermeture puis réouverture de la copie : le rejeu refait les couches de résultat.
    project.clear()
    check("la copie rouvre", project.read(COPY))
    new_client(on_second)


# -- phase 2 : rejouer --------------------------------------------------------
def on_second(kind, payload):
    if kind == client.READY:
        check("le rejeu part", state["client"].replay())
    elif kind == client.REPLAY_DONE:
        check("le rejeu rend identical: true", payload.get("identical") is True, payload)
        print("étapes rejouées : %s" % payload.get("steps"), flush=True)
        finish()
    elif kind in (client.REPLAY_FAILED, client.FAILED, client.DIED):
        check("rejeu nominal", False, "%s %s" % (kind, payload))
        finish()


new_client(on_first)
QTimer.singleShot(GIVE_UP_MS, give_up)
app.exec_() if hasattr(app, "exec_") else app.exec()
if state["client"] is not None:
    state["client"].stop()
QgsApplication.exitQgis()
failed = [label for label, ok in checks if not ok]
print("verdict : %s" % ("réussite" if checks and not failed else "échec (%s)" % "; ".join(failed)), flush=True)
sys.exit(0 if checks and not failed else 1)
