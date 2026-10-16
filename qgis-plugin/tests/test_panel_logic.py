"""Tests sans QGIS de la logique pure du panneau : texts, form, runlog."""

import os
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "cassini_qgis"))

import form  # noqa: E402
import runlog  # noqa: E402
import texts  # noqa: E402


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

    def test_required_flag_and_choices(self):
        fields = {f.name: f for f in form.build_fields(SCHEMA)}
        self.assertTrue(fields["name"].required)
        self.assertFalse(fields["count"].required)
        self.assertEqual(fields["mode"].choices, ["a", "b"])

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
