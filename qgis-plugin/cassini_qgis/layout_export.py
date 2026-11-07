"""Geste `layout.export_pdf` : la carte du projet en PDF par la mise en page de QGIS.

Le noyau décide ; ce module exécute. Il monte en mémoire une mise en page (A4 paysage par
défaut ; carte, titre, légende, barre d'échelle, flèche du nord, sources), l'exporte dans le
fichier `out` nommé par le noyau (AD-10) puis la jette : rien n'est ajouté au projet,
aucune couche n'est modifiée. La carte montre les couches visibles du projet, cadrée sur
leur emprise réunie dans le SCR du projet, sauf celles que le noyau nomme dans `leave_out`
(au rejeu seulement). Les sources sont des noms de fichiers
(`source_name`), jamais un chemin absolu ni une URI à identifiants.

Avec `scale` (dénominateur entier, 100 à 100 000 000, ticket 5.38), la carte de la mise en
page est tenue à 1:`scale`, centrée sur le centre de l'emprise que montre la carte de QGIS ;
sans carte (QGIS sans interface), sur le centre de l'emprise des couches visibles. Au rejeu,
le noyau passe `center` (le centre journalisé, un `Point` x, y, epsg) : il prime sur la carte
de QGIS, reprojeté dans le système du projet s'il le faut. Le résultat rend `scale` et
`center`, le centre employé, dans le système du projet (un projet sans code EPSG refuse
l'export à l'échelle). L'échelle est relue sur la carte de mise en page : si elle ne vaut pas
`scale`, aucun fichier n'est laissé (`layout.export_failed`) ; si aucune couche visible ne
touche le cadre, non plus (`layout.empty`). Le résultat ne porte les clés `scale` et `center`
que si l'échelle a été demandée ; sans `scale`, cadrage et résultat sont ceux d'avant.

Format, orientation et carroyage (epic 11, entrée 11). `page_size` (`a4` par défaut, `a3`,
`a2`, `a1`, `a0`) et `orientation` (`landscape` par défaut, `portrait`) donnent la page ; les
positions des éléments, pensées pour 297 × 210 mm, sont rapportées à la page en proportion
(abscisses et largeurs × largeur / 297, ordonnées et hauteurs × hauteur / 210, polices par le
plus petit des deux facteurs) : une A4 paysage garde les positions d'avant. `grid` pose une
grille sur la carte : `{"kind": "degrees", "interval": Angle}` (graticule en EPSG:4326,
intervalle en degrés, grades × 0,9) ou `{"kind": "projected", "epsg": n, "interval":
Distance}` (quadrillage en EPSG:n, intervalle en mètres, km × 1 000), étiquettes en décimal,
jamais en MGRS. Un `epsg` inconnu de QGIS rend `data.unknown_crs` ; une grille de plus de 200
lignes par axe sur le cadre de la carte (AD-8), un système géographique pour `projected` ou un
cadre qu'on ne sait pas ramener dans le système de la grille rendent `layout.bad_params` :
tous avant l'export, sans fichier. Sans ces trois paramètres, le fond est appelé avec les
arguments d'avant et le PDF est le même ; le résultat ne change jamais.

La mise en page de QGIS ne tourne pas hors du fil principal : l'export est synchrone
(commande non annulable). Accès à QGIS par un « fond » injectable ; `LayoutBackend` est
le seul endroit qui touche `qgis.core`, les tests mettent une doublure.
"""

import math
import os

try:  # importés à plat par les tests, en paquet par QGIS
    from .gestures import GestureError
    from .inventory import QgisBackend, _ReadGesture, _union, source_name
    from .point_transform import UNKNOWN_CRS
except ImportError:  # pragma: no cover
    from gestures import GestureError
    from inventory import QgisBackend, _ReadGesture, _union, source_name
    from point_transform import UNKNOWN_CRS

BAD_PARAMS = "layout.bad_params"
EMPTY = "layout.empty"
EXPORT_FAILED = "layout.export_failed"

MAX_SOURCES = 50  # borne du schéma de sortie du noyau (AD-8)
MIN_SCALE = 100  # bornes de `scale`, les mêmes que le schéma du noyau
MAX_SCALE = 100000000

# Formats de page : dimensions en mm, en paysage (largeur, hauteur). Les noms QGIS sont en
# majuscules (« A4 »).
PAGE_SIZES = {"a4": (297.0, 210.0), "a3": (420.0, 297.0), "a2": (594.0, 420.0), "a1": (841.0, 594.0), "a0": (1189.0, 841.0)}
ORIENTATIONS = ("landscape", "portrait")
BASE_PAGE = (297.0, 210.0)  # page pour laquelle les positions des éléments sont écrites
MAX_GRID_LINES = 200  # lignes de grille par axe, au plus (AD-8)


class GridError(Exception):
    """Levée par le fond quand la grille ne peut pas être posée (trop dense, système
    géographique pour un quadrillage, cadre hors du système) : aucun export n'a eu lieu."""


def page_dimensions(page):
    """(largeur, hauteur) en mm d'une page (`size`, `orientation`) ; None : A4 paysage."""
    if page is None:
        return BASE_PAGE
    size, orientation = page
    w, h = PAGE_SIZES[size]
    return (w, h) if orientation == "landscape" else (h, w)


def check_grid_density(frame, interval):
    """Refuse (`GridError`) une grille de plus de `MAX_GRID_LINES` lignes par axe sur le cadre
    `frame` (xmin, ymin, xmax, ymax), exprimé dans le système de la grille."""
    if frame is None:
        raise GridError("Le cadre de la carte ne peut pas être ramené dans le système de la grille.")
    for side in (frame[2] - frame[0], frame[3] - frame[1]):
        # Lignes tracées sur un côté : un intervalle de plus que d'écarts entiers.
        if not math.isfinite(side) or math.floor(side / interval) + 1 > MAX_GRID_LINES:
            raise GridError(
                "Carroyage trop serré : plus de %d lignes par axe sur la carte ; choisissez un intervalle plus grand."
                % MAX_GRID_LINES
            )


def _decimals(interval):
    """Décimales qu'il faut aux étiquettes pour écrire les multiples de `interval` (6 au plus)."""
    for d in range(7):
        scaled = interval * 10**d
        if abs(scaled - round(scaled)) < 1e-9 * max(1.0, scaled):
            return d
    return 6


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

    def knows(self, epsg):
        """Vrai si QGIS connaît le code EPSG `epsg`."""
        from qgis.core import QgsCoordinateReferenceSystem

        return QgsCoordinateReferenceSystem("EPSG:%d" % epsg).isValid()

    def reproject_point(self, x, y, from_authid, to_authid):
        """Point (x, y) du SCR `from_authid` dans le SCR `to_authid` ; None si l'un des deux est
        invalide ou si le point sort du domaine. Sans `iface` : sondé sous QGIS sans fenêtre."""
        if not to_authid or from_authid == to_authid:
            return (x, y)
        from qgis.core import QgsCoordinateReferenceSystem, QgsCoordinateTransform, QgsPointXY, QgsProject

        source = QgsCoordinateReferenceSystem(from_authid)
        target = QgsCoordinateReferenceSystem(to_authid)
        if not source.isValid() or not target.isValid():
            return None
        try:
            point = QgsCoordinateTransform(source, target, QgsProject.instance()).transform(QgsPointXY(x, y))
        except Exception:  # QgsCsException : point hors du domaine du SCR
            return None
        return (point.x(), point.y())

    def canvas_center(self, authid):
        """Centre (x, y) de l'emprise que montre la carte de QGIS, dans le SCR `authid` ; None
        sans interface (sondes, QGIS sans fenêtre), carte vide ou reprojection impossible."""
        try:
            from qgis.utils import iface
        except ImportError:  # pragma: no cover
            return None
        if iface is None:
            return None
        canvas = iface.mapCanvas()
        if canvas is None:
            return None
        rect = canvas.extent()
        if rect is None or rect.isNull() or rect.isEmpty():
            return None
        center = rect.center()
        return self.reproject_point(center.x(), center.y(), canvas.mapSettings().destinationCrs().authid(), authid)

    def export_pdf(self, layers, title, sources_text, out, authid, box, scale=None, page=None, grid=None):
        """Monte la mise en page, l'exporte dans `out`, rend `{"items": éléments posés (clés de
        `EXPECTED_ITEMS`), "scale": échelle relue sur la carte, "extent": emprise de la carte
        (xmin, ymin, xmax, ymax)}`. Avec `scale`, `box` est un
        petit rectangle centré : la carte y est cadrée puis tenue à 1:`scale` (`setScale` garde
        le centre). `page` : (`size`, `orientation`), A4 paysage si absent. `grid` : `{"authid":
        système de la grille, "interval": écart en unités de ce système}`. Lève `GridError`
        avant l'export si la grille ne peut pas être posée, `RuntimeError` si l'export échoue."""
        from qgis.core import (
            Qgis,
            QgsCoordinateReferenceSystem,
            QgsCoordinateTransform,
            QgsLayout,
            QgsLayoutExporter,
            QgsLayoutItemLabel,
            QgsLayoutItemLegend,
            QgsLayoutItemMap,
            QgsLayoutItemMapGrid,
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
        size, orientation = page if page is not None else ("a4", "landscape")
        layout.pageCollection().page(0).setPageSize(
            size.upper(),
            QgsLayoutItemPage.Orientation.Landscape
            if orientation == "landscape"
            else QgsLayoutItemPage.Orientation.Portrait,
        )
        # Positions écrites pour 297 × 210 mm, rapportées à la page (facteurs 1 en A4 paysage).
        width, height = page_dimensions(page)
        fx, fy = width / BASE_PAGE[0], height / BASE_PAGE[1]
        ff = min(fx, fy)

        def place(item, x, y, w, h):
            item.attemptMove(QgsLayoutPoint(x * fx, y * fy))
            item.attemptResize(QgsLayoutSize(w * fx, h * fy))
            layout.addLayoutItem(item)
            return item

        def points(size):
            # Taille de police entière quand le facteur vaut 1 : rendu d'avant à l'identique.
            return size if ff == 1 else max(1, int(round(size * ff)))

        def label(text, x, y, w, h, size, bold=False):
            item = QgsLayoutItemLabel(layout)
            item.setText(text)
            font = QFont()
            font.setPointSize(points(size))
            font.setBold(bold)
            item.setFont(font)
            return place(item, x, y, w, h)

        # Carte : couches visibles seules, cadrée sur leur emprise (zoomToExtent garde la taille).
        carte = QgsLayoutItemMap(layout)
        carte.setCrs(QgsCoordinateReferenceSystem(authid))
        carte.setLayers(layers)
        place(carte, 10, 22, 200, 160)
        carte.zoomToExtent(QgsRectangle(*box))
        if scale is not None:
            carte.setScale(scale)
        if grid is not None:
            grid_crs = QgsCoordinateReferenceSystem(grid["authid"])
            if not grid_crs.isValid():
                raise GridError("Système de la grille inconnu de QGIS : %s." % grid["authid"])
            if grid.get("projected") and grid_crs.isGeographic():
                raise GridError(
                    "Le quadrillage demande un système projeté ; pour des degrés, demandez un graticule."
                )
            frame = carte.extent()
            try:
                to_grid = QgsCoordinateTransform(carte.crs(), grid_crs, QgsProject.instance())
                in_grid = to_grid.transformBoundingBox(frame)
                in_grid = (in_grid.xMinimum(), in_grid.yMinimum(), in_grid.xMaximum(), in_grid.yMaximum())
            except Exception:  # QgsCsException : cadre hors du domaine du système
                in_grid = None
            check_grid_density(in_grid, grid["interval"])
            map_grid = QgsLayoutItemMapGrid("Carroyage", carte)
            map_grid.setCrs(grid_crs)
            map_grid.setIntervalX(grid["interval"])
            map_grid.setIntervalY(grid["interval"])
            map_grid.setStyle(QgsLayoutItemMapGrid.GridStyle.Solid)
            map_grid.setAnnotationEnabled(True)
            map_grid.setAnnotationFormat(QgsLayoutItemMapGrid.AnnotationFormat.Decimal)
            map_grid.setAnnotationPrecision(_decimals(grid["interval"]))
            # Étiquettes à gauche et en bas seulement : à droite, elles mordraient sur la légende,
            # en haut sur le titre.
            for side in (QgsLayoutItemMapGrid.BorderSide.Right, QgsLayoutItemMapGrid.BorderSide.Top):
                map_grid.setAnnotationDisplay(QgsLayoutItemMapGrid.DisplayMode.HideAll, side)
            grid_format = map_grid.annotationTextFormat()
            grid_format.setSize(points(6))
            map_grid.setAnnotationTextFormat(grid_format)
            carte.grids().addGrid(map_grid)
            map_grid.setEnabled(True)

        title_label = label(title, 10, 6, 277, 14, 18, bold=True)

        legend = QgsLayoutItemLegend(layout)
        legend.setLinkedMap(carte)
        legend.setAutoUpdateModel(False)  # sinon la légende reprend aussi les couches masquées
        root = legend.model().rootGroup()
        root.clear()
        for layer in layers:
            root.addLayer(layer)
        place(legend, 215, 22, 72, 100)

        scalebar = QgsLayoutItemScaleBar(layout)
        scalebar.setLinkedMap(carte)
        scalebar.applyDefaultSize()
        # Graduations en km dès que le segment fait 1 000 m : « 0 1 2 km » ne se chevauche pas.
        if scalebar.units() == Qgis.DistanceUnit.Meters and scalebar.unitsPerSegment() >= 1000:
            scalebar.setUnitsPerSegment(scalebar.unitsPerSegment() / 1000.0)
            scalebar.setUnits(Qgis.DistanceUnit.Kilometers)
            scalebar.setUnitLabel("km")
        scale_font = QFont()
        scale_font.setPointSize(points(8))
        scalebar.setFont(scale_font)
        place(scalebar, 10, 185, 80, 12)

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
        frame = carte.extent()
        return {
            "items": placed,
            "scale": carte.scale(),
            "extent": (frame.xMinimum(), frame.yMinimum(), frame.xMaximum(), frame.yMaximum()),
        }


def _padded(box):
    """Marge de 5 % autour de l'emprise ; une emprise réduite à un point reçoit une marge fixe."""
    xmin, ymin, xmax, ymax = box
    pad = max(xmax - xmin, ymax - ymin) * 0.05 or 1.0
    return (xmin - pad, ymin - pad, xmax + pad, ymax + pad)


def _centered(center):
    """Petit rectangle centré sur `center` : la carte y est cadrée avant de prendre l'échelle."""
    x, y = center
    return (x - 1.0, y - 1.0, x + 1.0, y + 1.0)


def _scale_param(params):
    """`scale` lu et vérifié : None si absent (ou nul, que le schéma du noyau admet), sinon un
    entier dans les bornes du schéma."""
    scale = params.get("scale")
    if scale is None:
        return None
    if (
        not isinstance(scale, int)
        or isinstance(scale, bool)
        or not MIN_SCALE <= scale <= MAX_SCALE
    ):
        raise GestureError(
            BAD_PARAMS, "L'échelle doit être un entier compris entre 1:100 et 1:100 000 000."
        )
    return scale


def _number(value):
    return isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(value)


def _center_param(params):
    """`center` (fourni par le noyau au rejeu seulement) lu et vérifié : None si absent, sinon
    (x, y, epsg)."""
    center = params.get("center")
    if center is None:
        return None
    if (
        not isinstance(center, dict)
        or set(center) != {"x", "y", "epsg"}
        or not _number(center["x"])
        or not _number(center["y"])
        or not isinstance(center["epsg"], int)
        or isinstance(center["epsg"], bool)
        or center["epsg"] <= 0
    ):
        raise GestureError(BAD_PARAMS, "Le centre de la carte doit être un point x, y, epsg.")
    return (float(center["x"]), float(center["y"]), center["epsg"])


def _page_param(params):
    """(`size`, `orientation`) lus et vérifiés ; None si ni l'un ni l'autre n'est donné (ou tous
    deux nuls) : le fond garde alors ses arguments d'avant."""
    size = params.get("page_size")
    orientation = params.get("orientation")
    if size is None and orientation is None:
        return None
    if size is not None and (not isinstance(size, str) or size not in PAGE_SIZES):
        raise GestureError(BAD_PARAMS, "Le format de page doit être a4, a3, a2, a1 ou a0.")
    if orientation is not None and (not isinstance(orientation, str) or orientation not in ORIENTATIONS):
        raise GestureError(BAD_PARAMS, "L'orientation doit être landscape ou portrait.")
    return (size or "a4", orientation or "landscape")


def _positive(value):
    return _number(value) and value > 0


def _grid_param(params):
    """`grid` lu et vérifié : None si absent, sinon `{"authid", "interval", "projected"}`,
    l'intervalle en degrés (graticule) ou en mètres (quadrillage)."""
    grid = params.get("grid")
    if grid is None:
        return None
    bad = GestureError(
        BAD_PARAMS,
        "Le carroyage doit être {kind: degrees, interval: angle} ou {kind: projected, epsg, interval: distance}, "
        "avec un intervalle strictement positif.",
    )
    if not isinstance(grid, dict):
        raise bad
    interval = grid.get("interval")
    if not isinstance(interval, dict) or not _positive(interval.get("value")):
        raise bad
    kind = grid.get("kind")
    if kind == "degrees":
        unit = interval.get("unit", "deg")
        if set(grid) != {"kind", "interval"} or not set(interval) <= {"value", "unit"} or unit not in ("deg", "gon"):
            raise bad
        degrees = interval["value"] * (0.9 if unit == "gon" else 1.0)
        return {"authid": "EPSG:4326", "interval": float(degrees), "projected": False}
    if kind == "projected":
        epsg = grid.get("epsg")
        if (
            set(grid) != {"kind", "epsg", "interval"}
            or set(interval) != {"value", "unit"}
            or interval["unit"] not in ("m", "km")
            or not isinstance(epsg, int)
            or isinstance(epsg, bool)
        ):
            raise bad
        metres = interval["value"] * (1000.0 if interval["unit"] == "km" else 1.0)
        return {"authid": "EPSG:%d" % epsg, "interval": float(metres), "projected": True, "epsg": epsg}
    raise bad


def _rounded(center):
    """(x, y) arrondis à 6 décimales : relus à l'identique par le noyau, et stables au rejeu."""
    return (round(float(center[0]), 6), round(float(center[1]), 6))


def _epsg_of(authid):
    """Code EPSG d'un authid « EPSG:n », sinon None."""
    prefix, _, code = (authid or "").partition(":")
    if prefix.upper() != "EPSG" or not code.isdigit() or int(code) <= 0:
        return None
    return int(code)


def _touches(a, b):
    """Vrai si les rectangles (xmin, ymin, xmax, ymax) se touchent (bord compris)."""
    return a[0] <= b[2] and b[0] <= a[2] and a[1] <= b[3] and b[1] <= a[3]


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
        scale = _scale_param(params)
        given_center = _center_param(params)
        page = _page_param(params)
        grid = _grid_param(params)
        backend = self._backend
        if grid is not None and grid["projected"] and not backend.knows(grid["epsg"]):
            raise GestureError(UNKNOWN_CRS, "Système de la grille inconnu de QGIS : EPSG:%d." % grid["epsg"])
        # Au rejeu, le noyau nomme les couches posées par une exécution précédente du même
        # journal : elles restent dans le projet, intactes, mais hors de la carte (ticket 5.36).
        leave_out = set(leave_out)
        layers = [layer for layer in backend.visible_layers() if backend.id_of(layer) not in leave_out]
        if not layers:
            raise GestureError(EMPTY, "Aucune couche visible : il n'y a rien à mettre sur la carte.")
        crs = backend.project_crs()
        extents = [backend.extent_of(layer, crs) for layer in layers]
        box = _union(extents)
        if box is None:
            raise GestureError(EMPTY, "Les couches visibles sont vides : la carte n'a pas d'emprise.")

        names = []
        for layer in layers:
            name = source_name(backend.source_of(layer))
            if name is not None and name not in names:
                names.append(name)
        sources_text = "Sources : " + (", ".join(names) if names else "couches du projet (hors fichier)")

        center = None
        if scale is not None:
            epsg = _epsg_of(crs)
            if epsg is None:
                raise GestureError(
                    BAD_PARAMS,
                    "Le système du projet n'a pas de code EPSG : la carte ne peut pas être tenue à une échelle.",
                )
            if given_center is not None:
                # Au rejeu : le centre journalisé, dans le système du projet.
                x, y, from_epsg = given_center
                center = backend.reproject_point(x, y, "EPSG:%d" % from_epsg, crs)
                if center is None:
                    raise GestureError(
                        BAD_PARAMS, "Le centre de la carte ne peut pas être ramené dans le système du projet."
                    )
            else:
                # Centre de la carte de QGIS ; sans carte, celui de l'emprise des couches.
                center = backend.canvas_center(crs)
                if center is None:
                    center = ((box[0] + box[2]) / 2.0, (box[1] + box[3]) / 2.0)
            # Arrondi au micromètre : un flottant à 17 chiffres peut être relu par le noyau à un
            # ULP près, et le résultat rejoué divergerait. Le centre arrondi cadre et est rendu.
            center = _rounded(center)

        # Les nouveaux arguments ne passent au fond que s'ils sont donnés : sans eux, l'appel est
        # celui d'avant.
        extra = {}
        if scale is not None:
            extra["scale"] = scale
        if page is not None:
            extra["page"] = page
        if grid is not None:
            extra["grid"] = {"authid": grid["authid"], "interval": grid["interval"], "projected": grid["projected"]}
        try:
            framing = _padded(box) if scale is None else _centered(center)
            done = backend.export_pdf(layers, title.strip(), sources_text, out, crs, framing, **extra)
            if scale is not None and round(done["scale"]) != scale:
                raise RuntimeError("échelle non tenue : 1:%s au lieu de 1:%s" % (done["scale"], scale))
            if not os.path.isfile(out):
                raise RuntimeError("fichier absent après l'export")
        except GridError as e:
            self._discard(out)
            raise GestureError(BAD_PARAMS, str(e))
        except Exception:
            self._discard(out)
            raise GestureError(EXPORT_FAILED, "L'export PDF a échoué : aucun fichier n'a été écrit.")
        if scale is not None and not any(e is not None and _touches(e, done["extent"]) for e in extents):
            # Cadre sans donnée : un PDF vide annoncerait des couches qu'il ne montre pas.
            self._discard(out)
            raise GestureError(
                EMPTY,
                "Aucune couche visible dans le cadre : déplacez la vue de QGIS sur les données, puis recommencez.",
            )
        self.items_placed = list(done["items"])
        result = {
            "file": out,
            "page_count": 1,
            "layer_count": len(layers),
            "sources": names[:MAX_SOURCES],
            "truncated": len(names) > MAX_SOURCES,
        }
        if scale is not None:
            result["scale"] = scale
            result["center"] = {"x": center[0], "y": center[1], "epsg": _epsg_of(crs)}
        return result

    @staticmethod
    def _discard(path):
        try:
            if os.path.exists(path):
                os.remove(path)
        except OSError:  # pragma: no cover
            pass
