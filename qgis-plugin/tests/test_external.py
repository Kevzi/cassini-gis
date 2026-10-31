"""Tests sans QGIS du canal d'IA externe côté extension (ticket 5.26, AD-29, flux 4).

Une ligne de la matrice du plan par test : le noyau est simulé par `FakeProcess`, et les
événements du client passent par `external.apply`, le routage même du panneau, sans Qt.
"""

import json
import os
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, "..", "cassini_qgis"))

import agentview  # noqa: E402
import client  # noqa: E402
import external  # noqa: E402
import runlog  # noqa: E402
import texts  # noqa: E402
from test_tracer import FakeProcess, reply  # noqa: E402

CATALOGUE = [
    {"id": "layer.describe", "description_fr": "Décrit une couche.",
     "description_en": "Describes a layer.", "available": True, "cancellable": False},
    {"id": "analysis.buffer", "description_fr": "Fait une zone tampon.",
     "description_en": "Makes a buffer.", "available": True, "cancellable": True},
]


def notification(method, params):
    return (json.dumps({"jsonrpc": "2.0", "method": method, "params": params}) + "\n").encode()


def request(ident, method, params):
    message = {"jsonrpc": "2.0", "id": ident, "method": method, "params": params}
    return (json.dumps(message) + "\n").encode()


def error(code, message):
    return {"code": -32000, "message": message, "data": {"code": code, "message": message}}


class Panel:
    """Ce que garde le panneau ; les événements passent par `external.apply`."""

    def __init__(self, ask=False):
        self.state = external.ExternalState(ask=ask)
        self.log = runlog.RunLog()
        self.outs = []

    def on_event(self, kind, payload):
        self.outs.append(
            external.apply(self.state, self.log, kind, payload, CATALOGUE, "fr", "10:00")
        )


class ExternalTest(unittest.TestCase):
    def setUp(self):
        self.p = FakeProcess()
        self.events = []
        self.panel = Panel()

        def on_event(kind, payload):
            self.events.append((kind, payload))
            if kind in client.EXTERNAL_EVENTS:
                self.panel.on_event(kind, payload)

        self.c = client.CoreClient(self.p, on_event)
        self.c.start("cassini", "/p.qgz")
        self.p.feed(reply(1, {"protocol_version": 2, "core_version": "0.1", "commands": CATALOGUE}))
        self.p.feed(reply(2, {"core_version": "0.1", "protocol_version": 2}))
        self.events.clear()
        self.p.written.clear()

    def sent(self):
        return [json.loads(w) for w in self.p.written]

    def kinds(self):
        return [k for k, _ in self.events]

    def controls(self):
        return external.controls(self.panel.state, True, CATALOGUE)

    def switch_on(self, ask=False):
        self.assertTrue(self.c.set_external(True, ask))
        ident = self.sent()[-1]["id"]
        self.p.feed(reply(ident, {"enabled": True, "ask": ask}))

    # -- Allumer -----------------------------------------------------------------
    def test_switch_on_sends_session_external_and_shows_the_banner(self):
        self.assertFalse(self.controls()["switch_checked"])  # éteint au démarrage
        self.assertTrue(self.c.set_external(True, True))
        sent = self.sent()[-1]
        self.assertEqual(sent["method"], "session.external")
        self.assertEqual(sent["params"], {"enabled": True, "ask": True})
        self.assertFalse(self.controls()["banner_visible"])  # pas avant la réponse du noyau
        self.p.feed(reply(sent["id"], {"enabled": True, "ask": True}))
        self.assertIn(client.EXTERNAL_STATE, self.kinds())
        c = self.controls()
        self.assertTrue(c["switch_checked"] and c["banner_visible"] and c["cut_enabled"])

    def test_switch_is_served_while_a_command_runs(self):
        self.assertTrue(self.c.run_command("analysis.buffer", {}))
        self.assertTrue(self.c.set_external(True, False))

    def test_ask_changed_before_the_reply_is_resent(self):
        self.panel.state.ask = False
        self.assertTrue(self.c.set_external(True, False))
        ident = self.sent()[-1]["id"]
        self.panel.state.ask = True  # case cochée avant la réponse du noyau
        self.p.feed(reply(ident, {"enabled": True, "ask": False}))
        self.assertTrue(self.panel.outs[-1]["resend_ask"])
        out = external.apply(self.panel.state, self.panel.log, client.EXTERNAL_STATE,
                             {"enabled": True, "ask": True}, CATALOGUE, "fr", "10:00")
        self.assertFalse(out["resend_ask"])
        out = external.apply(self.panel.state, self.panel.log, client.EXTERNAL_STATE,
                             {"enabled": False}, CATALOGUE, "fr", "10:00")
        self.assertFalse(out["resend_ask"])  # canal éteint : rien à renvoyer

    def test_ask_setting(self):
        for value, expected in (("true", True), (True, True), (1, True),
                                ("false", False), (None, False)):
            with self.subTest(value=value):
                self.assertIs(external.ask_setting(value), expected)

    def test_event_kinds_match_the_client(self):
        self.assertEqual(
            client.EXTERNAL_EVENTS,
            ("external_state", "external_failed", "external_call_started",
             "external_call_finished", "approval_asked", "approval_withdrawn"),
        )

    def test_nothing_is_sent_before_the_core_is_ready(self):
        p = FakeProcess()
        c = client.CoreClient(p, lambda k, v: None)
        c.start("cassini", "/p.qgz")
        p.written.clear()
        self.assertFalse(c.set_external(True, False))
        self.assertEqual(p.written, [])

    def test_channel_taken_greys_the_switch_until_settings_reopen(self):
        self.c.set_external(True, False)
        ident = self.sent()[-1]["id"]
        self.p.feed(reply(ident, error=error("external.channel_taken", "Canal pris.")))
        c = self.controls()
        self.assertFalse(c["switch_checked"] or c["switch_enabled"])
        self.assertEqual(c["switch_reason"], "external_taken")
        self.assertEqual(
            texts.tr("fr", "external_taken"), "Une autre session QGIS accepte déjà l'IA externe."
        )
        self.panel.state.settings_opened()
        self.assertTrue(self.controls()["switch_enabled"])

    def test_other_failure_leaves_the_switch_off_with_its_message(self):
        self.c.set_external(True, False)
        ident = self.sent()[-1]["id"]
        self.p.feed(reply(ident, error=error("external.io", "Dossier ouvert aux autres.")))
        c = self.controls()
        self.assertFalse(c["switch_checked"])
        self.assertTrue(c["switch_enabled"])
        self.assertEqual(c["switch_error"], ("external.io", "Dossier ouvert aux autres."))

    # -- Couper ------------------------------------------------------------------
    def test_failed_cut_keeps_the_channel_shown_on(self):
        self.switch_on()
        self.c.set_external(False, False)
        ident = self.sent()[-1]["id"]
        self.p.feed(reply(ident, error=error("external.io", "Panne.")))
        c = self.controls()
        self.assertTrue(c["switch_checked"] and c["banner_visible"])  # le noyau le garde ouvert
        self.assertEqual(c["switch_error"], ("external.io", "Panne."))

    def test_cut_sends_enabled_false_and_hides_the_banner(self):
        self.switch_on()
        self.assertTrue(self.c.set_external(False, True))
        sent = self.sent()[-1]
        self.assertEqual(sent["params"], {"enabled": False})  # ni `ask` ni rien d'autre
        self.p.feed(reply(sent["id"], {"enabled": False}))
        self.assertFalse(self.controls()["banner_visible"])

    # -- Appel -------------------------------------------------------------------
    def test_call_started_then_finished_makes_one_external_line(self):
        self.switch_on()
        written = len(self.p.written)
        self.p.feed(notification("external.call", {
            "call_id": "external-3", "event": "started", "command": "layer.describe",
            "params": {"layer_id": "l1"}}))
        entry = self.panel.log.entries[-1]
        self.assertEqual((entry.origin, entry.call_id, entry.status),
                         (runlog.EXTERNAL, "external-3", runlog.RUNNING))
        self.assertEqual(entry.label, "IA externe · Décrit une couche.")
        self.assertEqual(entry.params, {"layer_id": "l1"})
        self.assertTrue(external.busy(self.panel.state))
        self.p.feed(notification("external.call", {
            "call_id": "external-3", "event": "finished", "command": "layer.describe",
            "result": {"name": "Gares"}, "duration_ms": 41}))
        self.assertEqual(len(self.panel.log.entries), 1)
        self.assertEqual(entry.status, runlog.DONE)
        self.assertEqual(entry.result, {"name": "Gares"})
        self.assertEqual(self.panel.outs[-1]["announce"],
                         "IA externe · Décrit une couche. : Terminée.")
        line = runlog.line_text(entry, texts.tr("fr", "done"),
                                external.duration_text(entry.duration_ms, "fr"))
        for part in ("Terminée", "IA externe", "41 ms"):
            self.assertIn(part, line)
        self.assertFalse(external.busy(self.panel.state))
        self.assertEqual(len(self.p.written), written)  # une notification ne reçoit rien

    def test_finished_with_error_or_cancel(self):
        for code, status in (("qgis.layer_missing", runlog.ERROR),
                             ("registry.cancelled", runlog.CANCELLED)):
            with self.subTest(code=code):
                call_id = "external-" + code
                self.p.feed(notification("external.call", {
                    "call_id": call_id, "event": "started", "command": "analysis.buffer",
                    "params": {}}))
                self.p.feed(notification("external.call", {
                    "call_id": call_id, "event": "finished", "command": "analysis.buffer",
                    "error": {"code": code, "message": "m"}, "duration_ms": 1500}))
                entry = self.panel.log.entries[-1]
                self.assertEqual(entry.status, status)
                if status == runlog.ERROR:
                    self.assertEqual(entry.error, (code, "m"))
        self.assertEqual(external.duration_text(1500, "fr"), "1,5 s")
        self.assertEqual(external.duration_text(1500, "en"), "1.5 s")

    def test_finished_without_started_makes_a_closed_line(self):
        self.p.feed(notification("external.call", {
            "call_id": "external-9", "event": "finished", "command": "layer.describe",
            "result": {}, "duration_ms": 3}))
        entry = self.panel.log.entries[-1]
        self.assertEqual((entry.status, entry.origin, entry.duration_ms),
                         (runlog.DONE, runlog.EXTERNAL, 3))
        self.assertFalse(external.busy(self.panel.state))

    def test_panel_command_never_closes_an_external_line(self):
        log = self.panel.log
        mine = log.start("system.about", "system.about", "10:00")
        self.p.feed(notification("external.call", {
            "call_id": "external-1", "event": "started", "command": "layer.describe",
            "params": {}}))
        log.finish(runlog.DONE)
        self.assertEqual(mine.status, runlog.DONE)
        self.assertEqual(log.find_call("external-1").status, runlog.RUNNING)

    # -- Arrêter -----------------------------------------------------------------
    def test_stop_sends_request_cancel_with_the_call_id(self):
        self.p.feed(notification("external.call", {
            "call_id": "external-4", "event": "started", "command": "analysis.buffer",
            "params": {}}))
        c = self.controls()
        self.assertTrue(c["stop_visible"] and c["stop_enabled"])
        self.assertEqual(c["stop_call"], "external-4")
        self.assertTrue(self.c.cancel_external(c["stop_call"]))
        sent = self.sent()[-1]
        self.assertEqual((sent["method"], sent["params"]), ("request.cancel", {"id": "external-4"}))

    def test_stop_is_greyed_with_its_reason_when_not_cancellable(self):
        self.p.feed(notification("external.call", {
            "call_id": "external-5", "event": "started", "command": "layer.describe",
            "params": {}}))
        c = self.controls()
        self.assertTrue(c["stop_visible"])
        self.assertFalse(c["stop_enabled"])
        self.assertEqual(c["stop_reason"], "not_cancellable")

    # -- Accord ------------------------------------------------------------------
    def approve(self, ident="core-4", call_id="external-3", params=None):
        self.p.feed(request(ident, "external.approve", {
            "call_id": call_id, "command": "layer.describe",
            "params": params if params is not None else {"layer_id": "l1"}}))

    def test_approval_question_and_both_answers(self):
        for approved in (True, False):
            with self.subTest(approved=approved):
                self.p.written.clear()
                self.approve()
                self.assertEqual(self.p.written, [])  # pas d'accord par défaut
                c = self.controls()
                self.assertTrue(c["question_visible"] and c["answer_enabled"])
                self.assertTrue(external.busy(self.panel.state))
                self.assertTrue(self.c.answer_approval("core-4", approved))
                self.panel.state.answered()
                self.assertEqual(self.sent(), [
                    {"jsonrpc": "2.0", "id": "core-4", "result": {"approved": approved}}])
                self.assertFalse(self.controls()["question_visible"])

    def test_question_text_masks_absolute_paths(self):
        self.approve(params={"path": "C:\\Users\\kev\\data\\a.gpkg", "n": 1})
        text = external.question_text(self.panel.state, CATALOGUE, "fr")
        self.assertIn("Décrit une couche.", text)
        self.assertIn("…/a.gpkg", text)
        self.assertNotIn("Users", text)

    def test_second_request_refuses_the_first(self):
        self.approve("core-4", "external-3")
        self.approve("core-5", "external-4")
        self.assertEqual(self.sent(), [
            {"jsonrpc": "2.0", "id": "core-4", "result": {"approved": False}}])
        self.assertEqual(self.panel.state.approval["request_id"], "core-5")
        self.assertFalse(self.c.answer_approval("core-4", True))  # déjà soldée
        self.assertEqual(len(self.p.written), 1)

    # -- Expiration --------------------------------------------------------------
    def test_withdrawal_closes_the_question_and_a_late_click_sends_nothing(self):
        self.approve()
        self.p.feed(notification("request.cancel", {"id": "core-4"}))
        self.assertIn(client.APPROVAL_WITHDRAWN, self.kinds())
        self.assertFalse(self.controls()["question_visible"])
        self.assertFalse(self.c.answer_approval("core-4", True))
        self.assertEqual(self.p.written, [])
        # Critère d'acceptation : la saisie redevient active.
        view = agentview.AgentView()
        c = agentview.controls(view, True, False, True, CATALOGUE,
                               external.busy(self.panel.state))
        self.assertTrue(c["input_enabled"] and c["send_enabled"])

    # -- Occupé ------------------------------------------------------------------
    def test_busy_greys_input_run_and_approve_with_the_external_reason(self):
        self.approve()
        busy = external.busy(self.panel.state)
        self.assertTrue(busy)
        view = agentview.AgentView()
        c = agentview.controls(view, True, False, True, CATALOGUE, busy)
        self.assertFalse(c["input_enabled"] or c["send_enabled"])
        self.assertEqual(c["input_reason"], "input_reason_external")
        view.proposed("plan-1", {"calls": [{"command": "layer.describe", "params": {}}]})
        c = agentview.controls(view, True, False, True, CATALOGUE, busy)
        self.assertFalse(c["approve_enabled"] or c["reject_enabled"])
        self.assertEqual(c["input_reason"], "input_reason_external")
        self.assertFalse(runlog.controls(True, True, False, True, busy)["run_enabled"])
        self.assertTrue(runlog.controls(True, True, False, True, False)["run_enabled"])
        for lang in ("fr", "en"):
            self.assertTrue(texts.tr(lang, "input_reason_external"))

    # -- Mort --------------------------------------------------------------------
    def test_core_death_forgets_the_question_and_cancels_running_lines(self):
        self.switch_on()
        self.p.feed(notification("external.call", {
            "call_id": "external-6", "event": "started", "command": "analysis.buffer",
            "params": {}}))
        self.approve("core-7", "external-7")
        self.p.written.clear()
        self.p.finished.emit(0)
        self.assertIn(client.DIED, self.kinds())
        self.assertFalse(self.c.answer_approval("core-7", True))
        self.assertEqual(self.p.written, [])
        # Le panneau, à la mort comme à la relance (`_drop_stale_run`), éteint tout.
        self.panel.state.lost(self.panel.log)
        c = self.controls()
        self.assertFalse(c["switch_checked"] or c["banner_visible"] or c["question_visible"])
        self.assertFalse(external.busy(self.panel.state))
        self.assertEqual(self.panel.log.find_call("external-6").status, runlog.CANCELLED)

    def test_relaunched_core_keeps_the_switch_off(self):
        """Critère d'acceptation : noyau relancé après un appel externe."""
        self.panel.state.ask = True  # case cochée par l'utilisateur, retenue par le panneau
        self.switch_on(ask=True)
        self.p.feed(notification("external.call", {
            "call_id": "external-1", "event": "started", "command": "layer.describe",
            "params": {}}))
        self.c.stop()
        self.panel.state.lost(self.panel.log)
        self.assertFalse(self.panel.state.enabled)
        self.assertTrue(self.panel.state.ask)  # la case est retenue, pas l'interrupteur
        self.assertFalse(
            any(e.origin == runlog.EXTERNAL and e.status == runlog.RUNNING
                for e in self.panel.log.entries)
        )
        self.assertIsNone(self.c.approval())


class TextsTest(unittest.TestCase):
    def test_external_keys_in_both_languages(self):
        for key in ("external_switch", "external_ask", "external_banner", "external_cut",
                    "external_tag", "input_reason_external", "approval_allow",
                    "approval_deny", "external_taken"):
            for lang in ("fr", "en"):
                self.assertTrue(texts.tr(lang, key))
        self.assertEqual(texts.tr("fr", "external_switch"), "Autoriser une IA externe")
        self.assertEqual(texts.tr("fr", "external_ask"), "Me demander avant chaque appel")


if __name__ == "__main__":
    unittest.main()
