"""Tests sans QGIS du geste layer.add : le fond QGIS est une doublure."""

import json
import os
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "cassini_qgis"))
FIXTURE = os.path.join(HERE, "..", "..", "fixtures", "parcelles-test.geojson")

import gestures  # noqa: E402
import layer_add  # noqa: E402


class FakeLayer:
    def __init__(self, valid, crs, name):
        self.valid, self.crs, self.name = valid, crs, name


class FakeBackend:
    """Lit le fichier en lecture seule, comme le ferait QGIS."""

    def __init__(self, crs="EPSG:4326", project_crs="EPSG:2154", valid=True):
        self.crs, self.project_crs_value, self.valid = crs, project_crs, valid
        self.added = []
        self.loaded = []

    def load(self, path, name, provider):
        self.loaded.append((path, name, provider))
        if self.valid:
            with open(path, "rb") as f:
                f.read()
        return FakeLayer(self.valid, self.crs, name)

    def is_valid(self, layer):
        return layer.valid

    def crs_of(self, layer):
        return layer.crs

    def project_crs(self):
        return self.project_crs_value

    def assign_crs(self, layer, authid):
        if authid not in ("EPSG:2154", "EPSG:4326"):
            return ""
        layer.crs = authid
        return authid

    def add(self, layer):
        self.added.append(layer)
        return "layer_%d" % len(self.added)

    def name_of(self, layer):
        return layer.name


class Harness:
    def __init__(self, backend):
        self.queue = []
        self.calls = []
        self.gesture = layer_add.LayerAddGesture(backend, self.queue.append)

    def start(self, path, crs=None):
        params = {"path": path} if crs is None else {"path": path, "crs": crs}
        self.gesture.start(params, lambda result=None, error=None: self.calls.append((result, error)))

    def pump(self):
        while self.queue:
            self.queue.pop(0)()


def run(backend, path, crs=None):
    h = Harness(backend)
    h.start(path, crs)
    assert h.calls == []  # asynchrone : rien avant l'ordonnanceur
    h.pump()
    assert len(h.calls) == 1
    return h.calls[0]


class LayerAddTest(unittest.TestCase):
    def test_geojson_reprojected(self):
        b = FakeBackend()
        result, error = run(b, FIXTURE)
        self.assertIsNone(error)
        self.assertEqual(
            result,
            {
                "layer_id": "layer_1",
                "name": "parcelles-test",
                "source_crs": "EPSG:4326",
                "project_crs": "EPSG:2154",
                "reprojected": True,
            },
        )
        self.assertEqual(len(b.added), 1)

    def test_same_crs_not_reprojected(self):
        result, _ = run(FakeBackend(crs="EPSG:2154"), FIXTURE)
        self.assertFalse(result["reprojected"])

    def test_gpkg_and_shp(self):
        with tempfile.TemporaryDirectory() as d:
            for ext in (".gpkg", ".shp", ".GEOJSON"):
                p = os.path.join(d, "x" + ext)
                with open(p, "wb") as f:
                    f.write(b"x")
                b = FakeBackend()
                result, error = run(b, p)
                self.assertIsNone(error, ext)
                self.assertEqual(result["name"], "x")
                self.assertEqual(b.loaded[0][2], "ogr")

    def test_file_without_crs_is_refused(self):
        b = FakeBackend(crs="")
        result, error = run(b, FIXTURE)
        self.assertIsNone(result)
        self.assertEqual(error.code, "layer.unknown_crs")
        self.assertIn("crs", error.message)
        self.assertEqual(b.added, [])

    def test_crs_given_when_the_file_has_none(self):
        b = FakeBackend(crs="")
        result, error = run(b, FIXTURE, "EPSG:2154")
        self.assertIsNone(error)
        self.assertEqual(result["source_crs"], "EPSG:2154")
        self.assertFalse(result["reprojected"])
        self.assertEqual(len(b.added), 1)

    def test_crs_that_contradicts_the_file(self):
        b = FakeBackend(crs="EPSG:4326")
        result, error = run(b, FIXTURE, "EPSG:2154")
        self.assertEqual(error.code, "layer.crs_conflict")
        self.assertEqual(b.added, [])

    def test_crs_same_as_the_file_is_accepted(self):
        result, error = run(FakeBackend(crs="EPSG:4326"), FIXTURE, "EPSG:4326")
        self.assertIsNone(error)
        self.assertEqual(result["source_crs"], "EPSG:4326")

    def test_malformed_or_unknown_crs(self):
        for bad in ("2154", "epsg:2154", "EPSG:", "EPSG:999999"):
            result, error = run(FakeBackend(crs=""), FIXTURE, bad)
            self.assertEqual(error.code, "layer.invalid_crs", bad)

    def test_unexpected_error_still_answers(self):
        class Boom(FakeBackend):
            def load(self, path, name, provider):
                raise RuntimeError("c:/secret/chemin")

        result, error = run(Boom(), FIXTURE)
        self.assertIsNone(result)
        self.assertEqual(error.code, "qgis.gesture_failed")
        self.assertNotIn("secret", error.message)

    def test_unsupported_format(self):
        b = FakeBackend()
        result, error = run(b, "dossier/notes.txt")
        self.assertIsNone(result)
        self.assertEqual(error.code, "layer.unsupported_format")
        self.assertEqual(b.loaded, [])
        self.assertEqual(b.added, [])

    def test_invalid_source_without_absolute_path(self):
        b = FakeBackend(valid=False)
        path = os.path.join(tempfile.gettempdir(), "secret-dir", "cassé.gpkg")
        result, error = run(b, path)
        self.assertEqual(error.code, "layer.invalid_source")
        self.assertNotIn("secret-dir", error.message)
        self.assertNotIn(tempfile.gettempdir(), error.message)
        self.assertEqual(b.added, [])

    def test_missing_path(self):
        h = Harness(FakeBackend())
        h.gesture.start({}, lambda result=None, error=None: h.calls.append((result, error)))
        h.pump()
        self.assertEqual(h.calls[0][1].code, "layer.invalid_source")

    def test_file_untouched(self):
        before = os.stat(FIXTURE)
        run(FakeBackend(), FIXTURE)
        after = os.stat(FIXTURE)
        self.assertEqual((before.st_size, before.st_mtime_ns), (after.st_size, after.st_mtime_ns))

    def test_cancel_before_end(self):
        b = FakeBackend()
        h = Harness(b)
        h.start(FIXTURE)
        h.gesture.cancel()
        h.pump()
        self.assertEqual(len(h.calls), 1)  # done rappelé une seule fois
        self.assertIsInstance(h.calls[0][1], gestures.GestureError)
        self.assertEqual(b.added, [])
        self.assertEqual(b.loaded, [])

    def test_fixture_is_valid_geojson(self):
        with open(FIXTURE, encoding="utf-8") as f:
            self.assertEqual(json.load(f)["type"], "FeatureCollection")


if __name__ == "__main__":
    unittest.main()
