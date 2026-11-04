"""Tests sans QGIS de l'ouverture des rasters du noyau (ticket 4.3) : fond en doublure."""

import os
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "cassini_qgis"))

import client  # noqa: E402
import raster_output  # noqa: E402
import runlog  # noqa: E402

CATALOGUE = [
    {"id": "terrain.slope", "runs_in": "core"},
    {"id": "system.about", "runs_in": "core"},
    {"id": "layout.export_pdf", "runs_in": "qgis"},
    {"id": "analysis.buffer", "runs_in": "qgis"},
]


class FakeBackend:
    """Un projet en mémoire : les chemins ouverts ; `bad` : fichiers que GDAL ne lit pas."""

    def __init__(self, bad=()):
        self.layers = []  # (chemin, nom)
        self.bad = set(bad)

    def is_open(self, path):
        return any(raster_output.same_file(p, path) for p, _ in self.layers)

    def add_raster(self, path, name):
        if path in self.bad:
            return False
        self.layers.append((path, name))
        return True


class RasterOutputTest(unittest.TestCase):
    def setUp(self):
        self._dir = tempfile.TemporaryDirectory()
        self.dir = self._dir.name
        self.slope = self._touch("1-terrain.slope.tif")

    def tearDown(self):
        self._dir.cleanup()

    def _touch(self, name):
        path = os.path.join(self.dir, name)
        with open(path, "wb") as f:
            f.write(b"II*\x00")
        return path

    def test_a_slope_run_by_hand_is_added_named_by_its_stem(self):
        payload = {"command": "terrain.slope", "result": {"file": self.slope, "unit": "degrees"}}
        items = raster_output.to_open(client.RUN_DONE, payload, CATALOGUE)
        self.assertEqual(items, [(self.slope, "1-terrain.slope")])
        backend = FakeBackend()
        outcomes = raster_output.open_rasters(items, backend)
        self.assertEqual(outcomes, [(self.slope, raster_output.ADDED)])
        self.assertEqual(backend.layers, [(self.slope, "1-terrain.slope")])

    def test_a_slope_in_a_plan_is_added_and_qgis_steps_are_not_reopened(self):
        slope = self._touch("2-terrain.slope.tif")
        buffer = self._touch("1-analysis.buffer.tif")  # même d'extension .tif : étape QGIS
        payload = {"steps": [
            {"step": 1, "command": "analysis.buffer", "result": {"file": buffer}},
            {"step": 2, "command": "terrain.slope", "result": {"file": slope}},
        ]}
        items = raster_output.to_open(client.PLAN_DONE, payload, CATALOGUE)
        self.assertEqual(items, [(slope, "2-terrain.slope")])

    def test_a_slope_from_the_external_ai_is_added(self):
        payload = {"call_id": "external-1", "command": "terrain.slope",
                   "result": {"file": self.slope}, "error": None, "duration_ms": 4}
        items = raster_output.to_open(client.EXTERNAL_CALL_FINISHED, payload, CATALOGUE)
        self.assertEqual(items, [(self.slope, "1-terrain.slope")])
        failed = dict(payload, error={"code": "x", "message": "y"})
        self.assertEqual(raster_output.to_open(client.EXTERNAL_CALL_FINISHED, failed, CATALOGUE), [])

    def test_a_non_raster_output_adds_nothing(self):
        pdf = self._touch("1-layout.export_pdf.pdf")
        cases = [
            ("layout.export_pdf", {"file": pdf}),
            ("terrain.slope", {"file": pdf}),
            ("system.about", {"protocol_version": 2}),
            ("terrain.slope", None),
            ("terrain.slope", {"file": 3}),
            ("terrain.slope", {"file": "1-terrain.slope.tif"}),  # relatif : jamais deviné
            ("inconnue", {"file": self.slope}),
        ]
        for command, result in cases:
            with self.subTest(command=command, result=result):
                self.assertEqual(raster_output.rasters_to_open(command, result, CATALOGUE), [])

    def test_a_missing_or_unreadable_file_adds_nothing_and_does_not_raise(self):
        missing = os.path.join(self.dir, "9-terrain.slope.tif")
        items = raster_output.to_open(
            client.RUN_DONE, {"command": "terrain.slope", "result": {"file": missing}}, CATALOGUE
        )
        self.assertEqual(items, [(missing, "9-terrain.slope")])
        backend = FakeBackend(bad={self.slope})
        outcomes = raster_output.open_rasters(items + [(self.slope, "1-terrain.slope")], backend)
        self.assertEqual(
            outcomes,
            [(missing, raster_output.UNREADABLE), (self.slope, raster_output.UNREADABLE)],
        )
        self.assertEqual(backend.layers, [])

        class Exploding(FakeBackend):
            def add_raster(self, path, name):
                raise RuntimeError("boom")

        outcomes = raster_output.open_rasters([(self.slope, "1-terrain.slope")], Exploding())
        self.assertEqual(outcomes, [(self.slope, raster_output.UNREADABLE)])

    def test_an_unopened_raster_leaves_an_error_line_in_the_panel_log(self):
        log = runlog.RunLog()
        missing = os.path.join(self.dir, "9-terrain.slope.tif")
        outcomes = [
            (self.slope, raster_output.ADDED),
            (missing, raster_output.UNREADABLE),
            (self.slope, raster_output.ALREADY_OPEN),
        ]
        entries = raster_output.report(log, outcomes, "fr", "14:02")
        self.assertEqual(log.entries, entries)
        self.assertEqual(len(entries), 1)
        entry = entries[0]
        self.assertEqual(entry.status, runlog.ERROR)
        self.assertEqual(entry.label, "Sortie raster non ouverte dans le projet : 9-terrain.slope.tif")
        self.assertNotIn(self.dir, entry.label)
        self.assertIsNone(log.current())  # jamais une ligne en cours
        english = raster_output.report(runlog.RunLog(), [(missing, raster_output.UNREADABLE)], "en", "")
        self.assertIn("9-terrain.slope.tif", english[0].label)
        windows = raster_output.report(
            runlog.RunLog(), [(r"C:\SIG\p.cassini\2-terrain.slope.tif", raster_output.UNREADABLE)], "fr", ""
        )
        self.assertTrue(windows[0].label.endswith(": 2-terrain.slope.tif"))

    def test_the_same_file_twice_gives_one_layer(self):
        backend = FakeBackend()
        items = [(self.slope, "1-terrain.slope")]
        raster_output.open_rasters(items, backend)
        outcomes = raster_output.open_rasters(items, backend)
        self.assertEqual(outcomes, [(self.slope, raster_output.ALREADY_OPEN)])
        self.assertEqual(len(backend.layers), 1)
        twice = {"steps": [
            {"step": 1, "command": "terrain.slope", "result": {"file": self.slope}},
            {"step": 2, "command": "terrain.slope", "result": {"file": self.slope}},
        ]}
        self.assertEqual(len(raster_output.to_open(client.PLAN_DONE, twice, CATALOGUE)), 1)

    def test_same_file_ignores_separators_and_provider_options(self):
        self.assertTrue(raster_output.same_file("/a/b/x.tif", "/a/b/./x.tif"))
        self.assertTrue(raster_output.same_file("/a/b/x.tif|option", "/a/b/x.tif"))
        self.assertFalse(raster_output.same_file("/a/b/x.tif", "/a/b/y.tif"))
        self.assertFalse(raster_output.same_file(None, "/a/b/x.tif"))


if __name__ == "__main__":
    unittest.main()
