"""Compare des modèles et des quantifications sur la meilleure forme de 8.5 (code jetable, entrée 8.4).

    python run_models.py --llama <llama-completion.exe> --models-dir <dossier des GGUF>

Mesure chaque combinaison de COMBOS avec la forme « schéma · catalogue complet · avec exemples », mêmes
phrases, température 0, graine fixe. Les options du harnais (--lang, --ids, --ctx, --replay…) sont
transmises telles quelles ; --catalog, --output, --examples, --model, --template et --quant sont fixés ici.
Écrit `resultats-8-4.md` (généré : un nouveau lancement l'écrase ; la lecture écrite à la main vit dans
`lecture-8-4.md`) et `resultats-8-4.json`. Ne choisit aucun modèle : c'est le verdict de 8.6.
"""

import argparse
import json
import platform
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).parent
sys.path.insert(0, str(HERE))  # le mode isolé (-I) n'ajoute pas le dossier du script

import harness  # noqa: E402
from run_forms import fmt, outcome  # noqa: E402

# (étiquette, fichier, quantification, gabarit d'invite, licence lue sur la fiche du modèle)
COMBOS = [
    ("Qwen3.5-9B Q4_K_M", "Qwen3.5-9B-Q4_K_M.gguf", "Q4_K_M", "chatml", "Apache-2.0"),
    ("Ministral 3 8B Q4_K_M", "Ministral-3-8B-Instruct-2512-Q4_K_M.gguf", "Q4_K_M", "mistral3", "Apache-2.0"),
    ("Ministral 3 8B Q6_K", "Ministral-3-8B-Instruct-2512-Q6_K.gguf", "Q6_K", "mistral3", "Apache-2.0"),
]
FORM = ["--catalog", "full", "--output", "schema", "--examples"]
RESERVED = ("--catalog", "--output", "--examples", "--model", "--template", "--quant")


def cpu_name():
    try:
        done = subprocess.run(["powershell", "-NoProfile", "-Command", "(Get-CimInstance Win32_Processor).Name"],
                              capture_output=True, timeout=30)
        name = done.stdout.decode("utf-8", errors="replace").strip()
    except (OSError, subprocess.SubprocessError):
        name = ""
    return name or platform.processor() or "inconnu"


def run(common_argv, models_dir, combos=COMBOS, log=print):
    clash = [a for a in common_argv if a.split("=")[0] in RESERVED]
    if clash:
        raise SystemExit(f"run_models.py fixe {', '.join(RESERVED)} lui-même : retire {', '.join(clash)} de la ligne de commande")
    catalog, inventory = harness.load_json("catalog.json"), harness.load_json("inventory.json")
    rows = []
    for label, filename, quant, template, licence in combos:
        argv = common_argv + FORM + ["--model", str(Path(models_dir) / filename), "--quant", quant, "--template", template]
        args = harness.parse_args(argv)
        sentences = harness.load_sentences(args.lang, [i for i in args.ids.split(",") if i])
        results = harness.measure(sentences, catalog, inventory, args, harness.make_responder(args))
        summary = harness.summarize(results)
        rows.append({"label": label, "file": filename, "quant": quant, "template": template, "licence": licence,
                     "ctx": args.ctx, "ngl": args.ngl, "summary": summary,
                     "total_ms": sum(a["timing"].get("wall_ms", 0) for r in results for a in r["attempts"]),
                     "outcomes": {r["id"]: outcome(r) for r in results},
                     "failures": {r["id"]: {"status": r["attempts"][-1]["status"], "errors": r["attempts"][-1]["errors"][:2],
                                            "output": r["attempts"][-1]["output"][:400]}
                                  for r in results if not r["correct_after"]}})
        log(f"{label} : {summary['correct_first']}/{summary['sentences']} d'emblée, "
            f"{summary['correct_after_retry']}/{summary['sentences']} après relance, {round(rows[-1]['total_ms'] / 1000)} s")
    return rows


def mb_to_go(value):
    return "n/a" if value is None else f"{value / 1024:.1f} Go"


def markdown(rows, cpu, logical):
    n = rows[0]["summary"]["sentences"]
    out = ["# Comparaison de modèles et de quantifications — entrée 8.4", "",
           f"Forme : schéma · catalogue complet · avec exemples (la meilleure de 8.5). {n} phrases françaises, température 0, "
           f"graine fixe, mêmes phrases pour toutes les combinaisons. Aucun accès réseau pendant la mesure. "
           f"Processeur : {cpu} ({logical} fils logiques).", "",
           "| Combinaison | Licence | Juste d'emblée | Juste après relance | RAM au pic | VRAM au pic | Chargement | Lecture de l'invite | Génération | Fils | Contexte |",
           "|---|---|---|---|---|---|---|---|---|---|---|"]
    for r in rows:
        s = r["summary"]
        prompt = f"{fmt(s['mean_prompt_ms'], 0, ' ms')} ({fmt(s['mean_prompt_tokens'], 0)} jetons)"
        generation = f"{fmt(s['mean_generation_ms'], 0, ' ms')} ({fmt(s['mean_generated_tokens'], 0)} jetons)"
        out.append(f"| {r['label']} | {r['licence']} | {s['correct_first']}/{n} ({s['rate_first']:.0%}) | "
                   f"{s['correct_after_retry']}/{n} ({s['rate_after_retry']:.0%}) | {mb_to_go(s['peak_ram_mb'])} | "
                   f"{mb_to_go(s['peak_vram_mb'])} | {fmt(s['mean_load_ms'], 0, ' ms')} | {prompt} | {generation} | "
                   f"{fmt(s['threads'], 0)} | {r['ctx']} |")
    fastest = min(r["summary"]["mean_prompt_ms"] for r in rows if r["summary"]["mean_prompt_ms"] is not None) if any(
        r["summary"]["mean_prompt_ms"] is not None for r in rows) else None
    slow = [r["label"] for r in rows if fastest and r["summary"]["mean_prompt_ms"] > 3 * fastest]
    base = rows[0]["summary"]["correct_first"]
    near = [r["label"] for r in rows[1:] if abs(r["summary"]["correct_first"] - base) < 2]
    out += ["", "Les temps sont des moyennes par appel ; chargement, lecture de l'invite et génération sont séparés. "
            "La RAM est le pic du jeu de travail du processus, la VRAM l'excédent d'occupation de la carte pendant l'appel.",
            ("Écart de moins de deux phrases d'emblée avec " + rows[0]["label"] + " (donc pas un écart) : " + ", ".join(near) + ".")
            if near else "Aucune combinaison à moins de deux phrases de " + rows[0]["label"] + " d'emblée.",
            ("**MISE EN GARDE : " + ", ".join(slow) + " lit l'invite plus de trois fois plus lentement que la plus rapide. "
             "Cela signale en général une mémoire vidéo saturée (voir « VRAM au pic ») : les temps de cette ligne ne sont pas "
             "comparables à ceux des autres, et le taux, lui, reste valable.**") if slow else "Aucun temps de lecture anormalement long.",
            ("**ATTENTION : " + ", ".join(f"{r['label']} ({r['summary']['call_errors']})" for r in rows if r["summary"]["call_errors"])
             + " erreur(s) d'appel : le modèle n'est pas en cause pour ces phrases, leur taux est sous-estimé.**")
            if any(r["summary"]["call_errors"] for r in rows) else "Aucune erreur d'appel.", "",
            "## État de chaque phrase", "",
            "✓ juste d'emblée · ↻ juste après relance · ✗ conforme mais différent de l'attendu · ! non conforme · E erreur d'appel", "",
            "| Phrase | " + " | ".join(r["label"] for r in rows) + " |", "|---|" + "---|" * len(rows)]
    for sid in rows[0]["outcomes"]:
        out.append(f"| {sid} | " + " | ".join(r["outcomes"].get(sid, "?") for r in rows) + " |")
    out += ["", "## Limites", "",
            "Trois combinaisons, 25 phrases, une mesure par combinaison (déterministe à température 0) : un écart de moins "
            "de deux phrases (8 points) n'est pas un écart. Une seule machine : les temps et les mémoires ne valent que pour elle. "
            "La VRAM est un maximum échantillonné (`nvidia-smi` toutes les 0,2 s environ), au-dessus de l'état de la carte avant l'appel : "
            "elle peut sous-estimer un pic très bref. Le nombre de fils est le choix par défaut de llama.cpp, pas celui de la machine."]
    return "\n".join(out) + "\n"


def main(argv=None):
    sys.stdout.reconfigure(encoding="utf-8")  # la console Windows n'est pas en UTF-8 par défaut
    own = argparse.ArgumentParser(add_help=False)
    own.add_argument("--models-dir", default="")
    own.add_argument("--md", default=str(HERE / "resultats-8-4.md"))
    own.add_argument("--json", default=str(HERE / "resultats-8-4.json"))
    own.add_argument("--rebuild", action="store_true", help="refait le tableau depuis le JSON, sans appeler le modèle")
    own_args, common = own.parse_known_args(argv)
    if own_args.rebuild:
        saved = json.loads(Path(own_args.json).read_text(encoding="utf-8"))
        Path(own_args.md).write_text(markdown(saved["rows"], saved["cpu"], saved["logical_processors"]), encoding="utf-8", newline="\n")
        print(f"tableau reconstruit : {own_args.md}")
        return 0
    if not own_args.models_dir and "--replay" not in common:
        raise SystemExit("--models-dir est requis, sauf avec --replay")
    rows = run(common, own_args.models_dir, log=lambda line: print(line, flush=True))
    import os
    cpu, logical = cpu_name(), os.cpu_count()
    Path(own_args.md).write_text(markdown(rows, cpu, logical), encoding="utf-8", newline="\n")
    Path(own_args.json).write_text(json.dumps({"cpu": cpu, "logical_processors": logical, "rows": rows}, ensure_ascii=False, indent=1),
                                   encoding="utf-8", newline="\n")
    print(own_args.md)
    return 0


if __name__ == "__main__":
    sys.exit(main())
