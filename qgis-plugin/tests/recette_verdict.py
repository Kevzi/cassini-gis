"""Jugement de la recette QGIS en phrases enchaînées (ticket 5.45), sans QGIS ni noyau.

Une phrase compte juste si son plan est conforme au plan attendu, écrit à la main, ET s'exécute ;
pour une phrase ambiguë, si le modèle pose une question (décision du propriétaire, 2026-11-10).
La conformité est celle de `ai/tests/plan_reel.rs` : les appels du plan montré par le noyau
(plan complété, 11.22) égalent les appels attendus comme deux `serde_json::Value` : ordre des
clés indifférent, ordre des appels compté, un entier n'égale pas un flottant ni un booléen.

Fichier de phrases : une liste JSON de `{id, text, expect, note?}`, où `expect` vaut
`{"plan": [appels]}` ou `{"question": true}`. Une couche s'y écrit `@<nom de la couche>` :
remplacée, à la lecture, par l'identifiant de la couche de ce nom dans le projet. Le même fichier
sert donc à tout projet qui porte ces noms de couches (ticket 5.50 : un projet de 60 couches).

Bibliothèque standard seule : importé par la sonde sous le Python de QGIS et par les tests.
"""

import json

BAR = 0.8  # barre officielle : 80 % des phrases (16 sur 20)

RIGHT = None  # verdict d'une phrase juste


class RecetteError(Exception):
    """Fichier de phrases mal formé, ou couche nommée absente du projet."""


def same_json(a, b):
    """Égalité de deux valeurs JSON, au sens de `serde_json::Value` : 1 ≠ 1.0, True ≠ 1."""
    if isinstance(a, bool) or isinstance(b, bool):
        return type(a) is type(b) and a == b
    if isinstance(a, (int, float)) or isinstance(b, (int, float)):
        return type(a) is type(b) and a == b
    if isinstance(a, dict) and isinstance(b, dict):
        return a.keys() == b.keys() and all(same_json(a[k], b[k]) for k in a)
    if isinstance(a, list) and isinstance(b, list):
        return len(a) == len(b) and all(same_json(x, y) for x, y in zip(a, b))
    return type(a) is type(b) and a == b


def _resolve(value, layers, where):
    if isinstance(value, str) and value.startswith("@"):
        name = value[1:]
        ids = layers.get(name)
        if not ids:
            raise RecetteError("%s : aucune couche « %s » dans le projet" % (where, name))
        if len(ids) > 1:
            raise RecetteError("%s : plusieurs couches « %s » dans le projet" % (where, name))
        return ids[0]
    if isinstance(value, dict):
        return {k: _resolve(v, layers, where) for k, v in value.items()}
    if isinstance(value, list):
        return [_resolve(v, layers, where) for v in value]
    return value


def layer_index(pairs):
    """`{nom: [identifiants]}` depuis les paires `(identifiant, nom)` des couches du projet."""
    index = {}
    for layer_id, name in pairs:
        index.setdefault(name, []).append(layer_id)
    return index


def load(text, layers):
    """Les phrases du fichier, couches `@nom` résolues par `layers` (`{nom: [identifiants]}`).

    Lève `RecetteError` sur un fichier mal formé ou une couche absente ou en double.
    """
    try:
        phrases = json.loads(text)
    except ValueError as error:
        raise RecetteError("fichier de phrases illisible : %s" % error)
    if not isinstance(phrases, list) or not phrases:
        raise RecetteError("le fichier de phrases doit être une liste non vide")
    seen, out = set(), []
    for i, phrase in enumerate(phrases, 1):
        where = "phrase %d" % i
        if not isinstance(phrase, dict):
            raise RecetteError("%s : un objet est attendu" % where)
        pid, text_, expect = phrase.get("id"), phrase.get("text"), phrase.get("expect")
        if not isinstance(pid, str) or not pid or pid in seen:
            raise RecetteError("%s : identifiant absent ou en double" % where)
        seen.add(pid)
        where = pid
        if not isinstance(text_, str) or not text_.strip():
            raise RecetteError("%s : texte absent" % where)
        if expect == {"question": True}:
            plan = None
        elif isinstance(expect, dict) and set(expect) == {"plan"} and isinstance(expect["plan"], list) \
                and expect["plan"] and all(isinstance(c, dict) and set(c) == {"command", "params"}
                                           for c in expect["plan"]):
            plan = _resolve(expect["plan"], layers, where)
        else:
            raise RecetteError("%s : « expect » vaut {\"plan\": [appels]} ou {\"question\": true}" % where)
        out.append({"id": pid, "text": text_, "plan": plan, "note": phrase.get("note")})
    return out


def wants_question(phrase):
    return phrase["plan"] is None


def conforms(phrase, plan):
    """Vrai si le plan montré (`{"kind": "calls", "calls": [...]}`) égale le plan attendu."""
    if wants_question(phrase) or not isinstance(plan, dict):
        return False
    calls = plan.get("calls")
    return isinstance(calls, list) and same_json(calls, phrase["plan"])


def on_proposed(phrase, plan):
    """Verdict provisoire d'un plan proposé : `RIGHT` s'il faut l'exécuter, sinon la raison."""
    if wants_question(phrase):
        return "plan rendu au lieu d'une question"
    if not conforms(phrase, plan):
        return "plan non conforme"
    return RIGHT


def on_question(phrase):
    return RIGHT if wants_question(phrase) else "question sur une phrase claire"


def on_failed(payload):
    return "refus : %s" % _code(payload)


def on_exec_failed(payload):
    step = payload.get("step") if isinstance(payload, dict) else None
    return "exécution en échec à l'étape %s : %s" % (step, _code(payload))


def _code(payload):
    return payload.get("code", "?") if isinstance(payload, dict) else "?"


def bar(total):
    """Nombre de phrases justes exigé : 80 % arrondi au-dessus (16 sur 20, 4 sur 5)."""
    return -(-int(BAR * 100) * total // 100)


def score_line(right, total):
    need = bar(total)
    return "SCORE : %d/%d (barre %d) : %s" % (
        right, total, need, "barre tenue" if right >= need else "SOUS LA BARRE")
