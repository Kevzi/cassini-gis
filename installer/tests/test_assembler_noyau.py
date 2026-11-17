import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import assembler_noyau as an  # noqa: E402

COMPLET = ["cassini.exe", "llama.dll", "ggml.dll", "ggml-base.dll", "ggml-vulkan.dll",
           "ggml-cpu-x64.dll", "ggml-cpu-haswell.dll"]


class TestAssembler(unittest.TestCase):
    def setUp(self):
        self._t = tempfile.TemporaryDirectory()
        self.d = Path(self._t.name)
        self.addCleanup(self._t.cleanup)
        self.src = self.d / "release"
        self.src.mkdir()
        self.out = self.d / "dist" / "noyau"
        self.txt = self.d / "licences"
        self.txt.mkdir()
        for n in an.TEXTES:
            (self.txt / n).write_bytes(n.encode())

    def poser(self, noms):
        for n in noms:
            (self.src / n).write_bytes(n.encode())

    def test_paquet_complet(self):
        self.poser(COMPLET + ["cassini.pdb", "cassini.d"])
        self.assertEqual(sorted(an.assembler(self.src, self.out, self.txt)), sorted(COMPLET + list(an.TEXTES)))
        self.assertEqual(sorted(f.name for f in self.out.iterdir()), sorted(COMPLET + list(an.TEXTES)))
        self.assertEqual(an.main(["--source", str(self.src), "--textes", str(self.txt), "--sortie", str(self.out)]), 0)

    def test_vulkan_manquante(self):
        self.poser([n for n in COMPLET if n != "ggml-vulkan.dll"])
        with self.assertRaisesRegex(an.AssemblageErreur, "ggml-vulkan.dll"):
            an.assembler(self.src, self.out, self.txt)
        self.assertFalse(self.out.exists())

    def test_aucun_moteur_processeur(self):
        self.poser(an.FIXES)
        with self.assertRaises(an.AssemblageErreur):
            an.assembler(self.src, self.out, self.txt)

    def test_llama_common_refusee(self):
        self.poser(COMPLET + ["llama-common.dll"])
        with self.assertRaisesRegex(an.AssemblageErreur, "llama-common.dll"):
            an.assembler(self.src, self.out, self.txt)
        self.assertFalse(self.out.exists())
        self.assertEqual(an.main(["--source", str(self.src), "--textes", str(self.txt), "--sortie", str(self.out)]), 1)

    def test_autre_executable_refuse(self):
        self.poser(COMPLET + ["autre.exe"])
        with self.assertRaises(an.AssemblageErreur):
            an.assembler(self.src, self.out, self.txt)

    def test_textes_de_licences_manquants(self):
        self.poser(COMPLET)
        (self.txt / "FRONTIERE-GPL.txt").unlink()
        with self.assertRaisesRegex(an.AssemblageErreur, "FRONTIERE-GPL.txt"):
            an.assembler(self.src, self.out, self.txt)
        self.assertFalse(self.out.exists())
        self.assertEqual(an.main(["--source", str(self.src), "--textes", str(self.txt), "--sortie", str(self.out)]), 1)

    def test_dossier_source_absent(self):
        self.assertEqual(an.main(["--source", str(self.d / "absent"), "--textes", str(self.txt), "--sortie", str(self.out)]), 1)

    def test_verifier_dossier_refuse_fichier_en_trop(self):
        self.poser(COMPLET)
        an.assembler(self.src, self.out, self.txt)
        (self.out / "llama-common.dll").write_bytes(b"x")
        with self.assertRaises(an.AssemblageErreur):
            an.verifier_dossier(self.out)


if __name__ == "__main__":
    unittest.main()
