"""Tests sans QGIS : Qt est remplacé par un double de QProcess."""

import json
import os
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
PKG = os.path.join(HERE, "..", "cassini_qgis")
sys.path.insert(0, PKG)

import client  # noqa: E402
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
        self.closed = False

    def start(self, program, args):
        self.started = (program, args)

    def write(self, data):
        self.written.append(data)

    def readAllStandardOutput(self):
        data, self.out = self.out, b""
        return data

    def readAllStandardError(self):
        return b""

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

    def test_garbage_and_orders_ignored(self):
        self.c.start("cassini", "/p.qgz")
        self.p.feed(b"pas du json\n")
        self.p.feed(b'{"jsonrpc":"2.0","id":"core-1","method":"order.execute"}\n')
        self.assertEqual(self.events, [])


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
    mod("qgis.PyQt.QtCore", QProcess=StubProcess, QSettings=type(
        "S", (), {"value": lambda self, *a: "", "setValue": lambda self, *a: None}), Qt=object)
    mod("qgis.PyQt.QtWidgets", QDockWidget=StubWidget, QFileDialog=StubWidget,
        QLabel=StubWidget, QPushButton=StubWidget, QVBoxLayout=StubWidget, QWidget=StubWidget)
    pkg = types.ModuleType("cassini_stub")
    pkg.__path__ = [PKG]
    sys.modules["cassini_stub"] = pkg
    for n in ("client", "locator"):
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
            "__init__": lambda s, proc, cb: None,
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
