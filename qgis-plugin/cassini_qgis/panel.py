"""Widgets du panneau Cassini [9.4]. Qt uniquement par `qgis.PyQt` (AD-18).

Aucune logique métier (AD-4) : le panneau affiche, demande et rend la main à
l'extension. Le noyau valide les paramètres et décide de tout. Les textes, le
formulaire par schéma et le journal sont dans des modules sans Qt, testés seuls.
"""

import os
import time

from qgis.PyQt.QtCore import QEvent, QRect, QSettings, Qt, QTimer, QUrl
from qgis.PyQt.QtGui import QDesktopServices, QFontDatabase, QIcon, QPalette, QTextCursor
from qgis.PyQt.QtWidgets import (
    QApplication,
    QCheckBox,
    QComboBox,
    QDialog,
    QFileDialog,
    QFormLayout,
    QFrame,
    QHBoxLayout,
    QLabel,
    QLineEdit,
    QListWidget,
    QListWidgetItem,
    QPlainTextEdit,
    QProgressBar,
    QPushButton,
    QScrollArea,
    QSizePolicy,
    QToolButton,
    QVBoxLayout,
    QWidget,
)

from . import (
    agentview,
    client,
    external,
    form,
    layer_remove,
    models,
    runlog,
    summary,
    texts,
    thread,
)


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
    # Seconde fenêtre QGIS sur le même projet : noyau vivant, seul le rejeu reste (AD-11).
    # Relancer : la session reprend l'écriture quand la première fenêtre a lâché le journal.
    "read_only": ("read_only", "relaunch"),
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


def _live_layer_name(layer_id):
    """Nom de la couche dans le projet QGIS, ou None ; jamais d'exception."""
    try:
        from qgis.core import QgsProject

        layer = QgsProject.instance().mapLayer(layer_id)
        return layer.name() if layer is not None else None
    except Exception:
        return None


def _layer_name(layer_id):
    """Nom de la couche dans le projet QGIS, ou None ; jamais d'exception. Une couche retirée
    du projet par `layer.remove` garde le nom qu'elle avait, pour l'affichage seulement."""
    return layer_remove.display_name(layer_id, _live_layer_name)


def _title(text_label):
    font = text_label.font()
    font.setBold(True)
    text_label.setFont(font)
    return text_label


def _mono(widget):
    widget.setFont(QFontDatabase.systemFont(QFontDatabase.SystemFont.FixedFont))
    return widget


def _wrap(label, plain=False, selectable=True):
    """Libellé qui passe à la ligne ; `plain` : texte venu d'ailleurs, jamais interprété."""
    label.setWordWrap(True)
    if plain:
        label.setTextFormat(Qt.TextFormat.PlainText)
    if selectable:
        label.setTextInteractionFlags(Qt.TextInteractionFlag.TextSelectableByMouse)
    return label


def _hline():
    """Filet du thème."""
    line = QFrame()
    line.setFrameShape(QFrame.Shape.HLine)
    line.setFrameShadow(QFrame.Shadow.Sunken)
    return line


def _card(name="cassiniCard"):
    """Carte du fil : un simple cadre stylé (DESIGN.md, `card`), qui grandit avec son contenu."""
    frame = QFrame()
    frame.setObjectName(name)
    layout = QVBoxLayout(frame)
    layout.setContentsMargins(8, 8, 8, 8)
    layout.setSpacing(6)
    return frame, layout


def _clear(layout):
    while layout.count():
        item = layout.takeAt(0)
        widget = item.widget()
        if widget is not None:
            widget.setParent(None)
            widget.deleteLater()


class _StepRow(QFrame):
    """Une étape : icône, mot d'état en demi-gras, libellé, et son message en erreur.

    L'étape en cours porte le marqueur (propriété `current`, stylée par la feuille de la
    carte). Un clic montre son détail.

    `parent` : la carte qui la reçoit. Créés sans parent, la ligne et ses libellés
    résolvaient leur police pour l'écran principal (96 ppp) et la gardaient une fois posés
    dans la carte (Qt ne compare pas la résolution des polices) ; peints à 144 ppp sur un
    écran mis à l'échelle (150 %), ils débordaient de leur taille mesurée : mot d'état
    tronqué, texte rogné en hauteur (défaut de 12.1 relevé en 12.2).
    """

    def __init__(self, icon, word, text, message="", current=False, on_click=None, parent=None):
        super().__init__(parent)
        self.setObjectName("cassiniStep")
        self.setProperty("current", bool(current))
        self._on_click = on_click
        row = QHBoxLayout(self)
        row.setContentsMargins(4, 3, 4, 3)
        row.setSpacing(6)
        top = Qt.AlignmentFlag.AlignTop
        # Libellés créés dans la ligne : leur police est résolue pour l'écran de la carte.
        row.addWidget(QLabel(icon, self), 0, top)
        row.addWidget(_title(QLabel(word, self)), 0, top)
        col = QVBoxLayout()
        col.setContentsMargins(0, 0, 0, 0)
        col.setSpacing(2)
        col.addWidget(_wrap(QLabel(text, self), plain=True, selectable=False))
        if message:
            col.addWidget(_wrap(QLabel(message, self), plain=True, selectable=False))
        row.addLayout(col, 1)
        self.setAccessibleName(" ".join(p for p in (word, text, message) if p))
        if on_click is not None:  # au clavier aussi : Tab, puis Entrée ou Espace
            self.setFocusPolicy(Qt.FocusPolicy.StrongFocus)

    def mousePressEvent(self, event):
        if self._on_click is not None:
            self._on_click()
        super().mousePressEvent(event)

    def keyPressEvent(self, event):
        keys = (Qt.Key.Key_Return, Qt.Key.Key_Enter, Qt.Key.Key_Space)
        if self._on_click is not None and event.key() in keys:
            self._on_click()
            return
        super().keyPressEvent(event)


class _Fold(QWidget):
    """Groupe repliable natif : une flèche à bascule et un contenu.

    `wide` : ligne de repli pleine largeur (Journal, Mode expert).
    """

    def __init__(self, on_toggle=None, wide=False):
        super().__init__()
        self._on_toggle = on_toggle
        self.button = QToolButton()
        self.button.setCheckable(True)
        self.button.setToolButtonStyle(Qt.ToolButtonStyle.ToolButtonTextBesideIcon)
        if wide:
            self.button.setSizePolicy(QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Fixed)
            # Jamais plus haut que son contenu : la place libre revient au fil.
            self.setSizePolicy(QSizePolicy.Policy.Preferred, QSizePolicy.Policy.Maximum)
        else:
            self.button.setAutoRaise(True)  # repli plat dans une carte (« Détails techniques »)
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
        # set_external(enabled, ask), stop_external(call_id), answer_approval(request_id, approved),
        # replay()
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
        self.banner_message = None  # texte du noyau à montrer tel quel dans le bandeau
        self.running = False
        self.last = None  # {"kind", "command", "result"/"message"/"code"}
        self.selected = None  # entrée du catalogue choisie
        self._fields = []
        self._inputs = {}
        self.project_file = None  # fichier du projet ouvert, pour « Parcourir… »
        # Le fil [12.1] : l'échange en cours (bulle) et ce que le panneau en montre.
        self._exchange = None  # `thread.request_exchange` / `thread.expert_exchange`
        self._plan_open = False  # « Voir » sur la carte du plan validé
        self._kinds_shown = None  # cartes affichées, pour caler le fil sur la plus récente
        self._cards = []  # cadres stylés en `card`
        self._small_logos = []  # logos 16 px des titres de carte
        self._metas = []  # libellés en `meta` (couleur atténuée du thème)
        self._styles = None  # feuilles de style de la paire claire ou sombre
        self._primary_shown = None
        self._styling = False
        # Guidage [12.2].
        self._cues = []  # lignes « À vous »
        self._titles = []  # titres de carte qui prennent le focus
        self._rephrase_buttons = []  # (bouton, ligne meta) de chaque carte
        self._rephrased = False  # la phrase est revenue dans la saisie : bouton retiré
        self._rephrase_note = None  # clé du texte quand rien n'a pu être remis
        self._focus_target = None  # dernière cible du focus appliquée (une fois par état)
        # Rejeu du journal [12.3] : issue affichée dans le repli Journal (`thread.replay_*`).
        self.replay = None
        # Le bouton du rejeu avait le focus au départ : (widget où Qt l'a mis en grisant le
        # bouton,) ; None sinon. Le verdict ne le rend que s'il n'a pas bougé depuis.
        self._replay_focus = None
        self._build()
        expert = str(settings.value(EXPERT_KEY, "false")).lower() in ("true", "1")
        self.expert.set_open(expert)
        self.render()

    # -- construction --------------------------------------------------
    def _dark(self):
        return thread.is_dark(self.palette().window().color().lightness())

    def _logo(self, size=24):
        name = "logo-cassini-dark.svg" if self._dark() else "logo-cassini-light.svg"
        return QIcon(os.path.join(ASSETS, name)).pixmap(size, size)

    def _meta(self, label):
        """Détail secondaire (`meta`) : un cran sous le corps, couleur atténuée du thème."""
        font = label.font()
        if font.pointSizeF() > 1:
            font.setPointSizeF(font.pointSizeF() - 1)
        label.setFont(font)
        self._metas.append(label)
        return label

    def _new_card(self):
        frame, layout = _card()
        self._cards.append(frame)
        return frame, layout

    def _card_title(self, layout):
        """Titre de carte précédé du petit logo 16 px ; rend le libellé du titre."""
        row = QHBoxLayout()
        row.setSpacing(6)
        icon = QLabel()
        self._small_logos.append(icon)
        label = _title(_wrap(QLabel()))
        row.addWidget(icon, 0, Qt.AlignmentFlag.AlignTop)
        row.addWidget(label, 1)
        layout.addLayout(row)
        return label

    def _focusable(self, label):
        """Titre de carte qui reçoit le focus du guidage [12.2] (et entre dans la tabulation)."""
        label.setObjectName("cassiniCardTitle")
        label.setFocusPolicy(Qt.FocusPolicy.TabFocus)
        self._titles.append(label)
        return label

    def _new_cue(self, layout):
        """Ligne « À vous » (`act-cue`), cachée tant qu'il n'y a rien à faire."""
        cue = _wrap(QLabel(), plain=True, selectable=False)
        cue.setObjectName("cassiniCue")
        cue.setVisible(False)
        self._cues.append(cue)
        layout.addWidget(cue)
        return cue

    def _new_rephrase(self, layout):
        """« Reformuler la demande » et sa ligne `meta` ; rend (bouton, ligne)."""
        button = QPushButton()
        button.clicked.connect(self._rephrase)
        button.setVisible(False)
        detail = self._meta(_wrap(QLabel(), selectable=False))
        detail.setVisible(False)
        row = QHBoxLayout()
        row.addWidget(button, 0, Qt.AlignmentFlag.AlignTop)
        row.addStretch(1)
        layout.addLayout(row)
        layout.addWidget(detail)
        self._rephrase_buttons.append((button, detail))
        return button, detail

    def _build(self):
        col = QVBoxLayout(self)
        col.setContentsMargins(8, 8, 8, 8)
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

        # Le fil [12.1] : seul à défiler, il prend toute la hauteur libre. Cartes dans
        # l'ordre où elles répondent à la demande (EXPERIENCE.md, Information Architecture).
        self.thread_scroll = QScrollArea()
        self.thread_scroll.setWidgetResizable(True)
        self.thread_scroll.setFrameShape(QFrame.Shape.NoFrame)
        self.thread_scroll.setHorizontalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAlwaysOff)
        self.thread_body = QWidget()
        self.thread_body.setAutoFillBackground(False)
        self.thread_scroll.setWidget(self.thread_body)
        self.thread_scroll.viewport().setAutoFillBackground(False)
        fil = QVBoxLayout(self.thread_body)
        fil.setContentsMargins(0, 0, 0, 0)
        fil.setSpacing(8)
        col.addWidget(self.thread_scroll, 1)

        # Carte d'accueil : seule dans le fil au repos.
        self.welcome_card, wl = self._new_card()
        self.welcome_title = self._card_title(wl)
        self.welcome_text = _wrap(QLabel())
        self.examples_title = _title(_wrap(QLabel()))
        self.examples = _wrap(QLabel())
        self.examples.setTextFormat(Qt.TextFormat.RichText)
        wl.addWidget(self.welcome_text)
        wl.addWidget(self.examples_title)
        wl.addWidget(self.examples)
        fil.addWidget(self.welcome_card)

        # Bulle de la demande, à droite, avec « Vous · heure » au-dessus.
        self.bubble_zone = QWidget()
        bz = QVBoxLayout(self.bubble_zone)
        bz.setContentsMargins(0, 0, 0, 0)
        bz.setSpacing(2)
        self.bubble_meta = self._meta(QLabel())
        self.bubble_meta.setAlignment(Qt.AlignmentFlag.AlignRight)
        self.bubble = QFrame()
        self.bubble.setObjectName("cassiniBubble")
        bbl = QVBoxLayout(self.bubble)
        bbl.setContentsMargins(8, 8, 8, 8)
        self.bubble_text = _wrap(QLabel(), plain=True)  # la phrase exacte de l'utilisateur
        bbl.addWidget(self.bubble_text)
        brow = QHBoxLayout()
        brow.setContentsMargins(0, 0, 0, 0)
        brow.addStretch(1)
        brow.addWidget(self.bubble)
        bz.addWidget(self.bubble_meta)
        bz.addLayout(brow)
        fil.addWidget(self.bubble_zone)

        # Carte de composition : « Je prépare un plan… », indicateur, Arrêter.
        self.composing_card, cl = self._new_card()
        self.composing_title = self._focusable(self._card_title(cl))
        self.plan_progress = QProgressBar()
        self.plan_progress.setRange(0, 0)  # indéterminée : la composition ne se mesure pas
        self.plan_progress.setTextVisible(False)
        self.plan_progress.setMaximumHeight(6)
        self.composing_detail = self._meta(_wrap(QLabel()))
        self.compose_stop = QPushButton()
        self.compose_stop.setEnabled(False)  # la composition n'est pas annulable (client)
        self.compose_stop_reason = self._meta(_wrap(QLabel()))
        crow = QHBoxLayout()
        crow.addWidget(self.compose_stop_reason, 1)
        crow.addWidget(self.compose_stop, 0, Qt.AlignmentFlag.AlignTop)
        cl.addWidget(self.plan_progress)
        cl.addWidget(self.composing_detail)
        cl.addLayout(crow)
        fil.addWidget(self.composing_card)

        # Carte d'échec de composition : message du noyau, code sous Détails techniques.
        self.failed_card, fl = self._new_card()
        self.failed_title = self._card_title(fl)
        self.failed_text = _wrap(QLabel(), plain=True)
        self.failed_details = _Fold()
        self.failed_details_view = _mono(_wrap(QLabel(), plain=True))
        fdl = QVBoxLayout(self.failed_details.body)
        fdl.setContentsMargins(0, 0, 0, 0)
        fdl.addWidget(self.failed_details_view)
        fl.addWidget(self.failed_text)
        self.failed_cue = self._new_cue(fl)
        self.failed_rephrase, self.failed_rephrase_detail = self._new_rephrase(fl)
        fl.addWidget(self.failed_details)
        fil.addWidget(self.failed_card)

        # Carte de question : la question de Cassini en clair.
        self.question_card, ql = self._new_card()
        self.question_title = self._card_title(ql)
        self.question_text = _wrap(QLabel(), plain=True)
        self.question_left_out = self._meta(_wrap(QLabel()))
        ql.addWidget(self.question_text)
        ql.addWidget(self.question_left_out)
        self.question_cue = self._new_cue(ql)
        fil.addWidget(self.question_card)

        # Carte de refus.
        self.rejected_card, rl = self._new_card()
        self.rejected_title = self._card_title(rl)
        self.rejected_cue = self._new_cue(rl)
        self.rejected_rephrase, self.rejected_rephrase_detail = self._new_rephrase(rl)
        fil.addWidget(self.rejected_card)

        # Carte du plan validé, repliée : « ✔ Plan validé — n étapes », « Voir ».
        self.plan_folded_card, pfl = self._new_card()
        pfrow = QHBoxLayout()
        self.plan_folded_text = _title(_wrap(QLabel()))
        self.plan_toggle = QToolButton()
        self.plan_toggle.setCheckable(True)
        self.plan_toggle.setToolButtonStyle(Qt.ToolButtonStyle.ToolButtonTextBesideIcon)
        self.plan_toggle.setAutoRaise(True)
        self.plan_toggle.toggled.connect(self._plan_toggled)
        pfrow.addWidget(self.plan_folded_text, 1)
        pfrow.addWidget(self.plan_toggle)
        pfl.addLayout(pfrow)
        fil.addWidget(self.plan_folded_card)

        # Carte du plan : titre, étapes en lecture seule, Valider / Refuser ou Arrêter.
        self.plan_card, pl = self._new_card()
        self.plan_title = self._focusable(self._card_title(pl))
        # Repère d'avancement (`progress-cue`) : « Étape k sur n en cours », sous le titre.
        self.progress_cue = self._meta(_wrap(QLabel(), selectable=False))
        self.progress_cue.setTextFormat(Qt.TextFormat.RichText)
        self.progress_cue.setVisible(False)
        self.progress_cue.setContentsMargins(22, 0, 0, 0)  # aligné sur le texte du titre
        pl.addWidget(self.progress_cue)
        self.plan_steps = QVBoxLayout()
        self.plan_steps.setContentsMargins(0, 0, 0, 0)
        self.plan_steps.setSpacing(2)
        pl.addLayout(self.plan_steps)
        # Couches de résultat que le modèle ne voit plus (ticket 5.33).
        self.left_out_note = self._meta(_wrap(QLabel()))
        self.left_out_note.setVisible(False)
        pl.addWidget(self.left_out_note)
        self.plan_cue = self._new_cue(pl)
        self.approve_button = QPushButton()
        self.approve_button.clicked.connect(self._approve)
        self.reject_button = QPushButton()
        self.reject_button.clicked.connect(self._reject)
        self.plan_stop_button = QPushButton()
        self.plan_stop_button.clicked.connect(self._stop)
        plan_buttons = QHBoxLayout()
        plan_buttons.addWidget(self.approve_button, 1)
        plan_buttons.addWidget(self.reject_button, 1)
        plan_buttons.addStretch(0)
        plan_buttons.addWidget(self.plan_stop_button)
        pl.addLayout(plan_buttons)
        self.plan_reason = self._meta(_wrap(QLabel()))
        pl.addWidget(self.plan_reason)
        self.plan_details = _Fold()
        self.plan_details_view = _mono(_wrap(QLabel(), plain=True))
        pdl = QVBoxLayout(self.plan_details.body)
        pdl.setContentsMargins(0, 0, 0, 0)
        pdl.addWidget(self.plan_details_view)
        pl.addWidget(self.plan_details)
        fil.addWidget(self.plan_card)

        # Carte de résultat : une ligne par étape en clair, Ouvrir le dossier, Détails.
        self.result_card, resl = self._new_card()
        self.result_title = self._focusable(self._card_title(resl))
        self.result_rows = QVBoxLayout()
        self.result_rows.setContentsMargins(0, 0, 0, 0)
        self.result_rows.setSpacing(2)
        resl.addLayout(self.result_rows)
        self.result_text = _wrap(QLabel(), plain=True)
        resl.addWidget(self.result_text)
        self.result_cue = self._new_cue(resl)
        self.result_rephrase, self.result_rephrase_detail = self._new_rephrase(resl)
        self.open_folder_button = QPushButton()
        self.open_folder_button.clicked.connect(self._open_folder)
        self.result_stop = QPushButton()
        self.result_stop.clicked.connect(self._stop)
        self.details = _Fold()
        self.details_view = _mono(_wrap(QLabel(), plain=True))
        dl = QVBoxLayout(self.details.body)
        dl.setContentsMargins(0, 0, 0, 0)
        dl.addWidget(self.details_view)
        rrow = QHBoxLayout()
        rrow.addWidget(self.details, 1, Qt.AlignmentFlag.AlignTop)
        self.result_stop_reason = self._meta(_wrap(QLabel()))
        rrow.addWidget(self.result_stop_reason, 1, Qt.AlignmentFlag.AlignTop)
        rrow.addWidget(self.result_stop, 0, Qt.AlignmentFlag.AlignTop)
        rrow.addWidget(self.open_folder_button, 0, Qt.AlignmentFlag.AlignTop)
        resl.addLayout(rrow)
        fil.addWidget(self.result_card)
        fil.addStretch(1)

        # Repli Journal : fermé par défaut, son compte dans le libellé.
        self.log_fold = _Fold(self._fold_toggled, wide=True)
        lfl = QVBoxLayout(self.log_fold.body)
        lfl.setContentsMargins(0, 0, 0, 0)
        self.log_list = QListWidget()
        self.log_list.setWordWrap(True)
        self.log_list.itemClicked.connect(self._log_clicked)
        self.log_list.itemActivated.connect(self._log_clicked)
        self.log_empty = QLabel()
        lfl.addWidget(self.log_empty)
        lfl.addWidget(self.log_list)
        # « Rejouer le journal » [12.3] : bouton natif, sa raison quand il est grisé, puis
        # l'issue du rejeu en `body` sous le bouton, et le code sous Détails techniques.
        self.replay_button = QPushButton()
        self.replay_button.clicked.connect(self._replay)
        replay_row = QHBoxLayout()
        replay_row.addWidget(self.replay_button, 0, Qt.AlignmentFlag.AlignTop)
        self.replay_reason = self._meta(_wrap(QLabel(), selectable=False))
        replay_row.addWidget(self.replay_reason, 1)
        replay_row.addStretch(0)  # sans raison, le bouton garde sa largeur naturelle
        lfl.addLayout(replay_row)
        self.replay_status = _wrap(QLabel(), plain=True)  # message du noyau, jamais interprété
        self.replay_status.setVisible(False)
        lfl.addWidget(self.replay_status)
        self.replay_details = _Fold(self._fold_toggled)
        self.replay_details_view = _mono(_wrap(QLabel(), plain=True))
        rdl = QVBoxLayout(self.replay_details.body)
        rdl.setContentsMargins(0, 0, 0, 0)
        rdl.addWidget(self.replay_details_view)
        self.replay_details.setVisible(False)
        lfl.addWidget(self.replay_details)
        col.addWidget(self.log_fold)

        # Repli Mode expert : son contenu défile à part s'il dépasse la moitié du panneau.
        self.expert = _Fold(self._expert_toggled, wide=True)
        efl = QVBoxLayout(self.expert.body)
        efl.setContentsMargins(0, 0, 0, 0)
        self.expert_scroll = QScrollArea()
        self.expert_scroll.setWidgetResizable(True)
        self.expert_scroll.setFrameShape(QFrame.Shape.NoFrame)
        self.expert_scroll.setHorizontalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAlwaysOff)
        self.expert_inner = QWidget()
        self.expert_scroll.setWidget(self.expert_inner)
        efl.addWidget(self.expert_scroll)
        el = QVBoxLayout(self.expert_inner)
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

        # Saisie épinglée en bas : champ multiligne et Envoyer, aligné sur le bas du champ.
        self.input = RequestEdit(self._send)
        lines = self.input.fontMetrics().lineSpacing()
        self.input.setFixedHeight(2 * lines + 14)
        self.input.textChanged.connect(self._input_changed)
        self.send_button = QPushButton()
        self.send_button.clicked.connect(self._send)
        irow = QHBoxLayout()
        irow.setSpacing(6)
        irow.addWidget(self.input, 1)
        irow.addWidget(self.send_button, 0, Qt.AlignmentFlag.AlignBottom)
        col.addLayout(irow)
        # Raison de la saisie grisée quand du texte cache le texte indicatif.
        self.input_reason = self._meta(_wrap(QLabel()))
        self.input_reason.setVisible(False)
        col.addWidget(self.input_reason)

        # Pied : information, pas une panne, sous un filet du thème.
        col.addWidget(_hline())
        self.footer = self._meta(_wrap(QLabel(), selectable=False))
        col.addWidget(self.footer)

        # Ordre de tabulation = ordre du fil, puis replis, saisie, Envoyer.
        order = [
            self.settings_button, self.lang_box, self.model_box, self.external_switch,
            self.external_ask, self.banner_button, self.external_stop, self.external_cut,
            self.approval_allow, self.approval_deny, self.composing_title,
            self.failed_rephrase, self.failed_details.button, self.rejected_rephrase,
            self.plan_toggle, self.plan_title, self.approve_button, self.reject_button,
            self.plan_stop_button, self.plan_details.button, self.result_title,
            # Résultat : du titre (ou de Reformuler) à la saisie, puis le reste de la carte.
            self.result_rephrase, self.input, self.send_button, self.details.button,
            self.result_stop, self.open_folder_button, self.log_fold.button, self.log_list,
            self.replay_button, self.replay_details.button, self.expert.button, self.command_list, self.run_button, self.stop_button,
        ]
        for a, b in zip(order, order[1:]):
            QWidget.setTabOrder(a, b)
        self._apply_styles()

    # -- état ----------------------------------------------------------
    def t(self, key, **values):
        return texts.tr(self.lang, key, **values)

    def set_banner(self, kind, message=None):
        """`kind` : clé de BANNERS, ou None quand le noyau est prêt.

        `message` : texte du noyau affiché tel quel à la place du texte du
        bandeau (refus de version) ; repli sur le texte du bandeau quand il est
        absent ou vide. La lecture seule passe par `set_ready`, avec le texte de la spine.
        """
        self.banner = kind
        self.banner_message = message if isinstance(message, str) and message else None
        self.focus = None
        if kind is not None:
            self.ready = False
            self._drop_stale_run()
        self.render()
        if kind is not None:
            _announce(self.banner_text, self._banner_text(self.t))

    def _banner_text(self, t):
        return self.banner_message or t(BANNERS[self.banner][0])

    def _drop_stale_run(self):
        """Le noyau a été fermé : une commande ou un plan en vol n'existe plus."""
        if self.running:
            self.running = False
            self.log.finish(runlog.CANCELLED)
        # Interrupteur éteint, question fermée, appels externes en cours annulés.
        self.external.lost(self.log)
        plan_in_flight = self.agent.phase != agentview.IDLE
        if self.agent.phase in (agentview.GENERATING, agentview.PROPOSED):
            self._exchange = None  # la demande n'a plus de réponse : le fil repart à l'accueil
        if thread.replay_running(self.replay):
            self.replay = None  # le noyau parti, le rejeu n'a plus de verdict à attendre
        self._close_plan_entry(runlog.ERROR)
        self.agent.forget()
        last = self.last
        if plan_in_flight and last and last.get("plan") and last["kind"] == runlog.RUNNING:
            self.last = None  # plus d'« En cours… » d'une étape qui n'existe plus

    @property
    def read_only(self):
        """Session en lecture seule : noyau prêt, bandeau de lecture seule (AD-11)."""
        return bool(self.ready and self.banner == "read_only")

    def set_ready(self, catalogue, history=(), read_only=False):
        """Noyau prêt. `read_only` : le journal est tenu par une autre session ; le bandeau
        le dit et seul « Rejouer le journal » reste actif (EXPERIENCE.md, AD-11)."""
        self._drop_stale_run()
        self.ready = True
        self.banner = "read_only" if read_only else None
        self.banner_message = None
        self.replay = None  # noyau (re)lancé : l'issue d'un rejeu précédent ne vaut plus
        self.catalogue = list(catalogue)
        # Le journal du panneau reprend celui du projet, que le noyau rend à l'ouverture :
        # les étapes passées restent visibles, et celles d'un autre projet disparaissent.
        # Un autre projet (ou le premier) : le journal du panneau reprend celui du projet, que
        # le noyau rend à l'ouverture. Un noyau relancé sur le même projet garde les lignes de
        # la session, dont celles que le journal ne tient pas (commande refusée avant d'être
        # lancée, étape coupée par l'arrêt du noyau).
        project = getattr(self, "project_file", None)
        if getattr(self, "_history_project", object()) != project:
            self._history_project = project
            self.focus = None  # le détail cliqué visait une ligne de l'ancien journal
            self.last = None
            self._shown_result = None
            self._exchange = None  # le fil repart de la carte d'accueil
            self._plan_open = False
            self.log = runlog.RunLog.from_history(
                history,
                lambda command, params: summary.label(
                    command, params, self.lang, _layer_name, None, self.catalogue
                ),
            )
        self.selected = None
        self.render()
        if read_only:
            _announce(self.banner_text, self._banner_text(self.t))
        else:
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
        self._rephrased = False  # un nouvel état : Reformuler peut revenir
        self._rephrase_note = None
        if kind == client.PLAN_PROPOSED:
            self._plan_entries = {}
            self._plan_refs = {}
            agent.proposed(payload.get("plan_id"), payload.get("plan"), payload.get("left_out"))
            self._plan_open = False
            _announce(self.plan_title, self.t("live_plan"))
        elif kind == client.PLAN_QUESTION:
            agent.question(
                payload.get("question", ""),
                payload.get("source", "model"),
                payload.get("left_out"),
            )
            restored = agentview.request_to_restore(
                agent, self._last_request, self.input.toPlainText()
            )
            if restored is not None:
                self.input.setPlainText(restored)  # à compléter par le point
            _announce(self.question_text, self.t("live_question"))
        elif kind == client.PLAN_FAILED:
            agent.failed(payload.get("code", ""), payload.get("message", ""))
            if models.asks_list_again(payload.get("code")):
                self._actions.list_models()  # la liste a changé : la redemander
            if self._last_request and not self.input.toPlainText().strip():
                self.input.setPlainText(self._last_request)  # à corriger sans tout retaper
            _announce(self.failed_text, thread.failed_announcement(agent, self.lang))
        elif kind == client.PLAN_REJECTED:
            agent.rejected()
            _announce(self.rejected_title, self.t("plan_rejected"))
        elif kind == client.PLAN_STEP:
            self._close_plan_entry(runlog.DONE)
            agent.step(payload.get("step"), payload.get("command"))
            target = agent.find(payload.get("step"))
            command = payload.get("command") or (target.command if target else "?")
            label = (
                agentview.step_label(
                    target, self.catalogue, self.lang, _layer_name, self._plan_refs
                )
                if target is not None
                else summary.short_title(command, self.catalogue, self.lang)
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
            # « Étape k sur n en cours » : annoncée sans déplacer le focus.
            # Une seule fois : le nom du titre porte déjà le repère.
            _announce(self.progress_cue, thread.progress_text(agent, self.lang))
        elif kind == client.PLAN_DONE:
            self._close_plan_entry(runlog.DONE)
            self.last = agentview.outcome(runlog.DONE, payload, agent)
            agent.done(payload.get("steps"))
            self._keep_plan_results()
            self.last["steps"] = list(agent.steps)
            self.last["refs"] = self._plan_refs
            _announce(self.result_text, thread.outcome_announcement(self.last, self.lang))
        elif kind == client.PLAN_EXEC_FAILED:
            self.last = agentview.outcome(runlog.ERROR, payload, agent)
            self._close_plan_entry(runlog.ERROR, self.last["step"])
            agent.exec_failed(self.last["step"], payload.get("outputs"))
            self._keep_plan_results(
                self.last["step"], (self.last.get("code", ""), self.last.get("message", ""))
            )
            _announce(self.result_text, thread.outcome_announcement(self.last, self.lang))
        elif kind == client.PLAN_CANCELLED:
            self.last = agentview.outcome(runlog.CANCELLED, payload, agent)
            self._close_plan_entry(runlog.CANCELLED, self.last["step"])
            agent.cancelled(self.last["step"], payload.get("outputs"))
            self._keep_plan_results()
            _announce(self.result_text, thread.outcome_announcement(self.last, self.lang))
        self.render()

    def on_models(self, kind, payload):
        """Liste des modèles reçue (MODELS) ou en échec (MODELS_FAILED)."""
        if kind == client.MODELS:
            self.models, chosen, forget = models.received(payload.get("models"), self.model)
            self.models_error = None
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
        if kind in (client.REPLAY_DONE, client.REPLAY_FAILED):
            self._on_replay_event(kind, payload or {})
            return
        self.focus = None  # tout nouvel événement efface le détail cliqué
        if kind == client.RUN_STARTED:
            self.running = True
            params, self._run_params = self._run_params, None
            # Le libellé court de l'étape, au journal comme dans la bulle [12.3].
            label = summary.label(
                payload["command"], params, self.lang, _layer_name, None, self.catalogue
            )
            self.log.start(payload["command"], label, time.strftime("%H:%M"), params=params)
            self.last = {"kind": runlog.RUNNING, "command": payload["command"]}
            # Un Lancer expert remplace l'échange en cours, comme une demande.
            self._exchange = thread.expert_exchange(label, time.strftime("%H:%M"))
            self._plan_open = False
            self._rephrased = False
            self._rephrase_note = None
            self.agent.reset()  # le plan fini précédent quitte le fil (il reste au journal)
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

    def _on_replay_event(self, kind, payload):
        """Verdict du rejeu : affiché et annoncé dans le repli Journal ; le fil, le détail
        cliqué et le focus restent comme ils étaient (EXPERIENCE.md, State Patterns)."""
        self.replay = thread.replay_outcome(kind, payload)
        self.replay["entries"] = len(self.log.entries)  # oublié dès que le journal grandit
        self.render()
        kept, self._replay_focus = self._replay_focus, None
        if kept is not None and self._focus_widget() in (None, kept[0]):
            if self.replay_button.isEnabled():  # l'utilisateur n'a pas déplacé le focus
                self.replay_button.setFocus(Qt.FocusReason.OtherFocusReason)
        _announce(self.replay_status, thread.replay_text(self.replay, self.lang))

    # -- affichage -----------------------------------------------------
    def render(self):
        t = self.t
        inside = self._focus_inside()  # lu avant de cacher des cartes (Qt déplace le focus)
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
            action = BANNERS[self.banner][1]
            text = self._banner_text(t)
            self.banner_text.setText(text)
            self.banner_text.setAccessibleName(text)
            self.banner_button.setVisible(action is not None)
            if action == "choose":
                self.banner_button.setText(t("choose_core"))
            elif action == "relaunch":
                self.banner_button.setText(t("relaunch"))
            self.banner_button.setAccessibleName(self.banner_button.text())
            self.banner_zone.setVisible(True)

        self._render_external()

        self.input.setAccessibleName(t("input_name"))
        self.send_button.setText(t("send"))
        self._render_agent()

        self.details.button.setText(t("details"))
        self.details.button.setAccessibleName(t("details"))
        self._render_result()
        self._render_thread()
        self._render_log()

        self.expert.button.setText(t("expert_title"))
        self.expert.button.setAccessibleName(t("expert_title"))
        # En lecture seule, le repli s'ouvre pour dire pourquoi Lancer est grisé.
        self.expert.setEnabled(self._controls()["expert_enabled"])
        self.footer.setText(t("offline"))
        self._render_commands()
        self._render_buttons()
        self._move_focus(inside)

    def _focus_widget(self):
        return QApplication.focusWidget()

    def _focus_inside(self):
        """Le focus clavier est-il dans le panneau (hors fenêtre des réglages) ?"""
        widget = self._focus_widget()
        return widget is not None and (widget is self or self.isAncestorOf(widget))

    def _move_focus(self, inside):
        """Le focus va, une fois par état, où le dit `thread.focus_target` ; jamais hors du
        panneau ni pendant que l'utilisateur tape (l'annonce suffit alors)."""
        target = thread.focus_target(
            self.agent, self._exchange, self.last, self.focus, self._guide()
        )
        if target == self._focus_target:
            return
        if target is None or not thread.may_move_focus(
            inside, self.input.hasFocus(), bool(self.input.toPlainText().strip())
        ):
            self._focus_target = target  # rien à faire, ou l'utilisateur tape : pas de reprise
            return
        if target == thread.FOCUS_REPHRASE:
            widget = next(
                (b for b, _d in self._rephrase_buttons if b.isVisibleTo(self)), None
            )
        else:
            widget = {
                thread.FOCUS_INPUT: self.input,
                thread.FOCUS_APPROVE: self.approve_button,
                thread.FOCUS_COMPOSING: self.composing_title,
                thread.FOCUS_PLAN_TITLE: self.plan_title,
                thread.FOCUS_RESULT_TITLE: self.result_title,
            }.get(target)
        if widget is not None:
            # Après la mise en page et le calage du fil : le widget est alors visible.
            QTimer.singleShot(0, lambda w=widget, k=target: self._set_focus(w, k))

    def _set_focus(self, widget, target):
        """Pose le focus ; la cible n'est retenue qu'une fois le focus posé : un widget
        grisé (Valider pendant un appel de l'IA externe) le recevra au rendu suivant."""
        if widget.isVisible() and widget.isEnabled():
            widget.setFocus(Qt.FocusReason.OtherFocusReason)
            self._focus_target = target
            if self.thread_body.isAncestorOf(widget):
                self.thread_scroll.ensureWidgetVisible(widget, 0, 0)

    # -- le fil ----------------------------------------------------------
    def _apply_styles(self):
        """Paire claire ou sombre des couleurs de Cassini, lue dans la palette de QGIS."""
        if self._styling:
            return
        self._styling = True
        try:
            self._styles = thread.styles(thread.colors(self._dark()))
            for card in self._cards:
                # Lignes d'étape stylées par la carte, jamais par une feuille propre.
                card.setStyleSheet(" ".join(
                    self._styles[k] for k in ("card", "step", "step_current")
                ))
            self.bubble.setStyleSheet(self._styles["bubble"])
            self.input.setStyleSheet(self._styles["input"])
            for cue in self._cues:
                cue.setStyleSheet(self._styles["cue"])
            for title in self._titles:
                title.setStyleSheet(self._styles["title"])
            self.logo.setPixmap(self._logo(24))
            for icon in self._small_logos:
                icon.setPixmap(self._logo(16))
            dim = self.palette().color(QPalette.ColorRole.PlaceholderText)
            for label in self._metas:
                palette = label.palette()
                palette.setColor(QPalette.ColorRole.WindowText, dim)
                label.setPalette(palette)
            self._primary_shown = None  # à reposer avec la nouvelle paire
        finally:
            self._styling = False

    def changeEvent(self, event):
        super().changeEvent(event)
        if event.type() == QEvent.Type.PaletteChange and self._styles is not None:
            self._apply_styles()
            self.render()

    def resizeEvent(self, event):
        super().resizeEvent(event)
        if self._styles is not None:
            self._fit_bubble()
            self._fit_folds()

    def _guide(self):
        """Guidage de l'état courant (logique dans `thread.guidance`)."""
        return thread.guidance(
            self.agent, self._exchange, self.last, self.focus, self._rephrased,
            self._controls(),
        )

    def _style_primary(self, c):
        """Un seul bouton principal visible (DESIGN.md, `button-primary`)."""
        shown = thread.shown_guidance(
            self._guide(), thread.replay_running(self.replay), self.read_only
        )
        which = thread.primary(c, shown["rephrase_enabled"])
        if which == self._primary_shown or self._styles is None:
            return
        self._primary_shown = which
        style = self._styles["primary"]
        self.send_button.setStyleSheet(style if which == "send" else "")
        self.approve_button.setStyleSheet(style if which == "approve" else "")
        for button, _detail in self._rephrase_buttons:
            button.setStyleSheet(style if which == "rephrase" else "")

    def _fit_bubble(self):
        """La bulle épouse sa phrase, sans dépasser 85 % du fil."""
        available = self.thread_scroll.viewport().width()
        widest = thread.bubble_width(available)
        metrics = self.bubble_text.fontMetrics()
        lines = (self.bubble_text.text() or " ").splitlines() or [" "]
        natural = max(metrics.horizontalAdvance(line) for line in lines) + 24
        self.bubble.setFixedWidth(max(48, min(widest, natural)))

    def _fit_folds(self):
        """Les replis dépliés prennent leur contenu, ensemble au plus la moitié du panneau,
        et le fil garde toujours une hauteur visible (`thread.fold_heights`)."""
        height = self.height()
        # Hauteur des lignes à la ligne près : `sizeHintForRow` sous-estime le texte replié.
        metrics = self.log_list.fontMetrics()
        width = max(80, self.width() - 40)
        content = 0
        for i in range(self.log_list.count()):
            box = metrics.boundingRect(
                QRect(0, 0, width, 100000), int(Qt.TextFlag.TextWordWrap),
                self.log_list.item(i).text(),
            )
            content += box.height() + 6
        content += 2 * self.log_list.frameWidth() + 4
        inner = self.expert_inner.sizeHint().height() + 2
        log_open = self.log_fold.is_open() and self.log_list.isVisibleTo(self.log_fold)
        expert_open = self.expert.is_open()
        # Ce que prend le reste du panneau : tout sauf le fil et le contenu des replis.
        reserved = height - self.thread_scroll.height()
        reserved -= self.log_list.height() if log_open else 0
        reserved -= self.expert_scroll.height() if expert_open else 0
        log_h, expert_h = thread.fold_heights(
            content if log_open else None, inner if expert_open else None, height, reserved
        )
        if log_open:
            self.log_list.setFixedHeight(max(24, log_h))
        if expert_open:
            self.expert_scroll.setFixedHeight(expert_h)
        # Le repli prend sa hauteur tout de suite, pas au prochain tour : chaque mise en page
        # est refaite de l'intérieur vers l'extérieur (sinon la taille du repli reste l'ancienne).
        for fold in (self.log_fold, self.expert):
            for widget in (fold.body, fold):
                if widget.layout() is not None:
                    widget.layout().invalidate()
                    widget.layout().activate()
            fold.updateGeometry()
        if self.layout() is not None:
            self.layout().invalidate()
            self.layout().activate()

    def _fold_toggled(self, _opened):
        self._fit_folds()

    def _plan_toggled(self, opened):
        self._plan_open = bool(opened)
        self._render_thread()

    def _scroll_to_newest(self):
        """Le fil se cale sur la carte la plus récente, sans cacher la bulle s'il tient."""
        cards = [w for w in (self.welcome_card, self.bubble_zone, self.composing_card,
                             self.failed_card, self.question_card, self.rejected_card,
                             self.plan_folded_card, self.plan_card, self.result_card)
                 if w.isVisible()]
        if not cards:
            return
        self.thread_body.layout().activate()  # géométrie à jour avant de la lire
        newest = cards[-1]
        bar = self.thread_scroll.verticalScrollBar()
        view = self.thread_scroll.viewport().height()
        top, bottom = newest.y(), newest.y() + newest.height()
        if self.bubble_zone.isVisible() and bottom - self.bubble_zone.y() <= view:
            top = self.bubble_zone.y()  # bulle et carte tiennent ensemble : les deux à l'écran
        # Défiler juste assez pour voir la carte ; si elle dépasse, montrer son haut.
        value = min(max(bar.value(), bottom - view), top)
        bar.setValue(max(0, min(bar.maximum(), value)))

    def _render_thread(self):
        """Quelles cartes du fil se voient (logique dans `thread.cards`)."""
        t = self.t
        lang = self.lang
        kinds = thread.cards(self.agent, self._exchange, self.last, self.focus)
        self.welcome_card.setVisible(thread.WELCOME in kinds)
        self.welcome_title.setText(t("welcome_title"))
        self.welcome_text.setText(t("welcome_text"))
        self.examples_title.setText(t("examples_title"))
        chevron = self._styles["chevron"] if self._styles else ""
        dim = self.palette().color(QPalette.ColorRole.PlaceholderText).name()
        self.examples.setText("<br>".join(
            "<span style='color:%s'>›</span> <span style='color:%s'>%s</span>"
            % (chevron, dim, t(key)) for key in ("example_1", "example_2", "example_3")
        ))

        self.bubble_zone.setVisible(thread.BUBBLE in kinds)
        self.bubble_text.setText(thread.bubble_text(self._exchange, lang))
        self.bubble_meta.setText(thread.bubble_meta(self._exchange, lang))
        self.bubble.setAccessibleName(thread.bubble_name(self._exchange, lang))

        self.composing_card.setVisible(thread.COMPOSING in kinds)
        self.failed_card.setVisible(thread.FAILED in kinds)
        self.question_card.setVisible(thread.QUESTION in kinds)
        self.rejected_card.setVisible(thread.REJECTED in kinds)
        folded = thread.PLAN_FOLDED in kinds
        if not folded and self.plan_toggle.isChecked():
            self.plan_toggle.blockSignals(True)
            self.plan_toggle.setChecked(False)
            self.plan_toggle.blockSignals(False)
            self._plan_open = False
        self.plan_folded_card.setVisible(folded)
        self.plan_card.setVisible(thread.PLAN in kinds or (folded and self._plan_open))
        n = len(self.agent.steps)
        self.plan_folded_text.setText(thread.folded_summary(self.agent, self.last, lang))
        self.plan_toggle.setText(t("plan_hide") if self._plan_open else t("plan_show"))
        self.plan_toggle.setArrowType(
            Qt.ArrowType.DownArrow if self._plan_open else Qt.ArrowType.RightArrow
        )
        self.plan_toggle.setAccessibleName(thread.folded_name(n, self._plan_open, lang))
        self.result_card.setVisible(thread.RESULT in kinds)
        self._render_guidance()
        self._fit_bubble()
        if kinds != self._kinds_shown:
            self._kinds_shown = kinds
            QTimer.singleShot(0, self._scroll_to_newest)

    def _render_guidance(self):
        """Ligne « À vous », Reformuler et repère d'avancement [12.2] (`thread.guidance`)."""
        t = self.t
        # Pendant un rejeu : pas de « À vous », Reformuler grisé [12.3].
        guide = thread.shown_guidance(
            self._guide(), thread.replay_running(self.replay), self.read_only
        )
        text = thread.cue_text(guide, self.lang)
        where = {
            thread.PLAN: (self.plan_cue, None),
            thread.QUESTION: (self.question_cue, None),
            thread.FAILED: (self.failed_cue, self.failed_rephrase),
            thread.REJECTED: (self.rejected_cue, self.rejected_rephrase),
            thread.RESULT: (self.result_cue, self.result_rephrase),
        }
        cue, button = where.get(guide["card"], (None, None))
        for label in self._cues:
            label.setText(text if label is cue else "")
            label.setVisible(label is cue and bool(text))
        detail = t(self._rephrase_note or "rephrase_detail")
        for b, d in self._rephrase_buttons:
            shown = b is button and guide["rephrase"]
            b.setText(t("rephrase"))
            b.setAccessibleName(t("rephrase"))
            b.setAccessibleDescription(detail if shown else "")
            b.setVisible(shown)
            b.setEnabled(guide["rephrase_enabled"] or not shown)
            d.setText(detail)
            d.setVisible(shown)
        dim = self.palette().color(QPalette.ColorRole.WindowText).name()
        progress = thread.progress_text(self.agent, self.lang, emphasis=dim)
        self.progress_cue.setText(progress)
        self.progress_cue.setVisible(bool(progress))
        self.plan_title.setAccessibleName(thread.plan_title_name(self.agent, self.lang))

    def _rephrase(self):
        """« Reformuler la demande » : la phrase revient dans une saisie vide, jamais
        par-dessus un texte déjà tapé ; le focus va dans la saisie."""
        if thread.replay_running(self.replay):
            return  # saisie grisée pendant le rejeu : rien n'y est écrit
        out = thread.rephrase(self._exchange, self._last_request, self.input.toPlainText())
        if out["text"] is not None:
            self.input.setPlainText(out["text"])
        if out["done"]:  # curseur à la fin, phrase rendue ou déjà là
            cursor = self.input.textCursor()
            cursor.movePosition(QTextCursor.MoveOperation.End)
            self.input.setTextCursor(cursor)
        self._rephrased = out["done"]
        self._rephrase_note = out["note"]
        self.render()
        self.input.setFocus(Qt.FocusReason.OtherFocusReason)
        _announce(self.input, self.t(out["note"] or "rephrase_detail"))

    def _render_log(self):
        t = self.t
        n = len(self.log.entries)
        label = thread.log_fold_label(n, self.lang)
        self.log_fold.button.setText(label)
        self.log_fold.button.setAccessibleName(label)
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
        self.log_list.setAccessibleName(label)
        self.log_list.scrollToBottom()
        self._render_replay()
        self._fit_folds()

    def _render_replay(self):
        """Bouton « Rejouer le journal », sa raison et l'issue du rejeu (`thread.replay_*`)."""
        t = self.t
        self.replay = thread.replay_current(self.replay, len(self.log.entries))
        b = self._replay_controls()
        reason = t(b["reason"]) if b["reason"] else ""
        self.replay_button.setText(t("replay"))
        self.replay_button.setEnabled(b["enabled"])
        self.replay_button.setAccessibleName("%s, %s" % (t("replay"), reason) if reason else t("replay"))
        self.replay_reason.setText(reason)
        self.replay_reason.setVisible(bool(reason))
        line = thread.replay_line(self.replay, self.lang)
        self.replay_status.setText(line)
        self.replay_status.setAccessibleName(thread.replay_text(self.replay, self.lang))
        self.replay_status.setVisible(bool(line))
        details = thread.replay_details(self.replay)
        self.replay_details.button.setText(t("details"))
        self.replay_details.button.setAccessibleName(t("details"))
        self.replay_details_view.setText(details)
        self.replay_details.setVisible(bool(details))

    def _replay_controls(self):
        return thread.replay_button(
            self.agent,
            self.ready,  # noyau prêt, lecture seule comprise : le rejeu y reste (AD-11)
            bool(self.log.entries),
            self.running,
            external.busy(self.external),
            self.replay,
        )

    def _busy(self):
        """Une commande experte ou un rejeu est en vol : la saisie et Lancer sont grisés."""
        return self.running or thread.replay_running(self.replay)

    def _fill_rows(self, layout, rows, on_click=None, current_style=""):
        """Lignes d'étapes d'une carte, séparées par un filet du thème.

        `rows` : (icône, mot, texte, message, en cours ?) de `thread`. Rend les lignes.
        """
        _clear(layout)
        widgets = []
        for index, (icon, word, text, message, current) in enumerate(rows):
            if index:
                layout.addWidget(_hline())
            click = (lambda i=index: on_click(i)) if on_click is not None else None
            row = _StepRow(
                icon, word, text, message, current, click,
                layout.parentWidget(),  # police résolue pour l'écran de la carte
            )
            layout.addWidget(row)
            widgets.append(row)
        return widgets

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
            self._render_thread()
            self._style_primary(self._controls())
            _announce(self.result_text, self.result_text.text())

    def _plan_clicked(self, row):
        if 0 <= row < len(self.agent.steps):
            self.focus = ("step", self.agent.steps[row])
            self._render_result()
            self._render_thread()
            self._style_primary(self._controls())
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
        if not message:  # une étape passée : le journal ne garde que le code
            return self.t("step_error_code", n=item.step or 1, code=code)
        return self.t("step_error", n=item.step or 1, code=code, message=message)

    def _render_focus(self):
        """Détail de la ligne du journal ou de l'étape du plan cliquée."""
        kind, item = self.focus
        if kind == "step":
            refs = self.agent.step_results()
            label = agentview.step_label(item, self.catalogue, self.lang, _layer_name, refs)
        else:
            label = item.label
            refs = item.refs
        result = item.result if item.status == runlog.DONE else None
        error = self._focus_error(self.focus) if item.status == runlog.ERROR else None
        self.result_title.setText(self.t("result_title"))
        self.result_text.setText(
            summary.detail(item, label, self.lang, _layer_name, refs, self.catalogue, error)
        )
        self._shown_result = result
        folder = runlog.result_folder(result)
        self.open_folder_button.setVisible(folder is not None)
        self.details_view.setText(runlog.step_details(item.params, result))
        self.details.setVisible(True)

    def _render_result(self):
        """Carte de résultat : plan exécuté ou arrêté, commande experte, détail cliqué."""
        t = self.t
        last = self.last
        self.details.setVisible(False)
        self.open_folder_button.setText(t("open_folder"))
        self.open_folder_button.setAccessibleName(t("open_folder"))
        self.result_stop.setVisible(False)
        self.result_stop_reason.setVisible(False)
        self.result_text.setVisible(True)
        self._fill_rows(self.result_rows, [])
        if self.focus is not None:
            self._render_focus()
            return
        self._shown_result = thread.shown_result(last)
        folder = runlog.result_folder(self._shown_result)
        self.open_folder_button.setVisible(folder is not None)
        if last is None:
            self.result_title.setText(t("result_title"))
            self.result_text.setText(t("result_none"))
            return
        kind = last["kind"]
        command = last.get("command", "")
        self.result_title.setText(thread.result_title(last, self.lang))
        details = ""
        if last.get("plan"):
            refs = last.get("refs") or self.agent.step_results()
            rows = thread.result_rows(
                last.get("steps") or self.agent.steps,
                last,
                self.lang,
                lambda s: "%d. %s" % (s.number, self._sentence(s.command, s.params, s.result, refs)),
                lambda s: agentview.step_label(s, self.catalogue, self.lang, _layer_name, refs),
            )
            self._fill_rows(self.result_rows, rows)
            closing = thread.closing(last, self.lang)
            self.result_text.setText(closing or "")
            self.result_text.setVisible(bool(closing))
            if kind == runlog.DONE:
                details = runlog.technical_details(last.get("result"))
            else:
                details = thread.error_details(last)
        elif kind == runlog.RUNNING:
            self.result_text.setVisible(False)
            c = runlog.controls(
                self.ready and self.banner is None, True, True,
                agentview.is_cancellable(command, self.catalogue),
            )
            self.result_stop.setText(t("stop"))
            self.result_stop.setEnabled(c["stop_enabled"])
            stop_reason = t(c["stop_reason"]) if c["stop_reason"] else ""
            self.result_stop.setAccessibleName(
                "%s, %s" % (t("stop"), stop_reason) if stop_reason else t("stop")
            )
            self.result_stop.setVisible(True)
            self.result_stop_reason.setText(stop_reason)  # à côté du bouton grisé
            self.result_stop_reason.setVisible(bool(stop_reason))
        elif kind == runlog.DONE:
            self.result_text.setText(
                self._sentence(command, last.get("params"), last.get("result"))
            )
            details = runlog.step_details(last.get("params"), last.get("result"))
        elif kind == runlog.CANCELLED:
            self.result_text.setText(t("cancelled_generic", command=command))
        else:
            self.result_text.setText(last.get("message", "") or t("live_error"))
            details = thread.error_details(last)
        self.details_view.setText(details)
        self.details.setVisible(bool(details))

    def _controls(self):
        return agentview.controls(
            self.agent,
            self.ready and self.banner is None,
            self._busy(),
            bool(self.input.toPlainText().strip()),
            self.catalogue,
            external.busy(self.external),
            self.read_only,
        )

    def _render_agent(self):
        """Saisie et cartes de l'agent, d'après l'état du plan (logique dans `agentview`)."""
        t = self.t
        lang = self.lang
        agent = self.agent
        c = self._controls()
        self.input.setEnabled(c["input_enabled"])
        self.send_button.setEnabled(c["send_enabled"])
        # Grisée, la saisie dit pourquoi à la place de son texte indicatif.
        hint = thread.input_hint(agent, c["input_reason"], self._busy(), self._exchange, lang)
        self.input.setPlaceholderText(hint)
        reason = hint if c["input_reason"] else ""
        self.input.setAccessibleDescription(reason)
        line = thread.reason_line(reason, bool(self.input.toPlainText().strip()))
        self.input_reason.setText(line)
        self.input_reason.setVisible(bool(line))
        self.send_button.setAccessibleName("%s, %s" % (t("send"), reason) if reason else t("send"))

        # Carte de composition.
        self.composing_title.setText(t("composing_title"))
        self.composing_detail.setText(t("composing_detail"))
        self.plan_progress.setVisible(c["progress_visible"])
        self.plan_progress.setAccessibleName(t("composing_title"))
        self.compose_stop.setText(t("stop"))
        self.compose_stop.setAccessibleName("%s, %s" % (t("stop"), t("compose_not_cancellable")))
        self.compose_stop_reason.setText(t("compose_not_cancellable"))

        # Cartes d'échec, de question, de refus.
        key, values = agent.notice if agent.notice is not None else (None, {})
        self.failed_title.setText(t("compose_failed_title"))
        self.failed_text.setText(values.get("message", "") if key == "propose_error" else "")
        self.failed_text.setVisible(bool(self.failed_text.text()))
        code = values.get("code", "") if key == "propose_error" else ""
        self.failed_details.button.setText(t("details"))
        self.failed_details.button.setAccessibleName(t("details"))
        self.failed_details_view.setText(str(code or ""))
        self.failed_details.setVisible(bool(code))
        self.question_title.setText(t("question_title"))
        self.question_text.setText(values.get("text", "") if key == "question" else "")
        self.rejected_title.setText(t("plan_rejected"))
        left_out = agentview.left_out_text(agent, lang) or ""
        self.left_out_note.setText(left_out)
        self.left_out_note.setVisible(bool(left_out))
        self.question_left_out.setText(left_out)
        self.question_left_out.setVisible(bool(left_out))

        # Carte du plan : titre, étapes, boutons, Détails techniques.
        has_steps = bool(agent.steps)
        self.plan_title.setText(thread.plan_title(agent, lang) if has_steps else "")
        rows = self._fill_rows(
            self.plan_steps,
            thread.plan_rows(agent, self.catalogue, lang, _layer_name) if has_steps else [],
            self._plan_clicked,
        )
        chain = [self.plan_toggle, self.plan_title] + rows + [self.approve_button]  # ordre du fil
        for a, b in zip(chain, chain[1:]):
            QWidget.setTabOrder(a, b)
        self.plan_details.setVisible(has_steps)
        self.plan_details.button.setText(t("details"))
        self.plan_details.button.setAccessibleName(t("details"))
        self.plan_details_view.setText(runlog.technical_details(agent.plan) if has_steps else "")

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
        self._style_primary(c)

    def _input_changed(self):
        """À chaque frappe, seuls Envoyer et le bouton principal changent."""
        c = self._controls()
        self.send_button.setEnabled(c["send_enabled"])
        self._style_primary(c)
        if self._rephrase_note is not None:  # « saisie remplie » : plus vrai après une frappe
            self._rephrase_note = None
            self._render_guidance()

    def _render_commands(self):
        current = self.selected["id"] if self.selected else None
        self.command_list.blockSignals(True)
        self.command_list.clear()
        self.command_list.setAccessibleName(self.t("commands_name"))
        usable = self.ready and self.banner is None
        for entry in form.offered(self.catalogue):
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
        replaying = thread.replay_running(self.replay)
        c = runlog.controls(
            self.ready and self.banner is None,
            bool(entry and entry.get("available")),
            self.running,
            bool(entry and entry.get("cancellable")),
            external.busy(self.external) or replaying,  # Lancer grisé, sans Arrêter
        )
        self.run_button.setText(self.t("run"))
        self.stop_button.setText(self.t("stop"))
        self.run_button.setVisible(c["run_visible"])
        self.run_button.setEnabled(c["run_enabled"] and entry is not None)
        self.stop_button.setVisible(c["stop_visible"])
        self.stop_button.setEnabled(c["stop_enabled"])
        reason = ""
        locked = self._controls()["run_reason"]
        if locked:
            reason = self.t(locked)
        elif self.running and c["stop_reason"]:
            reason = self.t(c["stop_reason"])
        elif self.running:
            reason = self.t("busy")
        elif replaying:
            reason = self.t("replay_reason_running")
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
        self._fit_folds()

    def _run(self):
        if self.selected is None or self._busy() or not (self.ready and self.banner is None):
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

    def _replay(self):
        """« Rejouer le journal » : le noyau rejoue tout ; le panneau n'enchaîne rien (AD-11)."""
        if not self._replay_controls()["enabled"]:
            return
        if not self._actions.replay():
            # Le client a déjà une requête en vol (liste des modèles…) : on le dit.
            self.replay = thread.replay_refused()
            self.replay["entries"] = len(self.log.entries)
            self.render()
            _announce(self.replay_status, self.t("replay_refused"))
            return
        had_focus = bool(self.replay_button.hasFocus())
        self.replay = thread.replay_started()
        self.render()  # le bouton se grise : Qt déplace le focus
        self._replay_focus = (self._focus_widget(),) if had_focus else None
        _announce(self.replay_status, self.t("replay_running"))

    def _send(self):
        text = self.input.toPlainText().strip()
        c = agentview.controls(
            self.agent,
            self.ready and self.banner is None,
            self._busy(),
            bool(text),
            self.catalogue,
            external.busy(self.external),
            self.read_only,
        )
        if not c["send_enabled"]:
            return
        if self._actions.propose(text, self.model):  # la demande part sans les blancs des bords
            self._last_request = text
            # Une nouvelle demande remplace l'échange précédent ; il reste au journal.
            self._exchange = thread.request_exchange(text, time.strftime("%H:%M"))
            self.last = None
            self._plan_open = False
            self._rephrased = False
            self._rephrase_note = None
            self.agent.generating()
            self.input.clear()
            self.focus = None  # l'étape cliquée n'existe plus
            self.render()
            _announce(self.composing_title, self.t("composing_title"))

    def _approve(self):
        plan_id = self.agent.plan_id
        if self.agent.phase != agentview.PROPOSED or plan_id is None:
            return
        if self._actions.approve(plan_id):  # seul le plan_id repart (AD-5, AD-23)
            self.plan_title.setAccessibleDescription("")  # plus « Un plan est proposé. »
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
        locked = self._controls()["model_reason"]
        if locked:
            reason = self.t(locked)
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
        locked = self._controls()["external_reason"]
        if locked:
            reason = t(locked)
        elif c["switch_reason"]:
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
        self._fit_folds()
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
