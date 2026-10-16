"""Tests sans QGIS des gestes project.inventory et layer.describe : fond en doublure."""

import os
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "cassini_qgis"))

import inventory  # noqa: E402


class FakeLayer:
    def __init__(self, lid, name, source, crs, box, vector=True, fields=(), count=0, geom="Polygon"):
        self.lid, self.name, self.source, self.crs, self.box = lid, name, source, crs, box
        self.vector, self.fields, self.count, self.geom = vector, list(fields), count, geom


class FakeBackend:
    """Rend les couches dans l'ordre donné ; l'étendue est déjà « reprojetée »."""

    def __init__(self, layers=(), project_crs="EPSG:2154"):
        self.items = list(layers)
        self.project_crs_value = project_crs

    def project_crs(self):
        return self.project_crs_value

    def layers(self):
        return list(self.items)

    def layer(self, layer_id):
        return next((l for l in self.items if l.lid == layer_id), None)

    def id_of(self, layer):
        return layer.lid

    def name_of(self, layer):
        return layer.name

    def source_of(self, layer):
        return layer.source

    def crs_of(self, layer):
        return layer.crs

    def extent_of(self, layer, to_authid=None):
        return layer.box

    def is_vector(self, layer):
        return layer.vector

    def fields_of(self, layer):
        return layer.fields

    def feature_count(self, layer):
        return layer.count

    def geometry_of(self, layer):
        return layer.geom


def run(gesture_class, backend, params):
    queue, calls = [], []
    g = gesture_class(backend, queue.append)
    g.start(params, lambda result=None, error=None: calls.append((result, error)))
    assert calls == []  # asynchrone : rien avant l'ordonnanceur
    while queue:
        queue.pop(0)()
    assert len(calls) == 1
    return calls[0]


TWO = [
    FakeLayer("a_1", "gares", r"C:\Users\x\proj\gares.gpkg|layername=gares", "EPSG:2154", (0.0, 1.0, 10.0, 11.0)),
    FakeLayer("b_2", "villes", "/home/x/villes.shp", "EPSG:4326", (5.0, -2.0, 20.0, 8.0)),
]


class InventoryTest(unittest.TestCase):
    def test_two_layers_in_order_with_union_extent(self):
        result, error = run(inventory.ProjectInventoryGesture, FakeBackend(TWO), {})
        self.assertIsNone(error)
        self.assertEqual([l["layer_id"] for l in result["layers"]], ["a_1", "b_2"])
        self.assertEqual(result["project_crs"], "EPSG:2154")
        self.assertEqual(
            result["extent"],
            {"xmin": 0.0, "ymin": -2.0, "xmax": 20.0, "ymax": 11.0, "crs": "EPSG:2154"},
        )
        self.assertEqual(result["layers"][1]["crs"], "EPSG:4326")

    def test_no_absolute_path_in_the_result(self):
        result, _ = run(inventory.ProjectInventoryGesture, FakeBackend(TWO), {})
        self.assertEqual([l["source"] for l in result["layers"]], ["gares.gpkg", "villes.shp"])
        text = repr(result)
        self.assertNotIn("Users", text)
        self.assertNotIn("/home", text)

    def test_empty_project(self):
        result, error = run(inventory.ProjectInventoryGesture, FakeBackend(), {})
        self.assertIsNone(error)
        self.assertEqual(result["layers"], [])
        self.assertIsNone(result["extent"])
        self.assertEqual(result["project_crs"], "EPSG:2154")

    def test_empty_layers_do_not_widen_the_extent(self):
        layers = [FakeLayer("e", "vide", "vide.gpkg", "EPSG:2154", None), TWO[0]]
        result, _ = run(inventory.ProjectInventoryGesture, FakeBackend(layers), {})
        self.assertEqual(result["extent"]["xmax"], 10.0)

    def test_csv_uri_source_gives_the_file_name(self):
        self.assertEqual(inventory.source_name("file:///d/data/gares.csv?type=csv&xField=X"), "gares.csv")
        self.assertIsNone(inventory.source_name(""))
        for remote in ("dbname='x' host=h user=u password=p table=t", "Point?crs=epsg:4326", "type=xyz&url=https://a/b/c.png", "https://h/wms"):
            self.assertIsNone(inventory.source_name(remote))
        self.assertIsNone(inventory.source_name(None))

    def test_unexpected_failure_still_answers(self):
        class Broken(FakeBackend):
            def layers(self):
                raise RuntimeError("boom")

        result, error = run(inventory.ProjectInventoryGesture, Broken(), {})
        self.assertIsNone(result)
        self.assertEqual(error.code, "qgis.gesture_failed")

    def test_cancel_answers_once(self):
        calls = []
        g = inventory.ProjectInventoryGesture(FakeBackend(), lambda fn: None)
        g.start({}, lambda result=None, error=None: calls.append(error))
        g.cancel()
        g.cancel()
        self.assertEqual(len(calls), 1)
        self.assertEqual(calls[0].code, "qgis.cancelled")


class DescribeTest(unittest.TestCase):
    def test_vector_layer(self):
        layer = FakeLayer(
            "a_1", "gares", "gares.gpkg", "EPSG:2154", (0.0, 1.0, 10.0, 11.0),
            fields=[("nom", "String"), ("pop", "Integer64")], count=8, geom="Point",
        )
        result, error = run(inventory.LayerDescribeGesture, FakeBackend([layer]), {"layer_id": "a_1"})
        self.assertIsNone(error)
        self.assertEqual(
            result,
            {
                "name": "gares",
                "fields": [{"name": "nom", "type": "String"}, {"name": "pop", "type": "Integer64"}],
                "crs": "EPSG:2154",
                "extent": {"xmin": 0.0, "ymin": 1.0, "xmax": 10.0, "ymax": 11.0, "crs": "EPSG:2154"},
                "feature_count": 8,
                "geometry": "Point",
            },
        )

    def test_unknown_layer(self):
        result, error = run(inventory.LayerDescribeGesture, FakeBackend(TWO), {"layer_id": "zzz"})
        self.assertIsNone(result)
        self.assertEqual(error.code, "layer.not_found")
        self.assertNotIn("zzz", error.message)

    def test_missing_layer_id(self):
        _, error = run(inventory.LayerDescribeGesture, FakeBackend(TWO), {})
        self.assertEqual(error.code, "layer.not_found")

    def test_empty_layer_has_null_extent(self):
        layer = FakeLayer("e", "vide", "vide.gpkg", "EPSG:2154", None, geom="Point")
        result, error = run(inventory.LayerDescribeGesture, FakeBackend([layer]), {"layer_id": "e"})
        self.assertIsNone(error)
        self.assertIsNone(result["extent"])
        self.assertEqual(result["feature_count"], 0)

    def test_raster_is_refused(self):
        layer = FakeLayer("r", "mnt", "mnt.tif", "EPSG:2154", (0, 0, 1, 1), vector=False)
        _, error = run(inventory.LayerDescribeGesture, FakeBackend([layer]), {"layer_id": "r"})
        self.assertEqual(error.code, "layer.not_vector")


if __name__ == "__main__":
    unittest.main()
