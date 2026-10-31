"""Tests sans QGIS du choix du modèle dans les Réglages (ticket 5.14)."""

import os
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "cassini_qgis"))

import models  # noqa: E402

LISTED = [
    {"file": "Qwen3.5-9B-Q4_K_M.gguf", "size_bytes": 5_630_000_000},
    {"file": "ajuste.modele.json", "size_bytes": 2_700_000_000},
]


class ChooseTest(unittest.TestCase):
    def test_a_remembered_model_in_the_list_is_kept(self):
        self.assertEqual(models.choose(LISTED, "ajuste.modele.json"), ("ajuste.modele.json", False))

    def test_without_a_choice_the_first_is_taken_and_nothing_is_forgotten(self):
        for remembered in (None, ""):
            self.assertEqual(models.choose(LISTED, remembered), ("Qwen3.5-9B-Q4_K_M.gguf", False))

    def test_a_vanished_choice_is_forgotten_for_the_first(self):
        self.assertEqual(models.choose(LISTED, "b.gguf"), ("Qwen3.5-9B-Q4_K_M.gguf", True))

    def test_the_name_is_compared_as_is(self):
        self.assertEqual(models.choose(LISTED, "AJUSTE.modele.json")[1], True)

    def test_an_entry_with_a_problem_is_never_chosen(self):
        listed = [
            {"file": "a.modele.json", "size_bytes": 0, "problem": "déclaration illisible"},
            {"file": "b.modele.json", "size_bytes": 3, "problem": "fichier manquant : x.gguf"},
            {"file": "c.gguf", "size_bytes": 2},
        ]
        self.assertEqual(models.choose(listed, None), ("c.gguf", False))
        # Un choix mémorisé devenu inutilisable est oublié.
        self.assertEqual(models.choose(listed, "b.modele.json"), ("c.gguf", True))
        self.assertEqual(models.usable(models.clean(listed)), ["c.gguf"])

    def test_an_empty_or_unreadable_list_chooses_nothing(self):
        self.assertEqual(models.choose([], "b.gguf"), (None, True))
        self.assertEqual(models.choose(None, None), (None, False))
        self.assertEqual(models.choose([{"size_bytes": 3}, "x", {"file": 3}], None), (None, False))


class CleanTest(unittest.TestCase):
    def test_bad_fields_are_dropped_or_neutralised(self):
        got = models.clean([
            {"file": "a.gguf", "size_bytes": True, "problem": "  "},
            {"file": "b.gguf", "size_bytes": -1, "problem": 3},
            {"file": "c.gguf", "size_bytes": 7, "problem": "fichier manquant : d.gguf"},
        ])
        self.assertEqual(got, [
            {"file": "a.gguf", "size_bytes": None, "problem": None},
            {"file": "b.gguf", "size_bytes": None, "problem": None},
            {"file": "c.gguf", "size_bytes": 7, "problem": "fichier manquant : d.gguf"},
        ])


class LabelTest(unittest.TestCase):
    def test_name_and_readable_size(self):
        entry = models.clean(LISTED)[0]
        self.assertEqual(models.label("fr", entry), "Qwen3.5-9B-Q4_K_M.gguf (5,6 Go)")
        self.assertEqual(models.label("en", entry), "Qwen3.5-9B-Q4_K_M.gguf (5.6 GB)")

    def test_a_declaration_is_shown_by_its_name(self):
        entry = models.clean(LISTED)[1]
        self.assertTrue(models.label("fr", entry).startswith("ajuste.modele.json ("))

    def test_sizes_in_every_unit(self):
        self.assertEqual(models.size_text("fr", 950), "950 octets")
        self.assertEqual(models.size_text("fr", 1_500), "1,5 ko")
        self.assertEqual(models.size_text("en", 12_300_000), "12.3 MB")
        self.assertIsNone(models.size_text("fr", None))

    def test_a_problem_shows_its_reason(self):
        entry = models.clean([{"file": "b.modele.json", "size_bytes": 0,
                               "problem": "fichier manquant : x.gguf"}])[0]
        text = models.label("fr", entry)
        self.assertIn("b.modele.json", text)
        self.assertIn("fichier manquant : x.gguf", text)
        self.assertNotIn("octets", text)
        self.assertEqual(text, "b.modele.json — indisponible : fichier manquant : x.gguf")
        sized = models.clean([{"file": "c.modele.json", "size_bytes": 3,
                               "problem": "fichier manquant : y.gguf"}])[0]
        self.assertIn("(3 octets)", models.label("fr", sized))
        no_size = models.clean([{"file": "a.gguf"}])[0]
        self.assertEqual(models.label("en", no_size), "a.gguf")


class ReasonTest(unittest.TestCase):
    def test_an_error_shows_message_and_code(self):
        self.assertEqual(
            models.reason("fr", None, ("models.no_folder", "Le dossier désigné par CASSINI_MODELS "
                                       "n'existe pas : « (dossier des modèles) ».")),
            "Le dossier désigné par CASSINI_MODELS n'existe pas : « (dossier des modèles) ». "
            "(models.no_folder)",
        )

    def test_an_empty_list_or_only_broken_models_say_so(self):
        self.assertEqual(models.reason("fr", [], None), models.texts.tr("fr", "models_none"))
        broken = models.clean([{"file": "a.modele.json", "problem": "illisible"}])
        self.assertEqual(models.reason("en", broken, None), models.texts.tr("en", "models_none"))

    def test_a_usable_list_or_no_list_yet_has_no_reason(self):
        self.assertIsNone(models.reason("fr", models.clean(LISTED), None))
        self.assertIsNone(models.reason("fr", None, None))


class ReceivedTest(unittest.TestCase):
    """La décision du panneau à la réception de MODELS, et quand redemander la liste."""

    def test_the_list_is_cleaned_and_a_kept_choice_is_not_forgotten(self):
        listed = LISTED + [{"size_bytes": 3}, "pas une entrée"]
        entries, chosen, forget = models.received(listed, "ajuste.modele.json")
        self.assertEqual([e["file"] for e in entries], [e["file"] for e in LISTED])
        self.assertEqual((chosen, forget), ("ajuste.modele.json", False))

    def test_a_vanished_or_unusable_choice_is_forgotten(self):
        _, chosen, forget = models.received(LISTED, "b.gguf")
        self.assertEqual((chosen, forget), ("Qwen3.5-9B-Q4_K_M.gguf", True))
        listed = [{"file": "b.gguf", "size_bytes": 3, "problem": "illisible"}]
        self.assertEqual(models.received(listed, "b.gguf"), (models.clean(listed), None, True))

    def test_nothing_remembered_is_never_forgotten(self):
        self.assertEqual(models.received([], None), ([], None, False))
        self.assertEqual(models.received(None, ""), ([], None, False))

    def test_only_a_stale_choice_asks_the_list_again(self):
        self.assertTrue(models.asks_list_again("models.unknown"))
        self.assertTrue(models.asks_list_again("models.unusable"))
        for code in ("ai.no_model", "models.no_folder", "agent.refused", "", None):
            self.assertFalse(models.asks_list_again(code), code)


if __name__ == "__main__":
    unittest.main()
