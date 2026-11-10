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
    def __init__(
        self, layers=(), fail=False, partial=False, center=None, held=None, frame=None, crs="EPSG:2154", grid_frame=None
    ):
        self.layers = list(layers)
        self.fail, self.partial = fail, partial
        self.center = center  # centre de la carte de QGIS ; None : pas de carte
        self.held = held  # échelle relue sur la carte ; None : celle demandée
        self.frame = frame  # emprise de la carte de mise en page ; None : voir `export_pdf`
        self.crs = crs
        self.grid_frame = grid_frame  # cadre de la carte dans le système de la grille
        self.known = {2154, 4326, 32631, 32632}
        self.exported = None
        self.scale = "jamais passée"
        self.center_asked = []
        self.reprojected = []

    def canvas_center(self, authid):
        self.center_asked.append(authid)
        return self.center

    def reproject_point(self, x, y, from_authid, to_authid):
        """Doublure : décale de (+10, +5) d'un système à l'autre ; None hors domaine."""
        self.reprojected.append((x, y, from_authid, to_authid))
        if from_authid == to_authid:
            return (x, y)
        if from_authid == "EPSG:9999":
            return None
        return (x + 10.0, y + 5.0)

    def knows(self, epsg):
        return epsg in self.known

    def project_crs(self):
        return self.crs

    def visible_layers(self):
        return list(self.layers)

    def id_of(self, layer):
        return layer.lid

    def source_of(self, layer):
        return layer.source

    def extent_of(self, layer, to_authid=None):
        return layer.box

    def is_vector(self, layer):
        return not layer.source.lower().endswith((".tif", ".tiff", ".asc", ".vrt"))

    # Détail des sources (epic 10, entrée 31) : rien par défaut, comme QGIS qui ne sait pas.
    project_folder, project_stem = "", ""
    resolutions, metadata_dates, file_dates = {}, {}, {}

    def project_dir(self):
        return self.project_folder

    def project_name(self):
        return self.project_stem

    def resolution_of(self, layer):
        return self.resolutions.get(layer.lid)

    def metadata_date(self, layer):
        return self.metadata_dates.get(layer.lid)

    def file_date(self, path):
        return self.file_dates.get(path)

    def export_pdf(self, layers, title, sources_text, out, authid, box, **kwargs):
        self.exported = (layers, title, sources_text, out, authid, box)
        self.kwargs = kwargs
        scale = kwargs.get("scale")
        grid = kwargs.get("grid")
        if grid is not None:
            # Comme le vrai fond : la densité est jugée avant l'export, aucun fichier n'est écrit.
            # Par défaut, un cadre de 10 km de côté (0,2° × 0,1° pour un graticule).
            default = (0.0, 0.0, 10000.0, 10000.0) if grid["projected"] else (2.3, 48.7, 2.5, 48.8)
            layout_export.check_grid_density(self.grid_frame or default, grid["interval"])
        if self.partial:
            with open(out, "wb") as f:
                f.write(b"%PD")  # fichier à demi écrit
        if self.fail:
            raise RuntimeError("disque plein")
        with open(out, "wb") as f:
            f.write(b"%PDF-1.7")
        held = self.held if self.held is not None else (scale if scale is not None else 31234.5)
        frame = self.frame
        if frame is None:
            # Cadre à l'échelle : 2 km de côté autour du centre reçu ; sinon le rectangle reçu.
            cx, cy = (box[0] + box[2]) / 2, (box[1] + box[3]) / 2
            frame = (cx - 1000.0, cy - 1000.0, cx + 1000.0, cy + 1000.0) if scale is not None else box
        return {"items": list(layout_export.EXPECTED_ITEMS), "scale": held, "extent": frame}


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
            {"file": self.out, "page_count": 1, "layer_count": 2, "sources": ["gares.gpkg", "communes.shp"], "truncated": False,
             "source_details": [
                 {"source": "gares.gpkg", "resolution": None, "date": None, "date_origin": None},
                 {"source": "communes.shp", "resolution": None, "date": None, "date_origin": None},
             ]},
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

    def test_the_map_is_framed_on_vector_layers_the_dem_staying_behind(self):
        # Un MNT de 0 à 100 autour de vecteurs de 40 à 60 : le cadre suit les vecteurs, marge de 5 %.
        layers = [
            FakeLayer("g", "/d/gares.gpkg|layername=g", (40.0, 40.0, 60.0, 50.0)),
            FakeLayer("z", "/d/zones.shp", (45.0, 42.0, 55.0, 60.0)),
            FakeLayer("mnt", "/d/mnt.tif", (0.0, 0.0, 100.0, 100.0)),
        ]
        backend = FakeBackend(layers)
        (result, error), _ = run(backend, {"title": "Carte", "out": self.out})
        self.assertIsNone(error)
        self.assertEqual(backend.exported[5], (39.0, 39.0, 61.0, 61.0))
        self.assertEqual(result["layer_count"], 3)  # le MNT reste sur la carte, en fond

    def test_without_vector_layers_the_map_is_framed_on_the_rasters(self):
        layers = [FakeLayer("mnt", "/d/mnt.tif", (0.0, 0.0, 100.0, 100.0))]
        backend = FakeBackend(layers)
        (_, error), _ = run(backend, {"title": "Carte", "out": self.out})
        self.assertIsNone(error)
        self.assertEqual(backend.exported[5], (-5.0, -5.0, 105.0, 105.0))

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

    def test_leave_out_removes_the_layer_its_source_and_its_count(self):
        backend = FakeBackend(TWO)
        params = {"title": "Carte", "out": self.out, "leave_out": ["a", "inconnue"]}
        (result, error), _ = run(backend, params)
        self.assertIsNone(error)
        self.assertEqual(result["layer_count"], 1)
        self.assertEqual(result["sources"], ["communes.shp"])
        layers, _, sources_text, _, _, box = backend.exported
        self.assertEqual([layer.lid for layer in layers], ["b"])
        self.assertEqual(sources_text, "Sources : communes.shp")
        # Emprise de « b » seule (5, -10, 30, 8), marge de 5 % du plus grand côté (25 → 1,25).
        self.assertEqual(box, (3.75, -11.25, 31.25, 9.25))
        # La couche laissée hors de la carte reste dans le projet.
        self.assertEqual([layer.lid for layer in backend.layers], ["a", "b"])

    def test_empty_or_absent_leave_out_changes_nothing(self):
        for extra in ({}, {"leave_out": []}):
            backend = FakeBackend(TWO)
            (result, error), _ = run(backend, dict({"title": "Carte", "out": self.out}, **extra))
            self.assertIsNone(error)
            self.assertEqual(result["layer_count"], 2)
            self.assertEqual(result["sources"], ["gares.gpkg", "communes.shp"])

    def test_all_layers_left_out_is_empty(self):
        params = {"title": "Carte", "out": self.out, "leave_out": ["a", "b"]}
        (_, error), _ = run(FakeBackend(TWO), params)
        self.assertEqual(error.code, "layout.empty")
        self.assertFalse(os.path.exists(self.out))

    def test_bad_leave_out_is_refused(self):
        for bad in ("a", [3], None):
            params = {"title": "Carte", "out": self.out, "leave_out": bad}
            (_, error), _ = run(FakeBackend(TWO), params)
            self.assertEqual(error.code, "layout.bad_params")

    def test_scale_is_held_centered_on_the_qgis_map(self):
        backend = FakeBackend(TWO, center=(100.0, 200.0))
        (result, error), gesture = run(backend, {"title": "Carte", "out": self.out, "scale": 25000})
        self.assertIsNone(error)
        self.assertEqual(result["scale"], 25000)
        self.assertEqual(result["center"], {"x": 100.0, "y": 200.0, "epsg": 2154})
        self.assertEqual(backend.kwargs, {"scale": 25000})
        self.assertEqual(backend.center_asked, ["EPSG:2154"])
        box = backend.exported[5]
        self.assertEqual(((box[0] + box[2]) / 2, (box[1] + box[3]) / 2), (100.0, 200.0))
        self.assertLess(box[0], box[2])
        self.assertTrue(os.path.isfile(self.out))
        self.assertEqual(gesture.items_placed, list(layout_export.EXPECTED_ITEMS))

    def test_without_qgis_map_the_center_is_the_layers_one(self):
        backend = FakeBackend(TWO)  # sans interface : pas de carte
        (result, error), _ = run(backend, {"title": "Carte", "out": self.out, "scale": 25000})
        self.assertIsNone(error)
        self.assertEqual(result["scale"], 25000)
        box = backend.exported[5]
        # Emprise réunie (0, -10, 30, 20) : centre (15, 5).
        self.assertEqual(((box[0] + box[2]) / 2, (box[1] + box[3]) / 2), (15.0, 5.0))
        self.assertEqual(result["center"], {"x": 15.0, "y": 5.0, "epsg": 2154})

    def test_without_scale_the_backend_gets_the_same_arguments_as_before(self):
        for extra in ({}, {"scale": None}):
            backend = FakeBackend(TWO, center=(100.0, 200.0))
            (result, error), _ = run(backend, dict({"title": "Carte", "out": self.out}, **extra))
            self.assertIsNone(error)
            self.assertNotIn("scale", result)
            self.assertNotIn("center", result)
            self.assertEqual(backend.kwargs, {})
            self.assertEqual(backend.center_asked, [])
            # Cadrage d'avant : emprise réunie (0, -10, 30, 20) avec sa marge de 5 % (1,5).
            self.assertEqual(backend.exported[5], (-1.5, -11.5, 31.5, 21.5))

    def test_bad_scale_is_refused_before_qgis(self):
        for bad in (0, -5, 99, 100000001, 2.5, "25000", True, [25000]):
            backend = FakeBackend(TWO)
            (result, error), _ = run(backend, {"title": "Carte", "out": self.out, "scale": bad})
            self.assertIsNone(result)
            self.assertEqual(error.code, "layout.bad_params", bad)
            self.assertIsNone(backend.exported)
        self.assertFalse(os.path.exists(self.out))

    def test_bounds_are_accepted(self):
        for scale in (100, 100000000):
            (result, error), _ = run(FakeBackend(TWO), {"title": "Carte", "out": self.out, "scale": scale})
            self.assertIsNone(error)
            self.assertEqual(result["scale"], scale)

    def test_scale_not_held_leaves_no_file(self):
        for held in (25001.0, 24000):
            backend = FakeBackend(TWO, held=held)
            (result, error), _ = run(backend, {"title": "Carte", "out": self.out, "scale": 25000})
            self.assertIsNone(result)
            self.assertEqual(error.code, "layout.export_failed")
            self.assertFalse(os.path.exists(self.out))

    def test_scale_read_back_is_rounded(self):
        backend = FakeBackend(TWO, held=24999.9999)
        (result, error), _ = run(backend, {"title": "Carte", "out": self.out, "scale": 25000})
        self.assertIsNone(error)
        self.assertEqual(result["scale"], 25000)

    def test_a_given_center_wins_over_the_qgis_map(self):
        # Au rejeu, le noyau repasse le centre journalisé : la vue de QGIS a bougé, peu importe.
        backend = FakeBackend(TWO, center=(100.0, 200.0))
        logged = {"x": 15.0, "y": 5.0, "epsg": 2154}
        params = {"title": "Carte", "out": self.out, "scale": 25000, "center": logged}
        (result, error), _ = run(backend, params)
        self.assertIsNone(error)
        self.assertEqual(backend.center_asked, [])
        box = backend.exported[5]
        self.assertEqual(((box[0] + box[2]) / 2, (box[1] + box[3]) / 2), (15.0, 5.0))
        # Rendu inchangé : le rejeu compare un résultat identique.
        self.assertEqual(result["center"], logged)
        self.assertEqual(result["scale"], 25000)

    def test_the_center_is_rounded_to_six_decimals(self):
        # Un flottant à 17 chiffres pourrait être relu par le noyau à un ULP près : le centre
        # est arrondi avant de cadrer et d'être rendu.
        near = [FakeLayer("c", "/d/c.gpkg", (650000.0, 6850000.0, 655000.0, 6855000.0))]
        backend = FakeBackend(near, center=(652341.1234567891, 6851234.9876543219))
        (result, error), _ = run(backend, {"title": "Carte", "out": self.out, "scale": 25000})
        self.assertIsNone(error)
        self.assertEqual(result["center"], {"x": 652341.123457, "y": 6851234.987654, "epsg": 2154})
        box = backend.exported[5]
        self.assertAlmostEqual((box[0] + box[2]) / 2, 652341.123457, places=6)
        # Au rejeu, le centre arrondi reçu est rendu tel quel.
        backend = FakeBackend(near, center=(100.0, 200.0))
        params = {"title": "Carte", "out": self.out, "scale": 25000, "center": result["center"]}
        (again, error), _ = run(backend, params)
        self.assertIsNone(error)
        self.assertEqual(again["center"], result["center"])

    def test_a_center_in_another_system_is_reprojected(self):
        backend = FakeBackend(TWO)
        params = {"title": "Carte", "out": self.out, "scale": 25000, "center": {"x": 2.4, "y": 48.7, "epsg": 4326}}
        (result, error), _ = run(backend, params)
        self.assertIsNone(error)
        self.assertEqual(backend.reprojected, [(2.4, 48.7, "EPSG:4326", "EPSG:2154")])
        self.assertEqual(result["center"], {"x": 12.4, "y": 53.7, "epsg": 2154})

    def test_a_center_that_cannot_be_reprojected_is_refused(self):
        params = {"title": "Carte", "out": self.out, "scale": 25000, "center": {"x": 1.0, "y": 2.0, "epsg": 9999}}
        backend = FakeBackend(TWO)
        (result, error), _ = run(backend, params)
        self.assertIsNone(result)
        self.assertEqual(error.code, "layout.bad_params")
        self.assertIsNone(backend.exported)

    def test_a_bad_center_is_refused_before_qgis(self):
        for bad in (
            {"x": 1.0, "y": 2.0},
            {"x": 1.0, "y": 2.0, "epsg": 2154, "z": 0.0},
            {"x": "1", "y": 2.0, "epsg": 2154},
            {"x": True, "y": 2.0, "epsg": 2154},
            {"x": float("nan"), "y": 2.0, "epsg": 2154},
            {"x": 1.0, "y": 2.0, "epsg": 0},
            {"x": 1.0, "y": 2.0, "epsg": "2154"},
            [1.0, 2.0],
            "ici",
        ):
            backend = FakeBackend(TWO)
            params = {"title": "Carte", "out": self.out, "scale": 25000, "center": bad}
            (result, error), _ = run(backend, params)
            self.assertEqual(error.code, "layout.bad_params", bad)
            self.assertIsNone(backend.exported)

    def test_without_scale_a_center_changes_nothing(self):
        backend = FakeBackend(TWO)
        params = {"title": "Carte", "out": self.out, "center": {"x": 1.0, "y": 2.0, "epsg": 2154}}
        (result, error), _ = run(backend, params)
        self.assertIsNone(error)
        self.assertNotIn("center", result)
        self.assertEqual(backend.exported[5], (-1.5, -11.5, 31.5, 21.5))

    def test_a_project_without_epsg_code_refuses_a_scale(self):
        for crs in ("", "USER:100000", "IGNF:LAMB93"):
            backend = FakeBackend(TWO, crs=crs)
            (result, error), _ = run(backend, {"title": "Carte", "out": self.out, "scale": 25000})
            self.assertEqual(error.code, "layout.bad_params", crs)
            self.assertIsNone(backend.exported)
        # Sans échelle, l'export d'avant reste permis.
        (result, error), _ = run(FakeBackend(TWO, crs="USER:100000"), {"title": "Carte", "out": self.out})
        self.assertIsNone(error)

    def test_an_empty_frame_is_refused_without_file(self):
        # La vue de QGIS montre une zone sans donnée : aucune couche ne touche le cadre.
        backend = FakeBackend(TWO, center=(5000.0, 5000.0), frame=(4000.0, 4000.0, 6000.0, 6000.0))
        (result, error), gesture = run(backend, {"title": "Carte", "out": self.out, "scale": 25000})
        self.assertIsNone(result)
        self.assertEqual(error.code, "layout.empty")
        self.assertIn("déplacez la vue", error.message)
        self.assertFalse(os.path.exists(self.out))
        self.assertEqual(gesture.items_placed, [])

    def test_a_frame_touching_one_layer_exports(self):
        # Le cadre touche le bord de « b » (5, -10, 30, 8) : la carte n'est pas vide.
        backend = FakeBackend(TWO, center=(35.0, 0.0), frame=(30.0, -5.0, 40.0, 5.0))
        (result, error), _ = run(backend, {"title": "Carte", "out": self.out, "scale": 25000})
        self.assertIsNone(error)
        self.assertTrue(os.path.isfile(self.out))

    def test_without_scale_the_frame_is_not_checked(self):
        backend = FakeBackend(TWO, frame=(4000.0, 4000.0, 6000.0, 6000.0))
        (result, error), _ = run(backend, {"title": "Carte", "out": self.out})
        self.assertIsNone(error)

    def test_scale_not_held_does_not_announce_items(self):
        backend = FakeBackend(TWO, held=24000)
        (_, error), gesture = run(backend, {"title": "Carte", "out": self.out, "scale": 25000})
        self.assertEqual(error.code, "layout.export_failed")
        self.assertEqual(gesture.items_placed, [])

    def test_defaults_keep_the_backend_arguments_unchanged(self):
        backend = FakeBackend(TWO)
        (result, error), _ = run(backend, {"title": "Carte", "out": self.out})
        self.assertIsNone(error)
        self.assertEqual(backend.kwargs, {})
        self.assertEqual(layout_export.page_dimensions(None), (297.0, 210.0))
        self.assertEqual(set(result), {"file", "page_count", "layer_count", "sources", "truncated", "source_details"})
        for extra in ({"page_size": None}, {"orientation": None}, {"grid": None}):
            backend = FakeBackend(TWO)
            (_, error), _ = run(backend, dict({"title": "Carte", "out": self.out}, **extra))
            self.assertIsNone(error)
            self.assertEqual(backend.kwargs, {}, extra)

    def test_a3_landscape_with_a_utm_km_grid(self):
        backend = FakeBackend(TWO)
        grid = {"kind": "projected", "epsg": 32631, "interval": {"value": 1, "unit": "km"}}
        (result, error), _ = run(backend, {"title": "Carte", "out": self.out, "page_size": "a3", "grid": grid})
        self.assertIsNone(error)
        self.assertEqual(
            backend.kwargs,
            {"page": ("a3", "landscape"), "grid": {"authid": "EPSG:32631", "interval": 1000.0, "projected": True}},
        )
        self.assertEqual(layout_export.page_dimensions(backend.kwargs["page"]), (420.0, 297.0))
        self.assertNotIn("grid", result)  # le résultat ne change pas
        self.assertTrue(os.path.isfile(self.out))

    def test_portrait_with_a_degree_graticule(self):
        backend = FakeBackend(TWO)
        grid = {"kind": "degrees", "interval": {"value": 0.01, "unit": "deg"}}
        params = {"title": "Carte", "out": self.out, "page_size": "a4", "orientation": "portrait", "grid": grid}
        (result, error), _ = run(backend, params)
        self.assertIsNone(error)
        self.assertEqual(backend.kwargs["page"], ("a4", "portrait"))
        self.assertEqual(layout_export.page_dimensions(backend.kwargs["page"]), (210.0, 297.0))
        self.assertEqual(backend.kwargs["grid"], {"authid": "EPSG:4326", "interval": 0.01, "projected": False})

    def test_orientation_alone_is_an_a4(self):
        backend = FakeBackend(TWO)
        (_, error), _ = run(backend, {"title": "Carte", "out": self.out, "orientation": "portrait"})
        self.assertIsNone(error)
        self.assertEqual(backend.kwargs, {"page": ("a4", "portrait")})
        self.assertEqual(layout_export.page_dimensions(("a0", "landscape")), (1189.0, 841.0))

    def test_grads_become_degrees(self):
        backend = FakeBackend(TWO)
        for interval, degrees in (({"value": 1, "unit": "gon"}, 0.9), ({"value": 0.5}, 0.5)):
            grid = {"kind": "degrees", "interval": interval}
            (_, error), _ = run(backend, {"title": "Carte", "out": self.out, "grid": grid})
            self.assertIsNone(error)
            self.assertAlmostEqual(backend.kwargs["grid"]["interval"], degrees)
            self.assertEqual(backend.kwargs["grid"]["authid"], "EPSG:4326")

    def test_values_outside_the_lists_are_refused_without_file(self):
        km = {"value": 1, "unit": "km"}
        for extra in (
            {"page_size": "a5"},
            {"page_size": "A4"},
            {"page_size": 4},
            {"orientation": "paysage"},
            {"orientation": True},
            {"grid": {"kind": "mgrs", "interval": km}},
            {"grid": {"interval": km}},
            {"grid": "utm"},
            {"grid": {"kind": "projected", "interval": km}},
            {"grid": {"kind": "projected", "epsg": "32631", "interval": km}},
            {"grid": {"kind": "projected", "epsg": True, "interval": km}},
            {"grid": {"kind": "projected", "epsg": 32631, "interval": {"value": 1, "unit": "deg"}}},
            {"grid": {"kind": "projected", "epsg": 32631, "interval": {"value": 1}}},
            {"grid": {"kind": "degrees", "interval": km}},
            {"grid": {"kind": "degrees", "epsg": 4326, "interval": {"value": 1}}},
            {"grid": {"kind": "degrees"}},
        ):
            backend = FakeBackend(TWO)
            (result, error), _ = run(backend, dict({"title": "Carte", "out": self.out}, **extra))
            self.assertIsNone(result, extra)
            self.assertEqual(error.code, "layout.bad_params", extra)
            self.assertIsNone(backend.exported, extra)
        self.assertFalse(os.path.exists(self.out))

    def test_a_null_or_negative_interval_is_refused(self):
        for interval in (
            {"value": 0, "unit": "km"},
            {"value": -1, "unit": "m"},
            {"value": float("nan"), "unit": "m"},
            {"value": "1", "unit": "m"},
        ):
            for grid in (
                {"kind": "projected", "epsg": 32631, "interval": interval},
                {"kind": "degrees", "interval": dict(interval, unit="deg")},
            ):
                backend = FakeBackend(TWO)
                (_, error), _ = run(backend, {"title": "Carte", "out": self.out, "grid": grid})
                self.assertEqual(error.code, "layout.bad_params", grid)
                self.assertIsNone(backend.exported)
        self.assertFalse(os.path.exists(self.out))

    def test_an_unknown_grid_system_is_refused_without_file(self):
        backend = FakeBackend(TWO)
        grid = {"kind": "projected", "epsg": 999999, "interval": {"value": 1, "unit": "km"}}
        (result, error), _ = run(backend, {"title": "Carte", "out": self.out, "grid": grid})
        self.assertIsNone(result)
        self.assertEqual(error.code, "data.unknown_crs")
        self.assertIn("999999", error.message)
        self.assertIsNone(backend.exported)
        self.assertFalse(os.path.exists(self.out))

    def test_a_grid_epsg_zero_is_an_unknown_system(self):
        backend = FakeBackend(TWO)
        grid = {"kind": "projected", "epsg": 0, "interval": {"value": 1, "unit": "km"}}
        (result, error), _ = run(backend, {"title": "Carte", "out": self.out, "grid": grid})
        self.assertIsNone(result)
        self.assertEqual(error.code, "data.unknown_crs")
        self.assertIsNone(backend.exported)
        self.assertFalse(os.path.exists(self.out))

    def test_a_too_dense_grid_is_refused_without_file(self):
        # Cadre de 25 km de large, écart de 124 m : 202 lignes, au-delà de 200 : refusé avant l'export.
        backend = FakeBackend(TWO, grid_frame=(0.0, 0.0, 25000.0, 10000.0))
        grid = {"kind": "projected", "epsg": 32631, "interval": {"value": 124, "unit": "m"}}
        (result, error), gesture = run(backend, {"title": "Carte", "out": self.out, "grid": grid})
        self.assertIsNone(result)
        self.assertEqual(error.code, "layout.bad_params")
        self.assertIn("200", error.message)
        self.assertFalse(os.path.exists(self.out))
        self.assertEqual(gesture.items_placed, [])
        # 125 m : 201 lignes (bords compris), refusé ; 126 m : 199 lignes, permis.
        grid["interval"]["value"] = 125
        (_, error), _ = run(backend, {"title": "Carte", "out": self.out, "grid": grid})
        self.assertEqual(error.code, "layout.bad_params")
        grid["interval"]["value"] = 126
        (_, error), _ = run(backend, {"title": "Carte", "out": self.out, "grid": grid})
        self.assertIsNone(error)

    def test_grid_density_helper(self):
        layout_export.check_grid_density((0.0, 0.0, 199.0, 199.0), 1.0)  # 200 lignes : permis
        for frame, interval in (
            ((0.0, 0.0, 200.0, 1.0), 1.0),  # 201 lignes
            ((0.0, 0.0, 2.01, 1.0), 0.01),
            (None, 1.0),
            ((0.0, 0.0, float("inf"), 1.0), 1.0),
        ):
            with self.assertRaises(layout_export.GridError):
                layout_export.check_grid_density(frame, interval)
        self.assertEqual(
            [layout_export._decimals(v) for v in (1000.0, 0.01, 0.9, 0.25, 1e-9)],
            [0, 2, 1, 2, 6],
        )

    def test_expected_items_are_the_five_plus_title(self):
        self.assertEqual(
            set(layout_export.EXPECTED_ITEMS),
            {"map", "title", "legend", "scalebar", "northarrow", "sources"},
        )



class LegendFitTest(unittest.TestCase):
    """Choix de l'essai qui fait tenir la légende dans sa colonne (`choose_legend_step`)."""

    @staticmethod
    def sizer(width, height):
        """Une légende de `width` × `height` mm en une colonne, texte entier, couches dépliées :
        les colonnes partagent la hauteur, la réduction joue sur les deux côtés, et replier les
        couches sur leur nom divise la hauteur par 4."""
        seen = []

        def measure(step):
            shrink, columns, collapsed = step
            seen.append(step)
            h = height * shrink / columns / (4 if collapsed else 1)
            return width * shrink * columns ** 0.5, h

        return measure, seen

    def test_a_small_legend_keeps_one_column_and_full_text(self):
        measure, seen = self.sizer(40, 80)
        self.assertEqual(layout_export.choose_legend_step(measure, 72, 173), (1.0, 1, False))
        self.assertEqual(len(seen), 1)

    def test_a_tall_legend_takes_columns_before_shrinking(self):
        measure, _ = self.sizer(30, 300)
        self.assertEqual(layout_export.choose_legend_step(measure, 72, 173), (1.0, 2, False))

    def test_layers_are_collapsed_only_when_no_full_legend_fits(self):
        measure, seen = self.sizer(30, 2000)
        step = layout_export.choose_legend_step(measure, 72, 173)
        self.assertTrue(step[2])
        full = [s for s in layout_export.LEGEND_STEPS if not s[2]]
        self.assertEqual(seen[: len(full)], full)  # chaque essai déplié avant de replier

    def test_the_chosen_step_never_overflows_when_one_fits(self):
        for height in (50, 200, 500, 1000, 2000, 4000):
            for width in (20, 40, 60):
                measure, _ = self.sizer(width, height)
                step = layout_export.choose_legend_step(measure, 72, 173)
                w, h = self.sizer(width, height)[0](step)
                fits = [s for s in layout_export.LEGEND_STEPS
                        if self.sizer(width, height)[0](s)[0] <= 72
                        and self.sizer(width, height)[0](s)[1] <= 173]
                if fits:
                    self.assertEqual(step, fits[0])
                    self.assertLessEqual(h, 173)
                    self.assertLessEqual(w, 72)

    def test_nothing_fits_leaves_the_smallest_step(self):
        measure, _ = self.sizer(500, 100000)
        self.assertEqual(
            layout_export.choose_legend_step(measure, 72, 173), layout_export.LEGEND_STEPS[-1]
        )
        self.assertEqual(layout_export.LEGEND_STEPS[-1], (0.4, 3, True))



class MgrsHeaderSourcesTest(unittest.TestCase):
    """Epic 10, entrée 31 : carroyage MGRS, en-tête et pied de page, sources détaillées."""

    MGRS = {"kind": "mgrs", "interval": {"value": 1, "unit": "km"}, "epsg": 32632, "squares": ["32T KR", "32T LR"]}

    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.out = os.path.join(self.dir.name, "3-layout.export_pdf.pdf")

    def tearDown(self):
        self.dir.cleanup()

    def params(self, **extra):
        return dict({"title": "Secteur de recherche", "out": self.out}, **extra)

    def test_the_mgrs_grid_goes_to_the_backend_with_the_core_squares(self):
        backend = FakeBackend(TWO)
        (result, error), _ = run(backend, self.params(grid=self.MGRS))
        self.assertIsNone(error)
        self.assertEqual(
            backend.kwargs["grid"],
            {"authid": "EPSG:32632", "interval": 1000.0, "projected": True, "mgrs": True,
             "squares": ["32T KR", "32T LR"]},
        )
        # 10 km et 100 km en mètres.
        for value, unit, metres in ((10, "km", 10000.0), (100000, "m", 100000.0)):
            backend = FakeBackend(TWO)
            grid = dict(self.MGRS, interval={"value": value, "unit": unit})
            (_, error), _ = run(backend, self.params(grid=grid))
            self.assertIsNone(error)
            self.assertEqual(backend.kwargs["grid"]["interval"], metres)

    def test_an_mgrs_grid_without_the_core_fields_or_at_another_interval_is_refused(self):
        bad = [
            {"kind": "mgrs", "interval": {"value": 1, "unit": "km"}},
            dict(self.MGRS, squares="32T LR"),
            dict(self.MGRS, epsg="32632"),
            dict(self.MGRS, interval={"value": 5, "unit": "km"}),
            dict(self.MGRS, zone="32T"),
        ]
        for grid in bad:
            backend = FakeBackend(TWO)
            (result, error), _ = run(backend, self.params(grid=grid))
            self.assertIsNone(result, grid)
            self.assertEqual(error.code, "layout.bad_params", grid)
            self.assertIsNone(backend.exported)
            self.assertFalse(os.path.exists(self.out))

    def test_the_mgrs_labels_are_a_fixed_format(self):
        # Une expression constante de QGIS, les deux chiffres du kilomètre ; aucun calcul MGRS.
        self.assertIn("@grid_number", layout_export.MGRS_LABEL_EXPRESSION)
        self.assertIn("% 100", layout_export.MGRS_LABEL_EXPRESSION)
        for word in ("square", "zone", "letter"):
            self.assertNotIn(word, layout_export.MGRS_LABEL_EXPRESSION)

    def test_header_and_footer_are_passed_as_written(self):
        backend = FakeBackend(TWO)
        (_, error), _ = run(backend, self.params(header="Document de travail", footer="Bureau Lac et Monts — confidentiel"))
        self.assertIsNone(error)
        self.assertEqual(backend.kwargs, {"header": "Document de travail", "footer": "Bureau Lac et Monts — confidentiel"})
        backend = FakeBackend(TWO)
        (_, error), _ = run(backend, self.params(header="  Pièce n° 4  "))
        self.assertEqual(backend.kwargs["header"], "  Pièce n° 4  ")

    def test_a_long_or_multiline_margin_text_is_refused(self):
        for extra in ({"header": "x" * 121}, {"footer": "deux\nlignes"}, {"header": "  "}, {"footer": 3},
                      {"header": "a\u2028b"}, {"footer": "a\tb"}):
            backend = FakeBackend(TWO)
            (result, error), _ = run(backend, self.params(**extra))
            self.assertEqual(error.code, "layout.bad_params", extra)
            self.assertIsNone(backend.exported)
        backend = FakeBackend(TWO)
        (_, error), _ = run(backend, self.params(header="é" * 120))
        self.assertIsNone(error)

    def test_source_details_give_resolution_and_date_with_their_origin(self):
        project = os.path.join(self.dir.name, "projet")
        layers = [
            FakeLayer("m", os.path.join(project, "mnt", "mnt_5m.tif"), (0.0, 0.0, 10.0, 10.0)),
            FakeLayer("c", os.path.join(project, "communes.gpkg") + "|layername=communes", (0.0, 0.0, 5.0, 5.0)),
            FakeLayer("p", os.path.join(project, "projet.cassini", "1-terrain.slope.tif"), (0.0, 0.0, 10.0, 10.0)),
            FakeLayer("z", os.path.join(project, "zonage.shp"), (0.0, 0.0, 5.0, 5.0)),
        ]
        backend = FakeBackend(layers)
        backend.project_folder, backend.project_stem = project, "projet"
        backend.resolutions = {"m": 5.0, "p": 5.0}
        backend.metadata_dates = {"z": "2024-05-01"}
        backend.file_dates = {
            os.path.join(project, "mnt", "mnt_5m.tif"): "2025-03-14",
            os.path.join(project, "communes.gpkg"): "2026-10-15",
            os.path.join(project, "projet.cassini", "1-terrain.slope.tif"): "2026-11-10",
            os.path.join(project, "zonage.shp"): "2026-01-02",
        }
        (result, error), _ = run(backend, self.params())
        self.assertIsNone(error)
        self.assertEqual(result["sources"], ["mnt_5m.tif", "communes.gpkg", "1-terrain.slope.tif", "zonage.shp"])
        self.assertEqual(result["source_details"], [
            {"source": "mnt_5m.tif", "resolution": {"value": 5.0, "unit": "m"}, "date": "2025-03-14", "date_origin": "file"},
            {"source": "communes.gpkg", "resolution": None, "date": "2026-10-15", "date_origin": "file"},
            # Sortie de Cassini, réécrite au rejeu : pas de date (P3).
            {"source": "1-terrain.slope.tif", "resolution": {"value": 5.0, "unit": "m"}, "date": None, "date_origin": None},
            # Les métadonnées priment sur la date du fichier.
            {"source": "zonage.shp", "resolution": None, "date": "2024-05-01", "date_origin": "metadata"},
        ])
        sources_text = backend.exported[2]
        self.assertEqual(
            sources_text,
            "Sources : mnt_5m.tif (5 m, fichier du 2025-03-14), communes.gpkg (fichier du 2026-10-15), "
            "1-terrain.slope.tif (5 m), zonage.shp (données du 2024-05-01)",
        )
        self.assertNotIn(self.dir.name, sources_text)

    def test_a_csv_uri_gives_its_decoded_file_path(self):
        self.assertEqual(
            layout_export._file_path("file:///C:/Mes%20donn%C3%A9es/gares.csv?type=csv&xField=X"),
            "C:/Mes données/gares.csv",
        )
        self.assertEqual(layout_export._file_path("file:///home/x/gares.csv?type=csv"), "/home/x/gares.csv")
        self.assertEqual(layout_export._file_path("/d/communes.gpkg|layername=communes"), "/d/communes.gpkg")
        self.assertIsNone(layout_export._file_path("memory?geometry=Point"))

    def test_a_decimal_resolution_is_written_with_a_comma(self):
        self.assertEqual(layout_export._metres_text(0.5), "0,5 m")
        self.assertEqual(layout_export._metres_text(25.0), "25 m")
        self.assertEqual(layout_export._metres_text(1.25), "1,25 m")


if __name__ == "__main__":
    unittest.main()
