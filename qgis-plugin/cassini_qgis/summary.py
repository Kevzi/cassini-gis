"""Une phrase claire par commande : ce qu'une étape a fait exactement.

Sans Qt. Affichage seul (AD-4, AD-5) : la phrase ne fait que lire les paramètres
envoyés et le résultat déjà rendu, elle ne décide rien. Aucun chemin absolu ni code
n'est produit (AD-9) : une couche s'affiche par son nom, un fichier par son seul nom.

- `layer_name(id)` rend le nom de la couche dans QGIS, ou None ;
- `step_names` : {k: résultat de l'étape k}, pour les références `step:k` et
  `step:k.champ` d'un plan.

Résultat absent ou illisible : phrase des seuls paramètres. Commande hors table, ou
paramètres illisibles : titre court de la description du registre (`short_title`), puis
« Terminée ». La description entière, écrite pour le modèle, n'est jamais montrée.

`label` rend la phrase « avant résultat » : c'est le libellé d'une étape du plan et d'une
ligne du journal.
"""

import math
import re

try:  # importé à plat par les tests, en paquet par QGIS
    from . import runlog, texts
except ImportError:  # pragma: no cover
    import runlog
    import texts

_STEP_REF = re.compile(r"^step:(\d+)(?:\.(.+))?$")
_SENTENCE_END = re.compile(r"\.(?:\s|$)")
_SHORT_MAX = 80  # un titre plus long est coupé au mot, suivi de « … »
_TITLE_END = re.compile(r":\s")  # « Pente d'un MNT : … », jamais « EPSG:4326 »
_ASIDE = re.compile(r"\s*[(;]")  # parenthèse ou point-virgule : précisions pour le modèle
# Une référence d'étape substituée (`sum_step_result`) est bornée par ces deux marques, pour
# que la contraction « de le » → « du » ne touche qu'elle, jamais le texte de l'utilisateur.
_REF_OPEN, _REF_CLOSE = "\ue000", "\ue001"
_CONTRACTIONS = (
    (re.compile(r"\bde " + _REF_OPEN + "le "), _REF_OPEN + "du "),
    (re.compile(r"\bà " + _REF_OPEN + "le "), _REF_OPEN + "au "),
)


def _finish(lang, text):
    """Contracte « de le », « à le » devant une référence d'étape, puis ôte les marques."""
    if texts.normalize(lang) == "fr":
        for pattern, replacement in _CONTRACTIONS:
            text = pattern.sub(replacement, text)
    return text.replace(_REF_OPEN, "").replace(_REF_CLOSE, "")


class _Unreadable(Exception):
    """Paramètres illisibles : la phrase se replie sur la description du registre."""


def _is_int(n):
    return isinstance(n, int) and not isinstance(n, bool)


def _is_number(n):
    return isinstance(n, (int, float)) and not isinstance(n, bool)


def count(lang, unit, n):
    """`8 entités`, `1 entité` ; FR : 0 et 1 au singulier, EN : 1 seul."""
    one = abs(n) < 2 if texts.normalize(lang) == "fr" else n == 1
    return texts.tr(lang, "unit_%s_%s" % (unit, "one" if one else "many"), n=n)


def number(lang, value):
    """`500`, `0,5` en français, `0.5` en anglais."""
    if not _is_number(value):
        raise _Unreadable()
    if float(value).is_integer():
        return str(int(value))
    text = repr(float(value))  # sans arrondi : 12345.75 reste 12345.75
    return text.replace(".", ",") if texts.normalize(lang) == "fr" else text


def file_name(path):
    """Le seul nom du fichier, jamais son dossier (AD-9)."""
    if not isinstance(path, str) or not path.strip():
        raise _Unreadable()
    return re.split(r"[\\/]", path.rstrip("\\/"))[-1]


def _raster(value, names):
    """Un raster cité : son nom de fichier sans dossier ni extension (`mnt-74-25m`), ou
    l'étape qui l'a produit pour une référence `step:k.file`."""
    if isinstance(value, str) and _STEP_REF.match(value):
        return names.layer(value)
    name = file_name(value)
    stem = re.sub(r"\.[A-Za-z0-9]{1,5}$", "", name)
    return stem or name


def _cells(lang, n):
    """Un nombre de cellules : `12 345` en français, `12,345` en anglais."""
    if not _is_int(n):
        raise _Unreadable()
    return scale_number(lang, n)


def _rounded(lang, value):
    """Une valeur rendue au dixième (`152,3`)."""
    if not _is_number(value) or not math.isfinite(value):
        raise _Unreadable()
    return number(lang, round(float(value), 1))


def _file_part(lang, r):
    """` : fichier pente.tif` ; fichier illisible : rien."""
    try:
        return texts.tr(lang, "sum_file", file=file_name(r.get("file")))
    except _Unreadable:
        return ""


def _range_part(lang, r, symbol=""):
    """`, de 0 à 152,3 %` quand min et max sont rendus ; sinon rien."""
    low, high = r.get("min"), r.get("max")
    if not (_is_number(low) and _is_number(high)):
        return ""
    return texts.tr(
        lang, "sum_range", min=_rounded(lang, low), max=_rounded(lang, high) + symbol
    )


def _text(value):
    """Texte de l'utilisateur (titre, valeur) : un chemin absolu y est masqué."""
    if not isinstance(value, str):
        raise _Unreadable()
    return runlog.redact(value)


class _Names:
    """Nom affiché d'une couche citée par un paramètre."""

    def __init__(self, lang, layer_name, step_names):
        self.lang = lang
        self.layer_name = layer_name
        self.step_names = step_names if isinstance(step_names, dict) else {}
        # Libellé d'une étape pas encore faite (`label`) : forme d'action, jamais au passé.
        # Une étape finie sans résultat lisible garde la forme au passé (`sentence`).
        self.ahead = False

    def _step_result(self, k):
        result = self.step_names.get(k)
        if result is None:
            result = self.step_names.get(str(k))
        return result

    def layer(self, value):
        if not isinstance(value, str) or not value:
            raise _Unreadable()
        ref = _STEP_REF.match(value)
        if ref:
            k = int(ref.group(1))
            result = self._step_result(k)
            name = result.get("name") if isinstance(result, dict) else None
            if isinstance(name, str) and name:
                return name
            return _REF_OPEN + texts.tr(self.lang, "sum_step_result", n=k) + _REF_CLOSE
        name = None
        if self.layer_name is not None:
            try:
                name = self.layer_name(value)
            except Exception:
                name = None
        if isinstance(name, str) and name:
            return name
        for result in self.step_names.values():  # couche rendue par une étape du plan
            if isinstance(result, dict) and result.get("layer_id") == value:
                rendered = result.get("name")
                if isinstance(rendered, str) and rendered:
                    return rendered
        return value


def _out_layer(lang, result):
    """` : nouvelle couche Tampon, 8 entités` ; illisible : rien."""
    name = result.get("name")
    n = result.get("feature_count")
    if not isinstance(name, str) or not name or not _is_int(n):
        return ""
    return texts.tr(lang, "sum_out_layer", name=name, count=count(lang, "entities", n))


# -- une fonction par commande : (lang, params, result ou None, noms) -> phrase sans point final
def _buffer(lang, p, r, names):
    distance = p.get("distance")
    if not isinstance(distance, dict) or distance.get("unit") not in ("m", "km"):
        raise _Unreadable()
    text = texts.tr(
        lang,
        "sum_buffer",
        distance="%s %s" % (number(lang, distance.get("value")), distance["unit"]),
        layer=names.layer(p.get("layer_id")),
    )
    if p.get("dissolve") is True:
        text += texts.tr(lang, "sum_dissolved")
    return text + (_out_layer(lang, r) if r else "")


def _intersect(lang, p, r, names):
    text = texts.tr(
        lang,
        "sum_intersect",
        layer=names.layer(p.get("layer_id")),
        overlay=names.layer(p.get("overlay_layer_id")),
    )
    return text + (_out_layer(lang, r) if r else "")


_OPERATORS = ("eq", "ne", "lt", "lte", "gt", "gte", "contains", "begins_with")


def _select(lang, p, r, names):
    operator = p.get("operator")
    field = p.get("field")
    if operator not in _OPERATORS or not isinstance(field, str) or not field:
        raise _Unreadable()
    text = texts.tr(
        lang,
        "sum_select",
        layer=names.layer(p.get("layer_id")),
        field=field,
        op=texts.tr(lang, "sum_op_" + operator),
        value=_text(p.get("value")),
    )
    return text + (_out_layer(lang, r) if r else "")


def _isochrone(lang, p, r, names):
    duration = p.get("duration")
    mode = p.get("mode")
    if not isinstance(duration, dict) or duration.get("unit") not in ("s", "min"):
        raise _Unreadable()
    if mode not in ("walk", "car"):
        raise _Unreadable()
    text = texts.tr(
        lang,
        "sum_isochrone",
        duration="%s %s" % (number(lang, duration.get("value")), duration["unit"]),
        mode=texts.tr(lang, "sum_mode_" + mode),
        layer=names.layer(p.get("layer_id")),
    )
    return text + (_out_layer(lang, r) if r else "")


def _zonal(lang, p, r, names):
    text = texts.tr(
        lang,
        "sum_zonal",
        points=names.layer(p.get("points_layer_id")),
        layer=names.layer(p.get("layer_id")),
    )
    if not r:
        return text
    text += _out_layer(lang, r)
    if r.get("truncated") is not False:
        return text  # zones tronquées (100 premières) : la plus fournie n'est pas sûre
    zones = r.get("zones")
    best = None
    for zone in zones if isinstance(zones, list) else []:
        if (
            isinstance(zone, dict)
            and isinstance(zone.get("label"), str)
            and _is_int(zone.get("count"))
            and (best is None or zone["count"] > best["count"])
        ):
            best = zone
    if best is not None and best["count"] > 0:
        text += texts.tr(
            lang, "sum_zonal_top", label=best["label"], count=count(lang, "points", best["count"])
        )
    return text


def _layer_add(lang, p, r, names):
    file = file_name(p.get("path"))
    name = r.get("name") if r else None
    if not isinstance(name, str) or not name:
        return texts.tr(lang, "sum_layer_add", file=file)
    text = texts.tr(lang, "sum_layer_added", file=file, name=name)
    source, project = r.get("source_crs"), r.get("project_crs")
    if r.get("reprojected") is True and isinstance(source, str) and isinstance(project, str):
        text += texts.tr(lang, "sum_reprojected", source=source, target=project)
    return text


def _layer_describe(lang, p, r, names):
    name = r.get("name") if r else None
    layer = name if isinstance(name, str) and name else names.layer(p.get("layer_id"))
    text = texts.tr(lang, "sum_describe", layer=layer)
    if not r:
        return text
    parts = []
    if _is_int(r.get("feature_count")):
        parts.append(count(lang, "entities", r["feature_count"]))
    if isinstance(r.get("fields"), list):
        parts.append(count(lang, "fields", len(r["fields"])))
    if isinstance(r.get("geometry"), str) and r["geometry"]:
        parts.append(texts.tr(lang, "sum_geometry", geometry=r["geometry"]))
    if parts:
        text += texts.tr(lang, "sum_colon") + ", ".join(parts)
    return text


def scale_number(lang, n):
    """`25 000` en français, `25,000` en anglais."""
    grouped = "{:,}".format(n)
    return grouped.replace(",", " ") if texts.normalize(lang) == "fr" else grouped


def _export_pdf(lang, p, r, names):
    key = "sum_export_pdf_ask" if names.ahead else "sum_export_pdf"
    text = texts.tr(lang, key, title=_text(p.get("title")))
    # L'échelle tenue, rendue par le geste seulement si elle a été demandée (ticket 5.38) ;
    # sans échelle lisible au résultat (absent ou incomplet), celle des paramètres.
    scale = r.get("scale") if r else None
    if not (_is_int(scale) and scale > 0):
        scale = p.get("scale")
    if _is_int(scale) and scale > 0:
        text += texts.tr(lang, "sum_pdf_scale", scale=scale_number(lang, scale))
    if not r:
        return text
    try:
        file = file_name(r.get("file"))
    except _Unreadable:
        return text
    pages = r.get("page_count")
    if _is_int(pages):
        return text + texts.tr(
            lang, "sum_pdf_file_pages", file=file, pages=count(lang, "pages", pages)
        )
    return text + texts.tr(lang, "sum_pdf_file", file=file)


def _inventory(lang, p, r, names):
    text = texts.tr(lang, "sum_inventory")
    layers = r.get("layers") if r else None
    if isinstance(layers, list):
        text += texts.tr(lang, "sum_colon") + count(lang, "layers", len(layers))
    return text


def _model_list(lang, p, r, names):
    text = texts.tr(lang, "sum_models")
    models = r.get("models") if r else None
    if isinstance(models, list):
        text += texts.tr(lang, "sum_colon") + count(lang, "models", len(models))
    return text


_FORMS = {"wgs84": "WGS84", "utm": "UTM", "mgrs": "MGRS"}


def _degrees(lang, value, positive, negative):
    """`48,8565567° N` : sept décimales (le centimètre), hémisphère en lettre."""
    if not _is_number(value):
        raise _Unreadable()
    text = "%.7f" % abs(value)
    if texts.normalize(lang) == "fr":
        text = text.replace(".", ",")
        negative = negative.replace("W", "O")
    return "%s° %s" % (text, positive if value >= 0 else negative)


def _metres(value):
    """Mètres entiers, tronqués comme le MGRS rendu à côté (`5411750` pour 5411750,6)."""
    if not _is_number(value):
        raise _Unreadable()
    return str(int(math.floor(value)))


def _utm_zone(epsg):
    """`31N` pour EPSG:32631, `31S` pour EPSG:32731."""
    if _is_int(epsg) and 32601 <= epsg <= 32660:
        return "%dN" % (epsg - 32600)
    if _is_int(epsg) and 32701 <= epsg <= 32760:
        return "%dS" % (epsg - 32700)
    raise _Unreadable()


def _coord_convert(lang, p, r, names):
    """Les formes rendues, dans l'ordre demandé ; affichage seul, aucun calcul (AD-5)."""
    to = p.get("to")
    if not isinstance(to, list) or not to or any(f not in _FORMS for f in to):
        raise _Unreadable()
    asked = texts.tr(lang, "sum_convert", forms=", ".join(_FORMS[f] for f in to))
    if not r:
        return asked
    parts = []
    for form in to:
        value = r.get(form)
        if form == "mgrs" and isinstance(value, str) and value:
            parts.append(texts.tr(lang, "sum_convert_mgrs", ref=value))
        elif form == "wgs84" and isinstance(value, dict):
            parts.append(
                texts.tr(
                    lang,
                    "sum_convert_wgs84",
                    lat=_degrees(lang, value.get("y"), "N", "S"),
                    lon=_degrees(lang, value.get("x"), "E", "W"),
                )
            )
        elif form == "utm" and isinstance(value, dict):
            parts.append(
                texts.tr(
                    lang,
                    "sum_convert_utm",
                    zone=_utm_zone(value.get("epsg")),
                    x=_metres(value.get("x")),
                    y=_metres(value.get("y")),
                )
            )
    if not parts:
        return asked
    return texts.tr(
        lang, "sum_converted", parts=texts.tr(lang, "sum_convert_sep").join(parts)
    )


def _angle(lang, value):
    """`45°`, `141,75°`, `50 gon` : affichage au dix-millième d'unité."""
    if not isinstance(value, dict) or not _is_number(value.get("value")):
        raise _Unreadable()
    unit = value.get("unit", "deg")
    if unit not in ("deg", "gon"):
        raise _Unreadable()
    shown = number(lang, round(float(value["value"]), 4))
    return shown + ("°" if unit == "deg" else " gon")


def _length(lang, value):
    """`39,528 m`, `660,5 km` : au-delà de 1000 m, en kilomètres au mètre près."""
    if not isinstance(value, dict) or value.get("unit") not in ("m", "km"):
        raise _Unreadable()
    v = value.get("value")
    if not _is_number(v):
        raise _Unreadable()
    metres = float(v) * (1000 if value["unit"] == "km" else 1)
    if abs(metres) >= 1000:
        return "%s km" % number(lang, round(metres / 1000, 3))
    return "%s m" % number(lang, round(metres, 3))


def _point(lang, value, any_system=False):
    """Un point rendu : latitude puis longitude en WGS84, ou zone et mètres en UTM ; avec
    `any_system`, un autre système par x et y nommés (`x 700000 ; y 6600000`), son code étant
    déjà nommé par la phrase."""
    if not isinstance(value, dict):
        raise _Unreadable()
    epsg = value.get("epsg")
    if any_system and _is_int(epsg) and epsg != 4326 and not (
        32601 <= epsg <= 32660 or 32701 <= epsg <= 32760
    ):
        return texts.tr(
            lang,
            "sum_point_xy",
            x=_coordinate(lang, value.get("x")),
            y=_coordinate(lang, value.get("y")),
        )
    if epsg == 4326:
        return "%s, %s" % (
            _degrees(lang, value.get("y"), "N", "S"),
            _degrees(lang, value.get("x"), "E", "W"),
        )
    return texts.tr(
        lang,
        "sum_geo_utm",
        zone=_utm_zone(value.get("epsg")),
        x=_metres(value.get("x")),
        y=_metres(value.get("y")),
    )


def _coordinate(lang, value):
    """Une coordonnée d'un système quelconque : au millimètre au-delà de 1000 (des mètres),
    sinon à sept décimales (des degrés, le centimètre)."""
    if not _is_number(value) or not math.isfinite(value):
        raise _Unreadable()
    return number(lang, round(float(value), 3 if abs(value) >= 1000 else 7))


def _transform_point(lang, p, r, names):
    """Le point reprojeté par QGIS ; affichage seul, aucun calcul (AD-5)."""
    target = p.get("target_epsg")
    if not _is_int(target):
        raise _Unreadable()
    if not r:
        return texts.tr(lang, "sum_transform_point", epsg=target)
    point = r.get("point")
    if not isinstance(point, dict) or point.get("epsg") != target:
        raise _Unreadable()
    return texts.tr(lang, "sum_transformed_point", epsg=target, point=_point(lang, point, any_system=True))


def _reproject(lang, p, r, names):
    """La couche reprojetée par QGIS, en nouvelle couche ; affichage seul (AD-5)."""
    target = p.get("target_epsg")
    if not _is_int(target):
        raise _Unreadable()
    key = "sum_reproject_ask" if names.ahead else "sum_reproject"
    text = texts.tr(lang, key, layer=names.layer(p.get("layer_id")), epsg=target)
    return text + (_out_layer(lang, r) if r else "")


_FORMAT_NAMES = {"gpkg": "GeoPackage", "shp": "Shapefile", "geojson": "GeoJSON"}


def _data_convert(lang, p, r, names):
    """La couche écrite par QGIS dans un autre format, en fichier ; affichage seul (AD-5)."""
    shown = _FORMAT_NAMES.get(p.get("format"))
    if shown is None:
        raise _Unreadable()
    key = "sum_data_convert_ask" if names.ahead else "sum_data_convert"
    text = texts.tr(lang, key, layer=names.layer(p.get("layer_id")), format=shown)
    if not r:
        return text
    try:
        file = file_name(r.get("file"))
    except _Unreadable:
        return text
    n = r.get("feature_count")
    if _is_int(n):
        return text + texts.tr(
            lang, "sum_data_convert_file", file=file, count=count(lang, "entities", n)
        )
    return text + texts.tr(lang, "sum_file", file=file)


def _layer_remove(lang, p, r, names):
    """La couche retirée du projet, aucun fichier effacé ; affichage seul (AD-5)."""
    key = "sum_layer_remove_ask" if names.ahead else "sum_layer_remove"
    return texts.tr(lang, key, layer=names.layer(p.get("layer_id")))


def _grid(p):
    """Suffixe des textes du nord employé : `_grid` pour un gisement (`north` vaut `grid`),
    `_magnetic` pour un azimut magnétique ; affichage seul (AD-5)."""
    north = p.get("north")
    if not isinstance(north, str):
        return ""
    return {"grid": "_grid", "magnetic": "_magnetic"}.get(north, "")


def _magnetic_tail(lang, r):
    """Déclinaison et date d'un azimut magnétique, puis l'avertissement du noyau s'il y en a
    un, recopié tel quel (date hors de la validité du WMM2025, pôle magnétique proche) ;
    affichage seul (AD-5)."""
    text = ""
    if isinstance(r.get("date"), str) and isinstance(r.get("declination"), dict):
        text += texts.tr(
            lang,
            "sum_geo_declination",
            declination=_angle(lang, r.get("declination")),
            date=r["date"],
        )
    warning = r.get("warning")
    if isinstance(warning, str) and warning.strip():
        text += texts.tr(lang, "sum_geo_warning", warning=warning.strip().rstrip("."))
    return text


def _distance_azimuth(lang, p, r, names):
    """Distance et azimuts rendus, « gisement » pour le nord du quadrillage, « azimut
    magnétique » pour le nord magnétique ; affichage seul (AD-5)."""
    grid = _grid(p)
    if not r:
        return texts.tr(lang, "sum_geo_measure" + grid)
    return texts.tr(
        lang,
        "sum_geo_measured" + grid,
        distance=_length(lang, r.get("distance")),
        azimuth=_angle(lang, r.get("azimuth")),
        back=_angle(lang, r.get("back_azimuth")),
    ) + _magnetic_tail(lang, r)


def _destination_point(lang, p, r, names):
    grid = _grid(p)
    distance = _length(lang, p.get("distance"))
    azimuth = _angle(lang, p.get("azimuth"))
    if not r:
        return texts.tr(
            lang, "sum_geo_destination" + grid, distance=distance, azimuth=azimuth
        )
    return texts.tr(
        lang,
        "sum_geo_reached" + grid,
        distance=distance,
        azimuth=azimuth,
        point=_point(lang, r.get("point")),
    ) + _magnetic_tail(lang, r)


def _bearing_intersection(lang, p, r, names):
    first = _angle(lang, p.get("first_azimuth"))
    second = _angle(lang, p.get("second_azimuth"))
    if not r:
        return texts.tr(lang, "sum_geo_intersection", first=first, second=second)
    return texts.tr(
        lang,
        "sum_geo_intersected",
        first=first,
        second=second,
        point=_point(lang, r.get("point")),
        d1=_length(lang, r.get("distance_from_first")),
        d2=_length(lang, r.get("distance_from_second")),
    )


# -- commandes de terrain et de raster (noyau), de style et de rangement (QGIS)
def _about(lang, p, r, names):
    return texts.tr(lang, "sum_about")


def _open_raster(lang, p, r, names):
    file = file_name(p.get("path"))
    name = r.get("name") if r else None
    if not isinstance(name, str) or not name:
        return texts.tr(lang, "sum_open_raster", file=file)
    return texts.tr(lang, "sum_opened_raster", file=file, name=name)


_SLOPE_SYMBOLS = {"percent": " %", "degrees": "°"}


def _slope(lang, p, r, names):
    unit = p.get("unit")
    if unit not in _SLOPE_SYMBOLS:
        raise _Unreadable()
    text = texts.tr(
        lang,
        "sum_slope",
        dem=_raster(p.get("dem"), names),
        unit=texts.tr(lang, "sum_slope_" + unit),
    )
    if not r:
        return text
    return text + _file_part(lang, r) + _range_part(lang, r, _SLOPE_SYMBOLS[unit])


def _aspect(lang, p, r, names):
    text = texts.tr(lang, "sum_aspect", dem=_raster(p.get("dem"), names))
    if not r:
        return text
    text += _file_part(lang, r)
    flat = r.get("flat_cells")
    if _is_int(flat) and flat > 0:
        text += texts.tr(lang, "sum_flat", n=_cells(lang, flat))
    return text


def _hillshade(lang, p, r, names):
    text = texts.tr(lang, "sum_hillshade", dem=_raster(p.get("dem"), names))
    return text + (_file_part(lang, r) if r else "")


def _viewshed(lang, p, r, names):
    text = texts.tr(lang, "sum_viewshed", dem=_raster(p.get("dem"), names))
    if not r:
        return text
    text += _file_part(lang, r)
    visible, hidden = r.get("visible_cells"), r.get("hidden_cells")
    if _is_int(visible) and _is_int(hidden):
        text += texts.tr(
            lang, "sum_viewshed_cells", visible=_cells(lang, visible), hidden=_cells(lang, hidden)
        )
    return text


def _viewshed_cumulative(lang, p, r, names):
    observers = p.get("observers")
    if not isinstance(observers, list) or not observers:
        raise _Unreadable()
    text = texts.tr(
        lang,
        "sum_viewshed_cumulative",
        observers=count(lang, "observers", len(observers)),
        dem=_raster(p.get("dem"), names),
    )
    if not r:
        return text
    text += _file_part(lang, r)
    visible, most = r.get("visible_cells"), r.get("max_count")
    if _is_int(visible) and _is_int(most):
        text += texts.tr(
            lang,
            "sum_viewshed_seen",
            visible=_cells(lang, visible),
            max=count(lang, "observers", most),
        )
    return text


def _metres_length(lang, value):
    """Une distance rendue en mètres par le noyau (`1,2 km`, `850 m`)."""
    return _length(lang, {"value": value, "unit": "m"})


def _line_of_sight(lang, p, r, names):
    text = texts.tr(lang, "sum_line_of_sight", dem=_raster(p.get("dem"), names))
    if not r:
        return text
    visible = r.get("visible")
    try:
        distance = _metres_length(lang, r.get("distance"))
        if visible is True:
            return text + texts.tr(lang, "sum_los_visible", distance=distance)
        if visible is False:
            obstruction = _metres_length(lang, r.get("obstruction_distance"))
            return text + texts.tr(
                lang, "sum_los_hidden", obstruction=obstruction, distance=distance
            )
    except _Unreadable:
        pass
    return text


def _peaks(lang, p, r, names):
    text = texts.tr(lang, "sum_peaks", dem=_raster(p.get("dem"), names))
    if not r:
        return text
    n = r.get("feature_count")
    try:
        file = file_name(r.get("file"))
    except _Unreadable:
        return text
    if not _is_int(n):
        return text + texts.tr(lang, "sum_file", file=file)
    text += texts.tr(lang, "sum_peaks_found", file=file, count=count(lang, "peaks", n))
    peaks = r.get("peaks")
    first = peaks[0] if isinstance(peaks, list) and peaks else None
    if isinstance(first, dict) and _is_number(first.get("elevation")):
        text += texts.tr(lang, "sum_peaks_top", elevation=_rounded(lang, first["elevation"]))
    return text


def _cost_distance(lang, p, r, names):
    sources = p.get("sources")
    if not isinstance(sources, list) or not sources:
        raise _Unreadable()
    text = texts.tr(
        lang,
        "sum_cost_distance",
        sources=count(lang, "sources", len(sources)),
        cost=_raster(p.get("cost"), names),
    )
    if not r:
        return text
    text += _file_part(lang, r)
    reached = r.get("reachable_cells")
    if _is_int(reached):
        text += texts.tr(lang, "sum_cost_reached", n=_cells(lang, reached))
    return text


def _least_cost_path(lang, p, r, names):
    text = texts.tr(lang, "sum_least_cost_path", cost=_raster(p.get("cost"), names))
    if not r:
        return text
    try:
        return text + texts.tr(
            lang,
            "sum_path_found",
            file=file_name(r.get("file")),
            length=_length(lang, r.get("length")),
            total=_rounded(lang, r.get("total_cost")),
        )
    except _Unreadable:
        return text + _file_part(lang, r)


def _reclassify(lang, p, r, names):
    classes = p.get("classes")
    if not isinstance(classes, list) or not classes:
        raise _Unreadable()
    text = texts.tr(
        lang,
        "sum_reclassify",
        raster=_raster(p.get("raster"), names),
        classes=count(lang, "classes", len(classes)),
    )
    return text + (_file_part(lang, r) if r else "")


def _weighted_overlay(lang, p, r, names):
    criteria = p.get("criteria")
    if not isinstance(criteria, list) or not criteria:
        raise _Unreadable()
    text = texts.tr(
        lang, "sum_weighted_overlay", criteria=count(lang, "criteria", len(criteria))
    )
    if not r:
        return text
    return text + _file_part(lang, r) + _range_part(lang, r)


def _topographic_style(lang, p, r, names):
    styled = r.get("styled") if r else None
    if isinstance(styled, list):
        return texts.tr(lang, "sum_topo_styled", layers=count(lang, "layers", len(styled)))
    layers = p.get("layers")
    if isinstance(layers, list) and layers:
        target = count(lang, "layers", len(layers))
    elif layers is None or layers == []:
        target = texts.tr(lang, "sum_topo_all")
    else:
        raise _Unreadable()
    return texts.tr(lang, "sum_topo_style", target=target)


def _graduated_style(lang, p, r, names):
    field = p.get("field")
    if not isinstance(field, str) or not field:
        raise _Unreadable()
    text = texts.tr(lang, "sum_graduated", layer=names.layer(p.get("layer_id")), field=field)
    n = r.get("class_count") if r else None
    if _is_int(n):
        text += texts.tr(lang, "sum_graduated_classes", classes=count(lang, "classes", n))
    return text


def _set_style(lang, p, r, names):
    layer = names.layer(p.get("layer_id"))
    return texts.tr(lang, "sum_style_set" if r else "sum_set_style", layer=layer)


def _arrange(lang, p, r, names):
    groups = p.get("groups")
    if not isinstance(groups, list) or not groups:
        raise _Unreadable()
    if r and _is_int(r.get("group_count")) and _is_int(r.get("layer_count")):
        return texts.tr(
            lang,
            "sum_arranged",
            layers=count(lang, "layers", r["layer_count"]),
            groups=count(lang, "groups", r["group_count"]),
        )
    return texts.tr(lang, "sum_arrange", groups=count(lang, "groups", len(groups)))


_SENTENCES = {
    "system.about": _about,
    "terrain.slope": _slope,
    "terrain.aspect": _aspect,
    "terrain.hillshade": _hillshade,
    "terrain.viewshed": _viewshed,
    "terrain.viewshed_cumulative": _viewshed_cumulative,
    "terrain.line_of_sight": _line_of_sight,
    "terrain.peaks": _peaks,
    "terrain.cost_distance": _cost_distance,
    "terrain.least_cost_path": _least_cost_path,
    "raster.reclassify": _reclassify,
    "raster.weighted_overlay": _weighted_overlay,
    "map.topographic_style": _topographic_style,
    "map.graduated_style": _graduated_style,
    "layer.set_style": _set_style,
    "layer.arrange": _arrange,
    "layer.open_raster": _open_raster,
    "layer.open_vector": _layer_add,
    "coord.convert": _coord_convert,
    "geo.distance_azimuth": _distance_azimuth,
    "geo.destination_point": _destination_point,
    "geo.bearing_intersection": _bearing_intersection,
    "data.transform_point": _transform_point,
    "data.reproject": _reproject,
    "data.convert": _data_convert,
    "layer.remove": _layer_remove,
    "analysis.buffer": _buffer,
    "analysis.intersect": _intersect,
    "analysis.select_by_attribute": _select,
    "analysis.isochrone": _isochrone,
    "analysis.zonal_statistics": _zonal,
    "layer.add": _layer_add,
    "layer.describe": _layer_describe,
    "layout.export_pdf": _export_pdf,
    "project.inventory": _inventory,
    "model.list": _model_list,
}


def short_title(command, catalogue, lang):
    """Titre court d'une commande : la description du registre avant « : » quand elle en a
    un, sinon sa première phrase, sans point final ; jamais la description entière, écrite
    pour le modèle. Sans description : l'id de la commande."""
    text = ""
    for entry in catalogue or []:
        if isinstance(entry, dict) and entry.get("id") == command:
            text = texts.pick(lang, entry.get("description_fr", ""), entry.get("description_en", ""))
            break
    text = text.strip() if isinstance(text, str) else ""
    if not text:
        return command if isinstance(command, str) and command else "?"
    end = _SENTENCE_END.search(text)
    first = text[: end.start()] if end else text
    head = _TITLE_END.split(first, 1)[0].strip()
    title = head if head else first.strip()
    title = _ASIDE.split(title, 1)[0]
    title = title.rstrip(".,: ").strip()
    if len(title) > _SHORT_MAX:
        cut = title[:_SHORT_MAX].rsplit(" ", 1)[0].rstrip(",;")
        title = cut + "…"
    return title or (command if isinstance(command, str) and command else "?")


def fallback(command, lang, catalogue=None):
    """Titre court de la commande, puis « Terminée »."""
    title = short_title(command, catalogue, lang).rstrip("….")
    return "%s. %s." % (title, texts.tr(lang, "done"))


def label(command, params, lang, layer_name=None, step_names=None, catalogue=None):
    """Libellé d'une étape, avant son résultat : `Pente de mnt-74-25m en pourcentage`.

    Sans point final. Commande hors table ou paramètres illisibles : le titre court.
    """
    make = _SENTENCES.get(command)
    if make is not None and isinstance(params, dict):
        try:
            names = _Names(lang, layer_name, step_names)
            names.ahead = True
            return _finish(lang, make(lang, params, None, names))
        except Exception:  # _Unreadable compris : jamais d'exception jusqu'à l'utilisateur
            pass
    return short_title(command, catalogue, lang)


def sentence(command, params, result, lang, layer_name=None, step_names=None, catalogue=None):
    """Ce que la commande a fait, en une phrase, dans la langue `lang`."""
    if command == "system.about":
        return texts.tr(lang, "about_result")
    make = _SENTENCES.get(command)
    if make is None or not isinstance(params, dict):
        return fallback(command, lang, catalogue)
    names = _Names(lang, layer_name, step_names)
    readable = result if isinstance(result, dict) else None
    try:
        text = make(lang, params, readable, names)
    except _Unreadable:
        return fallback(command, lang, catalogue)
    except Exception:  # jamais d'exception jusqu'à l'utilisateur
        return fallback(command, lang, catalogue)
    return _finish(lang, text) + "."


def detail(item, label, lang, layer_name=None, refs=None, catalogue=None, error_text=None):
    """Texte affiché au clic sur une ligne du journal ou une étape du plan.

    `item` porte `command`, `params`, `status` et `result` (ligne `runlog.Entry` ou étape
    `agentview.Step`). Terminée : la phrase de la commande ; en erreur : `error_text`
    quand il est connu ; sinon le libellé et le statut, sans résultat.
    """
    status = item.status
    if status == runlog.DONE:
        return sentence(item.command, item.params, item.result, lang, layer_name, refs, catalogue)
    if status == runlog.ERROR and error_text:
        return error_text
    word = texts.tr(lang, status).rstrip("….").strip()  # « En cours… » : pas de « …. »
    return texts.tr(lang, "focus_status", label=label, status=word)
