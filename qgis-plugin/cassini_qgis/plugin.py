"""Point d'entrée de l'extension : branche le panneau [9.4] sur le client.

Qt uniquement par `qgis.PyQt` (AD-18). Aucune logique métier ici : le noyau
décide, le panneau affiche (AD-4).
"""

import os

from qgis.core import QgsApplication, QgsProject
from qgis.PyQt.QtCore import QProcess, QSettings, Qt, QTimer
from qgis.PyQt.QtWidgets import QDockWidget, QFileDialog

from . import client, gestures, layer_add, locator, panel, texts

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
        self.panel = None
        self.client = None
        self.process = None
        self.project_path = None

    # -- QGIS ---------------------------------------------------------
    def initGui(self):
        self.dock = QDockWidget("Cassini", self.iface.mainWindow())
        self.dock.setObjectName("CassiniDock")
        self.panel = panel.CassiniPanel(self)
        self.dock.setWidget(self.panel)
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
            self.panel = None

    # -- actions demandées par le panneau --------------------------------
    def run(self, command, params):
        if self.client is not None:
            self.client.run_command(command, params)

    def stop(self):
        if self.client is not None:
            self.client.cancel_run()  # request.cancel, jamais la mort du noyau

    def choose_core(self):
        path, _ = QFileDialog.getOpenFileName(
            self.iface.mainWindow(),
            texts.tr(self.panel.lang if self.panel else "fr", "choose_core_title"),
        )
        if path:
            QSettings().setValue(locator.SETTING_KEY, path)
            self.project_path = None
            self._refresh()

    def relaunch(self):
        self._stop()
        self.project_path = None
        self._refresh()

    # -- logique du plugin ----------------------------------------------
    def _banner(self, kind):
        if self.panel is not None:
            self.panel.set_banner(kind)

    def _refresh(self, *_):
        file_name = QgsProject.instance().fileName()
        if not file_name:
            self._stop()
            self.project_path = None
            self._banner("no_project")
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
            self.project_path = None
            self._banner("not_found")
            return
        program, _origin = found
        self._banner("starting")
        self.process = QProcess()
        registry = gestures.GestureRegistry()
        registry.register("test.qgis_probe", SlowProbe)
        registry.register("layer.add", layer_add.LayerAddGesture)
        self.client = client.CoreClient(self.process, self._on_event, registry)
        self.client.start(program, self.project_path)

    def _on_event(self, kind, payload):
        if self.panel is None:
            return
        if kind == client.READY:
            self.panel.set_ready(payload.get("commands", []))
        elif kind in (client.FAILED, client.DIED):
            self.client = None
            self.process = None
            self.project_path = None
            # FAILED : le noyau a répondu non (protocole) ; DIED : il s'est arrêté.
            self._banner("refused" if kind == client.FAILED else "died")
        elif kind in (
            client.RUN_STARTED,
            client.RUN_DONE,
            client.RUN_CANCELLED,
            client.RUN_FAILED,
        ):
            self.panel.on_event(kind, payload)

    def _stop(self, *_):
        if self.client is not None:
            self.client.stop()
        self.client = None
        self.process = None
