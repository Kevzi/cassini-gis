"""Conformité et égalité des plans (code jetable, réutilisé tel quel par le harnais de 8.3).

Un plan est une liste d'étapes {"command", "params"}, ou un refus
{"refusal": "<raison>"}. Bibliothèque standard seule.
"""

import json

from schema_validator import resolve, validate

LAYER_REF = "#/$defs/layer_ref"
POINT_REF = "#/$defs/point"
RASTER_PARAMS = {"dem", "raster"}
RASTER_PRODUCERS = {"terrain.slope", "terrain.viewshed"}
IGNORED_PARAMS = {"title", "sources_note"}
MAX_STEPS = 12


def _catalog_index(catalog):
    return {c["id"]: c for c in catalog["commands"]}


def _find(params, schema, ref, defs, path=""):
    """Rend (chemin, valeur) pour chaque valeur dont le schéma est exactement `ref`."""
    found = []
    if schema.get("$ref") == ref:
        return [(path, params)]
    schema = resolve(schema, defs)
    if isinstance(params, dict):
        for name, sub in schema.get("properties", {}).items():
            if name in params:
                found += _find(params[name], sub, ref, defs, f"{path}.{name}" if path else name)
    elif isinstance(params, list) and "items" in schema:
        for i, item in enumerate(params):
            found += _find(item, schema["items"], ref, defs, f"{path}[{i}]")
    return found


def _produces_layer(command, defs):
    result = resolve(command["result"], defs)
    return "layer_id" in result.get("properties", {})


def _epsg_of(ref, plan, layers):
    """EPSG d'une couche citée, suivie à travers les étapes ; None si le plan ne permet pas de le dire."""
    if ref in layers:
        return layers[ref]["epsg"]
    if isinstance(ref, str) and ref.startswith("step:"):
        before = plan[int(ref[5:]) - 1]
        if before["command"].startswith("terrain."):
            return _epsg_of(before["params"]["dem"], plan, layers)
        if before["command"] == "layer.reproject":
            return before["params"]["target_epsg"]
    return None


def check_plan(plan, catalog, inventory):
    """Rend la liste des écarts d'un plan au catalogue et à l'inventaire (AD-23)."""
    errors = []
    if not isinstance(plan, list) or not plan:
        return ["plan: une liste non vide d'étapes est attendue"]
    if len(plan) > MAX_STEPS:
        errors.append(f"plan: {len(plan)} étapes, plafond {MAX_STEPS}")
    commands = _catalog_index(catalog)
    defs = catalog["$defs"]
    layers = {l["layer_id"]: l for l in inventory["layers"]}
    files = {f["path"] for f in inventory["unloaded_files"]}

    for rank, step in enumerate(plan, 1):
        where = f"étape {rank}"
        if not isinstance(step, dict) or set(step) != {"command", "params"}:
            errors.append(f"{where}: attendu exactement les clés command et params")
            continue
        if not isinstance(step["command"], str):
            errors.append(f"{where}: command doit être un texte")
            continue
        command = commands.get(step["command"])
        if command is None:
            errors.append(f"{where}: commande absente du catalogue « {step['command']} »")
            continue
        params = step["params"]
        schema_errors = validate(params, command["params"], defs, f"{where}.params")
        errors += schema_errors
        if schema_errors:
            continue
        for path, ref in _find(params, command["params"], LAYER_REF, defs):
            name = path.split(".")[0].split("[")[0]
            if ref.startswith("step:"):
                k = int(ref[5:])
                if k >= rank:
                    errors.append(f"{where}: {ref} ne désigne pas une étape antérieure")
                    continue
                before = plan[k - 1]
                before_command = commands.get(before.get("command")) if isinstance(before, dict) else None
                if before_command is None or not _produces_layer(before_command, defs):
                    errors.append(f"{where}: {ref} désigne une étape qui ne produit pas de couche")
                elif name in RASTER_PARAMS and before["command"] not in RASTER_PRODUCERS:
                    errors.append(f"{where}: {name} doit être un raster, or {ref} ne l'est pas")
            elif ref not in layers:
                errors.append(f"{where}: couche absente de l'inventaire « {ref} »")
            elif name in RASTER_PARAMS and layers[ref]["kind"] != "raster":
                errors.append(f"{where}: {name} doit être un raster, or « {ref} » est un vecteur")
        if step["command"] == "layer.add" and params["source"] not in files:
            errors.append(f"{where}: fichier absent de l'inventaire « {params['source']} »")
        if step["command"] == "layer.select_by_attribute" and params["layer"] in layers:
            fields = {f["name"]: f["type"] for f in layers[params["layer"]]["fields"]}
            flt = params["filter"]
            if flt["field"] not in fields:
                errors.append(f"{where}: champ inconnu « {flt['field']} » sur cette couche")
            elif fields[flt["field"]] in ("integer", "number") and (
                    isinstance(flt["value"], (str, bool))):
                errors.append(f"{where}: le champ « {flt['field']} » est numérique, la valeur ne l'est pas")
            elif fields[flt["field"]] == "string" and not isinstance(flt["value"], str):
                errors.append(f"{where}: le champ « {flt['field']} » est un texte, la valeur n'en est pas un")
        if step["command"].startswith("terrain.") and "dem" in params:
            dem_epsg = _epsg_of(params["dem"], plan, layers)
            for path, point in _find(params, command["params"], POINT_REF, defs):
                if dem_epsg is not None and point["epsg"] != dem_epsg:
                    errors.append(f"{where}: {path} en EPSG {point['epsg']}, le modèle d'élévation est en {dem_epsg} (AD-19)")
    return errors


def check_output(output, catalog, inventory):
    """Rend (genre, écarts) : genre vaut « plan » ou « refusal »."""
    if isinstance(output, dict):
        reason = output.get("refusal")
        if set(output) == {"refusal"} and isinstance(reason, str) and reason.strip():
            return "refusal", []
        return "refusal", ["sortie: un refus s'écrit {\"refusal\": \"<raison>\"}"]
    return "plan", check_plan(output, catalog, inventory)


def _norm(value):
    if isinstance(value, float) and value.is_integer():
        return int(value)
    return value


def canonical(plan, catalog):
    """Forme canonique d'un plan : arbres des étapes terminales, sans l'ordre ni les doublons de forme."""
    commands = _catalog_index(catalog)
    nodes, used = [], set()

    def conv(value):
        if isinstance(value, str) and value.startswith("step:"):
            k = int(value[5:])
            used.add(k)
            return nodes[k - 1]
        if isinstance(value, list):
            return sorted((conv(v) for v in value), key=lambda v: json.dumps(v, sort_keys=True))
        if isinstance(value, dict):
            return {k: conv(v) for k, v in value.items()}
        return _norm(value)

    for step in plan:
        schema = commands[step["command"]]["params"]["properties"]
        params = {}
        for name, value in step["params"].items():
            if name in IGNORED_PARAMS:
                continue
            if name in schema and "default" in schema[name] and value == schema[name]["default"]:
                continue
            params[name] = conv(value)
        if step["command"] == "analysis.intersect":
            inputs = []
            for part in (params["layer"], params["overlay"]):
                if isinstance(part, dict) and part.get("command") == "analysis.intersect":
                    inputs += part["inputs"]
                else:
                    inputs.append(part)
            inputs.sort(key=lambda v: json.dumps(v, sort_keys=True))
            nodes.append({"command": "analysis.intersect", "inputs": inputs})
        else:
            nodes.append({"command": step["command"], "params": params})
    return sorted(json.dumps(n, sort_keys=True) for i, n in enumerate(nodes, 1) if i not in used)


def plans_equal(a, b, catalog):
    """Deux plans valides sont égaux s'ils ont la même forme canonique ; deux refus le sont toujours."""
    a_refusal, b_refusal = isinstance(a, dict), isinstance(b, dict)
    if a_refusal or b_refusal:
        return a_refusal and b_refusal
    return canonical(a, catalog) == canonical(b, catalog)
