"""Tests sans QGIS : confiance envers le programme noyau branché (ticket 15)."""

import hashlib
import os
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "cassini_qgis"))

import locator  # noqa: E402
import texts  # noqa: E402
import trust  # noqa: E402


def _program(content=b"programme"):
    handle, path = tempfile.mkstemp()
    with os.fdopen(handle, "wb") as f:
        f.write(content)
    return path


class TrustTest(unittest.TestCase):
    def test_fingerprint_is_sha256(self):
        path = _program(b"abc")
        self.assertEqual(trust.fingerprint(path), hashlib.sha256(b"abc").hexdigest())

    def test_official_install_location_launches_without_question(self):
        self.assertEqual(trust.decide("absent", trust.OFFICIAL_ORIGIN, []), ("launch", None))

    def test_first_unofficial_launch_asks_then_remembered_does_not(self):
        path = _program()
        action, digest = trust.decide(path, "réglage cassini/core_path", [])
        self.assertEqual(action, "confirm")
        accepted = trust.remember([], digest)
        self.assertEqual(trust.decide(path, "réglage cassini/core_path", accepted)[0], "launch")
        self.assertFalse(trust.needs_confirmation(path, "variable CASSINI_CORE", accepted))

    def test_changed_binary_asks_again(self):
        path = _program(b"v1")
        accepted = trust.remember([], trust.fingerprint(path))
        with open(path, "wb") as f:
            f.write(b"v2")
        self.assertEqual(trust.decide(path, "variable CASSINI_CORE", accepted)[0], "confirm")

    def test_unreadable_file_is_a_refusal(self):
        self.assertEqual(trust.decide("n-existe-pas", "réglage cassini/core_path", []), ("refuse", None))
        self.assertTrue(trust.needs_confirmation("n-existe-pas", "PATH", []))

    def test_accepted_list_round_trip_and_tolerance(self):
        self.assertEqual(trust.parse_accepted(trust.dump_accepted(["b", "a", "a"])), ["a", "b"])
        for raw in (None, "", "pas du json", "{}", "[1, null]"):
            self.assertEqual(trust.parse_accepted(raw), [])

    def test_official_origin_is_the_one_locator_produces(self):
        origins = [o for o, _p in locator.candidates("", {}, "linux", "/h")]
        self.assertIn(trust.OFFICIAL_ORIGIN, origins)

    def test_derived_decided_on_program_name(self):
        self.assertFalse(trust.is_derived("cassini"))
        self.assertTrue(trust.is_derived("mon-programme"))
        self.assertFalse(trust.is_derived(None))


class TextsTest(unittest.TestCase):
    def test_new_texts_exist_in_both_languages(self):
        for key in ("core_program_label", "program_line", "program_derived", "program_ad14",
                    "trust_text", "trust_refused", "trust_unreadable"):
            for lang in ("fr", "en"):
                self.assertTrue(texts.tr(lang, key, **dict(name="n", version="1", path="p", digest="d")))


if __name__ == "__main__":
    unittest.main()
