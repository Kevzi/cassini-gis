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
    def __init__(self, lid, name="gares", crs="EPSG:2154", vector=True, metric=True, count=8):
        self.lid, self.name, self.crs, self.vector, self.metric, self.count = lid, name, crs, vector, metric, count


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
        self.assertEqual(calls[0][1].code, "qgis.cancelled")
        self.assertFalse(os.path.exists(self.out))
        b.finished(False, {})  # le rappel tardif de la tâche ne répond pas deux fois
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
