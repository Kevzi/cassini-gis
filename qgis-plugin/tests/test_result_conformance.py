"""Conformité des résultats de l'extension aux schémas de sortie du noyau (suite de 10.1).

`fixtures/resultats-extension.json` est lu aussi par le noyau (`core/tests/conformite_resultats.rs`),
qui fait valider chaque exemple par le schéma de sortie de sa commande. Ici, chaque geste est rejoué
avec son faux backend (aucun QGIS) et son résultat doit avoir la forme de l'exemple : mêmes clés,
mêmes types JSON, jusque dans les listes et les objets imbriqués. Le schéma du noyau fait foi : un
écart se corrige dans le producteur Python.
"""

import json
import os
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "cassini_qgis"))
sys.path.insert(0, HERE)
EXAMPLES = os.path.join(HERE, "..", "..", "fixtures", "resultats-extension.json")

import analysis  # noqa: E402,F401
import inventory  # noqa: E402
import layer_add  # noqa: E402,F401
import layer_remove  # noqa: E402
import layout_export  # noqa: E402
import point_transform  # noqa: E402
import raster_output  # noqa: E402
import style_gestures  # noqa: E402
import vector_output  # noqa: E402

import test_analysis as ta  # noqa: E402
import test_inventory as ti  # noqa: E402
import test_layer_add as tla  # noqa: E402
import test_layer_remove as tlr  # noqa: E402
import test_layout_export as tle  # noqa: E402
import test_point_transform as tpt  # noqa: E402
import test_raster_output as tro  # noqa: E402
import test_style_gestures as tsg  # noqa: E402
import test_vector_output as tvo  # noqa: E402
import test_delegated as tdel  # noqa: E402


def id_set_errors(expected, found):
    """Un message par écart entre les commandes déléguées et les ids du fichier d'exemples."""
    errors = ["%s : commande déléguée sans exemple dans resultats-extension.json" % i for i in sorted(expected - found)]
    errors += [
        "%s : exemple d'une commande qui n'est pas déléguée à QGIS (ou inconnue)" % i
        for i in sorted(found - expected)
    ]
    return errors


def load_examples():
    with open(EXAMPLES, encoding="utf-8") as f:
        examples = json.load(f)
    # Garde-fou : un fichier vide ou mal lu ne doit pas faire passer la recette.
    assert len(examples) >= 20, "fichier d'exemples vide ou tronqué"
    return examples


def shape(value):
    """Forme JSON d'une valeur : types et clés, jusque dans les listes (un modèle par élément)."""
    if value is None:
        return "null"
    if isinstance(value, bool):
        return value  # un booléen porte son sens (`truncated`, `reprojected`) : comparé par valeur
    if isinstance(value, int):
        return "integer"
    if isinstance(value, float):
        return "number"
    if isinstance(value, str):
        return "string"
    if isinstance(value, dict):
        return {k: shape(v) for k, v in value.items()}
    if isinstance(value, (list, tuple)):
        return [shape(v) for v in value]
    return "autre:" + type(value).__name__


def mismatch(actual, expected, path="$"):
    """Premier écart de forme de `actual` par rapport à `expected`, ou None.

    Un entier est admis là où l'exemple donne un nombre ; une liste se compare élément par élément
    à ceux de l'exemple (chacun doit avoir la forme d'un des éléments de l'exemple)."""
    if isinstance(expected, dict):
        if not isinstance(actual, dict):
            return "%s : un objet était attendu, %r" % (path, actual)
        if set(actual) != set(expected):
            return "%s : clés %s au lieu de %s" % (path, sorted(actual), sorted(expected))
        for key in expected:
            found = mismatch(actual[key], expected[key], path + "." + key)
            if found:
                return found
        return None
    if isinstance(expected, list):
        if not isinstance(actual, list):
            return "%s : une liste était attendue" % path
        if actual and not expected:
            return "%s : liste non vide, l'exemple est vide" % path
        for i, item in enumerate(actual):
            if not any(mismatch(item, model, "%s[%d]" % (path, i)) is None for model in expected):
                return "%s[%d] : aucune forme de l'exemple ne convient" % (path, i)
        return None
    if isinstance(expected, bool):
        if actual is not expected:
            return "%s : %r au lieu de %r" % (path, actual, expected)
        return None
    if actual == "autre" or isinstance(actual, (dict, list)):
        return "%s : type %r au lieu de %r" % (path, shape(actual), expected)
    got = shape(actual)
    if got == expected or (got == "integer" and expected == "number"):
        return None
    return "%s : type %s au lieu de %s" % (path, got, expected)


# -- producteurs : chaque geste rejoué avec son faux backend, un par exemple -----------------------


def _analysis(test_class, setup):
    """Rejoue un geste générique d'analyse ; `setup(t)` rend (backend, paramètres)."""
    t = test_class("setUp")
    t.setUp()
    try:
        backend, params = setup(t)
        _, calls = ta.start(t.entry, backend, params)
        for _ in range(4):  # une chaîne à plusieurs passes : la dernière seule répond
            if calls:
                break
            backend.finished(True, {"OUTPUT": "temporaire"})
        assert len(calls) == 1 and calls[0][1] is None, calls
        return calls[0][0]
    finally:
        t.tearDown()


def _buffer():
    return _analysis(ta.BufferTest, lambda t: (t.backend(), t.params()))


def _intersect():
    return _analysis(ta.IntersectTest, lambda t: (t.backend, t.params()))


def _select():
    return _analysis(ta.SelectByAttributeTest, lambda t: (t.backend, t.params()))


def _count_points(pairs, loaded_count, weight=None):
    def setup(t):
        t.backend.pairs = pairs
        t.backend.loaded_count = loaded_count
        if weight:
            t.backend.sums = {label: 41.5 for label, _ in pairs}
            return t.backend, t.params(weight_field=weight)
        return t.backend, t.params()

    return lambda: _analysis(ta.CountPointsInPolygonTest, setup)


def _dissolve():
    t = ta.DissolveTest("setUp")
    t.setUp()
    try:
        _, calls = ta.start(t.entry, t.backend, t.params())
        t.backend.finished(True, {"OUTPUT": "fusion"})
        assert calls[0][1] is None, calls
        return calls[0][0]
    finally:
        t.tearDown()


def _join_table():
    t = ta.JoinTableTest("setUp")
    t.setUp()
    try:
        _, calls = ta.start(t.entry, t.backend, t.params())
        t.backend.finished(True, {"OUTPUT": "jointe", "UNJOINABLE_COUNT": 1})
        assert calls[0][1] is None, calls
        return calls[0][0]
    finally:
        t.tearDown()


def _nearest():
    t = ta.NearestTest("setUp")
    t.setUp()
    try:
        _, calls = ta.start(t.entry, t.backend, t.params())
        t.backend.finished(True, {"OUTPUT": "jointe"})
        if len(t.backend.calls) > 1:
            t.backend.finished(True, {})
        assert calls[0][1] is None, calls
        return calls[0][0]
    finally:
        t.tearDown()


def _statistics(table, summed=False, params=None):
    def setup(t):
        t.backend.table = table
        extra = {"sum_field": "surface"} if summed else {}
        return t.backend, t.params(**extra)

    return lambda: _analysis(ta.StatisticsByCategoryTest, setup)


def _line_lengths(unit):
    def setup(t):
        t.backend.numbers = [("Nord", 6206.4), ("Sud", 3885.5), ("Vide", 0.0)][: 3 if unit == "m" else 2]
        t.backend.loaded_count = len(t.backend.numbers)
        return t.backend, t.params(unit=unit)

    return lambda: _analysis(ta.SumLineLengthsTest, setup)


def _isochrone():
    t = ta.IsochroneTest("setUp")
    t.setUp()
    try:
        _, calls = ta.start(t.entry, t.backend, t.params())
        t.backend.finished(True, {"OUTPUT_LINES": "tmp_1"})
        t.backend.finished(True, {"OUTPUT": t.out})
        assert calls[0][1] is None, calls
        return calls[0][0]
    finally:
        t.tearDown()


def _reproject():
    t = ta.ReprojectTest("setUp")
    t.setUp()
    try:
        backend = t.backend()
        _, calls = ta.start(t.entry, backend, t.params())
        backend.finished(True, {})
        assert calls[0][1] is None, calls
        return calls[0][0]
    finally:
        t.tearDown()


def _convert():
    t = ta.ConvertTest("setUp")
    t.setUp()
    try:
        backend = t.backend()
        _, calls = ta.start(t.entry, backend, t.params("gpkg"))
        backend.finished(True, {})
        assert calls[0][1] is None, calls
        return calls[0][0]
    finally:
        t.tearDown()


def _convert_raster(written_epsg):
    def produce():
        t = ta.ConvertRasterTest("setUp")
        t.setUp()
        try:
            backend = t.backend(written_epsg=written_epsg)
            _, calls = ta.start(t.entry, backend, t.params())
            backend.finished(True, {})
            assert calls[0][1] is None, calls
            return calls[0][0]
        finally:
            t.tearDown()

    return produce


def _transform_point():
    answers = []
    gesture = point_transform.PointTransformGesture(tpt.FakeBackend(), lambda fn: fn())
    gesture.start(tpt.WGS84_ORIGIN, lambda result=None, error=None: answers.append((result, error)))
    (result, error), = answers
    assert error is None, error
    return result


def _layer_add(crs):
    def produce():
        result, error = tla.run(tla.FakeBackend(crs=crs), tla.FIXTURE)
        assert error is None, error
        return result

    return produce


def _layer_add_table():
    result, error = tla.run_params(tla.FakeBackend(crs=""), {"table": "prix.csv"})
    assert error is None, error
    return result


def _arrange():
    result, error = tsg.arrange(tsg.project(), tsg.ArrangeTest.GROUPS)
    assert error is None, error
    return result


def _describe(layer):
    def produce():
        result, error = ti.run(inventory.LayerDescribeGesture, ti.FakeBackend([layer]), {"layer_id": layer.lid})
        assert error is None, error
        return result

    return produce


def _open_raster():
    with tempfile.TemporaryDirectory() as d:
        path = os.path.join(d, "1-terrain.slope.tif")
        with open(path, "wb") as f:
            f.write(b"II*\x00")
        gesture = raster_output.LayerOpenRasterGesture(tro.FakeBackend(), schedule=lambda fn: fn())
        result, error = tro.run(gesture, {"path": path})
    assert error is None, error
    return result


def _open_vector():
    with tempfile.TemporaryDirectory() as d:
        path = os.path.join(d, "1-terrain.peaks.geojson")
        with open(tvo.FIXTURE, "rb") as src, open(path, "wb") as f:
            f.write(src.read())
        gesture = vector_output.LayerOpenVectorGesture(tvo.FakeBackend(), schedule=lambda fn: fn())
        result, error = tvo.run(gesture, {"path": path})
    assert error is None, error
    return result


def _remove():
    backend = tlr.FakeBackend({"routes_1": ("Routes", "routes.gpkg")})
    gesture = layer_remove.LayerRemoveGesture(backend, schedule=lambda fn: fn())
    result, error = tlr.run(gesture, {"layer_id": "routes_1"})
    assert error is None, error
    return result


def _set_style():
    result, error = tsg.set_style(tsg.project(), "plu", {"kind": "single", "fill": "#ffffff"})
    assert error is None, error
    return result


def _export(backend_setup, extra=None, layers=None):
    def produce():
        with tempfile.TemporaryDirectory() as d:
            out = os.path.join(d, "1-layout.export_pdf.pdf")
            backend = tle.FakeBackend(layers if layers is not None else tle.TWO)
            backend_setup(backend, d)
            params = {"title": "Carte", "out": out}
            params.update(extra or {})
            (result, error), _ = tle.run(backend, params)
        assert error is None, error
        return result

    return produce


def _no_setup(backend, directory):
    return None


def _scaled_setup(backend, directory):
    backend.center = (5.0, 10.0)
    backend.resolutions = {"m": 10.0}
    backend.metadata_dates = {"g": "2025-12-01"}
    backend.file_dates = {"/d/mnt.tif": "2026-03-14"}


def _warnings_setup(backend, directory):
    backend.warnings = ["Un résultat n'a pas pu être mis en évidence."]


def _inventory(layers, **kw):  # noqa: D103
    def produce():
        result, error = ti.run(inventory.ProjectInventoryGesture, ti.FakeBackend(layers, **kw), {})
        assert error is None, error
        return result

    return produce


def _inventory_full():
    plu = ti.FakeLayer(
        "plu_1", "Zonage du PLU", "plu.gpkg", "EPSG:2154", (0.0, 1.0, 10.0, 11.0),
        fields=[("typezone", "String"), ("surface", "Real")], count=3,
        values={"typezone": ["U", "N", "Nh"]}, lonlat=(6.0, 45.8, 6.2, 45.95),
    )
    mnt = ti.FakeLayer("mnt_1", "MNT", "/home/k/proj/dem/mnt.tif", "EPSG:2154", None, vector=False)

    class WithUnits(ti.FakeBackend):
        def crs_unit_of(self, layer):
            return "m"

        def project_crs_unit(self):
            return "m"

    result, error = ti.run(
        inventory.ProjectInventoryGesture,
        WithUnits([plu, mnt], project_name="projet-test", project_dir="/home/k/proj"),
        {},
    )
    assert error is None, error
    return result


def _many_sources():
    layers = [tle.FakeLayer(str(i), "/d/f%d.shp" % i, (0.0, 0.0, 1.0, 1.0)) for i in range(60)]
    return _export(_no_setup, layers=layers)()


def producers():
    gares = ti.FakeLayer(
        "a_1", "gares", "gares.gpkg", "EPSG:2154", (0.0, 1.0, 10.0, 11.0),
        fields=[("nom", "String"), ("pop", "Integer64")], count=8, geom="Point",
    )
    vide = ti.FakeLayer("e", "vide", "vide.gpkg", "EPSG:2154", None, geom="Point")
    scaled_layers = [
        tle.FakeLayer("m", "/d/mnt.tif", (0.0, 0.0, 10.0, 20.0)),
        tle.FakeLayer("g", "/d/gares.gpkg", (0.0, 0.0, 10.0, 20.0)),
    ]
    return {
        ("analysis.buffer", "couche créée"): _buffer,
        ("analysis.intersect", "couche créée"): _intersect,
        ("analysis.isochrone", "un polygone"): _isochrone,
        ("analysis.dissolve", "couche créée"): _dissolve,
        ("analysis.join_table", "une entité sans ligne"): _join_table,
        ("analysis.nearest", "couche créée"): _nearest,
        ("analysis.select_by_attribute", "entités retenues"): _select,
        ("analysis.statistics_by_category", "comptes seuls"): _statistics(
            (["zonage", "count"], [["U", "5"], ["N", "3"]], 2)
        ),
        ("analysis.statistics_by_category", "avec somme"): _statistics(
            (ta.STATS_HEADER, [["U", "5", "5", "1", "9", "8", "41.5", "8"], ["N", "3", "3", "1", "9", "8", "12", "4"]], 2),
            summed=True,
        ),
        ("analysis.statistics_by_category", "tronqué"): _statistics(
            (["zonage", "count"], [[str(i), "1"] for i in range(150)], 150)
        ),
        ("analysis.sum_line_lengths", "en mètres"): _line_lengths("m"),
        ("analysis.sum_line_lengths", "en kilomètres"): _line_lengths("km"),
        ("analysis.count_points_in_polygon", "avec zones"): _count_points([("Nord", 5), ("Sud", 3), ("Vide", 0)], 3),
        ("analysis.count_points_in_polygon", "pondéré"): _count_points([("Nord", 5), ("Sud", 3)], 2, "poids"),
        ("analysis.count_points_in_polygon", "tronqué"): _count_points([(str(i), i) for i in range(150)], 150),
        ("analysis.count_points_in_polygon", "sans zone"): _count_points([], 0),
        ("data.convert", "GeoPackage"): _convert,
        ("data.convert_raster", "avec EPSG"): _convert_raster(2154),
        ("data.convert_raster", "sans EPSG"): _convert_raster(None),
        ("data.reproject", "couche créée"): _reproject,
        ("data.transform_point", "point reprojeté"): _transform_point,
        ("layer.add", "reprojetée à l'affichage"): _layer_add("EPSG:4326"),
        ("layer.add", "non reprojetée"): _layer_add("EPSG:2154"),
        ("layer.add", "tableau sans géométrie"): _layer_add_table,
        ("layer.arrange", "trois groupes"): _arrange,
        ("layer.describe", "avec emprise"): _describe(gares),
        ("layer.describe", "sans emprise"): _describe(vide),
        ("layer.open_raster", "raster ouvert"): _open_raster,
        ("layer.open_vector", "vecteur ouvert"): _open_vector,
        ("layer.remove", "couche retirée"): _remove,
        ("layer.set_style", "couche mise en forme"): _set_style,
        ("layout.export_pdf", "sans échelle"): _export(_no_setup),
        ("layout.export_pdf", "avec échelle et centre"): _export(
            _scaled_setup, {"scale": 25000}, layers=scaled_layers
        ),
        ("layout.export_pdf", "avec avertissements"): _export(_warnings_setup),
        ("layout.export_pdf", "tronqué"): _many_sources,
        ("project.inventory", "projet vide"): _inventory([], project_name=""),
        ("project.inventory", "vecteur et raster"): _inventory_full,
    }


class ResultConformanceTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.examples = load_examples()
        cls.by_key = {(e["id"], e["case"]): e["result"] for e in cls.examples}

    def test_every_delegated_command_has_an_example(self):
        expected = tdel.delegated_ids(tdel.load_fiches())
        self.assertGreaterEqual(len(expected), 15)  # garde-fou : un catalogue mal lu serait vide
        self.assertEqual(id_set_errors(expected, {e["id"] for e in self.examples}), [])

    def test_every_example_has_a_producer_and_every_producer_an_example(self):
        made = set(producers())
        self.assertEqual(sorted(set(self.by_key) - made), [], "exemple que aucun geste ne rejoue")
        self.assertEqual(sorted(made - set(self.by_key)), [], "geste rejoué sans exemple")

    def test_each_gesture_produces_the_shape_of_its_example(self):
        for (command_id, case), produce in sorted(producers().items()):
            with self.subTest(command=command_id, case=case):
                actual = produce()
                found = mismatch(actual, shape(self.by_key[(command_id, case)]))
                self.assertIsNone(found, "%s / %s : %s (produit : %r)" % (command_id, case, found, shape(actual)))

    def test_the_comparison_detects_a_difference(self):
        # Un cas négatif : une comparaison qui accepterait tout ne prouverait rien.
        model = {"layer_id": "a", "name": "b", "feature_count": 1, "crs": "EPSG:2154"}
        expected = shape(model)
        self.assertIsNone(mismatch(dict(model), expected))
        missing = {k: v for k, v in model.items() if k != "crs"}
        self.assertIsNotNone(mismatch(missing, expected))
        self.assertIsNotNone(mismatch(dict(model, extra=1), expected))
        self.assertIsNotNone(mismatch(dict(model, feature_count="1"), expected))
        self.assertIsNotNone(mismatch(dict(model, crs=None), expected))
        zone = shape({"zones": [{"label": "a"}]})
        self.assertIsNotNone(mismatch({"zones": [{"label": 1}]}, zone))
        self.assertIsNotNone(mismatch({"zones": [{"label": "a"}]}, shape({"zones": []})))
        self.assertIsNone(mismatch({"zones": []}, zone))

    def test_a_command_without_example_or_an_unknown_one_is_named(self):
        expected = tdel.delegated_ids(tdel.load_fiches())
        found = {e["id"] for e in self.examples}
        errors = id_set_errors(expected, found - {"layer.arrange"})
        self.assertEqual(len(errors), 1)
        self.assertIn("layer.arrange", errors[0])
        errors = id_set_errors(expected, found | {"analysis.fake"})
        self.assertEqual(len(errors), 1)
        self.assertIn("analysis.fake", errors[0])

    def test_an_inverted_boolean_is_rejected(self):
        expected = shape({"truncated": True, "reprojected": False})
        self.assertIsNone(mismatch({"truncated": True, "reprojected": False}, expected))
        self.assertIsNotNone(mismatch({"truncated": False, "reprojected": False}, expected))
        self.assertIsNotNone(mismatch({"truncated": True, "reprojected": True}, expected))
        self.assertIsNotNone(mismatch({"truncated": 1, "reprojected": False}, expected))


if __name__ == "__main__":
    unittest.main()
