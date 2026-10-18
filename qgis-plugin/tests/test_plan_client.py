"""Tests sans QGIS des plans de l'agent côté canal : plan.propose, plan.execute,
plan.reject, notification plan.step, garde d'une seule requête, annulation, mort."""

import json
import os
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, "..", "cassini_qgis"))

import client  # noqa: E402
from test_tracer import FakeProcess, reply  # noqa: E402

PLAN = {
    "kind": "calls",
    "calls": [
        {"command": "analysis.buffer", "params": {"layer_id": "gares_1"}},
        {"command": "layout.export_pdf", "params": {"title": "Gares"}},
    ],
}


def notification(method, params):
    return (json.dumps({"jsonrpc": "2.0", "method": method, "params": params}) + "\n").encode()


def error(code, message, step=None):
    data = {"code": code, "message": message}
    if step is not None:
        data["step"] = step
    return {"code": -32000, "message": message, "data": data}


class PlanClientTest(unittest.TestCase):
    def setUp(self):
        self.p = FakeProcess()
        self.events = []
        self.c = client.CoreClient(self.p, lambda k, v: self.events.append((k, v)))
        self.c.start("cassini", "/p.qgz")
        self.p.feed(reply(1, {"protocol_version": 1, "core_version": "0.1", "commands": []}))
        self.p.feed(reply(2, {"core_version": "0.1", "protocol_version": 1}))
        self.events.clear()
        self.p.written.clear()

    def sent(self):
        return [json.loads(w) for w in self.p.written]

    def last_sent(self):
        return self.sent()[-1]

    def propose(self, text="Tampon de 500 m autour des gares."):
        self.assertTrue(self.c.propose(text))
        return self.last_sent()["id"]

    def proposed(self):
        ident = self.propose()
        self.p.feed(reply(ident, {"plan_id": "plan-1", "plan": PLAN}))
        self.events.clear()

    # -- proposition ------------------------------------------------------
    def test_propose_sends_the_request_as_is(self):
        ident = self.propose("  Tampon  ")
        msg = self.last_sent()
        self.assertEqual(msg["method"], "plan.propose")
        self.assertEqual(msg["params"], {"request": "  Tampon  "})
        self.assertTrue(self.c.plan_busy())
        self.p.feed(reply(ident, {"plan_id": "plan-1", "plan": PLAN}))
        self.assertEqual(self.events, [(client.PLAN_PROPOSED, {"plan_id": "plan-1", "plan": PLAN})])
        self.assertFalse(self.c.plan_busy())

    def test_question_is_a_result_without_plan_id(self):
        ident = self.propose()
        self.p.feed(reply(ident, {"question": "Laquelle ?"}))
        self.assertEqual(self.events, [(client.PLAN_QUESTION, {"question": "Laquelle ?"})])

    def test_propose_failure_keeps_code_and_french_message(self):
        for code in ("agent.refused", "ai.no_model", "agent.plan_invalid"):
            self.events.clear()
            ident = self.propose()
            self.p.feed(reply(ident, error=error(code, "Message français.")))
            self.assertEqual(
                self.events, [(client.PLAN_FAILED, {"code": code, "message": "Message français."})]
            )
            self.assertFalse(self.c.plan_busy())

    def test_unreadable_answer_is_a_failure(self):
        ident = self.propose()
        self.p.feed(reply(ident, {"plan": PLAN}))
        self.assertEqual(self.events[0][0], client.PLAN_FAILED)

    def test_inventory_order_during_proposal_is_served(self):
        self.propose()
        self.p.feed(
            (json.dumps({"jsonrpc": "2.0", "id": "core-1", "method": "order.execute",
                         "params": {"command": "project.inventory", "params": {}}}) + "\n").encode()
        )
        answer = self.last_sent()
        self.assertEqual(answer["id"], "core-1")
        self.assertIn("error", answer)  # aucun geste dans ce double : le noyau tranchera
        self.assertTrue(self.c.plan_busy())

    # -- une seule requête en vol ------------------------------------------------
    def test_one_request_in_flight(self):
        self.propose()
        self.assertFalse(self.c.propose("autre"))
        self.assertFalse(self.c.run_command("system.about"))
        self.assertFalse(self.c.replay())
        self.assertFalse(self.c.execute_plan("plan-1"))
        self.assertFalse(self.c.reject_plan("plan-1"))
        self.assertEqual(len(self.p.written), 1)

    def test_no_plan_while_a_command_runs(self):
        self.assertTrue(self.c.run_command("system.about"))
        self.assertFalse(self.c.propose("x"))
        self.assertFalse(self.c.execute_plan("plan-1"))

    def test_not_ready_sends_nothing(self):
        c = client.CoreClient(FakeProcess(), lambda k, v: None)
        self.assertFalse(c.propose("x"))
        self.assertFalse(c.execute_plan("plan-1"))
        self.assertFalse(c.reject_plan("plan-1"))

    # -- exécution ----------------------------------------------------------
    def test_execute_sends_only_the_plan_id_and_follows_steps(self):
        self.proposed()
        self.assertTrue(self.c.execute_plan("plan-1"))
        msg = self.last_sent()
        self.assertEqual(msg["method"], "plan.execute")
        self.assertEqual(msg["params"], {"plan_id": "plan-1"})
        ident = msg["id"]
        count = len(self.p.written)
        self.p.feed(notification("plan.step", {"plan_id": "plan-1", "step": 1, "command": "analysis.buffer"}))
        self.p.feed(notification("plan.step", {"plan_id": "plan-1", "step": 2, "command": "layout.export_pdf"}))
        self.assertEqual(len(self.p.written), count)  # notification : jamais de réponse
        steps = [{"step": 1, "command": "analysis.buffer", "result": {}},
                 {"step": 2, "command": "layout.export_pdf", "result": {}}]
        self.p.feed(reply(ident, {"plan_id": "plan-1", "steps": steps}))
        self.assertEqual(
            self.events,
            [
                (client.PLAN_STEP, {"step": 1, "command": "analysis.buffer"}),
                (client.PLAN_STEP, {"step": 2, "command": "layout.export_pdf"}),
                (client.PLAN_DONE, {"steps": steps}),
            ],
        )
        self.assertFalse(self.c.plan_busy())

    def test_plan_step_with_an_id_gets_no_answer(self):
        self.proposed()
        self.c.execute_plan("plan-1")
        count = len(self.p.written)
        self.p.feed((json.dumps({"jsonrpc": "2.0", "id": 99, "method": "plan.step",
                                 "params": {"step": 1, "command": "a"}}) + "\n").encode())
        self.assertEqual(len(self.p.written), count)

    def test_boolean_step_is_ignored(self):
        self.proposed()
        self.c.execute_plan("plan-1")
        self.p.feed(notification("plan.step", {"plan_id": "plan-1", "step": True, "command": "a"}))
        self.assertEqual(self.events, [])

    def test_stray_plan_step_is_ignored(self):
        self.p.feed(notification("plan.step", {"plan_id": "plan-1", "step": 1, "command": "a"}))
        self.assertEqual(self.events, [])
        self.assertEqual(self.p.written, [])

    def test_step_error_carries_its_step(self):
        self.proposed()
        self.c.execute_plan("plan-1")
        ident = self.last_sent()["id"]
        self.p.feed(reply(ident, error=error("qgis.layer_missing", "La couche est absente.", step=2)))
        self.assertEqual(
            self.events,
            [(client.PLAN_EXEC_FAILED,
              {"code": "qgis.layer_missing", "message": "La couche est absente.", "step": 2})],
        )

    def test_cancel_targets_the_plan_execute(self):
        self.proposed()
        self.c.execute_plan("plan-1")
        ident = self.last_sent()["id"]
        self.assertTrue(self.c.cancel_run())
        cancel = self.last_sent()
        self.assertEqual(cancel["method"], "request.cancel")
        self.assertEqual(cancel["params"], {"id": ident})
        self.p.feed(reply(cancel["id"], {"cancelled": True}))
        self.assertEqual(self.events, [])
        self.p.feed(reply(ident, error=error("registry.cancelled", "Annulée.", step=1)))
        self.assertEqual(self.events, [(client.PLAN_CANCELLED, {"step": 1})])

    def test_no_cancel_during_proposal(self):
        self.propose()
        self.assertFalse(self.c.cancel_run())
        self.assertEqual(len(self.p.written), 1)

    # -- refus ------------------------------------------------------------
    def test_reject(self):
        self.proposed()
        self.assertTrue(self.c.reject_plan("plan-1"))
        msg = self.last_sent()
        self.assertEqual((msg["method"], msg["params"]), ("plan.reject", {"plan_id": "plan-1"}))
        self.p.feed(reply(msg["id"], {"plan_id": "plan-1", "rejected": True}))
        self.assertEqual(self.events, [(client.PLAN_REJECTED, {"plan_id": "plan-1"})])
        self.assertTrue(self.c.propose("nouvelle demande"))

    def test_reject_unknown_plan(self):
        self.proposed()
        self.c.reject_plan("plan-0")
        self.p.feed(reply(self.last_sent()["id"], error=error("plan.unknown", "Plan inconnu.")))
        self.assertEqual(self.events, [(client.PLAN_FAILED, {"code": "plan.unknown", "message": "Plan inconnu."})])

    # -- mort du noyau --------------------------------------------------------
    def test_core_death_during_generation(self):
        self.propose()
        self.p.finished.emit(0)
        kinds = [k for k, _ in self.events]
        self.assertEqual(kinds, [client.PLAN_FAILED, client.DIED])
        self.assertEqual(self.events[0][1]["code"], client.CORE_DIED)
        self.assertFalse(self.c.plan_busy())

    def test_core_death_during_execution(self):
        self.proposed()
        self.c.execute_plan("plan-1")
        self.p.feed(notification("plan.step", {"plan_id": "plan-1", "step": 1, "command": "a"}))
        self.events.clear()
        self.p.errorOccurred.emit(1)
        self.assertEqual(
            self.events[0],
            (client.PLAN_EXEC_FAILED, {"code": client.CORE_DIED,
                                        "message": "Le noyau Cassini s'est arrêté.", "step": None}),
        )
        self.assertEqual(self.events[1][0], client.DIED)

    def test_stop_forgets_the_plan_silently(self):
        self.propose()
        self.c.stop()
        self.assertFalse(self.c.plan_busy())
        self.p.finished.emit(0)
        self.assertEqual(self.events, [])


if __name__ == "__main__":
    unittest.main()
