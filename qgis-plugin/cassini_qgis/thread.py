"""Le fil de la demande [12.1] : quelles cartes montrer, leurs titres et lignes, les
comptes des replis et les couleurs propres à Cassini.

Sans Qt. Le panneau ne fait que poser les widgets que ce module désigne (AD-4) : toute
décision d'affichage testable sans Qt vit ici. Les couleurs suivent `DESIGN.md` : fonds,
textes et bordures restent ceux de la palette de QGIS (`palette(...)` dans les feuilles
de style), seules les couleurs de Cassini ont une paire claire et sombre, choisie en
lisant la luminosité de la fenêtre, comme le logo.
"""

try:  # importé à plat par les tests, en paquet par QGIS
    from . import agentview, runlog, texts
except ImportError:  # pragma: no cover
    import agentview
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


def primary(c):
    """Le seul bouton principal visible : « approve », « send » ou None.

    `c` : activations de `agentview.controls`. Désactivé, un bouton perd sa couleur.
    """
    if c.get("approve_visible") and c.get("approve_enabled"):
        return "approve"
    if c.get("send_enabled"):
        return "send"
    return None


def log_fold_label(n, lang):
    if n == 0:
        return texts.tr(lang, "log_fold_empty")
    if n == 1:
        return texts.tr(lang, "log_fold_one")
    return texts.tr(lang, "log_fold_many", n=n)


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
        "step_current": (
            "QFrame#cassiniStep { background: %(accent_tint)s;"
            " border: none; border-left: 3px solid %(accent)s; }"
            " QFrame#cassiniStep QLabel { background: transparent; border: none; }"
            " QFrame#cassiniStep:focus { border: 2px solid %(accent)s;"
            " border-left: 3px solid %(accent)s; }" % c
        ),
        "step": (
            "QFrame#cassiniStep { border: none; border-left: 3px solid transparent; }"
            " QFrame#cassiniStep:focus { border: 2px solid %(accent)s; }" % c
        ),
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
        "chevron": c["accent"],
    }
