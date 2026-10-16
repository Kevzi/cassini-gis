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


class Entry:
    def __init__(self, command, label, when):
        self.command = command
        self.label = label
        self.when = when
        self.status = RUNNING


class RunLog:
    def __init__(self):
        self.entries = []

    def start(self, command, label, when):
        entry = Entry(command, label, when)
        self.entries.append(entry)
        return entry

    def current(self):
        """La dernière ligne en cours, ou None."""
        for entry in reversed(self.entries):
            if entry.status == RUNNING:
                return entry
        return None

    def finish(self, status):
        entry = self.current()
        if entry is not None:
            entry.status = status
        return entry


def line_text(entry, status_word):
    """`✔ Terminée  system.about  14:02` ; `status_word` est le mot déjà traduit."""
    return "%s %s  %s  %s" % (ICONS[entry.status], status_word, entry.label, entry.when)


def redact(value):
    """Remplace tout chemin absolu d'un résultat par `…/nom` (AD-9)."""
    if isinstance(value, str):
        if _ABSOLUTE.match(value) and ("/" in value or "\\" in value):
            tail = re.split(r"[\\/]", value.rstrip("\\/"))[-1]
            return "…/" + tail
        return value
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


def controls(ready, available, running, cancellable):
    """État des boutons Lancer / Arrêter pour la commande choisie.

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
        "run_enabled": bool(ready and available),
        "stop_visible": False,
        "stop_enabled": False,
        "stop_reason": None,
    }
