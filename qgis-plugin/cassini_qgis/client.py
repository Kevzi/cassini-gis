"""Client du canal local : JSON-RPC 2.0, un objet JSON par ligne.

Le client ne contient aucune logique métier. Il lance le noyau (processus
enfant), envoie `session.hello` puis `command.run` de `system.about`, rend le
résultat par un rappel, puis sert d'exécutant au noyau : il traite
`order.execute` en lançant le geste enregistré et relaie `request.cancel`.
Il porte aussi les plans de l'agent (`plan.propose`, `plan.execute`,
`plan.reject`, notification `plan.step`) sans jamais les composer ni les
enchaîner : il n'envoie qu'une demande, le nom du modèle choisi ou un `plan_id`.
La liste des modèles vient du noyau par `agent.models` (noms et tailles seuls).
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
RUN_STARTED = "run_started"  # payload : {"command"}
RUN_DONE = "run_done"  # payload : {"command", "result"}
RUN_CANCELLED = "run_cancelled"  # payload : {"command"}
RUN_FAILED = "run_failed"  # payload : {"command", "code", "message"}
REPLAY_DONE = "replay_done"  # payload : {"steps", "identical"}
REPLAY_FAILED = "replay_failed"  # payload : {"code", "message", "step"}
# Plans de l'agent (docs/canal-local.md, « Plans de l'agent »).
PLAN_PROPOSED = "plan_proposed"  # payload : {"plan_id", "plan"}
PLAN_QUESTION = "plan_question"  # payload : {"question"}
PLAN_FAILED = "plan_failed"  # payload : {"code", "message"} (proposition ou refus)
PLAN_STEP = "plan_step"  # payload : {"step", "command"}
PLAN_DONE = "plan_done"  # payload : {"steps"}
# `outputs` : {étape: résultat} des ordres QGIS soldés avec succès pendant ce plan,
# gardés côté extension pour dire ce que les étapes finies ont fait (rien sur le fil).
PLAN_EXEC_FAILED = "plan_exec_failed"  # payload : {"code", "message", "step", "outputs"}
PLAN_CANCELLED = "plan_cancelled"  # payload : {"step", "outputs"}
PLAN_REJECTED = "plan_rejected"  # payload : {"plan_id"}
# Modèles de l'agent (`agent.models`) : noms de fichiers et tailles, jamais de chemin.
MODELS = "models"  # payload : {"models": [{"file", "size_bytes", "problem"?}, ...]}
MODELS_FAILED = "models_failed"  # payload : {"code", "message"}
MODEL_EVENTS = (MODELS, MODELS_FAILED)
PLAN_EVENTS = (
    PLAN_PROPOSED,
    PLAN_QUESTION,
    PLAN_FAILED,
    PLAN_STEP,
    PLAN_DONE,
    PLAN_EXEC_FAILED,
    PLAN_CANCELLED,
    PLAN_REJECTED,
)

GESTURE_UNKNOWN = "qgis.gesture_unknown"
GESTURE_FAILED = "qgis.gesture_failed"
BUSY = "qgis.busy"
CORE_DIED = "client.core_died"
CANCELLED = "registry.cancelled"


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
        self._catalogue = []  # entrées reçues à session.hello
        self._ready = False
        self._run = None  # command.run en vol : {"id", "command"}
        self._replay = None  # journal.replay en vol : id de la requête
        # plan.* en vol : {"id", "kind"} (propose, execute, reject) ; pour execute, aussi
        # "step" (étape annoncée par plan.step) et "outputs" ({étape: résultat d'ordre}).
        self._plan = None
        self._models = None  # agent.models en vol : id de la requête
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
        self._run = None
        self._replay = None
        self._plan = None
        self._models = None
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

    def catalogue(self):
        """Catalogue reçu à `session.hello` (liste d'entrées telles que le noyau les décrit)."""
        return list(self._catalogue)

    def run_busy(self):
        return self._run is not None

    def run_command(self, command, params=None):
        """Envoie `command.run` et rend la main aussitôt (AD-5 : une seule à la fois).

        La réponse arrive par le signal de lecture, sous forme d'un événement
        RUN_DONE, RUN_CANCELLED ou RUN_FAILED. Rend False si rien n'est parti.
        """
        if not self._can_send():
            return False
        ident = self._request(
            "command.run", {"command": command, "params": params or {}}, "run"
        )
        self._run = {"id": ident, "command": command}
        self._notify(RUN_STARTED, {"command": command})
        return True

    def replay_busy(self):
        return self._replay is not None

    def replay(self):
        """Demande au noyau de rejouer le journal du projet (`journal.replay`).

        Le noyau conduit tout : il réexécute les étapes et envoie ses ordres à
        l'extension, qui les exécute comme d'habitude (AD-11). Le rejeu n'est pas
        annulable. Le verdict arrive par REPLAY_DONE ou REPLAY_FAILED. Rend False
        si rien n'est parti (noyau pas prêt, commande ou rejeu déjà en vol).
        """
        if not self._can_send():
            return False
        self._replay = self._request("journal.replay", {}, "replay")
        return True

    def _can_send(self):
        """Une seule requête en vol côté client : commande, rejeu ou plan (AD-5)."""
        return (
            self._ready
            and self._run is None
            and self._replay is None
            and self._plan is None
            and self._models is None
            and not self._closing
        )

    def plan_busy(self):
        return self._plan is not None

    def models_busy(self):
        return self._models is not None

    def list_models(self):
        """Demande au noyau la liste des modèles (`agent.models`, hors journal).

        La liste arrive par MODELS, l'échec par MODELS_FAILED. Rend False si rien
        n'est parti (noyau pas prêt, ou une autre requête en vol).
        """
        if not self._can_send():
            return False
        self._models = self._request("agent.models", {}, "models")
        return True

    def propose(self, request, model=None):
        """Envoie `plan.propose` avec la demande en langage naturel, telle quelle.

        `model` : nom d'un modèle de la liste, envoyé seulement s'il est un texte ;
        sinon le noyau prend le premier. Le verdict arrive par PLAN_PROPOSED,
        PLAN_QUESTION ou PLAN_FAILED. La composition n'est pas annulable. Rend False
        si rien n'est parti.
        """
        if not self._can_send() or not isinstance(request, str):
            return False
        params = {"request": request}
        if isinstance(model, str) and model:
            params["model"] = model
        ident = self._request("plan.propose", params, "propose")
        self._plan = {"id": ident, "kind": "propose"}
        return True

    def execute_plan(self, plan_id):
        """Envoie `plan.execute` : seul le `plan_id` part, jamais le plan (AD-5, AD-23).

        Chaque étape s'annonce par PLAN_STEP ; le verdict arrive par PLAN_DONE,
        PLAN_EXEC_FAILED ou PLAN_CANCELLED. Rend False si rien n'est parti.
        """
        if not self._can_send() or not isinstance(plan_id, str):
            return False
        ident = self._request("plan.execute", {"plan_id": plan_id}, "execute")
        self._plan = {"id": ident, "kind": "execute", "step": None, "outputs": {}}
        return True

    def reject_plan(self, plan_id):
        """Envoie `plan.reject` ; le verdict arrive par PLAN_REJECTED ou PLAN_FAILED."""
        if not self._can_send() or not isinstance(plan_id, str):
            return False
        ident = self._request("plan.reject", {"plan_id": plan_id}, "reject")
        self._plan = {"id": ident, "kind": "reject"}
        return True

    def cancel_run(self):
        """Envoie `request.cancel` pour le `command.run` ou le `plan.execute` en vol.

        Ne tue jamais le noyau ; il refuse lui-même une étape non annulable (AD-12).
        """
        if self._closing:
            return False
        if self._run is not None:
            target = self._run["id"]
        elif self._plan is not None and self._plan["kind"] == "execute":
            target = self._plan["id"]
        else:
            return False
        self._request("request.cancel", {"id": target}, "cancel")
        return True

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
        return ident

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
        if step == "cancel":
            return  # le verdict arrive par la réponse du command.run
        if step == "run":
            self._run_answer(msg)
        elif step == "replay":
            self._replay_answer(msg)
        elif step in ("propose", "execute", "reject"):
            self._plan_answer(step, msg)
        elif step == "models":
            self._models_answer(msg)
        elif "error" in msg:
            self._fail(self._error_message(msg["error"]))
        elif step == "hello":
            result = msg.get("result") or {}
            commands = result.get("commands")
            self._catalogue = [c for c in commands if isinstance(c, dict)] if isinstance(commands, list) else []
            self._request("command.run", {"command": "system.about", "params": {}}, "about")
        elif step == "about":
            result = msg.get("result") or {}
            self._ready = True
            self._finish(
                READY,
                {
                    "core_version": result.get("core_version"),
                    "protocol_version": result.get("protocol_version"),
                    "commands": self.catalogue(),
                },
            )

    def _run_answer(self, msg):
        run, self._run = self._run, None
        command = run["command"] if run else None
        if "error" in msg:
            error = msg["error"]
            data = error.get("data") if isinstance(error, dict) else None
            code = data.get("code") if isinstance(data, dict) else None
            if code == CANCELLED:
                self._notify(RUN_CANCELLED, {"command": command})
                return
            self._notify(
                RUN_FAILED,
                {
                    "command": command,
                    "code": code if isinstance(code, str) else "core.unknown",
                    "message": self._error_message(error),
                },
            )
        else:
            self._notify(RUN_DONE, {"command": command, "result": msg.get("result")})

    def _replay_answer(self, msg):
        self._replay = None
        if "error" in msg:
            error = msg["error"]
            data = error.get("data") if isinstance(error, dict) else None
            code = data.get("code") if isinstance(data, dict) else None
            step = data.get("step") if isinstance(data, dict) else None
            self._notify(
                REPLAY_FAILED,
                {
                    "code": code if isinstance(code, str) else "core.unknown",
                    "message": self._error_message(error),
                    "step": step if isinstance(step, int) else None,
                },
            )
            return
        result = msg.get("result")
        result = result if isinstance(result, dict) else {}
        self._notify(
            REPLAY_DONE,
            {"steps": result.get("steps"), "identical": result.get("identical")},
        )

    @staticmethod
    def _error_fields(error):
        """(code, step) portés par `error.data` ; code `core.unknown` à défaut."""
        data = error.get("data") if isinstance(error, dict) else None
        code = data.get("code") if isinstance(data, dict) else None
        step = data.get("step") if isinstance(data, dict) else None
        return (
            code if isinstance(code, str) else "core.unknown",
            step if isinstance(step, int) and not isinstance(step, bool) else None,
        )

    def _plan_answer(self, kind, msg):
        plan, self._plan = self._plan, None
        outputs = dict(plan.get("outputs") or {}) if plan is not None else {}
        result = msg.get("result")
        if "error" in msg:
            error = msg["error"]
            code, step = self._error_fields(error)
            message = self._error_message(error)
            if kind == "execute":
                if code == CANCELLED:
                    self._notify(PLAN_CANCELLED, {"step": step, "outputs": outputs})
                else:
                    self._notify(
                        PLAN_EXEC_FAILED,
                        {"code": code, "message": message, "step": step, "outputs": outputs},
                    )
            else:
                self._notify(PLAN_FAILED, {"code": code, "message": message})
            return
        result = result if isinstance(result, dict) else {}
        if kind == "propose":
            question = result.get("question")
            plan_id = result.get("plan_id")
            plan = result.get("plan")
            if isinstance(question, str) and question.strip():
                self._notify(PLAN_QUESTION, {"question": question})
            elif isinstance(plan_id, str) and isinstance(plan, dict):
                self._notify(PLAN_PROPOSED, {"plan_id": plan_id, "plan": plan})
            else:
                self._notify(
                    PLAN_FAILED,
                    {"code": "core.unknown", "message": "Réponse du noyau illisible."},
                )
        elif kind == "execute":
            steps = result.get("steps")
            self._notify(PLAN_DONE, {"steps": steps if isinstance(steps, list) else []})
        else:
            self._notify(PLAN_REJECTED, {"plan_id": result.get("plan_id")})

    def _models_answer(self, msg):
        self._models = None
        if "error" in msg:
            code, _step = self._error_fields(msg["error"])
            self._notify(
                MODELS_FAILED, {"code": code, "message": self._error_message(msg["error"])}
            )
            return
        result = msg.get("result")
        listed = result.get("models") if isinstance(result, dict) else None
        if not isinstance(listed, list):
            self._notify(
                MODELS_FAILED, {"code": "core.unknown", "message": "Réponse du noyau illisible."}
            )
            return
        self._notify(MODELS, {"models": [m for m in listed if isinstance(m, dict)]})

    def _fail_models(self, message):
        if self._models is not None:
            self._models = None
            self._notify(MODELS_FAILED, {"code": CORE_DIED, "message": message})

    def _fail_plan(self, message):
        plan, self._plan = self._plan, None
        if plan is None:
            return
        if plan["kind"] == "execute":
            self._notify(
                PLAN_EXEC_FAILED,
                {
                    "code": CORE_DIED,
                    "message": message,
                    "step": None,
                    "outputs": dict(plan.get("outputs") or {}),
                },
            )
        else:
            self._notify(PLAN_FAILED, {"code": CORE_DIED, "message": message})

    def _fail_replay(self, message):
        if self._replay is not None:
            self._replay = None
            self._notify(
                REPLAY_FAILED, {"code": CORE_DIED, "message": message, "step": None}
            )

    def _fail_run(self, message):
        run, self._run = self._run, None
        if run is not None:
            self._notify(
                RUN_FAILED,
                {"command": run["command"], "code": CORE_DIED, "message": message},
            )

    def _handle_call(self, msg):
        method = msg.get("method")
        params = msg.get("params")
        if not isinstance(params, dict):
            params = {}
        ident = msg.get("id")
        if method == "order.execute" and ident is not None:
            self._execute(ident, params)
        elif method == "plan.step":
            # Notification : jamais de réponse, même si un id l'accompagnait.
            plan = self._plan
            step = params.get("step")
            if (
                plan is not None
                and plan["kind"] == "execute"
                and isinstance(step, int)
                and not isinstance(step, bool)
            ):
                plan["step"] = step  # le résultat du prochain ordre lui revient
                command = params.get("command")
                self._notify(
                    PLAN_STEP,
                    {"step": step, "command": command if isinstance(command, str) else None},
                )
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
            plan = self._plan
            if plan is not None and plan["kind"] == "execute" and plan.get("step") is not None:
                plan["outputs"][plan["step"]] = result
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
            self._fail_run("Le noyau Cassini s'est arrêté.")
            self._fail_replay("Le noyau Cassini s'est arrêté.")
            self._fail_plan("Le noyau Cassini s'est arrêté.")
            self._fail_models("Le noyau Cassini s'est arrêté.")
            self._finish(DIED, "Le noyau Cassini s'est arrêté.")

    def _error(self, *_):
        if not self._closing:
            self._abort_current()
            self._fail_run("Le noyau Cassini s'est arrêté.")
            self._fail_replay("Le noyau Cassini s'est arrêté.")
            self._fail_plan("Le noyau Cassini s'est arrêté.")
            self._fail_models("Le noyau Cassini s'est arrêté.")
            self._finish(DIED, "Le noyau Cassini n'a pas pu démarrer ou s'est arrêté.")
