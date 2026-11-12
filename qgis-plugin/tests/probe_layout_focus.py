"""Sonde manuelle sous le vrai QGIS : le résultat d'un plan mis en évidence dans le PDF (entrée 7.9).

À lancer avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`) :

    "C:\\Program Files\\QGIS 3.44.15\\bin\\python-qgis-ltr.bat" qgis-plugin/tests/probe_layout_focus.py

Aucun noyau ni aucune donnée hors dépôt : un projet en mémoire (Lambert-93) avec des parcelles et
des routes de l'utilisateur, deux couches intermédiaires, un résultat, et le MNT de test
`fixtures/mnt-plan-incline-2154.tif` au rendu gris par défaut de QGIS. Le geste reçoit `focus`
comme le noyau le lui donne. Contrôles, relevés sur la mise en page juste avant l'export :

1. la carte dessine le résultat en premier, par un clone en mémoire au style magenta du noyau,
   absent du projet ; les intermédiaires ne sont ni sur la carte, ni dans la légende, ni dans les
   sources ;
2. la légende commence par le résultat, nommé par `label` ;
3. l'intitulé de bande du MNT (« Band 1 (Gray) ») a quitté la légende, sa rampe reste ;
4. la carte est cadrée sur le résultat (marge de 5 %) ;
5. après l'export, les couches du projet ont gardé nom, couleur, visibilité et place ; le projet
   n'a gagné ni couche ni mise en page ;
6. sans `focus`, la légende et la carte sont celles d'avant (intermédiaires compris, nom de
   bande gardé) ;
7. un résultat raster (une copie du MNT, comme une pente ouverte) est posé juste au-dessus du MNT
   de l'utilisateur, sous ses vecteurs, nommé, sans restylage ;
8. un résultat vecteur qu'un `layer.set_style` antérieur a titré et borné en échelle est dessiné et
   nommé par `label` : le clone ne garde ni le titre ni le seuil d'échelle.

Une image PNG de chaque mise en page est écrite à côté des PDF, pour la regarder.
"""

import os
import sys
import tempfile
import traceback

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "qgis-plugin", "cassini_qgis"))
DEM = os.path.join(ROOT, "fixtures", "mnt-plan-incline-2154.tif")

from qgis.core import (  # noqa: E402
    QgsApplication,
    QgsCoordinateReferenceSystem,
    QgsFeature,
    QgsGeometry,
    QgsLayoutExporter,
    QgsLayoutItemLegend,
    QgsLayoutItemMap,
    QgsProject,
    QgsRasterLayer,
    QgsVectorLayer,
)
from qgis.PyQt.QtCore import QCoreApplication, Qt, QTimer  # noqa: E402

app = QgsApplication([], False)
QgsApplication.initQgis()
import layout_export  # noqa: E402

project = QgsProject.instance()
project.setCrs(QgsCoordinateReferenceSystem("EPSG:2154"))
work = tempfile.mkdtemp(prefix="cassini-focus-")


def polygons(name, boxes, color):
    layer = QgsVectorLayer("Polygon?crs=EPSG:2154&field=id:integer", name, "memory")
    feats = []
    for i, (x0, y0, x1, y1) in enumerate(boxes):
        f = QgsFeature(layer.fields())
        f.setAttributes([i])
        f.setGeometry(QgsGeometry.fromWkt("POLYGON((%f %f,%f %f,%f %f,%f %f,%f %f))" % (
            x0, y0, x1, y0, x1, y1, x0, y1, x0, y0)))
        feats.append(f)
    layer.dataProvider().addFeatures(feats)
    layer.updateExtents()
    layer.renderer().symbol().setColor(color)
    return layer


def lines(name, a, b):
    layer = QgsVectorLayer("LineString?crs=EPSG:2154", name, "memory")
    f = QgsFeature()
    f.setGeometry(QgsGeometry.fromWkt("LINESTRING(%f %f, %f %f)" % (a[0], a[1], b[0], b[1])))
    layer.dataProvider().addFeatures([f])
    layer.updateExtents()
    return layer


from qgis.PyQt.QtGui import QColor  # noqa: E402

# Le MNT de test couvre une petite zone ; les données vectorielles sont posées autour de lui.
# Copie du MNT dans le dossier temporaire : QGIS écrit ses statistiques (`.aux.xml`) à côté.
import shutil  # noqa: E402

dem_copy = os.path.join(work, "mnt-plan-incline-2154.tif")
shutil.copy(DEM, dem_copy)
dem = QgsRasterLayer(dem_copy, "mnt")
assert dem.isValid(), "MNT de test illisible"
ext = dem.extent()
x0, y0 = ext.xMinimum(), ext.yMinimum()
w, h = ext.width(), ext.height()
parcelles = polygons("parcelles", [(x0 + i * w / 8, y0, x0 + (i + 1) * w / 8 - 1, y0 + h) for i in range(8)],
                     QColor("#d9d9d9"))
routes = lines("routes", (x0, y0), (x0 + w, y0 + h))
tampon = polygons("2-analysis.buffer", [(x0, y0, x0 + w / 2, y0 + h / 2)], QColor("#ff7f00"))
inter = polygons("3-analysis.intersect", [(x0, y0, x0 + w / 3, y0 + h / 3)], QColor("#ff7f00"))
result = polygons("4-analysis.intersect", [(x0 + w / 10, y0 + h / 10, x0 + w / 4, y0 + h / 5)], QColor("#ff7f00"))
slope_copy = os.path.join(work, "6-terrain.slope.tif")
shutil.copy(DEM, slope_copy)
slope = QgsRasterLayer(slope_copy, "6-terrain.slope")
assert slope.isValid()
for layer in (dem, parcelles, routes, tampon, inter, result):
    project.addMapLayer(layer)


def snapshot():
    tree = [n.layerId() for n in project.layerTreeRoot().findLayers()]
    rows = []
    for layer in project.mapLayers().values():
        color = layer.renderer().symbol().color().name() if isinstance(layer, QgsVectorLayer) else None
        node = project.layerTreeRoot().findLayer(layer.id())
        rows.append((layer.id(), layer.name(), color, node.isVisible()))
    return sorted(rows), tree, len(project.layoutManager().layouts())


before = snapshot()
seen = {}  # relevés de la mise en page de chaque export, par fichier
_export_to_pdf = QgsLayoutExporter.exportToPdf


def _spy_export(self, path, settings):
    layout = self.layout()
    row = {}
    for item in layout.items():
        if isinstance(item, QgsLayoutItemMap) and item.layers() and item.layers()[0].name() != "vide":
            row["map_layers"] = [(l.name(), l.id(), project.mapLayer(l.id()) is not None) for l in item.layers()]
            first = item.layers()[0]
            row["first_color"] = (first.renderer().symbol().color().name()
                                  if isinstance(first, QgsVectorLayer) else None)
            e = item.extent()
            row["extent"] = (e.xMinimum(), e.yMinimum(), e.xMaximum(), e.yMaximum())
        if isinstance(item, QgsLayoutItemLegend):
            model = item.model()
            nodes = []
            for node in model.rootGroup().findLayers():
                labels = [n.data(Qt.ItemDataRole.DisplayRole) for n in model.layerLegendNodes(node)]
                nodes.append((node.name(), node.layer().name(), labels))
            row["legend"] = nodes
    self.exportToImage(path[:-4] + ".png", QgsLayoutExporter.ImageExportSettings())
    seen[path] = row
    return _export_to_pdf(self, path, settings)


QgsLayoutExporter.exportToPdf = _spy_export

LABEL = "Parcelles constructibles à moins de 500 m d'une gare"
FOCUS = {
    "results": [{"layer_id": result.id(), "label": LABEL,
                 "style": {"kind": "single", "fill": "#c2187a", "stroke": "#3d0026", "width_mm": 0.5}}],
    "hidden": [tampon.id(), inter.id()], "frame": [result.id()], "framed_on": "results",
    "hide_band_labels": True,
}
out_focus = os.path.join(work, "5-layout.export_pdf.pdf")
out_plain = os.path.join(work, "6-layout.export_pdf.pdf")
out_raster = os.path.join(work, "7-layout.export_pdf.pdf")
RASTER_LABEL = "Pentes du secteur"
results = []
checks = []
gestures = []


def check(name, ok, detail=None):
    checks.append((name, bool(ok), detail))


def finish():
    try:
        f = seen.get(out_focus, {})
        p = seen.get(out_plain, {})
        res_focus, res_plain = results[0], results[1]
        check("export avec focus : pas d'erreur", res_focus[1] is None, res_focus)
        check("export sans focus : pas d'erreur", res_plain[1] is None, res_plain)
        layers = f.get("map_layers", [])
        check("carte : le résultat en premier", layers and layers[0][0] == result.name(), layers)
        check("carte : le résultat est un clone hors du projet",
              layers and layers[0][1] != result.id() and not layers[0][2], layers)
        check("carte : clone au magenta du noyau", f.get("first_color") == "#c2187a", f.get("first_color"))
        names = [n for n, _, _ in layers]
        check("carte : intermédiaires absents", tampon.name() not in names and inter.name() not in names, names)
        legend = f.get("legend", [])
        check("légende : le résultat en tête, nommé", legend and legend[0][0] == LABEL, legend)
        check("légende : intermédiaires absents",
              all(layer not in (tampon.name(), inter.name()) for _, layer, _ in legend), legend)
        dem_rows = [labels for _, layer, labels in legend if layer == "mnt"]
        check("légende : MNT sans intitulé de bande",
              dem_rows and not any("Band" in str(t) or "Bande" in str(t) for t in dem_rows[0]), dem_rows)
        check("légende : MNT garde sa rampe", dem_rows and len(dem_rows[0]) >= 1, dem_rows)
        sources = res_focus[0]["sources"] if res_focus[0] else []
        check("sources : sans fichier intermédiaire", res_focus[0] and res_focus[0]["layer_count"] == 4, res_focus)
        e = f.get("extent")
        rb = result.extent()
        # Cadré sur le résultat : le cadre le contient et ne dépasse pas trois fois sa taille (la
        # page a un autre rapport que le résultat).
        framed = e is not None and e[0] <= rb.xMinimum() and e[2] >= rb.xMaximum() and \
            e[1] <= rb.yMinimum() and e[3] >= rb.yMaximum() and (e[2] - e[0]) < 3 * max(rb.width(), rb.height())
        check("carte : cadrée sur le résultat", framed, (e, rb.toString()))

        plain_names = [n for n, _, _ in p.get("map_layers", [])]
        check("sans focus : intermédiaires dessinés", tampon.name() in plain_names, plain_names)
        plain_dem = [labels for _, layer, labels in p.get("legend", []) if layer == "mnt"]
        check("sans focus : la légende du MNT est celle d'avant", plain_dem, plain_dem)
        r = seen.get(out_raster, {})
        res_raster = results[2]
        check("export raster et résultat restylé : pas d'erreur", res_raster[1] is None, res_raster)
        rnames = [n for n, _, _ in r.get("map_layers", [])]
        check("raster : au-dessus du MNT, sous les vecteurs",
              rnames[-2:] == [slope.name(), dem.name()] and rnames[0] == result.name(), rnames)
        rlegend = r.get("legend", [])
        check("raster : nommé dans la légende", any(n == RASTER_LABEL for n, _, _ in rlegend), rlegend)
        check("résultat restylé : nommé par label, pas par l'ancien titre",
              rlegend and rlegend[0][0] == LABEL and not any("Ancien" in str(x) for x in rlegend), rlegend)
        check("résultat restylé : dessiné malgré l'ancien seuil d'échelle",
              r.get("first_color") == "#c2187a", r.get("first_color"))
        check("résultat restylé : la couche du projet garde titre et seuil",
              result.customProperty("cassini/legend_title") == "Ancien titre" and result.hasScaleBasedVisibility())
        print("légende raster :", rlegend)
        print("sources :", sources)
        print("légende avec focus :", legend)
        print("légende sans focus :", p.get("legend"))
    except Exception:
        traceback.print_exc()
        check("relevés lisibles", False)
    QCoreApplication.quit()


def before_raster():
    # Les contrôles du projet inchangé portent sur les deux premiers exports.
    checks.append(("projet inchangé après les deux exports", snapshot() == before, None))
    try:
        step_raster()
    except Exception:
        traceback.print_exc()
        QCoreApplication.quit()


def step_plain():
    gestures.append(layout_export.LayoutExportGesture())

    def done(result=None, error=None):
        results.append((result, error and error.code))
        QTimer.singleShot(0, before_raster)

    gestures[-1].start({"title": "Sans mise en évidence", "out": out_plain}, done)


def step_raster():
    # Un `layer.set_style` antérieur a titré le résultat et borné son échelle (au-delà du
    # 1:1, il ne serait plus dessiné) ; puis un résultat raster ouvert au projet.
    result.setCustomProperty("cassini/legend_title", "Ancien titre")
    result.setMinimumScale(1.0)
    result.setMaximumScale(0.0)
    result.setScaleBasedVisibility(True)
    project.addMapLayer(slope)
    project.layerTreeRoot().findLayer(slope.id())  # posé en tête de l'arbre par QGIS
    gestures.append(layout_export.LayoutExportGesture())
    focus = dict(FOCUS, results=FOCUS["results"] + [{"layer_id": slope.id(), "label": RASTER_LABEL}])

    def done(**answer):
        error = answer.get("error")
        results.append((answer.get("result"), error and (error.code, error.message)))
        QTimer.singleShot(0, finish)

    gestures[-1].start({"title": "Avec un raster", "out": out_raster, "focus": focus}, done)


def step_focus():
    gestures.append(layout_export.LayoutExportGesture())

    def done(result=None, error=None):
        results.append((result, error and (error.code, error.message)))
        QTimer.singleShot(0, step_plain)

    gestures[-1].start({"title": LABEL + " à Annecy", "out": out_focus, "focus": FOCUS}, done)


QTimer.singleShot(0, step_focus)
QTimer.singleShot(120000, QCoreApplication.quit)
QCoreApplication.exec()

print("PDF :", out_focus, out_plain)
for name, ok, detail in checks:
    print("OK  " if ok else "ÉCHEC", name, "" if ok else detail)
ok = len(checks) >= 21 and all(c[1] for c in checks)
print("RÉSULTAT :", "OK" if ok else "ÉCHEC")
sys.exit(0 if ok else 1)
