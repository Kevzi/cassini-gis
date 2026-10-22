"""Lecture du jeu de prompts écrit par `cassini-training --format chat` (ticket 5.10, tâche 2).

Python ne met rien en forme : il lit `prompt` et `completion` tels que le crate `training`
les a écrits (module `prompt`), seul endroit qui décide du texte vu par le modèle.

Aucune dépendance lourde ici (ni torch, ni transformers) : ce module se teste en CI.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Callable, Iterable

# Fin de la réponse attendue, écrite par `training/src/prompt.rs` (`COMPLETION_END`).
END_OF_ANSWER = "<|im_end|>"


@dataclass(frozen=True)
class Example:
    """Un exemple : le texte exact vu par le modèle et la réponse attendue."""

    prompt: str
    completion: str


class UnreadableDataset(ValueError):
    """Une ligne du fichier n'est pas un exemple `{prompt, completion}` bien formé."""


def read_jsonl(path: Path) -> list[Example]:
    """Lit le fichier JSONL, une ligne par exemple. Refuse toute ligne mal formée."""
    examples = []
    with open(path, encoding="utf-8") as file:
        for number, line in enumerate(file, start=1):
            if not line.strip():
                continue
            examples.append(read_line(line, number))
    return examples


def read_line(line: str, number: int = 1) -> Example:
    """Lit une ligne `{prompt, completion}` ; deux chaînes non vides, rien d'autre."""
    try:
        record = json.loads(line)
    except json.JSONDecodeError as error:
        raise UnreadableDataset(f"ligne {number} : JSON illisible ({error})") from error
    if not isinstance(record, dict) or set(record) != {"prompt", "completion"}:
        raise UnreadableDataset(f"ligne {number} : attendu exactement {{prompt, completion}}")
    prompt, completion = record["prompt"], record["completion"]
    if not isinstance(prompt, str) or not isinstance(completion, str) or not prompt:
        raise UnreadableDataset(f"ligne {number} : prompt et completion doivent être du texte")
    if not completion.endswith(END_OF_ANSWER) or completion == END_OF_ANSWER:
        raise UnreadableDataset(f"ligne {number} : la réponse doit finir par {END_OF_ANSWER}")
    return Example(prompt=prompt, completion=completion)


def drop_too_long(
    examples: Iterable[Example],
    length: Callable[[Example], int],
    max_length: int,
) -> tuple[list[Example], int]:
    """Garde les exemples dont la longueur (en jetons, prompt et réponse) tient dans
    `max_length`. Un exemple trop long est écarté et compté, jamais tronqué : une réponse
    coupée apprendrait au modèle un plan incomplet."""
    kept, dropped = [], 0
    for example in examples:
        if length(example) > max_length:
            dropped += 1
        else:
            kept.append(example)
    return kept, dropped


def drop_overlap(
    validation: Iterable[Example], training: Iterable[Example]
) -> tuple[list[Example], int]:
    """Retire de la validation les exemples dont le prompt figure à l'identique dans le jeu
    d'entraînement : le score doit mesurer une généralisation, pas une récitation."""
    seen = {e.prompt for e in training}
    kept, removed = [], 0
    for example in validation:
        if example.prompt in seen:
            removed += 1
        else:
            kept.append(example)
    return kept, removed


def expected_answer(example: Example) -> str:
    """Le plan JSON attendu, sans la marque de fin de tour."""
    return example.completion[: -len(END_OF_ANSWER)]
