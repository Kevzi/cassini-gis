import hashlib
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import manifeste  # noqa: E402

CARGO = """[workspace]
members = ["core"]

[workspace.package]
version = "0.1.0"
edition = "2021"
"""


class TestManifeste(unittest.TestCase):
    def setUp(self):
        self._t = tempfile.TemporaryDirectory()
        self.d = Path(self._t.name)
        self.addCleanup(self._t.cleanup)
        self.cargo = self.d / "Cargo.toml"
        self.cargo.write_text(CARGO, encoding="utf-8")
        self.exe = self.d / "cassini.exe"
        self.exe.write_bytes(b"contenu du noyau")

    def test_version_cargo(self):
        self.assertEqual(manifeste.version_cargo(self.cargo), "0.1.0")

    def test_etiquette_identique(self):
        manifeste.verifier_etiquette("v0.1.0", "0.1.0")

    def test_etiquette_differente_refusee(self):
        with self.assertRaises(manifeste.ManifesteErreur):
            manifeste.verifier_etiquette("v0.2.0", "0.1.0")

    def test_manifeste_contenu_et_forme(self):
        texte = manifeste.construire("0.1.0", [self.exe], "https://exemple/dl/v0.1.0/")
        data = json.loads(texte)
        self.assertEqual(data["version"], "0.1.0")
        f = data["fichiers"][0]
        self.assertEqual(f["nom"], "cassini.exe")
        self.assertEqual(f["url"], "https://exemple/dl/v0.1.0/cassini.exe")
        self.assertEqual(f["sha256"], hashlib.sha256(b"contenu du noyau").hexdigest())
        # Forme fixe lue par l'installeur : un champ par ligne.
        self.assertIn(f'      "sha256": "{f["sha256"]}"\n', texte)
        self.assertIn('      "nom": "cassini.exe",\n', texte)

    def test_main_ecrit(self):
        sortie = self.d / "manifeste.json"
        code = manifeste.main(
            ["--etiquette", "v0.1.0", "--cargo", str(self.cargo), "--sortie", str(sortie), str(self.exe)]
        )
        self.assertEqual(code, 0)
        self.assertIn("releases/download/v0.1.0/cassini.exe", sortie.read_text(encoding="utf-8"))

    def test_main_refuse_etiquette_et_ne_publie_rien(self):
        sortie = self.d / "manifeste.json"
        code = manifeste.main(
            ["--etiquette", "v0.2.0", "--cargo", str(self.cargo), "--sortie", str(sortie), str(self.exe)]
        )
        self.assertEqual(code, 1)
        self.assertFalse(sortie.exists())


class TestVerifierEtiquetteMain(unittest.TestCase):
    """La garde du premier pas de la release : TAG contre la version de Cargo.toml."""

    def _main(self, tag):
        import os
        from unittest import mock

        import verifier_etiquette

        version = manifeste.version_cargo(manifeste.RACINE / "Cargo.toml")
        env = {} if tag is None else {"TAG": tag.format(version=version)}
        with mock.patch.dict(os.environ, env, clear=True):
            return verifier_etiquette.main()

    def test_etiquette_egale_a_la_version(self):
        self.assertEqual(self._main("v{version}"), 0)

    def test_etiquette_differente_refusee(self):
        self.assertEqual(self._main("v99.0.0"), 1)

    def test_etiquette_absente_refusee(self):
        self.assertEqual(self._main(None), 1)


if __name__ == "__main__":
    unittest.main()
