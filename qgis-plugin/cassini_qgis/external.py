"""État affiché du canal d'IA externe (AD-29, flux 4), sans Qt.

Le panneau ne décide rien (AD-4) : cet état reflète ce que le noyau a répondu
(`session.external`) et ce qu'il annonce (`external.call`, `external.approve`,
retrait par `request.cancel`). L'interrupteur n'est jamais rallumé ici ; il s'éteint
quand le noyau meurt, est relancé ou arrêté. Aucun chemin absolu n'est affiché
(AD-9) : les paramètres passent par `runlog.redact`.
"""

import json

try:  # importé à plat par les tests, en paquet par QGIS
    from . import agentview, runlog, texts
except ImportError:  # pragma: no cover
    import agentview
    import runlog
    import texts

CHANNEL_TAKEN = "external.channel_taken"
CANCELLED_CODE = "registry.cancelled"  # appel arrêté (Arrêter, ou Couper pendant l'appel)


class ExternalState:
    def __init__(self, ask=False):
        self.ask = bool(ask)  # case « Me demander avant chaque appel », retenue par le panneau
        self.taken = False  # une autre session QGIS tient le canal : interrupteur grisé
        self.error = None  # (code, message) du dernier refus autre que le canal pris
        self.enabled = False
        self.calls = {}  # call_id -> ligne du journal en cours
        self.approval = None  # {"request_id", "call_id", "command", "params"} ou None

    # -- réponses du noyau ----------------------------------------------------
    def switched(self, enabled):
        """Réponse de `session.external` : l'état affiché est celui du noyau."""
        self.enabled = bool(enabled)
        self.error = None
        self.taken = False

    def refused(self, code, message):
        """`session.external` refusé : raison affichée.

        `enabled` ne bouge pas : il reste ce que le noyau a répondu en dernier (un Couper
        refusé laisse le canal ouvert, et l'interrupteur le montre).
        """
        if code == CHANNEL_TAKEN:
            self.taken = True
            self.error = None
        else:
            self.error = (code, message)

    def settings_opened(self):
        """Ouverture des Réglages : l'interrupteur grisé par un canal pris redevient cliquable."""
        self.taken = False
        self.error = None

    # -- appels de l'IA externe -------------------------------------------------
    def call_started(self, log, call_id, command, params, label, when):
        entry = log.start(
            command or "?", label, when, params=params, origin=runlog.EXTERNAL, call_id=call_id
        )
        self.calls[call_id] = entry
        return entry

    def call_finished(self, log, call_id, command, result, error, duration_ms, label, when):
        """Ferme la ligne de l'appel ; un `finished` sans `started` crée une ligne close."""
        if error is None:
            status = runlog.DONE
        elif error.get("code") == CANCELLED_CODE:
            status = runlog.CANCELLED
        else:
            status = runlog.ERROR
        self.calls.pop(call_id, None)
        entry = log.finish_call(call_id, status)
        if entry is None:
            entry = log.start(
                command or "?", label, when, origin=runlog.EXTERNAL, call_id=call_id
            )
            entry.status = status
        if status == runlog.DONE:
            entry.result = result
        elif status == runlog.ERROR:
            entry.error = (error.get("code", ""), error.get("message", ""))
        entry.duration_ms = duration_ms
        return entry

    def running_call(self):
        """L'appel externe en vol le plus récent : (call_id, ligne), ou None."""
        if not self.calls:
            return None
        call_id = next(reversed(list(self.calls)))
        return call_id, self.calls[call_id]

    # -- question d'accord ------------------------------------------------------
    def approval_asked(self, request_id, call_id, command, params):
        self.approval = {
            "request_id": request_id,
            "call_id": call_id,
            "command": command,
            "params": params,
        }

    def approval_withdrawn(self, request_id):
        if self.approval is not None and self.approval["request_id"] == request_id:
            self.approval = None

    def answered(self):
        self.approval = None

    # -- noyau parti ------------------------------------------------------------
    def lost(self, log):
        """Noyau mort, relancé ou arrêté : interrupteur éteint, question fermée,
        lignes externes en cours closes annulées."""
        self.enabled = False
        self.approval = None
        self.error = None
        self.calls = {}
        for entry in log.entries:
            if entry.origin == runlog.EXTERNAL and entry.status == runlog.RUNNING:
                entry.status = runlog.CANCELLED


def ask_setting(value):
    """Case « Me demander avant chaque appel » lue dans QSettings (texte, booléen ou entier)."""
    return str(value).lower() in ("true", "1")


def apply(state, log, kind, payload, catalogue, lang, when):
    """Applique un événement externe du client à `state` et au journal `log`.

    Rend un dict : `announce` (texte pour la zone vivante, ou None), `where` (zone
    annoncée : "external", "log" ou "approval") et `resend_ask` (vrai si le noyau a
    allumé le canal avec une case différente de celle que l'utilisateur a cochée
    entre-temps : le panneau renvoie `session.external {enabled: true, ask}`).
    """
    # `kind` : les constantes EXTERNAL_* / APPROVAL_* de `client` (mêmes chaînes, sans l'importer).
    payload = payload if isinstance(payload, dict) else {}
    out = {"announce": None, "where": "external", "resend_ask": False}
    if kind == "external_state":
        was = state.enabled
        state.switched(payload.get("enabled"))
        if state.enabled and bool(payload.get("ask")) != state.ask:
            out["resend_ask"] = True
        if state.enabled != was:
            live = "live_external_on" if state.enabled else "live_external_off"
            out["announce"] = texts.tr(lang, live)
    elif kind == "external_failed":
        state.refused(payload.get("code", ""), payload.get("message", ""))
    elif kind == "external_call_started":
        command = payload.get("command")
        state.call_started(
            log, payload.get("call_id"), command, payload.get("params"),
            label(command, catalogue, lang), when,
        )
    elif kind == "external_call_finished":
        command = payload.get("command")
        entry = state.call_finished(
            log, payload.get("call_id"), command, payload.get("result"), payload.get("error"),
            payload.get("duration_ms"), label(command, catalogue, lang), when,
        )
        word = texts.tr(lang, entry.status).rstrip("….").strip()
        out["announce"] = texts.tr(lang, "focus_status", label=entry.label, status=word)
        out["where"] = "log"
    elif kind == "approval_asked":
        state.approval_asked(
            payload.get("request_id"), payload.get("call_id"), payload.get("command"),
            payload.get("params"),
        )
        out["announce"] = texts.tr(lang, "live_approval")
        out["where"] = "approval"
    elif kind == "approval_withdrawn":
        state.approval_withdrawn(payload.get("request_id"))
    return out


def busy(state):
    """Un appel externe est en vol ou une question attend : le panneau ne lance rien."""
    return bool(state.calls) or state.approval is not None


def label(command, catalogue, lang):
    """`IA externe · Décrit une couche.` : la ligne est marquée comme venant de l'IA externe."""
    return "%s · %s" % (texts.tr(lang, "external_tag"), agentview.describe(command, catalogue, lang))


def duration_text(duration_ms, lang):
    """`41 ms`, `1,2 s` (`1.2 s` en anglais), ou None sans durée."""
    if duration_ms is None:
        return None
    if duration_ms < 1000:
        return texts.tr(lang, "duration_ms", n=duration_ms)
    seconds = "%.1f" % (duration_ms / 1000.0)
    if texts.normalize(lang) == "fr":
        seconds = seconds.replace(".", ",")
    return texts.tr(lang, "duration_s", n=seconds)


def question_text(state, catalogue, lang):
    """Texte de la question d'accord : la commande décrite et ses paramètres, chemins masqués."""
    approval = state.approval
    if approval is None:
        return None
    params = json.dumps(
        runlog.redact(approval.get("params")), ensure_ascii=False, sort_keys=True
    )
    return texts.tr(
        lang,
        "approval_question",
        description=agentview.describe(approval.get("command"), catalogue, lang),
        params=params,
    )


def controls(state, ready, catalogue):
    """État des contrôles de l'IA externe.

    Rend un dict : `switch_checked`, `switch_enabled`, `switch_reason` (clé de texts
    ou None), `switch_error` ((code, message) ou None),
    `ask_checked`, `banner_visible`, `cut_enabled`, `stop_visible`, `stop_enabled`,
    `stop_reason` (clé de texts ou None), `stop_call` (call_id visé ou None),
    `question_visible`, `answer_enabled`.
    """
    running = state.running_call()
    call_id = running[0] if running else None
    command = running[1].command if running else None
    cancellable = command is not None and agentview.is_cancellable(command, catalogue)
    return {
        "switch_checked": bool(state.enabled),
        "switch_enabled": bool(ready and not state.taken),
        "switch_reason": "external_taken" if state.taken else None,
        "switch_error": state.error,
        "ask_checked": bool(state.ask),
        "banner_visible": bool(state.enabled),
        "cut_enabled": bool(ready and state.enabled),
        "stop_visible": running is not None,
        "stop_enabled": bool(ready and running is not None and cancellable),
        "stop_reason": "not_cancellable" if running is not None and not cancellable else None,
        "stop_call": call_id,
        "question_visible": state.approval is not None,
        "answer_enabled": bool(ready and state.approval is not None),
    }
