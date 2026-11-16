"""Verdict de la démonstration de bout en bout (`demo_chaine_verdict.py`), sans QGIS."""

import os
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import demo_chaine_verdict as v  # noqa: E402
import demo_verdict  # noqa: E402

GRID = {"kind": "projected", "epsg": 32632, "interval": {"value": 1, "unit": "km"}}
CALLS = [
    {"command": "coord.convert", "params": {}},
    {"command": "data.transform_point", "params": {}},
    {"command": "terrain.viewshed_cumulative", "params": {}},
    {"command": "layout.export_pdf", "params": {"page_size": "a3", "grid": GRID}},
]
GOOD_GRIDS = [("EPSG:32632", 1000.0, 1000.0, True)]


def pdf(width=1190.55, height=841.89, pages=1):
    body = b"%%PDF-1.7\n1 0 obj << /Type /Pages /Count %d >> endobj\n" % pages
    for _ in range(pages):
        body += b"2 0 obj << /Type /Page /MediaBox [0 0 %.2f %.2f] >> endobj\n" % (width, height)
    return body


def steps(calls=CALLS):
    out = [{"step": i + 1, "command": c["command"], "result": {}} for i, c in enumerate(calls)]
    out[-1]["result"] = {"file": "x.pdf"}
    return out


class PlanTest(unittest.TestCase):
    def test_four_families_are_ok(self):
        self.assertIsNone(v.judge_plan(CALLS))

    def test_reproject_counts_as_reprojection(self):
        calls = [dict(c) for c in CALLS]
        calls[1]["command"] = "data.reproject"
        self.assertIsNone(v.judge_plan(calls))

    def test_missing_family_is_named_with_the_plan(self):
        for index, label in ((0, "conversion"), (1, "reprojection"), (2, "champ de vision"), (3, "export")):
            reason = v.judge_plan(CALLS[:index] + CALLS[index + 1:])
            self.assertIn(label, reason)
            self.assertIn("plan obtenu", reason)

    def test_out_of_order_is_refused(self):
        reason = v.judge_plan([CALLS[3], CALLS[0], CALLS[1], CALLS[2]])
        self.assertIn("hors de l'ordre", reason)

    def test_empty_plan(self):
        self.assertIn("conversion", v.judge_plan([]))


class RequestedTest(unittest.TestCase):
    def test_a3_and_one_km(self):
        self.assertIsNone(v.judge_requested(CALLS))

    def test_wrong_page_size(self):
        calls = [dict(c) for c in CALLS]
        calls[3] = {"command": "layout.export_pdf", "params": {"page_size": "a4", "grid": GRID}}
        self.assertIn("a4", v.judge_requested(calls))

    def test_no_grid(self):
        calls = CALLS[:3] + [{"command": "layout.export_pdf", "params": {"page_size": "a3"}}]
        self.assertIn("carroyage", v.judge_requested(calls))

    def test_thousand_metres_is_one_km(self):
        grid = dict(GRID, interval={"value": 1000, "unit": "m"})
        calls = CALLS[:3] + [{"command": "layout.export_pdf", "params": {"page_size": "a3", "grid": grid}}]
        self.assertIsNone(v.judge_requested(calls))

    def test_two_km_is_refused(self):
        for interval in ({"value": 2, "unit": "km"}, {"value": 2000, "unit": "m"}):
            grid = dict(GRID, interval=interval)
            calls = CALLS[:3] + [{"command": "layout.export_pdf", "params": {"page_size": "a3", "grid": grid}}]
            self.assertIn("1 km", v.judge_requested(calls))

    def test_ten_km_grid(self):
        grid = dict(GRID, interval={"value": 10, "unit": "km"})
        calls = CALLS[:3] + [{"command": "layout.export_pdf", "params": {"page_size": "a3", "grid": grid}}]
        self.assertIn("1 km", v.judge_requested(calls))


class PdfTest(unittest.TestCase):
    def test_a3_landscape_with_grid_is_ok(self):
        self.assertIsNone(v.judge_pdf(pdf(), GOOD_GRIDS))

    def test_a3_portrait_is_ok(self):
        self.assertIsNone(v.judge_pdf(pdf(841.89, 1190.55), GOOD_GRIDS))

    def test_a4_is_named(self):
        reason = v.judge_pdf(pdf(841.89, 595.28), GOOD_GRIDS)
        self.assertIn("hors A3", reason)

    def test_unreadable(self):
        self.assertIn("illisible", v.judge_pdf(None, GOOD_GRIDS))
        self.assertIn("illisible", v.judge_pdf(b"<html>", GOOD_GRIDS))

    def test_no_media_box(self):
        self.assertIn("boîte", v.judge_pdf(b"%PDF-1.7\n", GOOD_GRIDS))

    def test_two_pages(self):
        self.assertIn("2 page", v.judge_pdf(pdf(pages=2), GOOD_GRIDS))

    def test_without_grid(self):
        self.assertIn("sans carroyage", v.judge_pdf(pdf(), []))

    def test_grid_of_ten_km_or_disabled_or_not_utm(self):
        for grids in ([("EPSG:32632", 10000.0, 10000.0, True)],
                      [("EPSG:32632", 1000.0, 1000.0, False)],
                      [("EPSG:2154", 1000.0, 1000.0, True)]):
            self.assertIn("1 km", v.judge_pdf(pdf(), grids))

    def test_utm_31_is_accepted(self):
        self.assertIsNone(v.judge_pdf(pdf(), [("EPSG:32631", 1000.0, 1000.0, True)]))


class StepsTest(unittest.TestCase):
    def test_all_steps_with_pdf(self):
        self.assertIsNone(v.judge_steps(CALLS, steps()))

    def test_missing_step(self):
        self.assertIn("étape(s)", v.judge_steps(CALLS, steps()[:2]))

    def test_no_export_file(self):
        bare = [{"step": i + 1, "command": c["command"], "result": {}} for i, c in enumerate(CALLS)]
        self.assertIn("aucun PDF", v.judge_steps(CALLS, bare))


class ReplayTest(unittest.TestCase):
    def test_identical_replay_with_all_steps(self):
        self.assertIsNone(demo_verdict.judge_replay(
            demo_verdict.REPLAY_DONE, {"identical": True, "steps": 4}, steps()))

    def test_fewer_steps_replayed_fails(self):
        self.assertIn("rejeu", demo_verdict.judge_replay(
            demo_verdict.REPLAY_DONE, {"identical": True, "steps": 3}, steps()))


class FirstReasonTest(unittest.TestCase):
    def test_first_non_null(self):
        self.assertIsNone(v.first_reason(None, None))
        self.assertEqual(v.first_reason(None, "a", "b"), "a")


if __name__ == "__main__":
    unittest.main()
