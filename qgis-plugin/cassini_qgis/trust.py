"""Confiance envers le programme noyau branché : empreinte SHA-256 et décision de confirmer.

Module pur : aucune dépendance à Qt ni à QGIS, aucun réseau. Seul le programme trouvé à
l'emplacement d'installation de Cassini est officiel avant lancement ; tout autre (réglage,
variable `CASSINI_CORE`, PATH) demande une confirmation, une fois par empreinte.
"""

import hashlib
import json

OFFICIAL_ORIGIN = "emplacement d'installation"  # origine rendue par locator.locate
OFFICIAL_NAME = "cassini"  # `program_name` du hello d'un programme officiel
ACCEPTED_KEY = "cassini/accepted_cores"  # empreintes acceptées, liste JSON dans QSettings
_CHUNK = 1 << 20


def fingerprint(path):
    """SHA-256 (hexadécimal) du fichier, ou None s'il est illisible."""
    try:
        digest = hashlib.sha256()
        with open(path, "rb") as handle:
            for chunk in iter(lambda: handle.read(_CHUNK), b""):
                digest.update(chunk)
        return digest.hexdigest()
    except Exception:
        return None


def parse_accepted(raw):
    """Liste des empreintes retenues, depuis la valeur stockée (JSON) ; tolérante."""
    if isinstance(raw, (list, tuple)):
        data = raw
    else:
        try:
            data = json.loads(raw) if isinstance(raw, str) and raw else []
        except Exception:
            data = []
    return [d for d in data if isinstance(d, str) and d] if isinstance(data, list) else []


def dump_accepted(accepted):
    return json.dumps(sorted(set(accepted)))


def remember(accepted, digest):
    """Nouvelle liste d'empreintes acceptées, `digest` ajouté."""
    return sorted(set(accepted) | {digest})


def is_official(origin):
    return origin == OFFICIAL_ORIGIN


def decide(path, origin, accepted):
    """Décision avant lancement : `(action, empreinte)`.

    action : "launch" (aucune question), "confirm" (demander), "refuse" (empreinte incalculable).
    Le programme officiel choisi à la main arrive avec l'origine « réglage » : il demande aussi.
    """
    if is_official(origin):
        return "launch", None
    digest = fingerprint(path)
    if digest is None:
        return "refuse", None
    if digest in accepted:
        return "launch", digest
    return "confirm", digest


def needs_confirmation(path, origin, accepted):
    """Vrai quand le programme doit être confirmé avant d'être lancé (illisible : refus)."""
    return decide(path, origin, accepted)[0] != "launch"


def is_derived(program_name):
    """Programme dérivé : le nom annoncé par `session.hello` n'est pas celui de Cassini."""
    return isinstance(program_name, str) and bool(program_name) and program_name.lower() != OFFICIAL_NAME  # comme le noyau
