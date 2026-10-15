"""Contrôle du harnais sans modèle (code jetable) : rejeu de sorties enregistrées.

    python check_harness.py     sort 0 si le harnais compte et rejette comme prévu
"""

import copy
import json
import sys
from pathlib import Path

HERE = Path(__file__).parent
sys.path.insert(0, str(HERE))  # le mode isolé (-I) n'ajoute pas le dossier du script

import harness  # noqa: E402
from plan_validator import check_plan  # noqa: E402

MIN_RECALL = 19  # plancher mesuré le 2026-10-15 ; un filtre qui régresse fait échouer le contrôle
CATALOG, INVENTORY = harness.load_json("catalog.json"), harness.load_json("inventory.json")
SENTENCES = harness.load_sentences("fr", [])


class Args:
    """Les paramètres du harnais pour un rejeu."""
    catalog, filter_size, examples, output = "full", 8, False, "free"
    template, no_think = "chatml", True


def run(canned, ids=None):
    sentences = [s for s in SENTENCES if ids is None or s["id"] in ids]
    replay = lambda prompt, schema, attempt, sentence: (  # noqa: E731
        (lambda o: o if isinstance(o, str) else json.dumps(o, ensure_ascii=False))(canned[sentence["id"]][attempt - 1]), {})
    results = harness.measure(sentences, CATALOG, INVENTORY, Args, replay)
    return results, harness.summarize(results)


def expected(s):
    return {"refusal": "coordonnées hors du système du modèle"} if s["expect_refusal"] else s["expected"]


def main():
    sys.stdout.reconfigure(encoding="utf-8")  # la console Windows n'est pas en UTF-8 par défaut
    errors = []
    perfect = {s["id"]: [expected(s)] for s in SENTENCES}

    # 1. Les 25 plans attendus rejoués d'un coup : deux taux de 100 %.
    _, summary = run(perfect)
    if summary["sentences"] != 25 or summary["rate_first"] != 1 or summary["rate_after_retry"] != 1:
        errors.append(f"rejeu parfait: attendu 25 phrases à 100 % deux fois, reçu {summary}")

    def one(sentence_id, outputs):
        results, _ = run({sentence_id: outputs}, {sentence_id})
        return results[0]

    s7, s8 = (next(s for s in SENTENCES if s["id"] == i)["expected"] for i in ("S07", "S08"))
    # 2. Un plan conforme mais faux est compté faux et n'est pas redemandé.
    r = one("S07", [s8])
    if r["correct_after"] or len(r["attempts"]) != 1 or r["attempts"][0]["status"] != "wrong":
        errors.append(f"plan faux: doit être compté faux sans relance, reçu {r['attempts']}")

    # 3. Une commande, une couche, un chemin, un texte absents ou invalides : non conformes, redemandés.
    absent_command = copy.deepcopy(s7); absent_command[0]["command"] = "layer.delete"
    absent_layer = copy.deepcopy(s7); absent_layer[0]["params"]["layer"] = "gares_00000000_0000_0000_0000_000000000000"
    absent_path = [{"command": "layer.add", "params": {"source": "donnees/inconnu.gpkg"}}]
    for label, bad, needle in (("commande absente", absent_command, "layer.delete"),
                               ("couche absente", absent_layer, "gares_00000000_0000_0000_0000_000000000000"),
                               ("chemin absent", absent_path, "donnees/inconnu.gpkg"),
                               ("texte libre", "Voici le plan que je propose.", "pas du JSON")):
        r = one("S07", [bad, s7])
        first = r["attempts"][0]
        if first["status"] != "nonconform" or not any(needle in e for e in first["errors"]):
            errors.append(f"{label}: doit être non conforme en nommant « {needle} », reçu {first['status']} {first['errors']}")
        if r["correct_first"] or not r["correct_after"] or len(r["attempts"]) != 2:
            errors.append(f"{label}: doit compter au taux après relance seulement")

    # 4. Une relance qui échoue encore reste fausse ; une sortie encadrée de ``` est lue.
    r = one("S07", [absent_command, absent_command])
    if r["correct_after"] or len(r["attempts"]) != 2:
        errors.append("deux sorties non conformes: doit rester faux après une seule relance")
    fenced = "```json\n" + json.dumps(s7) + "\n```"
    if not one("S07", [fenced])["correct_first"]:
        errors.append("sortie encadrée de ```: doit être lue")
    thought = "<think>\nje réfléchis [1] à {x}\n</think>\n" + json.dumps(s7)
    if not one("S07", [thought])["correct_first"]:
        errors.append("bloc de réflexion: doit être ignoré")
    for label, wrapped in (("texte avant", "Voici le plan : " + json.dumps(s7)), ("texte après", json.dumps(s7) + " Voilà.")):
        r = one("S07", [wrapped, s7])
        if r["correct_first"] or not r["correct_after"]:
            errors.append(f"{label}: la sortie doit être non conforme puis redemandée (AD-23)")
    crashed = harness.measure([SENTENCES[0]], CATALOG, INVENTORY, Args,
                              lambda prompt, schema, attempt, sentence: ("", {"exit": 1}))
    if crashed[0]["attempts"][-1]["status"] != "error" or len(crashed[0]["attempts"]) != 1 or crashed[0]["correct_after"]:
        errors.append("échec d'appel: doit être une erreur d'appel, sans relance")

    # 5. Le refus WGS84 : correct s'il refuse ; un plan en 4326 est non conforme puis redemandé.
    if not one("S25", [{"refusal": "Le point est en WGS84, le modèle en Lambert-93."}])["correct_first"]:
        errors.append("refus WGS84: doit être correct")
    wgs84_plan = [{"command": "terrain.viewshed", "params": {"dem": next(l["layer_id"] for l in INVENTORY["layers"] if l["kind"] == "raster"),
                   "observer": {"x": 2.415, "y": 48.735, "epsg": 4326}, "observer_height": {"value": 10, "unit": "m"}, "radius": {"value": 1, "unit": "km"}}}]
    r = one("S25", [wgs84_plan, {"refusal": "WGS84"}])
    if r["attempts"][0]["status"] != "nonconform" or not r["correct_after"] or r["correct_first"]:
        errors.append(f"plan WGS84: doit être non conforme puis corrigé par un refus, reçu {r['attempts'][0]['status']}")

    # 6. Invite, catalogue filtré, schéma, exemples.
    ids = harness.filter_commands("Fais une zone tampon de 300 mètres autour des gares.", CATALOG, 8)
    if len(ids) != 8 or "analysis.buffer" not in ids:
        errors.append(f"catalogue filtré: attendu 8 commandes dont analysis.buffer, reçu {ids}")
    recall = sum(1 for s in SENTENCES if not s["expect_refusal"]
                 and {x["command"] for x in s["expected"]} <= set(harness.filter_commands(s["text"], CATALOG, 8)))
    total = sum(1 for s in SENTENCES if not s["expect_refusal"])
    print(f"rappel du catalogue filtré à 8 commandes : {recall}/{total}")
    if recall < MIN_RECALL:
        errors.append(f"catalogue filtré: rappel {recall}/{total} sous le plancher {MIN_RECALL}")
    schema_text = json.dumps(harness.plan_schema(CATALOG, [c["id"] for c in CATALOG["commands"]]))
    if "(?!" in schema_text:
        errors.append("schéma: une assertion négative est restée dans les motifs")
    examples = harness.load_examples(INVENTORY)
    texts = {s["text"] for s in SENTENCES} | {e["text"] for e in harness.load_json("sentences_en.json")}
    for e in examples:
        if e["text"] in texts:
            errors.append(f"exemple « {e['text']} » est aussi une phrase mesurée")
        bad = check_plan(e["plan"], CATALOG, INVENTORY)
        if bad:
            errors.append(f"exemple « {e['text']} » non conforme: {bad}")
    prompt = harness.build_prompt("chatml", True, "SYSTEME", "Demande", "REPONSE_PRECEDENTE", ["écart nommé"])
    if "REPONSE_PRECEDENTE" not in prompt or "écart nommé" not in prompt or prompt.count("<|im_start|>assistant") != 2:
        errors.append("invite de relance: doit rappeler la réponse précédente et les écarts")

    # 7. run_forms : huit formes rejouées sans modèle, huit lignes à 100 %, la meilleure forme est contrainte.
    import tempfile
    import run_forms
    with tempfile.TemporaryDirectory() as tmp:
        replay_file = Path(tmp) / "rejeu.json"
        replay_file.write_text(json.dumps(perfect, ensure_ascii=False), encoding="utf-8")
        rows = run_forms.run(["--replay", str(replay_file)], log=lambda line: None)
        text = run_forms.markdown(rows, "?", "rejeu")
    if len(rows) != 8 or any(r["summary"]["rate_first"] != 1 for r in rows):
        errors.append("run_forms: huit formes attendues, toutes à 100 % en rejeu")
    if run_forms.best_form(rows)["output"] != "schema" or "Meilleure forme contrainte" not in text:
        errors.append("run_forms: la meilleure forme doit être cherchée parmi les formes contraintes")
    if "Aucune erreur d'appel" not in text or "n/a" not in text:
        errors.append("run_forms: en rejeu, le tableau doit dire « n/a » pour une mesure absente et « Aucune erreur d'appel »")
    # symboles de la grille, départage écrit, avertissement d'erreur d'appel
    ok = {"correct_first": True, "correct_after": True, "attempts": [{"status": "correct"}]}
    retried = {"correct_first": False, "correct_after": True, "attempts": [{"status": "nonconform"}, {"status": "correct"}]}
    wrong = {"correct_first": False, "correct_after": False, "attempts": [{"status": "wrong"}]}
    nonconform = {"correct_first": False, "correct_after": False, "attempts": [{"status": "nonconform"}]}
    broken = {"correct_first": False, "correct_after": False, "attempts": [{"status": "error"}]}
    if [run_forms.outcome(r) for r in (ok, retried, wrong, nonconform, broken)] != ["✓", "↻", "✗", "!", "E"]:
        errors.append("run_forms: les symboles de la grille ne correspondent pas aux statuts")
    tied = copy.deepcopy(rows)
    for r in tied:
        r["summary"]["call_errors"] = 0
    schema_rows = [r for r in tied if r["output"] == "schema"]
    schema_rows[0]["summary"]["rate_after_retry"], schema_rows[1]["summary"]["rate_after_retry"] = 1, 0.5
    schema_rows[0]["total_ms"], schema_rows[1]["total_ms"] = 1000, 2000
    tied_text = run_forms.markdown(tied, "?", "rejeu")
    if "Départage, à taux d'emblée égal" not in tied_text or "taux après relance" not in tied_text:
        errors.append("run_forms: le départage doit être écrit, avec le critère qui a joué")
    tied[0]["summary"]["call_errors"] = 2
    if "ATTENTION" not in run_forms.markdown(tied, "?", "rejeu"):
        errors.append("run_forms: une erreur d'appel doit être signalée dans le tableau")
    try:
        run_forms.run(["--replay", "x.json", "--examples"], log=lambda line: None)
        errors.append("run_forms: --examples passé en ligne de commande doit être refusé")
    except SystemExit:
        pass

    # 8. Gabarit mistral3 (fiche officielle de Ministral 3) : pas de <s> écrit à la main (le chargeur l'ajoute), relance en [INST].
    prompt = harness.build_prompt("mistral3", True, "SYSTEME", "Demande", "REPONSE_PRECEDENTE", ["écart nommé"])
    if (not prompt.startswith("[SYSTEM_PROMPT]SYSTEME[/SYSTEM_PROMPT][INST]Demande[/INST]REPONSE_PRECEDENTE</s>[INST]")
            or "<s>" in prompt.replace("</s>", "") or not prompt.endswith("[/INST]") or "écart nommé" not in prompt):
        errors.append(f"gabarit mistral3: ne suit pas la fiche officielle, reçu {prompt!r}")

    # 9. run_models : trois combinaisons rejouées sans modèle, tableau avec licence, taux, mémoire, temps.
    import run_models
    with tempfile.TemporaryDirectory() as tmp:
        replay_file = Path(tmp) / "rejeu.json"
        replay_file.write_text(json.dumps(perfect, ensure_ascii=False), encoding="utf-8")
        model_rows = run_models.run(["--replay", str(replay_file)], tmp, log=lambda line: None)
        model_text = run_models.markdown(model_rows, "CPU de test", 16)
    if len(model_rows) != 3 or any(r["summary"]["rate_first"] != 1 for r in model_rows):
        errors.append("run_models: trois combinaisons attendues, toutes à 100 % en rejeu")
    for needle in ("Licence", "Apache-2.0", "RAM au pic", "VRAM au pic", "Chargement", "Lecture de l'invite", "Génération", "Fils", "CPU de test", "n/a"):
        if needle not in model_text:
            errors.append(f"run_models: le tableau doit contenir « {needle} »")
    try:
        run_models.run(["--replay", "x.json", "--examples"], "x", log=lambda line: None)
        errors.append("run_models: --examples passé en ligne de commande doit être refusé")
    except SystemExit:
        pass
    # mémoire et fils repris au pic sur les appels ; une mesure absente ne casse pas la synthèse
    fake = [{"status": "correct", "errors": [], "output": "", "timing": {"ram_peak_mb": 100.0, "vram_peak_mb": 5.0, "threads": 8}},
            {"status": "correct", "errors": [], "output": "", "timing": {"ram_peak_mb": 300.0, "vram_peak_mb": None, "threads": 8}}]
    fake_summary = harness.summarize([{"attempts": fake, "correct_first": True, "correct_after": True, "conform_first": True, "recall": True}])
    if fake_summary["peak_ram_mb"] != 300.0 or fake_summary["peak_vram_mb"] != 5.0 or fake_summary["threads"] != 8:
        errors.append(f"synthèse: pics de mémoire et fils mal relevés, reçu {fake_summary}")

    if errors:
        print("\n".join(errors))
        return 1
    print("harnais: 25 phrases rejouées à 100 %, plan faux compté faux, 4 types de sortie non conforme redemandés, "
          "refus WGS84, catalogue filtré, schéma, exemples, gabarit mistral3 et comparaison de modèles contrôlés")
    return 0


if __name__ == "__main__":
    sys.exit(main())
