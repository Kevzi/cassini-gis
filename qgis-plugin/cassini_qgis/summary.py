"""Une phrase claire par commande : ce qu'une étape a fait exactement.

Sans Qt. Affichage seul (AD-4, AD-5) : la phrase ne fait que lire les paramètres
envoyés et le résultat déjà rendu, elle ne décide rien. Aucun chemin absolu ni code
n'est produit (AD-9) : une couche s'affiche par son nom, un fichier par son seul nom.

- `layer_name(id)` rend le nom de la couche dans QGIS, ou None ;
- `step_names` : {k: résultat de l'étape k}, pour les références `step:k` et
  `step:k.champ` d'un plan.

Résultat absent ou illisible : phrase des seuls paramètres. Commande hors table, ou
paramètres illisibles : description du registre, puis « Terminée ».
"""

import math
import re

try:  # importé à plat par les tests, en paquet par QGIS
    from . import agentview, runlog, texts
except ImportError:  # pragma: no cover
    import agentview
    import runlog
    import texts

_STEP_REF = re.compile(r"^step:(\d+)(?:\.(.+))?$")


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
            return texts.tr(self.lang, "sum_step_result", n=k)
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
    text = texts.tr(lang, "sum_export_pdf", title=_text(p.get("title")))
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


def _distance_azimuth(lang, p, r, names):
    """Distance et azimuts rendus ; affichage seul (AD-5)."""
    if not r:
        return texts.tr(lang, "sum_geo_measure")
    return texts.tr(
        lang,
        "sum_geo_measured",
        distance=_length(lang, r.get("distance")),
        azimuth=_angle(lang, r.get("azimuth")),
        back=_angle(lang, r.get("back_azimuth")),
    )


def _destination_point(lang, p, r, names):
    distance = _length(lang, p.get("distance"))
    azimuth = _angle(lang, p.get("azimuth"))
    if not r:
        return texts.tr(lang, "sum_geo_destination", distance=distance, azimuth=azimuth)
    return texts.tr(
        lang,
        "sum_geo_reached",
        distance=distance,
        azimuth=azimuth,
        point=_point(lang, r.get("point")),
    )


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


_SENTENCES = {
    "coord.convert": _coord_convert,
    "geo.distance_azimuth": _distance_azimuth,
    "geo.destination_point": _destination_point,
    "geo.bearing_intersection": _bearing_intersection,
    "data.transform_point": _transform_point,
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


def fallback(command, lang, catalogue=None):
    """Description du registre, puis « Terminée »."""
    description = agentview.describe(command, catalogue, lang).strip().rstrip(".")
    return "%s. %s." % (description, texts.tr(lang, "done"))


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
    return text + "."


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
