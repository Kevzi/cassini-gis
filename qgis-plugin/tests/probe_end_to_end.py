"""Sonde manuelle sous le vrai QGIS et le vrai binaire : la chaîne entière.

À lancer avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`),
après `cargo build --release` :

    "C:\\Program Files\\QGIS 3.44.15\\bin\\python-qgis-ltr.bat" qgis-plugin/tests/probe_end_to_end.py

Le noyau est cherché dans `CASSINI_CORE`, sinon `target/release/cassini.exe`, sinon `target/debug/`.

Dans un dossier temporaire, avec la fixture synthétique `fixtures/points-2154.csv` :

1. un projet vide est enregistré, le noyau démarre, puis `layer.add` (CSV, `crs`, et le réseau
   `fixtures/reseau-grille-2154.geojson`), `analysis.buffer`, `analysis.select_by_attribute`,
   `analysis.intersect`, `analysis.isochrone` (à pied puis en voiture) et `layout.export_pdf`
   s'enchaînent par le canal ;
2. le noyau s'arrête, le projet est fermé puis rouvert (vide : le rejeu refait les couches) ;
3. un nouveau noyau démarre et `replay()` rejoue toutes les étapes : `identical: true`, et les
   sorties des analyses (dont les deux isochrones) existent sous `replay/` ;
4. un résultat du journal est altéré : le rejeu s'arrête en nommant l'étape de `analysis.buffer`.

Le client lance `system.about` à chaque démarrage, et le noyau le journalise : le journal compte donc
`system.about`, les étapes de `STEPS`, puis le `system.about` du second démarrage.

Vérifie aussi que le CSV d'origine est inchangé (empreinte) et que les PDF commencent par `%PDF`.
Aucune donnée hors dépôt n'est utilisée.
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
FIXTURE = os.path.join(ROOT, "fixtures", "points-2154.csv")
NETWORK = os.path.join(ROOT, "fixtures", "reseau-grille-2154.geojson")

from qgis.analysis import QgsNativeAlgorithms  # noqa: E402
from qgis.core import QgsApplication, QgsCoordinateReferenceSystem, QgsProject  # noqa: E402
from qgis.PyQt.QtCore import QCoreApplication, QProcess, QTimer  # noqa: E402

app = QgsApplication([], False)
QgsApplication.initQgis()
QgsApplication.processingRegistry().addProvider(QgsNativeAlgorithms())
import analysis  # noqa: E402
import client  # noqa: E402
import gestures  # noqa: E402
import layer_add  # noqa: E402
import layout_export  # noqa: E402


def find_core():
    candidates = [os.environ.get("CASSINI_CORE")]
    for profile in ("release", "debug"):
        candidates.append(os.path.join(ROOT, "target", profile, "cassini.exe"))
        candidates.append(os.path.join(ROOT, "target", profile, "cassini"))
    for path in candidates:
        if path and os.path.isfile(path):
            return path
    sys.exit("Noyau introuvable : lancez `cargo build --release`.")


def sha256(path):
    with open(path, "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()


CORE = find_core()
work = tempfile.mkdtemp(prefix="cassini-e2e-")
csv_path = os.path.join(work, "points-2154.csv")
shutil.copyfile(FIXTURE, csv_path)
csv_hash = sha256(csv_path)
network_path = os.path.join(work, "reseau-grille-2154.geojson")
shutil.copyfile(NETWORK, network_path)
network_hash = sha256(network_path)
project_file = os.path.join(work, "essai.qgz").replace("\\", "/")
cassini_dir = os.path.join(work, "essai.cassini")

project = QgsProject.instance()
project.setCrs(QgsCoordinateReferenceSystem("EPSG:2154"))
assert project.write(project_file), "projet non enregistré"

checks = []  # (libellé, vrai/faux)
state = {"client": None, "layer_id": None}


def check(label, condition, detail=""):
    checks.append((label, bool(condition)))
    print(("ok     " if condition else "ECHEC  ") + label + (" : %s" % (detail,) if detail and not condition else ""))


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
    with open(os.path.join(cassini_dir, "journal.jsonl"), encoding="utf-8") as f:
        return [json.loads(line) for line in f if line.strip()]


def new_client(on_event):
    process = QProcess()
    registry = gestures.GestureRegistry()
    registry.register("layer.add", layer_add.LayerAddGesture)
    analysis.register_all(registry)
    registry.register("layout.export_pdf", layout_export.LayoutExportGesture)
    state["process"] = process  # garder la référence
    state["client"] = client.CoreClient(process, guarded(on_event), registry)
    state["client"].start(CORE, project_file)


# -- phase 1 : exécuter -------------------------------------------------------
# Origine au centre de la grille (carré de 1 km de côté depuis 652000, 6850000).
ORIGIN = {"x": 652500, "y": 6850500, "epsg": 2154}
STEPS = [
    ("layer.add", lambda: {"path": "points-2154.csv", "crs": "EPSG:2154"}),
    ("layer.add", lambda: {"path": "reseau-grille-2154.geojson"}),
    ("analysis.buffer", lambda: {"layer_id": results[0]["layer_id"], "distance": {"value": 100, "unit": "m"}}),
    (
        "analysis.select_by_attribute",
        lambda: {"layer_id": results[0]["layer_id"], "field": "name", "operator": "eq", "value": "Point A"},
    ),
    (
        "analysis.intersect",
        lambda: {"layer_id": results[0]["layer_id"], "overlay_layer_id": results[2]["layer_id"]},
    ),
    (
        "analysis.isochrone",
        lambda: {"layer_id": results[1]["layer_id"], "origin": ORIGIN, "mode": "walk", "duration": {"value": 3, "unit": "min"}},
    ),
    (
        "analysis.isochrone",
        lambda: {"layer_id": results[1]["layer_id"], "origin": ORIGIN, "mode": "car", "duration": {"value": 180, "unit": "s"}},
    ),
    ("layout.export_pdf", lambda: {"title": "Essai de bout en bout"}),
]
STEP_IDS = [name for name, _ in STEPS]
results = []


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
            QTimer.singleShot(0, between)
    elif kind in (client.RUN_FAILED, client.RUN_CANCELLED, client.FAILED, client.DIED):
        check("exécution de toutes les étapes", False, "%s %s" % (kind, payload))
        finish()


def between():
    try:
        _between()
    except Exception:
        traceback.print_exc()
        check("aucune exception dans la sonde", False)
        finish()


def _between():
    buffer_result, select_result, intersect_result = results[2], results[3], results[4]
    walk_result, car_result, export_result = results[5], results[6], results[7]
    check("le tampon rend 4 entités", buffer_result.get("feature_count") == 4, buffer_result)
    check("la sélection par attribut rend 1 entité", select_result.get("feature_count") == 1, select_result)
    check("l'intersection rend 4 entités", intersect_result.get("feature_count") == 4, intersect_result)
    check("les deux isochrones rendent un polygone", walk_result.get("feature_count") == 1 and car_result.get("feature_count") == 1, (walk_result, car_result))
    check("l'export dessine 7 couches", export_result.get("layer_count") == 7, export_result)
    rows = read_journal()
    # Le client lance `system.about` à chaque démarrage : c'est l'étape 1 du journal.
    check(
        "le journal a l'accueil puis toutes les étapes",
        [r["id"] for r in rows] == ["system.about"] + STEP_IDS,
        [r["id"] for r in rows],
    )
    state["export_step"] = [r["id"] for r in rows].index("layout.export_pdf") + 1
    pdf = os.path.join(cassini_dir, "%d-layout.export_pdf.pdf" % state["export_step"])
    with open(pdf, "rb") as f:
        check("le PDF d'origine commence par %PDF", f.read(4) == b"%PDF")
    check("le journal garde le layer_id des couches créées", all(r.get("layer_id") for r in rows[1:-1]))
    state["analysis_steps"] = [
        (i + 1, r["id"]) for i, r in enumerate(rows) if r["id"].startswith("analysis.")
    ]
    state["client"].stop()
    # Fermeture, puis rouverture du projet : vide, car le rejeu refait les couches.
    project.clear()
    check("le projet rouvre", project.read(project_file))
    check("le projet rouvert est vide", len(project.mapLayers()) == 0)
    new_client(on_second)


# -- phase 2 : rejouer --------------------------------------------------------
def on_second(kind, payload):
    if kind == client.READY:
        check("l'extension est branchée au nouveau noyau", state["client"].replay())
    elif kind == client.REPLAY_DONE:
        # Le journal compte alors le second `system.about` : le rejeu les refait tous.
        expected = {"steps": len(read_journal()), "identical": True}
        check(
            "le rejeu rend identical: true, toutes étapes refaites",
            payload == expected and expected["steps"] == len(STEPS) + 2,
            payload,
        )
        # Chaque analyse a écrit sa sortie sous `replay/` (GeoPackage non vide), isochrones compris.
        missing = [
            "%d-%s.gpkg" % (n, cid)
            for n, cid in state["analysis_steps"]
            if not os.path.isfile(os.path.join(cassini_dir, "replay", "%d-%s.gpkg" % (n, cid)))
            or os.path.getsize(os.path.join(cassini_dir, "replay", "%d-%s.gpkg" % (n, cid))) == 0
        ]
        check("les sorties des analyses sont rejouées sous replay/", not missing, missing)
        replayed = os.path.join(cassini_dir, "replay", "%d-layout.export_pdf.pdf" % state["export_step"])
        ok = os.path.isfile(replayed)
        if ok:
            with open(replayed, "rb") as f:
                ok = f.read(4) == b"%PDF"
        check("le PDF rejoué commence par %PDF", ok)
        QTimer.singleShot(0, tamper)
    elif kind in (client.REPLAY_FAILED, client.FAILED, client.DIED):
        check("rejeu nominal", False, "%s %s" % (kind, payload))
        finish()


# -- phase 3 : un écart doit être vu -------------------------------------------
def tamper():
    path = os.path.join(cassini_dir, "journal.jsonl")
    rows = read_journal()
    state["buffer_step"] = [r["id"] for r in rows].index("analysis.buffer") + 1
    rows[state["buffer_step"] - 1]["result"]["feature_count"] = 5
    with open(path, "w", encoding="utf-8") as f:
        for row in rows:
            f.write(json.dumps(row, ensure_ascii=False) + "\n")
    # Nouveau départ propre : projet vidé (QGIS lâche les fichiers du rejeu), noyau relancé.
    state["client"].stop()
    project.clear()
    check("le projet rouvre encore", project.read(project_file))
    new_client(on_third)


def on_third(kind, payload):
    if kind == client.READY:
        check("le rejeu altéré part", state["client"].replay())
    elif kind == client.REPLAY_FAILED:
        check(
            "le rejeu s'arrête à l'étape d'analysis.buffer en la nommant",
            payload.get("code") == "replay.diverged"
            and payload.get("step") == state["buffer_step"]
            and "analysis.buffer" in payload.get("message", ""),
            payload,
        )
        finish()
    elif kind in (client.REPLAY_DONE, client.FAILED, client.DIED):
        check("le rejeu altéré est refusé", False, "%s %s" % (kind, payload))
        finish()


new_client(on_first)
QTimer.singleShot(150000, QCoreApplication.quit)
QCoreApplication.exec()

if state["client"] is not None:
    state["client"].stop()
check("le CSV d'origine est inchangé (empreinte)", sha256(csv_path) == csv_hash)
check("le réseau d'origine est inchangé (empreinte)", sha256(network_path) == network_hash)
ok = len(checks) >= 10 and all(v for _, v in checks)
shutil.rmtree(work, ignore_errors=True)
print("RÉSULTAT :", "OK" if ok else "ÉCHEC")
sys.exit(0 if ok else 1)
