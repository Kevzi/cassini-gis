"""Formulaire engendré depuis le schéma JSON des paramètres d'une commande.

Sans Qt. Le panneau ne valide rien : le noyau valide. Sous-ensemble compris :
texte, nombre, entier, booléen, énumération (éventuellement « ou null »). Toute
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

# Paramètres remplis par le noyau (AD-10 ; au rejeu seulement : `leave_out`, ticket 5.36, et
# `center`, ticket 5.38) : le formulaire ne les montre pas.
CORE_FILLED = ("out", "leave_out", "center")


def offered(catalogue):
    """Les entrées du catalogue offertes au formulaire : toutes, sauf celles que le noyau dicte
    seul (`dictated_by_core`, ticket 4.9 : `layer.open_raster`)."""
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


def is_path_field(field):
    """Un champ texte nommé `path` ou `*_path` désigne un fichier du projet."""
    return field.kind == TEXT and (field.name == "path" or field.name.endswith("_path"))


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
        prop = props[name]
        kind, choices = _kind(prop)
        description = prop.get("description", "") if isinstance(prop, dict) else ""
        fields.append(Field(name, kind, name in required, choices, str(description)))
    return fields


def build_params(fields, values):
    """Construit `params` depuis les valeurs saisies (`nom -> str ou bool`).

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
