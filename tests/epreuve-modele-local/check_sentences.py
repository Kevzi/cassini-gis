"""Contrôle des phrases de référence et du validateur de plans (code jetable).

    python check_sentences.py                    contrôle complet, sort 0 si tout tient
    python check_sentences.py lecture.json       compare en plus une lecture indépendante
                                                 {"S01": plan, ...} aux plans attendus

Bibliothèque standard seule.
"""

import copy
import json
import sys
from pathlib import Path

HERE = Path(__file__).parent
sys.path.insert(0, str(HERE))  # le mode isolé (-I) n'ajoute pas le dossier du script

from plan_validator import check_output, check_plan, plans_equal  # noqa: E402


def load(name):
    return json.loads((HERE / name).read_text(encoding="utf-8"))


def positive_checks(sentences, english, catalog, inventory):
    errors, terrain = [], 0
    ids = [s["id"] for s in sentences]
    if len(sentences) != 25 or len(set(ids)) != 25:
        errors.append(f"phrases: attendu 25 phrases à identifiants uniques, trouvé {len(sentences)}")
    for s in sentences:
        if not s["text"].strip():
            errors.append(f"{s['id']}: texte vide")
        if s["expect_refusal"]:
            if s["expected"] is not None:
                errors.append(f"{s['id']}: un refus attendu n'a pas de plan")
            continue
        errors += [f"{s['id']}: {e}" for e in check_plan(s["expected"], catalog, inventory)]
        if any(step["command"].startswith("terrain.") for step in s["expected"]):
            terrain += 1
    if terrain < 5:
        errors.append(f"terrain: au moins 5 cas avec un plan attendu, trouvé {terrain}")
    if not any(s["expect_refusal"] for s in sentences):
        errors.append("terrain: aucun cas de refus")
    if not 8 <= len(english) <= 12:
        errors.append(f"anglais: une dizaine de phrases attendue, trouvé {len(english)}")
    known = set(ids)
    for e in english:
        if e["same_plan_as"] not in known:
            errors.append(f"{e['id']}: renvoie à une phrase inconnue « {e['same_plan_as']} »")
        if not e["text"].strip():
            errors.append(f"{e['id']}: texte vide")
    return errors, terrain


def negative_checks(by_id, catalog, inventory):
    """Chaque plan faux doit être refusé, avec un message qui contient `expect`."""
    cases = []

    def case(label, plan, expect):
        cases.append((label, expect, check_plan(plan, catalog, inventory)))

    def edit(sentence, fn):
        plan = copy.deepcopy(by_id[sentence]["expected"])
        fn(plan)
        return plan

    case("paramètre mal typé", edit("S07", lambda p: p[0]["params"]["distance"].__setitem__("value", "300")), "attendu number")
    case("unité inconnue", edit("S07", lambda p: p[0]["params"]["distance"].__setitem__("unit", "miles")), "hors de")
    case("commande absente", edit("S02", lambda p: p[0].__setitem__("command", "layer.delete")), "absente du catalogue")
    case("couche absente", edit("S02", lambda p: p[0]["params"].__setitem__("layer", "gares_00000000_0000_0000_0000_000000000000")), "absente de l'inventaire")
    case("fichier absent", edit("S01", lambda p: p[0]["params"].__setitem__("source", "donnees/inconnu.gpkg")), "fichier absent")
    case("référence vers l'avenir", [by_id["S14"]["expected"][1], by_id["S14"]["expected"][0]], "antérieure")
    case("référence vers une étape sans couche",
         [{"command": "project.inventory", "params": {}},
          {"command": "analysis.buffer", "params": {"layer": "step:1", "distance": {"value": 1, "unit": "m"}}}],
         "ne produit pas de couche")
    case("clé en trop dans une étape", edit("S02", lambda p: p[0].__setitem__("step", 1)), "clés command et params")
    case("EPSG du point différent du MNT", edit("S20", lambda p: p[0]["params"]["observer"].__setitem__("epsg", 4326)), "AD-19")
    case("MNT vecteur", edit("S20", lambda p: p[0]["params"].__setitem__("dem", by_id["S06"]["expected"][0]["params"]["layer"])), "raster")
    case("champ inconnu", edit("S06", lambda p: p[0]["params"]["filter"].__setitem__("field", "zone_code")), "champ inconnu")
    case("paramètre en trop", edit("S02", lambda p: p[0]["params"].__setitem__("output", "x.gpkg")), "propriété inconnue")
    case("plan vide", [], "liste non vide")
    case("commande qui n'est pas un texte", edit("S02", lambda p: p[0].__setitem__("command", ["layer.describe"])), "command doit être un texte")
    case("référence avec saut de ligne final", edit("S14", lambda p: p[1]["params"].__setitem__("layer", "step:1\n")), "motif")
    case("EPSG du point différent du MNT suivi à travers une étape",
         [by_id["S22"]["expected"][0],
          {"command": "terrain.viewshed", "params": {"dem": "step:1", "observer": {"x": 2.4, "y": 48.7, "epsg": 4326},
                                                     "observer_height": {"value": 2, "unit": "m"}, "radius": {"value": 1, "unit": "km"}}}],
         "AD-19")
    case("valeur texte sur un champ numérique", edit("S11", lambda p: p[0]["params"]["filter"].__setitem__("value", "grand")), "numérique")
    case("valeur numérique sur un champ texte", edit("S12", lambda p: p[0]["params"]["filter"].__setitem__("value", 3)), "texte")

    errors = []
    for label, expect, found in cases:
        if not any(expect in e for e in found):
            errors.append(f"négatif: « {label} » n'a pas été refusé comme attendu ({expect}) ; reçu {found}")
    kind, found = check_output({"refusal": ""}, {"commands": []}, {})
    if not found:
        errors.append("négatif: un refus sans raison a été accepté")
    return errors, len(cases) + 1


def equality_checks(by_id, catalog, inventory):
    errors = []
    base = by_id["S17"]["expected"]
    ids = {l["name"]: l["layer_id"] for l in inventory["layers"]}
    reorder = [base[1], base[0],
               {"command": "analysis.intersect", "params": {"layer": ids["Parcelles cadastrales"], "overlay": "step:2"}},
               {"command": "analysis.intersect", "params": {"layer": "step:3", "overlay": "step:1"}},
               copy.deepcopy(base[4])]
    reorder[4]["params"]["layers"] = ["step:4"]
    reorder[4]["params"]["title"] = "Autre titre"
    if check_plan(reorder, catalog, inventory):
        errors.append(f"égalité: le plan permuté est lui-même non conforme: {check_plan(reorder, catalog, inventory)}")
    if not plans_equal(base, reorder, catalog):
        errors.append("égalité: un plan permuté, autre titre, doit rester égal")
    with_extra = base + [{"command": "layer.add", "params": {"source": "donnees/ecoles.geojson"}}]
    if plans_equal(base, with_extra, catalog):
        errors.append("égalité: une étape en trop doit rendre les plans différents")
    s7, s8 = by_id["S07"]["expected"], by_id["S08"]["expected"]
    if plans_equal(s7, s8, catalog):
        errors.append("égalité: 300 m sans fusion et 1 km fusionné doivent différer")
    explicit_default = copy.deepcopy(s7)
    explicit_default[0]["params"]["dissolve"] = False
    if not plans_equal(s7, explicit_default, catalog):
        errors.append("égalité: une valeur égale à son défaut doit équivaloir à l'absence")
    if not plans_equal({"refusal": "a"}, {"refusal": "b"}, catalog) or plans_equal({"refusal": "a"}, s7, catalog):
        errors.append("égalité: deux refus sont égaux, un refus et un plan ne le sont pas")
    return errors


def compare_reading(path, sentences, catalog, inventory):
    reading = json.loads(Path(path).read_text(encoding="utf-8"))
    errors, same = [], 0
    for s in sentences:
        got = reading.get(s["id"])
        want = {"refusal": "x"} if s["expect_refusal"] else s["expected"]
        kind, found = check_output(got, catalog, inventory) if got is not None else ("plan", ["absent de la lecture"])
        if found:
            errors.append(f"{s['id']}: la lecture indépendante n'est pas conforme: {found}")
        elif plans_equal(got, want, catalog):
            same += 1
        else:
            errors.append(f"{s['id']}: la lecture indépendante diffère du plan attendu")
    return errors, same


def main():
    sys.stdout.reconfigure(encoding="utf-8")  # la console Windows n'est pas en UTF-8 par défaut
    catalog, inventory = load("catalog.json"), load("inventory.json")
    sentences, english = load("sentences.json"), load("sentences_en.json")
    by_id = {s["id"]: s for s in sentences}
    errors, terrain = positive_checks(sentences, english, catalog, inventory)
    neg_errors, n_neg = negative_checks(by_id, catalog, inventory)
    errors += neg_errors + equality_checks(by_id, catalog, inventory)
    if errors:
        print("\n".join(errors))
        return 1
    print(f"{len(sentences)} phrases françaises dont {terrain} cas de terrain avec plan et un refus, "
          f"{len(english)} anglaises, {n_neg} cas négatifs refusés, égalité de plans vérifiée")
    if len(sys.argv) > 1:
        reading_errors, same = compare_reading(sys.argv[1], sentences, catalog, inventory)
        print(f"lecture indépendante : {same}/{len(sentences)} plans identiques")
        if reading_errors:
            print("\n".join(reading_errors))
            return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
