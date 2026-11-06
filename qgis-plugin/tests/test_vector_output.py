"""Tests sans QGIS du geste `layer.open_vector` (AD-30, ticket 11.8) : fond en doublure."""

import os
import shutil
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "cassini_qgis"))
FIXTURE = os.path.join(HERE, "..", "..", "fixtures", "points-2154.geojson")

import gestures  # noqa: E402
import vector_output  # noqa: E402


class FakeBackend:
    """Un projet en mémoire : (id, chemin, nom). `bad` : fichiers qu'OGR ne lit pas ; `read_as` :
    le système que QGIS lit (`EPSG:2154` par défaut ; OGR prend WGS84 en silence pour un code
    qu'il ne connaît pas). Comme le vrai fond, un système autre que l'attendu n'ajoute rien."""

    def __init__(self, bad=(), read_as="EPSG:2154"):
        self.layers = []
        self.bad = set(bad)
        self.read_as = read_as
        self.expected = []

    def add_vector(self, path, name, expected_crs):
        self.expected.append(expected_crs)
        if path in self.bad or self.read_as != expected_crs:
            return None
        layer_id = "vector_%d" % (len(self.layers) + 1)
        self.layers.append((layer_id, path, name))
        return layer_id, self.read_as


def run(gesture, params):
    """Lance le geste avec un ordonnanceur immédiat ; rend (résultat, erreur)."""
    got = {}
    gesture.start(params, lambda result=None, error=None: got.update(result=result, error=error))
    return got.get("result"), got.get("error")


class LayerOpenVectorTest(unittest.TestCase):
    def setUp(self):
        self._dir = tempfile.TemporaryDirectory()
        self.dir = self._dir.name
        self.points = os.path.join(self.dir, "1-terrain.peaks.geojson")
        shutil.copyfile(FIXTURE, self.points)

    def tearDown(self):
        self._dir.cleanup()

    def gesture(self, backend):
        return vector_output.LayerOpenVectorGesture(backend, schedule=lambda fn: fn())

    def test_the_geojson_is_added_named_by_its_stem_with_its_crs(self):
        backend = FakeBackend()
        result, error = run(self.gesture(backend), {"path": self.points})
        self.assertIsNone(error)
        self.assertEqual(
            result, {"layer_id": "vector_1", "name": "1-terrain.peaks", "crs": "EPSG:2154"}
        )
        self.assertEqual(backend.layers, [("vector_1", self.points, "1-terrain.peaks")])
        self.assertEqual(backend.expected, ["EPSG:2154"])

    def test_an_open_file_gets_a_new_layer(self):
        backend = FakeBackend()
        run(self.gesture(backend), {"path": self.points})
        backend.layers[0] = ("vector_1", self.points, "renommée")
        result, error = run(self.gesture(backend), {"path": self.points})
        self.assertIsNone(error)
        self.assertEqual(result["layer_id"], "vector_2")
        self.assertEqual(result["name"], "1-terrain.peaks")
        self.assertEqual(len(backend.layers), 2)

    def _write(self, name, text):
        path = os.path.join(self.dir, name)
        with open(path, "w", encoding="utf-8") as f:
            f.write(text)
        return path

    def test_a_missing_unreadable_or_crs_less_file_is_a_named_error_without_path(self):
        missing = os.path.join(self.dir, "9-terrain.peaks.geojson")
        with open(FIXTURE, encoding="utf-8") as f:
            fixture = f.read()
        corrupt = self._write("8-terrain.peaks.geojson", '{"type":"FeatureCollection","features":[')
        undeclared = self._write(
            "7-terrain.peaks.geojson",
            fixture.replace(
                ',"crs":{"type":"name","properties":{"name":"urn:ogc:def:crs:EPSG::2154"}}', ""
            ),
        )
        unknown = self._write(
            "6-terrain.peaks.geojson", fixture.replace("EPSG::2154", "EPSG::999999")
        )
        cases = [
            (missing, FakeBackend(), "9-terrain.peaks.geojson"),
            (self.points, FakeBackend(bad={self.points}), "1-terrain.peaks.geojson"),
            (corrupt, FakeBackend(), "8-terrain.peaks.geojson"),
            (undeclared, FakeBackend(), "7-terrain.peaks.geojson"),
            # OGR lit WGS84 pour un code qu'il ne connaît pas : refusé, rien d'ajouté.
            (unknown, FakeBackend(read_as="EPSG:4326"), "6-terrain.peaks.geojson"),
            (self.points, FakeBackend(read_as="EPSG:4326"), "1-terrain.peaks.geojson"),
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

    def test_an_empty_answer_from_the_backend_is_refused(self):
        class Empty(FakeBackend):
            def add_vector(self, path, name, expected_crs):
                return "vector_1", ""

        result, error = run(self.gesture(Empty()), {"path": self.points})
        self.assertIsNone(result)
        self.assertEqual(error.code, "layer.invalid_source")

    def test_an_exploding_backend_still_answers_the_order(self):
        class Exploding(FakeBackend):
            def add_vector(self, path, name, expected_crs):
                raise RuntimeError("boom")

        result, error = run(self.gesture(Exploding()), {"path": self.points})
        self.assertIsNone(result)
        self.assertEqual(error.code, "qgis.gesture_failed")

    def test_cancel_answers_once(self):
        calls = []
        pending = []
        gesture = vector_output.LayerOpenVectorGesture(FakeBackend(), schedule=pending.append)
        gesture.start({"path": self.points}, lambda result=None, error=None: calls.append(error))
        gesture.cancel()
        pending[0]()  # le travail planifié arrive après l'annulation : rien de plus
        self.assertEqual(len(calls), 1)
        self.assertEqual(calls[0].code, "qgis.cancelled")

    def test_the_declared_crs_is_read_from_the_2008_member(self):
        self.assertEqual(vector_output.declared_crs(self.points), "EPSG:2154")
        for text in (
            '{"type":"FeatureCollection","features":[]}',
            '{"crs":{"type":"name","properties":{"name":"EPSG:2154"}}}',
            '{"crs":{"type":"name","properties":{"name":"urn:ogc:def:crs:EPSG::0"}}}',
            '{"crs":{"type":"name","properties":{"name":"urn:ogc:def:crs:OGC:1.3:CRS84"}}}',
            "[1, 2]",
            "pas du JSON",
            '{"crs":{"type":"name","properties":{"name":"urn:ogc:def:crs:EPSG::2154\\n"}}}',
            "[" * 100000 + "]" * 100000,
        ):
            with self.subTest(text=text):
                path = self._write("x.geojson", text)
                self.assertIsNone(vector_output.declared_crs(path))
        self.assertIsNone(vector_output.declared_crs(os.path.join(self.dir, "absent.geojson")))

    def test_no_direct_pyqt_import(self):
        with open(vector_output.__file__, encoding="utf-8") as f:
            source = f.read()
        self.assertNotIn("import PyQt5", source)
        self.assertNotIn("import PyQt6", source)
        self.assertNotIn("from PyQt", source)


if __name__ == "__main__":
    unittest.main()
