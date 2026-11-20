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

    def load_raster(self, path, name):
        self.loaded.append((path, name, "gdal"))
        if self.valid and os.path.exists(path):
            with open(path, "rb") as f:
                f.read()
        return FakeLayer(self.valid, self.crs, name)

    def load(self, path, name, provider, csv=None):
        self.loaded.append((path, name, provider))
        self.csv = csv
        if csv is not None and "crs" in csv:
            self.crs = csv["crs"]  # le SCR est dans l'URI du fournisseur
        if self.valid and os.path.exists(path):
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

    def start(self, path, crs=None, **extra):
        params = {"path": path} if crs is None else {"path": path, "crs": crs}
        params.update(extra)
        self.gesture.start(params, lambda result=None, error=None: self.calls.append((result, error)))

    def pump(self):
        while self.queue:
            self.queue.pop(0)()


def run_params(backend, params):
    h = Harness(backend)
    h.gesture.start(params, lambda result=None, error=None: h.calls.append((result, error)))
    h.pump()
    assert len(h.calls) == 1
    return h.calls[0]


def run(backend, path, crs=None, **extra):
    h = Harness(backend)
    h.start(path, crs, **extra)
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
                "kind": "vector",
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
            def load(self, path, name, provider, csv=None):
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
        h.gesture.start({"path": ""}, lambda result=None, error=None: h.calls.append((result, error)))
        h.pump()
        self.assertEqual(h.calls[0][1].code, "layer.invalid_source")

    def test_csv_opens_with_default_columns_and_crs(self):
        with tempfile.TemporaryDirectory() as d:
            p = os.path.join(d, "gares.csv")
            with open(p, "wb") as f:
                f.write(b"X,Y,name\n656903.3,6845516.7,Vigneux\n")
            before = os.stat(p)
            b = FakeBackend(project_crs="EPSG:2154")
            result, error = run(b, p, "EPSG:2154")
            self.assertIsNone(error)
            self.assertEqual(b.loaded[0][2], "delimitedtext")
            self.assertEqual(b.csv, {"x": "X", "y": "Y", "crs": "EPSG:2154"})
            self.assertEqual(result["source_crs"], "EPSG:2154")
            self.assertFalse(result["reprojected"])
            after = os.stat(p)
            self.assertEqual((before.st_size, before.st_mtime_ns), (after.st_size, after.st_mtime_ns))

    def test_csv_custom_columns(self):
        b = FakeBackend()
        result, error = run(b, "villes.csv", "EPSG:4326", x_field="lon", y_field="lat")
        self.assertIsNone(error)
        self.assertEqual(b.csv, {"x": "lon", "y": "lat", "crs": "EPSG:4326"})

    def test_csv_without_crs_is_refused_before_opening(self):
        b = FakeBackend()
        result, error = run(b, "gares.csv")
        self.assertEqual(error.code, "layer.unknown_crs")
        self.assertEqual(b.loaded, [])

    def test_csv_bad_crs_or_column(self):
        _, error = run(FakeBackend(), "g.csv", "2154")
        self.assertEqual(error.code, "layer.invalid_crs")
        b = FakeBackend()
        _, error = run(b, "g.csv", "EPSG:2154", x_field="  ")
        self.assertEqual(error.code, "layer.invalid_column")
        self.assertEqual(b.loaded, [])

    def test_a_table_opens_without_geometry_and_without_crs(self):
        with tempfile.TemporaryDirectory() as d:
            p = os.path.join(d, "prix.csv")
            with open(p, "wb") as f:
                f.write(b"code,prix\n01001,100\n")
            before = os.stat(p)
            b = FakeBackend(crs="", project_crs="EPSG:2154")
            result, error = run_params(b, {"table": p})
            self.assertIsNone(error)
            self.assertEqual(b.loaded[0][2], "delimitedtext")
            self.assertEqual(b.csv, {"table": True})
            self.assertEqual(
                result,
                {"kind": "vector", "layer_id": "layer_1", "name": "prix", "source_crs": "", "project_crs": "EPSG:2154", "reprojected": False},
            )
            self.assertEqual(len(b.added), 1)
            after = os.stat(p)
            self.assertEqual((before.st_size, before.st_mtime_ns), (after.st_size, after.st_mtime_ns))

    def test_path_and_table_together_or_neither_are_refused(self):
        for params in ({"path": "a.gpkg", "table": "t.csv"}, {}):
            b = FakeBackend()
            _, error = run_params(b, params)
            self.assertEqual(error.code, "layer.one_input", params)
            self.assertEqual((b.loaded, b.added), ([], []))

    def test_a_table_takes_no_coordinate_option_and_only_csv(self):
        for extra in ({"crs": "EPSG:2154"}, {"x_field": "X"}, {"y_field": "Y"}):
            b = FakeBackend(crs="")
            _, error = run_params(b, dict(extra, table="t.csv"))
            self.assertEqual(error.code, "layer.one_input", extra)
            self.assertEqual(b.loaded, [])
        b = FakeBackend(crs="")
        _, error = run_params(b, {"table": "t.xlsx"})
        self.assertEqual(error.code, "layer.unsupported_format")
        self.assertEqual(b.loaded, [])

    def test_a_geotiff_opens_as_a_raster_layer(self):
        b = FakeBackend(crs="EPSG:2154")
        result, error = run_params(b, {"raster": "dem/mnt-colline-2154.tif"})
        self.assertIsNone(error)
        self.assertEqual(
            result,
            {
                "kind": "raster",
                "layer_id": "layer_1",
                "name": "mnt-colline-2154",
                "source_crs": "EPSG:2154",
                "project_crs": "EPSG:2154",
                "reprojected": False,
            },
        )
        self.assertEqual(b.loaded[0][2], "gdal")
        _, error = run_params(FakeBackend(crs="EPSG:2154"), {"raster": "a/MNT.TIFF"})
        self.assertIsNone(error)

    def test_a_raster_without_crs_needs_one_and_a_given_one_is_assigned(self):
        _, error = run_params(FakeBackend(crs=""), {"raster": "mnt.tif"})
        self.assertEqual(error.code, "layer.unknown_crs")
        b = FakeBackend(crs="")
        result, error = run_params(b, {"raster": "mnt.tif", "crs": "EPSG:2154"})
        self.assertIsNone(error)
        self.assertEqual(result["source_crs"], "EPSG:2154")
        self.assertEqual(len(b.added), 1)

    def test_a_raster_crs_contradicting_the_file_is_refused(self):
        _, error = run_params(FakeBackend(crs="EPSG:2154"), {"raster": "mnt.tif", "crs": "EPSG:4326"})
        self.assertEqual(error.code, "layer.crs_conflict")

    def test_a_raster_refuses_other_formats_and_coordinate_columns(self):
        b = FakeBackend(crs="EPSG:2154")
        for name in ("mnt.asc", "mnt.vrt", "mnt"):
            _, error = run_params(b, {"raster": name})
            self.assertEqual(error.code, "layer.unsupported_format", name)
        for key in ("x_field", "y_field"):
            _, error = run_params(b, {"raster": "mnt.tif", key: "X"})
            self.assertEqual(error.code, "layer.one_input", key)
        self.assertEqual(b.added, [])
        _, error = run_params(FakeBackend(crs="EPSG:2154", valid=False), {"raster": "mnt.tif"})
        self.assertEqual(error.code, "layer.invalid_source")

    def test_zero_two_or_three_inputs_are_refused(self):
        for params in (
            {},
            {"path": "a.gpkg", "raster": "m.tif"},
            {"table": "t.csv", "raster": "m.tif"},
            {"path": "a.gpkg", "table": "t.csv", "raster": "m.tif"},
        ):
            _, error = run_params(FakeBackend(), params)
            self.assertEqual(error.code, "layer.one_input", params)

    def test_parquet_goes_through_ogr(self):
        b = FakeBackend(crs="EPSG:4326")
        with tempfile.TemporaryDirectory() as d:
            p = os.path.join(d, "villes.parquet")
            with open(p, "wb") as f:
                f.write(b"PAR1")
            result, error = run(b, p)
        self.assertIsNone(error)
        self.assertEqual(b.loaded[0][2], "ogr")
        self.assertIsNone(b.csv)
        self.assertEqual(result["source_crs"], "EPSG:4326")

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
