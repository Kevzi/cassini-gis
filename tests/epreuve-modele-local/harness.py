"""Harnais de mesure du taux de plans corrects (code jetable, epic 8, entrée 8.3).

Pose chaque phrase à un modèle local par `llama-completion` en ligne de commande,
valide la sortie avec `plan_validator.py`, redemande UNE fois si elle n'est pas
conforme (AD-23 : rejetée et redemandée, jamais réparée), la compare au plan
attendu et imprime deux taux : au premier essai, et après relance.

    python harness.py --llama <llama-completion.exe> --model <fichier.gguf> --quant Q4_K_M
    python harness.py ... --catalog filtered --examples --output schema --ids S07,S17

Aucun réseau, aucun serveur : un processus par appel. Bibliothèque standard seule.
`--replay fichier.json` rejoue des sorties enregistrées {"S01": [essai1, essai2]}
sans modèle, ce qui sert au contrôle du harnais lui-même.
"""

import argparse
import json
import re
import subprocess
import sys
import tempfile
import time
from pathlib import Path

HERE = Path(__file__).parent
sys.path.insert(0, str(HERE))  # le mode isolé (-I) n'ajoute pas le dossier du script

from plan_validator import check_output, plans_equal  # noqa: E402

ANSI = re.compile(r"\x1b\[[0-9;]*m")
THINK = re.compile(r"<think>.*?</think>", re.S)
FENCE = re.compile(r"```[a-z]*\s*(.*?)\s*```", re.S)
LOOKAHEAD = re.compile(r"\(\?![^)]*\)")
WORD = re.compile(r"[a-zà-ÿ0-9]{3,}")

SYSTEM = (
    "Tu es l'agent d'un SIG hors ligne. Tu transformes la demande de l'utilisateur en un plan : "
    "une suite d'appels de commandes du catalogue, rien d'autre. Réponds uniquement par un tableau JSON "
    "d'objets {\"command\": id, \"params\": {...}}, dans l'ordre d'exécution, sans aucun texte autour.\n"
    "Règles : une couche existante se désigne par son identifiant exact de l'inventaire ; la couche produite "
    "par l'étape k (en comptant à partir de 1) se désigne par \"step:k\" ; un fichier à ouvrir se prend parmi "
    "les fichiers non ouverts de l'inventaire ; une option facultative n'est posée que si la demande la "
    "réclame ; une carte PDF ne montre que la couche résultat, sauf si la demande nomme d'autres couches ; "
    "n'ajoute aucune étape que la demande ne réclame pas ; n'invente ni commande, ni couche, ni chemin.\n"
    "Les coordonnées de terrain doivent être dans le système du modèle d'élévation : si elles sont dans un "
    "autre, ou si la demande ne peut pas être satisfaite avec ces commandes, réponds uniquement "
    "{\"refusal\": \"<raison courte>\"} au lieu d'un plan."
)


def load_json(name):
    return json.loads((HERE / name).read_text(encoding="utf-8"))


# -- sentences ---------------------------------------------------------------

def load_sentences(lang, ids):
    french = load_json("sentences.json")
    if lang == "fr":
        chosen = french
    else:
        by_id = {s["id"]: s for s in french}
        chosen = [{"id": e["id"], "text": e["text"], "expected": by_id[e["same_plan_as"]]["expected"],
                   "expect_refusal": by_id[e["same_plan_as"]]["expect_refusal"]} for e in load_json("sentences_en.json")]
    if ids:
        wanted = set(ids)
        chosen = [s for s in chosen if s["id"] in wanted]
    return chosen


def load_examples(inventory):
    """Exemples phrase-plan hors du jeu mesuré ; « @Nom » désigne la couche de l'inventaire de ce nom."""
    names = {l["name"]: l["layer_id"] for l in inventory["layers"]}

    def resolve(value):
        if isinstance(value, str) and value.startswith("@"):
            return names[value[1:]]
        if isinstance(value, list):
            return [resolve(v) for v in value]
        if isinstance(value, dict):
            return {k: resolve(v) for k, v in value.items()}
        return value
    return [{"text": e["text"], "plan": resolve(e["plan"])} for e in load_json("examples.json")]


# -- catalogue, schéma, invite ------------------------------------------------

STOPWORDS = {"les", "des", "une", "dans", "sur", "pour", "par", "est", "sont", "qui", "que", "aux", "avec",
             "sans", "plus", "moins", "tout", "toute", "ses", "son", "leur", "depuis", "puis", "cette", "ces",
             "the", "and", "for", "with", "from", "that", "this", "are", "its"}


def words(text):
    return {w[:5] for w in WORD.findall(text.lower()) if w not in STOPWORDS}


def filter_commands(text, catalog, size):
    """Les `size` commandes les plus proches de la phrase par recouvrement lexical, ordre du catalogue en cas d'égalité."""
    sentence = words(text)
    scored = []
    for rank, c in enumerate(catalog["commands"]):
        vocabulary = words(c["description_fr"] + " " + c["description_en"] + " " + c["id"].replace(".", " ").replace("_", " "))
        scored.append((-len(sentence & vocabulary), rank, c["id"]))
    return [cid for _, _, cid in sorted(scored)[:size]]


def sanitize(node):
    """Retire des motifs ce que llama.cpp ne sait pas lire (assertions négatives)."""
    if isinstance(node, dict):
        out = {k: sanitize(v) for k, v in node.items()}
        if isinstance(out.get("pattern"), str):
            out["pattern"] = LOOKAHEAD.sub("", out["pattern"])
        return out
    if isinstance(node, list):
        return [sanitize(v) for v in node]
    return node


def plan_schema(catalog, command_ids):
    steps = [{"type": "object",
              "properties": {"command": {"const": c["id"]}, "params": c["params"]},
              "required": ["command", "params"], "additionalProperties": False}
             for c in catalog["commands"] if c["id"] in command_ids]
    return sanitize({
        "$schema": "https://json-schema.org/draft/2020-12/schema", "$defs": catalog["$defs"],
        "anyOf": [{"type": "array", "items": {"anyOf": steps}, "minItems": 1, "maxItems": 12},
                  {"type": "object", "properties": {"refusal": {"type": "string", "minLength": 1, "maxLength": 300}},
                   "required": ["refusal"], "additionalProperties": False}]})


def system_text(catalog, inventory, command_ids, examples):
    commands = [{"id": c["id"], "description": c["description_fr"], "params": c["params"]}
                for c in catalog["commands"] if c["id"] in command_ids]
    text = (SYSTEM + "\n\nDÉFINITIONS PARTAGÉES (les $ref renvoient ici) :\n" + json.dumps(catalog["$defs"], ensure_ascii=False)
            + "\n\nCATALOGUE :\n" + json.dumps(commands, ensure_ascii=False)
            + "\n\nINVENTAIRE DU PROJET :\n" + json.dumps(inventory, ensure_ascii=False))
    if examples:
        text += "\n\nEXEMPLES :\n" + "\n".join(
            f"Demande : {e['text']}\nPlan : {json.dumps(e['plan'], ensure_ascii=False)}" for e in examples)
    return text


def build_prompt(template, no_think, system, user, previous=None, errors=None):
    """Invite brute au format du modèle. En relance : sa réponse précédente puis la liste des écarts."""
    turns = [("user", user)]
    if previous is not None:
        turns += [("assistant", previous),
                  ("user", "Ta réponse n'est pas conforme : " + " ; ".join(errors[:8])
                   + ". Redonne le plan corrigé, uniquement le JSON.")]
    if template == "mistral":
        text = "<s>[INST] " + system + "\n\n" + turns[0][1] + " [/INST]"
        for role, content in turns[1:]:
            text += (" " + content + "</s>") if role == "assistant" else ("[INST] " + content + " [/INST]")
        return text
    text = f"<|im_start|>system\n{system}<|im_end|>\n"
    for role, content in turns:
        text += f"<|im_start|>{role}\n{content}<|im_end|>\n"
    text += "<|im_start|>assistant\n" + ("<think>\n\n</think>\n\n" if no_think else "")
    return text


# -- sortie du modèle ----------------------------------------------------------

def extract_json(text):
    """La sortie doit être exactement du JSON (AD-23 : rejetée, jamais réparée).

    Sont tolérés, parce qu'ils ne touchent pas au contenu : le bloc de réflexion du modèle
    (<think>…</think>) et une clôture de code ``` autour du JSON. Tout autre texte avant ou
    après rend la sortie non conforme."""
    text = THINK.sub("", text).strip()
    fenced = FENCE.fullmatch(text)
    if fenced:
        text = fenced.group(1).strip()
    try:
        return json.loads(text), None
    except json.JSONDecodeError as exc:
        return None, f"la sortie n'est pas du JSON valide ({exc.msg})"


def judge(text, sentence, catalog, inventory):
    """Rend (statut, écarts, valeur) ; statut vaut correct, wrong ou nonconform."""
    value, problem = extract_json(text)
    if problem:
        return "nonconform", [problem], None
    _, errors = check_output(value, catalog, inventory)
    if errors:
        return "nonconform", errors, value
    want = {"refusal": "x"} if sentence["expect_refusal"] else sentence["expected"]
    return ("correct" if plans_equal(value, want, catalog) else "wrong"), [], value


# -- appel du modèle ------------------------------------------------------------

def call_llama(args, prompt, schema):
    with tempfile.TemporaryDirectory() as tmp:
        prompt_file = Path(tmp) / "prompt.txt"
        prompt_file.write_text(prompt, encoding="utf-8")
        cmd = [args.llama, "-m", args.model, "-c", str(args.ctx), "-ngl", str(args.ngl), "-f", str(prompt_file),
               "-n", str(args.n_predict), "--temp", "0", "--seed", "1", "--no-display-prompt", "-no-cnv"]
        if schema is not None:
            schema_file = Path(tmp) / "schema.json"
            schema_file.write_text(json.dumps(schema), encoding="utf-8")
            cmd += ["-jf", str(schema_file)]
        started = time.time()
        done = subprocess.run(cmd, capture_output=True, timeout=args.timeout)
        wall = (time.time() - started) * 1000
    out = ANSI.sub("", done.stdout.decode("utf-8", errors="replace")).replace("[end of text]", "").strip()
    err = done.stderr.decode("utf-8", errors="replace")

    def grab(pattern):
        found = re.search(pattern, err)
        return tuple(float(g) for g in found.groups()) if found else None
    timing = {"wall_ms": round(wall), "exit": done.returncode,
              "load_ms": (grab(r"load time\s*=\s*([\d.]+)\s*ms") or (None,))[0],
              "prompt": grab(r"prompt eval time\s*=\s*([\d.]+)\s*ms\s*/\s*(\d+)\s*tokens"),
              "generation": grab(r"\beval time\s*=\s*([\d.]+)\s*ms\s*/\s*(\d+)\s*runs")}
    return out, timing


def make_responder(args):
    if args.replay:
        canned = json.loads(Path(args.replay).read_text(encoding="utf-8"))

        def replay(prompt, schema, attempt, sentence):
            outputs = canned[sentence["id"]]
            if attempt > len(outputs):
                raise KeyError(f"{sentence['id']}: pas de sortie enregistrée pour l'essai {attempt}")
            out = outputs[attempt - 1]
            return (out if isinstance(out, str) else json.dumps(out, ensure_ascii=False)), {}
        return replay
    return lambda prompt, schema, attempt, sentence: call_llama(args, prompt, schema)


# -- mesure ---------------------------------------------------------------------

def measure(sentences, catalog, inventory, args, responder):
    examples = load_examples(inventory) if args.examples else []
    all_ids = [c["id"] for c in catalog["commands"]]
    results = []
    for sentence in sentences:
        ids = filter_commands(sentence["text"], catalog, args.filter_size) if args.catalog == "filtered" else all_ids
        wanted = set()
        if not sentence["expect_refusal"]:
            wanted = {s["command"] for s in sentence["expected"]}
        schema = plan_schema(catalog, ids) if args.output == "schema" else None
        system = system_text(catalog, inventory, ids, examples)
        record = {"id": sentence["id"], "text": sentence["text"], "recall": (not wanted or wanted <= set(ids)), "attempts": []}
        previous = errors = None
        for attempt in (1, 2):
            prompt = build_prompt(args.template, args.no_think, system, sentence["text"], previous, errors)
            try:
                text, timing = responder(prompt, schema, attempt, sentence)
            except (subprocess.TimeoutExpired, OSError) as exc:
                text, timing = "", {"exit": -1, "failure": f"{type(exc).__name__}: {exc}"}
            if timing.get("exit") not in (0, None):
                record["attempts"].append({"status": "error", "errors": [timing.get("failure", f"llama-completion a rendu le code {timing['exit']}")],
                                           "output": text, "timing": timing})
                break
            status, errors, _ = judge(text, sentence, catalog, inventory)
            record["attempts"].append({"status": status, "errors": errors, "output": text, "timing": timing})
            if status != "nonconform":
                break
            previous = text
        record["correct_first"] = record["attempts"][0]["status"] == "correct"
        record["correct_after"] = any(a["status"] == "correct" for a in record["attempts"])
        record["conform_first"] = record["attempts"][0]["status"] not in ("nonconform", "error")
        results.append(record)
    return results


def summarize(results):
    n = len(results)

    def mean(values):
        values = [v for v in values if v is not None]
        return round(sum(values) / len(values), 1) if values else None
    attempts = [a for r in results for a in r["attempts"]]
    gen = [a["timing"].get("generation") for a in attempts]
    pro = [a["timing"].get("prompt") for a in attempts]
    return {
        "sentences": n,
        "correct_first": sum(r["correct_first"] for r in results),
        "correct_after_retry": sum(r["correct_after"] for r in results),
        "rate_first": round(sum(r["correct_first"] for r in results) / n, 3) if n else None,
        "rate_after_retry": round(sum(r["correct_after"] for r in results) / n, 3) if n else None,
        "conform_first": sum(r["conform_first"] for r in results),
        "retried": sum(len(r["attempts"]) > 1 for r in results),
        "call_errors": sum(any(a["status"] == "error" for a in r["attempts"]) for r in results),
        "catalog_recall": sum(r["recall"] for r in results),
        "mean_load_ms": mean([a["timing"].get("load_ms") for a in attempts]),
        "mean_prompt_ms": mean([p[0] for p in pro if p]),
        "mean_prompt_tokens": mean([p[1] for p in pro if p]),
        "mean_generation_ms": mean([g[0] for g in gen if g]),
        "mean_generated_tokens": mean([g[1] for g in gen if g]),
    }


def parse_args(argv):
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--llama", help="chemin de llama-completion")
    p.add_argument("--model", help="chemin du fichier GGUF")
    p.add_argument("--quant", default="?", help="étiquette de quantification, pour le rapport")
    p.add_argument("--catalog", choices=["full", "filtered"], default="full")
    p.add_argument("--filter-size", type=int, default=8, help="commandes gardées si le catalogue est filtré")
    p.add_argument("--examples", action="store_true", help="ajoute des exemples (examples.json) à l'invite")
    p.add_argument("--output", choices=["free", "schema"], default="free", help="sortie libre ou contrainte par schéma JSON")
    p.add_argument("--lang", choices=["fr", "en"], default="fr")
    p.add_argument("--ids", default="", help="phrases à mesurer, séparées par des virgules (défaut : toutes)")
    p.add_argument("--template", choices=["chatml", "mistral"], default="chatml")
    p.add_argument("--think", dest="no_think", action="store_false", help="laisse le mode réflexion du modèle (chatml)")
    p.add_argument("--ctx", type=int, default=8192)
    p.add_argument("--ngl", type=int, default=99)
    p.add_argument("--n-predict", type=int, default=1200)
    p.add_argument("--timeout", type=int, default=300)
    p.add_argument("--replay", help="rejoue des sorties enregistrées au lieu d'appeler un modèle")
    p.add_argument("--out", help="rapport JSON (défaut : reports/<date>.json)")
    args = p.parse_args(argv)
    if not args.replay and not (args.llama and args.model):
        p.error("--llama et --model sont requis, sauf avec --replay")
    return args


def main(argv=None):
    sys.stdout.reconfigure(encoding="utf-8")  # la console Windows n'est pas en UTF-8 par défaut
    args = parse_args(argv)
    catalog, inventory = load_json("catalog.json"), load_json("inventory.json")
    sentences = load_sentences(args.lang, [i for i in args.ids.split(",") if i])
    if not sentences:
        known = ", ".join(s["id"] for s in load_sentences(args.lang, []))
        print(f"aucune phrase choisie : --ids ne correspond à rien en --lang {args.lang}. Identifiants connus : {known}")
        return 2
    if not args.replay:
        for label, path in (("--llama", args.llama), ("--model", args.model)):
            if not Path(path).is_file():
                print(f"{label} : fichier introuvable « {path} »")
                return 2
    results = measure(sentences, catalog, inventory, args, make_responder(args))
    summary = summarize(results)
    report = {"parameters": {k: v for k, v in vars(args).items() if k != "out"}, "summary": summary, "results": results}
    out = Path(args.out) if args.out else HERE / "reports" / (time.strftime("%Y%m%d-%H%M%S") + ".json")
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(report, ensure_ascii=False, indent=1), encoding="utf-8")
    n = summary["sentences"]
    print(f"{n} phrases : correctes au premier essai {summary['correct_first']}/{n} ({summary['rate_first']:.0%}), "
          f"après relance {summary['correct_after_retry']}/{n} ({summary['rate_after_retry']:.0%}) ; "
          f"conformes d'emblée {summary['conform_first']}/{n}, relancées {summary['retried']}"
          + (f", ERREURS D'APPEL {summary['call_errors']} (le modèle n'est pas en cause)" if summary["call_errors"] else ""))
    for r in results:
        if not r["correct_after"]:
            last = r["attempts"][-1]
            print(f"  {r['id']} faux ({last['status']}) : {'; '.join(last['errors'][:2]) or 'plan conforme mais différent'}")
    print(f"rapport : {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
