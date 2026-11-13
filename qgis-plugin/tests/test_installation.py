"""Tests sans QGIS : lecture de installation.json et rappel de mise à jour (ticket 8)."""

import datetime
import json
import os
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "cassini_qgis"))

import installation  # noqa: E402
import texts  # noqa: E402


def _core_with(content):
    folder = tempfile.mkdtemp()
    if content is not None:
        with open(os.path.join(folder, "installation.json"), "w", encoding="utf-8") as f:
            f.write(content)
    return os.path.join(folder, "cassini.exe")


class ReadTest(unittest.TestCase):
    def test_valid_file(self):
        core = _core_with(json.dumps({"version": "0.1.0", "installe_le": "2026-11-13T09:30:00Z"}))
        info = installation.read(core)
        self.assertEqual(info["version"], "0.1.0")
        self.assertEqual(info["installed_at"], datetime.datetime(2026, 11, 13, 9, 30))

    def test_unusable_files_give_none_without_exception(self):
        for content in (
            None,
            "pas du json",
            "[]",
            json.dumps({"version": "0.1.0"}),
            json.dumps({"version": "0.1.0", "installe_le": "hier"}),
            json.dumps({"version": "0.1.0", "installe_le": "2026-13-45T00:00:00Z"}),
            json.dumps({"installe_le": "2026-11-13T09:30:00Z"}),
            json.dumps({"version": 3, "installe_le": "2026-11-13T09:30:00Z"}),
        ):
            self.assertIsNone(installation.read(_core_with(content)), content)

    def test_home_url_is_fixed(self):
        self.assertEqual(installation.HOME_URL, "https://kevzi.github.io/cassini-gis")


class ReminderTest(unittest.TestCase):
    def due(self, installed, now):
        return installation.reminder_due(installed, now)

    def test_old_installation_is_due(self):
        self.assertTrue(self.due(datetime.datetime(2026, 1, 1), datetime.datetime(2026, 11, 13)))

    def test_recent_installation_is_not_due(self):
        self.assertFalse(self.due(datetime.datetime(2026, 11, 1), datetime.datetime(2026, 11, 13)))

    def test_month_boundary(self):
        installed = datetime.datetime(2026, 8, 13, 9, 30)
        self.assertFalse(self.due(installed, datetime.datetime(2026, 11, 13, 9, 29)))
        self.assertTrue(self.due(installed, datetime.datetime(2026, 11, 13, 9, 30)))

    def test_end_of_month_is_brought_back(self):
        installed = datetime.datetime(2026, 11, 30)  # + 3 mois : 28 février 2027
        self.assertFalse(self.due(installed, datetime.datetime(2027, 2, 27)))
        self.assertTrue(self.due(installed, datetime.datetime(2027, 2, 28)))

    def test_year_rollover(self):
        installed = datetime.datetime(2026, 12, 15)
        self.assertFalse(self.due(installed, datetime.datetime(2027, 3, 14)))
        self.assertTrue(self.due(installed, datetime.datetime(2027, 3, 15)))


class TextsTest(unittest.TestCase):
    def test_keys_present_in_both_languages(self):
        for lang in ("fr", "en"):
            for key in ("not_found", "open_home", "install_version", "update_reminder"):
                self.assertTrue(texts.tr(lang, key) not in ("", key), (lang, key))
            self.assertIn("{version}", texts.TEXTS[lang]["install_version"])

    def test_not_found_says_the_engine_is_needed(self):
        self.assertIn("a besoin de son moteur pour fonctionner", texts.tr("fr", "not_found"))
        self.assertNotIn("introuvable", texts.tr("fr", "not_found"))
        self.assertIn("needs its engine", texts.tr("en", "not_found"))


if __name__ == "__main__":
    unittest.main()
