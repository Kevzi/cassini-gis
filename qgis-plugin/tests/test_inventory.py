"""Tests sans QGIS des gestes project.inventory et layer.describe : fond en doublure."""

import os
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "cassini_qgis"))

import inventory  # noqa: E402


class FakeLayer:
    def __init__(
        self, lid, name, source, crs, box, vector=True, fields=(), count=0, geom="Polygon",
        kind=None, wkb="MultiPolygon", values=None,
    ):
        self.lid, self.name, self.source, self.crs, self.box = lid, name, source, crs, box
        self.vector, self.fields, self.count, self.geom = vector, list(fields), count, geom
        self.kind = kind or ("vector" if vector else "raster")
        self.wkb = wkb
        self.values = dict(values or {})  # champ texte -> valeurs distinctes brutes (None, "" compris)


class FakeBackend:
    """Rend les couches dans l'ordre donné ; l'étendue est déjà « reprojetée »."""

    def __init__(self, layers=(), project_crs="EPSG:2154", project_name="projet", project_dir=""):
        self.items = list(layers)
        self.project_dir_value = project_dir
        self.project_crs_value = project_crs
        self.project_name_value = project_name
        self.value_reads = []  # (champ, plafond) de chaque lecture de valeurs

    def project_crs(self):
        return self.project_crs_value

    def project_name(self):
        return self.project_name_value

    def project_dir(self):
        return self.project_dir_value

    def kind_of(self, layer):
        return layer.kind

    def wkb_name(self, layer):
        return layer.wkb

    def text_values(self, layer, name, limit):
        self.value_reads.append((name, limit))
        if name not in layer.values:
            return None
        # Comme `uniqueValues(idx, limit + 2)` : des valeurs distinctes, au plus limit + 2.
        found = set(list(dict.fromkeys(layer.values[name]))[: limit + 2])
        return inventory._distinct_texts(found, limit)

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

    def test_vector_layer_reports_its_raw_qgis_facts(self):
        plu = FakeLayer(
            "plu_1", "Zonage du PLU", "plu.gpkg", "EPSG:2154", (0, 0, 1, 1),
            fields=[("typezone", "String"), ("surface", "Real")], count=3,
            values={"typezone": ["U", "N", "Nh"]},
        )
        backend = FakeBackend([plu], project_name="projet-test-villeneuve")
        result, error = run(inventory.ProjectInventoryGesture, backend, {})
        self.assertIsNone(error)
        self.assertEqual(result["project_name"], "projet-test-villeneuve")
        layer = result["layers"][0]
        self.assertEqual(layer["kind"], "vector")
        self.assertEqual(layer["geometry"], "MultiPolygon")
        self.assertEqual(layer["feature_count"], 3)
        self.assertEqual(
            layer["fields"],
            [
                {"name": "typezone", "type": "String", "values": ["N", "Nh", "U"]},
                {"name": "surface", "type": "Real"},
            ],
        )
        # Les clés d'avant restent, avec le même sens.
        for key in ("layer_id", "name", "source", "crs"):
            self.assertIn(key, layer)
        self.assertIn("extent", result)
        self.assertEqual(result["project_crs"], "EPSG:2154")

    def test_values_are_read_with_a_ceiling(self):
        many = ["P%04d" % i for i in range(5000)]
        parcelles = FakeLayer(
            "p", "Parcelles", "p.gpkg", "EPSG:2154", None,
            fields=[("id_parcelle", "String")], values={"id_parcelle": many},
        )
        backend = FakeBackend([parcelles])
        result, _ = run(inventory.ProjectInventoryGesture, backend, {})
        self.assertEqual(backend.value_reads, [("id_parcelle", inventory.VALUES_LIMIT)])
        self.assertEqual(len(result["layers"][0]["fields"][0]["values"]), inventory.VALUES_LIMIT)
        self.assertEqual(inventory.VALUES_LIMIT, 11)  # une de plus que le plafond du noyau

    def test_raster_has_no_geometry_no_fields_no_reads(self):
        mnt = FakeLayer("r", "MNT", "mnt.tif", "EPSG:2154", (0, 0, 1, 1), vector=False,
                        fields=[("x", "String")], count=9)
        backend = FakeBackend([mnt])
        result, _ = run(inventory.ProjectInventoryGesture, backend, {})
        layer = result["layers"][0]
        self.assertEqual(layer["kind"], "raster")
        self.assertIsNone(layer["geometry"])
        self.assertEqual(layer["fields"], [])
        self.assertNotIn("feature_count", layer)
        self.assertEqual(backend.value_reads, [])

    def test_a_raster_source_is_relative_to_the_project_file(self):
        # Les quatre lignes d'inventaire de la matrice du ticket 4.3, Windows et POSIX.
        cases = [
            ("/home/k/proj/dem/mnt.tif", "/home/k/proj", "dem/mnt.tif"),
            ("C:/Users/k/proj/dem/mnt.tif", "C:/Users/k/proj", "dem/mnt.tif"),
            (r"C:\Users\k\proj\dem\mnt.tif", "C:/Users/k/proj", "dem/mnt.tif"),
            ("/home/k/donnees/mnt.tif", "/home/k/proj", "../donnees/mnt.tif"),
            (r"C:\Users\k\donnees\mnt.tif", "C:/Users/k/proj", "../donnees/mnt.tif"),
            (r"c:\users\k\proj\dem\mnt.tif", "C:/Users/k/proj", "dem/mnt.tif"),
            ("/vsizip//home/k/proj/dem.zip/mnt.tif", "/home/k/proj", "mnt.tif"),
            ("/vsigzip//home/k/proj/dem/mnt.tif.gz", "/home/k/proj", "mnt.tif.gz"),
            (r"D:\mnt.tif", "C:/Users/k/proj", "mnt.tif"),
            ("/home/k/proj/dem/mnt.tif", "", "mnt.tif"),
            ("/home/k/proj/dem/mnt.tif", None, "mnt.tif"),
            ("/home/k/proj/dem/mnt.tif", "C:/Users/k/proj", "mnt.tif"),
            ("file:///C:/Users/k/proj/dem/mnt.tif", "C:/Users/k/proj", "dem/mnt.tif"),
            ("dem/mnt.tif", "/home/k/proj", "mnt.tif"),
            ("/home/k/proj/p.cassini/1-terrain.slope.tif", "/home/k/proj", "p.cassini/1-terrain.slope.tif"),
        ]
        for raw, project_dir, expected in cases:
            self.assertEqual(inventory.raster_source(raw, project_dir), expected, (raw, project_dir))
        self.assertIsNone(inventory.raster_source("type=xyz&url=https://a/b/c.png", "/home/k/proj"))
        self.assertIsNone(inventory.raster_source(None, "/home/k/proj"))
        # Flux distant : jamais nommé (comme `source_name`), jamais un faux chemin relatif.
        self.assertIsNone(inventory.raster_source("/vsicurl/https://h/mnt.tif", "/home/k/proj"))

    def test_the_inventory_gives_rasters_a_relative_path_and_vectors_a_name(self):
        layers = [
            FakeLayer("r", "MNT", "/home/k/proj/dem/mnt.tif", "EPSG:2154", None, vector=False),
            FakeLayer("s", "Voisin", "/home/k/donnees/mnt.tif", "EPSG:2154", None, vector=False),
            FakeLayer("v", "PLU", "/home/k/proj/data/plu.gpkg|layername=zone", "EPSG:2154", None),
        ]
        result, _ = run(inventory.ProjectInventoryGesture, FakeBackend(layers, project_dir="/home/k/proj"), {})
        self.assertEqual(
            [l["source"] for l in result["layers"]], ["dem/mnt.tif", "../donnees/mnt.tif", "plu.gpkg"]
        )
        self.assertNotIn("/home", repr(result))
        windows = [FakeLayer("w", "MNT", r"D:\mnt.tif", "EPSG:2154", None, vector=False)]
        result, _ = run(inventory.ProjectInventoryGesture, FakeBackend(windows, project_dir="C:/proj"), {})
        self.assertEqual(result["layers"][0]["source"], "mnt.tif")

    def test_a_failed_value_read_keeps_the_field_without_values(self):
        class Flaky(FakeBackend):
            def text_values(self, layer, name, limit):
                raise RuntimeError("boom")

        layer = FakeLayer("a", "a", "a.gpkg", "EPSG:2154", None, fields=[("nom", "String")])
        result, error = run(inventory.ProjectInventoryGesture, Flaky([layer]), {})
        self.assertIsNone(error)
        self.assertEqual(result["layers"][0]["fields"], [{"name": "nom", "type": "String"}])

    def test_null_and_empty_text_do_not_hide_an_overflow(self):
        found = {None, ""} | {"V%02d" % i for i in range(11)}
        texts = inventory._distinct_texts(found, inventory.VALUES_LIMIT)
        self.assertEqual(texts, ["V%02d" % i for i in range(11)])  # le noyau voit le trop-plein

    def test_a_boolean_only_field_has_no_values(self):
        self.assertIsNone(inventory._distinct_texts({True, False, None}, inventory.VALUES_LIMIT))
        layer = FakeLayer("a", "a", "a.gpkg", "EPSG:2154", None, fields=[("ok", "Boolean")],
                          values={"ok": [True, False, None]})
        result, _ = run(inventory.ProjectInventoryGesture, FakeBackend([layer]), {})
        self.assertEqual(result["layers"][0]["fields"], [{"name": "ok", "type": "Boolean"}])

    def test_unsaved_project_has_no_name(self):
        result, _ = run(inventory.ProjectInventoryGesture, FakeBackend(project_name=""), {})
        self.assertIsNone(result["project_name"])

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
