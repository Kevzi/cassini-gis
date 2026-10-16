"""Tests sans QGIS du geste layout.export_pdf : fond en doublure."""

import os
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "cassini_qgis"))

import layout_export  # noqa: E402


class FakeLayer:
    def __init__(self, lid, source, box):
        self.lid, self.source, self.box = lid, source, box


class FakeBackend:
    def __init__(self, layers=(), fail=False, partial=False):
        self.layers = list(layers)
        self.fail, self.partial = fail, partial
        self.exported = None

    def project_crs(self):
        return "EPSG:2154"

    def visible_layers(self):
        return list(self.layers)

    def source_of(self, layer):
        return layer.source

    def extent_of(self, layer, to_authid=None):
        return layer.box

    def export_pdf(self, layers, title, sources_text, out, authid, box):
        self.exported = (layers, title, sources_text, out, authid, box)
        if self.partial:
            with open(out, "wb") as f:
                f.write(b"%PD")  # fichier à demi écrit
        if self.fail:
            raise RuntimeError("disque plein")
        with open(out, "wb") as f:
            f.write(b"%PDF-1.7")
        return list(layout_export.EXPECTED_ITEMS)


def run(backend, params):
    queue, calls = [], []
    g = layout_export.LayoutExportGesture(backend, queue.append)
    g.start(params, lambda result=None, error=None: calls.append((result, error)))
    assert calls == []  # asynchrone : rien avant l'ordonnanceur
    while queue:
        queue.pop(0)()
    assert len(calls) == 1
    return calls[0], g


TWO = [
    FakeLayer("a", r"C:\Users\x\proj\gares.gpkg|layername=gares", (0.0, 0.0, 10.0, 20.0)),
    FakeLayer("b", "/home/x/communes.shp", (5.0, -10.0, 30.0, 8.0)),
]


class LayoutExportTest(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.out = os.path.join(self.dir.name, "1-layout.export_pdf.pdf")

    def tearDown(self):
        self.dir.cleanup()

    def test_export_writes_the_file_and_reports(self):
        backend = FakeBackend(TWO)
        (result, error), gesture = run(backend, {"title": "  Ma carte ", "out": self.out})
        self.assertIsNone(error)
        self.assertEqual(
            result,
            {"file": self.out, "page_count": 1, "layer_count": 2, "sources": ["gares.gpkg", "communes.shp"], "truncated": False},
        )
        with open(self.out, "rb") as f:
            self.assertTrue(f.read().startswith(b"%PDF"))
        layers, title, sources_text, out, authid, box = backend.exported
        self.assertEqual((title, out, authid), ("Ma carte", self.out, "EPSG:2154"))
        self.assertEqual(sources_text, "Sources : gares.gpkg, communes.shp")
        self.assertNotIn("Users", sources_text)  # jamais de chemin absolu
        self.assertLess(box[0], -0.0)  # marge autour de l'emprise réunie (0, -10, 30, 20)
        self.assertGreater(box[2], 30.0)
        self.assertEqual(gesture.items_placed, list(layout_export.EXPECTED_ITEMS))

    def test_no_visible_layer_is_refused_without_file(self):
        (result, error), _ = run(FakeBackend([]), {"title": "Carte", "out": self.out})
        self.assertIsNone(result)
        self.assertEqual(error.code, "layout.empty")
        self.assertFalse(os.path.exists(self.out))

    def test_blank_title_is_refused(self):
        for title in ("", "   ", None, 3):
            (result, error), _ = run(FakeBackend(TWO), {"title": title, "out": self.out})
            self.assertEqual(error.code, "layout.bad_params")
        self.assertFalse(os.path.exists(self.out))

    def test_missing_out_is_refused(self):
        (_, error), _ = run(FakeBackend(TWO), {"title": "Carte"})
        self.assertEqual(error.code, "layout.bad_params")

    def test_non_file_layers_are_drawn_but_not_listed(self):
        layers = [
            FakeLayer("m", "Point?crs=EPSG:2154&field=id:integer", (0.0, 0.0, 1.0, 1.0)),
            FakeLayer("p", "dbname='x' host=h user='u' password='secret' table=t", (0.0, 0.0, 1.0, 1.0)),
            FakeLayer("f", "/d/gares.gpkg|layername=g", (0.0, 0.0, 1.0, 1.0)),
        ]
        (result, error), _ = run(FakeBackend(layers), {"title": "Carte", "out": self.out})
        self.assertIsNone(error)
        self.assertEqual(result["layer_count"], 3)
        self.assertEqual(result["sources"], ["gares.gpkg"])

    def test_only_non_file_layers_still_exports(self):
        layers = [FakeLayer("m", "Point?crs=EPSG:2154", (3.0, 3.0, 3.0, 3.0))]
        backend = FakeBackend(layers)
        (result, error), _ = run(backend, {"title": "Carte", "out": self.out})
        self.assertIsNone(error)
        self.assertEqual(result["sources"], [])
        box = backend.exported[5]
        self.assertLess(box[0], box[2])  # un point seul reçoit une marge

    def test_sources_are_bounded_and_deduplicated(self):
        layers = [FakeLayer(str(i), "/d/f%d.shp" % i, (0.0, 0.0, 1.0, 1.0)) for i in range(60)]
        layers.append(FakeLayer("dup", "/autre/f0.shp", (0.0, 0.0, 1.0, 1.0)))
        (result, _), _ = run(FakeBackend(layers), {"title": "Carte", "out": self.out})
        self.assertEqual(len(result["sources"]), layout_export.MAX_SOURCES)
        self.assertTrue(result["truncated"])
        self.assertEqual(result["layer_count"], 61)

    def test_failed_export_leaves_no_partial_file(self):
        (result, error), _ = run(FakeBackend(TWO, fail=True, partial=True), {"title": "Carte", "out": self.out})
        self.assertIsNone(result)
        self.assertEqual(error.code, "layout.export_failed")
        self.assertFalse(os.path.exists(self.out))

    def test_layers_without_extent_are_empty(self):
        layers = [FakeLayer("v", "/d/vide.gpkg", None)]
        (_, error), _ = run(FakeBackend(layers), {"title": "Carte", "out": self.out})
        self.assertEqual(error.code, "layout.empty")

    def test_expected_items_are_the_five_plus_title(self):
        self.assertEqual(
            set(layout_export.EXPECTED_ITEMS),
            {"map", "title", "legend", "scalebar", "northarrow", "sources"},
        )


if __name__ == "__main__":
    unittest.main()
