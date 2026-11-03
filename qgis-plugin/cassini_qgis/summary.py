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


_SENTENCES = {
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
