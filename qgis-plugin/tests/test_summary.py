"""Tests sans QGIS des phrases du journal explicite : ce que chaque étape a fait."""

import os
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "cassini_qgis"))

import summary  # noqa: E402
import texts  # noqa: E402

NAMES = {"gares_1": "Gares", "parcelles_2": "Parcelles", "tampon_3": "Tampon"}


def qgis_name(layer_id):
    return NAMES.get(layer_id)


def say(command, params, result=None, lang="fr", step_names=None, catalogue=None):
    return summary.sentence(command, params, result, lang, qgis_name, step_names, catalogue)


BUFFER = {"layer_id": "gares_1", "distance": {"value": 500, "unit": "m"}}
BUFFER_OUT = {"layer_id": "tampon_3", "name": "Tampon", "feature_count": 8, "crs": "EPSG:2154"}


class LayerCommandsTest(unittest.TestCase):
    def test_buffer_matches_the_demo_sentence(self):
        self.assertEqual(
            say("analysis.buffer", BUFFER, BUFFER_OUT),
            "Zone tampon de 500 m autour de Gares : nouvelle couche Tampon, 8 entités.",
        )
        self.assertEqual(
            say("analysis.buffer", BUFFER, BUFFER_OUT, lang="en"),
            "Buffer of 500 m around Gares: new layer Tampon, 8 features.",
        )

    def test_singular_and_plural(self):
        one = dict(BUFFER_OUT, feature_count=1)
        zero = dict(BUFFER_OUT, feature_count=0)
        self.assertTrue(say("analysis.buffer", BUFFER, one).endswith("1 entité."))
        self.assertTrue(say("analysis.buffer", BUFFER, zero).endswith("0 entité."))
        self.assertTrue(say("analysis.buffer", BUFFER, one, lang="en").endswith("1 feature."))
        self.assertTrue(say("analysis.buffer", BUFFER, zero, lang="en").endswith("0 features."))

    def test_decimal_distance_and_dissolve(self):
        params = {"layer_id": "gares_1", "distance": {"value": 0.5, "unit": "km"}, "dissolve": True}
        self.assertEqual(say("analysis.buffer", params), "Zone tampon de 0,5 km autour de Gares, fusionnée.")
        self.assertEqual(say("analysis.buffer", params, lang="en"), "Buffer of 0.5 km around Gares, dissolved.")

    def test_without_result_only_parameters(self):
        self.assertEqual(say("analysis.buffer", BUFFER), "Zone tampon de 500 m autour de Gares.")
        self.assertEqual(say("analysis.buffer", BUFFER, "illisible"), "Zone tampon de 500 m autour de Gares.")
        self.assertEqual(
            say("analysis.buffer", BUFFER, {"name": "Tampon"}), "Zone tampon de 500 m autour de Gares."
        )

    def test_intersect(self):
        params = {"layer_id": "parcelles_2", "overlay_layer_id": "tampon_3"}
        out = {"layer_id": "x_4", "name": "Parcelles retenues", "feature_count": 42}
        self.assertEqual(
            say("analysis.intersect", params, out),
            "Intersection de Parcelles avec Tampon : nouvelle couche Parcelles retenues, 42 entités.",
        )

    def test_select_by_attribute_operators(self):
        params = {"layer_id": "parcelles_2", "field": "zone", "operator": "begins_with", "value": "U"}
        self.assertEqual(
            say("analysis.select_by_attribute", params),
            "Sélection dans Parcelles des entités dont le champ zone commence par « U ».",
        )
        for op in ("eq", "ne", "lt", "lte", "gt", "gte", "contains", "begins_with"):
            for lang in ("fr", "en"):
                text = say("analysis.select_by_attribute", dict(params, operator=op), lang=lang)
                self.assertIn(" %s « U »" % texts.tr(lang, "sum_op_" + op) if lang == "fr"
                              else ' %s "U"' % texts.tr(lang, "sum_op_" + op), text)

    def test_isochrone(self):
        params = {
            "layer_id": "routes_9",
            "origin": {"x": 1, "y": 2, "epsg": 2154},
            "mode": "walk",
            "duration": {"value": 10, "unit": "min"},
        }
        self.assertEqual(
            say("analysis.isochrone", params),
            "Zone accessible en 10 min à pied sur le réseau routes_9.",  # repli : l'identifiant
        )
        self.assertEqual(
            say("analysis.isochrone", dict(params, mode="car"), lang="en"),
            "Area reachable in 10 min by car on the network routes_9.",
        )

    def test_zonal_statistics(self):
        params = {"layer_id": "parcelles_2", "points_layer_id": "gares_1"}
        out = {
            "layer_id": "z_5",
            "name": "Décompte",
            "feature_count": 3,
            "zones": [{"label": "A", "count": 1}, {"label": "B", "count": 4}],
            "truncated": False,
        }
        self.assertEqual(
            say("analysis.zonal_statistics", params, out),
            "Décompte des points de Gares dans chaque zone de Parcelles : nouvelle couche "
            "Décompte, 3 entités ; zone la plus fournie : B, 4 points.",
        )


class OtherCommandsTest(unittest.TestCase):
    def test_layer_add_shows_only_the_file_name(self):
        params = {"path": "C:\\Users\\camille\\donnees\\gares.gpkg"}
        out = {"layer_id": "g_1", "name": "Gares", "source_crs": "EPSG:4326",
               "project_crs": "EPSG:2154", "reprojected": True}
        text = say("layer.add", params, out)
        self.assertEqual(
            text,
            "Fichier gares.gpkg ouvert : couche Gares, reprojetée de EPSG:4326 vers EPSG:2154.",
        )
        self.assertEqual(say("layer.add", {"path": "/home/c/gares.gpkg"}), "Ouverture du fichier gares.gpkg.")

    def test_layer_describe(self):
        out = {"name": "Gares", "fields": [{"name": "nom"}], "feature_count": 12, "geometry": "Point"}
        self.assertEqual(
            say("layer.describe", {"layer_id": "gares_1"}, out),
            "Description de Gares : 12 entités, 1 champ, géométrie Point.",
        )

    def test_export_pdf_never_shows_an_absolute_path(self):
        out = {"file": "/home/camille/projet/cassini/2-layout.export_pdf.pdf", "page_count": 1}
        text = say("layout.export_pdf", {"title": "Parcelles près des gares"}, out)
        self.assertEqual(
            text,
            "Carte « Parcelles près des gares » exportée en PDF : fichier "
            "2-layout.export_pdf.pdf, 1 page.",
        )
        self.assertNotIn("/home", text)
        windows = dict(out, file="C:\\Projets\\out\\2-layout.export_pdf.pdf", page_count=2)
        text = say("layout.export_pdf", {"title": "T"}, windows, lang="en")
        self.assertEqual(text, 'Map "T" exported to PDF: file 2-layout.export_pdf.pdf, 2 pages.')

    def test_export_pdf_names_the_scale_held(self):
        out = {"file": "/home/c/3-layout.export_pdf.pdf", "page_count": 1, "scale": 25000}
        self.assertEqual(
            say("layout.export_pdf", {"title": "Gares", "scale": 25000}, out),
            "Carte « Gares » exportée en PDF au 1:25 000 : fichier 3-layout.export_pdf.pdf, 1 page.",
        )
        self.assertEqual(
            say("layout.export_pdf", {"title": "Gares", "scale": 1250000}, dict(out, scale=1250000), lang="en"),
            'Map "Gares" exported to PDF at 1:1,250,000: file 3-layout.export_pdf.pdf, 1 page.',
        )
        # Sans échelle dans le résultat, rien n'est annoncé ; une valeur illisible non plus.
        for result in (dict(out, scale=None), dict(out, scale="25000"), dict(out, scale=True)):
            self.assertNotIn("1:", say("layout.export_pdf", {"title": "Gares"}, result))
        self.assertEqual(
            say("layout.export_pdf", {"title": "Gares", "scale": 500}, {"file": "a.pdf", "scale": 500}),
            "Carte « Gares » exportée en PDF au 1:500 : fichier a.pdf.",
        )
        # Résultat absent : l'échelle des paramètres est dite quand même.
        self.assertEqual(
            say("layout.export_pdf", {"title": "Gares", "scale": 25000}, None),
            "Carte « Gares » exportée en PDF au 1:25 000.",
        )
        self.assertNotIn("1:", say("layout.export_pdf", {"title": "Gares", "scale": None}, None))
        # Résultat sans échelle lisible : celle des paramètres est dite.
        for result in ({"file": "a.pdf"}, {"file": "a.pdf", "scale": None}):
            self.assertEqual(
                say("layout.export_pdf", {"title": "Gares", "scale": 25000}, result),
                "Carte « Gares » exportée en PDF au 1:25 000 : fichier a.pdf.",
            )

    def test_inventory_models_and_about(self):
        self.assertEqual(
            say("project.inventory", {}, {"layers": [{}, {}]}), "Inventaire du projet : 2 couches."
        )
        self.assertEqual(
            say("model.list", {}, {"folder": "/x", "models": [{"file": "q.gguf"}]}, lang="en"),
            "Search for local models: 1 model.",
        )
        self.assertEqual(
            say("system.about", {}, {}), "Le noyau Cassini répond et son protocole est compatible."
        )


CONVERT = {"mgrs": "31UDQ4825111750", "to": ["mgrs", "utm"]}
CONVERT_OUT = {
    "mgrs": "31UDQ4825111750",
    "utm": {"x": 448251.0, "y": 5411750.6, "epsg": 32631},
}


class CoordConvertTest(unittest.TestCase):
    def test_the_rendered_forms_are_shown_in_the_asked_order(self):
        self.assertEqual(
            say("coord.convert", CONVERT, CONVERT_OUT),
            "Point converti : en MGRS : 31UDQ4825111750 ; en UTM 31N : 448251 E, 5411750 N.",
        )
        self.assertEqual(
            say("coord.convert", CONVERT, CONVERT_OUT, lang="en"),
            "Point converted: in MGRS: 31UDQ4825111750; in UTM 31N: 448251 E, 5411750 N.",
        )

    def test_wgs84_shows_latitude_then_longitude_with_hemispheres(self):
        params = {"point": {"x": -3, "y": -1, "epsg": 4326}, "to": ["wgs84", "utm"]}
        result = {
            "wgs84": {"x": -3.0, "y": -1.0, "epsg": 4326},
            "utm": {"x": 500000.0, "y": 9889469.841, "epsg": 32730},
        }
        self.assertEqual(
            say("coord.convert", params, result),
            "Point converti : en WGS84 : 1,0000000° S, 3,0000000° O ; "
            "en UTM 30S : 500000 E, 9889469 N.",
        )
        self.assertEqual(
            say("coord.convert", params, result, lang="en"),
            "Point converted: in WGS84: 1.0000000° S, 3.0000000° W; "
            "in UTM 30S: 500000 E, 9889469 N.",
        )

    def test_without_result_only_the_asked_forms(self):
        self.assertEqual(
            say("coord.convert", CONVERT), "Conversion du point vers MGRS, UTM."
        )
        self.assertEqual(
            say("coord.convert", CONVERT, {}, lang="en"), "Conversion of the point to MGRS, UTM."
        )

    def test_unreadable_values_fall_back(self):
        catalogue = [{"id": "coord.convert", "description_fr": "Convertit un point."}]
        for params, result in [
            ({"to": ["lambert"]}, CONVERT_OUT),
            ({"to": "mgrs"}, CONVERT_OUT),
            (CONVERT, {"mgrs": "31UDQ", "utm": {"x": 1, "y": 2, "epsg": 2154}}),
        ]:
            self.assertEqual(
                say("coord.convert", params, result, catalogue=catalogue),
                "Convertit un point. Terminée.",
            )


MEASURE = {
    "from": {"x": 2.3522, "y": 48.8566, "epsg": 4326},
    "to": {"x": 5.3698, "y": 43.2965, "epsg": 4326},
}
MEASURE_OUT = {
    "distance": {"value": 660503.512, "unit": "m"},
    "azimuth": {"value": 157.957382201, "unit": "deg"},
    "back_azimuth": {"value": 340.1, "unit": "deg"},
}
DESTINATION = {
    "from": {"x": 448251.0, "y": 5411750.0, "epsg": 32631},
    "azimuth": {"value": 100, "unit": "gon"},
    "distance": {"value": 1.5, "unit": "km"},
}
DESTINATION_OUT = {"point": {"x": 449750.981, "y": 5411751.338, "epsg": 32631}}
INTERSECTION = {
    "first": {"x": 2.0, "y": 48.0, "epsg": 4326},
    "first_azimuth": {"value": 45},
    "second": {"x": 2.2, "y": 48.0, "epsg": 4326},
    "second_azimuth": {"value": 315},
}
INTERSECTION_OUT = {
    "point": {"x": 2.1, "y": 48.047609571, "epsg": 4326},
    "distance_from_first": {"value": 7469.237, "unit": "m"},
    "distance_from_second": {"value": 7469.237, "unit": "m"},
}


class GeoTest(unittest.TestCase):
    def test_distance_and_azimuths_are_shown(self):
        self.assertEqual(
            say("geo.distance_azimuth", MEASURE, MEASURE_OUT),
            "Distance géodésique : 660,504 km, azimut 157,9574°, contre-azimut 340,1°.",
        )
        self.assertEqual(
            say("geo.distance_azimuth", MEASURE, MEASURE_OUT, lang="en"),
            "Geodesic distance: 660.504 km, azimuth 157.9574°, back azimuth 340.1°.",
        )
        grads = dict(MEASURE_OUT, distance={"value": 39.528, "unit": "m"},
                     azimuth={"value": 61.9, "unit": "gon"},
                     back_azimuth={"value": 261.9, "unit": "gon"})
        self.assertEqual(
            say("geo.distance_azimuth", MEASURE, grads),
            "Distance géodésique : 39,528 m, azimut 61,9 gon, contre-azimut 261,9 gon.",
        )
        self.assertEqual(
            say("geo.distance_azimuth", MEASURE),
            "Mesure de la distance et de l'azimut entre deux points.",
        )

    def test_the_reached_point_is_shown(self):
        self.assertEqual(
            say("geo.destination_point", DESTINATION, DESTINATION_OUT),
            "Point atteint à 1,5 km, azimut 100 gon : UTM 31N 449750 E, 5411751 N.",
        )
        self.assertEqual(
            say("geo.destination_point", DESTINATION, {}, lang="en"),
            "Point at 1.5 km, azimuth 100 gon.",
        )
        wgs84 = {"point": {"x": -3.5, "y": 48.25, "epsg": 4326}}
        self.assertEqual(
            say("geo.destination_point", DESTINATION, wgs84, lang="en"),
            "Point reached at 1.5 km, azimuth 100 gon: 48.2500000° N, 3.5000000° W.",
        )

    def test_the_intersection_is_shown_with_its_distances(self):
        self.assertEqual(
            say("geo.bearing_intersection", INTERSECTION, INTERSECTION_OUT),
            "Recoupement des visées 45° et 315° : 48,0476096° N, 2,1000000° E, "
            "à 7,469 km de la première station et 7,469 km de la seconde.",
        )
        self.assertEqual(
            say("geo.bearing_intersection", INTERSECTION, None, lang="en"),
            "Intersection of the bearings 45° and 315°.",
        )

    def test_unreadable_values_fall_back(self):
        catalogue = [
            {"id": "geo.destination_point", "description_fr": "Point atteint."},
            {"id": "geo.distance_azimuth", "description_fr": "Distance et azimuts."},
        ]
        bad_point = {"point": {"x": 1, "y": 2, "epsg": 2154}}
        self.assertEqual(
            say("geo.destination_point", DESTINATION, bad_point, catalogue=catalogue),
            "Point atteint. Terminée.",
        )
        self.assertEqual(
            say("geo.destination_point", dict(DESTINATION, azimuth={"value": 1, "unit": "rad"}),
                catalogue=catalogue),
            "Point atteint. Terminée.",
        )
        self.assertEqual(
            say("geo.distance_azimuth", MEASURE, dict(MEASURE_OUT, distance=12),
                catalogue=catalogue),
            "Distance et azimuts. Terminée.",
        )


TRANSFORM = {"point": {"x": 3, "y": 46.5, "epsg": 4326}, "target_epsg": 2154}
TRANSFORM_OUT = {"point": {"x": 700000.0000004, "y": 6599999.99961, "epsg": 2154}}


REPROJECT = {"layer_id": "gares_1", "target_epsg": 2154}
REPROJECT_OUT = {"layer_id": "r_5", "name": "1-data.reproject", "feature_count": 5, "crs": "EPSG:2154"}


class ReprojectTest(unittest.TestCase):
    def test_the_new_layer_is_named_with_its_code(self):
        self.assertEqual(
            say("data.reproject", REPROJECT, REPROJECT_OUT),
            "Couche Gares reprojetée en EPSG:2154 : nouvelle couche 1-data.reproject, 5 entités.",
        )
        self.assertEqual(
            say("data.reproject", REPROJECT, REPROJECT_OUT, lang="en"),
            "Layer Gares reprojected to EPSG:2154: new layer 1-data.reproject, 5 features.",
        )

    def test_without_result_only_parameters(self):
        self.assertEqual(say("data.reproject", REPROJECT), "Couche Gares reprojetée en EPSG:2154.")
        self.assertEqual(
            say("data.reproject", dict(REPROJECT, layer_id="step:1"), step_names={1: BUFFER_OUT}),
            "Couche Tampon reprojetée en EPSG:2154.",
        )

    def test_unreadable_code_falls_back_to_the_description(self):
        catalogue = [{"id": "data.reproject", "description_fr": "Reprojette une couche."}]
        for bad in ("2154", None, 2154.5):
            self.assertEqual(
                say("data.reproject", dict(REPROJECT, target_epsg=bad), catalogue=catalogue),
                "Reprojette une couche. Terminée.",
            )


class TransformPointTest(unittest.TestCase):
    def test_the_reprojected_point_is_shown_with_its_code(self):
        self.assertEqual(
            say("data.transform_point", TRANSFORM, TRANSFORM_OUT),
            "Point reprojeté en EPSG:2154 : x 700000 ; y 6600000.",
        )
        self.assertEqual(
            say("data.transform_point", TRANSFORM, TRANSFORM_OUT, lang="en"),
            "Point reprojected to EPSG:2154: x 700000; y 6600000.",
        )
        decimals = {"point": {"x": 652000.25, "y": 6853000.5, "epsg": 2154}}
        self.assertEqual(
            say("data.transform_point", TRANSFORM, decimals),
            "Point reprojeté en EPSG:2154 : x 652000,25 ; y 6853000,5.",
        )

    def test_wgs84_and_utm_targets_reuse_the_geo_forms(self):
        back = {"point": {"x": 700000, "y": 6600000, "epsg": 2154}, "target_epsg": 4326}
        self.assertEqual(
            say("data.transform_point", back, {"point": {"x": 3.0, "y": 46.5, "epsg": 4326}}),
            "Point reprojeté en EPSG:4326 : 46,5000000° N, 3,0000000° E.",
        )
        utm = dict(back, target_epsg=32631)
        self.assertEqual(
            say("data.transform_point", utm,
                {"point": {"x": 500000.0, "y": 5150000.4, "epsg": 32631}}, lang="en"),
            "Point reprojected to EPSG:32631: UTM 31N 500000 E, 5150000 N.",
        )

    def test_geographic_coordinates_of_another_system_keep_their_decimals(self):
        rgf93 = dict(TRANSFORM, target_epsg=4171)
        self.assertEqual(
            say("data.transform_point", rgf93, {"point": {"x": 3.0, "y": 46.123456789, "epsg": 4171}},
                lang="en"),
            "Point reprojected to EPSG:4171: x 3; y 46.1234568.",
        )

    def test_without_result_only_the_target(self):
        self.assertEqual(
            say("data.transform_point", TRANSFORM), "Reprojection du point en EPSG:2154."
        )
        self.assertEqual(
            say("data.transform_point", TRANSFORM, {}, lang="en"),
            "Reprojection of the point to EPSG:2154.",
        )

    def test_unreadable_values_fall_back(self):
        catalogue = [{"id": "data.transform_point", "description_fr": "Reprojette un point."}]
        for params, result in [
            ({"point": TRANSFORM["point"], "target_epsg": "2154"}, TRANSFORM_OUT),
            (TRANSFORM, {"point": {"x": 1, "y": 2, "epsg": 4326}}),
            (TRANSFORM, {"point": {"x": "1", "y": 2, "epsg": 2154}}),
            (TRANSFORM, {"point": {"x": float("inf"), "y": 2, "epsg": 2154}}),
            (TRANSFORM, {"point": [1, 2]}),
        ]:
            self.assertEqual(
                say("data.transform_point", params, result, catalogue=catalogue),
                "Reprojette un point. Terminée.",
            )


class ReferencesTest(unittest.TestCase):
    def test_step_reference_uses_the_name_rendered_by_the_step(self):
        params = {"layer_id": "parcelles_2", "overlay_layer_id": "step:1.layer_id"}
        self.assertEqual(
            say("analysis.intersect", params, step_names={1: BUFFER_OUT}),
            "Intersection de Parcelles avec Tampon.",
        )
        self.assertEqual(
            say("analysis.intersect", dict(params, overlay_layer_id="step:1"), step_names={1: BUFFER_OUT}),
            "Intersection de Parcelles avec Tampon.",
        )

    def test_step_without_result(self):
        params = {"layer_id": "parcelles_2", "overlay_layer_id": "step:1.layer_id"}
        self.assertEqual(
            say("analysis.intersect", params), "Intersection de Parcelles avec le résultat de l'étape 1."
        )
        self.assertEqual(
            say("analysis.intersect", params, lang="en"),
            "Intersection of Parcelles with the result of step 1.",
        )

    def test_layer_unknown_to_qgis_named_by_a_step(self):
        params = {"layer_id": "parcelles_2", "overlay_layer_id": "t_9"}
        names = {1: {"layer_id": "t_9", "name": "Tampon"}}
        self.assertEqual(say("analysis.intersect", params, step_names=names),
                         "Intersection de Parcelles avec Tampon.")

    def test_layer_name_failure_is_harmless(self):
        def boom(_):
            raise RuntimeError("QGIS")

        text = summary.sentence("analysis.buffer", BUFFER, None, "fr", boom, None)
        self.assertEqual(text, "Zone tampon de 500 m autour de gares_1.")


class FallbackTest(unittest.TestCase):
    CATALOGUE = [{"id": "test.echo", "description_fr": "Renvoie ses paramètres.",
                  "description_en": "Returns its parameters."}]

    def test_unknown_command_uses_the_registry(self):
        self.assertEqual(say("test.echo", {}, {}, catalogue=self.CATALOGUE),
                         "Renvoie ses paramètres. Terminée.")
        self.assertEqual(say("test.echo", {}, {}, lang="en", catalogue=self.CATALOGUE),
                         "Returns its parameters. Done.")

    def test_unreadable_parameters_fall_back(self):
        catalogue = [{"id": "analysis.buffer", "description_fr": "Fait une zone tampon.",
                      "description_en": "Makes a buffer."}]
        self.assertEqual(say("analysis.buffer", {"layer_id": "gares_1"}, catalogue=catalogue),
                         "Fait une zone tampon. Terminée.")
        self.assertEqual(say("analysis.buffer", None, catalogue=catalogue),
                         "Fait une zone tampon. Terminée.")

    def test_user_text_is_masked(self):
        text = say("layout.export_pdf", {"title": "Carte de /home/camille/secret.gpkg"})
        self.assertNotIn("/home", text)


class DetailTest(unittest.TestCase):
    """Texte affiché au clic : ligne du journal (mode expert ou plan) ou étape du plan."""

    def setUp(self):
        import agentview  # noqa: E402
        import runlog  # noqa: E402

        self.agentview = agentview
        self.runlog = runlog

    def test_expert_line_shows_the_command_sentence(self):
        entry = self.runlog.Entry("analysis.buffer", "analysis.buffer", "14:02", params=BUFFER)
        entry.status = self.runlog.DONE
        entry.result = BUFFER_OUT
        self.assertEqual(
            summary.detail(entry, entry.label, "fr", qgis_name),
            "Zone tampon de 500 m autour de Gares : nouvelle couche Tampon, 8 entités.",
        )

    def test_pending_and_cancelled_steps_show_label_and_status(self):
        step = self.agentview.Step(2, "analysis.buffer", BUFFER)
        step.result = BUFFER_OUT  # jamais montré tant que l'étape n'est pas terminée
        for status in (self.agentview.PENDING, self.agentview.CANCELLED):
            step.status = status
            text = summary.detail(step, "2. Tampon", "fr", qgis_name)
            self.assertEqual(text, "2. Tampon : %s." % texts.tr("fr", status))
            self.assertNotIn("8", text)

    def test_failed_step_shows_its_error(self):
        step = self.agentview.Step(3, "analysis.intersect", {})
        step.status = self.agentview.ERROR
        self.assertEqual(
            summary.detail(step, "3. X", "fr", error_text="Étape 3 en erreur"), "Étape 3 en erreur"
        )
        self.assertEqual(
            summary.detail(step, "3. X", "en"), "3. X: %s." % texts.tr("en", "error")
        )

    def test_finished_core_step_without_result_uses_parameters(self):
        step = self.agentview.Step(1, "analysis.buffer", BUFFER)
        step.status = self.agentview.DONE
        self.assertEqual(
            summary.detail(step, "1. X", "fr", qgis_name), "Zone tampon de 500 m autour de Gares."
        )

    def test_running_line_has_no_double_punctuation(self):
        entry = self.runlog.Entry("analysis.buffer", "analysis.buffer", "14:02", params=BUFFER)
        self.assertEqual(summary.detail(entry, "analysis.buffer", "fr"), "analysis.buffer : En cours.")
        self.assertEqual(summary.detail(entry, "analysis.buffer", "en"), "analysis.buffer: Running.")


class ReviewFixesTest(unittest.TestCase):
    def test_truncated_zones_give_no_busiest_zone(self):
        params = {"layer_id": "parcelles_2", "points_layer_id": "gares_1"}
        out = {"name": "Décompte", "feature_count": 300,
               "zones": [{"label": "B", "count": 4}], "truncated": True}
        text = say("analysis.zonal_statistics", params, out)
        self.assertNotIn("fournie", text)
        self.assertTrue(text.endswith("nouvelle couche Décompte, 300 entités."))

    def test_numbers_are_not_rounded(self):
        params = {"layer_id": "gares_1", "distance": {"value": 12345.75, "unit": "m"}}
        self.assertEqual(say("analysis.buffer", params), "Zone tampon de 12345,75 m autour de Gares.")
        self.assertEqual(say("analysis.buffer", params, lang="en"), "Buffer of 12345.75 m around Gares.")
        big = {"layer_id": "gares_1", "distance": {"value": 1234567.5, "unit": "m"}}
        self.assertIn("1234567,5 m", say("analysis.buffer", big))


CATALOGUE_MD = os.path.join(
    HERE, "..", "..", "_bmad-output", "initiative-cassini-mvp", "spec-cassini-mvp",
    "catalogue-des-commandes.md",
)
L93 = {"x": 945000.0, "y": 6540000.0, "epsg": 2154}
WGS = {"x": 2.35, "y": 48.85, "epsg": 4326}
DEM = "C:/donnees/annecy/mnt-74-25m.tif"

# Paramètres et résultat d'exemple de chaque commande `faite` du catalogue : la phrase
# avant résultat (libellé d'étape) et la phrase après résultat doivent toutes deux exister.
SAMPLES = {
    "system.about": ({}, {"core_version": "0.1.0", "protocol_version": 1}),
    "model.list": ({}, {"models": [{"name": "qwen"}]}),
    "project.inventory": ({}, {"layers": [{"id": "gares_1"}, {"id": "parcelles_2"}]}),
    "layer.add": ({"path": "C:/d/gares.gpkg"}, {"layer_id": "gares_1", "name": "Gares"}),
    "layer.describe": ({"layer_id": "gares_1"},
                       {"name": "Gares", "feature_count": 3, "fields": ["nom"], "geometry": "Point"}),
    "layer.open_raster": ({"path": DEM}, {"layer_id": "mnt_1", "name": "mnt-74-25m"}),
    "layer.open_vector": ({"path": "C:/d/routes.shp"},
                          {"layer_id": "routes_1", "name": "routes", "crs": "EPSG:2154"}),
    "analysis.buffer": (BUFFER, BUFFER_OUT),
    "analysis.intersect": ({"layer_id": "parcelles_2", "overlay_layer_id": "tampon_3"},
                           {"layer_id": "x_4", "name": "Intersection", "feature_count": 2}),
    "analysis.select_by_attribute": (
        {"layer_id": "parcelles_2", "field": "zone", "operator": "eq", "value": "U"},
        {"layer_id": "x_5", "name": "Sélection", "feature_count": 12}),
    "analysis.zonal_statistics": ({"layer_id": "parcelles_2", "points_layer_id": "gares_1"},
                                  {"name": "Décompte", "feature_count": 4, "zones": [],
                                   "truncated": False}),
    "analysis.isochrone": ({"layer_id": "routes_1", "duration": {"value": 10, "unit": "min"},
                            "mode": "walk"},
                           {"layer_id": "iso_1", "name": "Isochrone", "feature_count": 1}),
    "layout.export_pdf": ({"title": "Gares"}, {"file": "C:/d/gares.pdf", "page_count": 1}),
    "terrain.slope": ({"dem": DEM, "unit": "percent"},
                      {"file": "C:/d/pente.tif", "unit": "percent", "min": 0.0, "max": 152.34,
                       "nodata_cells": 0}),
    "terrain.viewshed": ({"dem": DEM, "observer": L93},
                         {"file": "C:/d/vue.tif", "visible_cells": 1200, "hidden_cells": 800,
                          "nodata_cells": 0}),
    "terrain.line_of_sight": ({"dem": DEM, "observer": L93, "target": L93},
                              {"visible": False, "unit": "m", "distance": 2500.0,
                               "obstruction_distance": 830.0, "profile": [],
                               "profile_step": 25.0, "nodata_samples": 0}),
    "coord.convert": ({"point": WGS, "to": ["mgrs"]}, {"mgrs": "31UDQ5248011718"}),
    "geo.distance_azimuth": ({"from": WGS, "to": WGS},
                             {"distance": {"value": 1200, "unit": "m"},
                              "azimuth": {"value": 45, "unit": "deg"},
                              "back_azimuth": {"value": 225, "unit": "deg"}}),
    "geo.destination_point": ({"from": WGS, "azimuth": {"value": 90, "unit": "deg"},
                               "distance": {"value": 1, "unit": "km"}},
                              {"point": WGS}),
    "geo.bearing_intersection": ({"first": WGS, "first_azimuth": {"value": 10, "unit": "deg"},
                                  "second": WGS, "second_azimuth": {"value": 80, "unit": "deg"}},
                                 {"point": WGS, "distance_from_first": {"value": 500, "unit": "m"},
                                  "distance_from_second": {"value": 700, "unit": "m"}}),
    "data.transform_point": ({"point": WGS, "target_epsg": 2154},
                             {"point": {"x": 652000.0, "y": 6862000.0, "epsg": 2154}}),
    "data.reproject": ({"layer_id": "gares_1", "target_epsg": 2154},
                       {"layer_id": "gares_2", "name": "Gares 2154", "feature_count": 3}),
    "terrain.aspect": ({"dem": DEM}, {"file": "C:/d/expo.tif", "flat_cells": 12,
                                      "nodata_cells": 0}),
    "terrain.hillshade": ({"dem": DEM}, {"file": "C:/d/ombrage.tif", "nodata_cells": 0}),
    "terrain.viewshed_cumulative": ({"dem": DEM, "observers": [L93, L93]},
                                    {"file": "C:/d/cumul.tif", "observer_count": 2,
                                     "max_count": 2, "visible_cells": 3000, "nodata_cells": 0}),
    "terrain.peaks": ({"dem": DEM}, {"file": "C:/d/sommets.geojson", "feature_count": 2,
                                     "peaks": [{"point": L93, "elevation": 2351.5,
                                                "prominence": 120.0}],
                                     "truncated": False, "unit": "m"}),
    "raster.reclassify": ({"raster": "C:/d/pente.tif", "classes": [{"max": 10, "value": 1},
                                                                   {"min": 10, "value": 2}]},
                          {"file": "C:/d/classes.tif", "counts": [], "nodata_cells": 0}),
    "raster.weighted_overlay": ({"criteria": [{"raster": "C:/d/a.tif", "weight": 0.5},
                                              {"raster": "C:/d/b.tif", "weight": 0.5}]},
                                {"file": "C:/d/aptitude.tif", "min": 1.0, "max": 4.5,
                                 "nodata_cells": 0}),
    "terrain.cost_distance": ({"cost": "C:/d/cout.tif", "sources": [L93]},
                              {"file": "C:/d/cumul-cout.tif", "max": 900.0,
                               "reachable_cells": 2500, "unreachable_cells": 0}),
    "terrain.least_cost_path": ({"cost": "C:/d/cout.tif", "start": L93, "end": L93},
                                {"file": "C:/d/chemin.geojson", "feature_count": 1,
                                 "total_cost": 412.25, "length": {"value": 1830.0, "unit": "m"}}),
    "map.topographic_style": ({}, {"styled": [{"layer_id": "mnt_1", "theme": "relief"}],
                                   "unchanged": [], "groups": []}),
    "map.graduated_style": ({"layer_id": "parcelles_2", "field": "surface"},
                            {"layer_id": "parcelles_2", "field": "surface", "class_count": 5}),
    "layer.set_style": ({"layer_id": "gares_1", "style": {"kind": "single"}},
                        {"layer_id": "gares_1"}),
    "layer.arrange": ({"groups": [{"name": "Relief", "layers": ["mnt_1"]}]},
                      {"group_count": 1, "layer_count": 1}),
}


def done_commands():
    """Ids des commandes au statut `faite` du catalogue (fiches ```json fiche```)."""
    import json
    import re

    with open(CATALOGUE_MD, encoding="utf-8") as f:
        text = f.read()
    ids = []
    for block in re.findall(r"```json fiche\n(.*?)```", text, re.S):
        sheet = json.loads(block)
        if sheet.get("status") == "faite":
            ids.append(sheet["id"])
    return ids


class CatalogueCompletenessTest(unittest.TestCase):
    """Chaque commande `faite` a sa phrase FR et EN, avant et après résultat."""

    def test_every_done_command_has_its_sentences(self):
        ids = done_commands()
        self.assertGreater(len(ids), 30)  # le catalogue a bien été lu
        for command in ids:
            with self.subTest(command=command):
                self.assertIn(command, SAMPLES, "exemple manquant dans SAMPLES")
                self.assertIn(command, summary._SENTENCES, "phrase manquante dans summary")
                params, result = SAMPLES[command]
                seen = {}
                for lang in ("fr", "en"):
                    before = summary.label(command, params, lang, qgis_name)
                    after = say(command, params, result, lang=lang)
                    self.assertNotEqual(before, command)  # pas le repli sur l'id
                    self.assertFalse(before.endswith("."), before)
                    for done in ("Terminée", "Done"):
                        self.assertNotIn(done, after)
                    self.assertNotEqual(after, before + ".", "pas de phrase après résultat")
                    self.assertNotIn("C:/", before + after)  # aucun chemin (AD-9)
                    seen[lang] = (before, after)
                self.assertNotEqual(seen["fr"], seen["en"])


class LabelTest(unittest.TestCase):
    """Libellé d'une étape : la phrase avant résultat, ou un repli court."""

    DESCRIPTION = [{
        "id": "terrain.slope",
        "description_fr": "Pente d'un MNT : calcule la pente de chaque cellule d'un modèle "
                          "numérique d'élévation (fichier GeoTIFF désigné par `dem`, chemin "
                          "de fichier MNT recopié de l'inventaire, pas un id de couche).",
        "description_en": "DEM slope: computes the slope of each cell (GeoTIFF file given by "
                          "`dem`, a DEM file path copied from the inventory).",
    }]

    def test_slope_plan_and_result_never_show_the_registry_description(self):
        params = {"dem": DEM, "unit": "percent"}
        result = SAMPLES["terrain.slope"][1]
        self.assertEqual(summary.label("terrain.slope", params, "fr"),
                         "Pente de mnt-74-25m en pourcentage")
        after = say("terrain.slope", params, result, catalogue=self.DESCRIPTION)
        self.assertEqual(
            after, "Pente de mnt-74-25m en pourcentage : fichier pente.tif, de 0 à 152,3 %.")
        self.assertEqual(
            say("terrain.slope", params, result, lang="en"),
            "Slope of mnt-74-25m in percent: file pente.tif, from 0 to 152.3 %.")
        for text in (after, summary.label("terrain.slope", params, "fr")):
            self.assertNotIn("GeoTIFF", text)
            self.assertNotIn("inventaire", text)

    def test_unreadable_parameters_give_the_short_title(self):
        for params in (None, {}, {"dem": DEM, "unit": "radians"}, {"dem": 3, "unit": "percent"}):
            self.assertEqual(
                summary.label("terrain.slope", params, "fr", catalogue=self.DESCRIPTION),
                "Pente d'un MNT")
            self.assertEqual(
                say("terrain.slope", params, catalogue=self.DESCRIPTION), "Pente d'un MNT. Terminée.")
        self.assertEqual(
            summary.label("terrain.slope", None, "en", catalogue=self.DESCRIPTION), "DEM slope")

    def test_unknown_command_gives_its_title_or_its_id(self):
        catalogue = [{"id": "x.new", "description_fr": "Nouvelle chose. Elle fait beaucoup."}]
        self.assertEqual(summary.label("x.new", {}, "fr", catalogue=catalogue), "Nouvelle chose")
        self.assertEqual(summary.label("x.other", {}, "fr", catalogue=catalogue), "x.other")
        self.assertEqual(summary.label(None, {}, "fr"), "?")

    def test_a_long_first_sentence_is_cut_at_a_word(self):
        catalogue = [{"id": "x.long", "description_fr": "mot " * 60}]
        title = summary.short_title("x.long", catalogue, "fr")
        self.assertTrue(title.endswith("…"))
        self.assertLessEqual(len(title), 81)
        self.assertNotIn("  ", title)

    def test_a_raster_from_an_earlier_step_reads_as_that_step(self):
        params = {"raster": "step:1.file", "classes": [{"value": 1}]}
        self.assertEqual(summary.label("raster.reclassify", params, "fr"),
                         "Reclassement du résultat de l'étape 1 en 1 classe")

    # Clés « après résultat » (au passé ou au bilan) : jamais dans le libellé d'une étape proposée.
    AFTER_KEYS = {
        "about_result", "sum_layer_added", "sum_opened_raster", "sum_export_pdf",
        "sum_reproject", "sum_converted", "sum_transformed_point", "sum_geo_measured",
        "sum_geo_reached", "sum_geo_intersected", "sum_topo_styled", "sum_style_set",
        "sum_arranged", "sum_out_layer", "sum_file", "sum_peaks_found", "sum_path_found",
    }

    def test_no_label_uses_an_after_result_key(self):
        used = []
        real = summary.texts.tr

        def spy(lang, key, **values):
            used.append(key)
            return real(lang, key, **values)

        summary.texts.tr = spy
        try:
            for command, (params, _) in SAMPLES.items():
                for lang in ("fr", "en"):
                    del used[:]
                    summary.label(command, params, lang, qgis_name)
                    with self.subTest(command=command, lang=lang):
                        self.assertFalse(self.AFTER_KEYS & set(used), used)
        finally:
            summary.texts.tr = real

    def test_real_registry_descriptions_give_clean_titles(self):
        import re

        core = os.path.join(HERE, "..", "..", "core", "src", "commands")
        string = r'"((?:[^"\\]|\\.)*)"'
        for command in ("coord.convert", "terrain.cost_distance", "terrain.least_cost_path"):
            with open(os.path.join(core, command.replace(".", "_") + ".rs"), encoding="utf-8") as f:
                source = f.read()
            found = re.search(
                r"CommandSpec::new::<[^>]*>\(\s*\"%s\",\s*%s,\s*%s" % (
                    re.escape(command), string, string), source)
            self.assertIsNotNone(found, command)
            catalogue = [{"id": command, "description_fr": found.group(1),
                          "description_en": found.group(2)}]
            for lang in ("fr", "en"):
                title = summary.short_title(command, catalogue, lang)
                with self.subTest(command=command, lang=lang, title=title):
                    self.assertNotIn("(", title)
                    self.assertNotIn("`", title)
                    self.assertFalse(title.rstrip("…").endswith("EPSG"), title)
                    self.assertGreater(len(title), 5)
                    self.assertNotIn("…. ", summary.fallback(command, lang, catalogue))

    def test_a_cut_title_gives_a_single_period_before_done(self):
        catalogue = [{"id": "x.long", "description_fr": "mot " * 60}]
        text = summary.fallback("x.long", "fr", catalogue)
        self.assertTrue(text.endswith("mot. Terminée."), text)

    def test_contraction_only_touches_the_step_reference(self):
        names = {"t_1": "de le test", "a_1": "à le bord"}.get
        params = {"layer_id": "t_1", "distance": {"value": 5, "unit": "m"}}
        self.assertEqual(summary.label("analysis.buffer", params, "fr", names),
                         "Zone tampon de 5 m autour de de le test")
        self.assertEqual(
            summary.label("analysis.intersect", {"layer_id": "a_1", "overlay_layer_id": "t_1"},
                          "fr", names),
            "Intersection de à le bord avec de le test")
        self.assertEqual(
            summary.label("layout.export_pdf", {"title": "Vue de le test"}, "fr"),
            "Export PDF de la carte « Vue de le test »")
        ref = {"layer_id": "step:1.layer_id", "distance": {"value": 5, "unit": "m"}}
        self.assertEqual(summary.label("analysis.buffer", ref, "fr"),
                         "Zone tampon de 5 m autour du résultat de l'étape 1")
        self.assertNotIn("\ue000", say("analysis.buffer", ref))

    def test_history_labels_use_the_parameters(self):
        import runlog

        history = [
            {"step": 1, "command": "terrain.slope", "params": {"dem": DEM, "unit": "percent"},
             "result": SAMPLES["terrain.slope"][1], "plan_step": 1},
            {"step": 2, "command": "terrain.slope",
             "params": {"truncated": True, "bytes": 900, "fingerprint": "ab"}, "plan_step": 2},
        ]
        log = runlog.RunLog.from_history(
            history,
            lambda c, p: summary.label(c, p, "fr", catalogue=self.DESCRIPTION),
        )
        self.assertEqual([e.label for e in log.entries],
                         ["1. Pente de mnt-74-25m en pourcentage", "2. Pente d'un MNT"])


if __name__ == "__main__":
    unittest.main()
