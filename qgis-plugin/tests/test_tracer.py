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
import locator  # noqa: E402


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
        self.assertEqual(hello["params"]["protocol_version"], 1)
        data = reply(1, {"protocol_version": 1, "core_version": "0.1", "commands": []})
        self.p.feed(data[:10])  # réponse coupée : le cadrage par ligne tient
        self.assertEqual(len(self.p.written), 1)
        self.p.feed(data[10:])
        run = self.sent()[1]
        self.assertEqual(run["method"], "command.run")
        self.assertEqual(run["params"], {"command": "system.about", "params": {}})
        self.p.feed(reply(2, {"core_version": "0.1.0", "protocol_version": 1}))
        self.assertEqual(
            self.events,
            [(client.READY, {"core_version": "0.1.0", "protocol_version": 1})],
        )

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
        self.p.feed(reply(1, {"protocol_version": 1, "core_version": "0.1", "commands": []}))
        self.p.feed(reply(2, {"core_version": "0.1.0", "protocol_version": 1}))
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
        self.visible = None

    def setVisible(self, v):
        self.visible = v


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
    mod("qgis.PyQt.QtCore", QProcess=StubProcess, QTimer=object, QSettings=type(
        "S", (), {"value": lambda self, *a: "", "setValue": lambda self, *a: None}), Qt=object)
    mod("qgis.PyQt.QtWidgets", QDockWidget=StubWidget, QFileDialog=StubWidget,
        QLabel=StubWidget, QPushButton=StubWidget, QVBoxLayout=StubWidget, QWidget=StubWidget)
    pkg = types.ModuleType("cassini_stub")
    pkg.__path__ = [PKG]
    sys.modules["cassini_stub"] = pkg
    for n in ("client", "gestures", "locator"):
        sys.modules["cassini_stub." + n] = sys.modules[n]
    return importlib.import_module("cassini_stub.plugin")


class PluginBehaviourTest(unittest.TestCase):
    def setUp(self):
        self.mod = load_plugin()
        self.p = self.mod.CassiniPlugin(None)
        self.shown = []
        self.p._show = lambda text, manual=False: self.shown.append((text, manual))
        self.p.label = object()
        StubProcess.created = 0

    def test_no_project(self):
        StubProject.name = ""
        self.p._refresh()
        self.assertEqual(self.shown, [(self.mod.NO_PROJECT, False)])
        self.assertEqual(StubProcess.created, 0)

    def test_core_not_found(self):
        StubProject.name = "/a/p.qgz"
        self.mod.locator.locate = lambda s: None
        self.p._refresh()
        self.assertEqual(self.shown, [(self.mod.NOT_FOUND, True)])
        self.assertEqual(StubProcess.created, 0)

    def test_died_allows_relaunch(self):
        StubProject.name = "/a/p.qgz"
        self.mod.locator.locate = lambda s: ("/x/cassini", "PATH")
        self.mod.client.CoreClient = type("C", (), {
            "__init__": lambda s, proc, cb, reg=None: None,
            "start": lambda s, prog, proj: None,
            "stop": lambda s: None})
        self.p._refresh()
        self.assertEqual(StubProcess.created, 1)
        self.p._on_event(self.mod.client.DIED, "mort")
        self.assertIsNone(self.p.project_path)
        self.p._refresh()
        self.assertEqual(StubProcess.created, 2)


if __name__ == "__main__":
    unittest.main()
