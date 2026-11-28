"""Sonde manuelle sous le vrai QGIS : confiance envers le programme noyau branché (ticket 9.15).

À lancer avec le Python de QGIS (pas par `unittest`), avec un affichage :

    set QT_QPA_PLATFORM=windows
    set CASSINI_EXAMPLE_CORE=C:\\tmp\\ct-9-15\\debug\\programme-exemple.exe
    "C:\\Program Files\\QGIS 3.44.15\\bin\\python-qgis-ltr.bat" qgis-plugin/tests/probe_program_trust.py

Construire le programme d'exemple de 2.14 :
    set CARGO_TARGET_DIR=C:\\tmp\\ct-9-15 && cargo build -p programme-exemple

Elle charge l'extension réelle (`CassiniPlugin`, son vrai `_launch`, son vrai panneau) et ne
remplace que `_confirm_program` par une réponse programmée. Les réglages QSettings sont ceux d'une
organisation de sonde, jamais ceux de l'utilisateur. Aucun modèle n'est chargé. Le programme
officiel est lu à `%LOCALAPPDATA%\\Cassini\\bin\\cassini.exe`, sans être modifié.

1. programme d'exemple (variable CASSINI_CORE) : refus => aucun processus, bannière
   `trust_refused` ; accord => lancé, nom/version/mention dérivée/AD-14 dans le panneau ; relance
   sur le même binaire => aucune question ; copie dont un octet change => nouvelle question ;
2. programme officiel à l'emplacement d'installation : aucune question, `program_name` = `cassini`,
   aucune mention dérivée.
"""

import hashlib
import os
import shutil
import sys
import tempfile
import traceback

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "qgis-plugin"))

from qgis.core import QgsApplication, QgsCoordinateReferenceSystem, QgsProject  # noqa: E402
from qgis.PyQt.QtCore import QCoreApplication, QEventLoop, QSettings, QTimer  # noqa: E402
from qgis.PyQt.QtWidgets import QMainWindow  # noqa: E402

QCoreApplication.setOrganizationName("CassiniProbe915")  # réglages isolés de ceux de l'utilisateur
QCoreApplication.setApplicationName("probe-program-trust")
app = QgsApplication([], True)
QgsApplication.initQgis()
from cassini_qgis import locator, panel, plugin, texts, trust  # noqa: E402

EXAMPLE = os.environ.get("CASSINI_EXAMPLE_CORE") or r"C:\tmp\ct-9-15\debug\programme-exemple.exe"
CURRENT = os.environ.get("CASSINI_CURRENT_CORE") or r"C:\tmp\ct-9-15\debug\cassini.exe"
OFFICIAL = os.path.join(os.environ.get("LOCALAPPDATA", ""), "Cassini", "bin", "cassini.exe")
checks = []


def check(label, condition, detail=""):
    checks.append((label, bool(condition)))
    print(("ok     " if condition else "ECHEC  ") + label
          + ("" if condition or detail == "" else " : %r" % (detail,)))


def sha256(path):
    with open(path, "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()


class FakeIface:
    def __init__(self):
        self.window = QMainWindow()

    def mainWindow(self):
        return self.window


def wait(condition, timeout_ms=30000):
    loop = QEventLoop()
    poll = QTimer()
    poll.setInterval(50)
    poll.timeout.connect(lambda: loop.quit() if condition() else None)
    poll.start()
    QTimer.singleShot(timeout_ms, loop.quit)
    loop.exec()
    poll.stop()
    return condition()


work = tempfile.mkdtemp(prefix="cassini-trust-")
project_file = os.path.join(work, "essai.qgz").replace("\\", "/")
project = QgsProject.instance()
project.setCrs(QgsCoordinateReferenceSystem("EPSG:2154"))
assert project.write(project_file), "projet non enregistré"

QSettings().remove(trust.ACCEPTED_KEY)
QSettings().remove(locator.SETTING_KEY)

calls = []
answers = []


def fake_confirm(self, program, digest):
    calls.append((program, digest))
    return answers.pop(0)


plugin.CassiniPlugin._confirm_program = fake_confirm  # seule substitution : la boîte modale
p = plugin.CassiniPlugin(FakeIface())
p._init_done = True
p.panel = panel.CassiniPanel(p)
p.panel.lang = "fr"
p.panel.project_file = project_file


def launch(core_env):
    """Relance réelle : arrête la session, remet le projet à zéro puis appelle le vrai `_launch`."""
    p._stop()
    p.project_path = project_file
    if core_env is None:
        os.environ.pop("CASSINI_CORE", None)
    else:
        os.environ["CASSINI_CORE"] = core_env
    del calls[:]
    p._launch()


def ready():
    return p.panel.ready and p.panel.program_name is not None


try:
    check("programme d'exemple présent", os.path.isfile(EXAMPLE), EXAMPLE)
    check("programme officiel présent à l'emplacement d'installation", os.path.isfile(OFFICIAL),
          OFFICIAL)
    found = locator.locate("", env={"LOCALAPPDATA": os.environ["LOCALAPPDATA"]})
    check("locator rend l'origine « emplacement d'installation » pour l'officiel",
          found == (OFFICIAL, trust.OFFICIAL_ORIGIN), found)
    expected = sha256(EXAMPLE)  # recalculée indépendamment de trust.fingerprint

    # 1a. refus
    answers[:] = [False]
    launch(EXAMPLE)
    check("refus : une seule question", len(calls) == 1, calls)
    check("refus : empreinte montrée = SHA-256 recalculée",
          calls and calls[0][1] == expected, calls)
    check("refus : chemin montré = programme d'exemple", calls and calls[0][0] == EXAMPLE, calls)
    check("refus : aucun QProcess ni client", p.process is None and p.client is None,
          (p.process, p.client))
    check("refus : bannière trust_refused", p.panel.banner == "trust_refused", p.panel.banner)
    check("refus : texte de la bannière affiché",
          p.panel.banner_text.text() == texts.tr("fr", "trust_refused"), p.panel.banner_text.text())
    check("refus : aucune empreinte retenue",
          trust.parse_accepted(QSettings().value(trust.ACCEPTED_KEY, "")) == [])

    # 1b. accord
    answers[:] = [True]
    launch(EXAMPLE)
    check("accord : une seule question", len(calls) == 1, calls)
    check("accord : le noyau est prêt", wait(ready), p.panel.banner)
    check("accord : empreinte retenue",
          trust.parse_accepted(QSettings().value(trust.ACCEPTED_KEY, "")) == [expected])
    pn = p.panel
    check("accord : program_name = programme-exemple", pn.program_name == "programme-exemple",
          pn.program_name)
    check("accord : program_version renseignée", bool(pn.program_version), pn.program_version)
    print("       version annoncée :", pn.program_version)
    check("accord : ligne nom + version visible",
          pn.program_line.isVisibleTo(pn)
          and pn.program_line.text() == texts.tr("fr", "program_line", name=pn.program_name,
                                                 version=pn.program_version), pn.program_line.text())
    check("accord : mention dérivée + AD-14 visibles",
          pn.program_note.isVisibleTo(pn)
          and texts.tr("fr", "program_derived") in pn.program_note.text()
          and texts.tr("fr", "program_ad14") in pn.program_note.text(), pn.program_note.text())
    print("       ligne :", pn.program_line.text())
    print("       note  :", pn.program_note.text())

    # 1c. même binaire : aucune question
    answers[:] = []
    launch(EXAMPLE)
    check("relance : aucune question", len(calls) == 0, calls)
    check("relance : le noyau est prêt", wait(ready))

    # 1d. binaire modifié (copie temporaire, un octet de plus en fin de fichier)
    copy = os.path.join(work, "programme-exemple-modifie.exe")
    shutil.copyfile(EXAMPLE, copy)
    with open(copy, "ab") as f:
        f.write(b"\x00")
    answers[:] = [True]
    launch(copy)
    check("binaire modifié : question redemandée", len(calls) == 1, calls)
    check("binaire modifié : nouvelle empreinte, différente et exacte",
          calls and calls[0][1] == sha256(copy) and calls[0][1] != expected, calls)
    check("binaire modifié : le noyau est prêt", wait(ready))
    check("binaire modifié : deux empreintes retenues",
          len(trust.parse_accepted(QSettings().value(trust.ACCEPTED_KEY, ""))) == 2)

    # 2a. programme officiel installé (emplacement d'installation, non modifié). Depuis
    # le 2026-11-28 elle est construite après 2.14 : elle annonce `cassini`, sans question.
    official_before = sha256(OFFICIAL)
    answers[:] = []
    launch(None)
    check("officiel installé : aucune question", len(calls) == 0, calls)
    check("officiel installé : le noyau est prêt", wait(lambda: p.panel.ready, 60000),
          p.panel.banner)
    pn = p.panel
    check("officiel installé : program_name = cassini", pn.program_name == "cassini",
          pn.program_name)
    check("officiel installé : ligne visible, aucune mention dérivée",
          pn.program_line.isVisibleTo(pn) and not pn.program_note.isVisibleTo(pn))
    check("officiel installé : binaire inchangé", sha256(OFFICIAL) == official_before)

    # 2b. programme officiel construit depuis ce dépôt (annonce `cassini`), via CASSINI_CORE :
    # la confirmation est alors demandée (origine « variable »), puis aucune mention dérivée.
    answers[:] = [True]
    launch(CURRENT)
    check("officiel du dépôt via CASSINI_CORE : confirmation demandée (origine variable)",
          len(calls) == 1 and calls[0][1] == sha256(CURRENT), calls)
    check("officiel du dépôt : le noyau est prêt", wait(ready, 60000), p.panel.banner)
    pn = p.panel
    check("officiel du dépôt : program_name = cassini", pn.program_name == "cassini",
          pn.program_name)
    check("officiel du dépôt : version renseignée", bool(pn.program_version), pn.program_version)
    check("officiel du dépôt : ligne nom + version visible",
          pn.program_line.isVisibleTo(pn)
          and pn.program_line.text() == texts.tr("fr", "program_line", name="cassini",
                                                 version=pn.program_version), pn.program_line.text())
    check("officiel du dépôt : aucune mention dérivée", not pn.program_note.isVisibleTo(pn),
          pn.program_note.text())
    print("       ligne :", pn.program_line.text())
finally:
    traceback.print_exc() if sys.exc_info()[0] else None
    p._stop()
    QSettings().remove(trust.ACCEPTED_KEY)
    shutil.rmtree(work, ignore_errors=True)

ok = len(checks) >= 30 and all(v for _, v in checks)
print("RÉSULTAT :", "OK" if ok else "ÉCHEC")
sys.exit(0 if ok else 1)
