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

    def __init__(self, layers=(), write=True, loaded_count=8, loadable=True, loaded_crs="EPSG:2154"):
        self.items = {l.lid: l for l in layers}
        self.loaded_crs, self.known, self.known_calls = loaded_crs, {2154, 4326, 32631, 32632}, []
        self.calls, self.handle, self.finished = [], FakeHandle(), None
        self.write, self.loaded_count, self.loadable = write, loaded_count, loadable
        self.pairs, self.counts_call = [], None
        self.near, self.near_call, self.temporary_count = True, None, 1
        self.encodings, self.driver = [], None

    def layer(self, layer_id):
        return self.items.get(layer_id)

    def is_vector(self, layer):
        return layer.vector

    def crs_is_metric(self, layer):
        return layer.metric

    def crs_is_known(self, epsg):
        self.known_calls.append(epsg)
        return epsg in self.known

    def epsg_is_metric(self, epsg):
        return epsg in (2154, 32631, 32632)

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

    def has_feature_near(self, layer, x, y, epsg, radius):
        self.near_call = (layer.lid, x, y, epsg, radius)
        return self.near

    def temporary_layer(self, handle, reference):
        return FakeLayer(reference, "temporaire", count=self.temporary_count)

    def run_algorithm(self, algorithm_id, parameters, finished, previous=None, encoding=None):
        self.calls.append((algorithm_id, dict(parameters)))
        self.encodings.append(encoding)
        self.finished = finished
        self.handle = FakeHandle()
        out = parameters.get("OUTPUT", parameters.get("OUTPUT_LINES"))
        if self.write and out != "TEMPORARY_OUTPUT":
            with open(out, "wb") as f:
                f.write(b"x")
        return self.handle

    _DRIVERS = {".gpkg": "GPKG", ".shp": "ESRI Shapefile", ".geojson": "GeoJSON"}

    def read_written(self, path):
        self.counted = path
        if not self.loadable or not os.path.exists(path):
            return None
        return self.loaded_count, self.driver or self._DRIVERS.get(os.path.splitext(path)[1])

    def load_output(self, path, name):
        self.loaded = path
        if not self.loadable:
            return None
        return FakeLayer("out_1", name, self.loaded_crs, count=self.loaded_count)


def start(entry, backend, params):
    queue, calls = [], []
    g = analysis.ProcessingGesture(entry, backend, queue.append)
    g.start(params, lambda result=None, error=None: calls.append((result, error)))
    assert calls == []  # asynchrone : rien avant l'ordonnanceur
    while queue:
        queue.pop(0)()
    return g, calls


class BufferTest(unittest.TestCase):
    """Tampon : quand le noyau donne `work_epsg` (couche en degrés), reprojection vers ce système
    puis `native:buffer` sur la couche temporaire ; sans lui (couche en mètres), `native:buffer`
    seul, comme avant (changement de cap du 2026-11-12, amendement RA06)."""

    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.out = os.path.join(self.dir.name, "1-analysis.buffer.gpkg")
        self.entry = analysis.ALGORITHMS["analysis.buffer"]

    def tearDown(self):
        self.dir.cleanup()

    def params(self, **extra):
        """Paramètres tels que le noyau les dicte pour une couche en mètres : sans `work_epsg`."""
        p = {"layer_id": "g", "distance": {"value": 500, "unit": "m"}, "out": self.out}
        p.update(extra)
        return p

    def degrees(self, **extra):
        """Paramètres d'une couche en degrés : le noyau donne le système de calcul."""
        return self.params(**dict({"layer_id": "w", "work_epsg": 2154}, **extra))

    def backend(self, **kw):
        layers = [FakeLayer("g"), FakeLayer("w", crs="EPSG:4326", metric=False), FakeLayer("r", vector=False)]
        return FakeBackend(layers, **kw)

    def algorithms(self, b):
        return [algo for algo, _ in b.calls]

    def reprojected(self, b, params):
        """Lance le geste et finit la reprojection : rend le geste et ses réponses."""
        g, calls = start(self.entry, b, params)
        self.assertEqual(b.calls[0][0], "native:reprojectlayer")
        b.finished(True, {"OUTPUT": "reprojetee"})
        return g, calls

    # -- couche en mètres : le tampon seul, comme avant -------------------------------------
    def test_a_metric_layer_is_buffered_alone(self):
        # Aucune reprojection : une couche dont des entités sont des collections de géométries
        # passe comme avant (RA06).
        b = self.backend()
        g, calls = start(self.entry, b, self.params())
        self.assertEqual(self.algorithms(b), ["native:buffer"])
        algo, p = b.calls[0]
        self.assertEqual(p["INPUT"].lid, "g")
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
        self.assertEqual(self.algorithms(b), ["native:buffer"])

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

    def test_an_empty_layer_is_buffered(self):
        b = self.backend(loaded_count=0)
        b.items["g"].count = 0
        _, calls = start(self.entry, b, self.params())
        self.assertEqual(self.algorithms(b), ["native:buffer"])
        b.finished(True, {})
        self.assertIsNone(calls[0][1])
        self.assertEqual(calls[0][0]["feature_count"], 0)

    def test_without_work_crs_a_layer_in_degrees_is_refused_as_before(self):
        b = self.backend()
        _, calls = start(self.entry, b, self.params(layer_id="w"))
        self.assertEqual(calls[0][1].code, "analysis.crs_not_metric")
        self.assertIn("reprojetez", calls[0][1].message)
        self.assertEqual(b.calls, [])

    def test_unknown_layer(self):
        b = self.backend()
        _, calls = start(self.entry, b, self.params(layer_id="nope"))
        self.assertEqual(calls[0][1].code, "layer.not_found")
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

    def test_a_refusal_never_removes_a_file_the_gesture_did_not_write(self):
        # Un fichier déjà au nom de `out` (écrit par quelqu'un d'autre) survit à un refus.
        with open(self.out, "wb") as f:
            f.write(b"autre")
        for params in (self.params(distance={"value": 1, "unit": "mi"}), self.degrees(work_epsg=4326)):
            b = self.backend()
            _, calls = start(self.entry, b, params)
            self.assertIsNotNone(calls[0][1])
            self.assertEqual(b.calls, [])
            self.assertTrue(os.path.exists(self.out))

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

    # -- couche en degrés : reprojection vers `work_epsg`, puis tampon ----------------------
    def test_a_layer_in_degrees_is_reprojected_then_buffered(self):
        b = self.backend()
        _, calls = start(self.entry, b, self.degrees())
        algo, p = b.calls[0]
        self.assertEqual(algo, "native:reprojectlayer")
        self.assertEqual(p["INPUT"].lid, "w")
        self.assertEqual(p["TARGET_CRS"], "EPSG:2154")  # le système du noyau, jamais un choix du geste
        self.assertEqual(p["OUTPUT"], "TEMPORARY_OUTPUT")
        self.assertFalse(os.path.exists(self.out))
        self.assertEqual(calls, [])
        b.finished(True, {"OUTPUT": "reprojetee"})
        algo, p = b.calls[1]
        self.assertEqual(algo, "native:buffer")
        self.assertEqual(p["INPUT"].lid, "reprojetee")  # la couche temporaire, pas l'entrée
        self.assertEqual(p["DISTANCE"], 500.0)
        self.assertEqual(p["OUTPUT"], self.out)
        self.assertEqual(len(b.calls), 2)
        b.finished(True, {})
        result, error = calls[0]
        self.assertIsNone(error)
        self.assertEqual(result["crs"], "EPSG:2154")
        self.assertTrue(os.path.exists(self.out))

    def test_a_given_work_crs_opens_the_chain_whatever_the_layer(self):
        # Le choix est dicté par le paramètre du noyau, jamais par un calcul du geste.
        b = self.backend()
        self.reprojected(b, self.params(work_epsg=2154))
        self.assertEqual(b.calls[0][1]["INPUT"].lid, "g")
        self.assertEqual(self.algorithms(b), ["native:reprojectlayer", "native:buffer"])

    def test_dissolve_and_kilometers_after_the_reprojection(self):
        b = self.backend()
        self.reprojected(b, self.degrees(dissolve=True, distance={"value": 0.5, "unit": "km"}))
        self.assertTrue(b.calls[1][1]["DISSOLVE"])
        self.assertEqual(b.calls[1][1]["DISTANCE"], 500.0)

    def test_an_empty_reprojected_layer_is_buffered_too(self):
        b = self.backend()
        b.temporary_count = 0
        _, calls = self.reprojected(b, self.degrees())
        self.assertEqual(b.calls[1][0], "native:buffer")
        b.finished(True, {})
        self.assertIsNone(calls[0][1])

    def test_an_unknown_work_crs_is_refused(self):
        b = self.backend()
        _, calls = start(self.entry, b, self.degrees(work_epsg=999999))
        self.assertEqual(calls[0][1].code, "data.unknown_crs")
        self.assertEqual(b.calls, [])

    def test_a_work_crs_in_degrees_is_refused(self):
        b = self.backend()
        _, calls = start(self.entry, b, self.degrees(work_epsg=4326))
        error = calls[0][1]
        self.assertEqual(error.code, "analysis.crs_not_metric")
        self.assertIn("« gares »", error.message)
        self.assertIn("EPSG:4326", error.message)
        self.assertIn("reprojetez", error.message)
        self.assertEqual(b.calls, [])

    def test_a_bad_distance_is_refused_before_any_launch(self):
        b = self.backend()
        _, calls = start(self.entry, b, self.degrees(distance={"value": 1, "unit": "mi"}))
        self.assertEqual(calls[0][1].code, "analysis.bad_params")
        self.assertEqual(b.calls, [])

    def test_cancel_during_the_reprojection_leaves_no_file(self):
        b = self.backend()
        g, calls = start(self.entry, b, self.degrees())
        g.cancel()
        self.assertTrue(b.handle.cancelled)
        self.assertEqual(calls, [])  # la tâche tourne encore : pas de réponse
        b.finished(False, {})
        self.assertEqual(calls[0][1].code, "qgis.cancelled")
        self.assertFalse(os.path.exists(self.out))
        self.assertEqual(len(b.calls), 1)  # le tampon n'est jamais lancé
        b.finished(True, {"OUTPUT": "reprojetee"})  # un rappel de plus ne répond pas deux fois
        self.assertEqual(len(calls), 1)

    def test_cancel_during_the_buffer_after_the_reprojection(self):
        b = self.backend()
        g, calls = self.reprojected(b, self.degrees())
        g.cancel()
        b.finished(False, {})
        self.assertEqual(calls[0][1].code, "qgis.cancelled")
        self.assertFalse(os.path.exists(self.out))

    def test_a_failed_reprojection_leaves_no_file(self):
        b = self.backend()
        _, calls = start(self.entry, b, self.degrees())
        b.finished(False, {})
        self.assertEqual(calls[0][1].code, "analysis.failed")
        self.assertEqual(len(b.calls), 1)
        self.assertFalse(os.path.exists(self.out))

    def test_a_reprojection_without_layer_fails(self):
        b = self.backend()
        _, calls = start(self.entry, b, self.degrees())
        b.finished(True, {})
        self.assertEqual(calls[0][1].code, "analysis.failed")
        self.assertEqual(len(b.calls), 1)

    def test_unreadable_output_after_the_reprojection(self):
        b = self.backend(loadable=False)
        _, calls = self.reprojected(b, self.degrees())
        b.finished(True, {})
        self.assertEqual(calls[0][1].code, "analysis.failed")
        self.assertFalse(os.path.exists(self.out))

    def test_the_form_never_shows_the_work_crs(self):
        self.assertIn("work_epsg", form.CORE_FILLED)

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


class IsochroneTest(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.out = os.path.join(self.dir.name, "5-analysis.isochrone.gpkg")
        self.entry = analysis.ALGORITHMS["analysis.isochrone"]
        self.backend = FakeBackend(
            [
                FakeLayer("n", name="rues", geometry="Line"),
                FakeLayer("w", name="rues4326", crs="EPSG:4326", metric=False, geometry="Line"),
                FakeLayer("p", name="gares"),
                FakeLayer("g", name="communes", geometry="Polygon"),
                FakeLayer("r", vector=False),
            ],
            loaded_count=1,
        )

    def tearDown(self):
        self.dir.cleanup()

    def params(self, **extra):
        p = {
            "layer_id": "n",
            "origin": {"x": 652500, "y": 6850500, "epsg": 2154},
            "mode": "walk",
            "duration": {"value": 3, "unit": "min"},
            "out": self.out,
        }
        p.update(extra)
        return p

    def test_service_area_then_convex_hull_write_one_file(self):
        _, calls = start(self.entry, self.backend, self.params())
        algo, p = self.backend.calls[0]
        self.assertEqual(algo, "native:serviceareafrompoint")
        self.assertEqual(p["STRATEGY"], 1)  # plus rapide
        self.assertEqual(p["DEFAULT_SPEED"], 5.0)
        self.assertAlmostEqual(p["TRAVEL_COST2"], 180 / 3600.0)  # en heures
        self.assertEqual(p["START_POINT"], "652500.0,6850500.0 [EPSG:2154]")
        self.assertEqual(p["OUTPUT_LINES"], "TEMPORARY_OUTPUT")  # jamais de fichier annexe
        self.assertFalse(os.path.exists(self.out))
        self.assertEqual(self.backend.near_call, ("n", 652500.0, 6850500.0, 2154, 100.0))
        self.backend.finished(True, {"OUTPUT_LINES": "tmp_1"})
        self.assertEqual(calls, [])  # la seconde étape tourne encore
        algo, p = self.backend.calls[1]
        self.assertEqual(algo, "native:convexhull")
        self.assertEqual(p["INPUT"].lid, "tmp_1")
        self.assertEqual(p["OUTPUT"], self.out)  # la seule sortie écrite
        self.backend.finished(True, {"OUTPUT": self.out})
        result, error = calls[0]
        self.assertIsNone(error)
        self.assertEqual(
            result, {"layer_id": "out_1", "name": "5-analysis.isochrone", "feature_count": 1, "crs": "EPSG:2154"}
        )
        self.assertTrue(os.path.exists(self.out))

    def test_car_is_faster_than_walking_and_seconds_equal_minutes(self):
        walk = FakeBackend([FakeLayer("n", geometry="Line")])
        start(self.entry, walk, self.params())
        car = FakeBackend([FakeLayer("n", geometry="Line")])
        start(self.entry, car, self.params(mode="car"))
        self.assertEqual(car.calls[0][1]["DEFAULT_SPEED"], 50.0)
        self.assertGreater(car.calls[0][1]["DEFAULT_SPEED"], walk.calls[0][1]["DEFAULT_SPEED"])
        secs = FakeBackend([FakeLayer("n", geometry="Line")])
        start(self.entry, secs, self.params(duration={"value": 180, "unit": "s"}))
        self.assertEqual(secs.calls[0][1]["TRAVEL_COST2"], walk.calls[0][1]["TRAVEL_COST2"])

    def test_no_speed_or_direction_field(self):
        start(self.entry, self.backend, self.params())
        p = self.backend.calls[0][1]
        self.assertEqual((p["SPEED_FIELD"], p["DIRECTION_FIELD"], p["DEFAULT_DIRECTION"]), ("", "", 2))

    def test_unknown_layer(self):
        _, calls = start(self.entry, self.backend, self.params(layer_id="nope"))
        self.assertEqual(calls[0][1].code, "layer.not_found")
        self.assertEqual(self.backend.calls, [])

    def test_non_line_layers_say_to_choose_lines(self):
        for lid in ("g", "p"):
            _, calls = start(self.entry, self.backend, self.params(layer_id=lid))
            self.assertEqual(calls[0][1].code, "analysis.bad_params")
            self.assertIn("couche de lignes", calls[0][1].message)
        self.assertEqual(self.backend.calls, [])

    def test_degrees_crs_refused(self):
        _, calls = start(self.entry, self.backend, self.params(layer_id="w"))
        self.assertEqual(calls[0][1].code, "analysis.crs_not_metric")
        self.assertEqual(self.backend.calls, [])

    def test_origin_far_from_the_network_refused_before_launch(self):
        self.backend.near = False
        _, calls = start(self.entry, self.backend, self.params())
        self.assertEqual(calls[0][1].code, "analysis.origin_off_network")
        self.assertEqual(self.backend.calls, [])
        self.assertFalse(os.path.exists(self.out))

    def test_nothing_reached_gives_origin_off_network_and_no_file(self):
        self.backend.temporary_count = 0
        _, calls = start(self.entry, self.backend, self.params())
        self.backend.finished(True, {"OUTPUT_LINES": "tmp_1"})
        self.assertEqual(calls[0][1].code, "analysis.origin_off_network")
        self.assertEqual(len(self.backend.calls), 1)
        self.assertFalse(os.path.exists(self.out))

    def test_bad_mode_or_origin_refused_even_if_the_core_let_it_through(self):
        for extra in ({"mode": "bike"}, {"origin": {"x": 1, "y": 2}}, {"duration": {"value": 0, "unit": "min"}}):
            backend = FakeBackend([FakeLayer("n", geometry="Line")])
            _, calls = start(self.entry, backend, self.params(**extra))
            self.assertEqual(calls[0][1].code, "analysis.bad_params")
            self.assertEqual(backend.calls, [])

    def test_failure_of_either_step_leaves_no_file(self):
        _, calls = start(self.entry, self.backend, self.params())
        self.backend.finished(False, {})
        self.assertEqual(calls[0][1].code, "analysis.failed")
        _, calls = start(self.entry, self.backend, self.params())
        self.backend.finished(True, {"OUTPUT_LINES": "tmp_1"})
        self.backend.finished(False, {})
        self.assertEqual(calls[0][1].code, "analysis.failed")
        self.assertFalse(os.path.exists(self.out))

    def test_cancel_during_the_second_step_leaves_no_file(self):
        g, calls = start(self.entry, self.backend, self.params())
        self.backend.finished(True, {"OUTPUT_LINES": "tmp_1"})
        self.assertTrue(os.path.exists(self.out))  # la seconde étape écrit
        g.cancel()
        self.assertTrue(self.backend.handle.cancelled)  # c'est la tâche en cours qui est arrêtée
        self.assertEqual(calls, [])
        self.backend.finished(False, {})
        self.assertEqual(calls[0][1].code, "qgis.cancelled")
        self.assertFalse(os.path.exists(self.out))

    def test_cancel_during_the_first_step_never_starts_the_second(self):
        g, calls = start(self.entry, self.backend, self.params())
        g.cancel()
        self.backend.finished(True, {"OUTPUT_LINES": "tmp_1"})
        self.assertEqual(calls[0][1].code, "qgis.cancelled")
        self.assertEqual(len(self.backend.calls), 1)

    def test_default_table_registers_the_isochrone(self):
        registry = gestures.GestureRegistry()
        analysis.register_all(registry, backend_factory=lambda: self.backend)
        self.assertIn("analysis.isochrone", registry.ids())


class ReprojectTest(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.out = os.path.join(self.dir.name, "1-data.reproject.gpkg")
        self.entry = analysis.ALGORITHMS["data.reproject"]

    def tearDown(self):
        self.dir.cleanup()

    def params(self, **extra):
        p = {"layer_id": "w", "target_epsg": 2154, "out": self.out}
        p.update(extra)
        return p

    def backend(self, **kw):
        layers = [
            FakeLayer("w", crs="EPSG:4326", metric=False, count=5),
            FakeLayer("g", count=5),
            FakeLayer("r", name="mnt", vector=False),
        ]
        return FakeBackend(layers, loaded_count=5, **kw)

    def test_degrees_to_lambert_93_gives_a_new_layer(self):
        b = self.backend()
        _, calls = start(self.entry, b, self.params())
        algo, p = b.calls[0]
        self.assertEqual(algo, "native:reprojectlayer")
        self.assertEqual(p["INPUT"].lid, "w")  # la couche d'origine n'est que lue
        self.assertEqual(p["TARGET_CRS"], "EPSG:2154")
        self.assertFalse(p["CONVERT_CURVED_GEOMETRIES"])
        self.assertNotIn("OPERATION", p)  # réglages de transformation du projet
        self.assertEqual(p["OUTPUT"], self.out)  # exactement le fichier du noyau
        b.finished(True, {})
        result, error = calls[0]
        self.assertIsNone(error)
        self.assertEqual(
            result, {"layer_id": "out_1", "name": "1-data.reproject", "feature_count": 5, "crs": "EPSG:2154"}
        )

    def test_same_system_copies_the_layer(self):
        b = self.backend()
        _, calls = start(self.entry, b, self.params(layer_id="g"))
        self.assertEqual(b.calls[0][1]["TARGET_CRS"], "EPSG:2154")
        b.finished(True, {})
        self.assertEqual(calls[0][0]["crs"], "EPSG:2154")

    def test_degrees_are_accepted_and_metres_to_wgs84_too(self):
        b = self.backend(loaded_crs="EPSG:4326")
        _, calls = start(self.entry, b, self.params(layer_id="g", target_epsg=4326))
        self.assertEqual(b.calls[0][1]["TARGET_CRS"], "EPSG:4326")
        b.finished(True, {})
        self.assertEqual(calls[0][0]["crs"], "EPSG:4326")

    def test_unknown_target_launches_nothing_and_names_the_code(self):
        b = self.backend()
        _, calls = start(self.entry, b, self.params(target_epsg=99999))
        error = calls[0][1]
        self.assertEqual(error.code, "data.unknown_crs")
        self.assertIn("EPSG:99999", error.message)
        self.assertEqual(b.known_calls, [99999])
        self.assertEqual(b.calls, [])
        self.assertFalse(os.path.exists(self.out))

    def test_unknown_layer_and_raster_refused(self):
        b = self.backend()
        _, calls = start(self.entry, b, self.params(layer_id="nope"))
        self.assertEqual(calls[0][1].code, "layer.not_found")
        _, calls = start(self.entry, b, self.params(layer_id="r"))
        self.assertEqual(calls[0][1].code, "layer.not_vector")
        self.assertEqual(b.calls, [])

    def test_bad_code_refused_even_if_the_core_let_it_through(self):
        for bad in (None, -2154, 0, 2154.5, "2154", True):
            b = self.backend()
            _, calls = start(self.entry, b, self.params(target_epsg=bad))
            self.assertEqual(calls[0][1].code, "analysis.bad_params", bad)
            self.assertEqual(b.calls, [])

    def test_cancellation_leaves_no_file(self):
        b = self.backend()
        g, calls = start(self.entry, b, self.params())
        g.cancel()
        self.assertTrue(b.handle.cancelled)
        b.finished(False, {})
        self.assertEqual(calls[0][1].code, "qgis.cancelled")
        self.assertFalse(os.path.exists(self.out))

    def test_failure_leaves_no_file(self):
        b = self.backend()
        _, calls = start(self.entry, b, self.params())
        b.finished(False, {})
        self.assertEqual(calls[0][1].code, "analysis.failed")
        self.assertFalse(os.path.exists(self.out))

    def test_default_table_registers_the_reprojection(self):
        registry = gestures.GestureRegistry()
        analysis.register_all(registry, backend_factory=lambda: self.backend())
        self.assertIn("data.reproject", registry.ids())

    def test_a_metric_analysis_on_the_layer_in_degrees_is_still_refused(self):
        b = self.backend()
        _, calls = start(
            analysis.ALGORITHMS["analysis.buffer"],
            b,
            {"layer_id": "w", "distance": {"value": 500, "unit": "m"}, "out": self.out, "work_epsg": 4326},
        )
        self.assertEqual(calls[0][1].code, "analysis.crs_not_metric")
        self.assertIn("reprojetez", calls[0][1].message)
        self.assertEqual(b.calls, [])


class ConvertTest(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.entry = analysis.ALGORITHMS["data.convert"]

    def tearDown(self):
        self.dir.cleanup()

    def out(self, ext):
        return os.path.join(self.dir.name, "2-data.convert" + ext)

    def params(self, fmt="gpkg", ext=None, **extra):
        p = {"layer_id": "g", "format": fmt, "out": self.out(ext or "." + fmt)}
        p.update(extra)
        return p

    def backend(self, **kw):
        layers = [
            FakeLayer("g", count=5),
            FakeLayer("r", name="mnt", vector=False),
            FakeLayer("t", name="table", geometry="NoGeometry"),
            FakeLayer("m", name="mixte", geometry="Unknown geometry"),
        ]
        return FakeBackend(layers, loaded_count=5, **kw)

    def test_a_shapefile_needs_a_single_geometry_type(self):
        for lid in ("t", "m"):
            b = self.backend()
            _, calls = start(self.entry, b, self.params("shp", layer_id=lid))
            error = calls[0][1]
            self.assertEqual(error.code, "analysis.bad_params", lid)
            self.assertIn("un seul type de géométrie", error.message)
            self.assertEqual(b.calls, [])
            # Le GeoPackage et le GeoJSON les prennent.
            for fmt in ("gpkg", "geojson"):
                b = self.backend()
                _, calls = start(self.entry, b, self.params(fmt, layer_id=lid))
                b.finished(True, {})
                self.assertIsNone(calls[0][1], (lid, fmt))

    def test_a_file_in_another_format_is_a_named_error_and_is_removed(self):
        b = self.backend()
        b.driver = "GeoJSON"
        _, calls = start(self.entry, b, self.params("shp"))
        b.finished(True, {})
        error = calls[0][1]
        self.assertEqual(error.code, "data.format_mismatch")
        self.assertIn("shp", error.message)
        self.assert_no_file_left()

    def test_other_algorithms_keep_the_system_encoding(self):
        b = self.backend()
        start(analysis.ALGORITHMS["data.reproject"], b,
              {"layer_id": "g", "target_epsg": 4326, "out": self.out(".gpkg")})
        self.assertEqual(b.encodings, [None])

    def test_each_format_writes_the_file_dictated_by_the_core(self):
        for fmt in ("gpkg", "shp", "geojson"):
            with self.subTest(fmt=fmt):
                b = self.backend()
                _, calls = start(self.entry, b, self.params(fmt))
                algo, p = b.calls[0]
                self.assertEqual(algo, "native:extractbyexpression")
                self.assertEqual(p["INPUT"].lid, "g")  # la couche d'origine n'est que lue
                self.assertEqual(p["EXPRESSION"], "TRUE")  # toutes les entités, copiées
                self.assertEqual(b.encodings, ["UTF-8"])  # textes en UTF-8, quel que soit le système
                self.assertEqual(p["OUTPUT"], self.out("." + fmt))  # exactement le fichier du noyau
                b.finished(True, {})
                result, error = calls[0]
                self.assertIsNone(error)
                self.assertEqual(
                    result, {"file": self.out("." + fmt), "format": fmt, "feature_count": 5}
                )
                # Le fichier est relu à part, jamais ajouté au projet.
                self.assertEqual(b.counted, self.out("." + fmt))
                self.assertFalse(hasattr(b, "loaded"))

    def test_an_extension_that_disagrees_with_the_format_launches_nothing(self):
        for fmt, ext in (("shp", ".gpkg"), ("gpkg", ".shp"), ("xlsx", ".xlsx"), (None, ".gpkg")):
            b = self.backend()
            _, calls = start(self.entry, b, self.params(fmt, ext=ext))
            self.assertEqual(calls[0][1].code, "analysis.bad_params", fmt)
            self.assertEqual(b.calls, [])

    def test_unknown_layer_and_raster_refused(self):
        b = self.backend()
        _, calls = start(self.entry, b, self.params(layer_id="nope"))
        self.assertEqual(calls[0][1].code, "layer.not_found")
        _, calls = start(self.entry, b, self.params(layer_id="r"))
        self.assertEqual(calls[0][1].code, "layer.not_vector")
        self.assertEqual(b.calls, [])

    def write_companions(self):
        for ext in (".shx", ".dbf", ".prj", ".cpg", ".qix"):
            with open(self.out(ext), "wb") as f:
                f.write(b"x")

    def assert_no_file_left(self):
        self.assertEqual([n for n in os.listdir(self.dir.name) if n.startswith("2-data.convert")], [])

    def test_a_failed_shapefile_leaves_no_companion(self):
        b = self.backend()
        _, calls = start(self.entry, b, self.params("shp"))
        self.write_companions()
        b.finished(False, {})
        self.assertEqual(calls[0][1].code, "analysis.failed")
        self.assert_no_file_left()

    def test_a_cancelled_shapefile_leaves_no_companion(self):
        b = self.backend()
        g, calls = start(self.entry, b, self.params("shp"))
        self.write_companions()
        g.cancel()
        self.assertTrue(b.handle.cancelled)
        b.finished(False, {})
        self.assertEqual(calls[0][1].code, "qgis.cancelled")
        self.assert_no_file_left()

    def test_an_unreadable_file_is_a_failure_and_is_removed(self):
        b = self.backend(loadable=False)
        _, calls = start(self.entry, b, self.params("geojson"))
        b.finished(True, {})
        self.assertEqual(calls[0][1].code, "analysis.failed")
        self.assert_no_file_left()

    def test_default_table_registers_the_conversion(self):
        registry = gestures.GestureRegistry()
        analysis.register_all(registry, backend_factory=lambda: self.backend())
        self.assertIn("data.convert", registry.ids())


class RasterSource:
    """Raster ouvert à part par la doublure : son système (None s'il n'en a pas)."""

    def __init__(self, path, crs):
        self.path, self.crs = path, crs


class RasterBackend(FakeBackend):
    """Doublure du genre d'entrée `raster` : sources lisibles et leur système, GeoTIFF relu."""

    def __init__(self, sources, written_epsg=2154, written_readable=True, **kw):
        super().__init__(**kw)
        self.sources, self.written_epsg, self.written_readable = sources, written_epsg, written_readable
        self.added = []

    def raster_source(self, path):
        name = os.path.basename(path)
        return RasterSource(path, self.sources[name]) if name in self.sources else None

    def raster_crs(self, layer):
        return layer.crs

    def same_crs(self, layer, epsg):
        return layer.crs == "EPSG:%d" % epsg

    def read_written_raster(self, path):
        if not self.written_readable or not os.path.exists(path):
            return False
        return self.written_epsg

    def load_output(self, path, name):  # jamais appelé : le noyau dicte l'ouverture
        self.added.append(path)
        return None


class ConvertRasterTest(unittest.TestCase):
    """`data.convert_raster` (epic 10, entrée 30) : genre d'entrée `raster`, sortie GeoTIFF."""

    SOURCES = {"dalle.asc": None, "dalle-2154.asc": "EPSG:2154", "mnt.img": "EPSG:32632"}

    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.out = os.path.join(self.dir.name, "1-data.convert_raster.tif")
        self.entry = analysis.ALGORITHMS["data.convert_raster"]
        for name in list(self.SOURCES) + ["texte.txt"]:
            with open(os.path.join(self.dir.name, name), "wb") as f:
                f.write(b"ncols 2")
        self.originals = {n: self.read(n) for n in self.SOURCES}

    def tearDown(self):
        self.dir.cleanup()

    def read(self, name):
        with open(os.path.join(self.dir.name, name), "rb") as f:
            return f.read()

    def src(self, name):
        return os.path.join(self.dir.name, name)

    def params(self, name="dalle.asc", **extra):
        p = {"raster": self.src(name), "out": self.out}
        p.update(extra)
        return p

    def backend(self, **kw):
        return RasterBackend(self.SOURCES, **kw)

    def assert_no_output(self):
        self.assertEqual([n for n in os.listdir(self.dir.name) if n.startswith("1-data.convert_raster")], [])

    def test_the_row_is_a_fixed_warp_to_float32_minus_9999_first_band(self):
        self.assertEqual(self.entry["algorithm"], "gdal:warpreproject")
        b = self.backend()
        _, calls = start(self.entry, b, self.params(assign_epsg=2154))
        algorithm, params = b.calls[0]
        self.assertEqual(algorithm, "gdal:warpreproject")
        self.assertEqual(params["INPUT"], self.src("dalle.asc"))
        self.assertEqual(params["OUTPUT"], self.out)
        self.assertEqual(params["SOURCE_CRS"], "EPSG:2154")
        # Jamais de reprojection ni de grille imposée : celle de la source, au plus proche voisin.
        self.assertIsNone(params["TARGET_CRS"])
        self.assertIsNone(params["TARGET_RESOLUTION"])
        self.assertIsNone(params["TARGET_EXTENT"])
        self.assertEqual(params["RESAMPLING"], 0)
        self.assertEqual(params["DATA_TYPE"], 6)  # Float32 sous QGIS 3.44
        self.assertEqual(params["NODATA"], -9999)
        self.assertEqual(params["EXTRA"], "-srcband 1 -dstband 1")
        b.finished(True, {"OUTPUT": self.out})
        self.assertEqual(calls, [({"file": self.out, "epsg": 2154}, None)])
        # Le GeoTIFF n'entre pas au projet : le noyau dicte `layer.open_raster`.
        self.assertEqual(b.added, [])

    def test_a_source_with_a_system_keeps_it_and_no_source_crs_is_sent(self):
        b = self.backend(written_epsg=32632)
        _, calls = start(self.entry, b, self.params("mnt.img"))
        self.assertIsNone(b.calls[0][1]["SOURCE_CRS"])
        b.finished(True, {})
        self.assertEqual(calls[0][0], {"file": self.out, "epsg": 32632})

    def test_the_same_system_declared_again_is_accepted(self):
        b = self.backend()
        _, calls = start(self.entry, b, self.params("dalle-2154.asc", assign_epsg=2154))
        self.assertEqual(len(b.calls), 1)

    def test_another_system_is_refused_never_reprojected(self):
        b = self.backend()
        _, calls = start(self.entry, b, self.params("mnt.img", assign_epsg=2154))
        error = calls[0][1]
        self.assertEqual(error.code, "analysis.bad_params")
        self.assertIn("EPSG:32632", error.message)
        self.assertIn("mnt.img", error.message)
        self.assertNotIn(self.dir.name, error.message)
        self.assertEqual(b.calls, [])
        self.assert_no_output()

    def test_an_unknown_code_is_refused_before_any_launch(self):
        b = self.backend()
        _, calls = start(self.entry, b, self.params(assign_epsg=99999))
        self.assertEqual(calls[0][1].code, "data.unknown_crs")
        self.assertIn("EPSG:99999", calls[0][1].message)
        self.assertEqual(b.calls, [])

    def test_a_source_without_a_system_and_no_code_gives_a_tif_without_epsg(self):
        b = self.backend(written_epsg=None)
        _, calls = start(self.entry, b, self.params())
        b.finished(True, {})
        self.assertEqual(calls[0][0], {"file": self.out, "epsg": None})

    def test_a_missing_or_unreadable_file_is_named_without_its_folder(self):
        b = self.backend()
        _, calls = start(self.entry, b, self.params("absent.asc"))
        self.assertEqual(calls[0][1].code, "layer.not_found")
        self.assertIn("absent.asc", calls[0][1].message)
        self.assertNotIn(self.dir.name, calls[0][1].message)
        _, calls = start(self.entry, b, self.params("texte.txt"))
        self.assertEqual(calls[0][1].code, "layer.invalid_source")
        self.assertIn("texte.txt", calls[0][1].message)
        self.assertEqual(b.calls, [])

    def test_a_failure_or_a_cancellation_leaves_no_partial_tif(self):
        b = self.backend()
        _, calls = start(self.entry, b, self.params())
        with open(self.out + ".aux.xml", "wb") as f:
            f.write(b"x")
        b.finished(False, {})
        self.assertEqual(calls[0][1].code, "analysis.failed")
        self.assert_no_output()
        g, calls = start(self.entry, b, self.params())
        g.cancel()
        self.assertTrue(b.handle.cancelled)
        b.finished(False, {})
        self.assertEqual(calls[0][1].code, "qgis.cancelled")
        self.assert_no_output()

    def test_an_unreadable_tif_is_a_failure_and_is_removed(self):
        b = self.backend(written_readable=False)
        _, calls = start(self.entry, b, self.params())
        b.finished(True, {})
        self.assertEqual(calls[0][1].code, "analysis.failed")
        self.assert_no_output()

    def test_the_original_is_never_written(self):
        b = self.backend()
        _, calls = start(self.entry, b, self.params(assign_epsg=2154))
        b.finished(True, {})
        for name, before in self.originals.items():
            self.assertEqual(self.read(name), before, name)

    def test_default_table_registers_the_conversion(self):
        registry = gestures.GestureRegistry()
        analysis.register_all(registry, backend_factory=lambda: self.backend())
        self.assertIn("data.convert_raster", registry.ids())


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
