"""Validateur minimal d'un sous-ensemble de JSON Schema draft 2020-12 (code jetable).

Bibliothèque standard seule. Il ne connaît que les mots-clés listés au README :
$ref (local, `#/$defs/nom`), type, enum, properties, required,
additionalProperties (false), items, minItems, maxItems, minimum,
exclusiveMinimum, minLength, maxLength, pattern (ancré, apparié en entier : un saut
de ligne final n'est pas toléré). Les autres sont ignorés ici ;
c'est `check_catalog.py` qui refuse un schéma qui en emploierait.
"""

import re


def resolve(schema, defs):
    """Suit un $ref local et rend le schéma désigné."""
    ref = schema.get("$ref")
    if ref is None:
        return schema
    name = ref.removeprefix("#/$defs/")
    return resolve(defs[name], defs)


def _is_type(value, name):
    if name == "null":
        return value is None
    if name == "boolean":
        return isinstance(value, bool)
    if name == "integer":
        return isinstance(value, int) and not isinstance(value, bool)
    if name == "number":
        return isinstance(value, (int, float)) and not isinstance(value, bool)
    if name == "string":
        return isinstance(value, str)
    if name == "array":
        return isinstance(value, list)
    if name == "object":
        return isinstance(value, dict)
    return False


def validate(value, schema, defs, path="$"):
    """Rend la liste des écarts entre `value` et `schema`, vide si conforme."""
    schema = resolve(schema, defs)
    errors = []
    types = schema.get("type")
    if types is not None:
        names = types if isinstance(types, list) else [types]
        if not any(_is_type(value, n) for n in names):
            return [f"{path}: attendu {'|'.join(names)}, reçu {type(value).__name__}"]
    if "enum" in schema and value not in schema["enum"]:
        errors.append(f"{path}: « {value} » hors de {schema['enum']}")
    if isinstance(value, (int, float)) and not isinstance(value, bool):
        if "minimum" in schema and value < schema["minimum"]:
            errors.append(f"{path}: {value} < minimum {schema['minimum']}")
        if "exclusiveMinimum" in schema and value <= schema["exclusiveMinimum"]:
            errors.append(f"{path}: {value} doit dépasser {schema['exclusiveMinimum']}")
    if isinstance(value, str):
        if "minLength" in schema and len(value) < schema["minLength"]:
            errors.append(f"{path}: texte trop court")
        if "maxLength" in schema and len(value) > schema["maxLength"]:
            errors.append(f"{path}: texte trop long ({len(value)} > {schema['maxLength']})")
        if "pattern" in schema and not re.fullmatch(schema["pattern"], value):
            errors.append(f"{path}: « {value} » ne respecte pas le motif")
    if isinstance(value, dict):
        props = schema.get("properties", {})
        for name in schema.get("required", []):
            if name not in value:
                errors.append(f"{path}: « {name} » manquant")
        for name, sub in value.items():
            if name in props:
                errors += validate(sub, props[name], defs, f"{path}.{name}")
            elif schema.get("additionalProperties") is False:
                errors.append(f"{path}: propriété inconnue « {name} »")
    if isinstance(value, list):
        if "minItems" in schema and len(value) < schema["minItems"]:
            errors.append(f"{path}: pas assez d'éléments")
        if "maxItems" in schema and len(value) > schema["maxItems"]:
            errors.append(f"{path}: trop d'éléments ({len(value)} > {schema['maxItems']})")
        if "items" in schema:
            for i, item in enumerate(value):
                errors += validate(item, schema["items"], defs, f"{path}[{i}]")
    return errors
