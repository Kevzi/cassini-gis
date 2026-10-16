"""Widgets du panneau Cassini [9.4]. Qt uniquement par `qgis.PyQt` (AD-18).

Aucune logique métier (AD-4) : le panneau affiche, demande et rend la main à
l'extension. Le noyau valide les paramètres et décide de tout. Les textes, le
formulaire par schéma et le journal sont dans des modules sans Qt, testés seuls.
"""

import os
import time

from qgis.PyQt.QtCore import QSettings, Qt
from qgis.PyQt.QtGui import QFontDatabase, QIcon
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
    QPushButton,
    QScrollArea,
    QToolButton,
    QVBoxLayout,
    QWidget,
)

from . import client, form, runlog, texts


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


LANG_KEY = "cassini/lang"
EXPERT_KEY = "cassini/expert_open"
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
        self._actions = actions  # run(cmd, params), stop(), choose_core(), relaunch()
        settings = QSettings()
        self.lang = texts.normalize(str(settings.value(LANG_KEY, texts.DEFAULT_LANG)))
        self.log = runlog.RunLog()
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

        # Réglages : langue, modèle grisé.
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
        self.model_label.setEnabled(False)
        self.model_label.setWordWrap(True)
        sl.addWidget(self.lang_label)
        sl.addWidget(self.lang_box)
        sl.addWidget(self.model_label)

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

        # Saisie principale : visible mais grisée jusqu'à l'agent.
        self.input = QPlainTextEdit()
        self.input.setEnabled(False)
        self.input.setMaximumHeight(72)
        self.input_reason = QLabel()
        self.input_reason.setWordWrap(True)
        self.send_button = QPushButton()
        self.send_button.setEnabled(False)
        col.addWidget(self.input)
        col.addWidget(self.input_reason)
        col.addWidget(self.send_button)

        # Résultat.
        self.result_title = _title(QLabel())
        self.result_text = QLabel()
        self.result_text.setWordWrap(True)
        self.result_text.setTextInteractionFlags(
            Qt.TextInteractionFlag.TextSelectableByMouse
        )
        self.details = _Fold()
        self.details_view = _mono(QPlainTextEdit())
        self.details_view.setReadOnly(True)
        self.details_view.setMaximumHeight(120)
        dl = QVBoxLayout(self.details.body)
        dl.setContentsMargins(0, 0, 0, 0)
        dl.addWidget(self.details_view)
        col.addWidget(self.result_title)
        col.addWidget(self.result_text)
        col.addWidget(self.details)

        # Journal de la session.
        self.log_title = _title(QLabel())
        self.log_list = QListWidget()
        self.log_list.setMaximumHeight(110)
        self.log_list.setWordWrap(True)
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
            self.settings_button, self.lang_box, self.banner_button, self.input,
            self.send_button, self.details.button, self.log_list,
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
        if kind is not None:
            self.ready = False
            self._drop_stale_run()
        self.render()
        if kind is not None:
            _announce(self.banner_text, self.t(BANNERS[kind][0]))

    def _drop_stale_run(self):
        """Le noyau a été fermé : une commande en vol n'existe plus."""
        if self.running:
            self.running = False
            self.log.finish(runlog.CANCELLED)

    def set_ready(self, catalogue):
        self._drop_stale_run()
        self.ready = True
        self.banner = None
        self.catalogue = list(catalogue)
        self.selected = None
        self.render()
        _announce(self.title, self.t("live_ready"))

    def on_event(self, kind, payload):
        """Événements RUN_* du client."""
        if kind == client.RUN_STARTED:
            self.running = True
            self.log.start(payload["command"], payload["command"], time.strftime("%H:%M"))
            self.last = {"kind": runlog.RUNNING, "command": payload["command"]}
        elif kind == client.RUN_DONE:
            self.running = False
            self.log.finish(runlog.DONE)
            self.last = {"kind": runlog.DONE, **payload}
            _announce(self.result_text, self.t("live_done"))
        elif kind == client.RUN_CANCELLED:
            self.running = False
            self.log.finish(runlog.CANCELLED)
            self.last = {"kind": runlog.CANCELLED, **payload}
            _announce(self.result_text, self.t("live_cancelled"))
        elif kind == client.RUN_FAILED:
            self.running = False
            self.log.finish(runlog.ERROR)
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

        self.input.setPlaceholderText(t("input_hint"))
        self.input.setAccessibleName(t("input_name"))
        self.input_reason.setText(t("input_reason"))
        self.send_button.setText(t("send"))
        self.send_button.setAccessibleName("%s, %s" % (t("send"), t("input_reason")))

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
            self.log_list.addItem(runlog.line_text(entry, t(entry.status)))
        self.log_list.setAccessibleName(t("log_title"))
        self.log_list.scrollToBottom()

        self.expert.button.setText(t("expert_title"))
        self.expert.button.setAccessibleName(t("expert_title"))
        self.expert.setEnabled(usable)
        self.footer.setText(t("offline"))
        self._render_commands()
        self._render_buttons()

    def _render_result(self):
        last = self.last
        self.details.setVisible(False)
        if last is None:
            self.result_text.setText(self.t("result_none"))
            return
        kind = last["kind"]
        command = last.get("command", "")
        if kind == runlog.RUNNING:
            self.result_text.setText("%s  %s" % (self.t("running"), command))
        elif kind == runlog.DONE:
            if command == "system.about":
                self.result_text.setText(self.t("about_result"))
            else:
                self.result_text.setText(self.t("done_generic", command=command))
            self.details_view.setPlainText(runlog.technical_details(last.get("result")))
            self.details.setVisible(True)
        elif kind == runlog.CANCELLED:
            self.result_text.setText(self.t("cancelled_generic", command=command))
        else:
            self.result_text.setText(
                self.t("step_error", n=1, code=last.get("code", ""), message=last.get("message", ""))
            )

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
        self._actions.run(self.selected["id"], params)

    def _stop(self):
        self._actions.stop()

    def _banner_action(self):
        action = BANNERS.get(self.banner, (None, None))[1]
        if action == "choose":
            self._actions.choose_core()
        elif action == "relaunch":
            self._actions.relaunch()

    def _open_settings(self):
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
