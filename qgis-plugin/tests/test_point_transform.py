"""Tests sans QGIS du geste data.transform_point : le fond QGIS est une doublure."""

import os
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "cassini_qgis"))

import gestures  # noqa: E402
import point_transform  # noqa: E402


class FakeBackend:
    """Connaît 4326 et 2154 ; rend des valeurs fixées, comme le ferait QGIS."""

    def __init__(self, known=(4326, 2154), answer=(700000.0, 6600000.0), fails=False):
        self.known, self.answer, self.fails = set(known), answer, fails
        self.asked = []
        self.calls = []

    def knows(self, epsg):
        self.asked.append(epsg)
        return epsg in self.known

    def transform(self, x, y, source, target):
        self.calls.append((x, y, source, target))
        if self.fails:
            raise point_transform.TransformError("hors domaine")
        return self.answer


class Run:
    """Lance le geste et garde chaque réponse ; l'ordonnanceur est appelé à la main."""

    def __init__(self, backend, params):
        self.answers = []
        self.queue = []
        self.gesture = point_transform.PointTransformGesture(backend, self.queue.append)
        self.gesture.start(params, self._done)

    def _done(self, result=None, error=None):
        self.answers.append((result, error))

    def step(self):
        while self.queue:
            self.queue.pop(0)()
        return self


def run(backend, params):
    return Run(backend, params).step()


WGS84_ORIGIN = {"point": {"x": 3, "y": 46.5, "epsg": 4326}, "target_epsg": 2154}


class TransformTest(unittest.TestCase):
    def test_a_point_is_reprojected_by_the_backend(self):
        backend = FakeBackend()
        r = run(backend, WGS84_ORIGIN)
        self.assertEqual(r.answers, [({"point": {"x": 700000.0, "y": 6600000.0, "epsg": 2154}}, None)])
        self.assertEqual(backend.calls, [(3, 46.5, 4326, 2154)])

    def test_the_way_back_names_the_target_code(self):
        backend = FakeBackend(answer=(3.0, 46.5))
        r = run(backend, {"point": {"x": 700000.0, "y": 6600000.0, "epsg": 2154}, "target_epsg": 4326})
        self.assertEqual(r.answers, [({"point": {"x": 3.0, "y": 46.5, "epsg": 4326}}, None)])

    def test_the_same_system_returns_the_point_untouched(self):
        backend = FakeBackend()
        point = {"x": 652000.25, "y": 6853000.5, "epsg": 2154}
        r = run(backend, {"point": point, "target_epsg": 2154})
        self.assertEqual(r.answers, [({"point": point}, None)])
        self.assertEqual(backend.calls, [])

    def test_an_unknown_target_is_named(self):
        r = run(FakeBackend(), dict(WGS84_ORIGIN, target_epsg=99999))
        (result, error), = r.answers
        self.assertIsNone(result)
        self.assertEqual(error.code, "data.unknown_crs")
        self.assertIn("EPSG:99999", error.message)

    def test_an_unknown_source_is_named(self):
        backend = FakeBackend()
        r = run(backend, {"point": {"x": 1, "y": 2, "epsg": 99999}, "target_epsg": 4326})
        (result, error), = r.answers
        self.assertEqual(error.code, "data.unknown_crs")
        self.assertIn("EPSG:99999", error.message)
        self.assertEqual(backend.calls, [])

    def test_an_unknown_code_is_refused_even_for_the_same_system(self):
        r = run(FakeBackend(), {"point": {"x": 1, "y": 2, "epsg": 99999}, "target_epsg": 99999})
        self.assertEqual(r.answers[0][1].code, "data.unknown_crs")

    def test_a_point_out_of_the_domain_fails_by_name(self):
        r = run(FakeBackend(fails=True), WGS84_ORIGIN)
        (result, error), = r.answers
        self.assertIsNone(result)
        self.assertEqual(error.code, "data.transform_failed")
        self.assertIn("EPSG:4326", error.message)
        self.assertIn("EPSG:2154", error.message)

    def test_a_non_finite_result_fails_by_name(self):
        for answer in [(float("inf"), 1.0), (1.0, float("nan")), (None, None)]:
            r = run(FakeBackend(answer=answer), WGS84_ORIGIN)
            self.assertEqual(r.answers[0][1].code, "data.transform_failed", answer)

    def test_a_non_finite_input_fails_by_name(self):
        r = run(FakeBackend(), {"point": {"x": float("nan"), "y": 2, "epsg": 4326}, "target_epsg": 2154})
        self.assertEqual(r.answers[0][1].code, "data.transform_failed")

    def test_unreadable_params_answer_once_without_exception(self):
        for params in [None, {}, {"point": "3,46", "target_epsg": 2154},
                       {"point": {"x": 3, "y": 46.5, "epsg": 4326}, "target_epsg": "2154"},
                       {"point": {"x": 3, "y": 46.5, "epsg": True}, "target_epsg": 2154}]:
            r = run(FakeBackend(), params)
            self.assertEqual(len(r.answers), 1, params)
            self.assertEqual(r.answers[0][1].code, "qgis.gesture_failed", params)

    def test_an_unexpected_backend_failure_still_answers(self):
        class Broken(FakeBackend):
            def knows(self, epsg):
                raise RuntimeError("panne")

        r = run(Broken(), WGS84_ORIGIN)
        self.assertEqual(len(r.answers), 1)
        self.assertEqual(r.answers[0][1].code, "qgis.gesture_failed")


class CancelTest(unittest.TestCase):
    def test_cancel_before_the_work_answers_once(self):
        backend = FakeBackend()
        r = Run(backend, WGS84_ORIGIN)
        r.gesture.cancel()
        r.step()
        self.assertEqual(len(r.answers), 1)
        self.assertEqual(r.answers[0][1].code, "qgis.cancelled")
        self.assertEqual(backend.calls, [])

    def test_cancel_after_the_answer_adds_nothing(self):
        r = run(FakeBackend(), WGS84_ORIGIN)
        r.gesture.cancel()
        self.assertEqual(len(r.answers), 1)
        self.assertIsNone(r.answers[0][1])

    def test_start_does_not_work_before_the_scheduler(self):
        backend = FakeBackend()
        r = Run(backend, WGS84_ORIGIN)
        self.assertEqual(r.answers, [])
        self.assertEqual(backend.asked, [])


class RegistryTest(unittest.TestCase):
    def test_the_gesture_registers_under_its_command_id(self):
        registry = gestures.GestureRegistry()
        registry.register("data.transform_point", point_transform.PointTransformGesture)
        self.assertIsInstance(registry.create("data.transform_point"), point_transform.PointTransformGesture)

    def test_only_the_backend_touches_qgis(self):
        with open(point_transform.__file__, encoding="utf-8") as f:
            source = f.read()
        head, _, rest = source.partition("class QgisBackend")
        backend, _, tail = rest.partition("def _singleshot")
        self.assertNotIn("from qgis.core", head + tail)
        self.assertIn("from qgis.core", backend)


if __name__ == "__main__":
    unittest.main()
