"""État affiché du plan de l'agent, libellés et activations du panneau [Epic 5].

Sans Qt. Le panneau ne décide rien (AD-4, AD-5) : cet état ne fait que refléter
les événements du client (plan proposé, étape annoncée, verdict). Il n'édite
aucune étape et ne garde du plan que ce qu'il faut pour l'afficher et renvoyer
son `plan_id`. Aucun chemin absolu ni code n'est produit ici (AD-9, AD-23).
"""

try:  # importé à plat par les tests, en paquet par QGIS
    from . import client, runlog, texts
except ImportError:  # pragma: no cover
    import client
    import runlog
    import texts

# Phases du dialogue avec l'agent.
IDLE = "idle"
GENERATING = "generating"
PROPOSED = "proposed"
EXECUTING = "executing"

# Statuts d'étape : ceux du journal, plus « en attente ».
PENDING = "pending"
RUNNING = runlog.RUNNING
DONE = runlog.DONE
CANCELLED = runlog.CANCELLED
ERROR = runlog.ERROR

# Icône + mot : l'état n'est jamais porté par la couleur seule.
ICONS = dict(runlog.ICONS, **{PENDING: "○"})
MARKER = "▶"  # étape en cours


class Step:
    def __init__(self, number, command, params):
        self.number = number
        self.command = command
        self.params = params
        self.status = PENDING
        self.result = None  # résultat rendu par l'étape, une fois finie


class AgentView:
    def __init__(self):
        self.reset()

    def reset(self):
        self.phase = IDLE
        self.plan_id = None
        self.plan = None
        self.steps = []
        self.current = None  # numéro de l'étape en cours
        self.notice = None  # (clé de texts, valeurs) ou None
        self.rejecting = False
        # Vrai après une question du garde-fou : la phrase d'origine revient à la saisie.
        self.restore_request = False
        # Couches de résultat hors de la vue du modèle (ticket 5.33), [{"layer_id", "name"}].
        self.left_out = []

    # -- événements -----------------------------------------------------
    def generating(self):
        self.reset()
        self.phase = GENERATING
        self.notice = ("generating", {})

    def proposed(self, plan_id, plan, left_out=None):
        self.reset()
        self.left_out = client.left_out_of({"left_out": left_out})
        calls = plan.get("calls") if isinstance(plan, dict) else None
        calls = calls if isinstance(calls, list) else []
        for number, call in enumerate(calls, start=1):
            call = call if isinstance(call, dict) else {}
            command = call.get("command")
            self.steps.append(
                Step(number, command if isinstance(command, str) else "?", call.get("params"))
            )
        self.phase = PROPOSED
        self.plan_id = plan_id
        self.plan = plan

    def question(self, text, source="model", left_out=None):
        """La question est une réplique : rien n'attend, la saisie est rendue.

        Une question du garde-fou (`source` « guard » : un point du plan ne venait pas de la
        demande) demande de compléter la phrase : le panneau la remet dans la saisie.
        """
        self.reset()
        self.left_out = client.left_out_of({"left_out": left_out})
        self.notice = ("question", {"text": text})
        self.restore_request = source == "guard"

    def failed(self, code, message):
        """Échec de proposition (ou de refus) : le plan est oublié."""
        self.reset()
        self.notice = ("propose_error", {"code": code, "message": message})

    def executing(self):
        self.phase = EXECUTING
        self.rejecting = False

    def start_rejecting(self):
        self.rejecting = True

    def rejected(self):
        self.reset()
        self.notice = ("plan_rejected", {})

    def step(self, number, command=None):
        """Annonce `plan.step` : l'étape précédente est terminée, celle-ci part."""
        previous = self.find(self.current)
        if previous is not None and previous.status == RUNNING:
            previous.status = DONE
        target = self.find(number)
        if target is not None:
            target.status = RUNNING
        self.current = number

    def done(self, results):
        reported = set()
        for item in results if isinstance(results, list) else []:
            number = item.get("step") if isinstance(item, dict) else None
            if isinstance(number, int) and not isinstance(number, bool):
                reported.add(number)
                target = self.find(number)
                if target is not None and "result" in item:
                    target.result = item.get("result")
        for s in self.steps:
            if s.status == RUNNING or s.number in reported:
                s.status = DONE
        self._close()

    def exec_failed(self, step=None, outputs=None):
        """L'étape fautive passe en Erreur ; les suivantes restent non marquées.

        `outputs` : {étape: résultat} des ordres QGIS déjà soldés par l'extension.
        """
        self._keep_outputs(outputs, step)
        self._end_step(step, ERROR)

    def cancelled(self, step=None, outputs=None):
        self._keep_outputs(outputs, step)
        self._end_step(step, CANCELLED)

    def step_results(self):
        """{k: résultat de l'étape k}, pour nommer les références `step:k`."""
        return {s.number: s.result for s in self.steps if s.result is not None}

    def forget(self):
        """Le noyau n'est plus là : le plan est oublié, l'étape en cours en Erreur."""
        if self.phase == EXECUTING:
            self._end_step(None, ERROR)
        elif self.phase in (GENERATING, PROPOSED):
            self.reset()

    # -- interne ----------------------------------------------------------
    def _keep_outputs(self, outputs, failed):
        """Range le résultat des étapes finies ; l'étape fautive n'en reçoit pas."""
        if failed is None:
            failed = self.current
        for number, result in (outputs if isinstance(outputs, dict) else {}).items():
            target = self.find(number)
            if target is not None and number != failed:
                target.result = result

    def find(self, number):
        for s in self.steps:
            if s.number == number:
                return s
        return None

    def _end_step(self, step, status):
        target = self.find(step if step is not None else self.current)
        if target is not None:
            target.status = status
        # Une étape nommée autre que celle en cours (référence `step:k` non résolue, annonce
        # impossible) : le noyau tranche avant d'annoncer l'étape k, l'étape en cours est finie.
        others = DONE if step is not None and step != self.current else status
        for s in self.steps:  # aucune autre étape ne reste « en cours »
            if s is not target and s.status == RUNNING:
                s.status = others
        self._close()

    def _close(self):
        self.phase = IDLE
        self.plan_id = None
        self.current = None
        self.rejecting = False

    def current_command(self):
        target = self.find(self.current)
        return target.command if target is not None else None


# -- libellés ----------------------------------------------------------------
def describe(command, catalogue, lang):
    """Description de la commande dans la langue (registre) ; repli : son id."""
    for entry in catalogue or []:
        if isinstance(entry, dict) and entry.get("id") == command:
            text = texts.pick(lang, entry.get("description_fr", ""), entry.get("description_en", ""))
            if text:
                return text
    return command or "?"


def step_label(step, catalogue, lang):
    """`1. Fait une zone tampon autour des entités d'une couche.`"""
    return "%d. %s" % (step.number, describe(step.command, catalogue, lang))


def step_line(step, catalogue, lang, current=None):
    """`▶ … En cours…  2. Fait …` : marqueur, icône et mot, puis le libellé."""
    marker = MARKER if current is not None and step.number == current else " "
    return "%s %s %s  %s" % (
        marker,
        ICONS[step.status],
        texts.tr(lang, step.status),
        step_label(step, catalogue, lang),
    )


def request_to_restore(view, last_request, current_input):
    """Texte à remettre dans la saisie après une question, ou None.

    Seule une question du garde-fou (`view.restore_request`) rend la phrase d'origine, et
    seulement si la saisie est vide : ce que l'utilisateur a déjà tapé n'est pas écrasé.
    """
    if not view.restore_request or not last_request:
        return None
    if current_input and current_input.strip():
        return None
    return last_request


def left_out_text(view, lang):
    """Mention des couches que le modèle ne voit plus (ticket 5.33), ou None.

    Noms distincts dans l'ordre d'arrivée, suffixés « ×k » quand ils se répètent.
    """
    layers = client.left_out_of({"left_out": getattr(view, "left_out", None)})
    if not layers:
        return None
    counts = {}
    for layer in layers:
        counts[layer["name"]] = counts.get(layer["name"], 0) + 1
    names = ", ".join(
        name if k == 1 else "%s ×%d" % (name, k) for name, k in counts.items()
    )
    key = "left_out_one" if len(layers) == 1 else "left_out_many"
    return texts.tr(lang, key, n=len(layers), names=names)


def notice_text(view, lang):
    """Réplique de Cassini au-dessus du plan, ou None."""
    if view.notice is None:
        return None
    key, values = view.notice
    if key == "question":
        return values.get("text", "")
    return texts.tr(lang, key, **values)


def outcome(status, payload, view):
    """Résultat d'un verdict d'exécution, calculé avant de mettre `view` à jour.

    `status` : DONE, ERROR ou CANCELLED. Sans `step` dans le verdict, l'étape en
    cours fait foi. Rend le dict `last` du panneau (marqué `plan`).
    """
    payload = payload if isinstance(payload, dict) else {}
    if status == DONE:
        steps = payload.get("steps")
        steps = steps if isinstance(steps, list) else []
        return {"kind": DONE, "plan": True, "n": len(steps), "result": steps}
    step = payload.get("step")
    if step is None:
        step = view.current
    last = {"kind": status, "plan": True, "step": step}
    if status == ERROR:
        last["code"] = payload.get("code", "")
        last["message"] = payload.get("message", "")
    return last


def outcome_text(last, lang):
    """Texte du résultat d'un plan : exécuté, étape annulée, étape en erreur, ou échec."""
    kind = last.get("kind")
    if kind == DONE:
        return texts.tr(lang, "plan_done", n=last.get("n", 0))
    if kind == CANCELLED:
        return texts.tr(lang, "plan_cancelled", n=last.get("step") or "?")
    code, message = last.get("code", ""), last.get("message", "")
    if last.get("step") is None:
        # Aucune étape en cause (noyau mort avant la première) : la demande n'a pas abouti.
        return texts.tr(lang, "propose_error", code=code, message=message)
    return texts.tr(lang, "step_error", n=last["step"], code=code, message=message)


def is_cancellable(command, catalogue):
    for entry in catalogue or []:
        if isinstance(entry, dict) and entry.get("id") == command:
            return bool(entry.get("cancellable"))
    return False


# -- activations -------------------------------------------------------------
def controls(view, ready, other_busy, has_text, catalogue, external_busy=False):
    """État des contrôles de la saisie et du plan.

    `ready` : noyau prêt et sans bandeau ; `other_busy` : une commande experte
    est en vol ; `external_busy` : un appel de l'IA externe est en vol ou
    attend l'accord (saisie, Valider et Refuser grisés, raison
    `input_reason_external`). Rend un dict : `input_enabled`, `send_enabled`,
    `input_reason` (clé de texts ou None), `progress_visible`, `approve_visible`,
    `approve_enabled`, `reject_visible`, `reject_enabled`, `stop_visible`,
    `stop_enabled`, `stop_reason` (clé de texts ou None), `expert_enabled`.
    """
    phase = view.phase
    idle = phase == IDLE
    input_enabled = bool(ready and idle and not other_busy and not external_busy)
    if not ready or input_enabled:
        reason = None  # hors état prêt, le bandeau dit pourquoi
    elif external_busy and phase in (IDLE, PROPOSED):
        reason = "input_reason_external"
    elif phase == PROPOSED:
        reason = "input_reason_plan"
    else:
        reason = "input_reason"
    proposed = phase == PROPOSED
    executing = phase == EXECUTING
    command = view.current_command() if executing else None
    cancellable = command is not None and is_cancellable(command, catalogue)
    if executing and command is not None and not cancellable:
        stop_reason = "not_cancellable"
    else:
        stop_reason = None
    return {
        "input_enabled": input_enabled,
        "send_enabled": bool(input_enabled and has_text),
        "input_reason": reason,
        "progress_visible": phase == GENERATING,
        "approve_visible": proposed,
        "approve_enabled": bool(proposed and ready and not view.rejecting and not external_busy),
        "reject_visible": proposed,
        # Refuser aussi : le noyau répondrait `session.busy` et le plan montré serait perdu.
        "reject_enabled": bool(proposed and ready and not view.rejecting and not external_busy),
        "stop_visible": executing,
        "stop_enabled": bool(executing and ready and cancellable),
        "stop_reason": stop_reason,
        "expert_enabled": bool(ready and idle),
    }
