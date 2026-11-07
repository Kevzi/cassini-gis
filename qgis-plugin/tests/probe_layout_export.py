"""Sonde manuelle sous le vrai QGIS : `layout.export_pdf`.

À lancer avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`) :

    "C:\\Program Files\\QGIS 3.44.15\\bin\\python-qgis-ltr.bat" qgis-plugin/tests/probe_layout_export.py

Ouvre `../donnees-test-cassini/projet-test-villeneuve.qgz`, exporte la carte, puis vérifie : le PDF
commence par `%PDF`, les six éléments (carte, titre, légende, échelle, flèche du nord, sources) sont
posés, le projet n'a gagné aucune mise en page ; puis un export à `scale` 25000 rend `scale: 25000`
(échelle relue sur la carte de mise en page, centrée sur les couches : pas de carte QGIS ici)
et `center` dans le système du projet ; puis ce centre, reprojeté en EPSG:4326 par le vrai QGIS
(`reproject_point`) et repassé comme au rejeu, est ramené dans le système du projet à moins d'un
centimètre près, l'échelle relue valant encore 25000 ; puis un centre à 500 km des données donne
un cadre vide, refusé (`layout.empty`) sans fichier ; puis un export A3 paysage avec un carroyage
UTM 31 N (EPSG:32631) au kilomètre a une boîte de page de 1190,55 × 841,89 pt (± 1 pt), et sa
carte de mise en page porte une grille active de ce système et de cet intervalle (le premier
export, sans les nouveaux paramètres, garde 841,89 × 595,28 pt et aucune grille) ; puis un
projet vide est refusé (`layout.empty`) sans fichier. Le jeu de données vit hors du dépôt (voir CLAUDE.md §5).
"""

import os
import re
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
out_scale = os.path.join(out_dir, "3-layout.export_pdf.pdf")
out_wgs84 = os.path.join(out_dir, "4-layout.export_pdf.pdf")
out_far = os.path.join(out_dir, "5-layout.export_pdf.pdf")
out_a3 = os.path.join(out_dir, "6-layout.export_pdf.pdf")
layouts_before = len(project.layoutManager().layouts())
results = []
gestures = []
scaled = {}
grids = []  # (système, intervalle X, intervalle Y, active) des grilles de chaque export


def media_box(path):
    """(largeur, hauteur) en points de la première boîte de page du PDF."""
    with open(path, "rb") as f:
        data = f.read()
    m = re.search(rb"/MediaBox\s*\[\s*([-\d.]+)\s+([-\d.]+)\s+([-\d.]+)\s+([-\d.]+)\s*\]", data)
    if m is None:
        return None
    x0, y0, x1, y1 = (float(v) for v in m.groups())
    return (x1 - x0, y1 - y0)


def close_to(box, want):
    return box is not None and abs(box[0] - want[0]) <= 1 and abs(box[1] - want[1]) <= 1


# Les grilles posées par le fond, relevées au moment de l'export (la mise en page est jetée après).
from qgis.core import QgsLayoutExporter, QgsLayoutItemMap  # noqa: E402

_export_to_pdf = QgsLayoutExporter.exportToPdf
grid_maps = []  # (fond, entités des couches, Y à gauche, X en bas) des cartes qui portent une grille


def _spy_export(self, *args, **kwargs):
    for item in self.layout().items():
        if isinstance(item, QgsLayoutItemMap):
            for g in item.grids().asList():
                grids.append((g.crs().authid(), g.intervalX(), g.intervalY(), g.enabled()))
                # Carte du carroyage : sans fond, une seule couche (vide), Y à gauche, X en bas.
                Side, Mode = g.BorderSide, g.DisplayMode
                grid_maps.append((
                    item.hasBackground(),
                    [l.featureCount() for l in item.layers()],
                    g.annotationDisplay(Side.Left) == Mode.LatitudeOnly,
                    g.annotationDisplay(Side.Bottom) == Mode.LongitudeOnly,
                ))
    return _export_to_pdf(self, *args, **kwargs)


QgsLayoutExporter.exportToPdf = _spy_export


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


def step_a3():
    grids.clear()
    grid_maps.clear()
    grid = {"kind": "projected", "epsg": 32631, "interval": {"value": 1, "unit": "km"}}

    def done(result=None, error=None):
        box = media_box(out_a3) if error is None and os.path.isfile(out_a3) else None
        ok = error is None and close_to(box, (1190.55, 841.89))
        ok = ok and grids == [("EPSG:32631", 1000.0, 1000.0, True)]
        ok = ok and grid_maps == [(False, [0], True, True)]
        ok = ok and "grid" not in result and "page_size" not in result
        results.append(("A3 paysage, carroyage UTM 1 km", (box, grids[:]) if error is None else error.code, ok))
        QTimer.singleShot(0, step_empty)

    gestures.append(layout_export.LayoutExportGesture())
    gestures[-1].start({"title": "Villeneuve-le-Roi, A3 carroyé", "out": out_a3, "page_size": "a3", "grid": grid}, done)


def step_far():
    # Centre à 500 km à l'est des données, dans le système du projet : aucune couche dans le cadre.
    c = scaled["center"]
    far = {"x": c["x"] + 500000.0, "y": c["y"], "epsg": c["epsg"]}

    def done(result=None, error=None):
        ok = error is not None and error.code == "layout.empty" and not os.path.exists(out_far)
        results.append(("cadre vide", error.code if error else result, ok))
        QTimer.singleShot(0, step_a3)

    gestures.append(layout_export.LayoutExportGesture())
    gestures[-1].start({"title": "Loin", "out": out_far, "scale": 25000, "center": far}, done)


def step_wgs84():
    # Le centre rendu, passé en EPSG:4326 par le vrai QGIS, puis repassé comme au rejeu.
    c = scaled["center"]
    backend = layout_export.LayoutBackend()
    lon_lat = backend.reproject_point(c["x"], c["y"], "EPSG:%d" % c["epsg"], "EPSG:4326")
    if lon_lat is None:
        # Reprojection impossible : l'étape échoue, les suivantes tournent quand même.
        results.append(("centre en EPSG:4326 reprojeté", "reproject_point a rendu None", False))
        QTimer.singleShot(0, step_far)
        return
    given = {"x": lon_lat[0], "y": lon_lat[1], "epsg": 4326}

    def done(result=None, error=None):
        ok = error is None and result.get("scale") == 25000 and os.path.isfile(out_wgs84)
        if ok:
            back = result["center"]
            ok = (
                back["epsg"] == c["epsg"]
                and abs(back["x"] - c["x"]) < 0.01
                and abs(back["y"] - c["y"]) < 0.01
                and -180 <= lon_lat[0] <= 180
                and abs(lon_lat[0] - c["x"]) > 1000  # vraiment reprojeté
            )
        results.append(("centre en EPSG:4326 reprojeté", (given, result) if error is None else error.code, ok))
        QTimer.singleShot(0, step_far)

    gestures.append(layout_export.LayoutExportGesture())
    gestures[-1].start({"title": "Rejeu", "out": out_wgs84, "scale": 25000, "center": given}, done)


def step_scale():
    def done(result=None, error=None):
        ok = error is None and result.get("scale") == 25000 and os.path.isfile(out_scale)
        if ok:
            with open(out_scale, "rb") as f:
                ok = f.read(4) == b"%PDF"
            center = result.get("center") or {}
            ok = ok and center.get("epsg") == layout_export._epsg_of(project.crs().authid())
            scaled.update(result)
        results.append(("export au 1:25 000", result if error is None else error.code, ok))
        QTimer.singleShot(0, step_wgs84 if ok else step_empty)

    gestures.append(layout_export.LayoutExportGesture())
    gestures[-1].start({"title": "Villeneuve-le-Roi au 1:25 000", "out": out_scale, "scale": 25000}, done)


def done_ok(result=None, error=None):
    ok = error is None and result["page_count"] == 1 and result["layer_count"] >= 1
    if ok:
        with open(out_ok, "rb") as f:
            ok = f.read(4) == b"%PDF" and os.path.getsize(out_ok) > 1000
        ok = ok and set(gestures[0].items_placed) == set(layout_export.EXPECTED_ITEMS)
        ok = ok and len(project.layoutManager().layouts()) == layouts_before
        ok = ok and "scale" not in result and "center" not in result
        # Sans les nouveaux paramètres : A4 paysage, aucune grille.
        ok = ok and close_to(media_box(out_ok), (841.89, 595.28)) and grids == []
    results.append(("export", result if error is None else error.code, ok))
    QTimer.singleShot(0, step_scale)


gestures.append(layout_export.LayoutExportGesture())
QTimer.singleShot(0, lambda: gestures[0].start({"title": "Villeneuve-le-Roi : essai", "out": out_ok}, done_ok))
QTimer.singleShot(90000, QCoreApplication.quit)
QCoreApplication.exec()

print("PDF :", out_ok)
print("PDF au 1:25 000 :", out_scale)
print("PDF A3 carroyé :", out_a3)
for row in results:
    print(row)
ok = len(results) == 6 and all(r[2] for r in results)
print("RÉSULTAT :", "OK" if ok else "ÉCHEC")
sys.exit(0 if ok else 1)
