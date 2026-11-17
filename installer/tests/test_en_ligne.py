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

EN_LIGNE = (INSTALLER / "cassini-en-ligne.iss").read_text(encoding="utf-8-sig")
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


def comparer_versions(a: str, b: str) -> int:
    """Miroir Python de `ComparerVersions` du script : -1, 0, 1, ou -2 si illisible.

    Trois nombres séparés par des points, parties manquantes à 0 (la logique Pascal
    est reproduite ici pour pouvoir la tester sans Inno Setup).
    """
    if not a or not b:
        return -2

    def parties(v: str) -> list[int] | None:
        morceaux = v.split(".")[:3]
        out = []
        for m in morceaux:
            if not m.isascii() or not m.isdigit():
                return None
            out.append(int(m))
        return out + [0] * (3 - len(out))

    pa, pb = parties(a), parties(b)
    if pa is None or pb is None:
        return -2
    return (pa > pb) - (pa < pb)


def a_jour(installee: str, manifeste: str, modele_present: bool) -> bool:
    """Règle de décision du script : à jour (rien à faire) ou mise à jour proposée."""
    cmp = comparer_versions(installee, manifeste)
    return cmp == 1 or (cmp == 0 and modele_present)


class TestMiseAJour(unittest.TestCase):
    def test_parametre_miseajour_et_raccourci(self):
        self.assertIn("'/MISEAJOUR'", EN_LIGNE)
        icones = section(EN_LIGNE, "Icons")
        self.assertIn("{autoprograms}", icones)
        self.assertIn("Mettre à jour Cassini", icones)
        self.assertIn('Parameters: "/MISEAJOUR"', icones)
        self.assertIn(r'{app}\mettre-a-jour-cassini.exe', icones)

    def test_copie_de_l_installeur_dans_app(self):
        fichiers = section(EN_LIGNE, "Files")
        ligne = [l for l in fichiers.splitlines() if "{srcexe}" in l][0]
        self.assertIn('DestDir: "{app}"', ligne)
        self.assertIn('DestName: "mettre-a-jour-cassini.exe"', ligne)

    def test_meme_appid_que_la_porte_2_et_une_seule_source(self):
        ident = re.search(r"^AppId=(.+)$", EN_LIGNE, re.M).group(1)
        self.assertIn(ident, HORS_LIGNE)

    def test_aucune_suppression_dans_models(self):
        # Aucune ligne de code qui supprime ou écrase quoi que ce soit dans models ou {app}.
        code = section(EN_LIGNE, "Code")
        for ligne in code.splitlines():
            nue = ligne.strip()
            if nue.startswith("{") or nue.startswith("//"):
                continue
            if re.search(r"DeleteFile|DelTree|DeleteFile|RemoveDir", nue):
                self.assertNotIn("models", nue)
                self.assertNotIn("{app}", nue)
        fichiers = section(EN_LIGNE, "Files")
        ligne = [l for l in fichiers.splitlines() if r"{tmp}\modele" in l][0]
        self.assertIn("onlyifdoesntexist", ligne)
        self.assertIn("uninsneveruninstall", ligne)

    def test_modele_existant_jamais_retelecharge(self):
        self.assertRegex(
            EN_LIGNE,
            r"if FileExists\(ExpandConstant\('\{app\}\\bin\\models\\'\) \+ NomModele\) then\s+Exit;",
        )

    def test_a_jour_sans_telechargement_ni_ecriture(self):
        # Le cas « à jour » sort (code 0) avant tout téléchargement des fichiers.
        i_sortie = EN_LIGNE.index("ExitProcess(0)")
        i_boucle = EN_LIGNE.index("Reste := Fichiers;")
        self.assertLess(i_sortie, i_boucle)
        self.assertIn("ComparerVersions(VersionPresente, VersionAPoser)", EN_LIGNE)

    def test_non_installe_et_noyau_verrouille(self):
        self.assertIn("Cassini n''est pas installé", EN_LIGNE)
        self.assertIn("Fermez QGIS", EN_LIGNE)
        self.assertIn("Cassini est à jour", EN_LIGNE)

    def test_silencieux_sans_dialogue(self):
        self.assertIn("WizardSilent", EN_LIGNE)

    def test_installation_json_garde_deux_cles(self):
        self.assertIn(
            """'{"version": "' + VersionAPoser + '", "installe_le": "' + DateIsoUtc + '"}'""",
            EN_LIGNE,
        )

    def test_nom_installe_versionne(self):
        ref = json.loads((INSTALLER / "modele.json").read_text(encoding="utf-8"))
        self.assertRegex(ref["nom_installe"], r"^cassini-composition-4b-\d+\.\d+\.\d+-Q4_K_M\.gguf$")
        self.assertIn(ref["nom_installe"], ref["url"])

    def test_table_de_comparaison(self):
        cas = [
            ("0.1.0", "0.1.1", -1),
            ("0.1.1", "0.1.1", 0),
            ("0.2.0", "0.1.1", 1),
            ("0.1.10", "0.1.9", 1),  # numérique, pas lexical
            ("1.0", "1.0.0", 0),  # partie manquante = 0
            ("1", "1.0.1", -1),
            ("", "0.1.1", -2),
            ("0.1.1", "", -2),
            ("abc", "0.1.1", -2),
            ("1..2", "1.0.2", -2),
        ]
        for a, b, attendu in cas:
            self.assertEqual(comparer_versions(a, b), attendu, (a, b))

    def test_table_de_decision(self):
        cas = [
            ("0.1.0", "0.1.1", True, False),  # noyau à mettre à jour
            ("0.1.1", "0.1.1", True, True),  # à jour
            ("0.2.0", "0.1.1", False, True),  # installée plus récente
            ("0.1.1", "0.1.1", False, False),  # même version, nouveau modèle
            ("", "0.1.1", True, False),  # version illisible : mise à jour proposée
        ]
        for inst, man, modele, attendu in cas:
            self.assertEqual(a_jour(inst, man, modele), attendu, (inst, man, modele))


class TestRobustesse(unittest.TestCase):
    def test_bom_utf8_et_decodage(self):
        brut = (INSTALLER / "cassini-en-ligne.iss").read_bytes()
        self.assertEqual(brut[:3], bytes([0xEF, 0xBB, 0xBF]))
        brut.decode("utf-8")

    def test_renommage_retour_echoue_refuse(self):
        i = EN_LIGNE.index("if not RenameFile(Essai, Bin) then")
        bloc = EN_LIGNE[i : i + 400]
        self.assertIn("Result := True;", bloc)
        self.assertIn("renommé en bin à la main", bloc)

    def test_message_en_cours_d_usage(self):
        self.assertIn(
            "Fermez QGIS et toute fenêtre ouverte sur le dossier de Cassini, puis relancez la mise à jour.",
            EN_LIGNE,
        )


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
