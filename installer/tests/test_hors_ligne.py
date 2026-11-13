import hashlib
import http.client
import json
import re
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import archive_extension as ae  # noqa: E402
import telecharger_modele as tm  # noqa: E402
import verifier_taille as vt  # noqa: E402


class Base(unittest.TestCase):
    def setUp(self):
        self._t = tempfile.TemporaryDirectory()
        self.d = Path(self._t.name)
        self.addCleanup(self._t.cleanup)


class TestArchive(Base):
    def source(self):
        s = self.d / "cassini_qgis"
        (s / "__pycache__").mkdir(parents=True)
        (s / "__pycache__" / "x.pyc").write_bytes(b"\x00\x01")
        (s / "a.py").write_text("x = 1\n", encoding="utf-8")
        (s / "metadata.txt").write_text("[general]\n", encoding="utf-8")
        return s

    def test_archive_propre(self):
        out = self.d / "o" / "e.zip"
        ae.construire(self.source(), out)
        with zipfile.ZipFile(out) as z:
            self.assertEqual(sorted(z.namelist()), ["cassini_qgis/a.py", "cassini_qgis/metadata.txt"])

    def test_refuse_binaire_par_extension(self):
        s = self.source()
        (s / "noyau.exe").write_bytes(b"texte")
        with self.assertRaises(ae.ArchiveErreur):
            ae.construire(s, self.d / "e.zip")
        self.assertFalse((self.d / "e.zip").exists())

    def test_refuse_binaire_par_signature(self):
        s = self.source()
        (s / "mystere.dat").write_bytes(b"MZ\x90\x00")
        with self.assertRaises(ae.ArchiveErreur):
            ae.construire(s, self.d / "e.zip")

    def test_refuse_trop_lourd(self):
        s = self.source()
        (s / "gros.txt").write_bytes(b"a" * 10000)
        with self.assertRaises(ae.ArchiveErreur):
            ae.construire(s, self.d / "e.zip", limite=100)
        self.assertFalse((self.d / "e.zip").exists())

    def test_vraie_extension_passe(self):
        out = self.d / "vraie.zip"
        self.assertLess(ae.construire(ae.SOURCE, out), ae.LIMITE)

    def test_vraie_archive_pour_le_depot_d_extensions(self):
        out = self.d / "vraie.zip"
        ae.construire(ae.SOURCE, out)
        with zipfile.ZipFile(out) as z:
            noms = set(z.namelist())
            self.assertIn("cassini_qgis/LICENSE", noms)
            self.assertIn("cassini_qgis/assets/logo-cassini-light.svg", noms)
            self.assertIn("cassini_qgis/metadata.txt", noms)

    def test_publication_vraie_extension(self):
        ae.verifier_publication(ae.SOURCE)

    def test_publication_refuse_licence_absente_ou_cle_vide(self):
        s = self.source()
        with self.assertRaises(ae.ArchiveErreur):
            ae.verifier_publication(s)
        (s / "i.svg").write_text("<svg/>", encoding="utf-8")
        cles = "".join(f"{k}=x\n" for k in ae.CLES_PUBLICATION if k != "icon")
        (s / "metadata.txt").write_text("[general]\n" + cles + "icon=i.svg\n", encoding="utf-8")
        with self.assertRaisesRegex(ae.ArchiveErreur, "LICENSE"):
            ae.verifier_publication(s)
        (s / "LICENSE").write_text("GPL", encoding="utf-8")
        ae.verifier_publication(s)
        (s / "metadata.txt").write_text("[general]\nemail=\n", encoding="utf-8")
        with self.assertRaisesRegex(ae.ArchiveErreur, "email"):
            ae.verifier_publication(s)

    def test_extension_vide(self):
        s = self.d / "vide"
        s.mkdir()
        with self.assertRaises(ae.ArchiveErreur):
            ae.construire(s, self.d / "e.zip")
        self.assertFalse((self.d / "e.zip").exists())

    def test_main_code_1(self):
        self.assertEqual(ae.main(["--source", str(self.d / "absent"), "--sortie", str(self.d / "e.zip")]), 1)


class TestModele(Base):
    def decl(self, contenu: bytes, sha=None, taille=None):
        src = self.d / "src.gguf"
        src.write_bytes(contenu)
        return {
            "url": src.as_uri(),
            "sha256": sha or hashlib.sha256(contenu).hexdigest(),
            "taille": len(contenu) if taille is None else taille,
            "nom_installe": "m.gguf",
        }

    def test_modele_valide(self):
        out = self.d / "m" / "m.gguf"
        tm.telecharger(self.decl(b"gguf"), out, marge=0)
        self.assertEqual(out.read_bytes(), b"gguf")

    def test_empreinte_fausse(self):
        out = self.d / "m.gguf"
        with self.assertRaises(tm.ModeleErreur):
            tm.telecharger(self.decl(b"gguf", sha="0" * 64), out, marge=0)
        self.assertFalse(out.exists())

    def test_taille_fausse(self):
        with self.assertRaises(tm.ModeleErreur):
            tm.telecharger(self.decl(b"gguf", taille=99), self.d / "m.gguf", marge=0)
        self.assertFalse((self.d / "m.gguf").exists())

    def test_coupure_en_cours_de_telechargement(self):
        class Coupe:
            def __enter__(s):
                return s

            def __exit__(s, *a):
                return False

            def read(s, n):
                raise http.client.IncompleteRead(b"par")

        out = self.d / "m.gguf"
        orig = tm.urllib.request.urlopen
        tm.urllib.request.urlopen = lambda *a, **k: Coupe()
        try:
            with self.assertRaises(tm.ModeleErreur):
                tm.telecharger(self.decl(b"gguf"), out, marge=0)
        finally:
            tm.urllib.request.urlopen = orig
        self.assertFalse(out.exists())

    def test_declaration_sha_et_taille_invalides(self):
        for champ, valeur in (("sha256", "A" * 64), ("sha256", "z" * 64), ("sha256", "0" * 63),
                              ("taille", 0), ("taille", -1), ("taille", "12"), ("taille", True)):
            d = {"url": "x", "sha256": "0" * 64, "taille": 1, "nom_installe": "m", champ: valeur}
            f = self.d / "d.json"
            f.write_text(json.dumps(d), encoding="utf-8")
            with self.assertRaises(tm.ModeleErreur, msg=f"{champ}={valeur!r}"):
                tm.lire_declaration(f)

    def test_introuvable(self):
        d = self.decl(b"gguf")
        d["url"] = (self.d / "absent.gguf").as_uri()
        with self.assertRaises(tm.ModeleErreur):
            tm.telecharger(d, self.d / "m.gguf", marge=0)

    def test_espace_insuffisant(self):
        with self.assertRaises(tm.ModeleErreur):
            tm.verifier_espace(self.d, 1, marge=1 << 60)

    def test_declaration_du_depot(self):
        d = tm.lire_declaration()
        self.assertEqual(d["sha256"], "b687313f4a49c23e13673d14d185f3ee56d3e710e8d01c08595339236b21a865")
        self.assertEqual(d["taille"], 2783446944)
        self.assertTrue(d["url"].startswith("https://huggingface.co/KevziFar/cassini-composition-4b-GGUF/"))

    def test_declaration_incomplete(self):
        f = self.d / "d.json"
        f.write_text(json.dumps({"url": "x"}), encoding="utf-8")
        with self.assertRaises(tm.ModeleErreur):
            tm.lire_declaration(f)


class TestTaille(Base):
    def test_sous_le_seuil(self):
        f = self.d / "i.exe"
        f.write_bytes(b"abc")
        self.assertEqual(vt.verifier(f), 3)

    def test_au_dessus_du_seuil(self):
        f = self.d / "i.exe"
        f.write_bytes(b"abc")
        with self.assertRaises(ValueError):
            vt.verifier(f, seuil=2)


class TestScriptSansReseau(unittest.TestCase):
    def test_iss_sans_code_reseau(self):
        chemin = Path(__file__).resolve().parent.parent / "cassini-hors-ligne.iss"
        texte = chemin.read_text(encoding="utf-8").lower().replace("kevzi.github.io", "")
        for mot in ("downloadtemporaryfile", "tdownloadwizardpage", "http://", "https://", "urlmon", "wininet", "winhttp", "internetopen"):
            self.assertNotIn(mot, texte, mot)

    def test_nom_modele_egal_a_la_declaration(self):
        iss = (Path(__file__).resolve().parent.parent / "cassini-hors-ligne.iss").read_text(encoding="utf-8")
        nom = re.search(r'^#define NomModele "([^"]+)"', iss, re.M).group(1)
        self.assertEqual(nom, tm.lire_declaration()["nom_installe"])


if __name__ == "__main__":
    unittest.main()
