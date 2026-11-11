"""Tests de la barre de non-oubli (ticket 5.53, livrable 2) : une ligne de la matrice par test,
sur de petits relevés écrits ici, puis des extraits des vrais formats.

    python3 -I -m unittest discover -s training/entrainement/tests
"""

import contextlib
import io
import json
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))

import non_oubli  # noqa: E402


def essais(*sections, modele=None):
    """Un relevé d'essais GPU : `(barre, ligne SCORE)` ou `(barre, ligne SCORE, code de fin)`
    par section. Sans `modele`, pas de ligne d'identité (le même modèle des deux côtés ferait
    un avertissement et un « à décider »)."""
    lignes = []
    for barre, score, *code in sections:
        lignes.append(f"=== {barre} ===")
        if modele:
            lignes.append(f"Modèle : {modele} ; adaptateur : aucun ; ")
        lignes += [score, f"=== fin {barre} : {code[0] if code else 0} ==="]
    return "\n".join(lignes + ["=== TOUT FINI ==="]) + "\n"


def recette(score, rejeu=True, prefixe="RA", modeles=None):
    lignes = [
        "phrases : C:\\r\\annecy.json (20) ; noyau : C:\\c.exe" + (f" ; modèles : {modeles}" if modeles else ""),
        f"[0.2] == {prefixe}01 : Qu'est-ce qu'il y a comme couches dans mon projet ?",
        "  JUSTE",
        "",
        score,
        "",
        "[159.6] FIN : rejeu identique" if rejeu else "[159.6] FIN : rejeu NON identique",
    ]
    if rejeu is None:
        lignes.pop()
    return "\n".join(lignes) + "\n"


class Cas(unittest.TestCase):
    def setUp(self):
        self._dossier = tempfile.TemporaryDirectory()
        self.dossier = Path(self._dossier.name)
        self.n = 0

    def tearDown(self):
        self._dossier.cleanup()

    def fichier(self, contenu, nom=None):
        self.n += 1
        chemin = self.dossier / (nom or f"releve-{self.n}.txt")
        if isinstance(contenu, bytes):
            chemin.write_bytes(contenu)
        else:
            chemin.write_text(contenu, encoding="utf-8")
        return chemin

    def lancer(self, parent, candidat):
        """Lance l'outil ; rend (code, sortie, erreur, résultat JSON ou None)."""
        sortie_json = self.dossier / "resultat.json"
        sortie_json.unlink(missing_ok=True)  # jamais le résultat d'un lancement précédent
        args = ["--parent", *map(str, parent), "--candidat", *map(str, candidat), "--json", str(sortie_json)]
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = non_oubli.main(args)
        resultat = json.loads(sortie_json.read_text(encoding="utf-8")) if sortie_json.exists() else None
        return code, out.getvalue(), err.getvalue(), resultat

    def rapport(self, justes, total, par_sorte=None, modele="ajuste-x"):
        dossier = self.dossier / f"{modele}-{self.n}"
        self.n += 1
        dossier.mkdir()
        validation = {"justes": justes, "total": total}
        if par_sorte:
            validation["par_sorte"] = {k: {"justes": 0, "total": v} for k, v in par_sorte.items()}
        (dossier / "rapport.json").write_text(json.dumps({"validation": validation}), encoding="utf-8")
        return dossier / "rapport.json"

    @staticmethod
    def verdict(resultat, barre):
        return next(l["verdict"] for l in resultat["barres"] if l["barre"] == barre)


class Matrice(Cas):
    def test_barre_tenue(self):
        p = self.fichier(essais(("plan_reel", "SCORE 25/25 plans conformes")))
        c = self.fichier(essais(("plan_reel", "SCORE 24/25 plans conformes")))
        code, sortie, _, r = self.lancer([p], [c])
        self.assertEqual(code, 0)
        self.assertEqual(self.verdict(r, "plan_reel"), "tenue")
        self.assertEqual(r["verdict"], "installable")
        self.assertRegex(sortie, r"plan_reel\s+25/25 → 24/25\s+tenue")

    def test_baisse(self):
        p = self.fichier(essais(("plan_reel", "SCORE 25/25 plans conformes")))
        c = self.fichier(essais(("plan_reel", "SCORE 23/25 plans conformes")))
        code, _, _, r = self.lancer([p], [c])
        self.assertEqual(code, 1)
        self.assertEqual(self.verdict(r, "plan_reel"), "baisse")
        self.assertEqual(r["verdict"], "refusé")

    def test_barre_perdue(self):
        p = self.fichier(essais(("plan_reel", "SCORE 25/25"), ("style_reel", "SCORE 10/10 réponses conformes")))
        c = self.fichier(essais(("plan_reel", "SCORE 25/25")))
        code, _, _, r = self.lancer([p], [c])
        self.assertEqual(code, 1)
        self.assertEqual(self.verdict(r, "style_reel"), "absente")
        self.assertEqual(r["verdict"], "refusé")

    def test_barre_nouvelle(self):
        p = self.fichier(essais(("plan_reel", "SCORE 25/25")))
        c = self.fichier(
            essais(("plan_reel", "SCORE 25/25"))
            + "=== par_commande_reel ===\nSCORE terrain.slope 3/3\nSCORE total 3/3\n=== fin par_commande_reel : 0 ===\n"
        )
        code, _, _, r = self.lancer([p], [c])
        self.assertEqual(code, 3)
        self.assertEqual(self.verdict(r, "par_commande_reel"), "sans parent")
        self.assertEqual(self.verdict(r, "par_commande:terrain.slope"), "sans parent")
        self.assertEqual(r["verdict"], "à décider")

    def test_totaux_differents(self):
        p = self.fichier("== phrase 1 : Trouve les parcelles\n\nPLANS EXECUTES : 4/5 ; ratés : [5]\n\nFIN : rejeu identique\n",
                         nom="recette-annecy-ancienne.txt")
        c = self.fichier(recette("SCORE : 18/20 (barre 16) : barre tenue"))
        code, _, _, r = self.lancer([p], [c])
        self.assertEqual(code, 3)
        self.assertEqual(self.verdict(r, "recette:annecy"), "non comparable")
        self.assertEqual(self.verdict(r, "recette:annecy:rejeu"), "tenue")
        self.assertEqual(r["verdict"], "à décider")

    def test_rejeu_perdu(self):
        p = self.fichier(recette("SCORE : 18/20 (barre 16) : barre tenue"))
        c = self.fichier(recette("SCORE : 18/20 (barre 16) : barre tenue", rejeu=False))
        code, _, _, r = self.lancer([p], [c])
        self.assertEqual(code, 1)
        self.assertEqual(self.verdict(r, "recette:annecy:rejeu"), "baisse")
        self.assertEqual(self.verdict(r, "recette:annecy"), "tenue")

    def test_par_commande(self):
        bloc = "=== par_commande_reel ===\nSCORE terrain.slope {}\nSCORE total {}\n=== fin par_commande_reel : 0 ===\n"
        p = self.fichier(bloc.format("3/3", "99/99"))
        c = self.fichier(bloc.format("1/3", "97/99"))
        code, _, _, r = self.lancer([p], [c])
        self.assertEqual(code, 1)
        self.assertEqual(self.verdict(r, "par_commande:terrain.slope"), "baisse")
        self.assertEqual(self.verdict(r, "par_commande_reel"), "baisse")

    def test_fichier_illisible(self):
        bon = self.fichier(essais(("plan_reel", "SCORE 25/25")))
        for contenu in ("", "\ufeff\r\n", "rien de connu ici\n", '{"validation": {"justes": "x"}}'):
            with self.subTest(contenu=contenu):
                mauvais = self.fichier(contenu, nom=f"mauvais-{self.n}.log")
                code, sortie, erreur, r = self.lancer([bon], [mauvais])
                self.assertEqual(code, 2)
                self.assertIn(str(mauvais), erreur)
                self.assertIsNone(r)
        absent = self.dossier / "absent.log"
        code, _, erreur, _ = self.lancer([bon], [absent])
        self.assertEqual(code, 2)
        self.assertIn(str(absent), erreur)


    def test_validation_de_jeux_differents_indicative(self):
        e1 = self.fichier(essais(("plan_reel", "SCORE 25/25")))
        e2 = self.fichier(essais(("plan_reel", "SCORE 25/25")))
        p = self.rapport(291, 300, {"calls": 253, "question": 25, "refusal": 22}, modele="ajuste-p")
        c = self.rapport(280, 300, {"calls": 249, "question": 38, "refusal": 13}, modele="ajuste-c")
        code, sortie, _, r = self.lancer([e1, p], [e2, c])
        ligne = next(l for l in r["barres"] if l["barre"] == "validation")
        self.assertTrue(ligne["indicative"])
        self.assertEqual(ligne["verdict"], "baisse")
        self.assertEqual(code, 0)
        self.assertIn("validation indicative", sortie)

    def test_validation_de_meme_jeu(self):
        sortes = {"calls": 253, "question": 25, "refusal": 22}
        p = self.rapport(291, 300, sortes, modele="ajuste-p")
        c = self.rapport(289, 300, sortes, modele="ajuste-c")
        code, _, _, r = self.lancer([p], [c])
        ligne = next(l for l in r["barres"] if l["barre"] == "validation")
        self.assertFalse(ligne["indicative"])
        self.assertEqual(ligne["verdict"], "baisse")
        self.assertEqual(code, 1)

    def test_validation_sans_composition_indicative(self):
        e1 = self.fichier(essais(("plan_reel", "SCORE 25/25")))
        e2 = self.fichier(essais(("plan_reel", "SCORE 25/25")))
        p, c = self.rapport(291, 300, modele="ajuste-p"), self.rapport(289, 300, modele="ajuste-c")
        code, _, _, r = self.lancer([e1, p], [e2, c])
        self.assertTrue(next(l for l in r["barres"] if l["barre"] == "validation")["indicative"])
        self.assertEqual(code, 0)

    def test_seuil_perdu(self):
        p = self.fichier(essais(("epic11_reel", "SCORE 25/25", 0)))
        c = self.fichier(essais(("epic11_reel", "SCORE 25/25", 101)))
        code, sortie, _, r = self.lancer([p], [c])
        self.assertEqual(code, 1)
        self.assertEqual(self.verdict(r, "epic11_reel"), "baisse")
        self.assertRegex(sortie, r"epic11_reel\s+25/25 → 25/25\s+baisse\s+seuil ok → raté \(101\)")

    def test_seuil_deja_rate_chez_le_parent(self):
        p = self.fichier(essais(("epic11_reel", "SCORE 24/25", 101)))
        c = self.fichier(essais(("epic11_reel", "SCORE 24/25", 101)))
        self.assertEqual(self.lancer([p], [c])[0], 0)

    def test_recette_sous_sa_barre(self):
        p = self.fichier(recette("SCORE : 16/20 (barre 16) : barre tenue"))
        for fin in ("SOUS LA BARRE", "barre non tenue"):
            with self.subTest(fin=fin):
                c = self.fichier(recette(f"SCORE : 15/20 (barre 16) : {fin}"))
                code, sortie, _, r = self.lancer([p], [c])
                self.assertEqual(code, 1)
                self.assertEqual(self.verdict(r, "recette:annecy"), "baisse")
                self.assertIn("seuil ok → raté", sortie)


class Regles(Cas):
    def test_la_recette_du_gros_projet_reste_indicative(self):
        p = self.fichier(essais(("plan_reel", "SCORE 25/25")))
        gros_p = self.fichier(recette("SCORE : 7/7 (barre 6) : barre tenue", prefixe="G"))
        gros_c = self.fichier(recette("SCORE : 3/7 (barre 6) : SOUS LA BARRE", prefixe="G", rejeu=False))
        code, sortie, _, r = self.lancer([p, gros_p], [p, gros_c])
        self.assertEqual(code, 0)
        self.assertEqual(self.verdict(r, "recette:gros"), "baisse")
        self.assertIn("(indicative)", sortie)

    def test_une_barre_lue_deux_fois_est_une_erreur(self):
        a = self.fichier(essais(("plan_reel", "SCORE 25/25")))
        b = self.fichier(essais(("plan_reel", "SCORE 24/25")))
        code, _, erreur, _ = self.lancer([a, b], [a])
        self.assertEqual(code, 2)
        self.assertIn("plan_reel", erreur)

    def test_un_candidat_meilleur_tient(self):
        p = self.fichier(essais(("ecart_reel", "SCORE 30/40 : plans 20/28")))
        c = self.fichier(essais(("ecart_reel", "SCORE 38/40 : plans 26/28")))
        self.assertEqual(self.lancer([p], [c])[0], 0)


    def test_refuse_l_emporte_sur_a_decider(self):
        p = self.fichier(essais(("plan_reel", "SCORE 25/25")))
        c = self.fichier(essais(("plan_reel", "SCORE 20/25"), ("style_reel", "SCORE 10/10")))
        code, _, _, r = self.lancer([p], [c])
        self.assertEqual(self.verdict(r, "style_reel"), "sans parent")
        self.assertEqual((code, r["verdict"]), (1, "refusé"))

    def test_aucune_barre_comptee_est_a_decider(self):
        p = self.fichier(recette("SCORE : 7/7 (barre 6) : barre tenue", prefixe="G"))
        code, sortie, _, r = self.lancer([p], [p])
        self.assertEqual((code, r["verdict"]), (3, "à décider"))
        self.assertIn("Aucune barre comptée", sortie)

    def test_meme_modele_des_deux_cotes(self):
        p = self.fichier(essais(("plan_reel", "SCORE 25/25"), modele="C:\\m\\ajuste-x\\x.gguf"))
        code, _, _, r = self.lancer([p], [p])
        self.assertTrue(any("même modèle" in a for a in r["avertissements"]))
        self.assertEqual((code, r["verdict"]), (3, "à décider"))

    def test_modeles_melanges_a_decider(self):
        a = self.fichier(essais(("plan_reel", "SCORE 25/25"), modele="C:\\m\\ajuste-a\\x.gguf"))
        b = self.fichier(essais(("plan_reel", "SCORE 25/25"), modele="C:\\m\\ajuste-b\\x.gguf"))
        c = self.fichier(recette("SCORE : 18/20 (barre 16) : barre tenue", modeles="C:\\m\\ajuste-c"))
        d = self.fichier(recette("SCORE : 18/20 (barre 16) : barre tenue"))
        code, _, _, r = self.lancer([a, d], [b, c])
        self.assertTrue(any("plusieurs modèles" in m for m in r["avertissements"]))
        self.assertEqual((code, r["verdict"]), (3, "à décider"))

    def test_json_egal_a_un_releve(self):
        p = self.fichier(essais(("plan_reel", "SCORE 25/25")))
        c = self.fichier(essais(("plan_reel", "SCORE 25/25")))
        avant = c.read_bytes()
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            code = non_oubli.main(["--parent", str(p), "--candidat", str(c), "--json", str(c)])
        self.assertEqual(code, 2)
        self.assertEqual(c.read_bytes(), avant)

    def test_json_perime_efface_sur_releve_illisible(self):
        p = self.fichier(essais(("plan_reel", "SCORE 25/25")))
        mauvais = self.fichier("")
        perime = self.dossier / "perime.json"
        perime.write_text('{"verdict": "installable", "code": 0}', encoding="utf-8")
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            code = non_oubli.main(["--parent", str(p), "--candidat", str(mauvais), "--json", str(perime)])
        self.assertEqual(code, 2)
        self.assertFalse(perime.exists())

    def test_validation_d_un_seul_cote_indicative(self):
        e1 = self.fichier(essais(("plan_reel", "SCORE 25/25")))
        e2 = self.fichier(essais(("plan_reel", "SCORE 25/25")))
        v = self.rapport(291, 300, {"calls": 253, "question": 25, "refusal": 22}, modele="ajuste-p")
        for parent, candidat, attendu in (([e1, v], [e2], "absente"), ([e1], [e2, v], "sans parent")):
            with self.subTest(attendu=attendu):
                code, _, _, r = self.lancer(parent, candidat)
                ligne = next(l for l in r["barres"] if l["barre"] == "validation")
                self.assertEqual(ligne["verdict"], attendu)
                self.assertTrue(ligne["indicative"])
                self.assertEqual(code, 0)
                self.assertTrue(any("d'un seul côté" in a for a in r["avertissements"]))

    def test_section_sans_fin(self):
        p = self.fichier(essais(("plan_reel", "SCORE 25/25", 0)))
        c = self.fichier("=== plan_reel ===\nSCORE 25/25 plans conformes\n")
        code, _, _, r = self.lancer([p], [c])
        self.assertTrue(any("sans ligne `=== fin" in a for a in r["avertissements"]))
        self.assertEqual(self.verdict(r, "plan_reel"), "non comparable")
        self.assertEqual(code, 3)

    def test_deux_score_dans_une_section(self):
        c = self.fichier("=== plan_reel ===\nSCORE 20/25\nSCORE 24/25\n=== fin plan_reel : 0 ===\n")
        releves = non_oubli.lire_releves([c])
        self.assertEqual(releves.barres["plan_reel"].justes, 24)
        self.assertTrue(any("plusieurs lignes SCORE" in a for a in releves.avertissements))

    def test_score_malforme_ou_sur_zero(self):
        c = self.fichier(
            "=== par_commande_reel ===\nSCORE terrain.slope 3 / 3\nSCORE terrain.aspect 0/0\n"
            "SCORE geo.distance_azimuth 3/3\nSCORE total 3/3\n=== fin par_commande_reel : 0 ===\n"
        )
        releves = non_oubli.lire_releves([c])
        self.assertEqual(set(releves.barres), {"par_commande:geo.distance_azimuth", "par_commande_reel"})
        self.assertTrue(any("SCORE terrain.slope 3 / 3" in a and "forme inconnue" in a for a in releves.avertissements))
        self.assertTrue(any("zéro phrase" in a for a in releves.avertissements))

    def test_seuil_de_recette_calcule_par_la_barre(self):
        for ligne, attendu in (("SCORE : 16/20 (barre 16)", True), ("SCORE : 15/20 (barre 16) : barre tenue", False)):
            with self.subTest(ligne=ligne):
                releves = non_oubli.lire_releves([self.fichier(recette(ligne))])
                self.assertIs(releves.barres["recette:annecy"].seuil, attendu)
        releves = non_oubli.lire_releves([self.fichier(recette("SCORE : 16/20"))])
        self.assertIsNone(releves.barres["recette:annecy"].seuil)
        self.assertTrue(any("sans seuil" in a for a in releves.avertissements))

    def test_recette_inconnue_et_nombre_de_phrases(self):
        texte = (
            "phrases : C:\\d\\phrases-gros-v2.json (9) ; noyau : C:\\c.exe\n"
            "SCORE : 6/7 (barre 6) : barre tenue\nFIN : rejeu identique\n"
        )
        releves = non_oubli.lire_releves([self.fichier(texte)])
        self.assertIn("recette:gros-v2", releves.barres)
        self.assertTrue(any("inconnue" in a for a in releves.avertissements))
        self.assertTrue(any("9 phrases annoncées, score sur 7" in a for a in releves.avertissements))

    def test_seuil_perdu_sur_des_totaux_differents(self):
        p = self.fichier(recette("SCORE : 5/5 (barre 4) : barre tenue"))
        c = self.fichier(recette("SCORE : 15/20 (barre 16) : SOUS LA BARRE"))
        code, _, _, r = self.lancer([p], [c])
        self.assertEqual(self.verdict(r, "recette:annecy"), "baisse")
        self.assertEqual(code, 1)

    def test_fraction_impossible(self):
        for score in ("SCORE 0/0", "SCORE 5/3"):
            with self.subTest(score=score):
                bon = self.fichier(essais(("plan_reel", "SCORE 3/3")))
                mauvais = self.fichier(essais(("plan_reel", score)))
                code, _, erreur, r = self.lancer([bon], [mauvais])
                self.assertEqual(code, 2)
                self.assertIn(str(mauvais), erreur)
                self.assertIsNone(r)
        bon = self.rapport(3, 3)
        mauvais = self.rapport(0, 0)
        self.assertEqual(self.lancer([bon], [mauvais])[0], 2)

    def test_erreur_imprevue_sort_en_2(self):
        p = self.fichier(essais(("plan_reel", "SCORE 25/25")))
        c = self.fichier(essais(("plan_reel", "SCORE 20/25")))
        err = io.StringIO()
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(err):
            code = non_oubli.main(
                ["--parent", str(p), "--candidat", str(c), "--json", str(self.dossier / "absent" / "r.json")]
            )
        self.assertEqual(code, 2)
        self.assertIn("erreur", err.getvalue())

    def test_recette_sans_fin(self):
        p = self.fichier(recette("SCORE : 18/20 (barre 16) : barre tenue"))
        c = self.fichier(recette("SCORE : 18/20 (barre 16) : barre tenue", rejeu=None))
        code, _, _, r = self.lancer([p], [c])
        self.assertEqual(self.verdict(r, "recette:annecy:rejeu"), "absente")
        self.assertEqual(code, 1)
        self.assertTrue(any("sans ligne `FIN`" in a for a in r["avertissements"]))

    def test_deux_lignes_de_score_dans_une_recette(self):
        c = self.fichier(recette("SCORE : 10/20 (barre 16) : SOUS LA BARRE\nSCORE : 18/20 (barre 16) : barre tenue"))
        releves = non_oubli.lire_releves([c])
        self.assertEqual(releves.barres["recette:annecy"].justes, 18)
        self.assertTrue(any("plusieurs lignes de score" in a for a in releves.avertissements))

    def test_section_finie_sans_score(self):
        c = self.fichier(essais(("plan_reel", "SCORE 25/25")) + "=== style_reel ===\n=== fin style_reel : 101 ===\n")
        releves = non_oubli.lire_releves([c])
        self.assertNotIn("style_reel", releves.barres)
        self.assertTrue(any("sans ligne SCORE" in a for a in releves.avertissements))

    def test_section_relancee(self):
        bloc = "=== par_commande_reel ===\n{}=== fin par_commande_reel : 0 ===\n"
        c = self.fichier(
            bloc.format("SCORE terrain.slope 1/3\nSCORE terrain.aspect 3/3\nSCORE total 4/6\n")
            + bloc.format("SCORE terrain.slope 3/3\nSCORE total 3/3\n")
            + essais(("plan_reel", "SCORE 20/25"), ("plan_reel", "SCORE 25/25"))
        )
        releves = non_oubli.lire_releves([c])
        scores = {b: (s.justes, s.total) for b, s in releves.barres.items()}
        self.assertEqual(
            scores, {"par_commande:terrain.slope": (3, 3), "par_commande_reel": (3, 3), "plan_reel": (25, 25)}
        )
        self.assertEqual(sum("écrite deux fois" in a for a in releves.avertissements), 2)

    def test_score_de_commande_hors_de_sa_section(self):
        c = self.fichier(essais(("plan_reel", "SCORE 25/25")) + "SCORE terrain.slope 1/3\n")
        releves = non_oubli.lire_releves([c])
        self.assertNotIn("par_commande:terrain.slope", releves.barres)
        self.assertTrue(any("hors de la section par_commande_reel" in a for a in releves.avertissements))


class VraisFormats(Cas):
    """Extraits des relevés gardés : BOM, fins de ligne Windows, accents abîmés par la console,
    sous-scores d'`ecart_reel`, rapport d'entraînement. Seul écart avec le vrai relevé de
    `ajuste-5-49-a` : le code de fin d'`epic11_reel` y est 0, il est mis à 101 ici pour lire un
    seuil raté."""

    ESSAIS = (
        "\ufeff=== plan_reel ===\r\n"
        "cargo : warning: method `params` is never used\r\n"
        "Mod\u251c\u00bfle : C:\\Users\\k\\cassini-modeles\\ajuste-5-49-a\\cassini-composition-4b-Q4_K_M.gguf ; adaptateur : aucun ; \r\n"
        "SCORE 24/25 plans conformes, 1 fausses questions, dont 0 du garde-fou\r\n"
        "test plan_reel_at_least_80_percent_of_the_plans_match ... ok\r\n"
        "=== fin plan_reel : 0 ===\r\n"
        "=== ecart_reel ===\r\n"
        "SCORE 35/40 : plans 23/28, questions 8/8, refus 4/4, dont 0 question(s) du garde-fou (Short, 186 s)\r\n"
        "=== fin ecart_reel : 0 ===\r\n"
        "=== epic11_reel ===\r\n"
        "SCORE 27/28 plans conformes ; carroyage sans syst\u251c\u00bfme : question ; points d'un autre syst\u251c\u00bfme (hors barre) : 3/4\r\n"
        "=== fin epic11_reel : 101 ===\r\n"
        "=== TOUT FINI ===\r\n"
    )
    RECETTE = (
        "\ufeffpython-qgis-ltr.bat : Warning 1: Field format 'character varying' not supported\r\n"
        "phrases : C:\\Users\\k\\recette\\annecy.json (20) ; noyau : C:\\ct\\cassini.exe ; mod\u251c\u00bfles : C:\\Users\\k\\cassini-modeles\\ajuste-5-49-a\r\n"
        "[0.2] == RA01 : Qu'est-ce qu'il y a comme couches dans mon projet ?\r\n"
        "SCORE : 18/20 (barre 16) : barre tenue\r\n"
        "rejeu en session (indicatif) : en \u251c\u00aechec : replay.diverged\r\n"
        "[159.6] FIN : rejeu identique\r\n"
    )

    def test_les_extraits_se_lisent(self):
        essais_ = self.fichier(self.ESSAIS.encode("utf-8"), nom="essais-gpu-a.log")
        rec = self.fichier(self.RECETTE.encode("utf-8"), nom="r.txt")
        rapport_dossier = self.dossier / "ajuste-5-49-a"
        rapport_dossier.mkdir()
        rapport = rapport_dossier / "rapport.json"
        rapport.write_text(
            json.dumps(
                {
                    "sorties": {"cassini-composition-4b-Q4_K_M.gguf": {"sha256": "abc"}},
                    "validation": {"justes": 289, "total": 300, "modele": "cassini-composition-4b-Q4_K_M.gguf"},
                },
                ensure_ascii=False,
            ),
            encoding="utf-8-sig",
        )
        releves = non_oubli.lire_releves([essais_, rec, rapport])
        scores = {b: (s.justes, s.total) for b, s in releves.barres.items()}
        self.assertEqual(
            scores,
            {
                "plan_reel": (24, 25),
                "ecart_reel": (35, 40),
                "epic11_reel": (27, 28),
                "recette:annecy": (18, 20),
                "recette:annecy:rejeu": (1, 1),
                "validation": (289, 300),
            },
        )
        self.assertEqual(releves.barres["epic11_reel"].code, 101)
        self.assertEqual(non_oubli.modeles_nommes(releves), ["ajuste-5-49-a"])
        sha = [i.get("sha256") for i in releves.identites if i["sorte"] == "rapport"]
        self.assertEqual(sha, ["abc"])

    def test_identite_lue_d_une_recette(self):
        c = self.fichier(recette("SCORE : 18/20 (barre 16) : barre tenue", modeles="C:\\m\\ajuste-5-49-a"))
        releves = non_oubli.lire_releves([c])
        self.assertEqual(non_oubli.modeles_nommes(releves), ["ajuste-5-49-a"])
        self.assertEqual(releves.identites[0]["chemin"], "C:\\m\\ajuste-5-49-a")

    def test_nom_tire_de_la_ligne_phrases(self):
        texte = (
            "phrases : C:\\d\\essais-5-48\\phrases-gros.json (7) ; noyau : C:\\c.exe ; modèles : C:\\m\\ajuste-x\n"
            "SCORE : 6/7 (barre 6) : barre tenue\nFIN : rejeu identique\n"
        )
        p = self.fichier(texte, nom="r1.txt")
        e = self.fichier(essais(("plan_reel", "SCORE 25/25")))
        releves = non_oubli.lire_releves([p])
        self.assertIn("recette:gros", releves.barres)
        self.assertEqual(releves.avertissements, [])
        _, _, _, r = self.lancer([e, p], [e, p])
        self.assertTrue(next(l for l in r["barres"] if l["barre"] == "recette:gros")["indicative"])

    def test_nom_tire_du_fichier_est_signale(self):
        p = self.fichier("== phrase 1 : x\nPLANS EXECUTES : 4/5\nFIN : rejeu identique\n", nom="recette-annecy-vieille.txt")
        releves = non_oubli.lire_releves([p])
        self.assertIn("recette:annecy", releves.barres)
        self.assertIsNone(releves.barres["recette:annecy"].seuil)
        self.assertTrue(any("d'après le nom du fichier" in a for a in releves.avertissements))

    def test_adaptateur_vide_sans_point_virgule_final(self):
        texte = (
            "=== plan_reel ===\r\n"
            "Mod\u251c\u00bfle : C:\\m\\ajuste-nuit-11-10-fusion\\cassini-composition-4b-Q4_K_M.gguf ; adaptateur : \r\n"
            "SCORE 25/25 plans conformes\r\n=== fin plan_reel : 0 ===\r\n"
        )
        releves = non_oubli.lire_releves([self.fichier(texte.encode("utf-8"))])
        self.assertEqual(non_oubli.modeles_nommes(releves), ["ajuste-nuit-11-10-fusion"])
        self.assertEqual(releves.identites[0]["adaptateur"], "aucun")
        self.assertTrue(releves.barres["plan_reel"].seuil)

    def test_deux_modeles_dans_un_meme_cote_sont_signales(self):
        a = self.fichier(essais(("plan_reel", "SCORE 25/25"), modele="C:\\m\\ajuste-a\\x.gguf"))
        b = self.fichier(essais(("style_reel", "SCORE 10/10"), modele="C:\\m\\ajuste-b\\x.gguf"))
        _, sortie, _, r = self.lancer([a, b], [a, b])
        self.assertTrue(any("plusieurs modèles" in m for m in r["avertissements"]))
        self.assertIn("attention", sortie)


if __name__ == "__main__":
    unittest.main()
