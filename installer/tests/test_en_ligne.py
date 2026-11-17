"""Gardes textuelles de l'installeur en ligne (porte 1 complète, 7.4), sans Inno Setup.

La compilation et l'essai contre un serveur local ont lieu dans la recette de release
(`.github/workflows/release.yml`), où Inno Setup est installé ; ici, on tient les invariants
qui se lisent dans le texte du script.
"""

import json
import re
import shutil
import sys
import unittest
from pathlib import Path

INSTALLER = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(INSTALLER))
import manifeste  # noqa: E402

EN_LIGNE = (INSTALLER / "cassini-en-ligne.iss").read_text(encoding="utf-8")
HORS_LIGNE = (INSTALLER / "cassini-hors-ligne.iss").read_text(encoding="utf-8")


def section(texte: str, nom: str) -> str:
    m = re.search(rf"^\[{nom}\](.*?)(?=^\[|\Z)", texte, re.S | re.M)
    return m.group(1) if m else ""


class TestDeclaration(unittest.TestCase):
    def test_meme_nom_de_declaration_que_la_porte_2(self):
        hors = re.search(r'^#define Declaration "([^"]+)"', HORS_LIGNE, re.M).group(1)
        self.assertIn(f"DeclarationModele = '{hors}'", EN_LIGNE)

    def test_meme_forme_de_declaration_que_la_porte_2(self):
        # Porte 2 : {"model": "<nom>", "prompt": "short", "model_sha256": "<sha>"}
        forme = re.compile(r'"model": ".*?", "prompt": "short", "model_sha256": ".*?"\}')
        self.assertTrue(forme.search(HORS_LIGNE.replace("{#NomModele}", "n").replace("{#EmpreinteModele}", "s")))
        self.assertIn("""'{"model": "' + NomModele + '", "prompt": "short", "model_sha256": "' + ShaModele + '"}'""", EN_LIGNE)

    def test_nom_et_empreinte_viennent_de_modele_json_via_le_manifeste(self):
        # Aucune valeur du modèle n'est écrite en dur dans l'installeur en ligne.
        ref = json.loads((INSTALLER / "modele.json").read_text(encoding="utf-8"))
        self.assertNotIn(ref["sha256"], EN_LIGNE)
        self.assertNotIn(ref["nom_installe"], EN_LIGNE)
        self.assertNotIn("huggingface", EN_LIGNE.lower().replace("kevzi.github.io", ""))
        m = manifeste.section_modele(INSTALLER / "modele.json")
        self.assertEqual(m["nom"], ref["nom_installe"])
        self.assertEqual(m["sha256"], ref["sha256"])


class TestOptionEtSuppression(unittest.TestCase):
    def test_parametre_silencieux_et_case_cochee_par_defaut(self):
        self.assertIn("{param:MODELE|1}", EN_LIGNE)
        self.assertIn("CreateInputOptionPage", EN_LIGNE)

    def test_models_jamais_supprime_a_la_desinstallation(self):
        lignes = section(EN_LIGNE, "UninstallDelete").splitlines()
        sup = " ".join(l for l in lignes if not l.lstrip().startswith(";"))
        self.assertNotIn("models", sup)
        self.assertNotIn("filesandordirs", sup)
        motif = r'Name: "\{app\}\\bin"'
        self.assertNotRegex(sup, motif)
        # Le motif doit attraper la ligne fautive et épargner la bonne.
        self.assertRegex(r'Type: filesandordirs; Name: "{app}\bin"', motif)
        self.assertNotRegex(r'Type: files; Name: "{app}\bin\*"', motif)
        fichiers = section(EN_LIGNE, "Files")
        ligne = [l for l in fichiers.splitlines() if r"{tmp}\modele" in l][0]
        self.assertIn("uninsneveruninstall", ligne)
        self.assertIn("Check: ModelePret", ligne)

    def test_la_boucle_des_fichiers_s_arrete_avant_modele_et_porte2(self):
        self.assertIn("Pos('\"modele\"', Json)", EN_LIGNE)
        self.assertIn("Pos('\"porte2\"', Json)", EN_LIGNE)
        self.assertIn("Reste := Fichiers;", EN_LIGNE)

    def test_modele_verifie_par_empreinte_avant_toute_pose(self):
        self.assertIn("DownloadTemporaryFile(Url, NomModele, ShaModele, nil)", EN_LIGNE)
        # La déclaration n'est écrite que si le modèle a été téléchargé.
        self.assertRegex(EN_LIGNE, r"if ModeleTelecharge then\s+begin[\s\S]*?SaveStringToFile")

    def test_declaration_existante_d_un_autre_modele_jamais_ecrasee(self):
        self.assertIn("NomDecl := DeclarationModele;", EN_LIGNE)
        self.assertIn("(Pos(NomModele, String(Existant)) = 0)", EN_LIGNE)
        self.assertIn("NomDecl := Copy(NomModele, 1, Length(NomModele) - 5) + '.modele.json';", EN_LIGNE)
        self.assertNotIn(r"ExpandConstant('{app}\bin\models\') + DeclarationModele", EN_LIGNE)


class TestManifesteCompatibleAvecLInstalleur(unittest.TestCase):
    def test_boucle_de_fichiers_n_avale_ni_modele_ni_porte2(self):
        import tempfile

        with tempfile.TemporaryDirectory() as t:
            f = Path(t) / "cassini.exe"
            f.write_bytes(b"x")
            modele = manifeste.section_modele(INSTALLER / "modele.json")
            porte2 = {"nom": "p.exe", "url": "u", "sha256": "0" * 64, "taille": 1}
            texte = manifeste.construire("0.1.1", [f], "https://e", modele, porte2)
        fichiers = texte[: texte.index('"modele"')]
        self.assertEqual(re.findall(r'"nom": "([^"]+)"', fichiers), ["cassini.exe"])


@unittest.skipUnless(
    shutil.which("ISCC") or Path(r"C:\Program Files (x86)\Inno Setup 6\ISCC.exe").exists(),
    "Inno Setup absent : compilation vérifiée par la recette de release",
)
class TestCompilation(unittest.TestCase):
    def test_compile(self):
        import subprocess
        import tempfile

        iscc = shutil.which("ISCC") or r"C:\Program Files (x86)\Inno Setup 6\ISCC.exe"
        with tempfile.TemporaryDirectory() as t:
            r = subprocess.run(
                [iscc, f"/O{t}", "/DVersion=0.0.0", str(INSTALLER / "cassini-en-ligne.iss")],
                capture_output=True,
                text=True,
            )
        self.assertEqual(r.returncode, 0, r.stdout[-2000:] + r.stderr[-2000:])


if __name__ == "__main__":
    unittest.main()
