"""Widgets du panneau Cassini [9.4]. Qt uniquement par `qgis.PyQt` (AD-18).

Aucune logique métier (AD-4) : le panneau affiche, demande et rend la main à
l'extension. Le noyau valide les paramètres et décide de tout. Les textes, le
formulaire par schéma et le journal sont dans des modules sans Qt, testés seuls.
"""

import os
import time

from qgis.PyQt.QtCore import QSettings, Qt, QUrl
from qgis.PyQt.QtGui import QDesktopServices, QFontDatabase, QIcon
from qgis.PyQt.QtWidgets import (
    QCheckBox,
    QComboBox,
    QDialog,
    QFileDialog,
    QFormLayout,
    QHBoxLayout,
    QLabel,
    QLineEdit,
    QListWidget,
    QListWidgetItem,
    QPlainTextEdit,
    QProgressBar,
    QPushButton,
    QScrollArea,
    QToolButton,
    QVBoxLayout,
    QWidget,
)

from . import agentview, client, external, form, models, runlog, summary, texts


class LayerCombo(QComboBox):
    """Liste déroulante des couches du projet, relue à chaque ouverture."""

    def __init__(self, parent=None):
        super().__init__(parent)
        self.reload()

    def reload(self):
        from qgis.core import QgsProject

        keep = self.currentData()
        self.clear()
        for node in QgsProject.instance().layerTreeRoot().findLayers():
            layer = node.layer()
            if layer is not None:
                self.addItem(layer.name(), layer.id())
        index = self.findData(keep)
        if index >= 0:
            self.setCurrentIndex(index)

    def showPopup(self):
        self.reload()
        super().showPopup()


class FieldCombo(QComboBox):
    """Liste déroulante des champs de la couche choisie dans une `LayerCombo`, relue quand
    cette couche change et à chaque ouverture. Reliée après coup (`bind`) : les champs du
    formulaire sont rangés par ordre alphabétique, `field` vient avant `layer_id`."""

    def __init__(self, parent=None, optional=False, empty_text=""):
        super().__init__(parent)
        self._layer_combo = None
        self._optional = optional
        self._empty_text = empty_text

    def bind(self, layer_combo):
        self._layer_combo = layer_combo
        layer_combo.currentIndexChanged.connect(lambda _=0: self.reload())
        self.reload()

    def reload(self):
        from qgis.core import QgsProject

        keep = self.currentData()
        self.clear()
        layer_id = self._layer_combo.currentData() if self._layer_combo is not None else None
        layer = QgsProject.instance().mapLayer(layer_id or "")
        if self._optional:
            self.addItem(self._empty_text, "")
        if layer is not None and hasattr(layer, "fields"):
            for f in layer.fields():
                self.addItem(f.name(), f.name())
        index = self.findData(keep)
        if index >= 0:
            self.setCurrentIndex(index)

    def showPopup(self):
        self.reload()
        super().showPopup()


class RequestEdit(QPlainTextEdit):
    """Saisie principale : Entrée envoie, Maj+Entrée saute une ligne, Tab change de champ."""

    def __init__(self, on_submit, parent=None):
        super().__init__(parent)
        self._on_submit = on_submit
        self.setTabChangesFocus(True)

    def keyPressEvent(self, event):
        if event.key() in (Qt.Key.Key_Return, Qt.Key.Key_Enter) and not (
            event.modifiers() & Qt.KeyboardModifier.ShiftModifier
        ):
            self._on_submit()
            return
        super().keyPressEvent(event)


LANG_KEY = "cassini/lang"
EXPERT_KEY = "cassini/expert_open"
EXTERNAL_ASK_KEY = "cassini/external_ask"  # case « Me demander avant chaque appel »
ASSETS = os.path.join(os.path.dirname(os.path.abspath(__file__)), "assets")

# Bandeaux : clé de texte, action proposée ("choose", "relaunch" ou aucune).
BANNERS = {
    "no_project": ("no_project", None),
    "starting": ("starting", None),
    "not_found": ("not_found", "choose"),
    "died": ("died", "relaunch"),
    "refused": ("refused", "choose"),  # pas de Relancer : même noyau, même refus
}


def _announce(widget, text):
    """Annonce aux lecteurs d'écran (zone vivante), sans jamais lever d'exception."""
    try:
        from qgis.PyQt.QtGui import QAccessible, QAccessibleEvent

        widget.setAccessibleDescription(text)
        QAccessible.updateAccessibility(
            QAccessibleEvent(widget, QAccessible.Event.NameChanged)
        )
    except Exception:
        pass


def _layer_name(layer_id):
    """Nom de la couche dans le projet QGIS, ou None ; jamais d'exception."""
    try:
        from qgis.core import QgsProject

        layer = QgsProject.instance().mapLayer(layer_id)
        return layer.name() if layer is not None else None
    except Exception:
        return None


def _title(text_label):
    font = text_label.font()
    font.setBold(True)
    text_label.setFont(font)
    return text_label


def _mono(widget):
    widget.setFont(QFontDatabase.systemFont(QFontDatabase.SystemFont.FixedFont))
    return widget


class _Fold(QWidget):
    """Groupe repliable natif : une flèche à bascule et un contenu."""

    def __init__(self, on_toggle=None):
        super().__init__()
        self._on_toggle = on_toggle
        self.button = QToolButton()
        self.button.setCheckable(True)
        self.button.setToolButtonStyle(Qt.ToolButtonStyle.ToolButtonTextBesideIcon)
        self.button.toggled.connect(self._toggled)
        self.body = QWidget()
        self.body.setVisible(False)
        layout = QVBoxLayout(self)
        layout.setContentsMargins(0, 0, 0, 0)
        layout.addWidget(self.button)
        layout.addWidget(self.body)
        self._sync(False)

    def _sync(self, opened):
        self.button.setArrowType(
            Qt.ArrowType.DownArrow if opened else Qt.ArrowType.RightArrow
        )

    def _toggled(self, opened):
        self._sync(opened)
        self.body.setVisible(opened)
        if self._on_toggle is not None:
            self._on_toggle(opened)

    def set_open(self, opened):
        self.button.setChecked(opened)

    def is_open(self):
        return self.button.isChecked()


class CassiniPanel(QWidget):
    """Colonne unique : en-tête, bandeau, saisie, résultat, journal, mode expert."""

    def __init__(self, actions):
        super().__init__()
        # run(cmd, params), stop(), choose_core(), relaunch(), propose(text), approve(), reject(),
        # set_external(enabled, ask), stop_external(call_id), answer_approval(request_id, approved)
        self._actions = actions
        settings = QSettings()
        # Interrupteur éteint à chaque démarrage ; seule la case est retenue.
        ask = external.ask_setting(settings.value(EXTERNAL_ASK_KEY, "false"))
        self.external = external.ExternalState(ask=ask)
        self.lang = texts.normalize(str(settings.value(LANG_KEY, texts.DEFAULT_LANG)))
        self.log = runlog.RunLog()
        self.agent = agentview.AgentView()
        self._plan_entry = None  # ligne du journal de l'étape de plan en cours
        self._plan_entries = {}  # numéro d'étape -> ligne du journal, pour le plan en cours
        self._plan_refs = {}  # {k: résultat de l'étape k} du plan en cours (partagé par ses lignes)
        self._run_params = None  # paramètres de la commande experte qui part
        self.focus = None  # ligne du journal ou étape du plan cliquée : ("entry", e) / ("step", s)
        self._shown_result = None  # résultat affiché, pour « Ouvrir le dossier »
        self._last_request = None  # dernière demande envoyée, rendue après un échec
        self.models = None  # dernière liste d'`agent.models` (nettoyée), None avant la première
        self.models_error = None  # (code, message) du dernier échec de la liste
        remembered = settings.value(models.SETTING_KEY, "")
        self.model = str(remembered) if remembered else None  # nom envoyé avec chaque demande
        self._filling_models = False  # remplissage de la liste : pas un choix de l'utilisateur
        self._models_shown = None  # (langue, liste, choix) déjà affichés dans la liste
        self.catalogue = []
        self.ready = False
        self.banner = "no_project"
        self.running = False
        self.last = None  # {"kind", "command", "result"/"message"/"code"}
        self.selected = None  # entrée du catalogue choisie
        self._fields = []
        self._inputs = {}
        self.project_file = None  # fichier du projet ouvert, pour « Parcourir… »
        self._build()
        expert = str(settings.value(EXPERT_KEY, "false")).lower() in ("true", "1")
        self.expert.set_open(expert)
        self.render()

    # -- construction --------------------------------------------------
    def _logo(self):
        dark = self.palette().window().color().lightness() < 128
        name = "logo-cassini-dark.svg" if dark else "logo-cassini-light.svg"
        return QIcon(os.path.join(ASSETS, name)).pixmap(24, 24)

    def _build(self):
        outer = QVBoxLayout(self)
        outer.setContentsMargins(0, 0, 0, 0)
        scroll = QScrollArea()
        scroll.setWidgetResizable(True)
        scroll.setHorizontalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAlwaysOff)
        outer.addWidget(scroll)
        body = QWidget()
        scroll.setWidget(body)
        col = QVBoxLayout(body)
        col.setSpacing(8)

        # En-tête : logo, titre, Réglages.
        head = QHBoxLayout()
        self.logo = QLabel()
        self.logo.setPixmap(self._logo())
        self.logo.setAccessibleName("Cassini")
        self.title = _title(QLabel())
        self.settings_button = QToolButton()
        self.settings_button.setText("⚙")
        self.settings_button.clicked.connect(self._open_settings)
        head.addWidget(self.logo)
        head.addWidget(self.title, 1)
        head.addWidget(self.settings_button)
        col.addLayout(head)

        # Réglages : langue, modèle (liste rendue par le noyau, `agent.models`).
        # Fenêtre à part : les réglages ne se mêlent pas au panneau.
        self.settings_zone = QDialog(self)
        self.settings_zone.setModal(True)
        sl = QVBoxLayout(self.settings_zone)
        self.lang_label = QLabel()
        self.lang_box = QComboBox()
        self.lang_box.addItem("Français", "fr")
        self.lang_box.addItem("English", "en")
        self.lang_box.setCurrentIndex(0 if self.lang == "fr" else 1)
        self.lang_box.currentIndexChanged.connect(self._lang_changed)
        self.model_label = QLabel()
        self.model_box = QComboBox()
        self.model_box.currentIndexChanged.connect(self._model_changed)
        self.model_label.setBuddy(self.model_box)
        self.model_reason = QLabel()
        self.model_reason.setWordWrap(True)
        self.model_reason.setTextInteractionFlags(
            Qt.TextInteractionFlag.TextSelectableByMouse
        )
        sl.addWidget(self.lang_label)
        sl.addWidget(self.lang_box)
        sl.addWidget(self.model_label)
        sl.addWidget(self.model_box)
        sl.addWidget(self.model_reason)
        # IA externe (AD-29) : interrupteur éteint par défaut, case retenue.
        self.external_switch = QCheckBox()
        self.external_switch.clicked.connect(self._switch_clicked)
        self.external_ask = QCheckBox()
        self.external_ask.clicked.connect(self._ask_clicked)
        self.external_reason = QLabel()
        self.external_reason.setTextFormat(Qt.TextFormat.PlainText)  # message venu du noyau
        self.external_reason.setWordWrap(True)
        self.external_reason.setTextInteractionFlags(
            Qt.TextInteractionFlag.TextSelectableByMouse
        )
        sl.addWidget(self.external_switch)
        sl.addWidget(self.external_ask)
        sl.addWidget(self.external_reason)

        # Bandeau d'état : un seul à la fois.
        self.banner_zone = QWidget()
        bl = QHBoxLayout(self.banner_zone)
        bl.setContentsMargins(0, 0, 0, 0)
        self.banner_icon = QLabel("ⓘ")
        self.banner_text = QLabel()
        self.banner_text.setWordWrap(True)
        self.banner_button = QPushButton()
        self.banner_button.clicked.connect(self._banner_action)
        bl.addWidget(self.banner_icon)
        bl.addWidget(self.banner_text, 1)
        bl.addWidget(self.banner_button)
        col.addWidget(self.banner_zone)

        # Bandeau de l'IA externe, distinct de celui du noyau : Couper, Arrêter l'appel en vol.
        self.external_zone = QWidget()
        xl = QVBoxLayout(self.external_zone)
        xl.setContentsMargins(0, 0, 0, 0)
        xrow = QHBoxLayout()
        self.external_icon = QLabel("⇄")
        self.external_text = QLabel()
        self.external_text.setWordWrap(True)
        self.external_cut = QPushButton()
        self.external_cut.clicked.connect(self._cut_external)
        self.external_stop = QPushButton()
        self.external_stop.clicked.connect(self._stop_external)
        xrow.addWidget(self.external_icon)
        xrow.addWidget(self.external_text, 1)
        xrow.addWidget(self.external_stop)
        xrow.addWidget(self.external_cut)
        self.external_stop_reason = QLabel()
        self.external_stop_reason.setWordWrap(True)
        xl.addLayout(xrow)
        xl.addWidget(self.external_stop_reason)
        col.addWidget(self.external_zone)

        # Question d'accord de l'IA externe : une seule, sans fenêtre modale.
        self.approval_zone = QWidget()
        al = QVBoxLayout(self.approval_zone)
        al.setContentsMargins(0, 0, 0, 0)
        self.approval_text = QLabel()
        # Paramètres écrits par l'IA externe : jamais interprétés comme du texte riche.
        self.approval_text.setTextFormat(Qt.TextFormat.PlainText)
        self.approval_text.setWordWrap(True)
        self.approval_text.setTextInteractionFlags(
            Qt.TextInteractionFlag.TextSelectableByMouse
        )
        self.approval_allow = QPushButton()
        self.approval_allow.clicked.connect(lambda _=False: self._answer(True))
        self.approval_deny = QPushButton()
        self.approval_deny.clicked.connect(lambda _=False: self._answer(False))
        arow = QHBoxLayout()
        arow.addWidget(self.approval_allow)
        arow.addWidget(self.approval_deny)
        al.addWidget(self.approval_text)
        al.addLayout(arow)
        col.addWidget(self.approval_zone)

        # Saisie principale en langage naturel [Epic 5].
        self.input = RequestEdit(self._send)
        self.input.setMaximumHeight(72)
        self.input.textChanged.connect(self._input_changed)
        self.input_reason = QLabel()
        self.input_reason.setWordWrap(True)
        self.send_button = QPushButton()
        self.send_button.clicked.connect(self._send)
        col.addWidget(self.input)
        col.addWidget(self.input_reason)
        col.addWidget(self.send_button)

        # Plan de l'agent : réplique, indicateur, étapes en lecture seule, verdict.
        self.plan_zone = QWidget()
        pl = QVBoxLayout(self.plan_zone)
        pl.setContentsMargins(0, 0, 0, 0)
        self.plan_title = _title(QLabel())
        self.plan_notice = QLabel()
        self.plan_notice.setWordWrap(True)
        self.plan_notice.setTextInteractionFlags(
            Qt.TextInteractionFlag.TextSelectableByMouse
        )
        self.plan_progress = QProgressBar()
        self.plan_progress.setRange(0, 0)  # indéterminée : la composition ne se mesure pas
        self.plan_progress.setTextVisible(False)
        self.plan_list = QListWidget()
        self.plan_list.setMaximumHeight(130)
        self.plan_list.setWordWrap(True)
        self.plan_list.itemClicked.connect(self._plan_clicked)
        self.plan_list.itemActivated.connect(self._plan_clicked)
        self.approve_button = QPushButton()
        self.approve_button.clicked.connect(self._approve)
        self.reject_button = QPushButton()
        self.reject_button.clicked.connect(self._reject)
        self.plan_stop_button = QPushButton()
        self.plan_stop_button.clicked.connect(self._stop)
        self.plan_reason = QLabel()
        self.plan_reason.setWordWrap(True)
        self.plan_details = _Fold()
        self.plan_details_view = _mono(QPlainTextEdit())
        self.plan_details_view.setReadOnly(True)
        self.plan_details_view.setMaximumHeight(120)
        pdl = QVBoxLayout(self.plan_details.body)
        pdl.setContentsMargins(0, 0, 0, 0)
        pdl.addWidget(self.plan_details_view)
        plan_buttons = QHBoxLayout()
        plan_buttons.addWidget(self.approve_button)
        plan_buttons.addWidget(self.reject_button)
        plan_buttons.addWidget(self.plan_stop_button)
        pl.addWidget(self.plan_notice)
        pl.addWidget(self.plan_progress)
        pl.addWidget(self.plan_title)
        pl.addWidget(self.plan_list)
        pl.addLayout(plan_buttons)
        pl.addWidget(self.plan_reason)
        pl.addWidget(self.plan_details)
        col.addWidget(self.plan_zone)

        # Résultat.
        self.result_title = _title(QLabel())
        self.result_text = QLabel()
        self.result_text.setWordWrap(True)
        self.result_text.setTextInteractionFlags(
            Qt.TextInteractionFlag.TextSelectableByMouse
        )
        self.open_folder_button = QPushButton()
        self.open_folder_button.clicked.connect(self._open_folder)
        self.details = _Fold()
        self.details_view = _mono(QPlainTextEdit())
        self.details_view.setReadOnly(True)
        self.details_view.setMaximumHeight(120)
        dl = QVBoxLayout(self.details.body)
        dl.setContentsMargins(0, 0, 0, 0)
        dl.addWidget(self.details_view)
        col.addWidget(self.result_title)
        col.addWidget(self.result_text)
        col.addWidget(self.open_folder_button)
        col.addWidget(self.details)

        # Journal de la session.
        self.log_title = _title(QLabel())
        self.log_list = QListWidget()
        self.log_list.setMaximumHeight(110)
        self.log_list.setWordWrap(True)
        self.log_list.itemClicked.connect(self._log_clicked)
        self.log_list.itemActivated.connect(self._log_clicked)
        self.log_empty = QLabel()
        col.addWidget(self.log_title)
        col.addWidget(self.log_empty)
        col.addWidget(self.log_list)

        # Mode expert.
        self.expert = _Fold(self._expert_toggled)
        el = QVBoxLayout(self.expert.body)
        el.setContentsMargins(0, 0, 0, 0)
        self.command_list = QListWidget()
        self.command_list.setMaximumHeight(130)
        self.command_list.currentItemChanged.connect(self._command_changed)
        self.form_zone = QWidget()
        self.form_layout = QFormLayout(self.form_zone)
        self.form_layout.setContentsMargins(0, 0, 0, 0)
        self.form_hint = QLabel()
        self.form_hint.setWordWrap(True)
        self.run_button = QPushButton()
        self.run_button.clicked.connect(self._run)
        self.stop_button = QPushButton()
        self.stop_button.clicked.connect(self._stop)
        self.reason = QLabel()
        self.reason.setWordWrap(True)
        self.form_error = QLabel()
        self.form_error.setWordWrap(True)
        buttons = QHBoxLayout()
        buttons.addWidget(self.run_button)
        buttons.addWidget(self.stop_button)
        el.addWidget(self.command_list)
        el.addWidget(self.form_hint)
        el.addWidget(self.form_zone)
        el.addLayout(buttons)
        el.addWidget(self.reason)
        el.addWidget(self.form_error)
        col.addWidget(self.expert)

        # Pied : information, pas une panne.
        self.footer = QLabel()
        self.footer.setWordWrap(True)
        self.footer.setEnabled(False)
        col.addWidget(self.footer)
        col.addStretch(1)

        # Ordre de tabulation = ordre vertical.
        order = [
            self.settings_button, self.lang_box, self.model_box, self.external_switch,
            self.external_ask, self.banner_button, self.external_stop, self.external_cut,
            self.approval_allow, self.approval_deny, self.input,
            self.send_button, self.plan_list, self.approve_button, self.reject_button,
            self.plan_stop_button, self.plan_details.button, self.details.button, self.log_list,
            self.expert.button, self.command_list, self.run_button, self.stop_button,
        ]
        for a, b in zip(order, order[1:]):
            QWidget.setTabOrder(a, b)

    # -- état ----------------------------------------------------------
    def t(self, key, **values):
        return texts.tr(self.lang, key, **values)

    def set_banner(self, kind):
        """`kind` : clé de BANNERS, ou None quand le noyau est prêt."""
        self.banner = kind
        self.focus = None
        if kind is not None:
            self.ready = False
            self._drop_stale_run()
        self.render()
        if kind is not None:
            _announce(self.banner_text, self.t(BANNERS[kind][0]))

    def _drop_stale_run(self):
        """Le noyau a été fermé : une commande ou un plan en vol n'existe plus."""
        if self.running:
            self.running = False
            self.log.finish(runlog.CANCELLED)
        # Interrupteur éteint, question fermée, appels externes en cours annulés.
        self.external.lost(self.log)
        plan_in_flight = self.agent.phase != agentview.IDLE
        self._close_plan_entry(runlog.ERROR)
        self.agent.forget()
        last = self.last
        if plan_in_flight and last and last.get("plan") and last["kind"] == runlog.RUNNING:
            self.last = None  # plus d'« En cours… » d'une étape qui n'existe plus

    def set_ready(self, catalogue):
        self._drop_stale_run()
        self.ready = True
        self.banner = None
        self.catalogue = list(catalogue)
        self.selected = None
        self.render()
        _announce(self.title, self.t("live_ready"))

    def _close_plan_entry(self, status, step=None):
        """Ferme la ligne de l'étape en cours ; si le verdict nomme une autre étape
        (`step`), la ligne en cours est finie : elle passe en Terminée."""
        entry, self._plan_entry = self._plan_entry, None
        if step is not None and entry is not None and entry.step != step:
            status = runlog.DONE
        if entry is not None and entry.status == runlog.RUNNING:
            entry.status = status

    def _keep_plan_results(self, failed=None, error=None):
        """Range le résultat de chaque étape finie dans sa ligne du journal."""
        self._plan_refs.update(self.agent.step_results())
        for step in self.agent.steps:
            entry = self._plan_entries.get(step.number)
            if entry is not None and step.result is not None:
                entry.result = step.result
        entry = self._plan_entries.get(failed)
        if entry is not None and error is not None:
            entry.error = error

    def _on_plan_event(self, kind, payload):
        agent = self.agent
        self.focus = None  # tout nouvel événement efface le détail cliqué
        if kind == client.PLAN_PROPOSED:
            self._plan_entries = {}
            self._plan_refs = {}
            agent.proposed(payload.get("plan_id"), payload.get("plan"))
            _announce(self.plan_title, self.t("live_plan"))
        elif kind == client.PLAN_QUESTION:
            agent.question(payload.get("question", ""), payload.get("source", "model"))
            restored = agentview.request_to_restore(
                agent, self._last_request, self.input.toPlainText()
            )
            if restored is not None:
                self.input.setPlainText(restored)  # à compléter par le point
            _announce(self.plan_notice, self.t("live_question"))
        elif kind == client.PLAN_FAILED:
            agent.failed(payload.get("code", ""), payload.get("message", ""))
            if payload.get("code") in ("models.unknown", "models.unusable"):
                self._actions.list_models()  # la liste a changé : la redemander
            if self._last_request and not self.input.toPlainText().strip():
                self.input.setPlainText(self._last_request)  # à corriger sans tout retaper
            _announce(self.plan_notice, self.t("live_error"))
        elif kind == client.PLAN_REJECTED:
            agent.rejected()
        elif kind == client.PLAN_STEP:
            self._close_plan_entry(runlog.DONE)
            agent.step(payload.get("step"), payload.get("command"))
            target = agent.find(payload.get("step"))
            command = payload.get("command") or (target.command if target else "?")
            label = (
                agentview.step_label(target, self.catalogue, self.lang)
                if target is not None
                else agentview.describe(command, self.catalogue, self.lang)
            )
            self._plan_entry = self.log.start(
                command,
                label,
                time.strftime("%H:%M"),
                params=target.params if target is not None else None,
                step=payload.get("step"),
                refs=self._plan_refs,
            )
            self._plan_entries[payload.get("step")] = self._plan_entry
            self.last = {"kind": runlog.RUNNING, "plan": True, "command": label}
        elif kind == client.PLAN_DONE:
            self._close_plan_entry(runlog.DONE)
            self.last = agentview.outcome(runlog.DONE, payload, agent)
            agent.done(payload.get("steps"))
            self._keep_plan_results()
            self.last["steps"] = list(agent.steps)
            self.last["refs"] = self._plan_refs
            _announce(self.result_text, self.t("live_done"))
        elif kind == client.PLAN_EXEC_FAILED:
            self.last = agentview.outcome(runlog.ERROR, payload, agent)
            self._close_plan_entry(runlog.ERROR, self.last["step"])
            agent.exec_failed(self.last["step"], payload.get("outputs"))
            self._keep_plan_results(
                self.last["step"], (self.last.get("code", ""), self.last.get("message", ""))
            )
            _announce(self.result_text, self.t("live_error"))
        elif kind == client.PLAN_CANCELLED:
            self.last = agentview.outcome(runlog.CANCELLED, payload, agent)
            self._close_plan_entry(runlog.CANCELLED, self.last["step"])
            agent.cancelled(self.last["step"], payload.get("outputs"))
            self._keep_plan_results()
            _announce(self.result_text, self.t("live_cancelled"))
        self.render()

    def on_models(self, kind, payload):
        """Liste des modèles reçue (MODELS) ou en échec (MODELS_FAILED)."""
        if kind == client.MODELS:
            self.models = models.clean(payload.get("models"))
            self.models_error = None
            chosen, forget = models.choose(self.models, self.model)
            if forget:
                QSettings().remove(models.SETTING_KEY)  # choix disparu : oublié
            self.model = chosen
        else:
            self.models_error = (payload.get("code", ""), payload.get("message", ""))
        self._render_models()

    def _on_external_event(self, kind, payload):
        """Réponses de `session.external`, appels de l'IA externe et question d'accord.

        La logique est dans `external.apply` ; ici, seulement l'annonce et le rendu.
        """
        out = external.apply(
            self.external, self.log, kind, payload, self.catalogue, self.lang,
            time.strftime("%H:%M"),
        )
        if out["resend_ask"]:  # case changée avant la réponse du noyau
            self._actions.set_external(True, self.external.ask)
        if out["announce"]:
            where = {"log": self.log_list, "approval": self.approval_text}.get(
                out["where"], self.external_text
            )
            _announce(where, out["announce"])
        self.render()

    def on_event(self, kind, payload):
        """Événements RUN_*, PLAN_*, des modèles et de l'IA externe du client."""
        if kind in client.EXTERNAL_EVENTS:
            self._on_external_event(kind, payload or {})
            return
        if kind in client.MODEL_EVENTS:
            self.on_models(kind, payload or {})
            return
        if kind in client.PLAN_EVENTS:
            self._on_plan_event(kind, payload or {})
            return
        self.focus = None  # tout nouvel événement efface le détail cliqué
        if kind == client.RUN_STARTED:
            self.running = True
            params, self._run_params = self._run_params, None
            self.log.start(
                payload["command"], payload["command"], time.strftime("%H:%M"), params=params
            )
            self.last = {"kind": runlog.RUNNING, "command": payload["command"]}
        elif kind == client.RUN_DONE:
            self.running = False
            entry = self.log.finish(runlog.DONE)
            if entry is not None:
                entry.result = payload.get("result")
            self.last = {"kind": runlog.DONE, "params": entry.params if entry else None, **payload}
            _announce(self.result_text, self.t("live_done"))
        elif kind == client.RUN_CANCELLED:
            self.running = False
            self.log.finish(runlog.CANCELLED)
            self.last = {"kind": runlog.CANCELLED, **payload}
            _announce(self.result_text, self.t("live_cancelled"))
        elif kind == client.RUN_FAILED:
            self.running = False
            entry = self.log.finish(runlog.ERROR)
            if entry is not None:
                entry.error = (payload.get("code", ""), payload.get("message", ""))
            self.last = {"kind": runlog.ERROR, **payload}
            _announce(self.result_text, self.t("live_error"))
        self.render()

    # -- affichage -----------------------------------------------------
    def render(self):
        t = self.t
        self.title.setText(t("app_title"))
        self.settings_button.setAccessibleName(t("settings"))
        self.settings_button.setToolTip(t("settings"))
        self.settings_zone.setWindowTitle(t("settings"))
        self.lang_label.setText(t("settings_language"))
        self.lang_box.setAccessibleName(t("settings_language"))
        self.model_label.setText(t("settings_model"))
        self._render_models()

        usable = self.ready and self.banner is None
        if self.banner is None:
            self.banner_zone.setVisible(False)
        else:
            key, action = BANNERS[self.banner]
            self.banner_text.setText(t(key))
            self.banner_text.setAccessibleName(t(key))
            self.banner_button.setVisible(action is not None)
            if action == "choose":
                self.banner_button.setText(t("choose_core"))
            elif action == "relaunch":
                self.banner_button.setText(t("relaunch"))
            self.banner_button.setAccessibleName(self.banner_button.text())
            self.banner_zone.setVisible(True)

        self._render_external()

        self.input.setPlaceholderText(t("input_hint"))
        self.input.setAccessibleName(t("input_name"))
        self.send_button.setText(t("send"))
        self._render_agent()

        self.result_title.setText(t("result_title"))
        self.details.button.setText(t("details"))
        self.details.button.setAccessibleName(t("details"))
        self._render_result()

        self.log_title.setText(t("log_title"))
        self.log_empty.setText(t("log_empty"))
        self.log_empty.setVisible(not self.log.entries)
        self.log_list.setVisible(bool(self.log.entries))
        self.log_list.clear()
        for entry in self.log.entries:
            if entry.origin == runlog.EXTERNAL:  # marquée « IA externe », dans la langue choisie
                entry.label = external.label(entry.command, self.catalogue, self.lang)
            self.log_list.addItem(
                runlog.line_text(
                    entry, t(entry.status), external.duration_text(entry.duration_ms, self.lang)
                )
            )
        self.log_list.setAccessibleName(t("log_title"))
        self.log_list.scrollToBottom()

        self.expert.button.setText(t("expert_title"))
        self.expert.button.setAccessibleName(t("expert_title"))
        self.expert.setEnabled(usable and self.agent.phase == agentview.IDLE)
        self.footer.setText(t("offline"))
        self._render_commands()
        self._render_buttons()

    def _open_folder(self):
        folder = runlog.result_folder(self._shown_result)
        if folder is not None:
            QDesktopServices.openUrl(QUrl.fromLocalFile(folder))

    def _sentence(self, command, params, result, refs=None):
        """Ce que la commande a fait, en une phrase (logique dans `summary`)."""
        return summary.sentence(
            command, params, result, self.lang, _layer_name, refs, self.catalogue
        )

    def _log_clicked(self, item):
        row = self.log_list.row(item)
        if 0 <= row < len(self.log.entries):
            self.focus = ("entry", self.log.entries[row])
            self._render_result()  # sans refaire les listes pendant le clic
            _announce(self.result_text, self.result_text.text())

    def _plan_clicked(self, item):
        row = self.plan_list.row(item)
        if 0 <= row < len(self.agent.steps):
            self.focus = ("step", self.agent.steps[row])
            self._render_result()
            _announce(self.result_text, self.result_text.text())

    def _focus_error(self, focus):
        """Texte d'erreur de la ligne ou de l'étape cliquée, ou None."""
        kind, item = focus
        if kind == "step":
            last = self.last
            if last and last.get("plan") and last.get("step") == item.number:
                return agentview.outcome_text(last, self.lang)
            return None
        if item.error is None:
            return None
        code, message = item.error
        return self.t("step_error", n=item.step or 1, code=code, message=message)

    def _render_focus(self):
        """Détail de la ligne du journal ou de l'étape du plan cliquée."""
        kind, item = self.focus
        if kind == "step":
            label = agentview.step_label(item, self.catalogue, self.lang)
            refs = self.agent.step_results()
        else:
            label = item.label
            refs = item.refs
        result = item.result if item.status == runlog.DONE else None
        error = self._focus_error(self.focus) if item.status == runlog.ERROR else None
        self.result_text.setText(
            summary.detail(item, label, self.lang, _layer_name, refs, self.catalogue, error)
        )
        self._shown_result = result
        folder = runlog.result_folder(result)
        self.open_folder_button.setVisible(folder is not None)
        self.details_view.setPlainText(runlog.step_details(item.params, result))
        self.details.setVisible(True)

    def _render_result(self):
        last = self.last
        self.details.setVisible(False)
        self.open_folder_button.setText(self.t("open_folder"))
        if self.focus is not None:
            self._render_focus()
            return
        self._shown_result = last.get("result") if last and last["kind"] == runlog.DONE else None
        folder = runlog.result_folder(self._shown_result)
        self.open_folder_button.setVisible(folder is not None)
        if last is None:
            self.result_text.setText(self.t("result_none"))
            return
        kind = last["kind"]
        command = last.get("command", "")
        if kind == runlog.RUNNING:
            self.result_text.setText("%s  %s" % (self.t("running"), command))
        elif last.get("plan"):
            lines = [agentview.outcome_text(last, self.lang)]
            if kind == runlog.DONE:
                for step in last.get("steps") or []:
                    lines.append(
                        "%d. %s"
                        % (
                            step.number,
                            self._sentence(step.command, step.params, step.result, last.get("refs")),
                        )
                    )
            self.result_text.setText("\n".join(lines))
            if kind == runlog.DONE:
                self.details_view.setPlainText(runlog.technical_details(last.get("result")))
                self.details.setVisible(True)
        elif kind == runlog.DONE:
            self.result_text.setText(
                self._sentence(command, last.get("params"), last.get("result"))
            )
            self.details_view.setPlainText(
                runlog.step_details(last.get("params"), last.get("result"))
            )
            self.details.setVisible(True)
        elif kind == runlog.CANCELLED:
            self.result_text.setText(self.t("cancelled_generic", command=command))
        else:
            self.result_text.setText(
                self.t("step_error", n=1, code=last.get("code", ""), message=last.get("message", ""))
            )

    def _render_agent(self):
        """Saisie et zone plan, d'après l'état du plan (logique dans `agentview`)."""
        t = self.t
        agent = self.agent
        c = agentview.controls(
            agent,
            self.ready and self.banner is None,
            self.running,
            bool(self.input.toPlainText().strip()),
            self.catalogue,
            external.busy(self.external),
        )
        self.input.setEnabled(c["input_enabled"])
        self.send_button.setEnabled(c["send_enabled"])
        reason = t(c["input_reason"]) if c["input_reason"] else ""
        self.input_reason.setText(reason)
        self.input_reason.setVisible(bool(reason))
        self.send_button.setAccessibleName("%s, %s" % (t("send"), reason) if reason else t("send"))

        notice = agentview.notice_text(agent, self.lang)
        if agent.notice is not None and agent.notice[0] == "question":
            notice = "%s %s" % (t("question_title"), notice)
        self.plan_notice.setText(notice or "")
        self.plan_notice.setVisible(bool(notice))
        self.plan_progress.setVisible(c["progress_visible"])
        self.plan_progress.setAccessibleName(t("generating"))

        has_steps = bool(agent.steps)
        self.plan_title.setText(t("plan_title"))
        self.plan_title.setVisible(
            has_steps and agent.phase in (agentview.PROPOSED, agentview.EXECUTING)
        )
        self.plan_list.setVisible(has_steps)
        self.plan_list.setAccessibleName(t("plan_steps_name"))
        self.plan_list.clear()
        for step in agent.steps:
            self.plan_list.addItem(
                agentview.step_line(step, self.catalogue, self.lang, agent.current)
            )
        self.plan_details.setVisible(has_steps)
        self.plan_details.button.setText(t("details"))
        self.plan_details.button.setAccessibleName(t("details"))
        self.plan_details_view.setPlainText(
            runlog.technical_details(agent.plan) if has_steps else ""
        )

        self.approve_button.setText(t("approve"))
        self.approve_button.setAccessibleName(t("approve"))
        self.approve_button.setVisible(c["approve_visible"])
        self.approve_button.setEnabled(c["approve_enabled"])
        self.reject_button.setText(t("reject"))
        self.reject_button.setAccessibleName(t("reject"))
        self.reject_button.setVisible(c["reject_visible"])
        self.reject_button.setEnabled(c["reject_enabled"])
        stop_reason = t(c["stop_reason"]) if c["stop_reason"] else ""
        self.plan_stop_button.setText(t("stop"))
        self.plan_stop_button.setAccessibleName(
            "%s, %s" % (t("stop"), stop_reason) if stop_reason else t("stop")
        )
        self.plan_stop_button.setVisible(c["stop_visible"])
        self.plan_stop_button.setEnabled(c["stop_enabled"])
        self.plan_reason.setText(stop_reason)
        self.plan_reason.setVisible(bool(stop_reason))
        self.plan_zone.setVisible(
            bool(notice) or has_steps or c["progress_visible"] or c["approve_visible"]
        )

    def _input_changed(self):
        """À chaque frappe, seul l'état d'Envoyer change : la zone plan n'est pas refaite."""
        c = agentview.controls(
            self.agent,
            self.ready and self.banner is None,
            self.running,
            bool(self.input.toPlainText().strip()),
            self.catalogue,
            external.busy(self.external),
        )
        self.send_button.setEnabled(c["send_enabled"])

    def _render_commands(self):
        current = self.selected["id"] if self.selected else None
        self.command_list.blockSignals(True)
        self.command_list.clear()
        self.command_list.setAccessibleName(self.t("commands_name"))
        usable = self.ready and self.banner is None
        for entry in self.catalogue:
            description = texts.pick(
                self.lang, entry.get("description_fr", ""), entry.get("description_en", "")
            )
            available = entry.get("available", False)
            label = "%s : %s" % (entry["id"], description)
            if not available:
                label += " (%s)" % self.t("unavailable", reason=self.t("unavailable_reason"))
            item = QListWidgetItem(label)
            item.setData(Qt.ItemDataRole.UserRole, entry["id"])
            if not available or not usable:
                item.setFlags(item.flags() & ~Qt.ItemFlag.ItemIsEnabled)
            self.command_list.addItem(item)
            if entry["id"] == current:
                self.command_list.setCurrentItem(item)
        self.command_list.setEnabled(usable and not self.running)
        self.command_list.blockSignals(False)
        if self.selected is None:
            self._build_form(None)

    def _render_buttons(self):
        entry = self.selected
        c = runlog.controls(
            self.ready and self.banner is None,
            bool(entry and entry.get("available")),
            self.running,
            bool(entry and entry.get("cancellable")),
            external.busy(self.external),
        )
        self.run_button.setText(self.t("run"))
        self.stop_button.setText(self.t("stop"))
        self.run_button.setVisible(c["run_visible"])
        self.run_button.setEnabled(c["run_enabled"] and entry is not None)
        self.stop_button.setVisible(c["stop_visible"])
        self.stop_button.setEnabled(c["stop_enabled"])
        reason = ""
        if self.running and c["stop_reason"]:
            reason = self.t(c["stop_reason"])
        elif self.running:
            reason = self.t("busy")
        elif external.busy(self.external):
            reason = self.t("input_reason_external")
        elif entry is not None and not entry.get("available"):
            reason = self.t("unavailable", reason=self.t("unavailable_reason"))
        self.reason.setText(reason)
        self.reason.setVisible(bool(reason))
        self.run_button.setAccessibleName(self.t("run"))
        self.stop_button.setAccessibleName(
            "%s, %s" % (self.t("stop"), reason) if reason and c["stop_visible"] else self.t("stop")
        )
        self.form_zone.setEnabled(not self.running)

    # -- formulaire ----------------------------------------------------
    def _build_form(self, entry):
        while self.form_layout.rowCount():
            self.form_layout.removeRow(0)
        self._fields = []
        self._inputs = {}
        self.form_error.setText("")
        if entry is None:
            self.form_hint.setText(self.t("pick_command"))
            return
        self._fields = form.build_fields(entry.get("params_schema"))
        self.form_hint.setText("" if self._fields else self.t("no_params"))
        for f in self._fields:
            label = f.name if f.kind != form.JSON else self.t("raw_json", name=f.name)
            if f.kind == form.BOOLEAN:
                widget = QCheckBox()
            elif form.is_layer_field(f):
                widget = LayerCombo()
            elif form.is_field_field(f) and any(g.name == "layer_id" for g in self._fields):
                widget = FieldCombo(optional=not f.required, empty_text=self.t("optional_empty"))
            elif f.kind == form.ENUM:
                widget = QComboBox()
                if not f.required:
                    widget.addItem(self.t("optional_empty"), "")
                for choice in f.choices:
                    widget.addItem(choice, choice)
            else:
                widget = QLineEdit()
            row = widget
            if form.is_path_field(f):
                row = QWidget()
                rl = QHBoxLayout(row)
                rl.setContentsMargins(0, 0, 0, 0)
                browse = QPushButton(self.t("browse"))
                browse.setAccessibleName(self.t("browse"))
                browse.clicked.connect(lambda _=False, w=widget: self._browse(w))
                rl.addWidget(widget, 1)
                rl.addWidget(browse)
            widget.setAccessibleName(label)
            if f.description:
                widget.setAccessibleDescription(f.description)
            self._inputs[f.name] = widget
            self.form_layout.addRow(label, row)
        layers = self._inputs.get("layer_id")
        for widget in self._inputs.values():
            if isinstance(widget, FieldCombo) and layers is not None:
                widget.bind(layers)

    def _browse(self, widget):
        if not self.project_file:
            return
        start = os.path.dirname(self.project_file)
        chosen, _ = QFileDialog.getOpenFileName(self, self.t("browse_title"), start)
        if not chosen:
            return
        rel = form.relative_path(chosen, self.project_file)
        if rel is None:
            self.form_error.setText(self.t("browse_other_drive"))
            return
        self.form_error.setText("")
        widget.setText(rel)

    def _values(self):
        values = {}
        for f in self._fields:
            widget = self._inputs[f.name]
            if f.kind == form.BOOLEAN:
                values[f.name] = widget.isChecked()
            elif f.kind == form.ENUM or form.is_layer_field(f) or isinstance(widget, FieldCombo):
                values[f.name] = widget.currentData() or ""
            else:
                values[f.name] = widget.text()
        return values

    # -- gestes de l'utilisateur ---------------------------------------
    def _command_changed(self, item, _previous=None):
        command = item.data(Qt.ItemDataRole.UserRole) if item is not None else None
        self.selected = next((e for e in self.catalogue if e["id"] == command), None)
        self._build_form(self.selected)
        self._render_buttons()

    def _run(self):
        if self.selected is None or self.running:
            return
        try:
            params = form.build_params(self._fields, self._values())
        except form.FormError as exc:
            self.form_error.setText(self.t(exc.key, name=exc.name))
            return  # rien n'est envoyé
        self.form_error.setText("")
        self._run_params = params  # gardés pour la ligne du journal (RUN_STARTED)
        self._actions.run(self.selected["id"], params)
        self._run_params = None

    def _stop(self):
        self._actions.stop()

    def _send(self):
        text = self.input.toPlainText().strip()
        c = agentview.controls(
            self.agent,
            self.ready and self.banner is None,
            self.running,
            bool(text),
            self.catalogue,
            external.busy(self.external),
        )
        if not c["send_enabled"]:
            return
        if self._actions.propose(text, self.model):  # la demande part sans les blancs des bords
            self._last_request = text
            self.agent.generating()
            self.input.clear()
            self.focus = None  # l'étape cliquée n'existe plus
            self.render()
            _announce(self.plan_notice, self.t("generating"))

    def _approve(self):
        plan_id = self.agent.plan_id
        if self.agent.phase != agentview.PROPOSED or plan_id is None:
            return
        if self._actions.approve(plan_id):  # seul le plan_id repart (AD-5, AD-23)
            self.agent.executing()
            self.focus = None
            self.render()

    def _reject(self):
        plan_id = self.agent.plan_id
        if self.agent.phase != agentview.PROPOSED or plan_id is None:
            return
        if self._actions.reject(plan_id):
            self.agent.start_rejecting()
            self.render()

    def _banner_action(self):
        action = BANNERS.get(self.banner, (None, None))[1]
        if action == "choose":
            self._actions.choose_core()
        elif action == "relaunch":
            self._actions.relaunch()

    def _render_models(self):
        """Liste des modèles des Réglages : grisée avec sa raison si rien ne se choisit."""
        t = self.t
        self.model_box.setAccessibleName(t("settings_model"))
        shown = (self.lang, repr(self.models), self.model)
        if shown != self._models_shown:  # ne refaire la liste que si elle a changé
            self._fill_models()
            self._models_shown = shown
        reason = models.reason(self.lang, self.models, self.models_error)
        usable = self.ready and self.banner is None
        self.model_box.setEnabled(
            usable and reason is None and bool(models.usable(self.models or []))
        )
        self.model_reason.setText(reason or "")
        self.model_reason.setVisible(bool(reason))
        self.model_box.setAccessibleDescription(reason or "")

    def _fill_models(self):
        self._filling_models = True
        try:
            self.model_box.clear()
            for entry in self.models or []:
                self.model_box.addItem(models.label(self.lang, entry), entry["file"])
                if entry["problem"] is not None:
                    item = self.model_box.model().item(self.model_box.count() - 1)
                    if item is not None:
                        item.setEnabled(False)  # grisée, la raison dans son libellé
            index = self.model_box.findData(self.model) if self.model else -1
            if index >= 0:
                self.model_box.setCurrentIndex(index)
        finally:
            self._filling_models = False

    def _model_changed(self, index):
        if self._filling_models or index < 0:
            return
        name = self.model_box.itemData(index)
        if not isinstance(name, str) or name not in models.usable(self.models or []):
            return
        self.model = name
        QSettings().setValue(models.SETTING_KEY, name)  # le choix survit à QGIS

    def _render_external(self):
        """Réglages, bandeau et question de l'IA externe (logique dans `external`)."""
        t = self.t
        state = self.external
        c = external.controls(state, self.ready and self.banner is None, self.catalogue)

        self.external_switch.setText(t("external_switch"))
        self.external_switch.setAccessibleName(t("external_switch"))
        self.external_switch.setChecked(c["switch_checked"])  # l'état répondu par le noyau
        self.external_switch.setEnabled(c["switch_enabled"])
        self.external_ask.setText(t("external_ask"))
        self.external_ask.setAccessibleName(t("external_ask"))
        self.external_ask.setChecked(c["ask_checked"])
        if c["switch_reason"]:
            reason = t(c["switch_reason"])
        elif c["switch_error"]:
            code, message = c["switch_error"]
            reason = t("external_error", code=code, message=runlog.redact(message))
        else:
            reason = ""
        self.external_reason.setText(reason)
        self.external_reason.setVisible(bool(reason))
        self.external_switch.setAccessibleDescription(reason)

        self.external_text.setText(t("external_banner"))
        self.external_text.setAccessibleName(t("external_banner"))
        self.external_cut.setText(t("external_cut"))
        self.external_cut.setAccessibleName(t("external_cut"))
        self.external_cut.setEnabled(c["cut_enabled"])
        stop_reason = t(c["stop_reason"]) if c["stop_reason"] else ""
        self.external_stop.setText(t("stop"))
        self.external_stop.setAccessibleName(
            "%s, %s" % (t("stop"), stop_reason) if stop_reason else t("stop")
        )
        self.external_stop.setVisible(c["stop_visible"])
        self.external_stop.setEnabled(c["stop_enabled"])
        self.external_stop_reason.setText(stop_reason)
        self.external_stop_reason.setVisible(bool(stop_reason))
        self.external_zone.setVisible(c["banner_visible"] or c["stop_visible"])

        question = external.question_text(state, self.catalogue, self.lang)
        self.approval_text.setText(question or "")
        self.approval_allow.setText(t("approval_allow"))
        self.approval_allow.setAccessibleName(t("approval_allow"))
        self.approval_allow.setEnabled(c["answer_enabled"])
        self.approval_deny.setText(t("approval_deny"))
        self.approval_deny.setAccessibleName(t("approval_deny"))
        self.approval_deny.setEnabled(c["answer_enabled"])
        self.approval_zone.setVisible(c["question_visible"])

    def _switch_clicked(self, checked):
        """L'interrupteur ne change qu'à la réponse du noyau : on le remet à l'état connu."""
        self._actions.set_external(bool(checked), self.external.ask)
        self.render()

    def _ask_clicked(self, checked):
        self.external.ask = bool(checked)
        try:
            QSettings().setValue(EXTERNAL_ASK_KEY, "true" if checked else "false")
        except Exception:
            pass
        if self.external.enabled:  # la case ne vaut que pour les appels suivants
            self._actions.set_external(True, self.external.ask)
        self.render()

    def _cut_external(self):
        self._actions.set_external(False, self.external.ask)

    def _stop_external(self):
        running = self.external.running_call()
        if running is not None:
            self._actions.stop_external(running[0])  # request.cancel {id: call_id}

    def _answer(self, approved):
        approval = self.external.approval
        if approval is None:
            return  # question déjà retirée : rien n'est envoyé
        if self._actions.answer_approval(approval["request_id"], approved):
            self.external.answered()
        else:
            self.external.approval_withdrawn(approval["request_id"])
        self.render()

    def _open_settings(self):
        self.external.settings_opened()  # un canal pris redevient cliquable
        self.render()
        self._actions.list_models()  # liste fraîche si le client est libre
        self.settings_zone.setWindowTitle(self.t("settings"))
        self.settings_zone.exec()
        self.settings_button.setFocus()

    def _expert_toggled(self, opened):
        try:
            QSettings().setValue(EXPERT_KEY, "true" if opened else "false")
        except Exception:
            pass

    def _lang_changed(self, _index):
        self.lang = texts.normalize(str(self.lang_box.currentData()))
        QSettings().setValue(LANG_KEY, self.lang)
        self.render()  # prend effet tout de suite

    def keyPressEvent(self, event):
        # Échap referme le repli du mode expert (la fenêtre Réglages gère le sien).
        if event.key() == Qt.Key.Key_Escape:
            if self.expert.is_open():
                self.expert.set_open(False)
                self.expert.button.setFocus()
                return
        super().keyPressEvent(event)
