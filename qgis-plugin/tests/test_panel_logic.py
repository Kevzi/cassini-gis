"""Tests sans QGIS de la logique pure du panneau : texts, form, runlog."""

import os
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "cassini_qgis"))

import form  # noqa: E402
import runlog  # noqa: E402
import texts  # noqa: E402


class HistoryTest(unittest.TestCase):
    """Les étapes passées du projet, rendues par le noyau à l'ouverture."""

    def test_past_steps_fill_the_log_with_their_outcome_and_no_time(self):
        history = [
            {"step": 1, "command": "system.about", "params": {}, "result": {"name": "cassini"}},
            {"step": 2, "command": "analysis.buffer", "params": {"layer": "l"},
             "error": "analysis.crs_not_metric", "plan_step": 1},
            {"step": 3, "command": "terrain.slope", "params": {}, "cancelled": True},
            {"step": 4, "command": "layer.add", "params": {}, "result": 1, "origin": "external"},
            {"step": 5},  # sans commande : sautée
            "pas un objet",
        ]
        log = runlog.RunLog.from_history(history, lambda c, p: "<%s>" % c)
        self.assertEqual(
            [(e.command, e.status, e.label, e.when) for e in log.entries],
            [
                ("system.about", runlog.DONE, "<system.about>", ""),
                ("analysis.buffer", runlog.ERROR, "1. <analysis.buffer>", ""),
                ("terrain.slope", runlog.CANCELLED, "<terrain.slope>", ""),
                ("layer.add", runlog.DONE, "<layer.add>", ""),
            ],
        )
        self.assertEqual(log.entries[0].result, {"name": "cassini"})
        self.assertEqual(log.entries[1].error, ("analysis.crs_not_metric", ""))
        self.assertEqual(log.entries[3].origin, runlog.EXTERNAL)
        # Aucune ligne passée n'est « en cours » : la session suivante ne la ferme pas.
        self.assertIsNone(log.current())
        self.assertEqual(
            runlog.line_text(log.entries[0], "Terminée"), "✔ Terminée  <system.about>"
        )

    def test_a_dated_step_shows_its_local_date_and_time(self):
        import time

        at = 1762600000000
        log = runlog.RunLog.from_history(
            [{"step": 1, "command": "system.about", "params": {}, "result": {}, "at": at}],
            lambda c, p: c,
        )
        expected = time.strftime("%d/%m %H:%M", time.localtime(at / 1000))
        self.assertEqual(log.entries[0].when, expected)
        self.assertEqual(
            runlog.line_text(log.entries[0], "Terminée"),
            "✔ Terminée  system.about  " + expected,
        )
        for bad in (None, True, "hier", -5, 10 ** 30):
            self.assertEqual(runlog.past_time(bad), "")

    def test_a_summarized_result_or_params_is_not_taken_for_the_real_one(self):
        stub = {"truncated": True, "bytes": 900, "fingerprint": "ab12"}
        log = runlog.RunLog.from_history(
            [{"step": 1, "command": "c", "params": stub, "result": stub, "plan_step": True}],
            lambda c, p: "%s %r" % (c, p),
        )
        entry = log.entries[0]
        self.assertIsNone(entry.result)
        self.assertIsNone(entry.params)
        self.assertIsNone(entry.step)  # un booléen n'est pas un numéro d'étape
        self.assertEqual(entry.label, "c None")  # le libellé ne reçoit pas l'empreinte

    def test_no_history_gives_an_empty_log(self):
        self.assertEqual(runlog.RunLog.from_history(None, str).entries, [])
        self.assertEqual(runlog.RunLog.from_history([], str).entries, [])


class ResultFolderTest(unittest.TestCase):
    def test_folder_of_an_existing_file(self):
        import tempfile

        with tempfile.TemporaryDirectory() as d:
            f = os.path.join(d, "1-layout.export_pdf.pdf")
            open(f, "w").close()
            self.assertEqual(runlog.result_folder({"file": f}), d)
            self.assertIsNone(runlog.result_folder({"file": f + ".absent"}))

    def test_no_folder_without_an_absolute_file(self):
        for result in (None, {}, {"file": 3}, {"file": "rel.pdf"}, {"layer_id": "x"}, "texte"):
            self.assertIsNone(runlog.result_folder(result))


class OfferedCommandsTest(unittest.TestCase):
    """Le formulaire n'offre pas une commande que le noyau dicte seul (ticket 4.9)."""

    def test_a_dictated_command_is_not_offered(self):
        catalogue = [
            {"id": "layer.add", "available": True},
            {"id": "layer.open_raster", "available": False, "dictated_by_core": True},
            {"id": "layer.open_vector", "available": False, "dictated_by_core": True},
            {"id": "terrain.slope", "available": True},
        ]
        self.assertEqual(
            [e["id"] for e in form.offered(catalogue)], ["layer.add", "terrain.slope"]
        )
        self.assertEqual(form.offered(None), [])
        self.assertEqual(form.offered([3, {"id": "x"}]), [{"id": "x"}])


class RedactTest(unittest.TestCase):
    def test_path_inside_a_message(self):
        cases = {
            "Echec sur C:\\Users\\kev\\data\\a.gpkg.": "Echec sur …/a.gpkg.",
            "ouvre /home/k/x.shp (erreur)": "ouvre …/x.shp (erreur)",
            "'C:/x/y.tif' absent": "'…/y.tif' absent",
            "lu dans \\\\srv\\p\\q.gpkg": "lu dans …/q.gpkg",
            "C:\\d\\f.gpkg: cannot open /x/bad": "…/f.gpkg: cannot open …/bad",
            "fichier:C:\\x\\a.gpkg": "fichier:…/a.gpkg",
            "path=/tmp/x.log": "path=…/x.log",
            "C:\\Users\\kev\\My Data\\a.gpkg": "…/a.gpkg",
        }
        for text, expected in cases.items():
            with self.subTest(text=text):
                self.assertEqual(runlog.redact(text), expected)

    def test_text_without_path_is_untouched(self):
        for text in ("a/b et 1/2", "fichier a.shp non lisible", "", "Le filtre / la valeur"):
            self.assertEqual(runlog.redact(text), text)

    def test_nested_values(self):
        self.assertEqual(
            runlog.redact({"e": ["voir /tmp/z.log"], "n": 3}), {"e": ["voir …/z.log"], "n": 3}
        )


class EntryTest(unittest.TestCase):
    def test_entry_carries_params_result_and_step(self):
        log = runlog.RunLog()
        plain = log.start("system.about", "system.about", "14:02")
        self.assertEqual((plain.params, plain.result, plain.step, plain.error), (None, None, None, None))
        self.assertEqual(plain.refs, {})
        refs = {}
        step = log.start("analysis.buffer", "1. Tampon", "14:03", params={"layer_id": "g"},
                         step=1, refs=refs)
        self.assertEqual((step.params, step.step), ({"layer_id": "g"}, 1))
        self.assertIs(step.refs, refs)  # partagé par les lignes du même plan
        entry = log.finish(runlog.DONE)
        entry.result = {"name": "Tampon"}
        self.assertIs(entry, step)

    def test_step_details_mask_paths(self):
        text = runlog.step_details({"path": "C:\\Users\\kev\\gares.gpkg"}, {"file": "/home/k/2.pdf"})
        self.assertNotIn("Users", text)
        self.assertNotIn("/home", text)
        self.assertIn("…/gares.gpkg", text)
        self.assertIn('"params"', text)
        self.assertIn('"result"', text)


class TextsTest(unittest.TestCase):
    def test_same_keys_in_both_languages(self):
        self.assertEqual(set(texts.TEXTS["fr"]), set(texts.TEXTS["en"]))

    def test_no_empty_text(self):
        for lang, table in texts.TEXTS.items():
            for key, value in table.items():
                self.assertTrue(value.strip(), (lang, key))

    def test_placeholders_match(self):
        import string

        def names(text):
            return {f for _, f, _, _ in string.Formatter().parse(text) if f}

        for key in texts.TEXTS["fr"]:
            self.assertEqual(
                names(texts.TEXTS["fr"][key]), names(texts.TEXTS["en"][key]), key
            )

    def test_tr_and_fallback(self):
        self.assertEqual(texts.tr("fr", "run"), "Lancer")
        self.assertEqual(texts.tr("en", "run"), "Run")
        self.assertEqual(texts.tr("zz", "run"), "Lancer")
        self.assertEqual(
            texts.tr("fr", "step_error", n=2, code="a.b", message="m"),
            "Étape 2 en erreur (a.b) : m",
        )

    def test_pick_registry_description(self):
        self.assertEqual(texts.pick("en", "fr", "en"), "en")
        self.assertEqual(texts.pick("en", "fr", ""), "fr")
        self.assertEqual(texts.pick("fr", "fr", "en"), "fr")


SCHEMA = {
    "type": "object",
    "required": ["name"],
    "properties": {
        "name": {"type": "string"},
        "count": {"type": ["integer", "null"]},
        "ratio": {"type": "number"},
        "flag": {"type": "boolean"},
        "mode": {"type": "string", "enum": ["a", "b"]},
        "nested": {"type": "object", "properties": {"x": {"type": "string"}}},
        "either": {"anyOf": [{"type": "string"}, {"type": "integer"}]},
        "ref": {"$ref": "#/definitions/X"},
    },
}


class FormTest(unittest.TestCase):
    def kinds(self):
        return {f.name: f.kind for f in form.build_fields(SCHEMA)}

    def test_subset_and_raw_json_fallback(self):
        k = self.kinds()
        self.assertEqual(k["name"], form.TEXT)
        self.assertEqual(k["count"], form.INTEGER)
        self.assertEqual(k["ratio"], form.NUMBER)
        self.assertEqual(k["flag"], form.BOOLEAN)
        self.assertEqual(k["mode"], form.ENUM)
        for name in ("nested", "either", "ref"):
            self.assertEqual(k[name], form.JSON, name)

    def test_a_ref_to_a_string_definition_is_a_text_field(self):
        # Un `RasterPath` (ticket 11.5) : `$ref` vers une définition de type chaîne.
        schema = {"type": "object", "required": ["dem"], "properties": {
            "dem": {"$ref": "#/$defs/RasterPath", "description": "MNT"},
            "observer": {"$ref": "#/$defs/Point"},
            "lost": {"$ref": "#/$defs/Absent"}},
            "$defs": {"RasterPath": {"type": "string", "x-cassini": "raster"},
                      "Point": {"type": "object", "properties": {"x": {"type": "number"}}}}}
        fields = {f.name: f for f in form.build_fields(schema)}
        self.assertEqual(fields["dem"].kind, form.TEXT)
        self.assertEqual(fields["dem"].description, "MNT")
        self.assertTrue(fields["dem"].required)
        self.assertEqual(fields["observer"].kind, form.JSON)
        self.assertEqual(fields["lost"].kind, form.JSON)
        self.assertEqual(form.build_params([fields["dem"]], {"dem": "mnt.tif"}), {"dem": "mnt.tif"})

    def test_required_flag_and_choices(self):
        fields = {f.name: f for f in form.build_fields(SCHEMA)}
        self.assertTrue(fields["name"].required)
        self.assertFalse(fields["count"].required)
        self.assertEqual(fields["mode"].choices, ["a", "b"])

    def test_core_filled_params_are_not_shown(self):
        # `out`, `leave_out`, `center` et `focus` (entrée 7.9) sont remplis par le noyau : jamais
        # saisis à la main.
        schema = {"type": "object", "properties": {
            "title": {"type": "string"}, "out": {"type": "string"},
            "leave_out": {"type": "array"}, "center": {"type": "object"},
            "focus": {"type": "object"}}}
        self.assertEqual([f.name for f in form.build_fields(schema)], ["title"])

    def test_no_properties_means_no_field(self):
        self.assertEqual(form.build_fields({"type": "object"}), [])
        self.assertEqual(form.build_fields(None), [])

    def test_build_params(self):
        fields = form.build_fields(SCHEMA)
        params = form.build_params(
            fields,
            {"name": "x", "count": "3", "ratio": "1.5", "flag": True, "mode": "b",
             "nested": '{"x": "y"}', "either": "", "ref": ""},
        )
        self.assertEqual(
            params,
            {"name": "x", "count": 3, "ratio": 1.5, "flag": True, "mode": "b",
             "nested": {"x": "y"}},
        )

    def test_empty_optional_is_omitted_empty_required_text_is_sent(self):
        fields = form.build_fields(SCHEMA)
        params = form.build_params(fields, {})
        self.assertEqual(params, {"name": "", "flag": False})

    def test_invalid_raw_json_sends_nothing(self):
        fields = form.build_fields(SCHEMA)
        with self.assertRaises(form.FormError) as ctx:
            form.build_params(fields, {"nested": "{pas du json"})
        self.assertEqual(ctx.exception.key, "bad_json")
        self.assertEqual(ctx.exception.name, "nested")

    def test_unreadable_number(self):
        fields = form.build_fields(SCHEMA)
        with self.assertRaises(form.FormError) as ctx:
            form.build_params(fields, {"count": "abc"})
        self.assertEqual(ctx.exception.key, "bad_number")

    def test_error_messages_exist_in_both_languages(self):
        for lang in ("fr", "en"):
            for key in ("bad_json", "bad_number"):
                self.assertIn("x", texts.tr(lang, key, name="x"))


class RunLogTest(unittest.TestCase):
    def test_lifecycle(self):
        log = runlog.RunLog()
        e = log.start("system.about", "system.about", "14:02")
        self.assertEqual(e.status, runlog.RUNNING)
        self.assertIs(log.current(), e)
        log.finish(runlog.DONE)
        self.assertEqual(e.status, runlog.DONE)
        self.assertIsNone(log.current())

    def test_finish_without_running_entry_is_harmless(self):
        self.assertIsNone(runlog.RunLog().finish(runlog.ERROR))

    def test_line_has_icon_word_label_and_time(self):
        log = runlog.RunLog()
        e = log.start("a.b", "a.b", "09:05")
        log.finish(runlog.CANCELLED)
        line = runlog.line_text(e, texts.tr("fr", "cancelled"))
        self.assertIn(runlog.ICONS[runlog.CANCELLED], line)
        for part in ("Annulée", "a.b", "09:05"):
            self.assertIn(part, line)

    def test_every_status_has_an_icon_and_a_word(self):
        for status in (runlog.RUNNING, runlog.DONE, runlog.CANCELLED, runlog.ERROR):
            self.assertIn(status, runlog.ICONS)
            self.assertIn(status, texts.TEXTS["fr"])
            self.assertIn(status, texts.TEXTS["en"])

    def test_no_absolute_path_in_details(self):
        shown = runlog.technical_details(
            {"out": "C:\\Users\\x\\p\\carte.pdf", "in": "/home/x/a.gpkg", "n": 3,
             "rel": "out/a.pdf", "l": ["/etc/passwd"]}
        )
        for banned in ("C:\\", "/home", "/etc"):
            self.assertNotIn(banned, shown)
        self.assertIn("…/carte.pdf", shown)
        self.assertIn("out/a.pdf", shown)

    def test_controls_idle(self):
        c = runlog.controls(True, True, False, True)
        self.assertTrue(c["run_visible"] and c["run_enabled"])
        self.assertFalse(c["stop_visible"])

    def test_controls_unavailable_command_cannot_run(self):
        self.assertFalse(runlog.controls(True, False, False, False)["run_enabled"])
        self.assertFalse(runlog.controls(False, True, False, False)["run_enabled"])

    def test_controls_running_cancellable_swaps_run_for_stop(self):
        c = runlog.controls(True, True, True, True)
        self.assertFalse(c["run_visible"])
        self.assertTrue(c["stop_visible"] and c["stop_enabled"])
        self.assertIsNone(c["stop_reason"])

    def test_controls_running_not_cancellable_greys_stop_with_reason(self):
        c = runlog.controls(True, True, True, False)
        self.assertTrue(c["stop_visible"])
        self.assertFalse(c["stop_enabled"])
        self.assertEqual(c["stop_reason"], "not_cancellable")
        self.assertFalse(c["run_enabled"])


class BrowsePathTest(unittest.TestCase):
    def test_only_text_fields_named_path(self):
        fields = form.build_fields(
            {"type": "object", "properties": {
                "path": {"type": "string"}, "out_path": {"type": "string"},
                "name": {"type": "string"}, "flag": {"type": "boolean"}}}
        )
        found = sorted(f.name for f in fields if form.is_path_field(f))
        self.assertEqual(found, ["out_path", "path"])

    def test_layer_fields_are_recognised(self):
        fields = form.build_fields(
            {"type": "object", "properties": {
                "layer_id": {"type": "string"}, "other_layer_id": {"type": "string"},
                "path": {"type": "string"}, "layer_id_n": {"type": "integer"}}}
        )
        found = sorted(f.name for f in fields if form.is_layer_field(f))
        self.assertEqual(found, ["layer_id", "other_layer_id"])

    def test_field_fields_are_recognised(self):
        fields = form.build_fields(
            {"type": "object", "properties": {
                "field": {"type": "string"}, "sort_field": {"type": "string"},
                "fields": {"type": "string"}, "field_n": {"type": "integer"}}}
        )
        found = sorted(f.name for f in fields if form.is_field_field(f))
        self.assertEqual(found, ["field", "sort_field"])

    def test_flat_operator_enum_is_a_choice_list(self):
        fields = form.build_fields(
            {"type": "object", "properties": {
                "operator": {"type": "string", "enum": ["eq", "ne", "begins_with"]}},
             "required": ["operator"]}
        )
        self.assertEqual((fields[0].kind, fields[0].choices), (form.ENUM, ["eq", "ne", "begins_with"]))

    def test_relative_to_the_project_with_slashes(self):
        project = os.path.abspath(os.path.join("proj", "villeneuve.qgz"))
        chosen = os.path.abspath(os.path.join("proj", "fr", "data.gpkg"))
        self.assertEqual(form.relative_path(chosen, project), "fr/data.gpkg")

    def test_file_outside_the_project_keeps_dotdot(self):
        project = os.path.abspath(os.path.join("proj", "p.qgz"))
        chosen = os.path.abspath(os.path.join("autre", "x.shp"))
        self.assertEqual(form.relative_path(chosen, project), "../autre/x.shp")

    def test_browse_texts_exist_in_both_languages(self):
        for lang in ("fr", "en"):
            for key in ("browse", "browse_title", "browse_other_drive"):
                self.assertTrue(texts.tr(lang, key))


if __name__ == "__main__":
    unittest.main()
