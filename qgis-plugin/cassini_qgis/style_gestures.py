"""Gestes `layer.set_style` et `layer.arrange` : mise en forme et rangement des couches.

Ticket 11.21. Le noyau compose le style de chaque couche (un `StyleSpec`, neutre, AD-22) et
le rangement en groupes, puis les dicte, juste après `map.topographic_style` ou
`map.graduated_style`. Ces gestes appliquent ce qu'on leur dicte, sans aucune décision de
thème ni de couleur (AD-5) :

- `layer.set_style` traduit chaque forme du `StyleSpec` en rendu de QGIS : symbole unique,
  catégories par valeur, dégradé dont QGIS calcule les classes sur les vraies valeurs,
  pseudo-couleur interpolée d'un MNT, ombrage posé en fondu « produit » (gris, sauf pour un
  raster que QGIS dessine déjà en relief ombré : ce rendu est gardé, seuls l'opacité et le
  fondu changent). Une forme vecteur porte aussi sa visibilité selon l'échelle
  (`max_scale`). Seul le style de la couche dans le projet change : le fichier de données
  n'est jamais touché ;
- `layer.arrange` pose les groupes, dans l'ordre, sous les couches que le noyau n'a pas
  listées (elles restent au-dessus, intactes), et y range les couches de haut en bas. Un
  groupe du même nom à la racine est réutilisé, pas dupliqué ; les couches non listées qu'il
  contenait remontent en tête de la racine. Un groupe vidé par le rangement est retiré ; un
  groupe déjà vide avant est laissé.

Tout est vérifié avant de toucher au projet : un ordre refusé ne change rien. Les erreurs
sont celles du catalogue : `qgis.layer_missing`, `style.invalid`, `style.field_missing`,
`style.geometry_mismatch`, `arrange.invalid`. La vérification du style suit
`StyleSpec::validate` du noyau ; `fixtures/style-cases.json` est lu des deux côtés.

Le déroulé (ordonnanceur, annulation, ordre soldé une fois) est celui d'`open_output.py`.
`QgisBackend` est le seul endroit qui importe `qgis.core`, à l'intérieur de ses méthodes ;
les tests mettent une doublure. Rendus vérifiés sous QGIS 3.44 (`tests/probe_style.py`) ;
pas encore essayés sous QGIS 4.x.
"""

import math
import re

try:  # importé à plat par les tests, en paquet par QGIS
    from .gestures import GestureError
    from .open_output import OpenOutputGesture
except ImportError:  # pragma: no cover
    from gestures import GestureError
    from open_output import OpenOutputGesture

LAYER_MISSING = "qgis.layer_missing"
STYLE_INVALID = "style.invalid"
FIELD_MISSING = "style.field_missing"
GEOMETRY_MISMATCH = "style.geometry_mismatch"
ARRANGE_INVALID = "arrange.invalid"

VECTOR = "vector"
RASTER = "raster"
GEOMETRIES = ("point", "line", "polygon")

# Champs admis par forme, comme le `deny_unknown_fields` de `core/src/style/spec.rs`.
FIELDS = {
    "single": {"stroke", "fill", "width_mm", "opacity", "max_scale"},
    "categories": {"field", "classes", "other", "opacity", "max_scale"},
    "graduated": {"field", "method", "class_count", "ramp", "stroke", "width_mm", "max_scale"},
    "elevation_ramp": {"stops", "opacity"},
    "hillshade": {"opacity"},
}
TARGETS = {
    "single": VECTOR,
    "categories": VECTOR,
    "graduated": VECTOR,
    "elevation_ramp": RASTER,
    "hillshade": RASTER,
}
CLASS_FIELDS = {"values", "label", "stroke", "fill", "width_mm", "max_scale"}
OTHER_FIELDS = {"label", "stroke", "fill", "width_mm", "max_scale"}
METHODS = ("equal_interval", "quantile", "jenks")
CLASS_COUNT = (2, 12)
COLOR = re.compile(r"^#[0-9a-fA-F]{6}$")


def _invalid(why):
    return GestureError(STYLE_INVALID, "Style refusé : %s." % why)


def _object(value, allowed, what):
    if not isinstance(value, dict):
        raise _invalid("%s n'est pas un objet" % what)
    unknown = sorted(str(k) for k in set(value) - allowed)
    if unknown:
        raise _invalid("champ inconnu « %s » dans %s" % (unknown[0], what))
    return value


def _color(value, what, required=False):
    if value is None and not required:
        return None
    if not isinstance(value, str) or not COLOR.match(value):
        raise _invalid("la couleur de %s n'est pas de la forme #rrggbb" % what)
    return value.lower()


def _number(value):
    return (
        isinstance(value, (int, float))
        and not isinstance(value, bool)
        and math.isfinite(value)
    )


def _width(value, what):
    if value is None:
        return None
    if not _number(value) or value < 0:
        raise _invalid("l'épaisseur de %s n'est pas un nombre positif" % what)
    return float(value)


def _opacity(value):
    if value is None:
        return 1.0
    if not _number(value) or not 0 <= value <= 1:
        raise _invalid("l'opacité n'est pas comprise entre 0 et 1")
    return float(value)


def _max_scale(value):
    """Dénominateur d'échelle entier, au moins 1 ; None : visible à toute échelle."""
    if value is None:
        return None
    if not isinstance(value, int) or isinstance(value, bool) or value < 1:
        raise _invalid("l'échelle de visibilité est un entier d'au moins 1")
    return value


def _field(value, what):
    """Un nom de champ : du texte qui n'est pas que des blancs (`field.trim()` du noyau)."""
    if not isinstance(value, str) or not value.strip():
        raise _invalid("%s est vide" % what)
    return value


def _symbol(value, what, stroke_required):
    """Un symbole normalisé : `{"stroke", "fill", "width_mm"}`, couleurs en minuscules."""
    return {
        "stroke": _color(value.get("stroke"), what, required=stroke_required),
        "fill": _color(value.get("fill"), what),
        "width_mm": _width(value.get("width_mm"), what),
    }


def _categories(body, out):
    out["field"] = _field(body.get("field"), "le champ des catégories")
    classes = body.get("classes")
    if not isinstance(classes, list) or not classes:
        raise _invalid("un style par catégories demande au moins une classe")
    out["classes"] = []
    for n, raw in enumerate(classes, start=1):
        what = "la classe %d" % n
        _object(raw, CLASS_FIELDS, what)
        values = raw.get("values")
        if not isinstance(values, list) or not values:
            raise _invalid("%s n'a aucune valeur" % what)
        if not all(isinstance(v, str) for v in values):
            raise _invalid("les valeurs de %s ne sont pas des textes" % what)
        # Pour QGIS, une catégorie de valeur vide est celle des « autres valeurs » : une valeur
        # vide dictée en ferait un fourre-tout. Refus propre à l'extension (le noyau l'admet).
        if any(v == "" for v in values):
            raise _invalid("une valeur de %s est vide" % what)
        if not isinstance(raw.get("label"), str):
            raise _invalid("le libellé de %s manque" % what)
        out["classes"].append(
            {
                "values": list(values),
                "label": raw["label"],
                "symbol": _symbol(raw, what, stroke_required=True),
                "max_scale": _max_scale(raw.get("max_scale")),
            }
        )
    other = body.get("other")
    if other is None:
        out["other"] = None
        return
    what = "le symbole des autres valeurs"
    _object(other, OTHER_FIELDS, what)
    if not isinstance(other.get("label"), str):
        raise _invalid("le libellé des autres valeurs manque")
    out["other"] = {
        "label": other["label"],
        "symbol": _symbol(other, what, stroke_required=True),
        "max_scale": _max_scale(other.get("max_scale")),
    }


def _graduated(body, out):
    out["field"] = _field(body.get("field"), "le champ du dégradé")
    method = body.get("method")
    if not isinstance(method, str) or method not in METHODS:
        raise _invalid("méthode de classes inconnue « %s »" % (method,))
    out["method"] = method
    count = body.get("class_count")
    low, high = CLASS_COUNT
    if not isinstance(count, int) or isinstance(count, bool) or not low <= count <= high:
        raise _invalid("le nombre de classes va de %d à %d" % CLASS_COUNT)
    out["class_count"] = count
    ramp = _object(body.get("ramp"), {"from", "to"}, "le dégradé")
    out["ramp"] = (
        _color(ramp.get("from"), "le dégradé", True),
        _color(ramp.get("to"), "le dégradé", True),
    )
    out["symbol"] = {
        "stroke": _color(body.get("stroke"), "le trait"),
        "fill": None,
        "width_mm": _width(body.get("width_mm"), "le trait"),
    }


def _elevation_ramp(body, out):
    stops = body.get("stops")
    if not isinstance(stops, list) or len(stops) < 2:
        raise _invalid("une rampe d'altitudes demande au moins deux paliers")
    out["stops"] = []
    for n, raw in enumerate(stops, start=1):
        what = "le palier %d" % n
        _object(raw, {"value", "color"}, what)
        if not _number(raw.get("value")):
            raise _invalid("l'altitude du palier %d n'est pas un nombre" % n)
        value = float(raw["value"])
        if out["stops"] and value <= out["stops"][-1][0]:
            raise _invalid("les paliers de la rampe ne sont pas croissants")
        out["stops"].append((value, _color(raw.get("color"), what, True)))
    out["opacity"] = _opacity(body.get("opacity"))


def normalize_style(style):
    """Vérifie un `StyleSpec` et le rend normalisé ; `GestureError(style.invalid)` sinon.

    Fonction pure, sans QGIS : les mêmes refus que la lecture et `StyleSpec::validate` du
    noyau, pour qu'un ordre mal formé ne touche jamais au projet.
    """
    if not isinstance(style, dict):
        raise _invalid("le style n'est pas un objet")
    kind = style.get("kind")
    if not isinstance(kind, str) or kind not in FIELDS:
        raise _invalid("forme inconnue « %s »" % (kind,))
    body = dict(style)
    del body["kind"]
    _object(body, FIELDS[kind], "le style « %s »" % kind)
    out = {"kind": kind}
    if TARGETS[kind] == VECTOR:
        out["max_scale"] = _max_scale(body.get("max_scale"))
    if kind == "single":
        out["symbol"] = _symbol(body, "le symbole", stroke_required=False)
        if out["symbol"]["stroke"] is None and out["symbol"]["fill"] is None:
            raise _invalid("un symbole unique sans trait ni remplissage")
        out["opacity"] = _opacity(body.get("opacity"))
    elif kind == "categories":
        _categories(body, out)
        out["opacity"] = _opacity(body.get("opacity"))
    elif kind == "graduated":
        _graduated(body, out)
    elif kind == "elevation_ramp":
        _elevation_ramp(body, out)
    else:  # hillshade
        out["opacity"] = _opacity(body.get("opacity"))
    return out


class QgisBackend:
    """Accès réel à QGIS. Seul ce type touche `qgis.core`, et seulement dans ses méthodes."""

    # -- lecture --------------------------------------------------------------
    def layer(self, layer_id):
        from qgis.core import QgsProject

        return QgsProject.instance().mapLayer(layer_id)

    def name_of(self, layer):
        return layer.name()

    def kind_of(self, layer):
        from qgis.core import QgsRasterLayer, QgsVectorLayer

        if isinstance(layer, QgsVectorLayer):
            return VECTOR
        if isinstance(layer, QgsRasterLayer):
            return RASTER
        return None

    def geometry_of(self, layer):
        """`point`, `line`, `polygon`, ou None pour une table sans géométrie."""
        from qgis.core import Qgis

        kinds = {
            Qgis.GeometryType.Point: "point",
            Qgis.GeometryType.Line: "line",
            Qgis.GeometryType.Polygon: "polygon",
        }
        return kinds.get(layer.geometryType())

    def field_kind(self, layer, name):
        """None si le champ manque, `numeric` ou `other` sinon."""
        index = layer.fields().lookupField(name)
        if index < 0:
            return None
        return "numeric" if layer.fields().at(index).isNumeric() else "other"

    # -- symboles -------------------------------------------------------------
    @staticmethod
    def _make_symbol(geometry, symbol, fill_override=None):
        """Un symbole simple de QGIS, par ses propriétés textuelles."""
        from qgis.core import QgsFillSymbol, QgsLineSymbol, QgsMarkerSymbol

        stroke, width = symbol["stroke"], symbol["width_mm"]
        fill = fill_override or symbol["fill"]
        if geometry == "polygon":
            props = {"style": "solid" if fill else "no", "color": fill or "#000000"}
            props["outline_style"] = "solid" if stroke else "no"
            props["outline_color"] = stroke or "#000000"
            if width is not None:
                props["outline_width"] = repr(width)
                props["outline_width_unit"] = "MM"
            return QgsFillSymbol.createSimple(props)
        if geometry == "line":
            props = {"line_color": stroke or fill, "line_style": "solid"}
            if width is not None:
                props["line_width"] = repr(width)
                props["line_width_unit"] = "MM"
            return QgsLineSymbol.createSimple(props)
        props = {"name": "circle", "color": fill or stroke}
        props["outline_style"] = "solid" if stroke else "no"
        props["outline_color"] = stroke or "#000000"
        if width is not None:
            props["outline_width"] = repr(width)
            props["outline_width_unit"] = "MM"
        return QgsMarkerSymbol.createSimple(props)

    @staticmethod
    def _plain_vector(layer):
        """Opacité et fondu de la couche remis à zéro : le style dicté est tout le style
        visible, une opacité posée avant (à la main, par un autre outil) ne le fausse pas,
        et le rejeu rend la même carte quel que soit l'état du projet."""
        layer.setOpacity(1.0)
        layer.setBlendMode(_normal_blend())

    # -- rendus ---------------------------------------------------------------
    def apply_visibility(self, layer, max_scale):
        """Dessinée seulement aux échelles plus grandes que 1:`max_scale` ; None : toujours.
        Pour QGIS, `minimumScale` est l'échelle la plus petite (la plus dézoomée) où la couche
        se dessine."""
        if max_scale is None:
            layer.setScaleBasedVisibility(False)
            return
        layer.setMinimumScale(float(max_scale))
        layer.setMaximumScale(0.0)  # aucune limite vers le grand zoom
        layer.setScaleBasedVisibility(True)

    def apply_single(self, layer, geometry, symbol, opacity):
        from qgis.core import QgsSingleSymbolRenderer

        qsymbol = self._make_symbol(geometry, symbol)
        qsymbol.setOpacity(opacity)
        layer.setRenderer(QgsSingleSymbolRenderer(qsymbol))
        _legend_labels(layer)
        self._plain_vector(layer)
        layer.triggerRepaint()

    def apply_categories(self, layer, geometry, field, classes, other, opacity=1.0):
        """Catégories par valeur. Une classe à seuil d'échelle (`max_scale`) demande un rendu
        par règles : celui de QGIS par catégories n'a pas d'échelle par catégorie. Une règle
        par classe, de même filtre, symbole, libellé et seuil, puis une règle SINON pour les
        autres valeurs : la légende garde une entrée par classe."""
        from qgis.core import QgsCategorizedSymbolRenderer, QgsRendererCategory

        scaled = [c for c in classes if c["max_scale"]] or (other and other["max_scale"])
        if scaled:
            self._apply_rules(layer, geometry, field, classes, other, opacity)
            return
        categories = []
        for item in classes:
            values = item["values"]
            value = values[0] if len(values) == 1 else list(values)
            qsymbol = self._make_symbol(geometry, item["symbol"])
            categories.append(QgsRendererCategory(value, qsymbol, item["label"]))
        if other is not None:
            # Une catégorie de valeur vide est, pour QGIS, celle de « toutes les autres valeurs ».
            qsymbol = self._make_symbol(geometry, other["symbol"])
            categories.append(QgsRendererCategory("", qsymbol, other["label"]))
        layer.setRenderer(QgsCategorizedSymbolRenderer(field, categories))
        _legend_labels(layer)
        self._plain_vector(layer)
        layer.setOpacity(opacity)  # après la remise à zéro : l'opacité dictée de la couche
        layer.triggerRepaint()

    def _apply_rules(self, layer, geometry, field, classes, other, opacity):
        from qgis.core import QgsExpression, QgsRuleBasedRenderer

        root = QgsRuleBasedRenderer.Rule(None)
        column = QgsExpression.quotedColumnRef(field)
        for item in classes:
            # Valeurs comparées comme du texte, comme le rendu par catégories.
            values = ", ".join(QgsExpression.quotedString(v) for v in item["values"])
            rule = QgsRuleBasedRenderer.Rule(
                self._make_symbol(geometry, item["symbol"]),
                0,  # aucune limite vers le grand zoom
                int(item["max_scale"] or 0),  # 0 : à toute échelle
                "to_string(%s) IN (%s)" % (column, values),
                item["label"],
            )
            root.appendChild(rule)
        if other is not None:
            rule = QgsRuleBasedRenderer.Rule(
                self._make_symbol(geometry, other["symbol"]),
                0,
                int(other["max_scale"] or 0),
                "",
                other["label"],
            )
            rule.setIsElse(True)
            root.appendChild(rule)
        layer.setRenderer(QgsRuleBasedRenderer(root))
        _legend_labels(layer)
        self._plain_vector(layer)
        layer.setOpacity(opacity)
        layer.triggerRepaint()

    def apply_graduated(self, layer, geometry, field, method, class_count, ramp, symbol):
        """Classes calculées par QGIS ; aucune classe (couche vide, champ nul ou constant) :
        le rendu d'avant reste, et l'ordre est refusé par `style.field_missing`."""
        from qgis.core import (
            QgsClassificationEqualInterval,
            QgsClassificationJenks,
            QgsClassificationQuantile,
            QgsGradientColorRamp,
            QgsGraduatedSymbolRenderer,
        )
        from qgis.PyQt.QtGui import QColor

        methods = {
            "equal_interval": QgsClassificationEqualInterval,
            "quantile": QgsClassificationQuantile,
            "jenks": QgsClassificationJenks,
        }
        renderer = QgsGraduatedSymbolRenderer(field)
        renderer.setClassificationMethod(methods[method]())
        renderer.setSourceSymbol(self._make_symbol(geometry, symbol, fill_override=ramp[0]))
        renderer.setSourceColorRamp(QgsGradientColorRamp(QColor(ramp[0]), QColor(ramp[1])))
        renderer.updateClasses(layer, class_count)
        if not renderer.ranges():
            # Le rendu neuf n'est jamais posé : celui d'avant reste à la couche.
            raise GestureError(
                FIELD_MISSING,
                "Le champ « %s » de la couche « %s » n'a pas de valeurs à classer."
                % (field, layer.name()),
            )
        renderer.updateColorRamp()  # colore les classes par la rampe source, déjà posée
        layer.setRenderer(renderer)
        _legend_labels(layer)
        self._plain_vector(layer)
        layer.triggerRepaint()

    def apply_elevation_ramp(self, layer, stops, opacity):
        from qgis.core import (
            Qgis,
            QgsColorRampLegendNodeSettings,
            QgsColorRampShader,
            QgsRasterShader,
            QgsSingleBandPseudoColorRenderer,
        )
        from qgis.PyQt.QtGui import QColor

        ramp = QgsColorRampShader(stops[0][0], stops[-1][0])
        ramp.setColorRampType(Qgis.ShaderInterpolationMethod.Linear)
        ramp.setColorRampItemList(
            [
                QgsColorRampShader.ColorRampItem(value, QColor(color), "%g m" % value)
                for value, color in stops
            ]
        )
        # Légende en une barre continue, bornée par les altitudes réelles du MNT. Les paliers et
        # leurs couleurs restent ceux que le noyau a dictés : QGIS colore chaque cellule d'après
        # la liste des paliers, en valeurs absolues. Les statistiques de la bande ne servent qu'à
        # l'affichage, comme un fait du fichier : les bornes de la barre, leurs libellés, et la
        # part de la rampe que la barre montre (celle des altitudes présentes).
        settings = QgsColorRampLegendNodeSettings()
        settings.setUseContinuousLegend(True)
        span = self._band_range(layer)
        low, high = span if span is not None else (stops[0][0], stops[-1][0])
        settings.setMinimumLabel("%d m" % round(low))
        settings.setMaximumLabel("%d m" % round(high))
        ramp.setLegendSettings(settings)
        shader = QgsRasterShader()
        shader.setRasterShaderFunction(ramp)
        renderer = QgsSingleBandPseudoColorRenderer(layer.dataProvider(), 1, shader)
        renderer.setClassificationMin(stops[0][0])
        renderer.setClassificationMax(stops[-1][0])
        renderer.setOpacity(opacity)
        layer.setRenderer(renderer)
        layer.setBlendMode(_normal_blend())
        if span is not None:
            _set_legend(layer, _elevation_legend(ramp, span, settings))
            # Le titre « Altitude (m) » est dans la légende elle-même ; les bornes sont gardées
            # dans le projet, pour la reposer à sa réouverture (voir `install_legend`).
            _legend_labels(layer, span=span)
        else:
            _set_legend(layer, _default_raster_legend(layer))
            _legend_labels(layer, first_label=ELEVATION_LEGEND)
        layer.triggerRepaint()

    def _plain_stretch(self, layer):
        """Gris de l'ombrage sans amplification : un ombrage en octets va de 0 à 255, et une
        plaine, presque uniforme, reste claire. L'étirement par défaut de QGIS (2 à 98 % des
        valeurs) noircirait ses moindres écarts. Pour un autre type de valeurs, de la plus basse
        à la plus haute réelles (relevé sous QGIS 3.44)."""
        from qgis.core import Qgis, QgsContrastEnhancement

        provider = layer.dataProvider()
        kind = provider.dataType(1)
        stretch = QgsContrastEnhancement(kind)
        stretch.setContrastEnhancementAlgorithm(
            QgsContrastEnhancement.ContrastEnhancementAlgorithm.StretchToMinimumMaximum
        )
        span = (0.0, 255.0) if kind == Qgis.DataType.Byte else self._band_range(layer)
        if span is not None:
            stretch.setMinimumValue(span[0])
            stretch.setMaximumValue(span[1])
        return stretch

    @staticmethod
    def _band_range(layer):
        """(plus basse, plus haute) valeur de la bande 1, lues par QGIS ; None si inconnues."""
        from qgis.core import Qgis

        from qgis.core import QgsRectangle

        # Statistiques échantillonnées (au plus 250 000 cellules), pas une lecture complète : un
        # MNT de 36 millions de cellules se lirait en secondes. Elles ne servent qu'aux libellés
        # et aux bornes de la légende, jamais au rendu ; arrondies au mètre, l'écart est minime.
        wanted = Qgis.RasterBandStatistic.Min | Qgis.RasterBandStatistic.Max
        stats = layer.dataProvider().bandStatistics(1, wanted, QgsRectangle(), STATS_SAMPLE)
        low, high = stats.minimumValue, stats.maximumValue
        if not (math.isfinite(low) and math.isfinite(high)) or low >= high:
            return None
        return (low, high)

    def apply_hillshade(self, layer, opacity):
        """Ombrage posé en fondu « produit ». Un raster d'ombrage (valeurs d'éclairement) est
        rendu en gris. Une couche que le projet ombre déjà par le rendu d'ombrage de QGIS (un
        MNT dessiné en relief ombré, comme dans les projets de test) garde ce rendu : la peindre
        en gris montrerait ses altitudes, pas son relief. Seuls l'opacité et le fondu changent."""
        from qgis.core import QgsSingleBandGrayRenderer

        if layer.renderer() is None or layer.renderer().type() != "hillshade":
            renderer = QgsSingleBandGrayRenderer(layer.dataProvider(), 1)
            renderer.setContrastEnhancement(self._plain_stretch(layer))
            layer.setRenderer(renderer)
        _set_legend(layer, _hillshade_legend())  # une légende d'altitudes d'avant part
        layer.renderer().setOpacity(opacity)
        layer.setBlendMode(_multiply_blend())
        _legend_labels(layer, title=HILLSHADE_LEGEND, no_nodes=True)
        layer.triggerRepaint()

    # -- arbre des couches ----------------------------------------------------
    def root(self):
        from qgis.core import QgsProject

        return QgsProject.instance().layerTreeRoot()

    def has_tree_node(self, layer_id):
        return self.root().findLayer(layer_id) is not None

    def root_groups(self):
        from qgis.core import QgsLayerTree

        return [node for node in self.root().children() if QgsLayerTree.isGroup(node)]

    def all_groups(self):
        """Tous les groupes de l'arbre, en largeur d'abord."""
        from qgis.core import QgsLayerTree

        found, todo = [], list(self.root().children())
        while todo:
            node = todo.pop(0)
            if QgsLayerTree.isGroup(node):
                found.append(node)
                todo.extend(node.children())
        return found

    def group_name(self, group):
        return group.name()

    def group_path(self, group):
        """Noms des groupes depuis la racine jusqu'à `group` compris."""
        names, node = [], group
        while node is not None and node.parent() is not None:  # la racine n'a pas de parent
            names.append(node.name())
            node = node.parent()
        return tuple(reversed(names))

    def has_layers(self, group):
        return bool(group.findLayers())

    def has_children(self, group):
        return bool(group.children())

    def layers_in(self, group):
        """Ids des couches sous `group`, de haut en bas, sous-groupes compris."""
        return [node.layerId() for node in group.findLayers()]

    def new_group(self, name):
        return self.root().addGroup(name)

    def move_group_to_end(self, group):
        # QGIS ne déplace pas un nœud : on pose une copie, puis on retire l'original. Les
        # couches restent au projet, la copie les citant encore quand l'original part.
        clone = group.clone()
        self.root().addChildNode(clone)
        group.parent().removeChildNode(group)
        return clone

    def move_layer_to_top(self, layer_id, group):
        """Pose la couche en tête de `group` (la racine si `group` vaut None)."""
        target = group if group is not None else self.root()
        node = self.root().findLayer(layer_id)
        clone = node.clone()
        target.insertChildNode(0, clone)
        node.parent().removeChildNode(node)

    def remove_group(self, group):
        group.parent().removeChildNode(group)


# Libellés de légende des rasters (demande du propriétaire, 2026-11-08), lus par
# `layout_export.apply_legend_labels` dans ces propriétés de la couche : la rampe d'altitudes
# est en mètres par contrat (`StyleSpec`), l'ombrage n'a pas de légende utile.
ELEVATION_LEGEND = "Altitude (m)"
HILLSHADE_LEGEND = "Ombrage"
LEGEND_TITLE = "cassini/legend_title"
LEGEND_FIRST_LABEL = "cassini/legend_first_label"
LEGEND_NODES = "cassini/legend_nodes"
# Bornes de la barre d'altitudes, « basse haute » en mètres : sa présence dit que la couche
# porte la légende d'altitudes de Cassini.
ELEVATION_SPAN = "cassini/elevation_span"
STATS_SAMPLE = 250_000


def _legend_labels(layer, title=None, first_label=None, no_nodes=False, span=None):
    """Pose ou retire les libellés de légende de la couche : un style neuf efface ceux d'avant.
    Ce sont des propriétés de la couche, enregistrées avec le projet."""
    for key, value in (
        (LEGEND_TITLE, title),
        (LEGEND_FIRST_LABEL, first_label),
        (LEGEND_NODES, "none" if no_nodes else None),
        (ELEVATION_SPAN, None if span is None else "%r %r" % (span[0], span[1])),
    ):
        if value is None:
            layer.removeCustomProperty(key)
        else:
            layer.setCustomProperty(key, value)


# Légendes posées sur une couche, tenues ici : une légende écrite en Python doit vivre aussi
# longtemps que la couche, sinon QGIS la verrait vide.
_LIVE_LEGENDS = {}


def _set_legend(layer, legend):
    _LIVE_LEGENDS[layer.id()] = legend
    layer.setLegend(legend)


def install_legend(layer):
    """Repose la légende de Cassini d'une couche d'après ses propriétés, si elle ne la porte pas.

    Une légende écrite en Python vit en mémoire : un projet rouvert, sans nouveau
    `layer.set_style`, retrouve la légende par défaut de QGIS (barre de 0 à 4000 m, sans le
    titre « Altitude (m) », couche d'ombrage sans nœud). `layout_export` appelle cette fonction
    avant de monter la légende de la carte. Sans propriété de Cassini, rien ne change."""
    from qgis.core import QgsRasterLayer

    if not isinstance(layer, QgsRasterLayer):
        return
    if _LEGEND_CLASS and isinstance(layer.legend(), _LEGEND_CLASS[0]):
        return  # déjà posée dans cette session
    span = layer.customProperty(ELEVATION_SPAN)
    renderer = layer.renderer()
    if span and renderer is not None and renderer.type() == "singlebandpseudocolor":
        try:
            low, high = (float(v) for v in str(span).split())
        except ValueError:
            return
        shader = renderer.shader().rasterShaderFunction()
        _set_legend(layer, _elevation_legend(shader, (low, high), shader.legendSettings()))
    elif layer.customProperty(LEGEND_NODES) == "none":
        _set_legend(layer, _hillshade_legend())


def _default_raster_legend(layer):
    from qgis.core import QgsMapLayerLegend

    return QgsMapLayerLegend.defaultRasterLegend(layer)


_LEGEND_CLASS = []


def _legend(make_nodes):
    """Une légende de couche écrite ici : `make_nodes(nœud de couche)` rend ses nœuds."""
    from qgis.core import QgsMapLayerLegend

    if not _LEGEND_CLASS:

        class CassiniLegend(QgsMapLayerLegend):
            def __init__(self, make_nodes):
                super().__init__()
                self.make_nodes = make_nodes

            def createLayerTreeModelLegendNodes(self, node_layer):
                from qgis.PyQt import sip

                nodes = self.make_nodes(node_layer)
                for node in nodes:  # le modèle de légende les détruit, pas Python
                    sip.transferto(node, None)
                return nodes

        _LEGEND_CLASS.append(CassiniLegend)
    return _LEGEND_CLASS[0](make_nodes)


def _elevation_legend(shader, span, settings):
    """Légende d'un MNT : le titre « Altitude (m) », puis une barre continue de `span[0]` à
    `span[1]`, aux couleurs que la rampe dictée donne à ces altitudes. Affichage seul : le
    rendu de la couche n'en dépend pas (voir `QgisBackend.apply_elevation_ramp`)."""
    from qgis.core import (
        QgsColorRampLegendNode,
        QgsGradientColorRamp,
        QgsGradientStop,
        QgsSimpleLegendNode,
    )
    from qgis.PyQt.QtGui import QColor

    def colour(value):
        _, r, g, b, a = shader.shade(value)
        return QColor(r, g, b, a)

    low, high = span
    inner = [
        QgsGradientStop((item.value - low) / (high - low), item.color)
        for item in shader.colorRampItemList()
        if low < item.value < high
    ]
    bar = QgsGradientColorRamp(colour(low), colour(high), False, inner)
    return _legend(
        lambda node_layer: [
            QgsSimpleLegendNode(node_layer, ELEVATION_LEGEND),
            QgsColorRampLegendNode(node_layer, bar.clone(), settings, low, high),
        ]
    )


def _hillshade_legend():
    """Légende d'un ombrage : une seule ligne, « Ombrage », sans échelle de gris. Le rendu
    d'ombrage de QGIS n'a aucun nœud de légende : sans celui-ci, le filtre de la légende sur
    la carte ôterait la couche."""
    from qgis.core import QgsSimpleLegendNode

    return _legend(lambda node_layer: [QgsSimpleLegendNode(node_layer, HILLSHADE_LEGEND)])


def _multiply_blend():
    """Mode de fondu « produit » ; avec `_normal_blend`, seul accès aux énumérations de
    QPainter, par leur nom complet, admis par les deux séries de Qt."""
    from qgis.PyQt.QtGui import QPainter

    return QPainter.CompositionMode.CompositionMode_Multiply


def _normal_blend():
    from qgis.PyQt.QtGui import QPainter

    return QPainter.CompositionMode.CompositionMode_SourceOver


def _layer_or_fail(backend, layer_id):
    layer = backend.layer(layer_id) if isinstance(layer_id, str) and layer_id else None
    if layer is None:
        raise GestureError(LAYER_MISSING, "Aucune couche du projet ne porte cet identifiant.")
    return layer


class SetStyleGesture(OpenOutputGesture):
    """`layer.set_style` : applique un `StyleSpec` à une couche ; rend `{"layer_id"}`."""

    UNEXPECTED = "Échec inattendu de la mise en forme de la couche."

    def __init__(self, backend=None, schedule=None):
        super().__init__(backend if backend is not None else QgisBackend(), schedule)

    def _open(self):
        params = self._params or {}
        layer_id = params.get("layer_id")
        style = normalize_style(params.get("style"))
        backend = self._backend
        layer = _layer_or_fail(backend, layer_id)
        name = backend.name_of(layer)
        kind = style["kind"]
        target = TARGETS[kind]
        layer_kind = backend.kind_of(layer)
        geometry = backend.geometry_of(layer) if layer_kind == VECTOR else None
        if layer_kind != target or (target == VECTOR and geometry not in GEOMETRIES):
            wanted = "une couche vecteur avec géométrie" if target == VECTOR else "un raster"
            raise GestureError(
                GEOMETRY_MISMATCH,
                "Le style « %s » va à %s : la couche « %s » n'en est pas un."
                % (kind, wanted, name),
            )
        if kind in ("categories", "graduated"):
            field_kind = backend.field_kind(layer, style["field"])
            if field_kind is None or (kind == "graduated" and field_kind != "numeric"):
                why = "n'existe pas" if field_kind is None else "n'est pas numérique"
                raise GestureError(
                    FIELD_MISSING,
                    "Le champ « %s » de la couche « %s » %s." % (style["field"], name, why),
                )
        if kind == "single":
            backend.apply_single(layer, geometry, style["symbol"], style["opacity"])
        elif kind == "categories":
            backend.apply_categories(
                layer,
                geometry,
                style["field"],
                style["classes"],
                style["other"],
                style["opacity"],
            )
        elif kind == "graduated":
            backend.apply_graduated(
                layer,
                geometry,
                style["field"],
                style["method"],
                style["class_count"],
                style["ramp"],
                style["symbol"],
            )
        elif kind == "elevation_ramp":
            backend.apply_elevation_ramp(layer, style["stops"], style["opacity"])
        else:
            backend.apply_hillshade(layer, style["opacity"])
        if target == VECTOR:
            backend.apply_visibility(layer, style["max_scale"])
        return {"layer_id": layer_id}


def _refused(why):
    return GestureError(ARRANGE_INVALID, "Rangement refusé : %s." % why)


def normalize_groups(groups):
    """Vérifie les groupes d'un rangement ; rend `[(nom, [ids])]` ou lève `arrange.invalid`."""
    if not isinstance(groups, list) or not groups:
        raise _refused("aucun groupe à poser")
    out, seen_layers, seen_names = [], set(), set()
    for raw in groups:
        if not isinstance(raw, dict) or set(raw) - {"name", "layers"}:
            raise _refused("un groupe est mal formé")
        name, layers = raw.get("name"), raw.get("layers")
        if not isinstance(name, str) or not name.strip():
            raise _refused("un groupe n'a pas de nom")
        if name in seen_names:
            raise _refused("le groupe « %s » est cité deux fois" % name)
        if not isinstance(layers, list) or not layers:
            raise _refused("le groupe « %s » est vide" % name)
        for layer_id in layers:
            if not isinstance(layer_id, str) or not layer_id:
                raise _refused("une couche du groupe « %s » n'a pas d'identifiant" % name)
            if layer_id in seen_layers:
                raise _refused("une couche est citée deux fois")
            seen_layers.add(layer_id)
        seen_names.add(name)
        out.append((name, list(layers)))
    return out


class ArrangeGesture(OpenOutputGesture):
    """`layer.arrange` : range les couches en groupes ; rend `{"group_count", "layer_count"}`."""

    UNEXPECTED = "Échec inattendu du rangement des couches."

    def __init__(self, backend=None, schedule=None):
        super().__init__(backend if backend is not None else QgisBackend(), schedule)

    def _open(self):
        groups = normalize_groups((self._params or {}).get("groups"))
        backend = self._backend
        listed = set()
        # Tout ou rien : chaque couche citée est au projet et dans l'arbre, avant tout geste.
        for _, layers in groups:
            for layer_id in layers:
                _layer_or_fail(backend, layer_id)
                if not backend.has_tree_node(layer_id):
                    raise GestureError(
                        LAYER_MISSING,
                        "La couche « %s » n'est pas dans l'arbre des couches."
                        % backend.name_of(backend.layer(layer_id)),
                    )
                listed.add(layer_id)
        # Les groupes qui portent des couches avant le rangement, par leur chemin de noms
        # (les copies faites pour déplacer un groupe ne gardent pas l'identité Python).
        filled_before = {
            backend.group_path(g) for g in backend.all_groups() if backend.has_layers(g)
        }
        available = list(backend.root_groups())
        for name, layers in groups:
            reused = next((g for g in available if backend.group_name(g) == name), None)
            if reused is not None:
                available = [g for g in available if g is not reused]
                # Les couches non listées du groupe repris remontent au-dessus des groupes.
                for layer_id in reversed(backend.layers_in(reused)):
                    if layer_id not in listed:
                        backend.move_layer_to_top(layer_id, None)
                group = backend.move_group_to_end(reused)
            else:
                group = backend.new_group(name)
            # De bas en haut, chaque couche posée en tête : l'ordre dicté se lit de haut en bas.
            for layer_id in reversed(layers):
                backend.move_layer_to_top(layer_id, group)
        # Les plus profonds d'abord : un sous-groupe vidé part avant son parent, qui part à son
        # tour s'il ne garde plus rien. Un groupe déjà vide avant le rangement reste.
        for group in reversed(backend.all_groups()):
            if not backend.has_children(group) and backend.group_path(group) in filled_before:
                backend.remove_group(group)
        return {"group_count": len(groups), "layer_count": len(listed)}
