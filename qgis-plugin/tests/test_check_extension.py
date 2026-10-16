"""Tests de la garde statique de l'extension, sur des sources temporaires."""

import os
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import check_extension as ce  # noqa: E402

GOOD_META = "[general]\nqgisMinimumVersion=3.44\nqgisMaximumVersion=4.99\n"
ROOT = os.path.abspath(os.path.join(HERE, "..", ".."))


class GardeTest(unittest.TestCase):
    def pkg(self, source="import os\n", meta=GOOD_META):
        d = tempfile.TemporaryDirectory()
        self.addCleanup(d.cleanup)
        with open(os.path.join(d.name, "mod.py"), "w", encoding="utf-8") as f:
            f.write(source)
        if meta is not None:
            with open(os.path.join(d.name, "metadata.txt"), "w", encoding="utf-8") as f:
                f.write(meta)
        return d.name

    def test_propre(self):
        self.assertEqual(ce.run(self.pkg("from qgis.PyQt import QtCore\n")), [])

    def test_extension_reelle_propre(self):
        self.assertEqual(ce.run(ce.DEFAULT_PKG), [])

    def test_modules_refuses(self):
        for mod in ["urllib.request", "requests", "socket", "http", "PyQt5.QtCore", "PyQt6"]:
            with self.subTest(mod=mod):
                errs = ce.check_imports(self.pkg(f"import {mod}\n"))
                self.assertEqual(len(errs), 1)
                self.assertIn("mod.py:1", errs[0])
                self.assertIn(mod, errs[0])

    def test_modules_reseau_etendus(self):
        for mod in ["ftplib", "smtplib", "ssl", "asyncio", "xmlrpc.client", "telnetlib"]:
            with self.subTest(mod=mod):
                self.assertEqual(len(ce.check_imports(self.pkg(f"import {mod}\n"))), 1)

    def test_qtnetwork_refuse(self):
        for src in ["from qgis.PyQt import QtNetwork\n", "from qgis.PyQt.QtNetwork import QTcpSocket\n",
                    "import qgis.PyQt.QtWebSockets\n"]:
            with self.subTest(src=src):
                self.assertEqual(len(ce.check_imports(self.pkg(src))), 1)

    def test_import_dynamique_refuse(self):
        for src in ['__import__("socket")\n', 'import importlib\nimportlib.import_module("socket")\n']:
            with self.subTest(src=src):
                errs = ce.check_imports(self.pkg(src))
                self.assertEqual(len(errs), 1)
                self.assertIn("dynamique", errs[0])

    def test_from_import(self):
        errs = ce.check_imports(self.pkg("from http import client\n"))
        self.assertEqual(len(errs), 1)

    def test_pyqt_cite_qgis_pyqt(self):
        errs = ce.check_imports(self.pkg("import PyQt5.QtCore\n"))
        self.assertIn("qgis.PyQt", errs[0])

    def test_import_tardif(self):
        errs = ce.check_imports(self.pkg("def f():\n    import urllib.request\n"))
        self.assertEqual(len(errs), 1)
        self.assertIn("mod.py:2", errs[0])

    def test_import_relatif_ignore(self):
        self.assertEqual(ce.check_imports(self.pkg("from . import socket\n")), [])

    def test_taille(self):
        p = self.pkg()
        old = ce.MAX_BYTES
        ce.MAX_BYTES = 5
        self.addCleanup(setattr, ce, "MAX_BYTES", old)
        errs = ce.check_size(p)
        self.assertEqual(len(errs), 1)
        self.assertIn("Mo", errs[0])

    def test_versions(self):
        bad = "[general]\nqgisMinimumVersion=3.40\nqgisMaximumVersion=4.99\n"
        errs = ce.check_metadata(self.pkg(meta=bad))
        self.assertEqual(len(errs), 1)
        self.assertIn("qgisMinimumVersion", errs[0])
        errs = ce.check_metadata(self.pkg(meta="[general]\n"))
        self.assertEqual(len(errs), 2)

    def test_noyau_sans_qgis(self):
        for crate in ("core", "cli", "ai"):
            path = os.path.join(ROOT, crate, "Cargo.toml")
            if crate == "ai" and not os.path.isfile(path):
                continue  # le crate /ai naît à l'étape 5 ; dès qu'il existe, il est contrôlé
            with open(path, encoding="utf-8") as f:
                text = f.read().lower()
            for word in ("qgis", "pyqt"):
                self.assertNotIn(word, text, f"{crate}/Cargo.toml mentionne {word}")


if __name__ == "__main__":
    unittest.main()
