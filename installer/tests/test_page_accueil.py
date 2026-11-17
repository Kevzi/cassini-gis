import hashlib
import json
import re
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import page_accueil  # noqa: E402

SHA_P2 = "a" * 64
SHA_LN = "b" * 64
SHA_LP = "c" * 64


def manifeste(linux=True, porte2=True):
    d = {"version": "0.1.1", "fichiers": []}
    if porte2:
        d["porte2"] = {"nom": "c.exe", "url": "https://h/c.exe", "sha256": SHA_P2, "taille": 3 << 30}
    if linux:
        d["linux"] = {
            "noyau": {"nom": "n.tar", "url": "https://g/n.tar", "sha256": SHA_LN},
            "porte2": {"nom": "p.tar", "url": "https://h/p.tar", "sha256": SHA_LP, "taille": 3 << 30},
        }
    return d


class TestPageAccueil(unittest.TestCase):
    def setUp(self):
        self._t = tempfile.TemporaryDirectory()
        self.addCleanup(self._t.cleanup)
        self.d = Path(self._t.name)
        self.exe = self.d / "en-ligne.exe"
        self.exe.write_bytes(b"installeur")
        self.sortie = self.d / "site"
        self.sortie.mkdir()

    def lancer(self, m):
        mf = self.d / "manifeste.json"
        mf.write_text(json.dumps(m), encoding="utf-8")
        return page_accueil.generer(mf, self.exe, self.sortie)

    def index(self):
        return (self.sortie / "index.html").read_text(encoding="utf-8")

    def test_page_de_release(self):
        self.lancer(manifeste())
        h = self.index()
        for sha in (SHA_P2, SHA_LN, SHA_LP, hashlib.sha256(b"installeur").hexdigest()):
            self.assertIn(sha, h)
        self.assertIn("Installer en trois étapes", h)
        self.assertEqual(h.count("SHA-256 : <code>"), 4)

    def test_empreinte_calculee_sur_le_fichier(self):
        self.lancer(manifeste())
        self.assertIn(hashlib.sha256(self.exe.read_bytes()).hexdigest(), self.index())

    def test_manifeste_sans_linux(self):
        self.lancer(manifeste(linux=False))
        h = self.index()
        self.assertNotIn("Linux :", h)
        self.assertIn(SHA_P2, h)

    def test_video_absente(self):
        msgs = self.lancer(manifeste())
        self.assertNotIn("<video", self.index())
        self.assertTrue(any("vidéo absente" in m for m in msgs))

    def test_video_presente(self):
        (self.sortie / "cassini-presentation.mp4").write_bytes(b"x" * 1000)
        self.lancer(manifeste())
        self.assertIn("<video controls", self.index())

    def test_video_trop_lourde(self):
        v = self.sortie / "cassini-presentation.mp4"
        with v.open("wb") as f:
            f.truncate(25 * 1024 * 1024)
        with self.assertRaises(page_accueil.PageErreur) as c:
            self.lancer(manifeste())
        self.assertIn("25.0 Mo", str(c.exception))

    def test_texte_faq_smartscreen_contact(self):
        self.lancer(manifeste())
        h = self.index()
        self.assertIn('href="avertissement-windows.html"', h)
        self.assertIn('href="faq.html"', h)
        self.assertIn("cassini-sig@proton.me", h)
        self.assertEqual(h.count("<p>Cassini est"), 1)
        self.assertEqual(h.count("<p>C'est une extension"), 1)
        faq = (self.sortie / "faq.html").read_text(encoding="utf-8")
        self.assertIn("<dt>", faq)
        self.assertIn("cassini-sig@proton.me", faq)

    def test_rien_hors_du_dossier(self):
        (self.sortie / "cassini-presentation.mp4").write_bytes(b"x")
        self.lancer(manifeste())
        for nom in ("index.html", "faq.html"):
            h = (self.sortie / nom).read_text(encoding="utf-8")
            self.assertNotIn("<script", h)
            self.assertNotIn("@import", h)
            for balise in re.findall(r"<(?:img|link|video|source|iframe)\b[^>]*>", h):
                self.assertNotRegex(balise, r"(?:src|href)=\"https?:")


if __name__ == "__main__":
    unittest.main()
