"""Une commande pour ré-entraîner le modèle de composition (ticket 5.10, tâche 2).

    uv run --project training/entrainement python training/entrainement/pipeline.py --sortie <dossier hors dépôt>

Étapes, dans l'ordre, chacune sautée sur `--reprendre` si son produit existe déjà :

 1. contrôle du GPU, avant tout téléchargement et avant de toucher au dossier de sortie ;
 2. dossier de sortie : refus s'il est déjà rempli (sauf `--reprendre`) ou dans le dépôt ;
 3. génération des jeux par `cassini-training --format chat` : graine 1 pour l'entraînement,
    graine 2 pour la validation (les prompts déjà vus à l'entraînement en sont retirés) ;
 4. base Qwen3.5-4B, épinglée à une révision, dans le cache de Hugging Face ;
 5. QLoRA (nf4, gradient checkpointing, graine fixe), perte sur la réponse seule ;
 6. score de validation, exact (`scoring.py`), décodage glouton ;
 7. llama.cpp cloné à un commit fixe dans un cache hors dépôt, construit sur CPU ;
 8. base et adaptateur convertis en GGUF, fusion, quantification Q4_K_M ; chaque fichier
    s'écrit sous un nom temporaire et ne prend son nom final qu'une fois complet ;
 9. essai de `llama-completion` sur une demande de validation, par les deux chemins :
    base GGUF avec l'adaptateur GGUF, et modèle fusionné ;
10. `rapport.json`.

Python ne met rien en forme : le texte vu par le modèle vient de `training/src/prompt.rs`.
Les modèles et poids sortent hors du dépôt. torch et transformers ne sont importés que dans
les étapes qui en ont besoin : les gardes se testent sans eux (`tests/`).
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import random
import shutil
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
sys.path.insert(0, str(HERE))

import data  # noqa: E402
import scoring  # noqa: E402

# -- choix épinglés ------------------------------------------------------------------------

BASE_REPO = "Qwen/Qwen3.5-4B"  # Apache-2.0
BASE_REVISION = "851bf6e806efd8d0a36b00ddf55e13ccb7b8cd0a"
# Le commit du sous-module llama.cpp de llama-cpp-sys-2, celui que lie `cassini-ai` ; un test
# vérifie que `Cargo.lock` tire toujours cette version de llama-cpp-sys-2.
LLAMA_CPP_URL = "https://github.com/ggml-org/llama.cpp"
LLAMA_CPP_COMMIT = "d81235049384534c167caea52b85a694f6103d14"
LLAMA_CPP_SYS_VERSION = "0.1.160"

TRAINING_SEED = 1
VALIDATION_SEED = 2
TORCH_SEED = 1
TRAINING_COUNT = 5000
VALIDATION_COUNT = 300
# Une séquence mesurée fait moins de 1 400 jetons : 2 048 laisse de la marge sans tronquer.
MAX_LENGTH = 2048
# Le GPU visé a 8 Go ; on refuse en dessous de 7 Gio de mémoire totale.
MIN_GPU_MEMORY = 7 * 1024**3

HYPERPARAMETERS: dict[str, Any] = {
    "methode": "QLoRA nf4, double quantification, calcul en bf16",
    "rang": 16,
    "alpha": 32,
    "dropout": 0.05,
    # Projections du modèle de langue seulement (la tour de vision n'est pas touchée).
    "modules": (
        r".*language_model\.layers\.\d+\."
        r"(self_attn\.(q_proj|k_proj|v_proj|o_proj)"
        r"|linear_attn\.(in_proj_z|out_proj)"
        r"|mlp\.(gate_proj|up_proj|down_proj))"
    ),
    "taux_apprentissage": 2e-4,
    "echauffement": 0.03,
    "planning": "cosinus",
    "epoques": 1,
    "lot": 1,
    "accumulation": 16,
    "norme_max_gradient": 1.0,
    "longueur_max": MAX_LENGTH,
    "jetons_max_generes": 512,
}

QUANTIZATION = "Q4_K_M"
BASE_NAME = "qwen3.5-4b-base-Q4_K_M.gguf"
ADAPTER_NAME = "cassini-composition-adaptateur-f16.gguf"
MERGED_NAME = "cassini-composition-4b-Q4_K_M.gguf"
# Déclarations de modèle (tâche 3) : lues par `model.list` du noyau, posées à côté des GGUF.
# Noms de fichiers seuls, même dossier ; le modèle ajusté attend le prompt court.
ADAPTED_DECLARATION_NAME = "cassini-composition-4b-adaptateur.modele.json"
MERGED_DECLARATION_NAME = "cassini-composition-4b.modele.json"
DECLARATIONS = {
    ADAPTED_DECLARATION_NAME: {"model": BASE_NAME, "adapter": ADAPTER_NAME, "prompt": "short"},
    MERGED_DECLARATION_NAME: {"model": MERGED_NAME, "prompt": "short"},
}


class PipelineStop(RuntimeError):
    """Arrêt volontaire, avec un message clair pour le propriétaire."""


def cache_dir() -> Path:
    """Cache hors dépôt : clone de llama.cpp et environnement."""
    return Path(os.environ.get("CASSINI_CACHE_ENTRAINEMENT", Path.home() / ".cache" / "cassini-entrainement"))


def log(message: str) -> None:
    print(f"[{time.strftime('%H:%M:%S')}] {message}", flush=True)


# -- gardes (testées sans torch) -----------------------------------------------------------


def prepare_output(output: Path, resume: bool) -> None:
    """Crée le dossier de sortie. Refuse un dossier dans le dépôt (les poids ne s'y versionnent
    jamais) et un dossier déjà rempli, sauf `--reprendre` : rien n'est jamais écrasé."""
    output = output.resolve()
    if output == ROOT or ROOT in output.parents:
        raise PipelineStop(f"le dossier de sortie {output} est dans le dépôt : choisissez-en un hors du dépôt")
    if output.exists() and any(output.iterdir()) and not resume:
        raise PipelineStop(
            f"le dossier de sortie {output} n'est pas vide : rien n'est écrasé. "
            "Choisissez un dossier vide, ou relancez avec --reprendre pour finir un entraînement commencé."
        )
    output.mkdir(parents=True, exist_ok=True)


def check_gpu(total_memory: int | None) -> None:
    """Arrête la chaîne si aucun GPU CUDA n'est vu, ou s'il a moins de 7 Gio."""
    if total_memory is None:
        raise PipelineStop(
            "aucun GPU CUDA visible : l'entraînement exige le GPU de 8 Go (vérifiez nvidia-smi sous WSL)"
        )
    if total_memory < MIN_GPU_MEMORY:
        raise PipelineStop(
            f"GPU trop petit : {total_memory / 1024**3:.1f} Gio, il en faut au moins "
            f"{MIN_GPU_MEMORY / 1024**3:.0f}"
        )


def gpu_memory() -> tuple[int | None, str | None]:
    import torch

    if not torch.cuda.is_available():
        return None, None
    properties = torch.cuda.get_device_properties(0)
    return properties.total_memory, properties.name


def run(command: list[str], **options: Any) -> subprocess.CompletedProcess:
    """Lance un outil ; en cas d'échec, arrête la chaîne en citant la fin de sa sortie d'erreur
    quand elle a été capturée."""
    log("$ " + " ".join(str(c) for c in command))
    result = subprocess.run(command, **options)
    if result.returncode != 0:
        message = f"échec ({result.returncode}) : {' '.join(str(c) for c in command)}"
        stderr = result.stderr
        if isinstance(stderr, bytes):
            stderr = stderr.decode("utf-8", errors="replace")
        if stderr:
            message += f"\n--- fin de la sortie d'erreur ---\n{stderr[-2000:]}"
        raise PipelineStop(message)
    return result


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def temporary(path: Path) -> Path:
    """Nom temporaire, dans le même dossier, sous lequel un outil écrit avant le renommage."""
    return path.with_name(path.name + ".tmp")


def write_atomically(path: Path, command: list[str], **options: Any) -> None:
    """Lance `command`, qui écrit dans `temporary(path)`, puis renomme : un fichier coupé en
    pleine écriture ne porte jamais le nom final, et `--reprendre` ne le reprend jamais."""
    partial = temporary(path)
    partial.unlink(missing_ok=True)
    run(command, **options)
    if not partial.exists():
        raise PipelineStop(f"l'outil n'a pas écrit {partial}")
    partial.replace(path)


# -- étape 3 : génération ------------------------------------------------------------------


def generate(folder: Path, count: int, validation_count: int) -> dict[str, Path]:
    folder.mkdir(parents=True, exist_ok=True)
    sets = {
        "entrainement": (folder / "entrainement.jsonl", TRAINING_SEED, count),
        "validation": (folder / "validation.jsonl", VALIDATION_SEED, validation_count),
    }
    for path, seed, n in sets.values():
        run(
            ["cargo", "run", "-q", "-p", "cassini-training", "--release", "--",
             "--format", "chat", "--seed", str(seed), "--count", str(n), "--out", str(path)],
            cwd=ROOT,
        )
    return {name: path for name, (path, _, _) in sets.items()}


# -- étapes 5 et 6 : entraînement et validation --------------------------------------------


def set_seeds(seed: int) -> None:
    import numpy
    import torch
    import transformers

    random.seed(seed)
    numpy.random.seed(seed)
    torch.manual_seed(seed)
    torch.cuda.manual_seed_all(seed)
    transformers.set_seed(seed)


def load_model(base_dir: str):
    import torch
    from transformers import AutoModelForImageTextToText, BitsAndBytesConfig

    quantization = BitsAndBytesConfig(
        load_in_4bit=True,
        bnb_4bit_quant_type="nf4",
        bnb_4bit_use_double_quant=True,
        bnb_4bit_compute_dtype=torch.bfloat16,
    )
    return AutoModelForImageTextToText.from_pretrained(
        base_dir,
        quantization_config=quantization,
        dtype=torch.bfloat16,
        device_map={"": 0},
    )


def encode(tokenizer, example: data.Example) -> tuple[list[int], int]:
    """Jetons du prompt puis de la réponse, et la longueur de la réponse. Le prompt et la
    réponse se découpent à part : le modèle répond après les jetons du prompt, comme en
    production. Les balises chatml sont lues comme jetons spéciaux."""
    prompt = tokenizer(example.prompt, add_special_tokens=False)["input_ids"]
    answer = tokenizer(example.completion, add_special_tokens=False)["input_ids"]
    return prompt + answer, len(answer)


def train(base_dir: str, examples: list[data.Example], adapter_dir: Path, max_steps: int | None) -> dict[str, Any]:
    import torch
    import torch.nn.functional as F
    from peft import LoraConfig, get_peft_model, prepare_model_for_kbit_training
    from transformers import AutoTokenizer, get_cosine_schedule_with_warmup

    set_seeds(TORCH_SEED)
    h = HYPERPARAMETERS
    tokenizer = AutoTokenizer.from_pretrained(base_dir)
    encoded = [encode(tokenizer, e) for e in examples]

    model = load_model(base_dir)
    model = prepare_model_for_kbit_training(
        model, use_gradient_checkpointing=True, gradient_checkpointing_kwargs={"use_reentrant": False}
    )
    model = get_peft_model(
        model,
        LoraConfig(r=h["rang"], lora_alpha=h["alpha"], lora_dropout=h["dropout"],
                   target_modules=h["modules"], bias="none", task_type="CAUSAL_LM"),
    )
    trainable, total = model.get_nb_trainable_parameters()
    log(f"paramètres entraînés : {trainable:,} sur {total:,}")

    order = []
    for epoch in range(h["epoques"]):
        indices = list(range(len(encoded)))
        random.Random(TORCH_SEED * 1000 + epoch).shuffle(indices)
        order.extend(indices)
    total_steps = len(order) // h["accumulation"]
    if max_steps is not None:
        total_steps = min(total_steps, max_steps)
    # Seuls les exemples d'un pas complet sont vus : un reste sans pas d'optimiseur ne compterait pas.
    order = order[: total_steps * h["accumulation"]]
    parameters = [p for p in model.parameters() if p.requires_grad]
    optimizer = torch.optim.AdamW(parameters, lr=h["taux_apprentissage"], weight_decay=0.0)
    schedule = get_cosine_schedule_with_warmup(optimizer, int(total_steps * h["echauffement"]), total_steps)

    model.train()
    torch.cuda.reset_peak_memory_stats()
    history, window, start = [], [], time.time()
    for rank, index in enumerate(order, start=1):
        tokens, answer_length = encoded[index]
        inputs = torch.tensor([tokens], device="cuda")
        # Logits sur les seules positions qui prédisent la réponse : la perte ne porte que
        # sur elle, et on évite des logits de 248 000 entrées sur tout le prompt.
        outputs = model(input_ids=inputs, logits_to_keep=answer_length + 1, use_cache=False)
        logits = outputs.logits[0, :-1].float()
        target = inputs[0, -answer_length:]
        loss = F.cross_entropy(logits, target)
        (loss / h["accumulation"]).backward()
        window.append(loss.item())
        if rank % h["accumulation"] == 0:
            torch.nn.utils.clip_grad_norm_(parameters, h["norme_max_gradient"])
            optimizer.step()
            schedule.step()
            optimizer.zero_grad(set_to_none=True)
            step = rank // h["accumulation"]
            if step % 10 == 0 or step == total_steps:
                mean = sum(window) / len(window)
                history.append({"pas": step, "perte": round(mean, 5)})
                elapsed = time.time() - start
                log(f"pas {step}/{total_steps} perte {mean:.4f} "
                    f"({elapsed / 60:.1f} min, reste ~{elapsed / step * (total_steps - step) / 60:.0f} min, "
                    f"pic {torch.cuda.max_memory_reserved() / 1024**3:.2f} Gio)")
                window = []

    model.save_pretrained(adapter_dir)
    return {
        "modele": model,
        "pas": total_steps,
        "exemples_vus": len(order),
        "perte": history,
        "perte_finale": history[-1]["perte"] if history else None,
        "duree_s": round(time.time() - start),
        "pic_memoire_gpu_octets": torch.cuda.max_memory_reserved(),
        "parametres_entraines": trainable,
    }


def validate(model, tokenizer, examples: list[data.Example], outputs_file: Path) -> dict[str, Any]:
    import torch

    end = tokenizer.convert_tokens_to_ids(data.END_OF_ANSWER)
    model.eval()
    model.config.use_cache = True
    score, start = scoring.Score(), time.time()
    with open(outputs_file, "w", encoding="utf-8") as outputs, torch.no_grad():
        for rank, example in enumerate(examples, start=1):
            inputs = tokenizer(example.prompt, add_special_tokens=False, return_tensors="pt")["input_ids"].to("cuda")
            produced = model.generate(
                input_ids=inputs,
                max_new_tokens=HYPERPARAMETERS["jetons_max_generes"],
                do_sample=False,
                eos_token_id=end,
                pad_token_id=end,
            )
            text = tokenizer.decode(produced[0, inputs.shape[1]:], skip_special_tokens=False)
            expected = data.expected_answer(example)
            match = score.record(text, expected)
            outputs.write(json.dumps({"rang": rank, "juste": match, "attendu": expected, "sortie": text},
                                     ensure_ascii=False) + "\n")
            if rank % 25 == 0:
                log(f"validation {rank}/{len(examples)} : {sum(score.correct.values())} justes")
    result = score.to_dict()
    result["duree_s"] = round(time.time() - start)
    return result


# -- étapes 7 à 9 : llama.cpp --------------------------------------------------------------


def prepare_llama_cpp() -> Path:
    """Clone llama.cpp au commit fixé (hors dépôt) et construit les outils sur CPU."""
    folder = cache_dir() / "llama.cpp"
    if not (folder / ".git").exists():
        folder.mkdir(parents=True, exist_ok=True)
        run(["git", "init", "-q"], cwd=folder)
        run(["git", "remote", "add", "origin", LLAMA_CPP_URL], cwd=folder)
    head = subprocess.run(["git", "rev-parse", "HEAD"], cwd=folder, capture_output=True, text=True).stdout.strip()
    if head != LLAMA_CPP_COMMIT:
        run(["git", "fetch", "-q", "--depth", "1", "origin", LLAMA_CPP_COMMIT], cwd=folder)
        run(["git", "checkout", "-q", "--detach", LLAMA_CPP_COMMIT], cwd=folder)
        shutil.rmtree(folder / "build", ignore_errors=True)
    # llama-tokenize sert à `parite_decoupeurs.py` (tâche 4b).
    tools = ["llama-quantize", "llama-export-lora", "llama-completion", "llama-tokenize"]
    if not all((folder / "build" / "bin" / t).exists() for t in tools):
        run(["cmake", "-S", ".", "-B", "build", "-DGGML_CUDA=OFF", "-DLLAMA_CURL=OFF",
             "-DLLAMA_BUILD_TESTS=OFF", "-DCMAKE_BUILD_TYPE=Release"], cwd=folder)
        run(["cmake", "--build", "build", "-j", str(os.cpu_count() or 4), "--target", *tools], cwd=folder)
    return folder


def convert(llama: Path, base_dir: str, adapter_dir: Path, output: Path, work: Path, resume: bool) -> None:
    py = sys.executable
    base_f16 = work / "base-f16.gguf"
    merged_f16 = work / "fusionne-f16.gguf"
    adapter = output / ADAPTER_NAME
    quantize = llama / "build" / "bin" / "llama-quantize"

    def todo(path: Path) -> bool:
        return not (resume and path.exists())

    if todo(output / BASE_NAME) or todo(output / MERGED_NAME):
        if todo(base_f16):
            write_atomically(base_f16, [py, str(llama / "convert_hf_to_gguf.py"), base_dir, "--outtype", "f16",
                                        "--outfile", str(temporary(base_f16))])
    if todo(adapter):
        write_atomically(adapter, [py, str(llama / "convert_lora_to_gguf.py"), str(adapter_dir), "--base", base_dir,
                                   "--outtype", "f16", "--outfile", str(temporary(adapter))])
    if todo(output / MERGED_NAME):
        if todo(merged_f16):
            write_atomically(merged_f16, [str(llama / "build" / "bin" / "llama-export-lora"), "-m", str(base_f16),
                                          "--lora", str(adapter), "-o", str(temporary(merged_f16))])
        write_atomically(output / MERGED_NAME,
                         [str(quantize), str(merged_f16), str(temporary(output / MERGED_NAME)), QUANTIZATION])
    if todo(output / BASE_NAME):
        write_atomically(output / BASE_NAME,
                         [str(quantize), str(base_f16), str(temporary(output / BASE_NAME)), QUANTIZATION])
    # Les intermédiaires f16 (plusieurs Go chacun) ne servent plus.
    for intermediate in (base_f16, merged_f16):
        intermediate.unlink(missing_ok=True)


def first_calls_example(examples: list[data.Example]) -> data.Example:
    """Le premier exemple de validation dont la réponse attendue est un plan d'appels."""
    for example in examples:
        plan = scoring.read_plan(data.expected_answer(example))
        if plan is not None and plan["kind"] == "calls":
            return example
    raise PipelineStop("aucun exemple de validation n'attend un plan d'appels")


def try_model(llama: Path, model_args: list[str], example: data.Example, work: Path, label: str) -> dict[str, Any]:
    """Une demande de validation passée à `llama-completion` (prompt brut, déjà en chatml,
    décodage glouton) : la sortie doit se lire comme un plan JSON."""
    prompt_file = work / "essai-prompt.txt"
    prompt_file.write_text(example.prompt, encoding="utf-8")
    result = run(
        [str(llama / "build" / "bin" / "llama-completion"), *model_args, "-f", str(prompt_file),
         "-n", str(HYPERPARAMETERS["jetons_max_generes"]), "--temp", "0", "-no-cnv", "--no-display-prompt",
         "-c", str(MAX_LENGTH + HYPERPARAMETERS["jetons_max_generes"]), "--seed", str(TORCH_SEED)],
        capture_output=True, text=True,
    )
    output = result.stdout.strip()
    # llama-completion peut écrire la marque de fin ou « [end of text] » après le plan.
    for end in ("[end of text]", data.END_OF_ANSWER):
        output = output.split(end)[0].strip()
    expected = data.expected_answer(example)
    log(f"essai llama-completion ({label}) : {output}")
    return {
        "outil": "llama-completion",
        "modele": label,
        "sortie": output,
        "attendu": expected,
        "se_lit_comme_un_plan": scoring.read_plan(output) is not None,
        "juste": scoring.plan_matches(output, expected),
    }


# -- orchestration -------------------------------------------------------------------------


def read_report(path: Path) -> dict[str, Any]:
    if path.exists():
        return json.loads(path.read_text(encoding="utf-8"))
    return {}


def write_declarations(output: Path) -> list[str]:
    """Écrit les déclarations de modèle à côté des GGUF (base + adaptateur, fusionné), sous un
    nom temporaire puis renommées, et rend leurs noms. Réécrites à l'identique à la reprise."""
    for name, declaration in DECLARATIONS.items():
        partial = temporary(output / name)
        partial.write_text(json.dumps(declaration, ensure_ascii=False) + "\n", encoding="utf-8")
        partial.replace(output / name)
    return list(DECLARATIONS)


def write_report(path: Path, report: dict[str, Any]) -> None:
    partial = path.with_suffix(".tmp")
    partial.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    partial.replace(path)


def versions() -> dict[str, str]:
    from importlib.metadata import version

    packages = ["torch", "transformers", "peft", "accelerate", "bitsandbytes", "safetensors", "numpy",
                "huggingface-hub"]
    return {p: version(p) for p in packages} | {"python": platform.python_version()}


def repository_state() -> dict[str, Any]:
    """Commit du dépôt, et si l'arbre de travail diffère de ce commit (chemins modifiés)."""
    def git(*args: str) -> str:
        return subprocess.run(["git", *args], cwd=ROOT, capture_output=True, text=True).stdout

    changed = [line[3:] for line in git("status", "--porcelain").splitlines() if line.strip()]
    return {
        "commit": git("rev-parse", "HEAD").strip(),
        "arbre_modifie": bool(changed),
        "chemins_modifies": changed,
        "uv_lock_sha256": sha256(HERE / "uv.lock"),
    }


def launch(args: argparse.Namespace) -> dict[str, Any]:
    output: Path = args.output.resolve()
    # Le GPU d'abord : un refus ne laisse ni dossier rempli, ni téléchargement.
    memory, gpu_name = gpu_memory()
    check_gpu(memory)
    log(f"GPU : {gpu_name}, {memory / 1024**3:.1f} Gio")

    prepare_output(output, args.resume)
    work = output / "travail"
    work.mkdir(exist_ok=True)
    report_path = output / "rapport.json"
    report = read_report(report_path) if args.resume else {}

    report.update({
        "base": {"depot": BASE_REPO, "revision": BASE_REVISION, "licence": "Apache-2.0"},
        "llama_cpp": {"url": LLAMA_CPP_URL, "commit": LLAMA_CPP_COMMIT,
                      "llama_cpp_sys_2": LLAMA_CPP_SYS_VERSION},
        "depot_cassini": repository_state(),
        "graines": {"entrainement": TRAINING_SEED, "validation": VALIDATION_SEED, "torch": TORCH_SEED},
        "hyperparametres": HYPERPARAMETERS | ({"pas_max": args.max_steps} if args.max_steps else {}),
        "gpu": {"nom": gpu_name, "memoire_totale_octets": memory},
        "versions": versions(),
    })
    report.pop("commit_cassini", None)
    write_report(report_path, report)

    # 3. jeux
    data_dir = work / "donnees"
    if args.resume and (data_dir / "validation.jsonl").exists():
        sets = {"entrainement": data_dir / "entrainement.jsonl", "validation": data_dir / "validation.jsonl"}
    else:
        sets = generate(data_dir, args.examples, args.validation)
    training = data.read_jsonl(sets["entrainement"])
    validation, overlap = data.drop_overlap(data.read_jsonl(sets["validation"]), training)

    # 4. base
    from huggingface_hub import snapshot_download
    from transformers import AutoTokenizer

    log(f"base {BASE_REPO} @ {BASE_REVISION}")
    base_dir = snapshot_download(BASE_REPO, revision=BASE_REVISION)
    tokenizer = AutoTokenizer.from_pretrained(base_dir)

    def length(e: data.Example) -> int:
        return len(encode(tokenizer, e)[0])

    training, too_long = data.drop_too_long(training, length, MAX_LENGTH)
    validation, too_long_validation = data.drop_too_long(validation, length, MAX_LENGTH)
    lengths = sorted(length(e) for e in training)
    report["donnees"] = {
        "entrainement": len(training),
        "validation": len(validation),
        "ecartes_trop_longs": too_long,
        "ecartes_trop_longs_validation": too_long_validation,
        "validation_retires_car_vus_a_l_entrainement": overlap,
        "jetons_mediane": lengths[len(lengths) // 2],
        "jetons_max": lengths[-1],
        "empreinte_entrainement": sha256(sets["entrainement"]),
        "empreinte_validation": sha256(sets["validation"]),
    }
    write_report(report_path, report)
    log(f"{len(training)} exemples d'entraînement, {len(validation)} de validation, "
        f"{too_long} écartés car trop longs")

    # 5 et 6. entraînement et validation
    adapter_dir = work / "adaptateur-hf"
    if not (args.resume and (adapter_dir / "adapter_model.safetensors").exists()):
        result = train(base_dir, training, adapter_dir, args.max_steps)
        model = result.pop("modele")
        report["entrainement"] = result
        write_report(report_path, report)
    else:
        model = None
    if not (args.resume and "validation" in report):
        if model is None:
            from peft import PeftModel

            model = PeftModel.from_pretrained(load_model(base_dir), adapter_dir)
        log(f"validation sur {len(validation)} exemples")
        report["validation"] = validate(model, tokenizer, validation, output / "validation-sorties.jsonl")
        write_report(report_path, report)
        log(f"score de validation : {report['validation']['justes']}/{report['validation']['total']}")
    del model
    import torch

    torch.cuda.empty_cache()

    # 7 et 8. GGUF
    llama = prepare_llama_cpp()
    convert(llama, base_dir, adapter_dir, output, work, args.resume)
    report["sorties"] = {
        name: {"fichier": name, "octets": (output / name).stat().st_size, "sha256": sha256(output / name)}
        for name in (BASE_NAME, ADAPTER_NAME, MERGED_NAME)
    }
    report["sorties"][BASE_NAME]["role"] = "base, sur laquelle se charge l'adaptateur"
    report["sorties"][ADAPTER_NAME]["role"] = "adaptateur LoRA de composition"
    report["sorties"][MERGED_NAME]["role"] = "base et adaptateur fusionnés, quantifiés"
    # Les déclarations : le dossier de sortie, copié dans un dossier de modèles, est prêt.
    report["declarations"] = write_declarations(output)
    write_report(report_path, report)

    # 9. essai, par les deux chemins, sur une demande qui attend un plan d'appels
    example = first_calls_example(validation)
    report["essai"] = {
        "base_et_adaptateur": try_model(
            llama, ["-m", str(output / BASE_NAME), "--lora", str(output / ADAPTER_NAME)],
            example, work, "base GGUF + adaptateur GGUF"),
        "fusionne": try_model(llama, ["-m", str(output / MERGED_NAME)], example, work, "modèle fusionné"),
    }
    write_report(report_path, report)
    unreadable = [name for name, trial in report["essai"].items() if not trial["se_lit_comme_un_plan"]]
    if unreadable and args.max_steps is not None:
        # Essai rapide (`--pas-max`) : un adaptateur à peine entraîné ne sait pas encore répondre ;
        # la chaîne a fait son travail, l'essai est seulement consigné.
        log(f"essai rapide : pas de plan JSON lisible ({', '.join(unreadable)}), attendu après si peu de pas")
    elif unreadable:
        raise PipelineStop(f"pas de plan JSON lisible ({', '.join(unreadable)}) : voir rapport.json")
    return report


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Ré-entraîne le modèle de composition de Cassini.")
    parser.add_argument("--sortie", dest="output", type=Path, required=True,
                        help="dossier de sortie, vide et hors du dépôt")
    parser.add_argument("--reprendre", dest="resume", action="store_true",
                        help="finir un entraînement commencé dans --sortie")
    parser.add_argument("--exemples", dest="examples", type=int, default=TRAINING_COUNT,
                        help="exemples d'entraînement")
    parser.add_argument("--validation", dest="validation", type=int, default=VALIDATION_COUNT,
                        help="exemples de validation engendrés")
    parser.add_argument("--pas-max", dest="max_steps", type=int, default=None,
                        help="borne de pas (essai rapide de la chaîne)")
    args = parser.parse_args(argv)
    start = time.time()
    try:
        report = launch(args)
    except PipelineStop as stop:
        print(f"arrêt : {stop}", file=sys.stderr)
        return 1
    report["duree_totale_s"] = round(time.time() - start)
    write_report(args.output.resolve() / "rapport.json", report)
    log(f"terminé en {report['duree_totale_s'] / 60:.0f} min ; rapport : {args.output / 'rapport.json'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
