"""Tests sans QGIS : Qt est remplacé par un double de QProcess."""

import json
import os
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
PKG = os.path.join(HERE, "..", "cassini_qgis")
sys.path.insert(0, PKG)

import client  # noqa: E402
import gestures  # noqa: E402
import installation  # noqa: E402,F401
import locator  # noqa: E402
import texts  # noqa: E402,F401


class Signal:
    def __init__(self):
        self.slots = []

    def connect(self, fn):
        self.slots.append(fn)

    def emit(self, *a):
        for s in self.slots:
            s(*a)


class FakeProcess:
    def __init__(self):
        self.readyReadStandardOutput = Signal()
        self.readyReadStandardError = Signal()
        self.finished = Signal()
        self.errorOccurred = Signal()
        self.started = None
        self.written = []
        self.out = b""
        self.err = b""
        self.err_reads = 0
        self.closed = False

    def start(self, program, args):
        self.started = (program, args)

    def write(self, data):
        self.written.append(data)

    def readAllStandardOutput(self):
        data, self.out = self.out, b""
        return data

    def readAllStandardError(self):
        self.err_reads += 1
        data, self.err = self.err, b""
        return data

    def waitForReadyRead(self, ms=0):  # jamais appelé par la boucle d'ordres
        raise AssertionError("lecture bloquante")

    def closeWriteChannel(self):
        self.closed = True

    def waitForFinished(self, ms):
        return True

    def feed(self, data):
        self.out += data
        self.readyReadStandardOutput.emit()


def reply(ident, result=None, error=None):
    m = {"jsonrpc": "2.0", "id": ident}
    if error is not None:
        m["error"] = error
    else:
        m["result"] = result
    return (json.dumps(m) + "\n").encode()


INSTALL = os.path.join("/h", ".local", "share", "cassini", "bin", "cassini")


class LocatorTest(unittest.TestCase):
    ENV = {"CASSINI_CORE": "/e", "PATH": "/bin"}

    def where(self, setting, existing):
        return locator.locate(setting, self.ENV, "linux", "/h", lambda p: p in existing)

    def test_order_is_fixed(self):
        all_ = {"/s", "/e", INSTALL, os.path.join("/bin", "cassini")}
        self.assertEqual(self.where("/s", all_)[0], "/s")
        self.assertEqual(self.where("", all_)[0], "/e")
        self.assertEqual(self.where("", all_ - {"/e"})[0], INSTALL)
        r = self.where("", {os.path.join("/bin", "cassini")})
        self.assertEqual(r[1], "PATH")

    def test_missing_setting_file_falls_through(self):
        self.assertEqual(self.where("/s", {"/e"})[0], "/e")

    def test_windows_install_path(self):
        p = locator.install_path("win32", {"LOCALAPPDATA": "C:\\L"}, "C:\\u")
        self.assertTrue(p.endswith(os.path.join("Cassini", "bin", "cassini.exe")))

    def test_not_found(self):
        self.assertIsNone(self.where("", set()))


class ClientTest(unittest.TestCase):
    def setUp(self):
        self.p = FakeProcess()
        self.events = []
        self.c = client.CoreClient(self.p, lambda k, v: self.events.append((k, v)))

    def sent(self):
        return [json.loads(w) for w in self.p.written]

    def test_nominal_one_json_per_line(self):
        self.c.start("cassini", "C:\\a\\p.qgz")
        self.assertEqual(self.p.started, ("cassini", ["serve"]))
        hello = self.sent()[0]
        self.assertTrue(self.p.written[0].endswith(b"\n"))
        self.assertEqual(hello["method"], "session.hello")
        self.assertEqual(hello["params"]["project"], "C:/a/p.qgz")
        self.assertEqual(hello["params"]["protocol_version"], 2)
        # Sans geste, aucune forme de style annoncée : le noyau n'en dicte aucune.
        self.assertEqual(hello["params"]["style_kinds"], [])
        catalogue = [{"id": "system.about", "available": True}]
        data = reply(1, {"protocol_version": 2, "core_version": "0.1.0",
                         "read_only": False, "commands": catalogue})
        self.p.feed(data[:10])  # réponse coupée : le cadrage par ligne tient
        self.assertEqual(self.events, [])
        self.p.feed(data[10:])
        # Les versions viennent du hello : aucun command.run, rien de journalisé.
        self.assertEqual(len(self.p.written), 1)
        self.assertEqual(
            self.events,
            [(client.READY, {"core_version": "0.1.0", "protocol_version": 2,
                             "commands": catalogue, "history": [], "read_only": False})],
        )

    def test_hello_announces_the_style_kinds_of_the_registry(self):
        registry = gestures.GestureRegistry()
        registry.register("layer.set_style", object)
        registry.announce_style_kinds(["result_ramp", "hillshade"])
        c = client.CoreClient(self.p, lambda k, v: None, registry)
        c.start("cassini", "C:\\a\\p.qgz")
        hello = self.sent()[0]
        self.assertEqual(hello["params"]["commands"], ["layer.set_style"])
        self.assertEqual(hello["params"]["style_kinds"], ["hillshade", "result_ramp"])

    def test_hello_history_reaches_ready_filtered(self):
        self.c.start("cassini", "/p.qgz")
        past = [{"step": 1, "command": "system.about"}, {"step": 2, "command": "layer.add"}]
        self.p.feed(reply(1, {"commands": [], "history": past[:1] + ["pas un objet"] + past[1:]}))
        self.assertEqual(self.events[0][1]["history"], past)

    def test_hello_without_versions_is_ready_with_none(self):
        self.c.start("cassini", "/p.qgz")
        self.p.feed(reply(1, {"commands": []}))
        self.assertEqual(
            self.events,
            [(client.READY, {"core_version": None, "protocol_version": None, "commands": [],
                             "history": [], "read_only": False})],
        )
        self.assertEqual(len(self.p.written), 1)
        self.assertFalse(self.p.closed)

    def test_writing_session_is_ready(self):
        self.c.start("cassini", "/p.qgz")
        self.p.feed(reply(1, {"protocol_version": 2, "core_version": "0.1.0",
                              "read_only": False, "commands": []}))
        self.assertEqual([k for k, _ in self.events], [client.READY])
        self.assertFalse(self.p.closed)

    def read_only_session(self):
        self.c.start("cassini", "/p.qgz")
        message = "Le journal de ce projet est tenu par une autre session : lecture seule."
        past = [{"step": 1, "command": "system.about"}]
        self.p.feed(reply(1, {"protocol_version": 2, "core_version": "0.1.0",
                              "read_only": True, "message": message,
                              "commands": [{"id": "system.about"}], "history": past}))
        return past

    def test_read_only_session_stays_open_and_ready(self):
        # Seconde fenêtre QGIS : le noyau reste vivant, la session dit qu'elle est en
        # lecture seule (AD-11) ; rien n'est écrit au journal.
        past = self.read_only_session()
        self.assertEqual(
            self.events,
            [(client.READY, {"core_version": "0.1.0", "protocol_version": 2,
                             "commands": [{"id": "system.about"}], "history": past,
                             "read_only": True})],
        )
        self.assertFalse(self.p.closed)
        self.assertTrue(self.c.read_only())
        self.assertEqual(len(self.p.written), 1)

    def test_read_only_session_sends_only_the_replay(self):
        self.read_only_session()
        self.assertFalse(self.c.run_command("system.about"))
        self.assertFalse(self.c.propose("Tampon"))
        self.assertFalse(self.c.execute_plan("plan-1"))
        self.assertFalse(self.c.reject_plan("plan-1"))
        self.assertFalse(self.c.list_models())
        self.assertFalse(self.c.set_external(True))
        self.assertEqual(len(self.p.written), 1)  # seule la poignée de main est partie
        self.assertTrue(self.c.replay())
        self.assertEqual(self.sent()[-1]["method"], "journal.replay")
        self.p.feed(reply(2, {"steps": 1, "identical": True}))
        self.assertEqual(self.events[-1], (client.REPLAY_DONE, {"steps": 1, "identical": True}))

    def test_read_only_replay_error_carries_the_core_message(self):
        self.read_only_session()
        self.assertTrue(self.c.replay())
        err = {"code": -32000, "message": "x",
               "data": {"code": "replay.no_journal", "message": "Aucun journal."}}
        self.p.feed(reply(2, error=err))
        self.assertEqual(
            self.events[-1],
            (client.REPLAY_FAILED, {"code": "replay.no_journal", "message": "Aucun journal.",
                                    "step": None}),
        )

    def test_writing_session_is_not_read_only(self):
        self.c.start("cassini", "/p.qgz")
        self.p.feed(reply(1, {"read_only": False, "commands": []}))
        self.assertFalse(self.c.read_only())
        self.assertTrue(self.c.run_command("system.about"))

    def test_mismatch_shown_as_is_and_core_stopped(self):
        self.c.start("cassini", "/p.qgz")
        err = {
            "code": -32000,
            "message": "x",
            "data": {"code": "session.protocol_mismatch", "message": "Mettez à jour."},
        }
        self.p.feed(reply(1, error=err))
        self.assertEqual(self.events, [(client.FAILED, "Mettez à jour.")])
        self.assertTrue(self.p.closed)

    def test_core_dies_before_answer(self):
        self.c.start("cassini", "/p.qgz")
        self.p.finished.emit(1, 0)
        self.assertEqual(self.events[0][0], client.DIED)
        self.p.errorOccurred.emit(1)
        self.assertEqual(len(self.events), 1)

    def test_stop_closes_stdin_without_death_event(self):
        self.c.start("cassini", "/p.qgz")
        self.c.stop()
        self.assertTrue(self.p.closed)
        self.p.finished.emit(0, 0)
        self.assertEqual(self.events, [])

    def test_garbage_and_malformed_orders_ignored(self):
        self.c.start("cassini", "/p.qgz")
        self.p.feed(b"pas du json\n")
        self.p.feed(b'{"jsonrpc":"2.0","method":"order.execute"}\n')
        self.assertEqual(self.events, [])
        self.assertEqual(len(self.p.written), 1)


class FakeGesture:
    """Geste lent factice : ne se termine que sur ordre du test."""

    instances = []

    def __init__(self):
        self.done = None
        self.params = None
        self.cancelled = 0
        FakeGesture.instances.append(self)

    def start(self, params, done):
        self.params = params
        self.done = done

    def cancel(self):
        self.cancelled += 1
        self.done(error=gestures.GestureError("qgis.cancelled", "Annulé."))


class Raising:
    def start(self, params, done):
        raise RuntimeError("boum")

    def cancel(self):
        pass


class OrderTest(unittest.TestCase):
    def setUp(self):
        FakeGesture.instances = []
        self.reg = gestures.GestureRegistry()
        self.reg.register("test.qgis_probe", FakeGesture)
        self.reg.register("test.raising", Raising)
        self.p = FakeProcess()
        self.events = []
        self.c = client.CoreClient(
            self.p, lambda k, v: self.events.append((k, v)), self.reg
        )
        self.c.start("cassini", "/p.qgz")
        self.p.feed(reply(1, {"protocol_version": 2, "core_version": "0.1", "commands": []}))
        self.base = len(self.p.written)

    def order(self, ident="core-1", command="test.qgis_probe", params=None):
        m = {"jsonrpc": "2.0", "id": ident, "method": "order.execute",
             "params": {"command": command, "params": params or {"layer": "l"}}}
        self.p.feed((json.dumps(m) + "\n").encode())

    def cancel(self, ident="core-1", request_id=None):
        m = {"jsonrpc": "2.0", "method": "request.cancel", "params": {"id": ident}}
        if request_id is not None:
            m["id"] = request_id
        self.p.feed((json.dumps(m) + "\n").encode())

    def new(self):
        return [json.loads(w) for w in self.p.written[self.base:]]

    def test_hello_announces_gesture_ids(self):
        hello = json.loads(self.p.written[0])
        self.assertEqual(hello["params"]["commands"], ["test.qgis_probe", "test.raising"])

    def test_nominal_order(self):
        self.order()
        self.assertEqual(self.new(), [])  # lancé, pas encore fini : rien ne bloque
        self.assertEqual(FakeGesture.instances[0].params, {"layer": "l"})
        self.assertTrue(self.c.busy())
        FakeGesture.instances[0].done(result={"ok": True})
        self.assertEqual(self.new(), [{"jsonrpc": "2.0", "id": "core-1", "result": {"ok": True}}])
        self.assertFalse(self.c.busy())
        self.assertEqual(
            [k for k, _ in self.events],
            [client.READY, client.ORDER_STARTED, client.ORDER_ENDED],
        )

    def test_cancel_gets_a_response(self):
        self.order()
        self.cancel()
        sent = self.new()
        self.assertEqual(len(sent), 1)
        self.assertEqual(sent[0]["id"], "core-1")
        self.assertEqual(sent[0]["error"]["data"]["code"], "qgis.cancelled")
        self.assertFalse(self.p.closed)  # le noyau n'est pas tué
        self.assertFalse(self.c.busy())

    def test_cancel_as_request_is_answered(self):
        self.order()
        self.cancel(request_id="core-9")
        answers = [m for m in self.new() if m["id"] == "core-9"]
        self.assertEqual(answers[0]["result"], {"cancelled": True})

    def test_cancel_request_with_other_id_answers_false(self):
        self.order()
        self.cancel("core-7", request_id="core-9")
        answers = [m for m in self.new() if m["id"] == "core-9"]
        self.assertEqual(answers[0]["result"], {"cancelled": False})
        self.assertEqual(FakeGesture.instances[0].cancelled, 0)

    def test_cancel_unknown_id_ignored(self):
        self.cancel("core-7")
        self.assertEqual(self.new(), [])
        self.order()
        self.cancel("core-7")
        self.assertEqual(FakeGesture.instances[0].cancelled, 0)
        self.assertEqual(self.new(), [])

    def test_unknown_gesture(self):
        self.order(command="nope")
        err = self.new()[0]["error"]
        self.assertEqual(err["code"], -32000)
        self.assertEqual(err["data"]["code"], "qgis.gesture_unknown")
        self.assertFalse(self.c.busy())

    def test_gesture_raising_is_reported_not_propagated(self):
        self.order(command="test.raising")
        err = self.new()[0]["error"]
        self.assertEqual(err["code"], -32000)
        self.assertEqual(err["data"]["code"], "qgis.gesture_failed")
        self.assertEqual(err["data"]["message"], "boum")
        self.assertFalse(self.c.busy())

    def test_order_during_order_is_refused(self):
        self.order("core-1")
        self.order("core-2")
        sent = self.new()
        self.assertEqual(sent[0]["id"], "core-2")
        self.assertEqual(sent[0]["error"]["data"]["code"], "qgis.busy")
        self.assertEqual(len(FakeGesture.instances), 1)
        FakeGesture.instances[0].done(result={"ok": True})
        self.assertEqual(self.new()[1]["id"], "core-1")

    def test_second_done_is_ignored(self):
        self.order()
        g = FakeGesture.instances[0]
        g.done(result=1)
        g.done(result=2)
        self.assertEqual(len(self.new()), 1)

    def test_local_stop_button(self):
        self.order()
        self.c.cancel_current()
        self.assertEqual(self.new()[0]["error"]["data"]["code"], "qgis.cancelled")

    def test_core_death_cancels_gesture_without_answer(self):
        self.order()
        self.p.finished.emit(1, 0)
        self.assertEqual(FakeGesture.instances[0].cancelled, 1)
        self.assertEqual(self.new(), [])
        self.assertEqual(self.events[-1][0], client.DIED)

    def test_noisy_stderr_drained_at_each_signal(self):
        for _ in range(50):
            self.p.err = b"x" * 65536
            self.p.readyReadStandardError.emit()
            self.assertEqual(self.p.err, b"")
        self.assertEqual(self.p.err_reads, 50)
        self.order()  # stdout continue d'être lu
        self.assertTrue(self.c.busy())

    def test_unknown_request_gets_method_not_found(self):
        m = {"jsonrpc": "2.0", "id": "core-3", "method": "autre", "params": {}}
        self.p.feed((json.dumps(m) + "\n").encode())
        self.assertEqual(self.new()[0]["error"]["code"], -32601)


class RunTest(unittest.TestCase):
    CATALOGUE = [
        {"id": "system.about", "available": True, "cancellable": False,
         "params_schema": {"type": "object"}},
        {"id": "test.qgis_probe", "available": True, "cancellable": True},
    ]

    def setUp(self):
        self.p = FakeProcess()
        self.events = []
        self.c = client.CoreClient(self.p, lambda k, v: self.events.append((k, v)))
        self.c.start("cassini", "/p.qgz")

    def ready(self):
        self.p.feed(reply(1, {"protocol_version": 2, "core_version": "0.1",
                              "commands": self.CATALOGUE}))
        self.base = len(self.p.written)

    def new(self):
        return [json.loads(w) for w in self.p.written[self.base:]]

    def kinds(self):
        return [k for k, _ in self.events]

    def test_catalogue_comes_with_ready(self):
        self.ready()
        self.assertEqual(self.events[0][1]["commands"], self.CATALOGUE)
        self.assertEqual(self.c.catalogue(), self.CATALOGUE)

    def test_run_before_ready_sends_nothing(self):
        self.assertFalse(self.c.run_command("system.about"))
        self.assertEqual(len(self.p.written), 1)

    def test_run_done(self):
        self.ready()
        self.assertTrue(self.c.run_command("system.about", {}))
        sent = self.new()[0]
        self.assertEqual(sent["method"], "command.run")
        self.assertEqual(sent["params"], {"command": "system.about", "params": {}})
        self.assertTrue(self.c.run_busy())
        self.p.feed(reply(sent["id"], {"core_version": "0.1.0"}))
        self.assertEqual(self.kinds()[-2:], [client.RUN_STARTED, client.RUN_DONE])
        self.assertEqual(
            self.events[-1],
            (client.RUN_DONE, {"command": "system.about", "result": {"core_version": "0.1.0"}}),
        )
        self.assertFalse(self.c.run_busy())

    def test_one_command_at_a_time(self):
        self.ready()
        self.assertTrue(self.c.run_command("test.qgis_probe"))
        self.assertFalse(self.c.run_command("system.about"))
        self.assertEqual(len(self.new()), 1)

    def test_core_error_is_an_event_not_an_exception(self):
        self.ready()
        self.c.run_command("system.about", {"x": 1})
        err = {"code": -32000, "message": "m",
               "data": {"code": "registry.invalid_params", "message": "Paramètres refusés."}}
        self.p.feed(reply(self.new()[0]["id"], error=err))
        kind, payload = self.events[-1]
        self.assertEqual(kind, client.RUN_FAILED)
        self.assertEqual(payload["code"], "registry.invalid_params")
        self.assertEqual(payload["message"], "Paramètres refusés.")
        self.assertFalse(self.c.run_busy())
        self.assertFalse(self.p.closed)

    def test_cancel_sends_request_cancel_for_the_run_in_flight(self):
        self.ready()
        self.c.run_command("test.qgis_probe")
        run_id = self.new()[0]["id"]
        self.assertTrue(self.c.cancel_run())
        cancel = self.new()[1]
        self.assertEqual(cancel["method"], "request.cancel")
        self.assertEqual(cancel["params"], {"id": run_id})
        self.assertIn("id", cancel)  # une requête, pas une notification
        self.assertFalse(self.p.closed)  # le noyau n'est pas tué
        self.p.feed(reply(cancel["id"], {"cancelled": True}))
        self.assertNotIn(client.RUN_CANCELLED, self.kinds())
        err = {"code": -32000, "message": "m",
               "data": {"code": "registry.cancelled", "message": "Annulée."}}
        self.p.feed(reply(run_id, error=err))
        self.assertEqual(self.events[-1], (client.RUN_CANCELLED, {"command": "test.qgis_probe"}))
        self.assertFalse(self.c.run_busy())
        self.assertTrue(self.c.run_command("system.about"))  # libre aussitôt

    def test_cancel_refused_keeps_the_run(self):
        self.ready()
        self.c.run_command("system.about")
        self.c.cancel_run()
        cancel = self.new()[1]
        err = {"code": -32000, "message": "m",
               "data": {"code": "request.not_cancellable", "message": "Non."}}
        self.p.feed(reply(cancel["id"], error=err))
        self.assertTrue(self.c.run_busy())
        self.assertNotIn(client.RUN_FAILED, self.kinds())
        self.assertFalse(self.p.closed)

    def test_cancel_without_run_sends_nothing(self):
        self.ready()
        self.assertFalse(self.c.cancel_run())
        self.assertEqual(self.new(), [])

    def test_core_death_fails_the_run(self):
        self.ready()
        self.c.run_command("test.qgis_probe")
        self.p.finished.emit(1, 0)
        self.assertEqual(self.kinds()[-2:], [client.RUN_FAILED, client.DIED])
        self.assertEqual(self.events[-2][1]["code"], client.CORE_DIED)


class PluginSourceTest(unittest.TestCase):
    def source(self, name):
        with open(os.path.join(PKG, name), encoding="utf-8") as f:
            return f.read()

    def test_forbidden_imports(self):
        for name in os.listdir(PKG):
            if name.endswith(".py"):
                src = self.source(name)
                for banned in ("PyQt5", "PyQt6", "urllib", "requests", "import socket", "http"):
                    self.assertNotIn(banned, src, name)

    def test_metadata_versions(self):
        meta = self.source("metadata.txt")
        self.assertIn("qgisMinimumVersion=3.44", meta)
        self.assertIn("qgisMaximumVersion=4.99", meta)


class Sig:
    def connect(self, f):
        pass

    def disconnect(self, f):
        pass


class StubProject:
    name = ""
    readProject = projectSaved = cleared = Sig()

    @classmethod
    def instance(cls):
        return cls

    @classmethod
    def fileName(cls):
        return cls.name


class StubProcess:
    created = 0

    def __init__(self):
        StubProcess.created += 1


class StubWidget:
    def __init__(self, *a):
        pass


class FakePanel:
    def __init__(self, actions):
        self.lang = "fr"
        self.banners = []
        self.messages = []
        self.ready = None
        self.events = []
        self.install_infos = []

    def set_banner(self, kind, message=None):
        self.banners.append(kind)
        self.messages.append(message)

    def set_install_info(self, info, reminder=False):
        self.install_infos.append((info, reminder))

    def set_ready(self, catalogue, history=(), read_only=False):
        self.ready = catalogue
        self.history = list(history)
        self.read_only = read_only

    def on_event(self, kind, payload):
        self.events.append((kind, payload))


class StubTimer:
    """QTimer muet : garde l'intervalle et l'état ; `fire()` simule un tick."""

    def __init__(self):
        self.interval = None
        self.active = False
        self.slots = []
        self.timeout = self

    def connect(self, slot):
        self.slots.append(slot)

    def setInterval(self, ms):
        self.interval = ms

    def start(self):
        self.active = True

    def stop(self):
        self.active = False

    def fire(self):
        for slot in self.slots:
            slot()


class StubDesktop:
    opened = []

    @staticmethod
    def openUrl(url):
        StubDesktop.opened.append(url)


def load_plugin():
    import importlib
    import types

    def mod(name, **attrs):
        m = types.ModuleType(name)
        m.__dict__.update(attrs)
        sys.modules[name] = m
        return m

    mod("qgis")
    mod("qgis.core", QgsApplication=object, QgsProject=StubProject)
    mod("qgis.PyQt")
    mod("qgis.PyQt.QtCore", QProcess=StubProcess, QTimer=StubTimer, QSettings=type(
        "S", (), {"value": lambda self, *a: "", "setValue": lambda self, *a: None}), Qt=object,
        QUrl=lambda url: url)
    mod("qgis.PyQt.QtGui", QDesktopServices=StubDesktop)
    mod("qgis.PyQt.QtWidgets", QDockWidget=StubWidget, QFileDialog=StubWidget)
    pkg = types.ModuleType("cassini_stub")
    pkg.__path__ = [PKG]
    sys.modules["cassini_stub"] = pkg
    for n in ("client", "gestures", "installation", "locator", "texts"):
        sys.modules["cassini_stub." + n] = sys.modules[n]
    mod("cassini_stub.panel", CassiniPanel=FakePanel)
    return importlib.import_module("cassini_stub.plugin")


class Recorder:
    """Widget muet : tout attribut est un enregistreur appelable ; `calls` garde ses appels."""

    def __init__(self, name=None, parent=None):
        self.__dict__["calls"] = []
        self.__dict__["_name"] = name
        self.__dict__["_parent"] = parent

    def __getattr__(self, name):
        if name.startswith("__"):
            raise AttributeError(name)
        child = Recorder(name, self)
        self.__dict__[name] = child
        return child

    def __call__(self, *args):
        if self._parent is not None:
            self._parent.calls.append((self._name, args))
        return Recorder()


def load_panel():
    """Charge le vrai panneau, Qt remplacé par des doubles muets."""
    import importlib
    import types

    class Stub:
        def __init__(self, *a, **k):
            pass

    def mod(name):
        m = types.ModuleType(name)
        m.__getattr__ = lambda attr: Stub
        sys.modules[name] = m

    for name in ("qgis", "qgis.PyQt", "qgis.PyQt.QtCore", "qgis.PyQt.QtGui",
                 "qgis.PyQt.QtWidgets"):
        mod(name)
    pkg = types.ModuleType("cassini_panel_stub")
    pkg.__path__ = [PKG]
    sys.modules["cassini_panel_stub"] = pkg
    return importlib.import_module("cassini_panel_stub.panel")


class PanelCoreRasterTest(unittest.TestCase):
    """Le panneau n'ouvre plus aucun raster de lui-même : le noyau dicte `layer.open_raster`
    (ticket 4.9, qui retire le branchement du ticket 4.3)."""

    def setUp(self):
        import tempfile
        import types

        def stubbed(k):
            return k in ("qgis", "cassini_panel_stub") or k.startswith(
                ("qgis.", "cassini_panel_stub."))

        saved = {k: v for k, v in sys.modules.items() if stubbed(k)}

        def restore():
            for k in [k for k in sys.modules if stubbed(k)]:
                del sys.modules[k]
            sys.modules.update(saved)

        self.addCleanup(restore)
        self.mod = load_panel()
        mod = self.mod
        cls = mod.CassiniPanel
        panel = self.panel = cls.__new__(cls)
        panel.lang = "fr"
        panel.focus = None
        panel.running = True
        panel._run_params = None
        panel.result_text = None
        panel.render = lambda: None
        panel.log = mod.runlog.RunLog()
        panel.catalogue = [{"id": "terrain.slope", "runs_in": "core"}]
        panel.agent = types.SimpleNamespace(done=lambda steps: None, steps=[])
        panel._plan_refs = {}
        panel._close_plan_entry = lambda *a: None
        panel._keep_plan_results = lambda *a: None
        panel.external = None
        mod._announce = lambda widget, text: None
        mod.agentview.outcome = lambda status, payload, agent: {"kind": status}
        mod.external.apply = lambda *a: {"resend_ask": False, "announce": None, "where": None}
        # Le panneau ne garde aucun branchement d'ouverture (ticket 4.9).
        self.assertFalse(hasattr(mod, "raster_output"))
        self.assertFalse(hasattr(cls, "_open_core_rasters"))
        folder = tempfile.TemporaryDirectory()
        self.addCleanup(folder.cleanup)
        self.dir = folder.name

    def slope(self, step):
        path = os.path.join(self.dir, "%d-terrain.slope.tif" % step)
        with open(path, "wb") as f:
            f.write(b"II*\x00")
        return path

    def test_events_carrying_a_raster_are_handled_without_opening_it(self):
        path = self.slope(1)
        self.panel.on_event(client.RUN_DONE, {"command": "terrain.slope", "result": {"file": path}})
        steps = [{"step": 2, "command": "terrain.slope", "result": {"file": self.slope(2)}}]
        self.panel.on_event(client.PLAN_DONE, {"plan_id": "plan-1", "steps": steps})
        payload = {"call_id": "external-1", "command": "terrain.slope",
                   "result": {"file": self.slope(3)}, "error": None, "duration_ms": 5}
        self.panel.on_event(client.EXTERNAL_CALL_FINISHED, payload)


class PanelBannerTest(unittest.TestCase):
    """Le bandeau `refused` montre le message du noyau tel quel (5.32)."""

    def setUp(self):
        def stubbed(k):
            return k in ("qgis", "cassini_panel_stub") or k.startswith(
                ("qgis.", "cassini_panel_stub."))

        saved = {k: v for k, v in sys.modules.items() if stubbed(k)}

        def restore():
            for k in [k for k in sys.modules if stubbed(k)]:
                del sys.modules[k]
            sys.modules.update(saved)

        self.addCleanup(restore)
        self.mod = load_panel()
        cls = self.mod.CassiniPanel
        self.panel = cls.__new__(cls)
        self.panel.lang = "fr"
        self.panel.banner = None
        self.panel.banner_message = None
        self.panel.focus = None
        self.panel.ready = True
        self.panel.title = None
        self.panel.banner_text = None
        self.panel.render = lambda: None
        self.panel._drop_stale_run = lambda: None
        self.announced = []
        real = self.mod._announce
        self.addCleanup(setattr, self.mod, "_announce", real)
        self.mod._announce = lambda widget, text: self.announced.append(text)

    def test_read_only_message_shown_as_is(self):
        message = "Le journal de ce projet est tenu par une autre session : lecture seule."
        self.panel.set_banner("refused", message)
        self.assertEqual(self.panel.banner, "refused")
        self.assertFalse(self.panel.ready)
        self.assertEqual(self.panel._banner_text(self.panel.t), message)
        self.assertEqual(self.announced, [message])

    def rendered_banner(self, message):
        """Lance le vrai `render()` (widgets enregistreurs) et rend les appels du bandeau."""
        import types

        panel = self.panel
        panel.set_banner("refused", message)
        del panel.render  # le vrai render de la classe
        for name in ("title", "settings_button", "settings_zone", "lang_label", "lang_box",
                     "model_label", "banner_zone", "banner_text", "banner_button", "banner_home",
                     "install_label", "install_note", "input",
                     "send_button", "details", "expert", "footer"):
            setattr(panel, name, Recorder())
        for name in ("_render_models", "_render_external", "_render_agent", "_render_result",
                     "_render_thread", "_render_log", "_render_commands", "_render_buttons"):
            setattr(panel, name, lambda: None)
        panel._controls = lambda: {"expert_enabled": False}  # activations : hors de ce test
        panel._focus_inside = lambda: False  # guidage du focus [12.2] : hors de ce test
        panel._move_focus = lambda inside: None
        panel.log = types.SimpleNamespace(entries=[])
        panel.agent = types.SimpleNamespace(phase=self.mod.agentview.IDLE)
        panel.render()
        self.assertIn(("setVisible", (True,)), panel.banner_zone.calls)
        return panel.banner_text.calls

    def test_render_shows_the_core_message_in_the_banner(self):
        message = "Le journal de ce projet est tenu par une autre session : lecture seule."
        calls = self.rendered_banner(message)
        self.assertIn(("setText", (message,)), calls)
        self.assertIn(("setAccessibleName", (message,)), calls)

    def test_render_falls_back_to_refused_on_empty_message(self):
        refused = texts.tr("fr", "refused")
        calls = self.rendered_banner("")
        self.assertIn(("setText", (refused,)), calls)
        self.assertIn(("setAccessibleName", (refused,)), calls)

    def test_empty_or_missing_message_falls_back_to_refused(self):
        refused = texts.tr("fr", "refused")
        for message in ("", None):
            self.announced.clear()
            self.panel.set_banner("refused", message)
            self.assertIsNone(self.panel.banner_message)
            self.assertEqual(self.panel._banner_text(self.panel.t), refused)
            self.assertEqual(self.announced, [refused])

    def test_read_only_session_is_ready_with_the_spine_banner(self):
        # Seconde fenêtre QGIS (AD-11) : prêt, bandeau de lecture seule, journal affiché.
        catalogue = [{"id": "system.about", "description_fr": "Version du noyau."}]
        past = [{"step": 1, "command": "system.about", "params": {}, "result": {}}]
        panel = self.panel
        panel.project_file = "C:/a/ro.qgz"
        panel.set_ready(catalogue, past, read_only=True)
        self.assertTrue(panel.ready)
        self.assertEqual(panel.banner, "read_only")
        self.assertTrue(panel.read_only)
        self.assertEqual(len(panel.log.entries), 1)  # l'historique comme d'habitude
        text = texts.tr("fr", "read_only")
        self.assertEqual(panel._banner_text(panel.t), text)
        self.assertEqual(self.announced, [text])
        # Relancer : reprendre l'écriture une fois la première fenêtre fermée.
        self.assertEqual(self.mod.BANNERS["read_only"][1], "relaunch")
        # Session normale : inchangée.
        self.announced.clear()
        panel.set_ready(catalogue, past)
        self.assertIsNone(panel.banner)
        self.assertFalse(panel.read_only)
        self.assertEqual(self.announced, [texts.tr("fr", "live_ready")])
        # Le noyau meurt ensuite : plus de lecture seule, bandeau « died ».
        panel.set_ready(catalogue, past, read_only=True)
        panel.set_banner("died")
        self.assertFalse(panel.read_only)

    def test_set_ready_fills_the_log_from_history_once_per_project(self):
        catalogue = [{"id": "system.about", "description_fr": "Version du noyau."}]
        past = [{"step": 1, "command": "system.about", "params": {}, "result": {}}]
        panel = self.panel
        panel.project_file = "C:/a/p.qgz"
        panel.focus = ("entry", object())
        panel.last = {"kind": "done"}
        panel.set_ready(catalogue, past)
        self.assertEqual([e.label for e in panel.log.entries], ["Vérification du noyau Cassini"])
        self.assertIsNone(panel.focus)
        self.assertIsNone(panel.last)
        # Noyau relancé sur le même projet : les lignes de la session restent.
        panel.log.start("analysis.buffer", "Tampon", "14:02")
        panel.set_ready(catalogue, past)
        self.assertEqual(len(panel.log.entries), 2)
        # Un autre projet : son journal remplace celui du précédent.
        panel.project_file = "C:/a/q.qgz"
        panel.set_ready(catalogue, [])
        self.assertEqual(panel.log.entries, [])

    def test_set_ready_clears_the_message(self):
        self.panel.set_banner("refused", "lecture seule")
        self.panel.set_ready([])
        self.assertIsNone(self.panel.banner)
        self.assertIsNone(self.panel.banner_message)
        self.panel.set_banner("died")
        self.assertEqual(self.panel._banner_text(self.panel.t), texts.tr("fr", "died"))


class PluginBehaviourTest(unittest.TestCase):
    def setUp(self):
        self.mod = load_plugin()
        self.p = self.mod.CassiniPlugin(None)
        self.p.panel = FakePanel(self.p)
        StubProcess.created = 0
        self.addCleanup(setattr, self.mod.locator, 'locate', self.mod.locator.locate)

    def fake_client(self):
        real = self.mod.client.CoreClient
        self.addCleanup(setattr, self.mod.client, 'CoreClient', real)
        self.gestures = None

        def init(s, proc, cb, reg=None):
            self.gestures = reg

        self.mod.client.CoreClient = type("C", (), {
            "__init__": init,
            "start": lambda s, prog, proj: None,
            "stop": lambda s: None})

    def test_no_project(self):
        StubProject.name = ""
        self.p._refresh()
        self.assertEqual(self.p.panel.banners, ["no_project"])
        self.assertEqual(StubProcess.created, 0)

    def test_core_not_found(self):
        StubProject.name = "/a/p.qgz"
        self.mod.locator.locate = lambda s: None
        self.p._refresh()
        self.assertEqual(self.p.panel.banners, ["not_found"])
        self.assertEqual(StubProcess.created, 0)

    def test_not_found_probes_every_3_seconds_then_launches(self):
        StubProject.name = "/a/p.qgz"
        found = [None]
        self.mod.locator.locate = lambda s: found[0]
        self.fake_client()
        self.p._refresh()
        probe = self.p._probe
        self.assertEqual((probe.interval, probe.active), (3000, True))
        probe.fire()  # toujours rien : le bandeau reste
        self.assertEqual(StubProcess.created, 0)
        found[0] = ("/x/cassini", "install")
        probe.fire()
        self.assertEqual(StubProcess.created, 1)
        self.assertFalse(probe.active)
        self.assertIsNone(self.p._probe)
        self.assertEqual(self.p.panel.banners[-1], "starting")

    def test_not_found_clears_install_info_and_closing_project_stops_probe(self):
        StubProject.name = "/a/p.qgz"
        self.mod.locator.locate = lambda s: None
        self.fake_client()
        self.p._refresh()
        self.assertEqual(self.p.panel.install_infos[-1], (None, False))
        probe = self.p._probe
        self.assertTrue(probe.active)
        StubProject.name = ""
        self.p._refresh()
        self.assertFalse(probe.active)
        self.assertIsNone(self.p._probe)

    def test_launch_passes_install_info_to_the_panel(self):
        StubProject.name = "/a/p.qgz"
        self.mod.locator.locate = lambda s: ("/nowhere/cassini", "PATH")
        self.fake_client()
        self.p._refresh()
        self.assertEqual(self.p.panel.install_infos, [(None, False)])

    def test_open_home_uses_desktop_services(self):
        StubDesktop.opened.clear()
        self.p.open_home()
        self.assertEqual(StubDesktop.opened, [installation.HOME_URL])

    def test_died_allows_relaunch(self):
        StubProject.name = "/a/p.qgz"
        self.mod.locator.locate = lambda s: ("/x/cassini", "PATH")
        self.fake_client()
        self.p._refresh()
        self.assertEqual(StubProcess.created, 1)
        self.p._on_event(self.mod.client.DIED, "mort")
        self.assertEqual(self.p.panel.banners[-1], "died")
        self.assertIsNone(self.p.project_path)
        self.p.relaunch()
        self.assertEqual(StubProcess.created, 2)

    def test_launch_registers_the_raster_opening_gesture(self):
        # Le noyau dicte `layer.open_raster` (ticket 4.9) : sans ce geste annoncé, plus aucun
        # raster du noyau n'entrerait au projet.
        StubProject.name = "/a/p.qgz"
        self.mod.locator.locate = lambda s: ("/x/cassini", "PATH")
        self.fake_client()
        self.p._refresh()
        self.assertIn("layer.open_raster", self.gestures.ids())
        self.assertIsInstance(
            self.gestures.create("layer.open_raster"),
            self.mod.delegated.raster_output.LayerOpenRasterGesture,
        )
        # De même pour un GeoJSON du noyau, dicté par `layer.open_vector` (AD-30).
        self.assertIn("layer.open_vector", self.gestures.ids())
        self.assertIsInstance(
            self.gestures.create("layer.open_vector"),
            self.mod.delegated.vector_output.LayerOpenVectorGesture,
        )

    def test_protocol_refusal_is_not_a_relaunchable_death(self):
        StubProject.name = "/a/p.qgz"
        self.mod.locator.locate = lambda s: ("/x/cassini", "PATH")
        self.fake_client()
        self.p._refresh()
        self.p._on_event(self.mod.client.FAILED, "versions")
        self.assertEqual(self.p.panel.banners[-1], "refused")
        self.assertEqual(self.p.panel.messages[-1], "versions")

    def test_read_only_session_keeps_the_core_and_asks_no_models(self):
        StubProject.name = "/a/p.qgz"
        self.mod.locator.locate = lambda s: ("/x/cassini", "PATH")
        self.fake_client()
        self.p._refresh()
        kept = self.p.client
        asked = []
        self.p.list_models = lambda: asked.append(True)
        past = [{"step": 1, "command": "a"}]
        self.p._on_event(self.mod.client.READY,
                         {"commands": [{"id": "a"}], "history": past, "read_only": True})
        self.assertIs(self.p.client, kept)  # noyau gardé : on peut rejouer
        self.assertEqual(self.p.panel.ready, [{"id": "a"}])
        self.assertEqual(self.p.panel.history, past)
        self.assertTrue(self.p.panel.read_only)
        self.assertNotIn("refused", self.p.panel.banners)
        self.assertEqual(asked, [])  # le modèle est grisé : pas de liste demandée
        self.p._on_event(self.mod.client.REPLAY_DONE, {"steps": 1, "identical": True})
        self.assertEqual(self.p.panel.events[-1][0], self.mod.client.REPLAY_DONE)

    def test_death_keeps_its_own_banner_text(self):
        self.p._on_event(self.mod.client.DIED, "mort")
        self.assertEqual(self.p.panel.banners[-1], "died")
        self.assertIsNone(self.p.panel.messages[-1])

    def test_ready_and_run_events_reach_the_panel(self):
        past = [{"step": 1, "command": "a"}]
        self.p._on_event(self.mod.client.READY, {"commands": [{"id": "a"}], "history": past})
        self.assertEqual(self.p.panel.ready, [{"id": "a"}])
        self.assertEqual(self.p.panel.history, past)
        self.p._on_event(self.mod.client.RUN_DONE, {"command": "a", "result": 1})
        self.assertEqual(self.p.panel.events[0][0], self.mod.client.RUN_DONE)

    def test_every_plan_event_reaches_the_panel(self):
        client = self.mod.client
        for kind in client.PLAN_EVENTS:
            self.p._on_event(kind, {"k": kind})
        self.assertEqual(self.p.panel.events, [(k, {"k": k}) for k in client.PLAN_EVENTS])

    def test_plan_actions_map_to_the_client(self):
        for action in ("propose", "approve", "reject"):
            self.p.client = None
            self.assertFalse(getattr(self.p, action)("x"), action)
        calls = []

        def answer(name, value):
            return lambda s, arg: calls.append((name, arg)) or value

        self.p.client = type("C", (), {
            "propose": lambda s, text, model=None: calls.append(("propose", text, model)) or True,
            "execute_plan": answer("execute_plan", False),
            "reject_plan": answer("reject_plan", True)})()
        self.assertIs(self.p.propose("Tampon"), True)
        self.assertIs(self.p.propose("Tampon", "b.gguf"), True)
        self.assertIs(self.p.approve("plan-1"), False)
        self.assertIs(self.p.reject("plan-1"), True)
        self.assertEqual(
            calls,
            [("propose", "Tampon", None), ("propose", "Tampon", "b.gguf"),
             ("execute_plan", "plan-1"), ("reject_plan", "plan-1")],
        )

    def test_models_are_asked_at_ready_and_their_events_reach_the_panel(self):
        client = self.mod.client
        asked = []
        self.p.client = type("C", (), {"list_models": lambda s: asked.append(1) or True})()
        self.p._on_event(client.READY, {"commands": []})
        self.assertEqual(asked, [1])
        self.assertIs(self.p.list_models(), True)
        for kind in client.MODEL_EVENTS:
            self.p._on_event(kind, {"k": kind})
        self.assertEqual(self.p.panel.events, [(k, {"k": k}) for k in client.MODEL_EVENTS])
        self.p.client = None
        self.assertFalse(self.p.list_models())

    def test_stop_sends_cancel_run(self):
        calls = []
        self.p.client = type("C", (), {"cancel_run": lambda s: calls.append(1)})()
        self.p.stop()
        self.assertEqual(calls, [1])

    def test_replay_action_and_its_verdicts_reach_the_panel(self):
        """« Rejouer le journal » [12.3] : le greffon passe au client, le verdict au panneau."""
        client = self.mod.client
        self.p.client = None
        self.assertFalse(self.p.replay())
        calls = []
        self.p.client = type("C", (), {"replay": lambda s: calls.append(1) or True})()
        self.assertIs(self.p.replay(), True)
        self.assertEqual(calls, [1])
        for kind in (client.REPLAY_DONE, client.REPLAY_FAILED):
            self.p._on_event(kind, {"k": kind})
        self.assertEqual(
            self.p.panel.events,
            [(client.REPLAY_DONE, {"k": client.REPLAY_DONE}),
             (client.REPLAY_FAILED, {"k": client.REPLAY_FAILED})],
        )




class ReplayTest(unittest.TestCase):
    """`replay()` envoie `journal.replay` et rend REPLAY_DONE / REPLAY_FAILED."""

    CATALOGUE = RunTest.CATALOGUE
    setUp = RunTest.setUp
    ready = RunTest.ready
    new = RunTest.new
    kinds = RunTest.kinds

    def test_replay_before_ready_sends_nothing(self):
        self.assertFalse(self.c.replay())
        self.assertEqual(len(self.p.written), 1)

    def test_replay_done(self):
        self.ready()
        self.assertTrue(self.c.replay())
        sent = self.new()[0]
        self.assertEqual(sent["method"], "journal.replay")
        self.assertEqual(sent["params"], {})
        self.assertTrue(self.c.replay_busy())
        self.assertFalse(self.c.replay())  # un seul rejeu à la fois
        self.assertFalse(self.c.run_command("system.about"))
        self.p.feed(reply(sent["id"], {"steps": 3, "identical": True}))
        self.assertEqual(self.events[-1], (client.REPLAY_DONE, {"steps": 3, "identical": True}))
        self.assertFalse(self.c.replay_busy())
        self.assertTrue(self.c.run_command("system.about"))

    def test_replay_refused_while_a_run_is_in_flight(self):
        self.ready()
        self.c.run_command("test.qgis_probe")
        self.assertFalse(self.c.replay())
        self.assertEqual(len(self.new()), 1)

    def test_replay_failure_names_the_step(self):
        self.ready()
        self.c.replay()
        err = {"code": -32000, "message": "m",
               "data": {"code": "replay.diverged", "message": "Le rejeu diverge.", "step": 2}}
        self.p.feed(reply(self.new()[0]["id"], error=err))
        self.assertEqual(
            self.events[-1],
            (client.REPLAY_FAILED,
             {"code": "replay.diverged", "message": "Le rejeu diverge.", "step": 2}),
        )
        self.assertFalse(self.c.replay_busy())
        self.assertFalse(self.p.closed)

    def test_orders_during_a_replay_are_executed_by_the_gestures(self):
        class Echo:
            def start(self, params, done):
                done(result={"echo": params})

            def cancel(self):
                pass

        registry = gestures.GestureRegistry()
        registry.register("layer.add", Echo)
        self.p = FakeProcess()
        c = client.CoreClient(self.p, lambda k, v: self.events.append((k, v)), registry)
        c.start("cassini", "/p.qgz")
        self.p.feed(reply(1, {"protocol_version": 2, "core_version": "0.1", "commands": []}))
        base = len(self.p.written)
        self.assertTrue(c.replay())
        self.p.feed((json.dumps({"jsonrpc": "2.0", "id": "core-1", "method": "order.execute",
                                 "params": {"command": "layer.add", "params": {"path": "a"}}}) + "\n").encode())
        sent = [json.loads(w) for w in self.p.written[base:]]
        self.assertEqual(sent[1], {"jsonrpc": "2.0", "id": "core-1", "result": {"echo": {"path": "a"}}})

    def test_core_death_during_a_replay_is_reported(self):
        self.ready()
        self.c.replay()
        self.p.finished.emit(1)
        self.assertIn(client.REPLAY_FAILED, self.kinds())
        self.assertEqual(self.events[-1][0], client.DIED)


class FocusButton:
    """Bouton dont le focus est explicite : `focused["w"]` est le widget qui l'a."""

    def __init__(self, focused):
        self.focused = focused

    def hasFocus(self):
        return self.focused["w"] is self

    def isEnabled(self):
        return True

    def setFocus(self, *_):
        self.focused["w"] = self


class PanelReplayTest(unittest.TestCase):
    """Le vrai panneau, Qt remplacé : rejeu lancé, verdict gardé et annoncé [12.3]."""

    def setUp(self):
        import types

        def stubbed(k):
            return k in ("qgis", "cassini_panel_stub") or k.startswith(
                ("qgis.", "cassini_panel_stub."))

        saved = {k: v for k, v in sys.modules.items() if stubbed(k)}

        def restore():
            for k in [k for k in sys.modules if stubbed(k)]:
                del sys.modules[k]
            sys.modules.update(saved)

        self.addCleanup(restore)
        self.mod = load_panel()
        mod = self.mod
        cls = mod.CassiniPanel
        panel = self.panel = cls.__new__(cls)
        panel.lang = "fr"
        panel.focus = ("entry", object())  # un détail cliqué, que le rejeu ne touche pas
        panel.ready = True
        panel.banner = None
        panel.running = False
        panel.replay = None
        panel._replay_focus = None
        panel.catalogue = []
        mod.Qt = types.SimpleNamespace(FocusReason=types.SimpleNamespace(OtherFocusReason=0))
        panel.agent = mod.agentview.AgentView()
        panel.log = mod.runlog.RunLog()
        panel.log.start("system.about", "Vérification du noyau Cassini", "10:00")
        panel.log.finish(mod.runlog.DONE)
        panel.external = mod.external.ExternalState()
        self.focused = {"w": None}
        panel.replay_button = FocusButton(self.focused)
        panel.replay_status = Recorder()
        panel._focus_widget = lambda: self.focused["w"]
        self.renders = []

        def render():
            self.renders.append(panel.replay)
            # Qt : un bouton qui se grise passe le focus au widget suivant.
            if mod.thread.replay_running(panel.replay) and panel.replay_button.hasFocus():
                self.focused["w"] = "repli expert"

        panel.render = render
        self.announced = []
        real = mod._announce
        self.addCleanup(setattr, mod, "_announce", real)
        mod._announce = lambda widget, text: self.announced.append(text)
        self.asked = []
        panel._actions = types.SimpleNamespace(replay=lambda: self.asked.append(1) or True)

    def test_click_starts_the_replay_and_greys_the_input(self):
        self.panel._replay()
        self.assertEqual(self.asked, [1])
        self.assertTrue(self.mod.thread.replay_running(self.panel.replay))
        self.assertEqual(self.announced, ["Rejeu en cours…"])
        self.assertTrue(self.panel._busy())  # saisie, Envoyer et Lancer grisés
        b = self.panel._replay_controls()
        self.assertEqual(b, {"enabled": False, "reason": "replay_reason_running"})
        self.panel._replay()  # second clic : rien ne repart
        self.assertEqual(self.asked, [1])

    def test_identical_verdict_is_kept_and_announced(self):
        self.panel._replay()
        self.panel.on_event(client.REPLAY_DONE, {"steps": 2, "identical": True})
        self.assertEqual(self.announced[-1], "Rejeu terminé : résultat identique.")
        self.assertFalse(self.panel._busy())
        self.assertIsNotNone(self.panel.focus)  # le détail cliqué reste

    def test_diverged_verdict_names_the_step(self):
        self.panel._replay()
        self.panel.on_event(client.REPLAY_FAILED, {
            "code": "replay.diverged", "step": 2,
            "message": "Le rejeu diverge du journal à l'étape 2 : le fichier a changé."})
        self.assertEqual(  # le message nomme déjà l'étape : pas répétée
            self.announced[-1], "Rejeu terminé : résultat différent. "
            "Le rejeu diverge du journal à l'étape 2 : le fichier a changé.")

    def test_empty_log_greys_the_button_with_its_reason(self):
        self.panel.log = self.mod.runlog.RunLog()
        self.assertEqual(self.panel._replay_controls(),
                         {"enabled": False, "reason": "replay_reason_empty"})
        self.panel._replay()
        self.assertEqual(self.asked, [])

    def test_focus_comes_back_only_if_it_did_not_move(self):
        self.focused["w"] = self.panel.replay_button
        self.panel._replay()
        self.assertEqual(self.focused["w"], "repli expert")
        self.panel.on_event(client.REPLAY_DONE, {"steps": 1, "identical": True})
        self.assertIs(self.focused["w"], self.panel.replay_button)
        # L'utilisateur a déplacé le focus pendant le rejeu : le verdict le laisse là.
        self.focused["w"] = self.panel.replay_button
        self.panel._replay()
        self.focused["w"] = "saisie"
        self.panel.on_event(client.REPLAY_DONE, {"steps": 1, "identical": True})
        self.assertEqual(self.focused["w"], "saisie")
        # Focus nulle part : il revient au bouton.
        self.panel._replay()
        self.focused["w"] = None
        self.panel.on_event(client.REPLAY_DONE, {"steps": 1, "identical": True})
        self.assertIsNone(self.focused["w"])  # le bouton n'avait pas le focus au départ
        self.focused["w"] = self.panel.replay_button
        self.panel._replay()
        self.focused["w"] = None
        self.panel.on_event(client.REPLAY_DONE, {"steps": 1, "identical": True})
        self.assertIs(self.focused["w"], self.panel.replay_button)

    def test_refused_click_says_so(self):
        self.panel._actions.replay = lambda: False
        self.panel._replay()
        self.assertEqual(self.panel.replay["status"], self.mod.thread.REPLAY_REFUSED)
        self.assertEqual(self.announced, ["Le noyau est occupé : réessayez dans un instant."])
        self.assertFalse(self.panel._busy())
        self.assertTrue(self.panel._replay_controls()["enabled"])  # on peut réessayer

    def test_rephrase_writes_nothing_during_a_replay(self):
        self.panel._replay()
        self.panel.input = Recorder()
        self.panel._rephrase()
        self.assertEqual(self.panel.input.calls, [])

    def test_lancer_send_and_input_are_greyed_during_a_replay(self):
        panel = self.panel
        panel._replay()
        for name in ("run_button", "stop_button", "reason", "form_zone", "input"):
            setattr(panel, name, Recorder())
        panel.selected = {"id": "system.about", "available": True, "cancellable": False}
        panel._render_buttons()
        self.assertIn(("setText", ("Patientez : le rejeu est en cours.",)), panel.reason.calls)
        self.assertIn(("setEnabled", (False,)), panel.run_button.calls)
        self.assertIn(("setVisible", (False,)), panel.stop_button.calls)  # pas d'Arrêter
        c = panel._controls()
        self.assertFalse(c["input_enabled"])
        self.assertFalse(c["send_enabled"])
        hint = self.mod.thread.input_hint(panel.agent, c["input_reason"], panel._busy(), None, "fr")
        self.assertEqual(hint, "Patientez : une étape est en cours.")

    def test_verdict_forgotten_when_the_log_grows_or_the_core_restarts(self):
        thread_ = self.mod.thread
        self.panel._replay()
        self.panel.on_event(client.REPLAY_DONE, {"steps": 1, "identical": True})
        n = len(self.panel.log.entries)
        self.assertIsNotNone(thread_.replay_current(self.panel.replay, n))
        self.assertIsNone(thread_.replay_current(self.panel.replay, n + 1))
        # Relancer : le noyau redit prêt, l'issue s'efface.
        self.panel._drop_stale_run = lambda: None
        self.panel.title = None
        self.panel.project_file = None
        self.panel._history_project = None
        self.panel.catalogue = []
        self.panel.set_ready([])
        self.assertIsNone(self.panel.replay)

    def test_core_gone_forgets_a_running_replay(self):
        self.panel._replay()
        self.panel._close_plan_entry = lambda *a: None
        self.panel.last = None
        self.panel._drop_stale_run()
        self.assertIsNone(self.panel.replay)


if __name__ == "__main__":
    unittest.main()
