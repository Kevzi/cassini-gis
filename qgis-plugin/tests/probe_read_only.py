"""Sonde manuelle sous le vrai QGIS et le vrai binaire : rejeu en session en lecture seule.

À lancer avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`),
avec un affichage (le panneau est construit) :

    set QT_QPA_PLATFORM=windows
    "C:\\Program Files\\QGIS 3.44.15\\bin\\python-qgis-ltr.bat" qgis-plugin/tests/probe_read_only.py

Le noyau est cherché dans `CASSINI_CORE`, sinon `target/release/cassini.exe`, sinon
`target/debug/`. Aucun modèle n'est chargé : une commande experte remplit le journal.

Dans un dossier temporaire, avec la fixture synthétique `fixtures/points-2154.csv` :

1. un premier noyau (la première fenêtre) tient le journal et lance `layer.add` ;
2. pendant qu'il vit, un second noyau s'ouvre sur le même projet : `session.hello` répond
   `read_only: true`, le client reste prêt, refuse d'envoyer une commande, et le vrai
   panneau, en lecture seule, montre le bandeau de la spine et ne laisse que « Rejouer » ;
3. le rejeu part de la seconde session : `identical: true`, le panneau dit « résultat
   identique », et le journal n'a pas changé.
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
sys.path.insert(0, os.path.join(ROOT, "qgis-plugin"))
FIXTURE = os.path.join(ROOT, "fixtures", "points-2154.csv")

from qgis.core import QgsApplication, QgsCoordinateReferenceSystem, QgsProject  # noqa: E402
from qgis.PyQt.QtCore import QCoreApplication, QProcess, QTimer  # noqa: E402

app = QgsApplication([], True)
QgsApplication.initQgis()
from cassini_qgis import client, gestures, layer_add, panel, texts  # noqa: E402


def find_core():
    candidates = [os.environ.get("CASSINI_CORE")]
    for profile in ("release", "debug"):
        candidates.append(os.path.join(ROOT, "target", profile, "cassini.exe"))
        candidates.append(os.path.join(ROOT, "target", profile, "cassini"))
    for path in candidates:
        if path and os.path.isfile(path):
            return path
    sys.exit("Noyau introuvable : posez CASSINI_CORE ou lancez `cargo build --release`.")


def sha256(path):
    with open(path, "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()


CORE = find_core()
work = tempfile.mkdtemp(prefix="cassini-lecture-seule-")
csv_path = os.path.join(work, "points-2154.csv")
shutil.copyfile(FIXTURE, csv_path)
csv_hash = sha256(csv_path)
project_file = os.path.join(work, "essai.qgz").replace("\\", "/")
journal = os.path.join(work, "essai.cassini", "journal.jsonl")

project = QgsProject.instance()
project.setCrs(QgsCoordinateReferenceSystem("EPSG:2154"))
assert project.write(project_file), "projet non enregistré"

checks = []
state = {}


def check(label, condition, detail=""):
    checks.append((label, bool(condition)))
    print(("ok     " if condition else "ECHEC  ") + label
          + (" : %s" % (detail,) if detail and not condition else ""))


def finish():
    QTimer.singleShot(0, QCoreApplication.quit)


def guarded(handler):
    def wrapper(kind, payload):
        try:
            handler(kind, payload)
        except Exception:
            traceback.print_exc()
            check("aucune exception dans la sonde", False)
            finish()

    return wrapper


def read_journal():
    with open(journal, encoding="utf-8") as f:
        return [json.loads(line) for line in f if line.strip()]


def new_client(name, on_event):
    process = QProcess()
    registry = gestures.GestureRegistry()
    registry.register("layer.add", layer_add.LayerAddGesture)
    state[name + "_process"] = process
    state[name] = client.CoreClient(process, guarded(on_event), registry)
    state[name].start(CORE, project_file)


# -- première fenêtre : elle tient le journal -----------------------------------
def on_writer(kind, payload):
    if kind == client.READY:
        check("la première session écrit", payload.get("read_only") is False, payload)
        state["writer"].run_command("layer.add", {"path": "points-2154.csv", "crs": "EPSG:2154"})
    elif kind == client.RUN_DONE:
        state["journal"] = read_journal()
        check("le journal a une étape", [r["id"] for r in state["journal"]] == ["layer.add"])
        new_client("reader", on_reader)  # seconde fenêtre, la première vivante
    elif kind in (client.RUN_FAILED, client.RUN_CANCELLED, client.FAILED, client.DIED):
        check("la première session lance layer.add", False, "%s %s" % (kind, payload))
        finish()


class Actions:
    """Gestes du panneau, branchés sur la seconde session comme le ferait le greffon."""

    def replay(self):
        return state["reader"].replay()

    def list_models(self):
        return state["reader"].list_models()

    def __getattr__(self, name):
        return lambda *a, **k: False


# -- seconde fenêtre : lecture seule ---------------------------------------------
def on_reader(kind, payload):
    if kind == client.READY:
        reader = state["reader"]
        check("la seconde session est prête en lecture seule", payload.get("read_only") is True,
              payload)
        check("le noyau de la seconde session reste lancé",
              state["reader_process"].state() != QProcess.ProcessState.NotRunning)
        check("le client refuse une commande", reader.run_command("system.about") is False)
        check("le client refuse une demande", reader.propose("Tampon") is False)
        p = state["panel"] = panel.CassiniPanel(Actions())
        p.lang = "fr"
        p.project_file = project_file
        p.set_ready(payload.get("commands", []), payload.get("history", []), read_only=True)
        p.input.setPlainText("Tampon de 100 m")  # même avec du texte, Envoyer reste grisé
        p.render()
        check("le bandeau de la spine est affiché",
              p.banner_zone.isVisibleTo(p)
              and p.banner_text.text() == texts.tr("fr", "read_only"), p.banner_text.text())
        reason = texts.tr("fr", "read_only_reason")
        check("saisie et Envoyer grisés avec la raison",
              not p.input.isEnabled() and not p.send_button.isEnabled()
              and p.input_reason.text() == reason, p.input_reason.text())
        check("Lancer grisé avec la raison",
              not p.run_button.isEnabled() and p.reason.text() == reason, p.reason.text())
        check("modèle et IA externe grisés",
              not p.model_box.isEnabled() and not p.external_switch.isEnabled())
        check("l'historique est affiché", len(p.log.entries) == 1, len(p.log.entries))
        check("Rejouer le journal est actif", p.replay_button.isEnabled())
        p._replay()
        check("le rejeu part de la seconde session", reader.replay_busy())
    elif kind in (client.REPLAY_DONE, client.REPLAY_FAILED):
        check("le rejeu en lecture seule rend identical: true",
              kind == client.REPLAY_DONE and payload == {"steps": 1, "identical": True}, payload)
        p = state["panel"]
        p.on_event(kind, payload)
        check("le repli Journal dit « résultat identique »",
              texts.tr("fr", "replay_identical") in p.replay_status.text(),
              p.replay_status.text())
        check("le journal n'a pas changé", read_journal() == state["journal"])
        check("la session reste en lecture seule", p.read_only and not p.send_button.isEnabled())
        finish()
    elif kind in (client.FAILED, client.DIED):
        check("la seconde session reste ouverte", False, "%s %s" % (kind, payload))
        finish()


new_client("writer", on_writer)
QTimer.singleShot(60000, QCoreApplication.quit)
QCoreApplication.exec()

for name in ("reader", "writer"):
    if state.get(name) is not None:
        state[name].stop()
check("le CSV d'origine est inchangé (empreinte)", sha256(csv_path) == csv_hash)
ok = len(checks) >= 12 and all(v for _, v in checks)
shutil.rmtree(work, ignore_errors=True)
print("RÉSULTAT :", "OK" if ok else "ÉCHEC")
sys.exit(0 if ok else 1)
