"""Client du canal local : JSON-RPC 2.0, un objet JSON par ligne.

Le client ne contient aucune logique métier. Il lance le noyau (processus
enfant), envoie `session.hello` puis `command.run` de `system.about`, rend le
résultat par un rappel, puis sert d'exécutant au noyau : il traite
`order.execute` en lançant le geste enregistré et relaie `request.cancel`.
Tout est piloté par les signaux du processus : aucune lecture bloquante.

Le processus est un objet à l'interface de QProcess, fourni par l'appelant, ce
qui permet de tester sans QGIS. Le registre de gestes est un objet à deux
méthodes, `ids()` et `create(id)` (voir `gestures.py`).
"""

import json

PROTOCOL_VERSION = 1

# Événements rendus au rappel `on_event(kind, payload)`.
READY = "ready"  # payload : {"core_version", "protocol_version"}
FAILED = "failed"  # payload : message à afficher tel quel
DIED = "died"  # payload : message
ORDER_STARTED = "order_started"  # payload : id de la commande
ORDER_ENDED = "order_ended"  # payload : id de la commande

GESTURE_UNKNOWN = "qgis.gesture_unknown"
GESTURE_FAILED = "qgis.gesture_failed"
BUSY = "qgis.busy"


class _NoGestures:
    def ids(self):
        return []

    def create(self, command_id):
        return None


class CoreClient:
    def __init__(self, process, on_event, gestures=None):
        self._p = process
        self._on_event = on_event
        self._gestures = gestures if gestures is not None else _NoGestures()
        self._buffer = b""
        self._pending = {}  # id -> étape
        self._next_id = 1
        self._current = None  # ordre en cours : {"id", "command", "gesture"}
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
        self._request(
            "session.hello",
            {
                "protocol_version": PROTOCOL_VERSION,
                "commands": self._gestures.ids(),
                "project": self._project,
            },
            "hello",
        )

    def stop(self, wait_ms=3000):
        """Ferme l'entrée du noyau, qui s'arrête seul.

        Ne sert qu'à la fermeture de QGIS : l'attente est volontaire.
        """
        self._closing = True
        self._done = True
        self._abort_current()
        self._p.closeWriteChannel()
        self._p.waitForFinished(wait_ms)

    def cancel_current(self):
        """Demande l'arrêt du geste en cours (bouton d'arrêt local).

        L'ordre reçoit tout de même une réponse, quand le geste rappelle `done`.
        """
        if self._current is not None:
            self._cancel_gesture(self._current)

    def busy(self):
        return self._current is not None

    # -- envoi --------------------------------------------------------
    def _write(self, message):
        if self._closing:
            return
        line = json.dumps(message, ensure_ascii=False)
        self._p.write((line + "\n").encode("utf-8"))

    def _request(self, method, params, step):
        ident = self._next_id
        self._next_id += 1
        self._pending[ident] = step
        self._write({"jsonrpc": "2.0", "id": ident, "method": method, "params": params})

    def _respond(self, ident, result=None, error=None):
        message = {"jsonrpc": "2.0", "id": ident}
        if error is not None:
            message["error"] = error
        else:
            message["result"] = result
        self._write(message)

    @staticmethod
    def _error_body(code, message):
        return {
            "code": -32000,
            "message": message,
            "data": {"code": code, "message": message},
        }

    # -- réception ----------------------------------------------------
    def _read(self):
        self._buffer += bytes(self._p.readAllStandardOutput())
        while b"\n" in self._buffer:
            raw, self._buffer = self._buffer.split(b"\n", 1)
            line = raw.decode("utf-8", errors="replace").strip()
            if line:
                self._handle(line)

    def _drain_stderr(self):
        # Débogage seulement : vidé à chaque signal pour ne pas bloquer le
        # noyau, puis ignoré.
        self._p.readAllStandardError()

    def _handle(self, line):
        try:
            msg = json.loads(line)
        except ValueError:
            return
        if not isinstance(msg, dict):
            return
        if "method" in msg:
            self._handle_call(msg)
            return
        step = self._pending.pop(msg.get("id"), None)
        if step is None:
            return
        if "error" in msg:
            self._fail(self._error_message(msg["error"]))
        elif step == "hello":
            self._request("command.run", {"command": "system.about", "params": {}}, "about")
        elif step == "about":
            result = msg.get("result") or {}
            self._finish(
                READY,
                {
                    "core_version": result.get("core_version"),
                    "protocol_version": result.get("protocol_version"),
                },
            )

    def _handle_call(self, msg):
        method = msg.get("method")
        params = msg.get("params")
        if not isinstance(params, dict):
            params = {}
        ident = msg.get("id")
        if method == "order.execute" and ident is not None:
            self._execute(ident, params)
        elif method == "request.cancel":
            current = self._current
            matches = current is not None and current["id"] == params.get("id")
            if matches:
                self._cancel_gesture(current)
            if ident is not None:
                self._respond(ident, {"cancelled": matches})
        elif ident is not None:
            self._respond(
                ident,
                error={"code": -32601, "message": "Méthode inconnue : %s" % method},
            )

    # -- ordres -------------------------------------------------------
    def _execute(self, ident, params):
        command = params.get("command")
        if self._current is not None:
            self._respond(
                ident,
                error=self._error_body(BUSY, "Un ordre est déjà en cours."),
            )
            return
        try:
            gesture = self._gestures.create(command) if isinstance(command, str) else None
        except Exception as exc:  # exception jamais propagée à Qt
            self._respond(ident, error=self._failure(exc))
            return
        if gesture is None:
            self._respond(
                ident,
                error=self._error_body(
                    GESTURE_UNKNOWN, "Geste QGIS inconnu : %s" % command
                ),
            )
            return
        order = {"id": ident, "command": command, "gesture": gesture, "settled": False}
        self._current = order
        self._notify(ORDER_STARTED, command)

        def done(result=None, error=None):
            self._settle(order, result, error)

        try:
            gesture.start(params.get("params") or {}, done)
        except Exception as exc:
            self._settle(order, None, exc)

    def _settle(self, order, result, error):
        if order["settled"]:
            return  # un geste ne répond qu'une fois
        order["settled"] = True
        if self._current is order:
            self._current = None
        if error is not None:
            self._respond(order["id"], error=self._failure(error))
        else:
            self._respond(order["id"], result)
        self._notify(ORDER_ENDED, order["command"])

    def _failure(self, error):
        code = getattr(error, "code", None)
        message = getattr(error, "message", None)
        if not isinstance(code, str):
            code = GESTURE_FAILED
        if not isinstance(message, str):
            message = str(error) or "Le geste QGIS a échoué."
        return self._error_body(code, message)

    def _cancel_gesture(self, order):
        try:
            order["gesture"].cancel()
        except Exception as exc:
            self._settle(order, None, exc)

    def _abort_current(self):
        order = self._current
        if order is not None:
            order["settled"] = True  # plus de réponse : le canal se ferme
            self._current = None
            try:
                order["gesture"].cancel()
            except Exception:
                pass

    def _notify(self, kind, payload):
        if not self._closing:
            self._on_event(kind, payload)

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
            self._abort_current()
            self._finish(DIED, "Le noyau Cassini s'est arrêté.")

    def _error(self, *_):
        if not self._closing:
            self._abort_current()
            self._finish(DIED, "Le noyau Cassini n'a pas pu démarrer ou s'est arrêté.")
