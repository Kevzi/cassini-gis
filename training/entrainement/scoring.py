"""Notation exacte d'un plan rendu par le modèle (ticket 5.10, tâche 2).

Règle de `ai/tests/ecart_reel.rs`, avec une exigence de plus : un plan est juste s'il est
identique au plan attendu, sauf le `title` de `layout.export_pdf`, dont la présence (non
vide) suffit ; les nombres se comparent en valeur (`500.0` vaut `500`). Une question attendue
doit rendre une question, un refus attendu un refus ; ici, en plus d'`ecart_reel.rs`, le
texte de la question ou du refus doit être non vide.

Aucune dépendance lourde : ce module se teste en CI.
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from typing import Any

END_OF_ANSWER = "<|im_end|>"
KINDS = ("calls", "question", "refusal")
# Le champ texte de chaque sorte de plan.
TEXT_FIELD = {"question": "question", "refusal": "reason"}


def read_plan(text: str) -> dict[str, Any] | None:
    """Lit la sortie du modèle comme un plan : un objet JSON seul (espaces et marque de fin
    de tour tolérés autour), de l'une des trois sortes, sans clé de plus. `None` sinon."""
    text = text.strip()
    if text.endswith(END_OF_ANSWER):
        text = text[: -len(END_OF_ANSWER)].rstrip()
    try:
        plan = json.loads(text)
    except json.JSONDecodeError:
        return None
    if not isinstance(plan, dict):
        return None
    kind = plan.get("kind")
    if kind == "calls":
        calls = plan.get("calls")
        if set(plan) != {"kind", "calls"} or not isinstance(calls, list) or not calls:
            return None
        for call in calls:
            if (
                not isinstance(call, dict)
                or set(call) != {"command", "params"}
                or not isinstance(call["command"], str)
                or not isinstance(call["params"], dict)
            ):
                return None
        return plan
    if kind in TEXT_FIELD:
        key = TEXT_FIELD[kind]
        if set(plan) != {"kind", key} or not isinstance(plan[key], str):
            return None
        return plan
    return None


def _normalize_numbers(value: Any) -> Any:
    """Un flottant entier devient l'entier ; les booléens restent des booléens."""
    if isinstance(value, bool):
        return value
    if isinstance(value, float) and value.is_integer() and abs(value) < 9.0e15:
        return int(value)
    if isinstance(value, list):
        return [_normalize_numbers(v) for v in value]
    if isinstance(value, dict):
        return {k: _normalize_numbers(v) for k, v in value.items()}
    return value


def _without_titles(calls: list[dict[str, Any]]) -> list[dict[str, Any]] | None:
    """Ôte le titre de chaque `layout.export_pdf` ; `None` si un titre manque ou est vide."""
    copy = json.loads(json.dumps(calls))
    for call in copy:
        if call["command"] == "layout.export_pdf":
            title = call["params"].pop("title", None)
            if not isinstance(title, str) or not title.strip():
                return None
    return copy


def _identical(a: Any, b: Any) -> bool:
    """Égalité stricte : `True` ne vaut pas `1`, une chaîne ne vaut pas un nombre."""
    if type(a) is not type(b):
        return False
    if isinstance(a, dict):
        return a.keys() == b.keys() and all(_identical(a[k], b[k]) for k in a)
    if isinstance(a, list):
        return len(a) == len(b) and all(_identical(x, y) for x, y in zip(a, b))
    return a == b


def plan_matches(output: str, expected: str) -> bool:
    """Vrai si la sortie du modèle vaut le plan attendu, selon la règle du module."""
    wanted = read_plan(expected)
    if wanted is None:
        raise ValueError(f"plan attendu illisible : {expected!r}")
    got = read_plan(output)
    if got is None or got["kind"] != wanted["kind"]:
        return False
    if wanted["kind"] in TEXT_FIELD:
        return bool(got[TEXT_FIELD[wanted["kind"]]].strip())
    wanted_calls, got_calls = _without_titles(wanted["calls"]), _without_titles(got["calls"])
    if wanted_calls is None or got_calls is None:
        return False
    return _identical(_normalize_numbers(wanted_calls), _normalize_numbers(got_calls))


@dataclass
class Score:
    """Décompte par sorte de plan attendu : justes et total."""

    correct: dict[str, int] = field(default_factory=lambda: {k: 0 for k in KINDS})
    total: dict[str, int] = field(default_factory=lambda: {k: 0 for k in KINDS})
    unreadable: int = 0

    def record(self, output: str, expected: str) -> bool:
        kind = read_plan(expected)["kind"]  # type: ignore[index]
        match = plan_matches(output, expected)
        self.total[kind] += 1
        self.correct[kind] += int(match)
        if read_plan(output) is None:
            self.unreadable += 1
        return match

    def to_dict(self) -> dict[str, Any]:
        correct, total = sum(self.correct.values()), sum(self.total.values())
        return {
            "justes": correct,
            "total": total,
            "taux": round(correct / total, 4) if total else None,
            "par_sorte": {k: {"justes": self.correct[k], "total": self.total[k]} for k in KINDS},
            "sorties_illisibles": self.unreadable,
        }
