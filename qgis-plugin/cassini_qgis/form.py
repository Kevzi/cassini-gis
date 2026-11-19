"""Formulaire engendré depuis le schéma JSON des paramètres d'une commande.

Sans Qt. Le panneau ne valide rien : le noyau valide. Sous-ensemble compris :
texte, nombre, entier, booléen, énumération (éventuellement « ou null »), liste de noms de
champs d'une couche (tableau de textes nommé `fields` ou `*_fields`). Toute
autre propriété reçoit un champ JSON brut, jamais un formulaire deviné.
"""

import json
import os

TEXT = "text"
NUMBER = "number"
INTEGER = "integer"
BOOLEAN = "boolean"
ENUM = "enum"
JSON = "json"
FIELD_LIST = "field_list"

# Paramètres remplis par le noyau (AD-10 ; au rejeu seulement : `leave_out`, ticket 5.36, et
# `center`, ticket 5.38 ; le système de calcul `work_epsg`, entrée 9.16 ; la mise en évidence
# du résultat d'un plan `focus`, entrée 7.9) : le formulaire ne les
# montre pas.
CORE_FILLED = ("out", "leave_out", "center", "work_epsg", "focus")


def offered(catalogue):
    """Les entrées du catalogue offertes au formulaire : toutes, sauf celles que le noyau dicte
    seul (`dictated_by_core` : `layer.open_raster`, ticket 4.9, et `layer.open_vector`, AD-30)."""
    return [
        entry for entry in catalogue or []
        if isinstance(entry, dict) and not entry.get("dictated_by_core")
    ]


class Field:
    def __init__(self, name, kind, required=False, choices=None, description=""):
        self.name = name
        self.kind = kind
        self.required = required
        self.choices = choices or []
        self.description = description


class FormError(Exception):
    """Saisie locale illisible : `key` est une clé de texts, `name` le champ."""

    def __init__(self, key, name):
        super().__init__(key)
        self.key = key
        self.name = name


def _kind(prop):
    if not isinstance(prop, dict):
        return JSON, []
    if {"$ref", "anyOf", "oneOf", "allOf", "properties", "items", "const"} & set(prop):
        return JSON, []
    enum = prop.get("enum")
    kind = prop.get("type")
    if isinstance(kind, list):
        rest = [k for k in kind if k != "null"]
        kind = rest[0] if len(rest) == 1 else None
    if isinstance(enum, list):
        values = [v for v in enum if v is not None]
        if values and all(isinstance(v, str) for v in values) and kind in (None, "string"):
            return ENUM, values
        return JSON, []
    if kind == "string":
        return TEXT, []
    if kind == "number":
        return NUMBER, []
    if kind == "integer":
        return INTEGER, []
    if kind == "boolean":
        return BOOLEAN, []
    return JSON, []


def _is_field_list(name, prop):
    """Un tableau de textes nommé `fields` ou `*_fields` désigne des champs (attributs) d'une couche."""
    if not (name == "fields" or name.endswith("_fields")) or not isinstance(prop, dict):
        return False
    items = prop.get("items")
    kind = prop.get("type")
    if isinstance(kind, list):
        kind = next((k for k in kind if k != "null"), None)
    return kind == "array" and isinstance(items, dict) and items.get("type") == "string"


def is_path_field(field):
    """Un champ texte nommé `path`, `table` (tableau CSV de `layer.add`) ou `*_path` désigne un fichier
    du projet."""
    return field.kind == TEXT and (field.name in ("path", "table") or field.name.endswith("_path"))


def is_layer_field(field):
    """Un champ texte nommé `layer_id` ou `*_layer_id` désigne une couche du projet."""
    return field.kind == TEXT and (field.name == "layer_id" or field.name.endswith("_layer_id"))


def is_field_field(field):
    """Un champ texte nommé `field` ou `*_field` désigne un champ (attribut) d'une couche."""
    return field.kind == TEXT and (field.name == "field" or field.name.endswith("_field"))


def relative_path(chosen, project_file):
    """Chemin de `chosen` relatif au projet, séparateurs `/` ; None si disques différents.

    Le canal ne porte que des chemins relatifs au projet (AD-9).
    """
    try:
        rel = os.path.relpath(chosen, os.path.dirname(project_file))
    except ValueError:
        return None
    return rel.replace("\\", "/")


def _follow(prop, schema):
    """Un `$ref` local vers une définition du schéma est remplacé par elle, les clés du champ
    (sa description) gardées : un `RasterPath` reste un champ texte. Sinon `prop` inchangé."""
    if not isinstance(prop, dict) or not isinstance(prop.get("$ref"), str):
        return prop
    for prefix, key in (("#/$defs/", "$defs"), ("#/definitions/", "definitions")):
        name = prop["$ref"][len(prefix):] if prop["$ref"].startswith(prefix) else None
        defs = schema.get(key)
        if name and isinstance(defs, dict) and isinstance(defs.get(name), dict):
            if "$ref" in defs[name]:
                return prop
            merged = dict(defs[name])
            merged.update({k: v for k, v in prop.items() if k != "$ref"})
            return merged
    return prop


def build_fields(schema):
    """Liste des champs d'un schéma d'objet (vide si aucune propriété)."""
    if not isinstance(schema, dict):
        return []
    props = schema.get("properties")
    if not isinstance(props, dict):
        return []
    required = schema.get("required")
    required = set(required) if isinstance(required, list) else set()
    fields = []
    for name in sorted(props):
        if name in CORE_FILLED:
            continue
        prop = _follow(props[name], schema)
        kind, choices = _kind(prop)
        if _is_field_list(name, prop):
            kind = FIELD_LIST
        description = prop.get("description", "") if isinstance(prop, dict) else ""
        fields.append(Field(name, kind, name in required, choices, str(description)))
    return fields


def build_params(fields, values):
    """Construit `params` depuis les valeurs saisies (`nom -> str, bool ou liste de noms`).

    Un champ facultatif vide est omis. Lève FormError pour une saisie que le
    panneau ne sait pas convertir (JSON brut invalide, nombre illisible) : rien
    n'est alors envoyé. Aucune autre vérification : le noyau valide.
    """
    params = {}
    for f in fields:
        raw = values.get(f.name)
        if f.kind == BOOLEAN:
            params[f.name] = bool(raw)
            continue
        if f.kind == FIELD_LIST:
            names = [n for n in raw if isinstance(n, str)] if isinstance(raw, (list, tuple)) else []
            if names or f.required:
                params[f.name] = names  # une liste vide exigée est refusée par le noyau
            continue
        text = (raw if isinstance(raw, str) else "").strip()
        if not text:
            if f.required and f.kind in (TEXT, ENUM):
                params[f.name] = ""
            continue
        if f.kind in (TEXT, ENUM):
            params[f.name] = raw
        elif f.kind in (NUMBER, INTEGER):
            try:
                params[f.name] = int(text) if f.kind == INTEGER else float(text)
            except ValueError:
                raise FormError("bad_number", f.name)
        else:
            try:
                params[f.name] = json.loads(text)
            except ValueError:
                raise FormError("bad_json", f.name)
    return params
