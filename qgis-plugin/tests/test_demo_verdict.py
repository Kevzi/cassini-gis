"""Verdict de la sonde de démo (`demo_verdict.py`) et fixtures de `fixtures/demo/`, sans QGIS."""

import json
import math
import os
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
DEMO = os.path.join(ROOT, "fixtures", "demo")

import demo_verdict as v  # noqa: E402

PDF = "C:/k/demo.cassini/5-layout.export_pdf.pdf"
CALLS = [
    {"command": "analysis.select_by_attribute", "params": {}},
    {"command": "analysis.buffer", "params": {}},
    {"command": "analysis.intersect", "params": {}},
    {"command": "analysis.intersect", "params": {}},
    {"command": "layout.export_pdf", "params": {}},
]


def steps(with_export=True):
    out = [
        {"step": 1, "command": "analysis.select_by_attribute", "result": {"layer_id": "sel_1"}},
        {"step": 2, "command": "analysis.buffer", "result": {"layer_id": "tampon_2"}},
        {"step": 3, "command": "analysis.intersect", "result": {"layer_id": "inter_3"}},
        {"step": 4, "command": "analysis.intersect", "result": {"layer_id": "inter_4"}},
    ]
    if with_export:
        out.append({"step": 5, "command": "layout.export_pdf", "result": {"file": PDF, "page_count": 1}})
    return out


def good_facts():
    return {
        "features": [
            {"name": "P01", "zone": "U", "distance": 300.0},
            {"name": "P10", "zone": "U", "distance": 500.0},
        ],
        "pdf_head": b"%PDF-",
        "parcel_fields": True,
    }


class RetainedLayerTest(unittest.TestCase):
    def test_last_layer_before_the_export(self):
        self.assertEqual(v.retained_layer_id(steps()), "inter_4")

    def test_without_export_the_last_layer(self):
        self.assertEqual(v.retained_layer_id(steps(with_export=False)), "inter_4")

    def test_steps_after_the_export_are_ignored(self):
        later = steps() + [{"step": 6, "command": "analysis.buffer", "result": {"layer_id": "x"}}]
        self.assertEqual(v.retained_layer_id(later), "inter_4")

    def test_a_style_step_is_not_the_answer(self):
        styled = steps(with_export=False) + [
            {"step": 5, "command": "layer.set_style", "result": {"layer_id": "tampon_2"}},
            {"step": 6, "command": "layer.arrange", "result": {"layer_id": "sel_1"}},
        ]
        self.assertEqual(v.retained_layer_id(styled), "inter_4")

    def test_none_without_layer(self):
        self.assertIsNone(v.retained_layer_id([{"step": 1, "command": "layout.export_pdf", "result": {}}]))

    def test_pdf_file(self):
        self.assertEqual(v.pdf_file(steps()), PDF)
        self.assertIsNone(v.pdf_file(steps(with_export=False)))


class JudgeDoneTest(unittest.TestCase):
    def test_right_answer_is_ok(self):
        self.assertIsNone(v.judge_done(steps(), good_facts()))
        self.assertEqual(v.verdict_line(None), "VERDICT : OK")

    def test_feature_in_zone_n_is_named(self):
        facts = good_facts()
        facts["features"].append({"name": "P03", "zone": "N", "distance": 425.0})
        reason = v.judge_done(steps(), facts)
        self.assertIn("P03", reason)
        self.assertIn("zone N", reason)

    def test_feature_outside_zoning_is_named(self):
        facts = good_facts()
        facts["features"].append({"name": "P09", "zone": None, "distance": 10.0})
        self.assertIn("P09", v.judge_done(steps(), facts))

    def test_feature_too_far_is_named(self):
        facts = good_facts()
        facts["features"].append({"name": "P14", "zone": "U", "distance": 535.4})
        reason = v.judge_done(steps(), facts)
        self.assertIn("P14", reason)
        self.assertIn("535.4", reason)

    def test_layer_without_parcel_fields(self):
        facts = good_facts()
        facts["parcel_fields"] = False
        self.assertIn("parcelles", v.judge_done(steps(), facts))

    def test_injected_failure_that_never_happened(self):
        reason = v.judge_done(steps(), good_facts(), injected="terrain.slope")
        self.assertIn("terrain.slope", reason)

    def test_empty_layer(self):
        facts = good_facts()
        facts["features"] = []
        self.assertIn("vide", v.judge_done(steps(), facts))

    def test_retained_layer_missing_from_project(self):
        facts = good_facts()
        facts["features"] = None
        self.assertIn("inter_4", v.judge_done(steps(), facts))

    def test_no_layer_step(self):
        only_export = [{"step": 1, "command": "layout.export_pdf", "result": {"file": PDF}}]
        self.assertIsNotNone(v.judge_done(only_export, good_facts()))

    def test_plan_without_pdf(self):
        reason = v.judge_done(steps(with_export=False), good_facts())
        self.assertIn("PDF", reason)

    def test_unreadable_pdf_is_named(self):
        for head in (None, b"<html"):
            facts = good_facts()
            facts["pdf_head"] = head
            reason = v.judge_done(steps(), facts)
            self.assertIn(PDF, reason)
            self.assertTrue(v.verdict_line(reason).startswith("VERDICT : ECHEC — "))


ITEMS = ["map", "title", "legend", "scalebar", "northarrow", "sources"]


def layout_facts():
    return {"sources": ["Parcelles cadastrales"], "page_count": 1, "pdf_head": b"%PDF-", "pdf_size": 1234}


class JudgeReplayTest(unittest.TestCase):
    def test_identical_replay_is_ok(self):
        self.assertIsNone(v.judge_replay(v.REPLAY_DONE, {"steps": 5, "identical": True}, steps()))

    def test_diverged_names_step_and_command(self):
        payload = {"code": "replay.diverged", "message": "x", "step": 2}
        self.assertEqual(
            v.judge_replay(v.REPLAY_FAILED, payload, steps()),
            "rejeu : étape 2 analysis.buffer diverge",
        )

    def test_other_failure_code(self):
        payload = {"code": "replay.core_died", "message": "x", "step": 3}
        self.assertEqual(v.judge_replay(v.REPLAY_FAILED, payload, steps()), "rejeu : replay.core_died étape 3")

    def test_not_identical(self):
        self.assertIn("identique", v.judge_replay(v.REPLAY_DONE, {"steps": 5, "identical": False}, steps()))

    def test_wrong_step_count(self):
        reason = v.judge_replay(v.REPLAY_DONE, {"steps": 3, "identical": True}, steps())
        self.assertIn("3 étapes refaites, 5 exécutées", reason)

    def test_tamper_refused_on_given_project(self):
        self.assertEqual(
            v.tamper_refused_reason(True, True), "altération refusée sur un projet donné")
        self.assertIsNone(v.tamper_refused_reason(True, False))
        self.assertIsNone(v.tamper_refused_reason(False, True))


SLOPE_FILE = "C:/k/demo.cassini/1-terrain.slope.tif"


def slope_steps():
    return [
        {"step": 1, "command": "terrain.slope", "result": {"file": SLOPE_FILE}},
        {"step": 2, "command": "layer.open_raster", "result": {}},
        {"step": 3, "command": "layout.export_pdf", "result": {"file": PDF, "page_count": 1}},
    ]


def slope_facts(unit="degrees", dem="mnt-colline-2154.tif", commands=("terrain.slope",)):
    return {
        "calls": [{"command": c, "params": {"dem": dem, "unit": unit}} for c in commands],
        "dem_name": "C:/k/mnt-colline-2154.tif",
        "slope_in_project": True,
        "earlier_names": ["1-analysis.select_by_attribute", "2-analysis.buffer"],
        "sources2": ["parcelles-2154.geojson", "1-terrain.slope.tif"],
    }


class JudgeSlopeTest(unittest.TestCase):
    def test_slope_in_degrees_on_the_dem_is_ok(self):
        self.assertIsNone(v.judge_slope(slope_steps(), slope_facts()))

    def test_plan_without_slope(self):
        reason = v.judge_slope(slope_steps()[1:], slope_facts(commands=("terrain.viewshed",)))
        self.assertEqual(reason, "pente : terrain.slope absent du plan")

    def test_wrong_unit(self):
        self.assertEqual(
            v.judge_slope(slope_steps(), slope_facts(unit="percent")),
            "pente : unité percent au lieu de degrees")

    def test_wrong_dem(self):
        self.assertEqual(
            v.judge_slope(slope_steps(), slope_facts(dem="autre.tif")),
            "pente : dem autre.tif n'est pas le MNT")

    def test_slope_result_without_file(self):
        steps = slope_steps()
        steps[0]["result"] = {}
        self.assertEqual(
            v.judge_slope(steps, slope_facts()),
            "pente : terrain.slope sans fichier de pente dans son résultat")

    def test_slope_call_without_params_names_the_unit(self):
        facts = slope_facts()
        facts["calls"] = [{"command": "terrain.slope"}]
        self.assertEqual(
            v.judge_slope(slope_steps(), facts), "pente : unité None au lieu de degrees")

    def test_slope_layer_missing_from_project(self):
        facts = slope_facts()
        facts["slope_in_project"] = False
        self.assertEqual(v.judge_slope(slope_steps(), facts), "pente : couche absente du projet")

    def test_slope_missing_from_pdf_sources(self):
        facts = slope_facts()
        facts["sources2"] = ["parcelles-2154.geojson", "gares-2154.geojson"]
        self.assertIn("couche absente du PDF", v.judge_slope(slope_steps(), facts))

    def test_plan_1_outputs_left_in_the_pdf(self):
        facts = slope_facts()
        facts["sources2"] = ["parcelles-2154.geojson", "1-terrain.slope.tif", "2-analysis.buffer.gpkg"]
        reason = v.judge_slope(slope_steps(), facts)
        self.assertIn("sorties du plan 1 encore dans le PDF", reason)
        self.assertIn("2-analysis.buffer.gpkg", reason)

    def test_plan_1_output_of_another_command_is_detected_by_its_name(self):
        facts = slope_facts()
        facts["earlier_names"] = ["3-terrain.hillshade"]
        facts["sources2"] = ["1-terrain.slope.tif", "3-terrain.hillshade.tif"]
        self.assertIn("3-terrain.hillshade.tif", v.judge_slope(slope_steps(), facts))

    def test_a_longer_numbered_name_is_not_taken_for_a_plan_1_output(self):
        facts = slope_facts()
        facts["earlier_names"] = ["1-analysis.buffer"]
        facts["sources2"] = ["parcelles-2154.geojson", "1-terrain.slope.tif", "11-analysis.buffer.gpkg"]
        self.assertIsNone(v.judge_slope(slope_steps(), facts))

    def test_missing_dem_layer_in_project(self):
        self.assertIn("MNT", v.missing_layers_reason(True, True, True, has_dem=False))
        self.assertIsNone(v.missing_layers_reason(True, True, True))


class FontsTest(unittest.TestCase):
    def test_no_font_is_refused(self):
        self.assertIn("aucune police", v.fonts_reason(0))

    def test_fonts_present_is_ok(self):
        self.assertIsNone(v.fonts_reason(154))


class DropItemTest(unittest.TestCase):
    def test_valid_items_and_unset_are_accepted(self):
        for key, _ in v.LAYOUT_ITEMS:
            self.assertIsNone(v.drop_item_refused_reason(key))
        self.assertIsNone(v.drop_item_refused_reason(None))

    def test_typo_and_map_are_refused(self):
        for bad in ("legende", "legend ", "map"):
            self.assertIn("élément inconnu", v.drop_item_refused_reason(bad))


class JudgeReplayEdgeTest(unittest.TestCase):
    def test_unexpected_kind(self):
        self.assertIn("inattendu", v.judge_replay("autre", {}, steps()))

    def test_diverged_without_a_usable_step_falls_back(self):
        for payload in ({"code": "replay.diverged"}, {"code": "replay.diverged", "step": 99}):
            self.assertIn("diverge", v.judge_replay(v.REPLAY_FAILED, payload, steps()))
        self.assertEqual(
            v.judge_replay(v.REPLAY_FAILED, {"code": "replay.diverged"}, steps()),
            "rejeu : étape ? ? diverge")

    def test_non_dict_payload(self):
        self.assertIn("identique", v.judge_replay(v.REPLAY_DONE, None, steps()))


class JudgeLayoutTest(unittest.TestCase):
    def test_all_five_items_are_ok(self):
        self.assertIsNone(v.judge_layout(ITEMS, layout_facts()))

    def test_each_missing_item_is_named(self):
        for key, label in v.LAYOUT_ITEMS:
            reason = v.judge_layout([i for i in ITEMS if i != key], layout_facts())
            self.assertEqual(reason, "mise en page : %s absente" % label)

    def test_legend_message(self):
        self.assertEqual(
            v.judge_layout([i for i in ITEMS if i != "legend"], layout_facts()),
            "mise en page : légende absente",
        )

    def test_wrong_page_count(self):
        facts = layout_facts()
        facts["page_count"] = 2
        self.assertEqual(v.judge_layout(ITEMS, facts), "mise en page : 2 page(s) au lieu d'une")

    def test_empty_sources(self):
        facts = layout_facts()
        facts["sources"] = []
        self.assertIn("sources", v.judge_layout(ITEMS, facts))

    def test_pdf_not_a_pdf(self):
        facts = layout_facts()
        facts["pdf_head"] = b"<html"
        self.assertIn("illisible", v.judge_layout(ITEMS, facts))

    def test_empty_pdf(self):
        facts = layout_facts()
        facts["pdf_size"] = 0
        self.assertIn("vide", v.judge_layout(ITEMS, facts))


class FailureReasonTest(unittest.TestCase):
    def test_failed_step_names_step_command_and_code(self):
        payload = {"code": "qgis.demo_injected", "message": "x", "step": 5, "outputs": {}}
        self.assertEqual(
            v.failure_reason(v.PLAN_EXEC_FAILED, payload, CALLS),
            "étape 5 layout.export_pdf : qgis.demo_injected",
        )

    def test_failed_step_without_plan(self):
        reason = v.failure_reason(v.PLAN_EXEC_FAILED, {"code": "c", "step": None}, [])
        self.assertEqual(reason, "étape ? ? : c")

    def test_question(self):
        reason = v.failure_reason(v.PLAN_QUESTION, {"question": "Quelle commune ?", "source": "model"})
        self.assertEqual(reason, "question : Quelle commune ?")

    def test_refusal(self):
        reason = v.failure_reason(v.PLAN_FAILED, {"code": "agent.refused", "message": "Non."})
        self.assertIn("agent.refused", reason)

    def test_core_death_carries_the_message(self):
        self.assertEqual(v.failure_reason(v.DIED, "arrêté"), "noyau : arrêté")
        self.assertEqual(v.failure_reason(v.FAILED, "illisible"), "noyau : illisible")

    def test_cancelled(self):
        self.assertIsNotNone(v.failure_reason(v.PLAN_CANCELLED, {"step": 2, "outputs": {}}, CALLS))

    def test_other_events_are_not_failures(self):
        for kind in ("ready", "plan_step", "plan_proposed", "run_done"):
            self.assertIsNone(v.failure_reason(kind, {}))


class MissingLayersTest(unittest.TestCase):
    def test_named(self):
        self.assertIsNone(v.missing_layers_reason(True, True, True))
        self.assertIn("typezone", v.missing_layers_reason(False, True, True))
        self.assertIn("gares", v.missing_layers_reason(True, False, True))
        self.assertIn("parcelles", v.missing_layers_reason(True, True, False))


# -- fixtures -----------------------------------------------------------------
# Emprise du MNT `fixtures/mnt-colline-2154.tif` (README des fixtures).
MNT_BOX = (652000.0, 6850000.0, 652500.0, 6850500.0)


def load(name):
    with open(os.path.join(DEMO, name), encoding="utf-8") as f:
        return json.load(f)


def box(feature):
    ring = feature["geometry"]["coordinates"][0]
    xs, ys = [p[0] for p in ring], [p[1] for p in ring]
    return min(xs), min(ys), max(xs), max(ys)


def rect_distance(rect, point):
    x0, y0, x1, y1 = rect
    dx = max(x0 - point[0], 0.0, point[0] - x1)
    dy = max(y0 - point[1], 0.0, point[1] - y1)
    return math.hypot(dx, dy)


class FixturesTest(unittest.TestCase):
    def setUp(self):
        self.zoning = load("plu-zonage-2154.geojson")
        self.parcels = load("parcelles-2154.geojson")
        self.stations = load("gares-2154.geojson")

    def test_crs_is_lambert_93(self):
        for layer in (self.zoning, self.parcels, self.stations):
            self.assertEqual(layer["crs"]["properties"]["name"], "urn:ogc:def:crs:EPSG::2154")

    def test_fields_of_the_trial_inventory(self):
        for f in self.zoning["features"]:
            self.assertIn(f["properties"]["typezone"], ("U", "N"))
        for f in self.parcels["features"]:
            self.assertTrue({"id_parcelle", "section"} <= set(f["properties"]))
        for f in self.stations["features"]:
            self.assertIn("nom", f["properties"])
            self.assertEqual(f["geometry"]["type"], "Point")

    def test_all_layers_inside_the_hill_dem(self):
        for layer in (self.zoning, self.parcels):
            for f in layer["features"]:
                x0, y0, x1, y1 = box(f)
                self.assertTrue(MNT_BOX[0] <= x0 < x1 <= MNT_BOX[2], f["properties"])
                self.assertTrue(MNT_BOX[1] <= y0 < y1 <= MNT_BOX[3], f["properties"])
        for f in self.stations["features"]:
            x, y = f["geometry"]["coordinates"]
            self.assertTrue(MNT_BOX[0] < x < MNT_BOX[2] and MNT_BOX[1] < y < MNT_BOX[3])

    def test_sixteen_parcels_of_125_m(self):
        self.assertEqual(len(self.parcels["features"]), 16)
        for f in self.parcels["features"]:
            x0, y0, x1, y1 = box(f)
            self.assertEqual((x1 - x0, y1 - y0), (125.0, 125.0))

    def test_expected_answer_by_hand(self):
        """7 parcelles U à 500 m au plus d'une gare ; la parcelle U du coin nord-ouest exclue par
        la distance ; des parcelles N à 500 m au plus exclues par la zone."""
        zones = [(box(f), f["properties"]["typezone"]) for f in self.zoning["features"]]
        stations = [f["geometry"]["coordinates"] for f in self.stations["features"]]

        def zone_of(rect):
            cx, cy = (rect[0] + rect[2]) / 2, (rect[1] + rect[3]) / 2
            return next(z for (x0, y0, x1, y1), z in zones if x0 <= cx <= x1 and y0 <= cy <= y1)

        near_u, near_n = [], []
        for f in self.parcels["features"]:
            rect = box(f)
            near = min(rect_distance(rect, s) for s in stations) <= 500.0
            zone = zone_of(rect)
            if near and zone == "U":
                near_u.append(rect)
            elif near and zone == "N":
                near_n.append(rect)
        self.assertEqual(len(near_u), 7)
        self.assertNotIn((652000.0, 6850375.0, 652125.0, 6850500.0), near_u)
        self.assertTrue(near_n)


if __name__ == "__main__":
    unittest.main()
