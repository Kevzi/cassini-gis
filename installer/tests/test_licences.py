import shutil
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import licences as li  # noqa: E402


def paquet(nom, licence, version="1.0.0", source="registry+x"):
    return {"id": f"{nom} {version}", "name": nom, "version": version, "license": licence,
            "source": None if source is None else source}


def meta(paquets, aretes):
    """aretes : {nom: [(dependance, kind)]} ; kind None = normale, 'dev' ou 'build'."""
    ids = {p["name"]: p["id"] for p in paquets}
    noeuds = [{"id": p["id"], "deps": [
        {"pkg": ids[d], "dep_kinds": [{"kind": k}]} for d, k in aretes.get(p["name"], [])]}
        for p in paquets]
    return {"packages": paquets, "resolve": {"nodes": noeuds}}


def base():
    return [paquet("cassini", "Apache-2.0", source=None), paquet("zeta", "MIT"), paquet("alpha", "MIT OR Apache-2.0")]


class TestLicences(unittest.TestCase):
    def test_liste_triee_avec_nom_version_spdx(self):
        m = meta(base(), {"cassini": [("zeta", None), ("alpha", None)]})
        self.assertEqual(li.lignes_crates(m), [
            ("alpha", "1.0.0", "MIT OR Apache-2.0"), ("cassini", "1.0.0", "Apache-2.0"), ("zeta", "1.0.0", "MIT")])
        texte = li.rendre_tierces(li.lignes_crates(m))
        for attendu in ("alpha 1.0.0 MIT OR Apache-2.0", "llama.cpp", "MIT", "Qwen3.5-4B", "Apache-2.0", "Inno Setup"):
            self.assertIn(attendu, texte)

    def test_crate_ajoutee_ou_retiree(self):
        avec = meta(base(), {"cassini": [("zeta", None), ("alpha", None)]})
        sans = meta(base(), {"cassini": [("zeta", None)]})
        self.assertIn("alpha", li.rendre_tierces(li.lignes_crates(avec)))
        self.assertNotIn("alpha", li.rendre_tierces(li.lignes_crates(sans)))

    def test_dev_et_build_non_livrees(self):
        m = meta(base(), {"cassini": [("zeta", "dev"), ("alpha", "build")]})
        self.assertEqual([n for n, _, _ in li.lignes_crates(m)], ["cassini"])

    def test_licence_absente_nomme_la_crate(self):
        p = base() + [paquet("sans-licence", None)]
        m = meta(p, {"cassini": [("sans-licence", None)]})
        with self.assertRaisesRegex(li.LicencesErreur, "sans-licence"):
            li.lignes_crates(m)

    def test_gpl_ou_lgpl_nomme_la_crate(self):
        for lic in ("GPL-3.0-only", "LGPL-2.1-or-later", "MIT AND GPL-2.0-only", "AGPL-3.0"):
            p = base() + [paquet("copyleft", lic)]
            m = meta(p, {"cassini": [("copyleft", None)]})
            with self.assertRaisesRegex(li.LicencesErreur, "copyleft"):
                li.lignes_crates(m)

    def test_alternative_permissive_acceptee(self):
        self.assertTrue(li.permissive_possible("MIT OR Apache-2.0 OR LGPL-2.1-or-later"))
        self.assertTrue(li.permissive_possible("(MIT OR GPL-2.0-only) AND Unicode-3.0"))
        self.assertFalse(li.permissive_possible("(GPL-2.0-only OR LGPL-3.0) AND MIT"))

    def test_deux_executions_memes_octets(self):
        m = meta(base(), {"cassini": [("zeta", None), ("alpha", None)]})
        with tempfile.TemporaryDirectory() as t:
            a, b = Path(t) / "a", Path(t) / "b"
            for d in (a, b):
                li.ecrire(d, li.rendre_tierces(li.lignes_crates(m)), li.rendre_frontiere())
            for nom in (li.NOM_TIERCES, li.NOM_FRONTIERE):
                self.assertEqual((a / nom).read_bytes(), (b / nom).read_bytes())
                self.assertNotIn(b"\r", (a / nom).read_bytes())

    @unittest.skipUnless(shutil.which("cargo"), "cargo absent")
    def test_depot_reel(self):
        try:
            li.lire_metadata()
        except li.LicencesErreur as e:  # sources Cargo pas encore téléchargées (recette avant construction)
            self.skipTest(f"cargo metadata hors ligne impossible : {e}")
        with tempfile.TemporaryDirectory() as t:
            self.assertEqual(li.main(["--sortie", t]), 0)
            texte = (Path(t) / li.NOM_TIERCES).read_text(encoding="utf-8")
            self.assertIn("cassini-core", texte)
            self.assertNotIn("cassini-app", texte)
            self.assertTrue((Path(t) / li.NOM_FRONTIERE).read_text(encoding="utf-8").startswith("# Frontière"))


if __name__ == "__main__":
    unittest.main()
