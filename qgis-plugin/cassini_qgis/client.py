"""Client du canal local : JSON-RPC 2.0, un objet JSON par ligne.

Le client ne contient aucune logique métier. Il lance le noyau (processus
enfant), envoie `session.hello` puis `command.run` de `system.about`, et rend
le résultat par un rappel. Le processus est un objet à l'interface de
QProcess, fourni par l'appelant, ce qui permet de tester sans QGIS.
"""

import json

PROTOCOL_VERSION = 1

# Événements rendus au rappel `on_event(kind, payload)`.
READY = "ready"  # payload : {"core_version", "protocol_version"}
FAILED = "failed"  # payload : message à afficher tel quel
DIED = "died"  # payload : message


class CoreClient:
    def __init__(self, process, on_event):
        self._p = process
        self._on_event = on_event
        self._buffer = b""
        self._pending = {}  # id -> étape
        self._closing = False
        self._done = False  # un événement terminal a déjà été rendu
        process.readyReadStandardOutput.connect(self._read)
        process.readyReadStandardError.connect(self._drain_stderr)
        process.finished.connect(self._finished)
        process.errorOccurred.connect(self._error)

    # -- cycle de vie -------------------------------------------------
    def start(self, program, project):
        """Lance `cassini serve` ; `project` est un chemin absolu."""
        self._project = project.replace("\\", "/")
        self._p.start(program, ["serve"])
        self._send(
            1,
            "session.hello",
            {
                "protocol_version": PROTOCOL_VERSION,
                "commands": [],
                "project": self._project,
            },
            "hello",
        )

    def stop(self, wait_ms=3000):
        """Ferme l'entrée du noyau, qui s'arrête seul."""
        self._closing = True
        self._done = True
        self._p.closeWriteChannel()
        self._p.waitForFinished(wait_ms)

    # -- envoi --------------------------------------------------------
    def _send(self, ident, method, params, step):
        self._pending[ident] = step
        line = json.dumps(
            {"jsonrpc": "2.0", "id": ident, "method": method, "params": params},
            ensure_ascii=False,
        )
        self._p.write((line + "\n").encode("utf-8"))

    # -- réception ----------------------------------------------------
    def _read(self):
        self._buffer += bytes(self._p.readAllStandardOutput())
        while b"\n" in self._buffer:
            raw, self._buffer = self._buffer.split(b"\n", 1)
            line = raw.decode("utf-8", errors="replace").strip()
            if line:
                self._handle(line)

    def _drain_stderr(self):
        # Débogage seulement : lu pour ne pas bloquer le noyau, puis ignoré.
        self._p.readAllStandardError()

    def _handle(self, line):
        try:
            msg = json.loads(line)
        except ValueError:
            return
        if not isinstance(msg, dict) or "method" in msg:
            return  # ordres du noyau : hors de cette story
        step = self._pending.pop(msg.get("id"), None)
        if step is None:
            return
        if "error" in msg:
            self._fail(self._error_message(msg["error"]))
        elif step == "hello":
            self._send(
                2, "command.run", {"command": "system.about", "params": {}}, "about"
            )
        elif step == "about":
            result = msg.get("result") or {}
            self._finish(
                READY,
                {
                    "core_version": result.get("core_version"),
                    "protocol_version": result.get("protocol_version"),
                },
            )

    @staticmethod
    def _error_message(error):
        data = error.get("data") if isinstance(error, dict) else None
        if isinstance(data, dict) and data.get("message"):
            return data["message"]
        if isinstance(error, dict) and error.get("message"):
            return error["message"]
        return "Erreur inconnue du noyau."

    def _fail(self, message):
        self._finish(FAILED, message)
        self.stop()

    def _finish(self, kind, payload):
        if self._done:
            return
        if kind != READY:  # après READY, une mort du noyau reste signalée
            self._done = True
        self._on_event(kind, payload)

    def _finished(self, *_):
        if not self._closing:
            self._finish(DIED, "Le noyau Cassini s'est arrêté.")

    def _error(self, *_):
        if not self._closing:
            self._finish(DIED, "Le noyau Cassini n'a pas pu démarrer ou s'est arrêté.")
