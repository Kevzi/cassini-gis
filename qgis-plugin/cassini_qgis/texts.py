"""Textes du panneau, en français et en anglais (microcopy de EXPERIENCE.md).

Sans Qt. Les deux langues portent exactement les mêmes clés ; un test le vérifie.
Les descriptions des commandes viennent du registre, déjà bilingues.
"""

DEFAULT_LANG = "fr"

_FR = {
    "app_title": "Cassini",
    "settings": "Réglages",
    "settings_language": "Langue",
    "settings_model": "Modèle : choisi avec l'agent.",
    "input_hint": "Décrivez ce que vous voulez faire sur vos données.",
    "input_reason": "Disponible avec l'agent.",
    "input_name": "Demande en langage naturel",
    "send": "Envoyer",
    "expert_title": "Mode expert : lancer une commande",
    "commands_name": "Commandes du catalogue",
    "run": "Lancer",
    "stop": "Arrêter",
    "running": "En cours…",
    "done": "Terminée",
    "cancelled": "Annulée",
    "error": "Erreur",
    "result_title": "Résultat",
    "result_none": "Aucun résultat pour l'instant.",
    "details": "Détails techniques",
    "log_title": "Journal de cette session",
    "log_empty": "Rien n'a encore été lancé.",
    "about_result": "Le noyau Cassini répond et son protocole est compatible.",
    "done_generic": "La commande {command} est terminée.",
    "cancelled_generic": "La commande {command} a été annulée.",
    "no_project": "Ouvrez ou enregistrez un projet QGIS : Cassini se lance avec le projet.",
    "starting": "Démarrage du noyau…",
    "not_found": (
        "Le noyau Cassini est introuvable. Installez le paquet Cassini, ou "
        "désignez le fichier. Rien n'est téléchargé."
    ),
    "died": "Le noyau s'est arrêté de façon inattendue. Votre projet n'est pas modifié.",
    "refused": (
        "Ce noyau et cette extension ne se comprennent pas (versions "
        "incompatibles). Installez des versions assorties."
    ),
    "choose_core": "Choisir le noyau",
    "choose_core_title": "Choisir le noyau Cassini",
    "relaunch": "Relancer",
    "busy": "Une commande est déjà en cours. Attendez sa fin ou arrêtez-la.",
    "unavailable": "Indisponible : {reason}",
    "unavailable_reason": "cette commande a besoin d'un geste QGIS que cette extension n'annonce pas.",
    "not_cancellable": "Arrêt impossible : cette étape ne peut pas être interrompue.",
    "step_error": "Étape {n} en erreur ({code}) : {message}",
    "offline": "Cassini fonctionne sans connexion. Aucune donnée ne quitte votre poste.",
    "pick_command": "Choisissez une commande dans la liste.",
    "no_params": "Cette commande n'a pas de paramètre.",
    "raw_json": "{name} (JSON)",
    "optional_empty": "(non renseigné)",
    "bad_json": "Le champ « {name} » n'est pas un JSON valide. Rien n'a été envoyé.",
    "bad_number": "Le champ « {name} » doit être un nombre. Rien n'a été envoyé.",
    "live_ready": "Le noyau Cassini est prêt.",
    "live_done": "Commande terminée.",
    "live_cancelled": "Commande annulée.",
    "live_error": "Commande en erreur.",
}

_EN = {
    "app_title": "Cassini",
    "settings": "Settings",
    "settings_language": "Language",
    "settings_model": "Model: chosen with the agent.",
    "input_hint": "Describe what you want to do with your data.",
    "input_reason": "Available with the agent.",
    "input_name": "Natural language request",
    "send": "Send",
    "expert_title": "Expert mode: run a command",
    "commands_name": "Catalogue commands",
    "run": "Run",
    "stop": "Stop",
    "running": "Running…",
    "done": "Done",
    "cancelled": "Cancelled",
    "error": "Error",
    "result_title": "Result",
    "result_none": "No result yet.",
    "details": "Technical details",
    "log_title": "This session's log",
    "log_empty": "Nothing has run yet.",
    "about_result": "The Cassini core is responding and its protocol is compatible.",
    "done_generic": "The command {command} is done.",
    "cancelled_generic": "The command {command} was cancelled.",
    "no_project": "Open or save a QGIS project: Cassini starts with the project.",
    "starting": "Starting the core…",
    "not_found": (
        "The Cassini core cannot be found. Install the Cassini package, or "
        "point to the file. Nothing is downloaded."
    ),
    "died": "The core stopped unexpectedly. Your project is unchanged.",
    "refused": (
        "This core and this extension do not understand each other "
        "(incompatible versions). Install matching versions."
    ),
    "choose_core": "Choose the core",
    "choose_core_title": "Choose the Cassini core",
    "relaunch": "Restart",
    "busy": "A command is already running. Wait for it to finish or stop it.",
    "unavailable": "Unavailable: {reason}",
    "unavailable_reason": "this command needs a QGIS gesture that this extension does not announce.",
    "not_cancellable": "Stop unavailable: this step cannot be interrupted.",
    "step_error": "Step {n} failed ({code}): {message}",
    "offline": "Cassini works offline. No data leaves your computer.",
    "pick_command": "Pick a command from the list.",
    "no_params": "This command has no parameter.",
    "raw_json": "{name} (JSON)",
    "optional_empty": "(not set)",
    "bad_json": "The field \"{name}\" is not valid JSON. Nothing was sent.",
    "bad_number": "The field \"{name}\" must be a number. Nothing was sent.",
    "live_ready": "The Cassini core is ready.",
    "live_done": "Command done.",
    "live_cancelled": "Command cancelled.",
    "live_error": "Command failed.",
}

TEXTS = {"fr": _FR, "en": _EN}


def normalize(lang):
    return lang if lang in TEXTS else DEFAULT_LANG


def tr(lang, key, **values):
    """Texte `key` dans la langue `lang` (FR si la langue est inconnue)."""
    text = TEXTS[normalize(lang)][key]
    return text.format(**values) if values else text


def pick(lang, fr, en):
    """Choisit entre deux textes bilingues venus du registre."""
    return en if normalize(lang) == "en" and en else fr
