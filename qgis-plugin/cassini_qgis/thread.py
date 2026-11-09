"""Le fil de la demande [12.1] : quelles cartes montrer, leurs titres et lignes, les
comptes des replis et les couleurs propres à Cassini. Son guidage [12.2] : la ligne
« À vous », le repère d'avancement, « Reformuler la demande », la cible du focus et les
annonces de chaque état. Le rejeu du journal [12.3] : son état et son bouton.

Sans Qt. Le panneau ne fait que poser les widgets que ce module désigne (AD-4) : toute
décision d'affichage testable sans Qt vit ici. Les couleurs suivent `DESIGN.md` : fonds,
textes et bordures restent ceux de la palette de QGIS (`palette(...)` dans les feuilles
de style), seules les couleurs de Cassini ont une paire claire et sombre, choisie en
lisant la luminosité de la fenêtre, comme le logo.
"""

import re

try:  # importé à plat par les tests, en paquet par QGIS
    from . import agentview, client, runlog, texts
except ImportError:  # pragma: no cover
    import agentview
    import client
    import runlog
    import texts

# Cartes du fil, dans l'ordre où elles peuvent apparaître.
WELCOME = "welcome"
BUBBLE = "bubble"
COMPOSING = "composing"
FAILED = "failed"
QUESTION = "question"
REJECTED = "rejected"
PLAN = "plan"
PLAN_FOLDED = "plan_folded"
RESULT = "result"

# Statut d'une étape que le plan n'a pas atteinte une fois fini.
NOT_RUN = "not_run"
ICONS = dict(agentview.ICONS, **{NOT_RUN: "–"})


# -- l'échange en cours ------------------------------------------------------
def request_exchange(text, when):
    """Bulle d'une demande tapée : la phrase exacte et l'heure."""
    return {"kind": "request", "text": text, "when": when}


def expert_exchange(label, when):
    """Bulle d'une commande experte : « Mode expert : {libellé court} »."""
    return {"kind": "expert", "text": label, "when": when}


def bubble_text(exchange, lang):
    if exchange is None:
        return ""
    if exchange.get("kind") == "expert":
        return texts.tr(lang, "bubble_expert", label=exchange.get("text", ""))
    return exchange.get("text", "")


def bubble_meta(exchange, lang):
    return texts.tr(lang, "bubble_meta", time=(exchange or {}).get("when", ""))


def bubble_name(exchange, lang):
    """Nom accessible : « Votre demande, {heure} : {phrase} »."""
    return texts.tr(
        lang, "bubble_name", time=(exchange or {}).get("when", ""),
        text=bubble_text(exchange, lang),
    )


def bubble_width(width):
    """Largeur maximale de la bulle : 85 % du fil, toute la largeur sous 300 px."""
    width = max(0, int(width))
    return width if width < 300 else int(width * 0.85)


# -- les cartes ----------------------------------------------------------------
def _running_plan(last):
    return bool(last and last.get("plan") and last.get("kind") == runlog.RUNNING)


def cards(view, exchange, last, focus=None):
    """Cartes du fil, dans l'ordre, pour l'état de l'agent `view`, l'échange en cours,
    le dernier résultat `last` (dict du panneau) et le détail cliqué `focus`.

    Au repos, la carte d'accueil seule. Sinon la bulle, puis ce qui lui répond.
    """
    phase = view.phase
    notice = view.notice[0] if view.notice is not None else None
    out = []
    if exchange is None and focus is None and phase == agentview.IDLE and not view.steps:
        return [WELCOME] if last is None else [RESULT]
    if exchange is not None:
        out.append(BUBBLE)
    if phase == agentview.GENERATING:
        out.append(COMPOSING)
    elif notice == "propose_error":
        out.append(FAILED)
    elif notice == "question":
        out.append(QUESTION)
    elif notice == "plan_rejected":
        out.append(REJECTED)
    if view.steps and phase in (agentview.PROPOSED, agentview.EXECUTING):
        out.append(PLAN)
    elif view.steps and phase == agentview.IDLE:
        out.append(PLAN_FOLDED)
    if focus is not None or (last is not None and not _running_plan(last)):
        out.append(RESULT)
    return out


def plan_title(view, lang):
    n = len(view.steps)
    if view.phase == agentview.PROPOSED:
        return texts.tr(lang, "plan_title_one" if n == 1 else "plan_title_n", n=n)
    return approved_title(n, lang)


def approved_title(n, lang):
    """« Plan validé — n étapes » (carte en exécution et carte repliée)."""
    return texts.tr(lang, "plan_approved_one" if n == 1 else "plan_approved_n", n=n)


def folded_summary(view, last, lang):
    """Ligne de la carte du plan validé repliée : icône et mot selon l'issue du plan.

    L'issue vient du verdict (`last`) ; sans verdict (noyau arrêté pendant l'exécution),
    des statuts des étapes. Rend « ✔ Plan validé — n étapes · exécuté » et ses variantes.
    """
    kind = None
    if last and last.get("plan") and last.get("kind") != runlog.RUNNING:
        kind = last.get("kind")
    if kind not in (runlog.DONE, runlog.ERROR, runlog.CANCELLED):
        statuses = {s.status for s in view.steps}
        if runlog.ERROR in statuses:
            kind = runlog.ERROR
        elif runlog.CANCELLED in statuses:
            kind = runlog.CANCELLED
        else:
            kind = runlog.DONE
    return "%s %s · %s" % (
        runlog.ICONS[kind], approved_title(len(view.steps), lang),
        texts.tr(lang, "plan_outcome_" + kind),
    )


def folded_name(n, opened, lang):
    """Nom accessible de la carte repliée : « Plan validé, n étapes, réduit »."""
    state = texts.tr(lang, "unfolded" if opened else "folded")
    key = "plan_folded_name_one" if n == 1 else "plan_folded_name"
    return texts.tr(lang, key, n=n, state=state)


def plan_rows(view, catalogue, lang, layer_name=None):
    """Lignes de la carte du plan : (icône, mot d'état, libellé, message, en cours ?).

    Même forme que `result_rows`, message toujours vide."""
    refs = view.step_results()
    rows = []
    finished = view.phase == agentview.IDLE
    for step in view.steps:
        status = step.status
        if finished and status == agentview.PENDING:
            status = NOT_RUN
        rows.append((
            ICONS[status],
            texts.tr(lang, status),
            agentview.step_label(step, catalogue, lang, layer_name, refs),
            "",
            view.current is not None and step.number == view.current,
        ))
    return rows


def result_title(last, lang):
    """Titre de la carte de résultat (plan ou commande experte)."""
    kind = last.get("kind")
    if not last.get("plan"):
        return "%s %s" % (runlog.ICONS.get(kind, ""), texts.tr(lang, kind))
    if kind == runlog.DONE:
        return texts.tr(lang, "plan_done", n=last.get("n", 0))
    step = last.get("step")
    if kind == runlog.CANCELLED:
        return texts.tr(lang, "plan_stopped_by_you", k=step or "?")
    if step is None:
        return texts.tr(lang, "compose_failed_title")
    return texts.tr(lang, "plan_stopped", k=step)


def result_rows(steps, last, lang, sentence, label):
    """Lignes de la carte de résultat d'un plan : (icône, mot d'état, texte, message,
    en cours ? — toujours faux).

    `sentence(step)` : ce que l'étape a fait, en clair ; `label(step)` : son libellé court.
    L'étape fautive porte le message français du noyau, jamais son code (AD-21) ; les
    suivantes « Non lancée ».
    """
    rows = []
    failed = last.get("step") if last.get("kind") == runlog.ERROR else None
    for step in steps:
        status = step.status
        if status == agentview.PENDING:
            status = NOT_RUN
        message = ""
        if status == runlog.DONE:
            text = sentence(step)
        else:
            text = label(step)
        if status == runlog.ERROR and step.number == failed:
            message = last.get("message", "")
        rows.append((ICONS[status], texts.tr(lang, status), text, message, False))
    return rows


def shown_result(last):
    """Résultat que vise « Ouvrir le dossier », ou None.

    Commande experte réussie : son résultat. Plan exécuté : le dernier fichier produit par
    une étape (dont le dossier existe). Sinon rien.
    """
    if not last or last.get("kind") != runlog.DONE:
        return None
    if not last.get("plan"):
        return last.get("result")
    files = [getattr(s, "result", None) for s in last.get("steps") or []]
    files = [r for r in files if runlog.result_folder(r) is not None]
    return files[-1] if files else None


def closing(last, lang):
    """Phrase de clôture d'un plan exécuté, ou None."""
    if last.get("plan") and last.get("kind") == runlog.DONE:
        return texts.tr(lang, "result_closing")
    return None


def error_details(last):
    """Code `domaine.cause` d'un échec, réservé aux Détails techniques."""
    if last.get("kind") == runlog.ERROR and last.get("code"):
        return str(last.get("code"))
    return ""


# -- saisie, replis, bouton principal -------------------------------------------
def input_hint(view, reason_key, other_busy, exchange, lang):
    """Texte indicatif de la saisie : la raison quand elle est grisée, sinon l'invite."""
    if reason_key == "input_reason":
        if view.phase == agentview.GENERATING:
            reason_key = "input_reason_composing"
        elif view.phase == agentview.EXECUTING or other_busy:
            reason_key = "input_reason_running"
    if reason_key:
        return texts.tr(lang, reason_key)
    return texts.tr(lang, "input_hint_next" if exchange is not None else "input_hint")


def reason_line(reason, has_text):
    """Raison affichée sous la saisie grisée : seulement si du texte y cache le texte
    indicatif, qui la porte sinon."""
    return reason if reason and has_text else ""


def primary(c, rephrase=False):
    """Le seul bouton principal visible : « approve », « rephrase », « send » ou None.

    `c` : activations de `agentview.controls` ; `rephrase` : « Reformuler la demande » est
    montré [12.2], et Envoyer redevient alors natif. Désactivé, un bouton perd sa couleur.
    """
    if c.get("approve_visible") and c.get("approve_enabled"):
        return "approve"
    if rephrase:
        return "rephrase"
    if c.get("send_enabled"):
        return "send"
    return None


# -- guidage [12.2] : « À vous », repère d'avancement, Reformuler, focus -------------
# Cibles du focus (le panneau les traduit en widgets).
FOCUS_INPUT = "input"
FOCUS_APPROVE = "approve"
FOCUS_REPHRASE = "rephrase"
FOCUS_COMPOSING = "composing_title"
FOCUS_PLAN_TITLE = "plan_title"
FOCUS_RESULT_TITLE = "result_title"


def is_request(exchange):
    return bool(exchange and exchange.get("kind") == "request" and exchange.get("text"))


def guidance(view, exchange, last, focus=None, rephrased=False, controls=None):
    """Ce que le fil demande à l'utilisateur, d'après `EXPERIENCE.md` (State Patterns).

    Rend {"card", "cue", "values", "rephrase"} : la carte qui porte la ligne « À vous »
    (None : aucune, elle n'est jamais dans deux cartes), sa clé de texte et ses valeurs, et
    si « Reformuler la demande » y est proposé. La ligne est là si et seulement si
    l'utilisateur doit agir : plan proposé, question, échec de composition, refus, erreur
    d'étape d'un plan. Jamais pendant la composition ni l'exécution ; jamais pour une
    commande experte (aucune phrase à rendre). `rephrased` : la phrase est déjà revenue
    dans la saisie, le bouton disparaît (Envoyer redevient le bouton principal).
    `controls` : activations de `agentview.controls` ; un plan proposé dont ni Valider ni
    Refuser n'est possible (refus en cours, IA externe au travail) n'a pas de ligne.
    Un détail cliqué (`focus`) ne retire ni la ligne ni Reformuler : l'utilisateur doit
    toujours agir.
    """
    none = {"card": None, "cue": None, "values": {}, "rephrase": False}
    phase = view.phase
    if phase == agentview.PROPOSED:
        possible = controls is None or controls.get("approve_enabled") or controls.get(
            "reject_enabled"
        )
        if view.rejecting or not possible:
            return none
        return {"card": PLAN, "cue": "act_plan", "values": {}, "rephrase": False}
    if phase != agentview.IDLE:
        return none
    can = is_request(exchange) and not rephrased
    notice = view.notice[0] if view.notice is not None else None
    if notice == "question":
        return {"card": QUESTION, "cue": "act_question", "values": {}, "rephrase": False}
    if notice == "propose_error":
        return {"card": FAILED, "cue": "act_rephrase", "values": {}, "rephrase": can}
    if notice == "plan_rejected":
        return {"card": REJECTED, "cue": "act_rephrase", "values": {}, "rephrase": can}
    if (
        is_request(exchange) and last and last.get("plan")
        and last.get("kind") == runlog.ERROR
    ):
        # `{suggestion}` vient du noyau ; sans elle, la consigne générale.
        suggestion = last.get("suggestion")
        if isinstance(suggestion, str) and suggestion.strip():
            return {"card": RESULT, "cue": "act_error",
                    "values": {"suggestion": suggestion.strip()}, "rephrase": can}
        return {"card": RESULT, "cue": "act_rephrase", "values": {}, "rephrase": can}
    return none


def cue_text(guide, lang):
    """Texte de la ligne « À vous », ou ""."""
    if not guide.get("cue"):
        return ""
    return texts.tr(lang, guide["cue"], **guide.get("values", {}))


def focus_target(view, exchange, last, focus, guide):
    """Où va le focus dans cet état (`EXPERIENCE.md`, State Patterns et Accessibility Floor).

    Sur l'action que nomme la ligne « À vous » ; au titre de la carte du plan pendant
    l'exécution (après Valider le plan) ; au titre de la carte de résultat une fois fini ;
    à la saisie au repos. None : le focus ne bouge pas (détail cliqué, commande experte
    en cours, qui garde son Arrêter).
    """
    if focus is not None:
        return None
    phase = view.phase
    if phase == agentview.PROPOSED:
        return FOCUS_APPROVE
    if phase == agentview.GENERATING:
        # Arrêter est grisé (la composition n'est pas annulable) : il ne peut pas prendre le
        # focus ; le titre de la carte le garde dans le fil.
        return FOCUS_COMPOSING
    if phase == agentview.EXECUTING:
        return FOCUS_PLAN_TITLE
    if guide.get("rephrase"):
        return FOCUS_REPHRASE
    if guide.get("card") is not None:
        # Question, ou phrase déjà revenue dans la saisie par Reformuler : c'est là qu'on agit.
        return FOCUS_INPUT
    if last is not None:
        if last.get("kind") == runlog.RUNNING:
            return None
        return FOCUS_RESULT_TITLE
    return FOCUS_INPUT


def may_move_focus(focus_inside, input_focused, input_has_text):
    """Le focus ne se déplace que s'il est déjà dans le panneau, et jamais pendant que
    l'utilisateur tape dans la saisie (l'annonce suffit alors)."""
    return bool(focus_inside and not (input_focused and input_has_text))


def rephrase(exchange, last_request, current_input):
    """Geste « Reformuler la demande » : {"text", "note", "done"}.

    La phrase rendue est celle de la bulle (`exchange`, une demande tapée) ; à défaut, la
    dernière demande envoyée (`last_request`).

    `text` : la phrase à remettre dans la saisie (None : rien n'y est écrit) ; `note` : clé
    du texte qui dit pourquoi rien n'est écrit ; `done` : la phrase est dans la saisie, le
    bouton peut disparaître. Même règle que `agentview.request_to_restore` : jamais
    par-dessus un texte déjà tapé. Si la saisie contient déjà la phrase (rendue après un
    échec de composition), il n'y a rien à faire.
    """
    if is_request(exchange):
        last_request = exchange.get("text")
    text = agentview.restorable(last_request, current_input)
    if text is not None:
        return {"text": text, "note": None, "done": True}
    if not last_request:
        return {"text": None, "note": None, "done": False}
    if (current_input or "").strip() == last_request.strip():
        return {"text": None, "note": None, "done": True}
    return {"text": None, "note": "rephrase_blocked", "done": False}


def progress_text(view, lang, emphasis=None):
    """Repère « Étape k sur n en cours » pendant l'exécution, sinon "".

    `emphasis` : couleur des chiffres (texte riche, chiffres en demi-gras), ou None pour
    le texte brut (annonces, noms accessibles).
    """
    if view.phase != agentview.EXECUTING or view.current is None or not view.steps:
        return ""
    k, n = int(view.current), len(view.steps)
    if emphasis:
        k = "<b style='color:%s'>%d</b>" % (emphasis, k)
        n = "<b style='color:%s'>%d</b>" % (emphasis, n)
    return texts.tr(lang, "progress", k=k, n=n)


def plan_title_name(view, lang):
    """Nom accessible du titre de la carte du plan : le titre, et le repère pendant
    l'exécution (le focus s'y pose après Valider le plan)."""
    title = plan_title(view, lang) if view.steps else ""
    if view.phase == agentview.EXECUTING and view.current is not None and view.steps:
        return texts.tr(
            lang, "plan_title_progress", title=title, k=view.current, n=len(view.steps)
        )
    return title


def outcome_announcement(last, lang):
    """Annonce de la fin d'un plan ou d'une commande experte (zone vivante)."""
    kind = last.get("kind")
    if not last.get("plan"):
        return texts.tr(lang, {
            runlog.DONE: "live_done", runlog.CANCELLED: "live_cancelled",
        }.get(kind, "live_error"))
    if kind == runlog.ERROR:
        message = last.get("message", "") or ""
        if last.get("step") is None:
            return texts.tr(lang, "live_compose_failed", message=message).strip()
        return texts.tr(lang, "live_step_error", k=last["step"], message=message).strip()
    return result_title(last, lang)


def failed_announcement(view, lang):
    """Annonce d'un échec de composition, avec le message du noyau."""
    values = view.notice[1] if view.notice is not None else {}
    return texts.tr(lang, "live_compose_failed", message=values.get("message", "") or "").strip()


def log_fold_label(n, lang):
    if n == 0:
        return texts.tr(lang, "log_fold_empty")
    if n == 1:
        return texts.tr(lang, "log_fold_one")
    return texts.tr(lang, "log_fold_many", n=n)


# -- rejeu du journal [12.3] -------------------------------------------------------
# Le noyau rejoue tout (`journal.replay`, AD-4, AD-11) ; le panneau ne garde que l'issue,
# affichée dans le repli Journal. Le rejeu n'est pas annulable : pas d'Arrêter, et pas de
# repère « étape k sur n », le noyau n'annonçant pas les étapes d'un rejeu.
REPLAY_RUNNING = "replay_running"
REPLAY_IDENTICAL = "replay_identical"
REPLAY_DIFFERENT = "replay_different"
REPLAY_STOPPED = "replay_stopped"
REPLAY_REFUSED = "replay_refused"  # le client n'a rien envoyé (une autre requête en vol)
DIVERGED = "replay.diverged"


def replay_started():
    """État du rejeu juste parti."""
    return {"status": REPLAY_RUNNING}


def replay_running(state):
    return bool(state and state.get("status") == REPLAY_RUNNING)


def replay_refused():
    """Le clic n'est pas parti : le client avait déjà une requête en vol (AD-5)."""
    return {"status": REPLAY_REFUSED}


def replay_current(state, entries):
    """L'issue d'un rejeu fini ne vaut que pour le journal qu'elle a rejoué : dès que le
    Journal de session n'a plus `entries` lignes, elle est oubliée (None)."""
    if not state or replay_running(state):
        return state
    if state.get("entries") is not None and state.get("entries") != entries:
        return None
    return state


def _names_step(message, step):
    """Le message du noyau nomme-t-il déjà l'étape (« à l'étape 2 », « step 2 ») ?"""
    return bool(re.search(r"(?i)\b(?:étape|step)\s+%d\b" % step, message or ""))


def shown_guidance(guide, replaying):
    """Guidage tel qu'affiché : pendant un rejeu, pas de ligne « À vous » et Reformuler
    grisé (la saisie l'est aussi). La cible du focus garde le guidage brut."""
    out = dict(guide)
    out["rephrase_enabled"] = bool(guide.get("rephrase")) and not replaying
    if replaying:
        out["cue"] = None
    return out


def _step_of(value):
    return value if isinstance(value, int) and not isinstance(value, bool) else None


def replay_outcome(kind, payload):
    """Issue du rejeu d'après REPLAY_DONE ou REPLAY_FAILED du client.

    `replay.diverged` : résultat différent à l'étape `step` ; toute autre erreur : rejeu
    arrêté (à l'étape `step` s'il y en a une). Le code ne sert qu'aux Détails techniques.
    """
    payload = payload if isinstance(payload, dict) else {}
    if kind == client.REPLAY_DONE:
        if payload.get("identical") is True:
            return {"status": REPLAY_IDENTICAL}
        return {"status": REPLAY_DIFFERENT, "step": None, "message": "", "code": ""}
    code = payload.get("code")
    message = payload.get("message")
    return {
        "status": REPLAY_DIFFERENT if code == DIVERGED else REPLAY_STOPPED,
        "step": _step_of(payload.get("step")),
        "message": message if isinstance(message, str) else "",
        "code": code if isinstance(code, str) else "",
    }


def replay_text(state, lang):
    """Phrase de l'état du rejeu (sans icône), ou "" s'il n'y a rien à dire."""
    if not state:
        return ""
    status = state.get("status")
    if status in (REPLAY_RUNNING, REPLAY_IDENTICAL, REPLAY_REFUSED):
        return texts.tr(lang, status)
    step, message = state.get("step"), (state.get("message") or "").strip()
    if status == REPLAY_DIFFERENT:
        if step is None or _names_step(message, step):
            return texts.tr(lang, "replay_different_nostep", message=message).strip()
        return texts.tr(lang, "replay_different", k=step, message=message).strip()
    if not message:  # rien du noyau : jamais « Rejeu impossible : » tout court
        if step is None:
            return texts.tr(lang, "replay_failed_generic")
        return texts.tr(lang, "replay_stopped_nomsg", k=step)
    if step is None:
        return texts.tr(lang, "replay_impossible", message=message)
    return texts.tr(lang, "replay_stopped", k=step, message=message)


def replay_line(state, lang):
    """Ligne affichée sous le bouton : icône et phrase (l'état n'est pas porté par la
    couleur seule), ou ""."""
    text = replay_text(state, lang)
    if not text:
        return ""
    icon = {
        REPLAY_RUNNING: runlog.ICONS[runlog.RUNNING],
        REPLAY_IDENTICAL: runlog.ICONS[runlog.DONE],
        REPLAY_REFUSED: "ⓘ",
    }.get(state.get("status"), runlog.ICONS[runlog.ERROR])
    return "%s %s" % (icon, text)


def replay_details(state):
    """Code `domaine.cause` d'un rejeu en échec, réservé aux Détails techniques."""
    if state and state.get("status") in (REPLAY_DIFFERENT, REPLAY_STOPPED):
        return str(state.get("code") or "")
    return ""


def replay_button(view, ready, has_journal, other_busy=False, external_busy=False,
                  replay=None):
    """Bouton « Rejouer le journal » : {"enabled", "reason"} (clé de texts ou None).

    Grisé hors état prêt (le bandeau dit pourquoi), pendant le rejeu, pendant une
    composition, un plan proposé ou en exécution, une commande experte, un appel de l'IA
    externe, et quand le journal est vide, chaque fois avec sa raison.
    """
    if replay_running(replay):
        return {"enabled": False, "reason": "replay_reason_running"}
    if not ready:
        return {"enabled": False, "reason": None}
    phase = view.phase
    if phase == agentview.GENERATING:
        reason = "input_reason_composing"
    elif phase == agentview.PROPOSED:
        reason = "input_reason_plan"
    elif phase == agentview.EXECUTING or other_busy:
        reason = "input_reason_running"
    elif external_busy:
        reason = "input_reason_external"
    elif not has_journal:
        reason = "replay_reason_empty"
    else:
        return {"enabled": True, "reason": None}
    return {"enabled": False, "reason": reason}


MIN_THREAD = 120  # hauteur que le fil garde toujours, replis ouverts


def fold_heights(log, expert, available, reserved=0):
    """Hauteurs des deux replis dépliés (`None` : replié), ensemble au plus la moitié du
    panneau et jamais au point d'ôter au fil ses `MIN_THREAD` pixels.

    `available` : hauteur du panneau ; `reserved` : ce qu'y prennent l'en-tête, les lignes
    des replis, la saisie et le pied. Chaque repli prend d'abord sa part égale, puis le
    reste du budget si l'autre n'en a pas besoin. Rend (journal, expert), 0 si replié.
    """
    available, reserved = max(0, int(available)), max(0, int(reserved))
    budget = min(available // 2, available - reserved - MIN_THREAD) if available else 10 ** 6
    budget = max(0, budget)
    want = [max(0, int(c)) if c is not None else None for c in (log, expert)]
    opened = [w for w in want if w is not None]
    if not opened:
        return (0, 0)
    share = budget // len(opened)
    first = [min(w, share) if w is not None else 0 for w in want]
    spare = budget - sum(first)
    out = []
    for w, f in zip(want, first):
        extra = min(spare, w - f) if w is not None else 0
        spare -= extra
        out.append(f + extra)
    return tuple(out)


# -- couleurs de Cassini ----------------------------------------------------------
LIGHT = {
    "accent": "#0E8F86",
    "accent_strong": "#0B7A72",
    "accent_strong_border": "#08605A",
    "on_accent_strong": "#FFFFFF",
    "accent_tint": "#E4F2F0",
    "accent_tint_border": "#9CCFC9",
    # Survol et appui du bouton principal (non fixés par DESIGN.md) : plus sombre en clair.
    "primary_hover": "#08605A",
    "primary_pressed": "#064E49",
}
DARK = {
    "accent": "#3FD0C4",
    "accent_strong": "#3FD0C4",
    "accent_strong_border": "#3FD0C4",
    "on_accent_strong": "#0F1F1E",
    "accent_tint": "#1E3B39",
    "accent_tint_border": "#2F6E68",
    # Plus clair en sombre ; le libellé #0F1F1E y reste au-dessus de 4,5 : 1.
    "primary_hover": "#6ADCD3",
    "primary_pressed": "#2FB3A8",
}


def is_dark(window_lightness):
    """Même règle que le logo : fenêtre de QGIS sombre sous 128 de luminosité."""
    return window_lightness < 128


def colors(dark):
    return dict(DARK if dark else LIGHT)


def styles(c):
    """Feuilles de style des seuls éléments propres à Cassini.

    Fonds, textes et bordures des cartes viennent de la palette de QGIS (`palette(...)`) ;
    seules les valeurs de `c` (paire claire ou sombre) sont propres à Cassini.
    """
    # Lignes d'étape : styles ajoutés par le panneau à la feuille de la carte, l'étape en cours repérée
    # par la propriété `current`. Une feuille propre à chaque ligne lui faisait résoudre sa
    # police pour un autre écran que la carte (lignes coupées à 150 %, défaut de 12.1).
    step = (
        "QFrame#cassiniStep { border: none; border-left: 3px solid transparent; }"
        " QFrame#cassiniStep:focus { border: 2px solid %(accent)s; }" % c
    )
    step_current = (
        "QFrame#cassiniStep[current=\"true\"] { background: %(accent_tint)s;"
        " border: none; border-left: 3px solid %(accent)s; }"
        " QFrame#cassiniStep[current=\"true\"]:focus { border: 2px solid %(accent)s;"
        " border-left: 3px solid %(accent)s; }" % c
    )
    return {
        "card": (
            "QFrame#cassiniCard { background: palette(base); border: 1px solid palette(mid);"
            " border-radius: 3px; }"
            " QFrame#cassiniCard QLabel { background: transparent; border: none; }"
        ),
        "bubble": (
            "QFrame#cassiniBubble { background: %(accent_tint)s;"
            " border: 1px solid %(accent_tint_border)s; border-radius: 8px;"
            " border-bottom-right-radius: 2px; }"
            " QFrame#cassiniBubble QLabel { background: transparent; border: none; }" % c
        ),
        "step_current": step_current,
        "step": step,
        "primary": (
            "QPushButton { background: %(accent_strong)s; color: %(on_accent_strong)s;"
            " border: 1px solid %(accent_strong_border)s; border-radius: 2px;"
            " font-weight: 600; padding: 4px 10px; }"
            " QPushButton:hover { background: %(primary_hover)s; border-color: %(primary_hover)s; }"
            " QPushButton:pressed { background: %(primary_pressed)s;"
            " border-color: %(primary_pressed)s; }"
            # `focus-ring-primary` : trait de 2 px à la couleur du texte du thème. QSS n'a pas
            # de contour décalé : le trait borde le bouton au lieu d'être à 2 px de lui.
            " QPushButton:focus { border: 2px solid palette(window-text); padding: 3px 9px; }"
            % c
        ),
        "input": (
            "QPlainTextEdit:focus { border: 2px solid %(accent)s; }" % c
        ),
        # Ligne « À vous » (`act-cue`) : fond AlternateBase du thème, trait gauche sarcelle.
        "cue": (
            "QLabel#cassiniCue { background: palette(alternate-base); border: none;"
            " border-left: 3px solid %(accent)s; padding: 4px 8px; }" % c
        ),
        # Titre de carte qui reçoit le focus (plan, résultat, composition) : anneau sarcelle.
        "title": (
            "QLabel#cassiniCardTitle { border: 2px solid transparent; }"
            " QLabel#cassiniCardTitle:focus { border: 2px solid %(accent)s; }" % c
        ),
        "chevron": c["accent"],
    }
