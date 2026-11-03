"""Geste `layout.export_pdf` : la carte du projet en PDF par la mise en page de QGIS.

Le noyau décide ; ce module exécute. Il monte en mémoire une mise en page A4 paysage
(carte, titre, légende, barre d'échelle, flèche du nord, sources), l'exporte dans le
fichier `out` nommé par le noyau (AD-10) puis la jette : rien n'est ajouté au projet,
aucune couche n'est modifiée. La carte montre les couches visibles du projet, cadrée sur
leur emprise réunie dans le SCR du projet, sauf celles que le noyau nomme dans `leave_out`
(au rejeu seulement). Les sources sont des noms de fichiers
(`source_name`), jamais un chemin absolu ni une URI à identifiants.

La mise en page de QGIS ne tourne pas hors du fil principal : l'export est synchrone
(commande non annulable). Accès à QGIS par un « fond » injectable ; `LayoutBackend` est
le seul endroit qui touche `qgis.core`, les tests mettent une doublure.
"""

import os

try:  # importés à plat par les tests, en paquet par QGIS
    from .gestures import GestureError
    from .inventory import QgisBackend, _ReadGesture, _union, source_name
except ImportError:  # pragma: no cover
    from gestures import GestureError
    from inventory import QgisBackend, _ReadGesture, _union, source_name

BAD_PARAMS = "layout.bad_params"
EMPTY = "layout.empty"
EXPORT_FAILED = "layout.export_failed"

MAX_SOURCES = 50  # borne du schéma de sortie du noyau (AD-8)

# Éléments que la mise en page doit porter (rendus par le fond, vérifiés par les tests).
EXPECTED_ITEMS = ("map", "title", "legend", "scalebar", "northarrow", "sources")


class LayoutBackend(QgisBackend):
    """Accès réel à QGIS pour la mise en page. Seul ce type touche `qgis.core`."""

    def visible_layers(self):
        """Couches cochées dans la légende (et dont tous les groupes le sont), de haut en bas."""
        from qgis.core import QgsProject

        found = []
        for node in QgsProject.instance().layerTreeRoot().findLayers():
            layer = node.layer()
            if layer is not None and node.isVisible():
                found.append(layer)
        return found

    def export_pdf(self, layers, title, sources_text, out, authid, box):
        """Monte la mise en page, l'exporte dans `out`, rend les éléments posés (clés de
        `EXPECTED_ITEMS`). Lève `RuntimeError` si l'export échoue."""
        from qgis.core import (
            Qgis,
            QgsCoordinateReferenceSystem,
            QgsLayout,
            QgsLayoutExporter,
            QgsLayoutItemLabel,
            QgsLayoutItemLegend,
            QgsLayoutItemMap,
            QgsLayoutItemPage,
            QgsLayoutItemPicture,
            QgsLayoutItemScaleBar,
            QgsLayoutPoint,
            QgsLayoutSize,
            QgsProject,
            QgsRectangle,
        )
        from qgis.PyQt.QtGui import QFont

        layout = QgsLayout(QgsProject.instance())  # jamais ajoutée au gestionnaire du projet
        layout.initializeDefaults()
        layout.pageCollection().page(0).setPageSize("A4", QgsLayoutItemPage.Orientation.Landscape)

        def place(item, x, y, w, h):
            item.attemptMove(QgsLayoutPoint(x, y))
            item.attemptResize(QgsLayoutSize(w, h))
            layout.addLayoutItem(item)
            return item

        def label(text, x, y, w, h, size, bold=False):
            item = QgsLayoutItemLabel(layout)
            item.setText(text)
            font = QFont()
            font.setPointSize(size)
            font.setBold(bold)
            item.setFont(font)
            return place(item, x, y, w, h)

        # Carte : couches visibles seules, cadrée sur leur emprise (zoomToExtent garde la taille).
        carte = QgsLayoutItemMap(layout)
        carte.setCrs(QgsCoordinateReferenceSystem(authid))
        carte.setLayers(layers)
        place(carte, 10, 22, 200, 160)
        carte.zoomToExtent(QgsRectangle(*box))

        title_label = label(title, 10, 6, 277, 14, 18, bold=True)

        legend = QgsLayoutItemLegend(layout)
        legend.setLinkedMap(carte)
        legend.setAutoUpdateModel(False)  # sinon la légende reprend aussi les couches masquées
        root = legend.model().rootGroup()
        root.clear()
        for layer in layers:
            root.addLayer(layer)
        place(legend, 215, 22, 72, 100)

        scale = QgsLayoutItemScaleBar(layout)
        scale.setLinkedMap(carte)
        scale.applyDefaultSize()
        # Graduations en km dès que le segment fait 1 000 m : « 0 1 2 km » ne se chevauche pas.
        if scale.units() == Qgis.DistanceUnit.Meters and scale.unitsPerSegment() >= 1000:
            scale.setUnitsPerSegment(scale.unitsPerSegment() / 1000.0)
            scale.setUnits(Qgis.DistanceUnit.Kilometers)
            scale.setUnitLabel("km")
        scale_font = QFont()
        scale_font.setPointSize(8)
        scale.setFont(scale_font)
        place(scale, 10, 185, 80, 12)

        arrow = QgsLayoutItemPicture(layout)
        arrow.setPicturePath(":/images/north_arrows/layout_default_north_arrow.svg")
        arrow.setLinkedMap(carte)
        arrow.setNorthMode(QgsLayoutItemPicture.NorthMode.GridNorth)
        place(arrow, 215, 130, 20, 20)

        sources_label = label(sources_text, 10, 198, 277, 10, 8)

        # Relu depuis la mise en page elle-même : un élément absent ne peut pas être annoncé.
        by_type = {type(i): i for i in layout.items()}
        present = {
            "map": QgsLayoutItemMap in by_type,
            "title": title_label in layout.items() and bool(title_label.text()),
            "legend": QgsLayoutItemLegend in by_type,
            "scalebar": QgsLayoutItemScaleBar in by_type,
            "northarrow": QgsLayoutItemPicture in by_type and bool(arrow.evaluatedPath()),
            "sources": sources_label in layout.items() and bool(sources_label.text()),
        }
        placed = [key for key in EXPECTED_ITEMS if present[key]]

        result = QgsLayoutExporter(layout).exportToPdf(out, QgsLayoutExporter.PdfExportSettings())
        if result != QgsLayoutExporter.ExportResult.Success:
            raise RuntimeError("export PDF refusé par QGIS (code %s)" % result)
        return placed


def _padded(box):
    """Marge de 5 % autour de l'emprise ; une emprise réduite à un point reçoit une marge fixe."""
    xmin, ymin, xmax, ymax = box
    pad = max(xmax - xmin, ymax - ymin) * 0.05 or 1.0
    return (xmin - pad, ymin - pad, xmax + pad, ymax + pad)


class LayoutExportGesture(_ReadGesture):
    failure = "Échec inattendu de l'export PDF."

    def __init__(self, backend=None, schedule=None):
        super().__init__(backend if backend is not None else LayoutBackend(), schedule)
        self.items_placed = []  # éléments posés par le dernier export (tests et sonde)

    def _read(self):
        params = self._params or {}
        title = params.get("title")
        if not isinstance(title, str) or not title.strip():
            raise GestureError(BAD_PARAMS, "Le titre de la carte ne doit pas être vide.")
        out = params.get("out")
        if not isinstance(out, str) or not out:
            raise GestureError(BAD_PARAMS, "Le fichier de sortie n'est pas fourni.")
        leave_out = params.get("leave_out", [])
        if not isinstance(leave_out, list) or not all(isinstance(i, str) for i in leave_out):
            raise GestureError(BAD_PARAMS, "La liste des couches à laisser hors de la carte est invalide.")
        backend = self._backend
        # Au rejeu, le noyau nomme les couches posées par une exécution précédente du même
        # journal : elles restent dans le projet, intactes, mais hors de la carte (ticket 5.36).
        leave_out = set(leave_out)
        layers = [layer for layer in backend.visible_layers() if backend.id_of(layer) not in leave_out]
        if not layers:
            raise GestureError(EMPTY, "Aucune couche visible : il n'y a rien à mettre sur la carte.")
        crs = backend.project_crs()
        box = _union([backend.extent_of(layer, crs) for layer in layers])
        if box is None:
            raise GestureError(EMPTY, "Les couches visibles sont vides : la carte n'a pas d'emprise.")

        names = []
        for layer in layers:
            name = source_name(backend.source_of(layer))
            if name is not None and name not in names:
                names.append(name)
        sources_text = "Sources : " + (", ".join(names) if names else "couches du projet (hors fichier)")

        try:
            self.items_placed = list(
                backend.export_pdf(layers, title.strip(), sources_text, out, crs, _padded(box))
            )
            if not os.path.isfile(out):
                raise RuntimeError("fichier absent après l'export")
        except Exception:
            self._discard(out)
            raise GestureError(EXPORT_FAILED, "L'export PDF a échoué : aucun fichier n'a été écrit.")
        return {
            "file": out,
            "page_count": 1,
            "layer_count": len(layers),
            "sources": names[:MAX_SOURCES],
            "truncated": len(names) > MAX_SOURCES,
        }

    @staticmethod
    def _discard(path):
        try:
            if os.path.exists(path):
                os.remove(path)
        except OSError:  # pragma: no cover
            pass
