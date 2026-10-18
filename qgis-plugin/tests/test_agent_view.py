"""Tests sans QGIS de l'état affiché du plan : statuts, marqueur, libellés, activations."""

import os
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "cassini_qgis"))

import agentview as av  # noqa: E402
import texts  # noqa: E402

CATALOGUE = [
    {"id": "analysis.buffer", "description_fr": "Fait une zone tampon autour des entités d'une couche.",
     "description_en": "Buffers the features of a layer.", "cancellable": True, "available": True},
    {"id": "layout.export_pdf", "description_fr": "Exporte la carte en PDF.", "description_en": "",
     "cancellable": False, "available": True},
]
PLAN = {"kind": "calls", "calls": [
    {"command": "analysis.buffer", "params": {"layer_id": "gares_1", "path": "C:/x/y.gpkg"}},
    {"command": "layout.export_pdf", "params": {"title": "Gares"}},
    {"command": "inconnue.cmd", "params": {}},
]}


def ctl(view, ready=True, other_busy=False, has_text=True):
    return av.controls(view, ready, other_busy, has_text, CATALOGUE)


class AgentViewTest(unittest.TestCase):
    def setUp(self):
        self.v = av.AgentView()

    def statuses(self):
        return [s.status for s in self.v.steps]

    def test_idle_ready_input_active(self):
        c = ctl(self.v)
        self.assertTrue(c["input_enabled"])
        self.assertTrue(c["send_enabled"])
        self.assertIsNone(c["input_reason"])
        self.assertFalse(ctl(self.v, has_text=False)["send_enabled"])
        self.assertTrue(c["expert_enabled"])
        for k in ("approve_visible", "reject_visible", "stop_visible", "progress_visible"):
            self.assertFalse(c[k], k)

    def test_not_ready_greys_everything_without_own_reason(self):
        c = ctl(self.v, ready=False)
        self.assertFalse(c["input_enabled"])
        self.assertFalse(c["send_enabled"])
        self.assertIsNone(c["input_reason"])  # le bandeau dit pourquoi
        self.assertFalse(c["expert_enabled"])

    def test_expert_command_in_flight_greys_input(self):
        c = ctl(self.v, other_busy=True)
        self.assertFalse(c["input_enabled"])
        self.assertEqual(c["input_reason"], "input_reason")

    def test_generating_shows_indicator_without_stop(self):
        self.v.generating()
        c = ctl(self.v)
        self.assertTrue(c["progress_visible"])
        self.assertFalse(c["stop_visible"])
        self.assertFalse(c["input_enabled"])
        self.assertFalse(c["expert_enabled"])
        self.assertEqual(c["input_reason"], "input_reason")
        self.assertEqual(av.notice_text(self.v, "fr"), texts.tr("fr", "generating"))

    def test_proposed_plan(self):
        self.v.generating()
        self.v.proposed("plan-1", PLAN)
        self.assertEqual(self.v.plan_id, "plan-1")
        self.assertEqual(self.statuses(), [av.PENDING] * 3)
        self.assertIsNone(av.notice_text(self.v, "fr"))
        c = ctl(self.v)
        self.assertTrue(c["approve_visible"] and c["approve_enabled"])
        self.assertTrue(c["reject_visible"] and c["reject_enabled"])
        self.assertFalse(c["input_enabled"])
        self.assertEqual(c["input_reason"], "input_reason_plan")
        self.assertFalse(c["progress_visible"])
        self.v.start_rejecting()
        c = ctl(self.v)
        self.assertFalse(c["approve_enabled"] or c["reject_enabled"])

    def test_labels_from_the_registry_with_fallback(self):
        self.v.proposed("plan-1", PLAN)
        s1, s2, s3 = self.v.steps
        self.assertEqual(av.step_label(s1, CATALOGUE, "fr"),
                         "1. Fait une zone tampon autour des entités d'une couche.")
        self.assertEqual(av.step_label(s1, CATALOGUE, "en"), "1. Buffers the features of a layer.")
        self.assertEqual(av.step_label(s2, CATALOGUE, "en"), "2. Exporte la carte en PDF.")
        self.assertEqual(av.step_label(s3, CATALOGUE, "fr"), "3. inconnue.cmd")

    def test_line_has_marker_icon_and_word(self):
        self.v.proposed("plan-1", PLAN)
        self.v.executing()
        self.v.step(1, "analysis.buffer")
        s1, s2, _ = self.v.steps
        line = av.step_line(s1, CATALOGUE, "fr", self.v.current)
        self.assertTrue(line.startswith(av.MARKER))
        self.assertIn(av.ICONS[av.RUNNING], line)
        self.assertIn(texts.tr("fr", "running"), line)
        line2 = av.step_line(s2, CATALOGUE, "fr", self.v.current)
        self.assertFalse(line2.startswith(av.MARKER))
        self.assertIn(texts.tr("fr", "pending"), line2)
        self.assertNotIn("C:/x", line + line2)  # aucun paramètre ni chemin à l'écran

    def test_execution_advances_the_marker(self):
        self.v.proposed("plan-1", PLAN)
        self.v.executing()
        self.assertTrue(ctl(self.v)["stop_visible"])
        self.v.step(1, "analysis.buffer")
        self.assertEqual(self.statuses(), [av.RUNNING, av.PENDING, av.PENDING])
        self.v.step(2, "layout.export_pdf")
        self.assertEqual(self.statuses(), [av.DONE, av.RUNNING, av.PENDING])
        self.assertEqual(self.v.current, 2)
        self.v.step(3, "inconnue.cmd")
        self.v.done([{"step": 1}, {"step": 2}, {"step": 3}])
        self.assertEqual(self.statuses(), [av.DONE] * 3)
        self.assertEqual(self.v.phase, av.IDLE)
        self.assertIsNone(self.v.plan_id)
        self.assertTrue(ctl(self.v)["input_enabled"])

    def test_stop_follows_the_step_command(self):
        self.v.proposed("plan-1", PLAN)
        self.v.executing()
        c = ctl(self.v)
        self.assertFalse(c["stop_enabled"])  # aucune étape annoncée encore
        self.v.step(1, "analysis.buffer")
        c = ctl(self.v)
        self.assertTrue(c["stop_enabled"])
        self.assertIsNone(c["stop_reason"])
        self.v.step(2, "layout.export_pdf")
        c = ctl(self.v)
        self.assertFalse(c["stop_enabled"])
        self.assertEqual(c["stop_reason"], "not_cancellable")
        self.assertFalse(ctl(self.v, ready=False)["stop_enabled"])

    def test_step_error_leaves_following_steps_unmarked(self):
        self.v.proposed("plan-1", PLAN)
        self.v.executing()
        self.v.step(1, "analysis.buffer")
        self.v.step(2, "layout.export_pdf")
        self.v.exec_failed(2)
        self.assertEqual(self.statuses(), [av.DONE, av.ERROR, av.PENDING])
        self.assertEqual(self.v.phase, av.IDLE)
        self.assertTrue(ctl(self.v)["input_enabled"])

    def test_cancelled_step(self):
        self.v.proposed("plan-1", PLAN)
        self.v.executing()
        self.v.step(1, "analysis.buffer")
        self.v.cancelled(None)
        self.assertEqual(self.statuses(), [av.CANCELLED, av.PENDING, av.PENDING])
        self.assertEqual(self.v.phase, av.IDLE)

    def test_rejected(self):
        self.v.proposed("plan-1", PLAN)
        self.v.rejected()
        self.assertEqual(self.v.steps, [])
        self.assertIsNone(self.v.plan_id)
        self.assertEqual(av.notice_text(self.v, "fr"), "Plan refusé. Reformulez votre demande.")
        self.assertTrue(ctl(self.v)["input_enabled"])

    def test_question_is_a_reply_without_approve(self):
        self.v.generating()
        self.v.question("Laquelle ?")
        c = ctl(self.v)
        self.assertFalse(c["approve_visible"])
        self.assertTrue(c["input_enabled"])
        self.assertEqual(av.notice_text(self.v, "en"), "Laquelle ?")

    def test_proposal_failure(self):
        self.v.generating()
        self.v.failed("agent.refused", "Aucune commande ne fait cela.")
        self.assertEqual(
            av.notice_text(self.v, "fr"),
            "La demande n'a pas abouti (agent.refused) : Aucune commande ne fait cela.",
        )
        self.assertTrue(ctl(self.v)["input_enabled"])

    def test_forget_on_core_death(self):
        self.v.proposed("plan-1", PLAN)
        self.v.executing()
        self.v.step(1, "analysis.buffer")
        self.v.forget()
        self.assertEqual(self.statuses(), [av.ERROR, av.PENDING, av.PENDING])
        self.assertIsNone(self.v.plan_id)
        self.assertEqual(self.v.phase, av.IDLE)
        self.v.proposed("plan-2", PLAN)
        self.v.forget()
        self.assertEqual(self.v.steps, [])
        self.assertIsNone(self.v.plan_id)

    def test_empty_plan_keeps_approve_and_reject(self):
        self.v.proposed("plan-1", {"kind": "calls", "calls": []})
        self.assertEqual(self.v.steps, [])
        c = ctl(self.v)
        self.assertTrue(c["approve_visible"] and c["approve_enabled"])
        self.assertTrue(c["reject_visible"] and c["reject_enabled"])
        self.assertFalse(c["input_enabled"])

    def test_done_ignores_boolean_steps(self):
        self.v.proposed("plan-1", PLAN)
        self.v.executing()
        self.v.done([{"step": True}, {"step": 2}])
        self.assertEqual(self.statuses(), [av.PENDING, av.DONE, av.PENDING])

    def test_malformed_plan_does_not_raise(self):
        self.v.proposed("plan-1", {"kind": "calls", "calls": "x"})
        self.assertEqual(self.v.steps, [])
        self.v.proposed("plan-1", {"calls": [None, {"command": 3}]})
        self.assertEqual([s.command for s in self.v.steps], ["?", "?"])


class OutcomeTest(unittest.TestCase):
    def setUp(self):
        self.v = av.AgentView()
        self.v.proposed("plan-1", PLAN)
        self.v.executing()
        self.v.step(2, "layout.export_pdf")

    def test_done_counts_steps(self):
        last = av.outcome(av.DONE, {"steps": [{"step": 1}, {"step": 2}]}, self.v)
        self.assertEqual(last["n"], 2)
        self.assertTrue(last["plan"])
        self.assertEqual(av.outcome_text(last, "fr"), "Plan exécuté. Étapes terminées : 2.")
        self.assertEqual(av.outcome_text(av.outcome(av.DONE, {}, self.v), "en"),
                         "Plan run. Steps done: 0.")

    def test_exec_failure_with_step(self):
        last = av.outcome(av.ERROR, {"code": "a.b", "message": "m", "step": 1}, self.v)
        self.assertEqual(last["step"], 1)
        self.assertEqual(av.outcome_text(last, "fr"), "Étape 1 en erreur (a.b) : m")

    def test_exec_failure_without_step_uses_current(self):
        last = av.outcome(av.ERROR, {"code": "client.core_died", "message": "m"}, self.v)
        self.assertEqual(last["step"], 2)
        self.assertEqual(av.outcome_text(last, "fr"), "Étape 2 en erreur (client.core_died) : m")

    def test_exec_failure_before_any_step(self):
        v = av.AgentView()
        v.proposed("plan-1", PLAN)
        v.executing()
        last = av.outcome(av.ERROR, {"code": "client.core_died", "message": "m"}, v)
        self.assertIsNone(last["step"])
        self.assertEqual(av.outcome_text(last, "fr"),
                         "La demande n'a pas abouti (client.core_died) : m")

    def test_cancel_without_step_uses_current(self):
        last = av.outcome(av.CANCELLED, {"step": None}, self.v)
        self.assertEqual(last["step"], 2)
        self.assertEqual(av.outcome_text(last, "fr"),
                         "Étape 2 annulée : les étapes suivantes n'ont pas été lancées.")


class PlanTextsTest(unittest.TestCase):
    def test_plan_keys_exist_in_both_languages(self):
        for key in ("plan_title", "approve", "reject", "plan_rejected", "generating",
                    "question_title", "propose_error", "plan_done", "pending",
                    "input_reason", "input_reason_plan", "plan_cancelled",
                    "plan_steps_name", "live_plan", "live_question"):
            for lang in ("fr", "en"):
                self.assertTrue(texts.tr(lang, key, code="c", message="m", n=2))
        self.assertEqual(texts.tr("fr", "plan_done", n=1), "Plan exécuté. Étapes terminées : 1.")
        self.assertEqual(texts.tr("fr", "approve"), "Valider le plan")
        self.assertEqual(texts.tr("en", "approve"), "Approve plan")


if __name__ == "__main__":
    unittest.main()
