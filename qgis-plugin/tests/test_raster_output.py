"""Tests sans QGIS du geste `layer.open_raster` (ticket 4.9) : fond en doublure."""

import os
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "cassini_qgis"))

import gestures  # noqa: E402
import raster_output  # noqa: E402


class FakeBackend:
    """Un projet en mémoire : (id, chemin, nom) ; `bad` : fichiers que GDAL ne lit pas."""

    def __init__(self, bad=()):
        self.layers = []
        self.bad = set(bad)

    def add_raster(self, path, name):
        if path in self.bad:
            return None
        layer_id = "raster_%d" % (len(self.layers) + 1)
        self.layers.append((layer_id, path, name))
        return layer_id


def run(gesture, params):
    """Lance le geste avec un ordonnanceur immédiat ; rend (résultat, erreur)."""
    got = {}
    gesture.start(params, lambda result=None, error=None: got.update(result=result, error=error))
    return got.get("result"), got.get("error")


class LayerOpenRasterTest(unittest.TestCase):
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

    def gesture(self, backend):
        return raster_output.LayerOpenRasterGesture(backend, schedule=lambda fn: fn())

    def test_the_raster_is_added_named_by_its_stem(self):
        backend = FakeBackend()
        result, error = run(self.gesture(backend), {"path": self.slope})
        self.assertIsNone(error)
        self.assertEqual(result, {"layer_id": "raster_1", "name": "1-terrain.slope"})
        self.assertEqual(backend.layers, [("raster_1", self.slope, "1-terrain.slope")])

    def test_an_open_file_gets_a_new_layer_named_by_its_stem(self):
        # Second rejeu dans la même session : `replay/` vidé puis réécrit au même chemin. La
        # couche du rejeu précédent (laissée hors de la carte) ne doit pas être rendue, ni son
        # nom courant (renommée à la main).
        backend = FakeBackend()
        run(self.gesture(backend), {"path": self.slope})
        backend.layers[0] = ("raster_1", self.slope, "renommée")
        result, error = run(self.gesture(backend), {"path": self.slope})
        self.assertIsNone(error)
        self.assertEqual(result, {"layer_id": "raster_2", "name": "1-terrain.slope"})
        self.assertEqual(len(backend.layers), 2)

    def test_a_missing_or_unreadable_raster_is_a_named_error_without_path(self):
        missing = os.path.join(self.dir, "9-terrain.slope.tif")
        cases = [
            (missing, FakeBackend(), "9-terrain.slope.tif"),
            (self.slope, FakeBackend(bad={self.slope}), "1-terrain.slope.tif"),
        ]
        for path, backend, name in cases:
            with self.subTest(path=path):
                result, error = run(self.gesture(backend), {"path": path})
                self.assertIsNone(result)
                self.assertIsInstance(error, gestures.GestureError)
                self.assertEqual(error.code, "layer.invalid_source")
                self.assertIn(name, error.message)
                self.assertNotIn(self.dir, error.message)
                self.assertEqual(backend.layers, [])
        for params in ({}, {"path": ""}, {"path": 3}, None):
            with self.subTest(params=params):
                _, error = run(self.gesture(FakeBackend()), params)
                self.assertEqual(error.code, "layer.invalid_source")

    def test_an_exploding_backend_still_answers_the_order(self):
        class Exploding(FakeBackend):
            def add_raster(self, path, name):
                raise RuntimeError("boom")

        result, error = run(self.gesture(Exploding()), {"path": self.slope})
        self.assertIsNone(result)
        self.assertEqual(error.code, "qgis.gesture_failed")

    def test_cancel_answers_once(self):
        calls = []
        pending = []
        gesture = raster_output.LayerOpenRasterGesture(FakeBackend(), schedule=pending.append)
        gesture.start({"path": self.slope}, lambda result=None, error=None: calls.append(error))
        gesture.cancel()
        pending[0]()  # le travail planifié arrive après l'annulation : rien de plus
        self.assertEqual(len(calls), 1)
        self.assertEqual(calls[0].code, "qgis.cancelled")


if __name__ == "__main__":
    unittest.main()
