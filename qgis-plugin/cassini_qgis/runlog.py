"""Journal de la session du panneau, résultat clair et état des boutons.

Sans Qt. Le journal est celui de la session (il ne relit pas celui du noyau) et
s'efface avec le panneau. Aucun chemin absolu ni code n'est jamais produit ici
(AD-9, AD-23).
"""

import json
import os
import re

RUNNING = "running"
DONE = "done"
CANCELLED = "cancelled"
ERROR = "error"

# Icône + mot : l'état n'est jamais porté par la couleur seule.
ICONS = {RUNNING: "…", DONE: "✔", CANCELLED: "⊘", ERROR: "✖"}

_ABSOLUTE = re.compile(r"^(?:[A-Za-z]:[\\/]|\\\\|/)")
# Chemin absolu au milieu d'un texte : lettre de disque, UNC, ou « / » au début du texte ou
# après un espace, une quote ou une parenthèse ; il s'arrête à l'espace, la quote ou la fermante.
_EMBEDDED = re.compile(r"(?:(?<=[\s'\"(\[:=,;])|^)(?:[A-Za-z]:[\\/]|\\\\|/(?=[^\s/]))[^\s'\")\]]*")


class Entry:
    """Une ligne du journal : de quoi rendre, au clic, ce que l'étape a fait exactement.

    `params` : paramètres envoyés (ou ceux du plan) ; `result` : résultat rendu, ou None ;
    `step` : numéro de l'étape dans le plan, None hors plan ; `refs` : {k: résultat de
    l'étape k} du même plan, pour nommer les références `step:k` ; `error` : (code,
    message) d'une ligne en erreur, ou None ; `origin` : None pour le panneau, `"external"`
    pour un appel de l'IA externe, qui porte alors son `call_id` et, une fois fini, sa
    durée `duration_ms` rendue par le noyau.
    """

    def __init__(
        self, command, label, when, params=None, result=None, step=None, refs=None,
        origin=None, call_id=None,
    ):
        self.command = command
        self.label = label
        self.when = when
        self.status = RUNNING
        self.params = params
        self.result = result
        self.step = step
        self.refs = refs if refs is not None else {}
        self.error = None
        self.origin = origin
        self.call_id = call_id
        self.duration_ms = None


EXTERNAL = "external"


class RunLog:
    def __init__(self):
        self.entries = []

    def start(
        self, command, label, when, params=None, result=None, step=None, refs=None,
        origin=None, call_id=None,
    ):
        entry = Entry(command, label, when, params, result, step, refs, origin, call_id)
        self.entries.append(entry)
        return entry

    def current(self):
        """La dernière ligne en cours lancée par le panneau, ou None.

        Une ligne de l'IA externe n'en est jamais : elle ne se ferme que par son `call_id`.
        """
        for entry in reversed(self.entries):
            if entry.status == RUNNING and entry.origin is None:
                return entry
        return None

    def find_call(self, call_id):
        """La dernière ligne de l'appel externe `call_id`, ou None."""
        for entry in reversed(self.entries):
            if entry.origin == EXTERNAL and entry.call_id == call_id:
                return entry
        return None

    def finish_call(self, call_id, status):
        """Ferme la ligne en cours de l'appel externe `call_id` ; rend la ligne, ou None."""
        entry = self.find_call(call_id)
        if entry is not None and entry.status == RUNNING:
            entry.status = status
            return entry
        return None

    def finish(self, status):
        entry = self.current()
        if entry is not None:
            entry.status = status
        return entry


def line_text(entry, status_word, duration=None):
    """`✔ Terminée  system.about  14:02` ; `status_word` est le mot déjà traduit.

    `duration` : durée déjà mise en forme (« 41 ms »), ajoutée en fin de ligne.
    """
    line = "%s %s  %s  %s" % (ICONS[entry.status], status_word, entry.label, entry.when)
    return "%s  (%s)" % (line, duration) if duration else line


def _mask(match):
    path = match.group(0).rstrip(".,;:")
    tail = re.split(r"[\\/]", path.rstrip("\\/"))[-1]
    return "…/" + tail + match.group(0)[len(path):]


def redact(value):
    """Remplace tout chemin absolu d'un résultat par `…/nom` (AD-9)."""
    if isinstance(value, str):
        # Un chemin seul (même avec des espaces) devient `…/nom` ; une phrase qui commence par un
        # chemin (« : » ou « , » suivi d'un blanc) passe par le masquage des chemins inclus.
        if (
            _ABSOLUTE.match(value)
            and ("/" in value or "\\" in value)
            and not re.search(r"[:;,]\s", value)
        ):
            tail = re.split(r"[\\/]", value.rstrip("\\/"))[-1]
            return "…/" + tail
        return _EMBEDDED.sub(_mask, value)
    if isinstance(value, list):
        return [redact(v) for v in value]
    if isinstance(value, dict):
        return {k: redact(v) for k, v in value.items()}
    return value


def result_folder(result):
    """Dossier du fichier produit (`file`, chemin absolu rendu par le geste) s'il existe, sinon None.

    Sert au bouton « Ouvrir le dossier » ; le chemin n'est jamais affiché (AD-9).
    """
    path = result.get("file") if isinstance(result, dict) else None
    if not isinstance(path, str) or not _ABSOLUTE.match(path) or not os.path.isfile(path):
        return None
    return os.path.dirname(path)


def technical_details(result):
    return json.dumps(redact(result), ensure_ascii=False, indent=2, sort_keys=True)


def step_details(params, result):
    """Détails techniques d'une étape : paramètres et résultat bruts, chemins masqués."""
    return technical_details({"params": params, "result": result})


def controls(ready, available, running, cancellable, external_busy=False):
    """État des boutons Lancer / Arrêter pour la commande choisie.

    `external_busy` : un appel de l'IA externe est en vol ou attend l'accord ; Lancer
    est alors grisé (le noyau répondrait `session.busy`).

    Rend un dict : `run_visible`, `run_enabled`, `stop_visible`, `stop_enabled`
    et `stop_reason` (clé de texts à afficher quand Arrêter est grisé, ou None).
    """
    if running:
        return {
            "run_visible": not cancellable,
            "run_enabled": False,
            "stop_visible": True,
            "stop_enabled": bool(cancellable),
            "stop_reason": None if cancellable else "not_cancellable",
        }
    return {
        "run_visible": True,
        "run_enabled": bool(ready and available and not external_busy),
        "stop_visible": False,
        "stop_enabled": False,
        "stop_reason": None,
    }
