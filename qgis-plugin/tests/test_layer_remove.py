"""Tests sans QGIS du geste `layer.remove` (epic 10, entrée 27) : fond en doublure."""

import os
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "cassini_qgis"))

import delegated  # noqa: E402
import layer_remove  # noqa: E402
import summary  # noqa: E402


class FakeBackend:
    """Un projet en mémoire : id -> (nom, fichier[, état]). L'état, absent pour une couche
    enregistrée, est ce que le retrait détruirait ("memory" ou "edits"). Le fond ne sait que
    lire une couche, son nom et son état, et la retirer du projet : aucun accès au fichier
    n'est offert au geste."""

    def __init__(self, layers):
        self.layers = dict(layers)
        self.removed = []

    def layer(self, layer_id):
        return self.layers.get(layer_id)

    def name_of(self, layer):
        return layer[0]

    def unsaved(self, layer):
        return layer[2] if len(layer) > 2 else None

    def remove(self, layer_id):
        self.removed.append(layer_id)
        del self.layers[layer_id]


def run(gesture, params):
    """Lance le geste avec un ordonnanceur immédiat ; rend (résultat, erreur)."""
    got = {}
    gesture.start(params, lambda result=None, error=None: got.update(result=result, error=error))
    return got.get("result"), got.get("error")


class LayerRemoveTest(unittest.TestCase):
    def setUp(self):
        self._dir = tempfile.TemporaryDirectory()
        self.file = os.path.join(self._dir.name, "routes.gpkg")
        with open(self.file, "wb") as f:
            f.write(b"donnees des routes")
        # Date ancienne et fixe : une écriture, même du même contenu, la changerait.
        os.utime(self.file, (1_000_000_000, 1_000_000_000))
        self.before = self._state()

    def tearDown(self):
        self._dir.cleanup()

    def _state(self):
        with open(self.file, "rb") as f:
            return f.read(), os.stat(self.file).st_mtime_ns

    def gesture(self, backend):
        return layer_remove.LayerRemoveGesture(backend, schedule=lambda fn: fn())

    def backend(self):
        return FakeBackend({"routes_1": ("Routes", self.file), "gares_2": ("Gares", "gares.gpkg")})

    def test_the_layer_leaves_the_project_and_its_file_is_untouched(self):
        backend = self.backend()
        result, error = run(self.gesture(backend), {"layer_id": "routes_1"})
        self.assertIsNone(error)
        self.assertEqual(result, {"layer_id": "routes_1"})
        self.assertEqual(backend.removed, ["routes_1"])
        self.assertEqual(sorted(backend.layers), ["gares_2"])
        self.assertTrue(os.path.isfile(self.file))
        self.assertEqual(self._state(), self.before)

    def test_an_absent_layer_gives_the_same_result_without_error(self):
        backend = self.backend()
        run(self.gesture(backend), {"layer_id": "routes_1"})
        result, error = run(self.gesture(backend), {"layer_id": "routes_1"})
        self.assertIsNone(error)
        self.assertEqual(result, {"layer_id": "routes_1"})
        self.assertEqual(backend.removed, ["routes_1"])  # pas de second retrait
        result, error = run(self.gesture(backend), {"layer_id": "inconnue_9"})
        self.assertIsNone(error)
        self.assertEqual(result, {"layer_id": "inconnue_9"})
        self.assertEqual(sorted(backend.layers), ["gares_2"])
        self.assertEqual(self._state(), self.before)

    def test_the_removed_name_is_kept_for_display_only(self):
        backend = self.backend()
        result, _ = run(self.gesture(backend), {"layer_id": "gares_2"})
        self.assertEqual(result, {"layer_id": "gares_2"})  # aucun nom au résultat (fiche)
        self.assertEqual(layer_remove.removed_name("gares_2"), "Gares")
        self.assertIsNone(layer_remove.removed_name("jamais_vue"))

    def test_an_empty_id_is_an_absent_layer_without_error(self):
        backend = self.backend()
        result, error = run(self.gesture(backend), {"layer_id": ""})
        self.assertIsNone(error)
        self.assertEqual(result, {"layer_id": ""})
        self.assertEqual(backend.removed, [])

    def test_the_removed_layer_is_named_in_the_sentence_not_by_its_id(self):
        backend = self.backend()
        run(self.gesture(backend), {"layer_id": "routes_1"})

        def name(layer_id):
            # Nom vivant : la couche n'est plus au projet, le repli donne son ancien nom.
            return layer_remove.display_name(
                layer_id, lambda i: backend.name_of(backend.layer(i)) if backend.layer(i) else None
            )

        self.assertEqual(name("gares_2"), "Gares")
        params = result = {"layer_id": "routes_1"}
        text = summary.sentence("layer.remove", params, result, "fr", name)
        self.assertIn("Routes", text)
        self.assertNotIn("routes_1", text)

    def test_a_failing_live_lookup_falls_back_to_the_removed_name(self):
        backend = self.backend()
        run(self.gesture(backend), {"layer_id": "routes_1"})

        def broken(layer_id):
            raise RuntimeError("QGIS absent")

        self.assertEqual(layer_remove.display_name("routes_1", broken), "Routes")
        self.assertIsNone(layer_remove.display_name("jamais_vue", lambda i: None))

    def test_an_order_without_a_string_id_is_answered_by_an_error(self):
        for params in ({}, {"layer_id": 3}, {"layer_id": None}, None):
            backend = self.backend()
            result, error = run(self.gesture(backend), params)
            self.assertIsNone(result)
            self.assertEqual(error.code, "qgis.gesture_failed")
            self.assertEqual(backend.removed, [])

    def test_an_unexpected_failure_is_answered_once(self):
        class Broken(FakeBackend):
            def remove(self, layer_id):
                raise RuntimeError("boum")

        calls = []
        gesture = self.gesture(Broken({"routes_1": ("Routes", self.file)}))
        gesture.start({"layer_id": "routes_1"}, lambda result=None, error=None: calls.append(error))
        gesture.cancel()
        self.assertEqual(len(calls), 1)
        self.assertEqual(calls[0].code, "qgis.gesture_failed")

    def test_a_memory_layer_is_refused_and_stays_in_the_project(self):
        backend = FakeBackend({"brouillon_1": ("Brouillon", None, "memory")})
        result, error = run(self.gesture(backend), {"layer_id": "brouillon_1"})
        self.assertIsNone(result)
        self.assertEqual(error.code, "layer.unsaved")
        self.assertIn("« Brouillon »", error.message)
        self.assertIn("sans fichier", error.message)
        self.assertEqual(backend.removed, [])
        self.assertIn("brouillon_1", backend.layers)
        self.assertIsNone(layer_remove.removed_name("brouillon_1"))

    def test_a_layer_with_unsaved_edits_is_refused_and_keeps_them(self):
        backend = FakeBackend({"routes_1": ("Routes", self.file, "edits")})
        result, error = run(self.gesture(backend), {"layer_id": "routes_1"})
        self.assertIsNone(result)
        self.assertEqual(error.code, "layer.unsaved")
        self.assertIn("modifications non enregistrées", error.message)
        self.assertIn("Enregistrez ou annulez", error.message)
        self.assertEqual(backend.removed, [])
        self.assertEqual(backend.layers["routes_1"][2], "edits")

    def test_the_real_backend_tells_what_a_removal_would_destroy(self):
        class Layer:
            def __init__(self, provider, editable=False, modified=False, raster=False):
                self._provider = provider
                if not raster:
                    self.isEditable = lambda: editable
                    self.isModified = lambda: modified

            def providerType(self):
                return self._provider

        unsaved = layer_remove.QgisBackend().unsaved
        self.assertEqual(unsaved(Layer("memory")), "memory")
        self.assertEqual(unsaved(Layer("memory", True, True)), "memory")
        self.assertEqual(unsaved(Layer("ogr", True, True)), "edits")
        self.assertIsNone(unsaved(Layer("ogr", True, False)))  # en édition, rien de modifié
        self.assertIsNone(unsaved(Layer("ogr", False, True)))
        self.assertIsNone(unsaved(Layer("ogr")))
        self.assertIsNone(unsaved(Layer("gdal", raster=True)))  # raster sans isModified

    def test_a_layer_without_unsaved_content_is_removed(self):
        # « En édition sans modification » : prédicat du vrai fond, test ci-dessus.
        backend = FakeBackend({"routes_1": ("Routes", self.file, None)})
        result, error = run(self.gesture(backend), {"layer_id": "routes_1"})
        self.assertIsNone(error)
        self.assertEqual(result, {"layer_id": "routes_1"})
        self.assertEqual(backend.removed, ["routes_1"])

    def test_the_gesture_is_in_the_closed_table(self):
        registry = delegated.build_registry(object)
        self.assertIn("layer.remove", registry.ids())
        self.assertIsInstance(registry.create("layer.remove"), layer_remove.LayerRemoveGesture)

    def test_only_the_backend_imports_qgis(self):
        with open(layer_remove.__file__, encoding="utf-8") as f:
            text = f.read()
        top = text.split("class QgisBackend", 1)[0]
        self.assertNotIn("qgis.core", top.replace("`qgis.core`", ""))
        self.assertIn("removeMapLayer(layer_id)", text)


if __name__ == "__main__":
    unittest.main()
