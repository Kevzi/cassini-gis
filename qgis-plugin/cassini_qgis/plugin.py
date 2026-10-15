"""Panneau ancré minimal du traceur. Pas une interface définitive.

Qt uniquement par `qgis.PyQt` (AD-18). Aucune logique métier ici.
"""

import os

from qgis.core import QgsApplication, QgsProject
from qgis.PyQt.QtCore import QProcess, QSettings, Qt, QTimer
from qgis.PyQt.QtWidgets import (
    QDockWidget,
    QFileDialog,
    QLabel,
    QPushButton,
    QVBoxLayout,
    QWidget,
)

from . import client, gestures, locator

NO_PROJECT = "Ouvrez ou enregistrez un projet QGIS : Cassini se lance avec le projet."
NOT_FOUND = (
    "Le noyau Cassini est introuvable. Installez le paquet Cassini (le noyau "
    "n'est pas fourni avec l'extension), ou désignez le fichier à la main. "
    "Rien n'est téléchargé."
)

SLOW_TICKS = 100  # 100 x 100 ms : un geste de test de dix secondes


class SlowProbe:
    """Geste de test `test.qgis_probe` : lent, sans appel bloquant.

    Avance par un QTimer de la boucle d'événements, donc QGIS reste réactif.
    """

    def __init__(self):
        self._timer = None
        self._done = None
        self._ticks = 0

    def start(self, params, done):
        self._done = done
        self._timer = QTimer()
        self._timer.setInterval(100)
        self._timer.timeout.connect(self._tick)
        self._timer.start()

    def _tick(self):
        self._ticks += 1
        if self._ticks >= SLOW_TICKS:
            self._finish(result={"ok": True})

    def cancel(self):
        self._finish(error=gestures.GestureError("qgis.cancelled", "Geste annulé."))

    def _finish(self, result=None, error=None):
        if self._timer is not None:
            self._timer.stop()
            self._timer = None
        done, self._done = self._done, None
        if done is not None:
            done(result=result, error=error)


class CassiniPlugin:
    def __init__(self, iface):
        self.iface = iface
        self.dock = None
        self.label = None
        self.button = None
        self.stop_button = None
        self.client = None
        self.process = None
        self.project_path = None

    # -- QGIS ---------------------------------------------------------
    def initGui(self):
        self.dock = QDockWidget("Cassini", self.iface.mainWindow())
        self.dock.setObjectName("CassiniDock")
        body = QWidget()
        layout = QVBoxLayout(body)
        self.label = QLabel("")
        self.label.setWordWrap(True)
        self.button = QPushButton("Choisir le noyau à la main…")
        self.button.clicked.connect(self._choose_core)
        self.button.hide()
        self.stop_button = QPushButton("Arrêter le geste en cours")
        self.stop_button.clicked.connect(self._cancel_order)
        self.stop_button.hide()
        layout.addWidget(self.label)
        layout.addWidget(self.button)
        layout.addWidget(self.stop_button)
        layout.addStretch(1)
        self.dock.setWidget(body)
        self.iface.addDockWidget(Qt.DockWidgetArea.RightDockWidgetArea, self.dock)
        QgsProject.instance().readProject.connect(self._refresh)
        QgsProject.instance().projectSaved.connect(self._refresh)
        QgsProject.instance().cleared.connect(self._refresh)
        QgsApplication.instance().aboutToQuit.connect(self._stop)
        self._refresh()

    def unload(self):
        self._stop()
        project = QgsProject.instance()
        app = QgsApplication.instance()
        for signal, slot in (
            (project.readProject, self._refresh),
            (project.projectSaved, self._refresh),
            (project.cleared, self._refresh),
            (app.aboutToQuit, self._stop),
        ):
            try:
                signal.disconnect(slot)
            except (TypeError, RuntimeError):
                pass
        if self.dock is not None:
            self.iface.removeDockWidget(self.dock)
            self.dock.deleteLater()
            self.dock = None

    # -- logique du panneau --------------------------------------------
    def _show(self, text, manual=False):
        if self.label is not None:
            self.label.setText(text)
            self.button.setVisible(manual)

    def _refresh(self, *_):
        file_name = QgsProject.instance().fileName()
        if not file_name:
            self._stop()
            self.project_path = None
            self._show(NO_PROJECT)
            return
        path = os.path.abspath(file_name).replace("\\", "/")
        if path == self.project_path and self.client is not None:
            return
        self._stop()
        self.project_path = path
        self._launch()

    def _launch(self):
        setting = QSettings().value(locator.SETTING_KEY, "") or ""
        found = locator.locate(str(setting))
        if found is None:
            self._show(NOT_FOUND, manual=True)
            return
        program, origin = found
        self._show("Démarrage du noyau (%s)…" % origin)
        self.process = QProcess()
        registry = gestures.GestureRegistry()
        registry.register("test.qgis_probe", SlowProbe)
        self.client = client.CoreClient(self.process, self._on_event, registry)
        self.client.start(program, self.project_path)

    def _on_event(self, kind, payload):
        if kind in (client.ORDER_STARTED, client.ORDER_ENDED):
            if self.stop_button is not None:
                self.stop_button.setVisible(kind == client.ORDER_STARTED)
        elif kind == client.READY:
            self._show(
                "Noyau Cassini %s, protocole %s."
                % (payload["core_version"], payload["protocol_version"])
            )
        else:
            self._show(str(payload))
            self.client = None
            self.process = None
            self.project_path = None
            if self.stop_button is not None:
                self.stop_button.hide()

    def _choose_core(self):
        path, _ = QFileDialog.getOpenFileName(
            self.iface.mainWindow(), "Choisir le noyau Cassini"
        )
        if path:
            QSettings().setValue(locator.SETTING_KEY, path)
            self.project_path = None
            self._refresh()

    def _cancel_order(self):
        if self.client is not None:
            self.client.cancel_current()

    def _stop(self, *_):
        if self.client is not None:
            self.client.stop()
        self.client = None
        self.process = None
