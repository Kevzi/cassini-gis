"""Contrôle de cohérence du catalogue et de l'inventaire factices (code jetable).

Bibliothèque standard seule. Sort 0 si le catalogue tient ses propres règles,
non nul sinon, en nommant l'intrus. `--self-test` ne se lance pas à part : il est
toujours exécuté après le contrôle, pour prouver que le contrôle sait refuser.
Pas d'option en ligne de commande : on le lance, il dit oui ou non.
"""

import copy
import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).parent

# Le sous-ensemble de JSON Schema draft 2020-12 que le validateur de la seconde
# session devra connaître. Tout autre mot-clé est refusé ici.
ALLOWED_KEYWORDS = {
    "$ref", "description", "default", "type", "enum", "properties", "required",
    "additionalProperties", "items", "minItems", "maxItems", "minimum",
    "exclusiveMinimum", "minLength", "maxLength", "pattern",
}
ALLOWED_TYPES = {"object", "array", "string", "number", "integer", "boolean", "null"}
DIALECT = "https://json-schema.org/draft/2020-12/schema"
COMMAND_ID = re.compile(r"^[a-z]+\.[a-z_]+$")
LAYER_ID = re.compile(r"^[a-z][a-z0-9_]*_[0-9a-f]{8}_[0-9a-f]{4}_[0-9a-f]{4}_[0-9a-f]{4}_[0-9a-f]{12}$")
FORBIDDEN_PARAMS = {"output", "output_path", "output_name", "output_dir", "path",
                    "file", "filename", "dest", "destination", "folder", "directory",
                    "expression", "sql", "code", "script", "algorithm", "filter_expression"}
FIELD_TYPES = {"string", "integer", "number", "boolean", "date"}
DEMO_CHAIN = ["layer.select_by_attribute", "analysis.buffer", "analysis.intersect", "layout.export_pdf"]
TERRAIN = {"terrain.viewshed", "terrain.line_of_sight", "terrain.slope"}


def walk(schema, where, defs, errors, in_params):
    """Parcourt un schéma et note chaque écart aux règles."""
    if not isinstance(schema, dict):
        errors.append(f"{where}: un schéma doit être un objet")
        return
    for key in schema:
        if key not in ALLOWED_KEYWORDS:
            errors.append(f"{where}: mot-clé hors sous-ensemble « {key} »")
    ref = schema.get("$ref")
    if ref is not None:
        match = re.fullmatch(r"#/\$defs/([a-z_]+)", ref) if isinstance(ref, str) else None
        if not match or match.group(1) not in defs:
            errors.append(f"{where}: $ref inconnu « {ref} »")
    types = schema.get("type")
    names = types if isinstance(types, list) else [types] if types else []
    for t in names:
        if t not in ALLOWED_TYPES:
            errors.append(f"{where}: type inconnu « {t} »")
        if in_params and t in ("number", "integer"):
            errors.append(f"{where}: nombre nu dans un paramètre (une mesure porte son unité)")
    if "object" in names:
        if schema.get("additionalProperties") is not False:
            errors.append(f"{where}: un objet doit déclarer additionalProperties: false")
        props = schema.get("properties")
        if not isinstance(props, dict):
            errors.append(f"{where}: un objet doit déclarer ses propriétés")
            props = {}
        for name, sub in props.items():
            if in_params and name in FORBIDDEN_PARAMS:
                errors.append(f"{where}: paramètre interdit « {name} » (AD-10, AD-22)")
            walk(sub, f"{where}.{name}", defs, errors, in_params)
        for name in schema.get("required", []):
            if name not in props:
                errors.append(f"{where}: « {name} » requis mais non déclaré")
    if "array" in names:
        if "maxItems" not in schema:
            errors.append(f"{where}: un tableau doit être borné par maxItems (AD-8)")
        if "items" not in schema:
            errors.append(f"{where}: un tableau doit déclarer ses items")
        else:
            walk(schema["items"], f"{where}[]", defs, errors, in_params)
    if "pattern" in schema:
        try:
            re.compile(schema["pattern"])
        except re.error as exc:
            errors.append(f"{where}: motif invalide ({exc})")


def check(catalog, inventory):
    errors = []
    if catalog.get("$schema") != DIALECT:
        errors.append("catalogue: dialecte non déclaré (draft 2020-12)")
    defs = catalog.get("$defs", {})
    for name, schema in defs.items():
        walk(schema, f"$defs.{name}", defs, errors, in_params=False)

    seen = set()
    commands = catalog.get("commands", [])
    for cmd in commands:
        cid = cmd.get("id", "?")
        if not COMMAND_ID.match(cid):
            errors.append(f"{cid}: id hors forme domaine.verbe en snake_case")
        if cid in seen:
            errors.append(f"{cid}: id dupliqué")
        seen.add(cid)
        for field in ("description_fr", "description_en"):
            if not str(cmd.get(field, "")).strip():
                errors.append(f"{cid}: {field} manquante")
        if cmd.get("runs_in") not in ("core", "qgis"):
            errors.append(f"{cid}: runs_in doit valoir core ou qgis")
        for part in ("params", "result"):
            if part not in cmd:
                errors.append(f"{cid}: {part} manquant")
            else:
                walk(cmd[part], f"{cid}.{part}", defs, errors, in_params=(part == "params"))

    for missing in DEMO_CHAIN:
        if missing not in seen:
            errors.append(f"phrase de démo: commande absente « {missing} »")
    for missing in TERRAIN - seen:
        errors.append(f"terrain: commande absente « {missing} »")

    layer_ids = set()
    for layer in inventory.get("layers", []):
        lid = layer.get("layer_id", "?")
        if not LAYER_ID.match(lid):
            errors.append(f"inventaire: identifiant de couche non opaque « {lid} »")
        if lid in layer_ids:
            errors.append(f"inventaire: identifiant dupliqué « {lid} »")
        layer_ids.add(lid)
        if layer.get("epsg") not in defs.get("epsg", {}).get("enum", []):
            errors.append(f"inventaire: {lid} a un EPSG hors du catalogue")
        if layer.get("kind") not in ("vector", "raster"):
            errors.append(f"inventaire: {lid} a un type inconnu")
        for field in layer.get("fields", []):
            if field.get("type") not in FIELD_TYPES:
                errors.append(f"inventaire: {lid} champ « {field.get('name')} » de type inconnu")
            values = field.get("values", [])
            if not isinstance(values, list) or len(values) > 12 or not all(isinstance(v, str) for v in values):
                errors.append(f"inventaire: {lid} champ « {field.get('name')} » a des valeurs d'exemple invalides")
    if inventory.get("project", {}).get("epsg") not in defs.get("epsg", {}).get("enum", []):
        errors.append("inventaire: l'EPSG du projet est hors du catalogue")
    kinds = [l.get("kind") for l in inventory.get("layers", [])]
    if (kinds.count("vector"), kinds.count("raster")) != (6, 1):
        errors.append(f"inventaire: attendu 6 couches vecteur et 1 raster, trouvé {kinds.count('vector')} et {kinds.count('raster')}")
    for f in inventory.get("unloaded_files", []):
        if f.get("kind") not in ("vector", "raster"):
            errors.append(f"inventaire: fichier non ouvert de type inconnu « {f.get('path')} »")
    paths = [l.get("source", "") for l in inventory.get("layers", [])]
    paths += [f.get("path", "") for f in inventory.get("unloaded_files", [])]
    for p in paths:
        if p.startswith(("/", "\\")) or re.match(r"^[A-Za-z]:", p) or ".." in p or "\\" in p:
            errors.append(f"inventaire: chemin non relatif ou non POSIX « {p} »")
    if len(set(paths)) != len(paths):
        errors.append("inventaire: un même fichier est cité deux fois")
    return errors, len(commands), len(layer_ids)


def self_test(catalog, inventory):
    """Chaque mutation doit faire échouer le contrôle, sinon le contrôle ne garde rien."""
    cases = []

    def mutate(label, fn, expect):
        cat, inv = copy.deepcopy(catalog), copy.deepcopy(inventory)
        fn(cat, inv)
        cases.append((label, expect, check(cat, inv)[0]))

    first = lambda cat: next(c for c in cat["commands"] if c["id"] == "analysis.buffer")
    mutate("mot-clé hors liste", lambda c, i: first(c)["params"]["properties"].__setitem__(
        "dissolve", {"type": "boolean", "oneOf": []}), "oneOf")
    mutate("paramètre expression", lambda c, i: first(c)["params"]["properties"].__setitem__(
        "expression", {"type": "string"}), "expression")
    mutate("nombre nu", lambda c, i: first(c)["params"]["properties"].__setitem__(
        "meters", {"type": "number"}), "nombre nu")
    mutate("id dupliqué", lambda c, i: c["commands"].append(copy.deepcopy(c["commands"][0])), "dupliqué")
    mutate("tableau non borné", lambda c, i: next(
        x for x in c["commands"] if x["id"] == "layout.export_pdf")["params"]["properties"]["layers"].pop("maxItems"), "maxItems")
    mutate("id de couche lisible", lambda c, i: i["layers"][0].__setitem__("layer_id", "parcelles"), "opaque")
    mutate("chemin absolu", lambda c, i: i["layers"][0].__setitem__("source", "C:/data/parcelles.gpkg"), "chemin")
    mutate("objet déclaré en liste de types", lambda c, i: first(c)["params"]["properties"].__setitem__(
        "extra", {"type": ["object", "null"], "properties": {}}), "additionalProperties")
    mutate("paramètre file", lambda c, i: first(c)["params"]["properties"].__setitem__(
        "file", {"type": "string"}), "file")
    for label, expect, errs in cases:
        if not any(expect in e for e in errs):
            print(f"AUTO-TEST ÉCHEC: « {label} » n'a pas été refusé ({expect})")
            return False
    print(f"auto-test: {len(cases)} mutations toutes refusées")
    return True


def main():
    sys.stdout.reconfigure(encoding="utf-8")  # la console Windows n'est pas en UTF-8 par défaut
    catalog = json.loads((HERE / "catalog.json").read_text(encoding="utf-8"))
    inventory = json.loads((HERE / "inventory.json").read_text(encoding="utf-8"))
    errors, n_cmd, n_layers = check(catalog, inventory)
    if errors:
        print("\n".join(errors))
        return 1
    print(f"{n_cmd} commandes, {n_layers} couches : cohérent")
    return 0 if self_test(catalog, inventory) else 1


if __name__ == "__main__":
    sys.exit(main())
