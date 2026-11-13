"""Point d'entrée de l'extension : branche le panneau [9.4] sur le client.

Qt uniquement par `qgis.PyQt` (AD-18). Aucune logique métier ici : le noyau
décide, le panneau affiche (AD-4).
"""

import datetime
import os

from qgis.core import QgsApplication, QgsProject
from qgis.PyQt.QtCore import QProcess, QSettings, Qt, QTimer, QUrl
from qgis.PyQt.QtGui import QDesktopServices
from qgis.PyQt.QtWidgets import QDockWidget, QFileDialog

from . import client, delegated, gestures, installation, locator, panel, texts

PROBE_MS = 3000  # resonde du noyau tant que le bandeau « moteur introuvable » est affiché
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
        self._probe = None

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
        self._stop_probe()
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

    def propose(self, text, model=None):
        """Demande en langage naturel -> `plan.propose`, avec le nom du modèle choisi
        (ou None : le noyau prend le premier). Rend True si elle est partie."""
        return self.client is not None and self.client.propose(text, model)

    def list_models(self):
        """Liste des modèles -> `agent.models`. Rend True si la demande est partie."""
        return self.client is not None and self.client.list_models()

    def approve(self, plan_id):
        """Validation de l'utilisateur -> `plan.execute` ; seul le `plan_id` repart."""
        return self.client is not None and self.client.execute_plan(plan_id)

    def reject(self, plan_id):
        return self.client is not None and self.client.reject_plan(plan_id)

    def replay(self):
        """« Rejouer le journal » -> `journal.replay` : le noyau rejoue tout (AD-11).
        Rend True si la demande est partie."""
        return self.client is not None and self.client.replay()

    def set_external(self, enabled, ask=False):
        """Interrupteur ou Couper -> `session.external`. Rend True si la demande est partie."""
        return self.client is not None and self.client.set_external(enabled, ask)

    def stop_external(self, call_id):
        """Arrêter un appel de l'IA externe -> `request.cancel {id: call_id}`."""
        return self.client is not None and self.client.cancel_external(call_id)

    def answer_approval(self, request_id, approved):
        """Autoriser / Refuser -> réponse à `external.approve`, si la question est encore ouverte."""
        return self.client is not None and self.client.answer_approval(request_id, approved)

    def choose_core(self):
        path, _ = QFileDialog.getOpenFileName(
            self.iface.mainWindow(),
            texts.tr(self.panel.lang if self.panel else "fr", "choose_core_title"),
        )
        if path:
            QSettings().setValue(locator.SETTING_KEY, path)
            self.project_path = None
            self._refresh()

    def open_home(self):
        """Ouvre la page d'accueil d'AD-17 dans le navigateur ; rien n'est téléchargé ici."""
        QDesktopServices.openUrl(QUrl(installation.HOME_URL))

    def relaunch(self):
        self._stop()
        self.project_path = None
        self._refresh()

    # -- logique du plugin ----------------------------------------------
    def _banner(self, kind, message=None):
        if self.panel is not None:
            self.panel.set_banner(kind, message)

    def _refresh(self, *_):
        file_name = QgsProject.instance().fileName()
        if not file_name:
            self._stop()
            self._stop_probe()
            self.project_path = None
            self._banner("no_project")
            return
        path = os.path.abspath(file_name).replace("\\", "/")
        if path == self.project_path and self.client is not None:
            return
        self._stop()
        self.project_path = path
        if self.panel is not None:
            self.panel.project_file = path
        self._launch()

    def _launch(self):
        setting = QSettings().value(locator.SETTING_KEY, "") or ""
        found = locator.locate(str(setting))
        if found is None:
            self.project_path = None
            if self.panel is not None:
                self.panel.set_install_info(None)  # plus d'installation retrouvée à décrire
            self._banner("not_found")
            self._start_probe()
            return
        self._stop_probe()
        program, _origin = found
        if self.panel is not None:
            info = installation.read(program)
            now = datetime.datetime.now(datetime.timezone.utc).replace(tzinfo=None)
            due = info is not None and installation.reminder_due(info["installed_at"], now)
            self.panel.set_install_info(info, due)
        self._banner("starting")
        self.process = QProcess()
        registry = delegated.build_registry(SlowProbe)
        self.client = client.CoreClient(self.process, self._on_event, registry)
        self.client.start(program, self.project_path)

    def _start_probe(self):
        if self._probe is None:
            self._probe = QTimer()
            self._probe.setInterval(PROBE_MS)
            self._probe.timeout.connect(self._probe_tick)
            self._probe.start()

    def _stop_probe(self):
        if self._probe is not None:
            self._probe.stop()
            self._probe = None

    def _probe_tick(self):
        """Le noyau est-il apparu ? Alors on se lance, sans redémarrer QGIS."""
        setting = QSettings().value(locator.SETTING_KEY, "") or ""
        if locator.locate(str(setting)) is not None:
            self._stop_probe()
            self.project_path = None
            self._refresh()

    def _on_event(self, kind, payload):
        if self.panel is None:
            return
        if kind == client.READY:
            # Lecture seule : le noyau reste vivant, le panneau ne laisse que le rejeu (AD-11).
            read_only = payload.get("read_only") is True
            self.panel.set_ready(
                payload.get("commands", []), payload.get("history", []), read_only=read_only
            )
            if not read_only:
                self.list_models()  # la liste des Réglages, dès que le noyau est prêt
        elif kind in (client.FAILED, client.DIED):
            self.client = None
            self.process = None
            self.project_path = None
            # FAILED : le noyau a répondu non (protocole), son message est affiché tel
            # quel ; DIED : il s'est arrêté.
            if kind == client.FAILED:
                self._banner("refused", payload)
            else:
                self._banner("died")
        elif kind in (
            client.RUN_STARTED,
            client.RUN_DONE,
            client.RUN_CANCELLED,
            client.RUN_FAILED,
            client.REPLAY_DONE,
            client.REPLAY_FAILED,
        ) or kind in client.PLAN_EVENTS or kind in client.MODEL_EVENTS or (
            kind in client.EXTERNAL_EVENTS
        ):
            self.panel.on_event(kind, payload)

    def _stop(self, *_):
        if self.client is not None:
            self.client.stop()
        self.client = None
        self.process = None
