"""Sonde manuelle sous le vrai QGIS : un raster du noyau s'ouvre dans le projet (ticket 4.3).

À lancer avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`),
plateforme offscreen, après `cargo build --release` :

    set QT_QPA_PLATFORM=offscreen
    "C:\\Program Files\\QGIS 3.44.15\\bin\\python-qgis-ltr.bat" qgis-plugin/tests/probe_raster_output.py

Le noyau est cherché dans `CASSINI_CORE`, sinon `target/release/`, sinon `target/debug/`.

Dans un projet temporaire, une copie de `fixtures/mnt-plan-incline-2154.tif` sous `dem/` ;
`cassini run terrain.slope` y écrit la pente et rend son chemin absolu. `open_rasters`
(le vrai `QgisBackend`) l'ajoute au projet : une couche raster valide nommée
`1-terrain.slope`, une seule même reçue deux fois. Un GeoTIFF corrompu n'ajoute rien et ne
lève pas. L'inventaire du projet donne au MNT `dem/plan-incline-2154.tif` et à la pente
`p.cassini/1-terrain.slope.tif`, jamais de chemin absolu. Aucune donnée hors dépôt.
"""

import json
import os
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "qgis-plugin", "cassini_qgis"))
FIXTURE = os.path.join(ROOT, "fixtures", "mnt-plan-incline-2154.tif")

from qgis.core import QgsApplication, QgsProject, QgsRasterLayer  # noqa: E402

app = QgsApplication([], False)
QgsApplication.initQgis()
import client  # noqa: E402
import inventory  # noqa: E402
import raster_output  # noqa: E402

FAILURES = []
CATALOGUE = [{"id": "terrain.slope", "runs_in": "core"}]


def check(label, got, expected):
    ok = got == expected
    print("%s %s : %r" % ("OK  " if ok else "ÉCHEC", label, got))
    if not ok:
        FAILURES.append(label)
        print("     attendu : %r" % (expected,))


def find_core():
    candidates = [os.environ.get("CASSINI_CORE")]
    for profile in ("release", "debug"):
        candidates.append(os.path.join(ROOT, "target", profile, "cassini.exe"))
        candidates.append(os.path.join(ROOT, "target", profile, "cassini"))
    for path in candidates:
        if path and os.path.isfile(path):
            return path
    sys.exit("Noyau introuvable : lancez `cargo build --release` ou posez CASSINI_CORE.")


def inventory_now():
    calls = []
    gesture = inventory.ProjectInventoryGesture(None, lambda fn: fn())
    gesture.start({}, lambda result=None, error=None: calls.append((result, error)))
    return calls[0]


def main():
    core = find_core()
    work = tempfile.mkdtemp(prefix="cassini-sonde-raster-")
    try:
        os.makedirs(os.path.join(work, "dem"))
        dem = os.path.join(work, "dem", "plan-incline-2154.tif")
        shutil.copyfile(FIXTURE, dem)
        project_path = os.path.join(work, "p.qgz")
        project = QgsProject.instance()
        mnt = QgsRasterLayer(dem, "MNT", "gdal")
        check("MNT valide", mnt.isValid(), True)
        project.addMapLayer(mnt)
        check("projet enregistré", project.write(project_path), True)

        run = subprocess.run(
            [core, "run", "terrain.slope",
             json.dumps({"dem": "dem/plan-incline-2154.tif", "unit": "degrees"}),
             "--project", project_path],
            capture_output=True, text=True, timeout=120,
        )
        check("cassini run : code", run.returncode, 0)
        if run.returncode != 0:
            print(run.stderr)
            return
        result = json.loads(run.stdout)
        path = result.get("file")
        check("file absolu", isinstance(path, str) and os.path.isabs(path), True)

        payload = {"command": "terrain.slope", "result": result}
        items = raster_output.to_open(client.RUN_DONE, payload, CATALOGUE)
        check("à ouvrir", [name for _, name in items], ["1-terrain.slope"])
        outcomes = raster_output.open_rasters(items)
        check("ouverture", [o for _, o in outcomes], [raster_output.ADDED])
        layers = project.mapLayersByName("1-terrain.slope")
        check("une couche 1-terrain.slope", len(layers), 1)
        if layers:
            check("couche raster", isinstance(layers[0], QgsRasterLayer), True)
            check("couche valide", layers[0].isValid(), True)
        again = raster_output.open_rasters(items)
        check("reçue deux fois", [o for _, o in again], [raster_output.ALREADY_OPEN])
        check("toujours une couche", len(project.mapLayersByName("1-terrain.slope")), 1)

        broken = os.path.join(work, "p.cassini", "9-terrain.slope.tif")
        with open(broken, "wb") as f:
            f.write(b"II*\x00pas un GeoTIFF")
        bad = raster_output.open_rasters([(broken, "9-terrain.slope")])
        check("GeoTIFF corrompu", [o for _, o in bad], [raster_output.UNREADABLE])
        check("rien d'ajouté", len(project.mapLayersByName("9-terrain.slope")), 0)

        result, error = inventory_now()
        check("inventaire sans erreur", error, None)
        sources = {layer["name"]: layer["source"] for layer in (result or {}).get("layers", [])}
        check("source du MNT", sources.get("MNT"), "dem/plan-incline-2154.tif")
        check("source de la pente", sources.get("1-terrain.slope"), "p.cassini/1-terrain.slope.tif")
        text = json.dumps(result)
        check("aucun chemin absolu", work.replace("\\", "/") in text.replace("\\\\", "/"), False)
        project.clear()
    finally:
        shutil.rmtree(work, ignore_errors=True)


try:
    main()
except Exception:
    import traceback

    traceback.print_exc()
    FAILURES.append("exception")
finally:
    QgsApplication.exitQgis()
print("RÉSULTAT :", "ÉCHEC (" + ", ".join(FAILURES) + ")" if FAILURES else "OK")
sys.exit(1 if FAILURES else 0)
