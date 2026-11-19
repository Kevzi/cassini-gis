"""Tests du jugement de la recette QGIS en phrases enchaînées (5.45), sans QGIS."""

import json
import os
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import recette_verdict as rv  # noqa: E402

ANNECY = os.path.join(HERE, "recette", "annecy.json")
GROS = os.path.join(HERE, "recette", "gros-annecy.json")
GROS_V2 = os.path.join(HERE, "recette", "gros-annecy-v2.json")
INVENTAIRE_GROS = os.path.join(HERE, "..", "..", "..", "donnees-test-cassini", "annecy",
                               "essais-5-48", "inventaire-gros.json")
LAYERS = rv.layer_index([("gares_1", "gares"), ("communes_2", "communes")])

BUFFER = {"command": "analysis.buffer",
          "params": {"layer_id": "@gares", "distance": {"value": 500, "unit": "m"}}}


def phrases(*expects):
    return json.dumps([{"id": "P%d" % i, "text": "phrase %d" % i, "expect": e}
                       for i, e in enumerate(expects, 1)])


def shown(*calls):
    return {"kind": "calls", "calls": list(calls)}


class SameJson(unittest.TestCase):
    def test_key_order_does_not_count(self):
        self.assertTrue(rv.same_json({"a": 1, "b": [1, "x"]}, {"b": [1, "x"], "a": 1}))

    def test_an_integer_is_not_a_float(self):
        self.assertFalse(rv.same_json(5, 5.0))
        self.assertTrue(rv.same_json(1.7, 1.7))

    def test_a_boolean_is_not_a_number(self):
        self.assertFalse(rv.same_json(True, 1))
        self.assertFalse(rv.same_json(0, False))

    def test_call_order_counts(self):
        self.assertFalse(rv.same_json([1, 2], [2, 1]))

    def test_a_missing_or_extra_key_differs(self):
        self.assertFalse(rv.same_json({"a": 1}, {"a": 1, "b": 2}))


class Load(unittest.TestCase):
    def test_layer_names_become_ids(self):
        [p] = rv.load(phrases({"plan": [BUFFER]}), LAYERS)
        self.assertEqual(p["plan"][0]["params"]["layer_id"], "gares_1")
        self.assertEqual(p["plan"][0]["params"]["distance"], {"value": 500, "unit": "m"})

    def test_names_inside_lists_are_resolved(self):
        call = {"command": "map.topographic_style", "params": {"layers": ["@gares", "@communes"]}}
        [p] = rv.load(phrases({"plan": [call]}), LAYERS)
        self.assertEqual(p["plan"][0]["params"]["layers"], ["gares_1", "communes_2"])

    def test_an_absent_layer_stops_the_probe(self):
        call = {"command": "layer.describe", "params": {"layer_id": "@routes"}}
        with self.assertRaisesRegex(rv.RecetteError, "routes"):
            rv.load(phrases({"plan": [call]}), LAYERS)

    def test_a_duplicated_layer_name_stops_the_probe(self):
        layers = rv.layer_index([("a", "gares"), ("b", "gares")])
        with self.assertRaisesRegex(rv.RecetteError, "plusieurs"):
            rv.load(phrases({"plan": [BUFFER]}), layers)

    def test_a_malformed_expectation_is_refused(self):
        for bad in ({}, {"plan": []}, {"question": False}, {"plan": [{"command": "x"}]}):
            with self.assertRaises(rv.RecetteError, msg=bad):
                rv.load(phrases(bad), LAYERS)

    def test_ids_are_unique(self):
        text = json.dumps([{"id": "A", "text": "x", "expect": {"question": True}}] * 2)
        with self.assertRaisesRegex(rv.RecetteError, "double"):
            rv.load(text, LAYERS)


class Judge(unittest.TestCase):
    def setUp(self):
        self.plan, self.question = rv.load(phrases({"plan": [BUFFER]}, {"question": True}), LAYERS)
        self.right = shown({"command": "analysis.buffer",
                            "params": {"distance": {"unit": "m", "value": 500}, "layer_id": "gares_1"}})

    def test_a_conforming_plan_is_executed(self):
        self.assertIs(rv.on_proposed(self.plan, self.right), rv.RIGHT)

    def test_a_plan_doing_something_else_is_a_miss(self):
        other = shown({"command": "analysis.buffer",
                       "params": {"distance": {"unit": "m", "value": 300}, "layer_id": "gares_1"}})
        self.assertEqual(rv.on_proposed(self.plan, other), "plan non conforme")
        self.assertEqual(rv.on_proposed(self.plan, {"kind": "calls"}), "plan non conforme")

    def test_a_failed_execution_is_a_miss(self):
        reason = rv.on_exec_failed({"code": "analysis.unknown_field", "step": 1})
        self.assertIn("analysis.unknown_field", reason)
        self.assertIn("1", reason)

    def test_an_ambiguous_phrase_wants_a_question(self):
        self.assertIs(rv.on_question(self.question), rv.RIGHT)
        self.assertEqual(rv.on_proposed(self.question, self.right), "plan rendu au lieu d'une question")

    def test_a_question_on_a_clear_phrase_is_a_miss(self):
        self.assertEqual(rv.on_question(self.plan), "question sur une phrase claire")

    def test_a_refusal_is_a_miss(self):
        self.assertEqual(rv.on_failed({"code": "agent.refused"}), "refus : agent.refused")

    def test_the_bar_is_80_percent(self):
        self.assertEqual(rv.bar(20), 16)
        self.assertEqual(rv.bar(5), 4)
        self.assertEqual(rv.bar(60), 48)
        self.assertIn("barre tenue", rv.score_line(16, 20))
        self.assertIn("SOUS LA BARRE", rv.score_line(15, 20))
        self.assertTrue(rv.score_line(15, 20).startswith("SCORE : 15/20 (barre 16)"))


class AnnecyFile(unittest.TestCase):
    """Le fichier d'Annecy, lu avec les noms de couches du projet d'Annecy."""

    NAMES = ("batiments", "communes", "cours_d_eau", "gares", "parcelles", "plans_d_eau",
             "plu_zones", "routes", "voies_ferrees", "mnt_25m", "mnt_5m_annecy", "relief_ombre")

    def setUp(self):
        with open(ANNECY, encoding="utf-8") as f:
            self.text = f.read()
        self.phrases = rv.load(self.text, rv.layer_index((n + "_id", n) for n in self.NAMES))

    def test_twenty_phrases_with_a_question(self):
        self.assertEqual(len(self.phrases), 20)
        self.assertGreaterEqual(sum(rv.wants_question(p) for p in self.phrases), 1)

    def test_a_count_by_commune_with_its_name(self):
        counts = [c for p in self.phrases if p["plan"] for c in p["plan"]
                  if c["command"] == "analysis.count_points_in_polygon" and c["params"].get("label_field")]
        self.assertTrue(counts)

    def test_every_command_family_is_there(self):
        commands = {c["command"].split(".")[0] for p in self.phrases if p["plan"] for c in p["plan"]}
        for family in ("project", "layer", "analysis", "terrain", "coord", "geo", "map", "layout", "data"):
            self.assertIn(family, commands)




class Summary(unittest.TestCase):
    def test_too_long_refusals_are_counted_apart(self):
        missed = [("A", "refus : ai.prompt_too_long"), ("B", "plan non conforme"),
                  ("C", "refus : ai.prompt_too_long"), ("D", "refus : ai.autre")]
        self.assertEqual(rv.too_long_count(missed), 2)
        self.assertEqual(rv.too_long_count([]), 0)

    def test_durations_first_median_max(self):
        line = rv.durations_line([30.0, 2.0, 4.0, 8.0, 6.0])
        self.assertIn("première 30.0 s", line)
        self.assertIn("médiane 6.0 s", line)
        self.assertIn("maximale 30.0 s", line)

    def test_median_of_an_even_count(self):
        self.assertIn("médiane 3.0 s", rv.durations_line([9.0, 2.0, 4.0, 1.0]))

    def test_no_duration(self):
        self.assertIn("aucune", rv.durations_line([]))


class GrosAnnecy(unittest.TestCase):
    PATH = GROS

    def raw(self):
        with open(self.PATH, encoding="utf-8") as f:
            return json.load(f)

    def test_twenty_phrases_with_unique_ids(self):
        raw = self.raw()
        self.assertEqual(len(raw), 20)
        self.assertEqual(len({p["id"] for p in raw}), 20)
        self.assertEqual([p["id"] for p in raw], ["GA%02d" % i for i in range(1, 21)])

    def test_at_least_two_questions(self):
        self.assertGreaterEqual(sum(p["expect"] == {"question": True} for p in self.raw()), 2)

    def test_at_least_two_layers_found_by_a_value(self):
        self.assertGreaterEqual(sum("trouvée par une valeur" in p.get("note", "") for p in self.raw()), 2)

    def test_layer_names_exist_in_the_inventory(self):
        if not os.path.exists(INVENTAIRE_GROS):
            self.skipTest("inventaire-gros.json absent (données hors dépôt)")
        with open(INVENTAIRE_GROS, encoding="utf-8") as f:
            names = [l["name"] for l in json.load(f)["layers"]]
        with open(self.PATH, encoding="utf-8") as f:
            text = f.read()
        loaded = rv.load(text, rv.layer_index((n, n) for n in names))
        self.assertEqual(len(loaded), 20)



class GrosAnnecyV2(GrosAnnecy):
    PATH = GROS_V2


if __name__ == "__main__":
    unittest.main()
