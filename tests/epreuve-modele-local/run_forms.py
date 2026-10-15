"""Enchaîne les huit formes de sollicitation du harnais et compare leurs taux (code jetable, entrée 8.5).

    python run_forms.py --llama <llama-completion.exe> --model <fichier.gguf> --quant Q4_K_M

Les options du harnais (--lang, --ids, --template, --ctx, --replay…) sont transmises telles quelles ;
seules --catalog, --examples et --output varient d'une forme à l'autre. Écrit `resultats-8-5.md`
(tableau, généré : un nouveau lancement l'écrase ; la lecture écrite à la main vit dans `lecture-8-5.md`) et `resultats-8-5.json` (une ligne de synthèse par forme et l'état de chaque phrase).
La sortie libre n'est qu'un témoin : la meilleure forme se cherche parmi les formes contraintes (AD-23).
"""

import argparse
import json
import sys
from pathlib import Path

HERE = Path(__file__).parent
sys.path.insert(0, str(HERE))  # le mode isolé (-I) n'ajoute pas le dossier du script

import harness  # noqa: E402

FORMS = [(catalog, examples, output) for output in ("free", "schema")
         for catalog in ("full", "filtered") for examples in (False, True)]
SYMBOL = {"first": "✓", "retry": "↻", "wrong": "✗", "nonconform": "!", "error": "E"}


def form_name(catalog, examples, output):
    return f"{'schéma' if output == 'schema' else 'libre'} · catalogue {'filtré' if catalog == 'filtered' else 'complet'} · {'avec' if examples else 'sans'} exemples"


def outcome(record):
    """Un symbole par phrase : ✓ juste d'emblée, ↻ juste après relance, ✗ conforme mais faux, ! non conforme, E erreur d'appel."""
    if record["correct_first"]:
        return SYMBOL["first"]
    if record["correct_after"]:
        return SYMBOL["retry"]
    last = record["attempts"][-1]["status"]
    return SYMBOL.get(last, "?")


def total_ms(results):
    return sum(a["timing"].get("wall_ms", 0) for r in results for a in r["attempts"])


def fmt(value, digits, unit=""):
    """Un nombre arrondi avec son unité, ou « n/a » quand la mesure manque (rejeu sans modèle)."""
    return "n/a" if value is None else f"{round(value, digits):.{digits}f}{unit}"


def best_form(rows):
    """Meilleure forme contrainte : taux au premier essai, puis après relance, puis temps le plus court."""
    constrained = [r for r in rows if r["output"] == "schema"]
    return min(constrained, key=lambda r: (-r["summary"]["rate_first"], -r["summary"]["rate_after_retry"], r["total_ms"]))


RESERVED = ("--catalog", "--output", "--examples")


def run(common_argv, log=print):
    clash = [a for a in common_argv if a.split("=")[0] in RESERVED]
    if clash:
        raise SystemExit(f"run_forms.py fait varier {', '.join(RESERVED)} lui-même : retire {', '.join(clash)} de la ligne de commande")
    catalog, inventory = harness.load_json("catalog.json"), harness.load_json("inventory.json")
    rows = []
    for catalog_mode, examples, output in FORMS:
        argv = common_argv + ["--catalog", catalog_mode, "--output", output] + (["--examples"] if examples else [])
        args = harness.parse_args(argv)
        sentences = harness.load_sentences(args.lang, [i for i in args.ids.split(",") if i])
        results = harness.measure(sentences, catalog, inventory, args, harness.make_responder(args))
        summary = harness.summarize(results)
        row = {"name": form_name(catalog_mode, examples, output), "catalog": catalog_mode, "examples": examples,
               "output": output, "summary": summary, "total_ms": total_ms(results),
               "outcomes": {r["id"]: outcome(r) for r in results},
               "failures": {r["id"]: {"status": r["attempts"][-1]["status"], "errors": r["attempts"][-1]["errors"][:2],
                                      "output": r["attempts"][-1]["output"][:400]}
                            for r in results if not r["correct_after"]}}
        rows.append(row)
        log(f"{row['name']} : {summary['correct_first']}/{summary['sentences']} d'emblée, "
            f"{summary['correct_after_retry']}/{summary['sentences']} après relance, {round(row['total_ms'] / 1000)} s")
    return rows


def markdown(rows, quant, model_label):
    best = best_form(rows)
    n = rows[0]["summary"]["sentences"]
    out = [f"# Formes de sollicitation — entrée 8.5", "",
           f"Modèle : {model_label}, quantification {quant}. {n} phrases françaises, température 0, graine fixe : "
           "les huit mesures ne diffèrent que par la forme. Aucun accès réseau.", "",
           "| Forme | Juste d'emblée | Juste après relance | Conformes d'emblée | Relancées | Erreurs d'appel | Rappel du filtre | Jetons d'invite | Génération moyenne | Temps total |",
           "|---|---|---|---|---|---|---|---|---|---|"]
    for r in rows:
        s = r["summary"]
        recall = f"{s['catalog_recall']}/{s['sentences']}" if r["catalog"] == "filtered" else "—"
        out.append(f"| {r['name']}{' **(meilleure)**' if r is best else ''} | {s['correct_first']}/{n} ({s['rate_first']:.0%}) | "
                   f"{s['correct_after_retry']}/{n} ({s['rate_after_retry']:.0%}) | {s['conform_first']}/{n} | {s['retried']} | "
                   f"{s['call_errors']} | {recall} | {fmt(s['mean_prompt_tokens'], 0)} | {fmt(s['mean_generation_ms'], 0, ' ms')} | {round(r['total_ms'] / 1000)} s |")
    ties = [r for r in rows if r["output"] == "schema" and r["summary"]["rate_first"] == best["summary"]["rate_first"] and r is not best]
    near = [r for r in rows if r is not best and best["summary"]["correct_first"] - r["summary"]["correct_first"] < 2]

    def decided_by(rival):
        if rival["summary"]["rate_after_retry"] != best["summary"]["rate_after_retry"]:
            return f"taux après relance ({best['summary']['correct_after_retry']} contre {rival['summary']['correct_after_retry']})"
        return f"temps total ({round(best['total_ms'] / 1000)} s contre {round(rival['total_ms'] / 1000)} s)"
    out += ["", f"**Meilleure forme contrainte : {best['name']}** ({best['summary']['correct_first']}/{n} d'emblée, "
            f"{best['summary']['correct_after_retry']}/{n} après relance).",
            ("Départage, à taux d'emblée égal : " + " ; ".join(f"contre {t['name']}, {decided_by(t)}" for t in ties) + ".")
            if ties else "Aucune égalité à départager.",
            ("Formes à moins de deux phrases de la meilleure, donc statistiquement indiscernables d'elle : "
             + " ; ".join(r["name"] for r in near) + ".") if near else "Aucune autre forme à moins de deux phrases de la meilleure.",
            ("**ATTENTION : " + ", ".join(f"{r['name']} ({r['summary']['call_errors']})" for r in rows if r["summary"]["call_errors"])
             + " erreur(s) d'appel : le modèle n'est pas en cause pour ces phrases, leur taux est sous-estimé.**")
            if any(r["summary"]["call_errors"] for r in rows) else "Aucune erreur d'appel.",
            "La sortie libre n'est qu'un témoin : le produit contraindra la sortie (AD-23).", "",
            "## État de chaque phrase", "",
            "✓ juste d'emblée · ↻ juste après relance · ✗ conforme mais différent de l'attendu · ! non conforme · E erreur d'appel", "",
            "| Phrase | " + " | ".join(f"F{i + 1}" for i in range(len(rows))) + " |",
            "|---|" + "---|" * len(rows)]
    for sid in rows[0]["outcomes"]:
        out.append(f"| {sid} | " + " | ".join(r["outcomes"].get(sid, "?") for r in rows) + " |")
    out += ["", "Légende des colonnes : " + " ; ".join(f"F{i + 1} = {r['name']}" for i, r in enumerate(rows)) + ".", "",
            "## Limites", "",
            "Un modèle, une quantification, 25 phrases, une mesure par forme (déterministe à température 0) : "
            "un écart de moins de deux phrases (8 points) n'est pas un écart. Le filtre lexical à 8 commandes ne retient "
            "les bonnes commandes que pour une partie des phrases (colonne « Rappel du filtre »), ce qui plafonne les formes filtrées."]
    return "\n".join(out) + "\n"


def main(argv=None):
    sys.stdout.reconfigure(encoding="utf-8")  # la console Windows n'est pas en UTF-8 par défaut
    own = argparse.ArgumentParser(add_help=False)
    own.add_argument("--md", default=str(HERE / "resultats-8-5.md"))
    own.add_argument("--json", default=str(HERE / "resultats-8-5.json"))
    own.add_argument("--rebuild", action="store_true", help="refait le tableau depuis le JSON, sans appeler le modèle")
    own.add_argument("--quant", default="?")
    own.add_argument("--label", default="rejeu")
    own_args, common = own.parse_known_args(argv)
    if own_args.rebuild:
        rows = json.loads(Path(own_args.json).read_text(encoding="utf-8"))
        Path(own_args.md).write_text(markdown(rows, own_args.quant, own_args.label), encoding="utf-8", newline="\n")
        print(f"tableau reconstruit : {own_args.md}")
        return 0
    probe = harness.parse_args(common)
    rows = run(common, log=lambda line: print(line, flush=True))
    label = Path(probe.model).stem if probe.model else "rejeu"
    Path(own_args.md).write_text(markdown(rows, probe.quant, label), encoding="utf-8", newline="\n")
    Path(own_args.json).write_text(json.dumps(rows, ensure_ascii=False, indent=1), encoding="utf-8", newline="\n")
    best = best_form(rows)
    print(f"meilleure forme contrainte : {best['name']}\n{own_args.md}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
