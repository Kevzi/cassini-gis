"""Tests sans QGIS de l'exécuteur générique des analyses : fond en doublure."""

import os
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "cassini_qgis"))

import analysis  # noqa: E402
import form  # noqa: E402
import gestures  # noqa: E402


class FakeLayer:
    def __init__(self, lid, name="gares", crs="EPSG:2154", vector=True, metric=True, count=8, fields=("zone", "pop"), geometry="Point"):
        self.lid, self.name, self.crs, self.vector, self.metric, self.count = lid, name, crs, vector, metric, count
        self.fields = [(f, "String") for f in fields]
        self.geometry = geometry


class FakeHandle:
    def __init__(self):
        self.cancelled = False

    def cancel(self):
        self.cancelled = True


class FakeBackend:
    """`run_algorithm` garde l'appel ; le test appelle `finished` quand il veut."""

    def __init__(self, layers=(), write=True, loaded_count=8, loadable=True):
        self.items = {l.lid: l for l in layers}
        self.calls, self.handle, self.finished = [], FakeHandle(), None
        self.write, self.loaded_count, self.loadable = write, loaded_count, loadable
        self.pairs, self.counts_call = [], None

    def layer(self, layer_id):
        return self.items.get(layer_id)

    def is_vector(self, layer):
        return layer.vector

    def crs_is_metric(self, layer):
        return layer.metric

    def name_of(self, layer):
        return layer.name

    def crs_of(self, layer):
        return layer.crs

    def id_of(self, layer):
        return layer.lid

    def feature_count(self, layer):
        return layer.count

    def fields_of(self, layer):
        return layer.fields

    def geometry_of(self, layer):
        return layer.geometry

    def counts_of(self, layer, count_field, label_field, limit):
        self.counts_call = (count_field, label_field, limit)
        pairs = self.pairs[:limit]
        return pairs, len(self.pairs) > limit

    def run_algorithm(self, algorithm_id, parameters, finished):
        self.calls.append((algorithm_id, dict(parameters)))
        self.finished = finished
        if self.write:
            with open(parameters["OUTPUT"], "wb") as f:
                f.write(b"x")
        return self.handle

    def load_output(self, path, name):
        if not self.loadable:
            return None
        return FakeLayer("out_1", name, "EPSG:2154", count=self.loaded_count)


def start(entry, backend, params):
    queue, calls = [], []
    g = analysis.ProcessingGesture(entry, backend, queue.append)
    g.start(params, lambda result=None, error=None: calls.append((result, error)))
    assert calls == []  # asynchrone : rien avant l'ordonnanceur
    while queue:
        queue.pop(0)()
    return g, calls


class BufferTest(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.out = os.path.join(self.dir.name, "1-analysis.buffer.gpkg")
        self.entry = analysis.ALGORITHMS["analysis.buffer"]

    def tearDown(self):
        self.dir.cleanup()

    def params(self, **extra):
        p = {"layer_id": "g", "distance": {"value": 500, "unit": "m"}, "out": self.out}
        p.update(extra)
        return p

    def backend(self, **kw):
        layers = [FakeLayer("g"), FakeLayer("w", crs="EPSG:4326", metric=False), FakeLayer("r", vector=False)]
        return FakeBackend(layers, **kw)

    def test_buffer_of_points_in_meters(self):
        b = self.backend()
        g, calls = start(self.entry, b, self.params())
        algo, p = b.calls[0]
        self.assertEqual(algo, "native:buffer")
        self.assertEqual(p["DISTANCE"], 500.0)
        self.assertFalse(p["DISSOLVE"])
        self.assertEqual(p["OUTPUT"], self.out)  # exactement le fichier du noyau
        self.assertEqual(calls, [])
        b.finished(True, {})
        result, error = calls[0]
        self.assertIsNone(error)
        self.assertEqual(
            result, {"layer_id": "out_1", "name": "1-analysis.buffer", "feature_count": 8, "crs": "EPSG:2154"}
        )
        self.assertTrue(os.path.exists(self.out))

    def test_kilometers_equal_meters(self):
        b = self.backend()
        start(self.entry, b, self.params(distance={"value": 0.5, "unit": "km"}))
        self.assertEqual(b.calls[0][1]["DISTANCE"], 500.0)

    def test_dissolve(self):
        b = self.backend(loaded_count=1)
        _, calls = start(self.entry, b, self.params(dissolve=True))
        self.assertTrue(b.calls[0][1]["DISSOLVE"])
        b.finished(True, {})
        self.assertEqual(calls[0][0]["feature_count"], 1)

    def test_unknown_layer(self):
        b = self.backend()
        _, calls = start(self.entry, b, self.params(layer_id="nope"))
        self.assertEqual(calls[0][1].code, "layer.not_found")
        self.assertEqual(b.calls, [])

    def test_degrees_crs_refused(self):
        b = self.backend()
        _, calls = start(self.entry, b, self.params(layer_id="w"))
        self.assertEqual(calls[0][1].code, "analysis.crs_not_metric")
        self.assertIn("reprojetez", calls[0][1].message)
        self.assertEqual(b.calls, [])

    def test_raster_refused(self):
        _, calls = start(self.entry, self.backend(), self.params(layer_id="r"))
        self.assertEqual(calls[0][1].code, "layer.not_vector")

    def test_missing_out_refused(self):
        p = self.params()
        del p["out"]
        b = self.backend()
        _, calls = start(self.entry, b, p)
        self.assertEqual(calls[0][1].code, "analysis.bad_params")
        self.assertEqual(b.calls, [])

    def test_cancel_stops_the_task_and_leaves_no_file(self):
        b = self.backend()
        g, calls = start(self.entry, b, self.params())
        self.assertTrue(os.path.exists(self.out))
        g.cancel()
        self.assertTrue(b.handle.cancelled)
        self.assertEqual(calls, [])  # la tâche tient encore le fichier : pas de réponse
        self.assertTrue(os.path.exists(self.out))
        b.finished(False, {})  # la tâche rend la main : nettoyage puis réponse
        self.assertEqual(calls[0][1].code, "qgis.cancelled")
        self.assertFalse(os.path.exists(self.out))
        b.finished(False, {})  # un rappel de plus ne répond pas deux fois
        self.assertEqual(len(calls), 1)

    def test_cancel_after_the_answer_keeps_the_output(self):
        b = self.backend()
        g, calls = start(self.entry, b, self.params())
        b.finished(True, {})
        g.cancel()
        self.assertTrue(os.path.exists(self.out))
        self.assertEqual(len(calls), 1)

    def test_failure_after_loading_keeps_the_output(self):
        b = self.backend()
        b.feature_count = lambda layer: 1 / 0
        _, calls = start(self.entry, b, self.params())
        b.finished(True, {})
        self.assertEqual(calls[0][1].code, "analysis.failed")
        self.assertTrue(os.path.exists(self.out))

    def test_algorithm_failure_leaves_no_file(self):
        b = self.backend()
        _, calls = start(self.entry, b, self.params())
        b.finished(False, {})
        self.assertEqual(calls[0][1].code, "analysis.failed")
        self.assertFalse(os.path.exists(self.out))

    def test_unreadable_output(self):
        b = self.backend(loadable=False)
        _, calls = start(self.entry, b, self.params())
        b.finished(True, {})
        self.assertEqual(calls[0][1].code, "analysis.failed")
        self.assertFalse(os.path.exists(self.out))

    def test_adding_a_table_row_is_enough(self):
        entry = {
            "algorithm": "native:fake",
            "translate": lambda params, layer: {"INPUT": layer},
            "output": "OUTPUT",
            "input": "layer_id",
        }
        registry = gestures.GestureRegistry()
        analysis.register_all(registry, {"analysis.fake": entry}, backend_factory=lambda: self.backend())
        self.assertEqual(registry.ids(), ["analysis.fake"])
        gesture = registry.create("analysis.fake")
        queue = []
        gesture._schedule = queue.append
        gesture.start({"layer_id": "g", "out": self.out}, lambda result=None, error=None: None)
        queue.pop(0)()
        algo, p = gesture._backend.calls[0]
        self.assertEqual((algo, p["OUTPUT"]), ("native:fake", self.out))

    def test_default_table_registers_the_buffer(self):
        registry = gestures.GestureRegistry()
        analysis.register_all(registry, backend_factory=lambda: self.backend())
        self.assertIn("analysis.buffer", registry.ids())
        self.assertIn("analysis.intersect", registry.ids())
        self.assertIn("analysis.select_by_attribute", registry.ids())
        self.assertIn("analysis.zonal_statistics", registry.ids())


class IntersectTest(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.out = os.path.join(self.dir.name, "2-analysis.intersect.gpkg")
        self.entry = analysis.ALGORITHMS["analysis.intersect"]
        self.backend = FakeBackend([FakeLayer("a"), FakeLayer("b", name="tampon"), FakeLayer("r", vector=False)])

    def tearDown(self):
        self.dir.cleanup()

    def params(self, **extra):
        p = {"layer_id": "a", "overlay_layer_id": "b", "out": self.out}
        p.update(extra)
        return p

    def test_two_layers_go_to_the_algorithm(self):
        g, calls = start(self.entry, self.backend, self.params())
        algo, p = self.backend.calls[0]
        self.assertEqual(algo, "native:intersection")
        self.assertEqual((p["INPUT"].lid, p["OVERLAY"].lid), ("a", "b"))
        self.assertEqual(p["OUTPUT"], self.out)
        self.backend.finished(True, {})
        self.assertIsNone(calls[0][1])
        self.assertEqual(calls[0][0]["layer_id"], "out_1")

    def test_same_layer_refused(self):
        _, calls = start(self.entry, self.backend, self.params(overlay_layer_id="a"))
        self.assertEqual(calls[0][1].code, "analysis.bad_params")
        self.assertEqual(self.backend.calls, [])

    def test_unknown_or_raster_overlay_refused(self):
        _, calls = start(self.entry, self.backend, self.params(overlay_layer_id="nope"))
        self.assertEqual(calls[0][1].code, "layer.not_found")
        _, calls = start(self.entry, self.backend, self.params(overlay_layer_id="r"))
        self.assertEqual(calls[0][1].code, "layer.not_vector")
        self.assertEqual(self.backend.calls, [])

    def test_degrees_are_accepted(self):
        backend = FakeBackend([FakeLayer("a", metric=False), FakeLayer("b", metric=False)])
        start(self.entry, backend, self.params())
        self.assertEqual(len(backend.calls), 1)  # l'intersection ne mesure rien


class SelectByAttributeTest(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.out = os.path.join(self.dir.name, "3-analysis.select_by_attribute.gpkg")
        self.entry = analysis.ALGORITHMS["analysis.select_by_attribute"]
        self.backend = FakeBackend([FakeLayer("a"), FakeLayer("w", crs="EPSG:4326", metric=False)])

    def tearDown(self):
        self.dir.cleanup()

    def params(self, **extra):
        p = {"layer_id": "a", "field": "zone", "operator": "eq", "value": "U", "out": self.out}
        p.update(extra)
        return p

    def test_equal_filter_passes_three_separate_parameters(self):
        start(self.entry, self.backend, self.params())
        algo, p = self.backend.calls[0]
        self.assertEqual(algo, "native:extractbyattribute")
        self.assertEqual((p["FIELD"], p["OPERATOR"], p["VALUE"]), ("zone", 0, "U"))
        self.assertNotIn("EXPRESSION", p)
        self.assertEqual(p["OUTPUT"], self.out)

    def test_every_operator_has_a_distinct_code(self):
        codes = []
        for op in ("eq", "ne", "lt", "lte", "gt", "gte", "contains", "begins_with"):
            backend = FakeBackend([FakeLayer("a")])
            start(self.entry, backend, self.params(operator=op))
            codes.append(backend.calls[0][1]["OPERATOR"])
        self.assertEqual(len(set(codes)), 8)

    def test_numeric_value_stays_text(self):
        start(self.entry, self.backend, self.params(field="pop", operator="gt", value="1000"))
        self.assertEqual(self.backend.calls[0][1]["VALUE"], "1000")

    def test_user_text_is_never_interpreted(self):
        evil = "U\" OR 1=1 --"
        start(self.entry, self.backend, self.params(value=evil))
        self.assertEqual(self.backend.calls[0][1]["VALUE"], evil)

    def test_no_feature_retained_gives_an_empty_layer(self):
        backend = FakeBackend([FakeLayer("a")], loaded_count=0)
        _, calls = start(self.entry, backend, self.params(value="zzz"))
        backend.finished(True, {})
        self.assertEqual(calls[0][0]["feature_count"], 0)

    def test_unknown_field_refused(self):
        _, calls = start(self.entry, self.backend, self.params(field="absent"))
        self.assertEqual(calls[0][1].code, "analysis.unknown_field")
        self.assertEqual(self.backend.calls, [])

    def test_bad_operator_refused_even_if_the_core_let_it_through(self):
        _, calls = start(self.entry, self.backend, self.params(operator="is_null"))
        self.assertEqual(calls[0][1].code, "analysis.bad_params")

    def test_unknown_layer_and_cancellation(self):
        _, calls = start(self.entry, self.backend, self.params(layer_id="nope"))
        self.assertEqual(calls[0][1].code, "layer.not_found")
        g, calls = start(self.entry, self.backend, self.params())
        g.cancel()
        self.backend.finished(False, {})
        self.assertEqual(calls[0][1].code, "qgis.cancelled")
        self.assertFalse(os.path.exists(self.out))

    def test_any_crs_is_accepted(self):
        start(self.entry, self.backend, self.params(layer_id="w"))
        self.assertEqual(len(self.backend.calls), 1)


class ZonalStatisticsTest(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.out = os.path.join(self.dir.name, "4-analysis.zonal_statistics.gpkg")
        self.entry = analysis.ALGORITHMS["analysis.zonal_statistics"]
        self.backend = FakeBackend(
            [
                FakeLayer("z", name="communes", geometry="Polygon", fields=("nom", "pop")),
                FakeLayer("p", name="gares"),
                FakeLayer("r", vector=False),
            ],
            loaded_count=3,
        )

    def tearDown(self):
        self.dir.cleanup()

    def params(self, **extra):
        p = {"layer_id": "z", "points_layer_id": "p", "label_field": "nom", "out": self.out}
        p.update(extra)
        return p

    def test_count_with_a_zone_without_point(self):
        self.backend.pairs = [("Villeneuve", 5), ("Orly", 3), ("Vide", 0)]
        _, calls = start(self.entry, self.backend, self.params())
        algo, p = self.backend.calls[0]
        self.assertEqual(algo, "native:countpointsinpolygon")
        self.assertEqual((p["POLYGONS"].lid, p["POINTS"].lid), ("z", "p"))
        self.assertEqual(p["OUTPUT"], self.out)
        self.backend.finished(True, {})
        result, error = calls[0]
        self.assertIsNone(error)
        self.assertEqual(result["feature_count"], 3)
        self.assertEqual(
            result["zones"],
            [{"label": "Villeneuve", "count": 5}, {"label": "Orly", "count": 3}, {"label": "Vide", "count": 0}],
        )
        self.assertFalse(result["truncated"])
        self.assertEqual(self.backend.counts_call, (p["FIELD"], "nom", 100))

    def test_label_field_is_optional(self):
        p = self.params()
        del p["label_field"]
        self.backend.pairs = [("1", 2)]
        _, calls = start(self.entry, self.backend, p)
        self.backend.finished(True, {})
        self.assertIsNone(calls[0][1])
        self.assertIsNone(self.backend.counts_call[1])

    def test_unknown_label_field_refused(self):
        _, calls = start(self.entry, self.backend, self.params(label_field="absent"))
        self.assertEqual(calls[0][1].code, "analysis.unknown_field")
        self.assertEqual(self.backend.calls, [])

    def test_more_than_100_zones_are_truncated(self):
        self.backend.pairs = [(str(i), i) for i in range(150)]
        self.backend.loaded_count = 150
        _, calls = start(self.entry, self.backend, self.params())
        self.backend.finished(True, {})
        result = calls[0][0]
        self.assertEqual(len(result["zones"]), 100)
        self.assertTrue(result["truncated"])
        self.assertEqual(result["feature_count"], 150)

    def test_same_layer_refused(self):
        _, calls = start(self.entry, self.backend, self.params(points_layer_id="z"))
        self.assertEqual(calls[0][1].code, "analysis.bad_params")
        self.assertEqual(self.backend.calls, [])

    def test_unknown_or_raster_layers_refused(self):
        for key, value, code in (
            ("layer_id", "nope", "layer.not_found"),
            ("layer_id", "r", "layer.not_vector"),
            ("points_layer_id", "nope", "layer.not_found"),
            ("points_layer_id", "r", "layer.not_vector"),
        ):
            _, calls = start(self.entry, self.backend, self.params(**{key: value}))
            self.assertEqual(calls[0][1].code, code)
        self.assertEqual(self.backend.calls, [])

    def test_zones_must_be_polygons(self):
        _, calls = start(self.entry, self.backend, self.params(layer_id="p", points_layer_id="z"))
        self.assertEqual(calls[0][1].code, "analysis.bad_params")
        self.assertEqual(self.backend.calls, [])

    def test_cancellation_leaves_no_file(self):
        g, calls = start(self.entry, self.backend, self.params())
        g.cancel()
        self.backend.finished(False, {})
        self.assertEqual(calls[0][1].code, "qgis.cancelled")
        self.assertFalse(os.path.exists(self.out))


class FormHidesCoreFilledTest(unittest.TestCase):
    def test_out_is_not_a_form_field(self):
        schema = {
            "type": "object",
            "properties": {"out": {"type": "string"}, "layer_id": {"type": "string"}},
            "required": ["layer_id"],
        }
        self.assertEqual([f.name for f in form.build_fields(schema)], ["layer_id"])


if __name__ == "__main__":
    unittest.main()
