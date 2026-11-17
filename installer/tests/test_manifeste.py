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

    def test_sections_modele_et_porte2(self):
        racine = Path(manifeste.__file__).resolve().parent
        modele = manifeste.section_modele(racine / "modele.json")
        ref = json.loads((racine / "modele.json").read_text(encoding="utf-8"))
        self.assertEqual(list(modele), ["nom", "url", "sha256", "taille"])
        self.assertEqual(modele["nom"], ref["nom_installe"])
        self.assertEqual(modele["sha256"], ref["sha256"])
        self.assertEqual(modele["taille"], ref["taille"])
        complet = self.d / "complet.exe"
        complet.write_bytes(b"xyz")
        porte2 = manifeste.section_porte2("0.1.1", complet, "https://hf/x/resolve/main/")
        self.assertEqual(list(porte2), ["nom", "url", "sha256", "taille"])
        self.assertEqual(porte2["nom"], "cassini-installeur-complet-0.1.1.exe")
        self.assertEqual(porte2["url"], "https://hf/x/resolve/main/cassini-installeur-complet-0.1.1.exe")
        self.assertEqual(porte2["taille"], 3)
        texte = manifeste.construire("0.1.1", [self.exe], "https://e/d", modele, porte2)
        data = json.loads(texte)
        self.assertEqual(list(data), ["version", "fichiers", "modele", "porte2"])
        # Un champ par ligne, dans l'ordre nom, url, sha256, taille ; `"modele"` après les fichiers.
        self.assertIn('  "modele": {\n    "nom": "' + modele["nom"] + '",\n    "url": ', texte)
        self.assertLess(texte.index('"fichiers"'), texte.index('"modele"'))
        self.assertLess(texte.index('"modele"'), texte.index('"porte2"'))
        self.assertEqual(texte.count('"modele"'), 1)

    def test_sections_absentes_quand_non_fournies(self):
        data = json.loads(manifeste.construire("0.1.0", [self.exe], "https://e/d"))
        self.assertNotIn("modele", data)
        self.assertNotIn("porte2", data)

    def test_section_linux_apres_porte2_et_sans_effet_sur_la_boucle_windows(self):
        racine = Path(manifeste.__file__).resolve().parent
        modele = manifeste.section_modele(racine / "modele.json")
        complet = self.d / "complet.exe"
        complet.write_bytes(b"xyz")
        porte2 = manifeste.section_porte2("0.1.1", complet, "https://hf/x")
        noyau_tar = self.d / "cassini-noyau-linux-0.1.1.tar.gz"
        noyau_tar.write_bytes(b"tar du noyau")
        complet_tar = self.d / "cassini-complet-linux-0.1.1.tar"
        complet_tar.write_bytes(b"tar complet")
        linux = manifeste.section_linux(noyau_tar, complet_tar, "https://gh/v0.1.1", "https://hf/x")
        self.assertEqual(list(linux), ["noyau", "porte2"])
        self.assertEqual(list(linux["noyau"]), ["nom", "url", "sha256"])
        self.assertEqual(list(linux["porte2"]), ["nom", "url", "sha256", "taille"])
        self.assertEqual(linux["noyau"]["url"], "https://gh/v0.1.1/cassini-noyau-linux-0.1.1.tar.gz")
        self.assertEqual(linux["porte2"]["sha256"], hashlib.sha256(b"tar complet").hexdigest())
        texte = manifeste.construire("0.1.1", [self.exe], "https://e/d", modele, porte2, linux)
        self.assertEqual(list(json.loads(texte)), ["version", "fichiers", "modele", "porte2", "linux"])
        # Les repères de l'installeur Windows (premier `"modele"`, premier `"porte2"`) restent
        # ceux des sections de premier niveau, et la boucle des fichiers s'arrête avant eux :
        # aucune entrée `linux` n'y est avalée.
        avant = texte[: texte.index('"modele"')]
        self.assertNotIn("linux", avant)
        self.assertEqual(avant.count('"sha256"'), 1)
        self.assertLess(texte.index('"porte2"'), texte.index('"linux"'))
        self.assertEqual(texte.count('"modele"'), 1)
        self.assertIn(
            '  "linux": {\n    "noyau": {\n      "nom": "cassini-noyau-linux-0.1.1.tar.gz",\n',
            texte,
        )

    def test_section_linux_absente_quand_non_fournie(self):
        sortie = self.d / "manifeste.json"
        code = manifeste.main(["--etiquette", "v0.1.0", "--cargo", str(self.cargo), "--modele",
                               "--sortie", str(sortie), str(self.exe)])
        self.assertEqual(code, 0)
        self.assertNotIn("linux", json.loads(sortie.read_text(encoding="utf-8")))

    def test_main_avec_linux(self):
        sortie = self.d / "manifeste.json"
        a = self.d / "n.tar.gz"
        b = self.d / "c.tar"
        a.write_bytes(b"1")
        b.write_bytes(b"22")
        code = manifeste.main(["--etiquette", "v0.1.0", "--cargo", str(self.cargo), "--sortie", str(sortie),
                               "--modele", "--linux", str(a), str(b), str(self.exe)])
        self.assertEqual(code, 0)
        data = json.loads(sortie.read_text(encoding="utf-8"))
        self.assertTrue(data["linux"]["noyau"]["url"].endswith("/releases/download/v0.1.0/n.tar.gz"))
        self.assertTrue(data["linux"]["porte2"]["url"].startswith("https://huggingface.co/"))
        self.assertEqual(data["linux"]["porte2"]["taille"], 2)

    def test_main_linux_sans_modele_refuse(self):
        sortie = self.d / "manifeste.json"
        a = self.d / "n.tar.gz"
        a.write_bytes(b"1")
        code = manifeste.main(["--etiquette", "v0.1.0", "--cargo", str(self.cargo), "--sortie", str(sortie),
                               "--linux", str(a), str(a), str(self.exe)])
        self.assertEqual(code, 1)
        self.assertFalse(sortie.exists())

    def test_main_avec_modele_et_porte2(self):
        sortie = self.d / "manifeste.json"
        complet = self.d / "complet.exe"
        complet.write_bytes(b"abc")
        code = manifeste.main(["--etiquette", "v0.1.0", "--cargo", str(self.cargo), "--sortie", str(sortie),
                               "--modele", "--porte2", str(complet), str(self.exe)])
        self.assertEqual(code, 0)
        data = json.loads(sortie.read_text(encoding="utf-8"))
        self.assertEqual(data["porte2"]["sha256"], hashlib.sha256(b"abc").hexdigest())
        self.assertTrue(data["porte2"]["url"].startswith("https://huggingface.co/"))

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
