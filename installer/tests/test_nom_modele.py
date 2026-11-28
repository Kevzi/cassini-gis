"""Garde contre le nom versionné du modèle écrit en dur (rétro de l'epic publication, F3).

`installer/modele.json` (`nom_installe`) est la seule source. Les endroits qui répètent le nom
(workflow de release, installeur hors ligne, script de publication, docstrings, test) doivent le
suivre : un oubli après une republication ne casserait la release qu'à l'étiquette.
"""

import json
import re
import unittest
from pathlib import Path

RACINE = Path(__file__).resolve().parents[2]
NOM_VERSIONNE = re.compile(r"cassini-composition-4b-\d+\.\d+\.\d+-Q4_K_M\.gguf")

ENDROITS = [
    ".github/workflows/release.yml",
    "installer/cassini-hors-ligne.iss",
    "docs/modele-hugging-face/publier.ps1",
    "docs/modele-hugging-face/README.md",
    "installer/telecharger_modele.py",
    "installer/assembler_linux.py",
    "installer/tests/test_hors_ligne.py",
]


class TestNomDuModele(unittest.TestCase):
    def test_noms_en_dur_identiques_a_modele_json(self):
        attendu = json.loads((RACINE / "installer" / "modele.json").read_text(encoding="utf-8"))["nom_installe"]
        self.assertRegex(attendu, NOM_VERSIONNE)
        ecarts = []
        for chemin in ENDROITS:
            texte = (RACINE / chemin).read_text(encoding="utf-8-sig")
            for numero, ligne in enumerate(texte.splitlines(), 1):
                ecarts += [
                    f"{chemin}:{numero} : « {nom} », attendu « {attendu} » (installer/modele.json)"
                    for nom in NOM_VERSIONNE.findall(ligne)
                    if nom != attendu
                ]
        self.assertEqual(ecarts, [])


if __name__ == "__main__":
    unittest.main()
