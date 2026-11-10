"""Sonde manuelle sous le vrai QGIS : carroyage MGRS, en-tête, pied de page et sources détaillées
de `layout.export_pdf` (epic 10, entrée 31), par le noyau, puis rejeu.

À lancer avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`), sous
`QT_QPA_PLATFORM=windows` (sous `offscreen`, Qt n'a pas les polices) :

    "C:\\Program Files\\QGIS 3.44.15\\bin\\python-qgis-ltr.bat" qgis-plugin/tests/probe_mgrs_layout.py

Le noyau est cherché dans `CASSINI_CORE`. Le projet est `CASSINI_PROJECT`, par défaut
`../donnees-test-cassini/annecy/projet-test-annecy.qgz` (hors dépôt) : il est relu puis
enregistré sous un autre nom dans le même dossier (`projet-sonde-10-31.qgz`, chemins relatifs
gardés), jamais modifié.

1. Par le noyau, deux exports A3 paysage : le département à 1:… (cadré sur les couches) avec un
   carroyage MGRS de 10 km, un en-tête et un pied de page ; puis à 1:25 000 avec un carroyage
   MGRS de 1 km. Le noyau lit l'inventaire et pose la zone et les carrés : le journal porte
   `grid.epsg` 32632 et `grid.squares` ; le résultat porte `source_details` (taille de cellule
   du MNT, date du fichier) ; la mise en page porte l'en-tête, le pied et le cadre des carrés.
2. Projet fermé et rouvert : rejeu du journal, `identical: true`.

Les PDF restent dans le dossier de travail du projet de sonde (`projet-sonde-10-31.cassini/`) :
à rendre en PNG et à regarder.
"""

import json
import os
import shutil
import sys
import traceback

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "qgis-plugin", "cassini_qgis"))
SOURCE_PROJECT = os.environ.get("CASSINI_PROJECT") or os.path.normpath(
    os.path.join(ROOT, "..", "donnees-test-cassini", "annecy", "projet-test-annecy.qgz")
)

from qgis.core import QgsApplication, QgsProject  # noqa: E402
from qgis.PyQt.QtCore import QCoreApplication, QProcess, QTimer  # noqa: E402

app = QgsApplication([], False)
QgsApplication.initQgis()
import client  # noqa: E402
import gestures  # noqa: E402
import inventory  # noqa: E402
import layout_export  # noqa: E402

checks = []
state = {"client": None}


def check(label, condition, detail=""):
    checks.append((label, bool(condition)))
    print(("ok     " if condition else "ECHEC  ") + label + (" : %s" % (detail,) if detail and not condition else ""))
    sys.stdout.flush()


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


core = os.environ.get("CASSINI_CORE")
assert core and os.path.isfile(core), "CASSINI_CORE manque"
project = QgsProject.instance()
assert project.read(SOURCE_PROJECT), "projet illisible : %s" % SOURCE_PROJECT
folder = os.path.dirname(SOURCE_PROJECT)
project_file = os.path.join(folder, "projet-sonde-10-31.qgz").replace("\\", "/")
cassini_dir = os.path.join(folder, "projet-sonde-10-31.cassini")
shutil.rmtree(cassini_dir, ignore_errors=True)
assert project.write(project_file), "projet de sonde non enregistré"
print("projet de sonde : %s (%d couches)" % (project_file, len(project.mapLayers())))

# Le geste d'export garde la dernière mise en page posée : on la relit pour le cadre des carrés.
placed = []


class WatchedExport(layout_export.LayoutExportGesture):
    def _read(self):
        result = super()._read()
        placed.append(list(self.items_placed))
        return result


def new_client(on_event):
    process = QProcess()
    registry = gestures.GestureRegistry()
    registry.register("project.inventory", inventory.ProjectInventoryGesture)
    registry.register("layer.describe", inventory.LayerDescribeGesture)
    registry.register("layout.export_pdf", WatchedExport)
    state["process"] = process
    state["client"] = client.CoreClient(process, protect(on_event), registry)
    state["client"].start(core, project_file)


def read_journal():
    with open(os.path.join(cassini_dir, "journal.jsonl"), encoding="utf-8") as f:
        return [json.loads(line) for line in f if line.strip()]


HEADER = "Document de travail — Bureau d'études Lac et Monts"
FOOTER = "Usage interne au client — ne pas diffuser — version du 10 novembre 2026"
STEPS = [
    ("layout.export_pdf", {"title": "Haute-Savoie — carroyage MGRS de 10 km", "page_size": "a3",
                           "header": HEADER, "footer": FOOTER,
                           "grid": {"kind": "mgrs", "interval": {"value": 10, "unit": "km"}}}),
    ("layout.export_pdf", {"title": "Secteur de recherche — MGRS 1 km", "page_size": "a3", "scale": 25000,
                           "header": "Secours en montagne — carte d'intervention",
                           "grid": {"kind": "mgrs", "interval": {"value": 1, "unit": "km"}}}),
]
results = []


def run_next():
    command, params = STEPS[len(results)]
    state["client"].run_command(command, params)


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
    rows = read_journal()
    check("journal : deux exports", [r["id"] for r in rows] == ["layout.export_pdf"] * 2, [r["id"] for r in rows])
    for n, row in enumerate(rows):
        grid = row["params"]["grid"]
        check("export %d : zone 32632 posée par le noyau" % (n + 1), grid.get("epsg") == 32632, grid)
        squares = grid.get("squares") or []
        check("export %d : carrés de 100 km en 32T" % (n + 1),
              squares and all(q.startswith("32T ") or q == "…" for q in squares), squares)
        print("       carrés : %s" % ", ".join(squares))
    for n, result in enumerate(results):
        details = result.get("source_details") or []
        check("export %d : source_details, une par source" % (n + 1),
              [d["source"] for d in details] == result.get("sources"), (result.get("sources"), details))
        mnt = [d for d in details if d["source"].startswith("mnt-")]
        check("export %d : le MNT a sa taille de cellule en mètres" % (n + 1),
              mnt and all(d["resolution"] and d["resolution"]["unit"] == "m" for d in mnt), mnt)
        dated = [d for d in details if d["date"]]
        check("export %d : des dates avec leur origine" % (n + 1),
              dated and all(d["date_origin"] in ("metadata", "file") for d in dated), details)
        for d in details:
            print("       %s" % d)
        check("export %d : PDF écrit" % (n + 1), os.path.isfile(result["file"]), result["file"])
        print("       PDF : %s" % result["file"])
    check("export 1 : en-tête, pied et cadre des carrés posés",
          placed and all(k in placed[0] for k in ("header", "footer", "grid_squares")), placed)
    check("export 2 : en-tête et cadre des carrés, sans pied",
          len(placed) > 1 and "header" in placed[1] and "grid_squares" in placed[1] and "footer" not in placed[1],
          placed)
    state["journal"] = rows
    state["client"].stop()
    project.clear()
    check("le projet rouvre", project.read(project_file))
    new_client(on_second)


def on_second(kind, payload):
    if kind == client.READY:
        check("le rejeu part", state["client"].replay())
    elif kind == client.REPLAY_DONE:
        check("le rejeu rend identical: true", payload == {"steps": 2, "identical": True}, payload)
        check("le journal est inchangé par le rejeu", read_journal() == state["journal"])
        finish()
    elif kind in (client.REPLAY_FAILED, client.FAILED, client.DIED):
        check("rejeu nominal", False, "%s %s" % (kind, payload))
        finish()


new_client(on_first)
QTimer.singleShot(1200000, QCoreApplication.quit)
QCoreApplication.exec()
if state["client"] is not None:
    state["client"].stop()
ok = len(checks) >= 18 and all(v for _, v in checks)
project.clear()
print("RÉSULTAT :", "OK" if ok else "ÉCHEC")
sys.stdout.flush()
os._exit(0 if ok else 1)
