"""Panneau ancré minimal du traceur. Pas une interface définitive.

Qt uniquement par `qgis.PyQt` (AD-18). Aucune logique métier ici.
"""

import os

from qgis.core import QgsApplication, QgsProject
from qgis.PyQt.QtCore import QProcess, QSettings, Qt
from qgis.PyQt.QtWidgets import (
    QDockWidget,
    QFileDialog,
    QLabel,
    QPushButton,
    QVBoxLayout,
    QWidget,
)

from . import client, locator

NO_PROJECT = "Ouvrez ou enregistrez un projet QGIS : Cassini se lance avec le projet."
NOT_FOUND = (
    "Le noyau Cassini est introuvable. Installez le paquet Cassini (le noyau "
    "n'est pas fourni avec l'extension), ou désignez le fichier à la main. "
    "Rien n'est téléchargé."
)


class CassiniPlugin:
    def __init__(self, iface):
        self.iface = iface
        self.dock = None
        self.label = None
        self.button = None
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
        layout.addWidget(self.label)
        layout.addWidget(self.button)
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
        self.client = client.CoreClient(self.process, self._on_event)
        self.client.start(program, self.project_path)

    def _on_event(self, kind, payload):
        if kind == client.READY:
            self._show(
                "Noyau Cassini %s, protocole %s."
                % (payload["core_version"], payload["protocol_version"])
            )
        else:
            self._show(str(payload))
            self.client = None
            self.process = None
            self.project_path = None

    def _choose_core(self):
        path, _ = QFileDialog.getOpenFileName(
            self.iface.mainWindow(), "Choisir le noyau Cassini"
        )
        if path:
            QSettings().setValue(locator.SETTING_KEY, path)
            self.project_path = None
            self._refresh()

    def _stop(self, *_):
        if self.client is not None:
            self.client.stop()
        self.client = None
        self.process = None
