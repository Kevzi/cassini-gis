"""Choix du modèle dans les Réglages (ticket 5.14). Sans Qt, testé seul.

Le noyau seul découvre les modèles et les rend par `agent.models` : des noms de
fichiers et des tailles, jamais un dossier ni un chemin, et un `problem` facultatif
pour un modèle qui ne peut pas servir. Le panneau n'en garde qu'un nom, qu'il
mémorise et renvoie tel quel avec chaque `plan.propose`.
"""

try:  # importé à plat par les tests, en paquet par QGIS
    from . import texts
except ImportError:  # pragma: no cover
    import texts

SETTING_KEY = "cassini/model"


def clean(listed):
    """Entrées lisibles de la liste reçue : `{"file", "size_bytes", "problem"}`.

    Une entrée sans nom de fichier texte est écartée ; une taille illisible vaut None,
    un problème vide ou illisible n'en est pas un.
    """
    entries = []
    if not isinstance(listed, list):
        return entries
    for item in listed:
        if not isinstance(item, dict):
            continue
        name = item.get("file")
        if not isinstance(name, str) or not name:
            continue
        size = item.get("size_bytes")
        if isinstance(size, bool) or not isinstance(size, int) or size < 0:
            size = None
        problem = item.get("problem")
        if not isinstance(problem, str) or not problem.strip():
            problem = None
        entries.append({"file": name, "size_bytes": size, "problem": problem})
    return entries


def usable(entries):
    """Noms des modèles sans problème, dans l'ordre de la liste."""
    return [e["file"] for e in entries if e["problem"] is None]


def choose(listed, remembered):
    """(choix, oublier) : le modèle à envoyer, et s'il faut oublier le choix mémorisé.

    Le choix mémorisé est gardé s'il figure dans la liste sans problème (comparé tel
    quel) ; sinon il est oublié et le premier modèle sans problème est pris (None si
    aucun).
    """
    names = usable(clean(listed))
    remembered = remembered if isinstance(remembered, str) and remembered else None
    if remembered is not None and remembered in names:
        return remembered, False
    return (names[0] if names else None), remembered is not None


# Codes d'échec d'une proposition qui disent que la liste a changé depuis qu'elle a été
# reçue : le modèle envoyé n'y est plus, ou ne peut plus servir.
STALE_LIST_CODES = ("models.unknown", "models.unusable")


def asks_list_again(code):
    """Un échec de proposition de code `code` demande-t-il de redemander la liste ?"""
    return code in STALE_LIST_CODES


def received(listed, remembered):
    """Décision à la réception de la liste (MODELS) : (entrées, choix, oublier).

    `entrées` : la liste nettoyée, à montrer ; `choix` : le modèle à envoyer (None si
    aucun) ; `oublier` : vrai si le choix mémorisé (`SETTING_KEY`) doit être effacé,
    parce qu'il n'est plus dans la liste sans problème.
    """
    entries = clean(listed)
    chosen, forget = choose(entries, remembered)
    return entries, chosen, forget


def size_text(lang, size_bytes):
    """Taille lisible, en unités décimales : « 5,6 Go » en français, « 5.6 GB » en anglais."""
    if size_bytes is None:
        return None
    for limit, key in ((10**9, "models_size_gb"), (10**6, "models_size_mb"), (10**3, "models_size_kb")):
        if size_bytes >= limit:
            value = "%.1f" % (size_bytes / limit)
            break
    else:
        value, key = str(size_bytes), "models_size_b"
    if texts.normalize(lang) == "fr":
        value = value.replace(".", ",")
    return texts.tr(lang, key, n=value)


def label(lang, entry):
    """Libellé d'une entrée : nom et taille, et la raison si le modèle ne peut pas servir."""
    size_bytes = entry.get("size_bytes")
    if entry.get("problem") and size_bytes == 0:
        size_bytes = None  # une déclaration cassée n'a pas de taille à montrer
    size = size_text(lang, size_bytes)
    text = texts.tr(lang, "models_entry", name=entry["file"], size=size) if size else entry["file"]
    if entry.get("problem"):
        text = texts.tr(lang, "models_entry_problem", label=text, problem=entry["problem"])
    return text


def reason(lang, entries, error):
    """Pourquoi la liste est grisée, ou None si un modèle peut être choisi.

    `error` : (code, message) du dernier échec de la liste, ou None ; `entries` : la
    dernière liste reçue (nettoyée), ou None si aucune n'est encore arrivée.
    """
    if error is not None:
        code, message = error
        return texts.tr(lang, "models_error", code=code, message=message)
    if entries is None:
        return None
    if not usable(entries):
        return texts.tr(lang, "models_none")
    return None
