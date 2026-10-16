"""Sonde manuelle sous le vrai QGIS : `layout.export_pdf`.

À lancer avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`) :

    "C:\\Program Files\\QGIS 3.44.15\\bin\\python-qgis-ltr.bat" qgis-plugin/tests/probe_layout_export.py

Ouvre `../donnees-test-cassini/projet-test-villeneuve.qgz`, exporte la carte, puis vérifie : le PDF
commence par `%PDF`, les six éléments (carte, titre, légende, échelle, flèche du nord, sources) sont
posés, le projet n'a gagné aucune mise en page ; puis un projet vide est refusé (`layout.empty`)
sans fichier. Le jeu de données vit hors du dépôt (voir CLAUDE.md §5).
"""

import os
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "qgis-plugin", "cassini_qgis"))
PROJECT = os.path.normpath(os.path.join(ROOT, "..", "donnees-test-cassini", "projet-test-villeneuve.qgz"))

from qgis.core import QgsApplication, QgsProject  # noqa: E402
from qgis.PyQt.QtCore import QCoreApplication, QTimer  # noqa: E402

app = QgsApplication([], False)
QgsApplication.initQgis()
import layout_export  # noqa: E402

project = QgsProject.instance()
assert project.read(PROJECT), "projet illisible"
out_dir = tempfile.mkdtemp()
out_ok = os.path.join(out_dir, "1-layout.export_pdf.pdf")
out_empty = os.path.join(out_dir, "2-layout.export_pdf.pdf")
layouts_before = len(project.layoutManager().layouts())
results = []
gestures = []


def step_empty():
    root = project.layerTreeRoot()
    saved = [(n, n.itemVisibilityChecked()) for n in root.findLayers()]
    for n, _ in saved:
        n.setItemVisibilityChecked(False)

    def done(result=None, error=None):
        for n, v in saved:
            n.setItemVisibilityChecked(v)
        results.append(("projet masqué", error.code if error else result, error is not None and error.code == "layout.empty" and not os.path.exists(out_empty)))
        QCoreApplication.quit()

    gestures.append(layout_export.LayoutExportGesture())
    gestures[-1].start({"title": "Vide", "out": out_empty}, done)


def done_ok(result=None, error=None):
    ok = error is None and result["page_count"] == 1 and result["layer_count"] >= 1
    if ok:
        with open(out_ok, "rb") as f:
            ok = f.read(4) == b"%PDF" and os.path.getsize(out_ok) > 1000
        ok = ok and set(gestures[0].items_placed) == set(layout_export.EXPECTED_ITEMS)
        ok = ok and len(project.layoutManager().layouts()) == layouts_before
    results.append(("export", result if error is None else error.code, ok))
    QTimer.singleShot(0, step_empty)


gestures.append(layout_export.LayoutExportGesture())
QTimer.singleShot(0, lambda: gestures[0].start({"title": "Villeneuve-le-Roi : essai", "out": out_ok}, done_ok))
QTimer.singleShot(90000, QCoreApplication.quit)
QCoreApplication.exec()

print("PDF :", out_ok)
for row in results:
    print(row)
ok = len(results) == 2 and all(r[2] for r in results)
print("RÉSULTAT :", "OK" if ok else "ÉCHEC")
sys.exit(0 if ok else 1)
