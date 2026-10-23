"""Parité des découpeurs (ticket 5.10, tâche 4b) : le découpeur de Hugging Face qui a servi à
l'entraînement et celui de llama.cpp, qui lit le GGUF dans le produit, rendent-ils les mêmes
jetons pour les mêmes prompts ?

    UV_PROJECT_ENVIRONMENT=~/.cache/cassini-entrainement/venv \\
      uv run --project training/entrainement python training/entrainement/parite_decoupeurs.py \\
      --gguf ~/cassini-entrainement/5-10-t2/qwen3.5-4b-base-Q4_K_M.gguf

Lancée à la main, jamais par la recette. Sur `--nombre` prompts (300 par défaut) du jeu
`--format chat` engendré avec la graine de validation (ou lus dans `--jeu`), elle découpe
chaque prompt par le découpeur de la base `Qwen/Qwen3.5-4B` à la révision épinglée
(`add_special_tokens=False`, comme `pipeline.encode`) et par `llama-tokenize` du llama.cpp du
cache (`--ids --no-bos --no-escape`, jetons spéciaux lus comme tels, aucune séquence
d'échappement réinterprétée), puis imprime le nombre de prompts
qui diffèrent et le premier écart. Code de sortie 1 s'il y a un écart.

La comparaison (`compare`, `parse_ids`) se teste sans torch ni transformers (`tests/`).
"""

from __future__ import annotations

import argparse
import ast
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
sys.path.insert(0, str(HERE))

import data  # noqa: E402
import pipeline  # noqa: E402

DEFAULT_COUNT = 300


@dataclass(frozen=True)
class Gap:
    """Premier écart d'un prompt : son rang dans le jeu, le rang du premier jeton qui diffère,
    et quelques jetons de chaque découpeur à partir de là."""

    prompt: int
    token: int
    hugging_face: list[int]
    llama_cpp: list[int]


def parse_ids(stdout: str) -> list[int]:
    """Lit la sortie de `llama-tokenize --ids` : une liste Python d'entiers, `[1, 2, 3]`, sur la
    dernière ligne non vide (les journaux de chargement vont sur la sortie d'erreur)."""
    lines = [line.strip() for line in stdout.splitlines() if line.strip()]
    if not lines:
        raise ValueError("llama-tokenize n'a rien écrit")
    ids = ast.literal_eval(lines[-1])
    if not isinstance(ids, list) or not all(isinstance(i, int) for i in ids):
        raise ValueError(f"sortie de llama-tokenize illisible : {lines[-1][:200]}")
    return ids


def first_difference(a: list[int], b: list[int]) -> int | None:
    """Rang du premier jeton qui diffère, la longueur la plus courte si l'une prolonge l'autre,
    `None` si les deux listes sont égales."""
    for k, (x, y) in enumerate(zip(a, b)):
        if x != y:
            return k
    return None if len(a) == len(b) else min(len(a), len(b))


def compare(hugging_face: list[list[int]], llama_cpp: list[list[int]], context: int = 8) -> tuple[int, Gap | None]:
    """Nombre de prompts dont les jetons diffèrent, et le premier écart (`None` sans écart)."""
    if len(hugging_face) != len(llama_cpp):
        raise ValueError(f"{len(hugging_face)} découpes Hugging Face contre {len(llama_cpp)} llama.cpp")
    count, first = 0, None
    for rank, (hf, ll) in enumerate(zip(hugging_face, llama_cpp)):
        k = first_difference(hf, ll)
        if k is None:
            continue
        count += 1
        if first is None:
            first = Gap(prompt=rank, token=k, hugging_face=hf[k:k + context], llama_cpp=ll[k:k + context])
    return count, first


def llama_tokenize(tool: Path, gguf: Path, prompt: str, work: Path) -> list[int]:
    """Jetons de `prompt` par `llama-tokenize`, lu d'un fichier pour garder l'octet près."""
    prompt_file = work / "prompt.txt"
    prompt_file.write_bytes(prompt.encode("utf-8"))
    result = subprocess.run(
        # `--no-escape` : sans lui, llama-tokenize réinterprète les `\n` et `\\` du fichier, et un
        # prompt qui porte une barre oblique inverse serait découpé sur d'autres octets que ceux de `ai/`.
        [str(tool), "-m", str(gguf), "-f", str(prompt_file), "--ids", "--no-bos", "--no-escape", "--log-disable"],
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    if result.returncode != 0:
        raise pipeline.PipelineStop(f"llama-tokenize a échoué ({result.returncode}) : {result.stderr[-2000:]}")
    return read_ids(result.stdout)


def read_ids(stdout: str) -> list[int]:
    """`parse_ids`, dont l'échec arrête la commande avec un message clair."""
    try:
        return parse_ids(stdout)
    except (ValueError, SyntaxError) as error:
        raise pipeline.PipelineStop(f"sortie de llama-tokenize illisible : {error}") from error


def read_prompts(jeu: Path | None, count: int, work: Path) -> list[str]:
    """Les `count` premiers prompts du jeu `jeu`, ou d'un jeu engendré avec la graine de
    validation par `cassini-training --format chat`."""
    if jeu is None:
        jeu = work / "validation.jsonl"
        pipeline.run(
            ["cargo", "run", "-q", "-p", "cassini-training", "--release", "--", "--format", "chat",
             "--seed", str(pipeline.VALIDATION_SEED), "--count", str(count), "--out", str(jeu)],
            cwd=ROOT,
        )
    return take(data.read_jsonl(jeu), count)


def take(examples: list[data.Example], count: int) -> list[str]:
    """Les `count` premiers prompts ; arrêt si le jeu en a moins (une parité sur moins de prompts
    que demandé ne se donnerait pas pour ce qu'elle est)."""
    if len(examples) < count:
        raise pipeline.PipelineStop(f"le jeu n'a que {len(examples)} prompts, {count} demandés")
    return [e.prompt for e in examples[:count]]


def positive(text: str) -> int:
    """Type de `--nombre` : un entier strictement positif."""
    value = int(text)
    if value <= 0:
        raise argparse.ArgumentTypeError(f"--nombre doit être positif, pas {value}")
    return value


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--gguf", type=Path, required=True, help="GGUF de la base (qwen3.5-4b-base-Q4_K_M.gguf)")
    parser.add_argument("--jeu", type=Path, help="jeu --format chat déjà engendré (sinon : graine de validation)")
    parser.add_argument("--nombre", type=positive, default=DEFAULT_COUNT, help="nombre de prompts comparés")
    args = parser.parse_args(argv)

    tool = pipeline.cache_dir() / "llama.cpp" / "build" / "bin" / "llama-tokenize"
    if not tool.exists():
        raise pipeline.PipelineStop(f"{tool} absent : lancer d'abord la chaîne (pipeline.py), qui construit llama.cpp")
    if not args.gguf.is_file():
        raise pipeline.PipelineStop(f"GGUF introuvable : {args.gguf}")

    from huggingface_hub import snapshot_download
    from transformers import AutoTokenizer

    base_dir = snapshot_download(pipeline.BASE_REPO, revision=pipeline.BASE_REVISION)
    tokenizer = AutoTokenizer.from_pretrained(base_dir)
    with tempfile.TemporaryDirectory() as folder:
        work = Path(folder)
        prompts = read_prompts(args.jeu, args.nombre, work)
        hugging_face = [tokenizer(p, add_special_tokens=False)["input_ids"] for p in prompts]
        llama_cpp = [llama_tokenize(tool, args.gguf, p, work) for p in prompts]
    count, first = compare(hugging_face, llama_cpp)
    tokens = sum(len(t) for t in hugging_face)
    pipeline.log(f"base {pipeline.BASE_REPO} @ {pipeline.BASE_REVISION}, llama.cpp @ {pipeline.LLAMA_CPP_COMMIT}")
    print(f"PARITE {len(prompts) - count}/{len(prompts)} prompts identiques ({tokens} jetons Hugging Face), "
          f"{count} écart(s)")
    if first is not None:
        print(f"premier écart : prompt {first.prompt}, jeton {first.token}")
        print(f"  Hugging Face : {first.hugging_face} {tokenizer.convert_ids_to_tokens(first.hugging_face)}")
        print(f"  llama.cpp    : {first.llama_cpp} {tokenizer.convert_ids_to_tokens(first.llama_cpp)}")
    return 1 if count else 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except pipeline.PipelineStop as stop:
        print(f"arrêt : {stop}", file=sys.stderr)
        sys.exit(2)
