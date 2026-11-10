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
