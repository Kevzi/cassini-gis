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
import gestures  # noqa: E402
from test_tracer import FakeGesture, FakeProcess, reply  # noqa: E402

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
        self.p.feed(reply(1, {"protocol_version": 2, "core_version": "0.1", "commands": []}))
        self.p.feed(reply(2, {"core_version": "0.1", "protocol_version": 2}))
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

    def test_propose_sends_the_model_only_when_it_is_a_text(self):
        for model, params in (
            ("b.gguf", {"request": "x", "model": "b.gguf"}),
            (None, {"request": "x"}),
            ("", {"request": "x"}),
            (3, {"request": "x"}),
        ):
            self.assertTrue(self.c.propose("x", model))
            msg = self.last_sent()
            self.assertEqual(msg["params"], params, model)
            self.p.feed(reply(msg["id"], {"question": "?"}))

    # -- modèles ----------------------------------------------------------
    def test_list_models_sends_agent_models_and_returns_the_list(self):
        self.assertTrue(self.c.list_models())
        msg = self.last_sent()
        self.assertEqual((msg["method"], msg["params"]), ("agent.models", {}))
        self.assertTrue(self.c.models_busy())
        # Une seule requête en vol : rien d'autre ne part pendant la liste.
        self.assertFalse(self.c.propose("x"))
        self.assertFalse(self.c.list_models())
        listed = [{"file": "a.gguf", "size_bytes": 5}, "pas un modèle",
                  {"file": "b.modele.json", "size_bytes": 0, "problem": "fichier manquant : c.gguf"}]
        self.p.feed(reply(msg["id"], {"models": listed}))
        self.assertEqual(
            self.events,
            [(client.MODELS, {"models": [listed[0], listed[2]]})],
        )
        self.assertFalse(self.c.models_busy())
        self.assertTrue(self.c.propose("x"))

    def test_list_models_failure_keeps_code_and_message(self):
        for code in ("models.no_folder", "agent.unavailable", "session.busy"):
            self.events.clear()
            self.assertTrue(self.c.list_models())
            self.p.feed(reply(self.last_sent()["id"], error=error(code, "Message français.")))
            self.assertEqual(
                self.events,
                [(client.MODELS_FAILED, {"code": code, "message": "Message français."})],
            )
        self.events.clear()
        self.assertTrue(self.c.list_models())
        self.p.feed(reply(self.last_sent()["id"], {"autre": 1}))
        self.assertEqual(self.events[0][0], client.MODELS_FAILED)
        self.assertEqual(self.events[0][1]["code"], "core.unknown")

    def test_list_models_needs_a_ready_free_client_and_fails_on_death(self):
        fresh = client.CoreClient(FakeProcess(), lambda k, v: None)
        self.assertFalse(fresh.list_models())
        self.assertTrue(self.c.propose("x"))
        self.assertFalse(self.c.list_models())
        self.p.feed(reply(self.last_sent()["id"], {"question": "?"}))
        self.events.clear()
        self.assertTrue(self.c.list_models())
        self.p.finished.emit(0)
        self.assertEqual(self.events[0], (client.MODELS_FAILED,
                                          {"code": client.CORE_DIED,
                                           "message": "Le noyau Cassini s'est arrêté."}))

    def test_question_is_a_result_without_plan_id(self):
        ident = self.propose()
        self.p.feed(reply(ident, {"question": "Laquelle ?", "source": "model"}))
        self.assertEqual(self.events, [(client.PLAN_QUESTION,
                                        {"question": "Laquelle ?", "source": "model"})])

    def test_question_carries_its_source_and_defaults_to_model(self):
        for given, expected in (("guard", "guard"), (None, "model"), ("autre", "model")):
            self.events.clear()
            ident = self.propose()
            result = {"question": "Quel point ?"}
            if given is not None:
                result["source"] = given
            self.p.feed(reply(ident, result))
            self.assertEqual(self.events, [(client.PLAN_QUESTION,
                                            {"question": "Quel point ?", "source": expected})])

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
              {"code": "qgis.layer_missing", "message": "La couche est absente.", "step": 2,
               "outputs": {}})],
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
        self.assertEqual(self.events, [(client.PLAN_CANCELLED, {"step": 1, "outputs": {}})])

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
                                        "message": "Le noyau Cassini s'est arrêté.", "step": None,
                                        "outputs": {}}),
        )
        self.assertEqual(self.events[1][0], client.DIED)

    def test_stop_forgets_the_plan_silently(self):
        self.propose()
        self.c.stop()
        self.assertFalse(self.c.plan_busy())
        self.p.finished.emit(0)
        self.assertEqual(self.events, [])



class PlanOutputsTest(unittest.TestCase):
    """Résultats des ordres soldés pendant un plan, rendus sur erreur et annulation.

    Rien ne change sur le fil : l'extension garde ce qu'elle a elle-même répondu."""

    def setUp(self):
        FakeGesture.instances = []
        reg = gestures.GestureRegistry()
        reg.register("analysis.buffer", FakeGesture)
        reg.register("layout.export_pdf", FakeGesture)
        self.p = FakeProcess()
        self.events = []
        self.c = client.CoreClient(self.p, lambda k, v: self.events.append((k, v)), reg)
        self.c.start("cassini", "/p.qgz")
        self.p.feed(reply(1, {"protocol_version": 2, "core_version": "0.1", "commands": []}))
        self.p.feed(reply(2, {"core_version": "0.1", "protocol_version": 2}))
        self.c.propose("x")
        self.p.feed(reply(3, {"plan_id": "plan-1", "plan": PLAN}))
        self.c.execute_plan("plan-1")
        self.execute = json.loads(self.p.written[-1])["id"]
        self.events.clear()

    def order(self, ident, command):
        m = {"jsonrpc": "2.0", "id": ident, "method": "order.execute",
             "params": {"command": command, "params": {}}}
        self.p.feed((json.dumps(m) + "\n").encode())

    def run_step(self, step, command, ident, result=None, error_=None):
        self.p.feed(notification("plan.step", {"plan_id": "plan-1", "step": step, "command": command}))
        self.order(ident, command)
        FakeGesture.instances[-1].done(result=result, error=error_)

    def test_finished_steps_travel_with_the_failure(self):
        buffer = {"layer_id": "t_1", "name": "Tampon", "feature_count": 8, "crs": "EPSG:2154"}
        self.run_step(1, "analysis.buffer", "core-1", result=buffer)
        written = len(self.p.written)
        self.run_step(2, "layout.export_pdf", "core-2",
                      error_=gestures.GestureError("qgis.layout_failed", "Échec."))
        self.p.feed(reply(self.execute, error=error("qgis.layout_failed", "Échec.", step=2)))
        kind, payload = self.events[-1]
        self.assertEqual(kind, client.PLAN_EXEC_FAILED)
        self.assertEqual(payload["outputs"], {1: buffer})  # l'étape en erreur n'a rien rendu
        # Le fil n'a porté que la réponse de l'ordre, rien de plus.
        self.assertEqual(len(self.p.written), written + 1)

    def test_finished_steps_travel_with_the_cancellation(self):
        self.run_step(1, "analysis.buffer", "core-1", result={"name": "Tampon"})
        self.p.feed(notification("plan.step", {"plan_id": "plan-1", "step": 2, "command": "layout.export_pdf"}))
        self.order("core-2", "layout.export_pdf")
        FakeGesture.instances[-1].cancel()
        self.p.feed(reply(self.execute, error=error("registry.cancelled", "Annulée.", step=2)))
        self.assertEqual(self.events[-1], (client.PLAN_CANCELLED,
                                           {"step": 2, "outputs": {1: {"name": "Tampon"}}}))

    def test_finished_steps_travel_with_core_death(self):
        self.run_step(1, "analysis.buffer", "core-1", result={"name": "Tampon"})
        self.events.clear()
        self.p.finished.emit(0)
        self.assertEqual(self.events[0][1]["outputs"], {1: {"name": "Tampon"}})

    def test_outputs_do_not_leak_into_the_next_plan(self):
        self.run_step(1, "analysis.buffer", "core-1", result={"name": "Tampon"})
        self.p.feed(reply(self.execute, {"plan_id": "plan-1", "steps": []}))
        self.c.propose("y")
        self.p.feed(reply(json.loads(self.p.written[-1])["id"], {"plan_id": "plan-2", "plan": PLAN}))
        self.c.execute_plan("plan-2")
        ident = json.loads(self.p.written[-1])["id"]
        self.p.feed(reply(ident, error=error("plan.bad_reference", "Référence.", step=1)))
        self.assertEqual(self.events[-1][1]["outputs"], {})


if __name__ == "__main__":
    unittest.main()
