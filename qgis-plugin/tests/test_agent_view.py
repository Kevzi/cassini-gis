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

    def test_labels_are_short_sentences_with_a_short_fallback(self):
        self.v.proposed("plan-1", PLAN)
        s1, s2, s3 = self.v.steps
        # Paramètres incomplets (pas de distance) : titre court du registre, sans point.
        self.assertEqual(av.step_label(s1, CATALOGUE, "fr"),
                         "1. Fait une zone tampon autour des entités d'une couche")
        self.assertEqual(av.step_label(s1, CATALOGUE, "en"), "1. Buffers the features of a layer")
        # Paramètres lisibles : la phrase tirée des paramètres, dans la langue.
        self.assertEqual(av.step_label(s2, CATALOGUE, "fr"), "2. Export PDF de la carte « Gares »")
        self.assertEqual(av.step_label(s2, CATALOGUE, "en"), '2. PDF export of the map "Gares"')
        self.assertEqual(av.step_label(s3, CATALOGUE, "fr"), "3. inconnue.cmd")

    def test_a_slope_step_reads_as_a_short_sentence_not_the_registry_description(self):
        description = (
            "Pente d'un MNT : calcule la pente de chaque cellule d'un modèle numérique "
            "d'élévation (fichier GeoTIFF désigné par `dem`, chemin de fichier MNT recopié "
            "de l'inventaire, pas un id de couche)."
        )
        catalogue = [{"id": "terrain.slope", "description_fr": description,
                      "description_en": "DEM slope: computes the slope of each cell."}]
        self.v.proposed("plan-1", {"calls": [{"command": "terrain.slope", "params": {
            "dem": "C:/donnees/annecy/mnt-74-25m.tif", "unit": "percent"}}]})
        (step,) = self.v.steps
        self.assertEqual(av.step_label(step, catalogue, "fr"),
                         "1. Pente de mnt-74-25m en pourcentage")
        self.assertEqual(av.step_label(step, catalogue, "en"), "1. Slope of mnt-74-25m in percent")
        line = av.step_line(step, catalogue, "fr")
        self.assertNotIn("GeoTIFF", line)
        self.assertNotIn("inventaire", line)
        # Paramètres illisibles : le titre avant « : », jamais le texte entier.
        self.v.proposed("plan-2", {"calls": [{"command": "terrain.slope", "params": {
            "dem": "mnt.tif", "unit": "radians"}}]})
        self.assertEqual(av.step_label(self.v.steps[0], catalogue, "fr"), "1. Pente d'un MNT")
        self.assertEqual(av.step_label(self.v.steps[0], catalogue, "en"), "1. DEM slope")

    def test_a_chained_reference_reads_as_the_step_it_cites(self):
        self.v.proposed("plan-1", {"calls": [
            {"command": "analysis.buffer", "params": {
                "layer_id": "gares_1", "distance": {"value": 500, "unit": "m"}}},
            {"command": "analysis.intersect", "params": {
                "layer_id": "parcelles_1", "overlay_layer_id": "step:1.layer_id"}},
        ]})
        s2 = self.v.steps[1]
        self.assertEqual(av.step_label(s2, CATALOGUE, "fr"),
                         "2. Intersection de parcelles_1 avec le résultat de l'étape 1")
        refs = {1: {"layer_id": "buffer_1", "name": "Tampon", "feature_count": 3}}
        names = {"parcelles_1": "Parcelles"}.get
        self.assertEqual(av.step_label(s2, CATALOGUE, "fr", names, refs),
                         "2. Intersection de Parcelles avec Tampon")

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
        self.assertFalse(self.v.restore_request)  # question du modèle : la saisie reste vide
        self.assertIsNone(av.request_to_restore(self.v, "Tampon autour des pharmacies.", ""))

    def test_guard_question_asks_to_restore_the_request(self):
        self.v.generating()
        self.v.question("Il manque un point.", "guard")
        self.assertTrue(self.v.restore_request)
        self.assertEqual(av.notice_text(self.v, "fr"), "Il manque un point.")
        self.assertTrue(ctl(self.v)["input_enabled"])
        self.assertEqual(av.request_to_restore(self.v, "10 min à pied", ""), "10 min à pied")
        self.assertEqual(av.request_to_restore(self.v, "10 min à pied", "  "), "10 min à pied")
        # Une saisie déjà commencée n'est pas écrasée ; sans phrase, rien à rendre.
        self.assertIsNone(av.request_to_restore(self.v, "10 min à pied", "x 652000"))
        self.assertIsNone(av.request_to_restore(self.v, None, ""))
        # Tout nouvel événement l'oublie.
        self.v.generating()
        self.assertFalse(self.v.restore_request)

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


class StepResultsTest(unittest.TestCase):
    """Résultat porté par chaque étape, pour dire au clic ce qu'elle a fait."""

    BUFFER = {"layer_id": "t_1", "name": "Tampon", "feature_count": 8}

    def setUp(self):
        self.v = av.AgentView()
        self.v.proposed("plan-1", PLAN)
        self.v.executing()

    def test_done_keeps_each_step_result(self):
        self.v.step(1, "analysis.buffer")
        self.v.done([{"step": 1, "command": "analysis.buffer", "result": self.BUFFER},
                     {"step": 2, "command": "layout.export_pdf", "result": {"file": "2.pdf"}}])
        self.assertEqual([s.result for s in self.v.steps], [self.BUFFER, {"file": "2.pdf"}, None])
        self.assertEqual(self.v.step_results(), {1: self.BUFFER, 2: {"file": "2.pdf"}})

    def test_failure_keeps_finished_outputs_not_the_failed_step(self):
        self.v.step(1, "analysis.buffer")
        self.v.step(2, "layout.export_pdf")
        self.v.exec_failed(2, {1: self.BUFFER, 2: {"file": "partiel.pdf"}})
        self.assertEqual(self.v.steps[0].result, self.BUFFER)
        self.assertIsNone(self.v.steps[1].result)
        self.assertEqual(self.statuses(), [av.DONE, av.ERROR, av.PENDING])

    def test_failure_without_step_uses_current(self):
        self.v.step(1, "analysis.buffer")
        self.v.step(2, "layout.export_pdf")
        self.v.exec_failed(None, {1: self.BUFFER, 2: {"x": 1}})
        self.assertIsNone(self.v.steps[1].result)

    def test_cancellation_keeps_finished_outputs(self):
        self.v.step(1, "analysis.buffer")
        self.v.step(2, "layout.export_pdf")
        self.v.cancelled(2, {1: self.BUFFER})
        self.assertEqual(self.v.step_results(), {1: self.BUFFER})
        self.assertEqual(self.statuses(), [av.DONE, av.CANCELLED, av.PENDING])

    def test_core_step_without_order_has_no_result(self):
        self.v.step(1, "analysis.buffer")
        self.v.step(2, "layout.export_pdf")
        self.v.exec_failed(2, None)
        self.assertEqual(self.v.step_results(), {})

    def test_error_naming_a_later_step_finishes_the_current_one(self):
        # Le noyau résout `step:k` avant d'annoncer l'étape k : l'étape 2 est finie.
        self.v.step(1, "analysis.buffer")
        self.v.step(2, "layout.export_pdf")
        self.v.exec_failed(3, {2: {"file": "2.pdf"}})
        self.assertEqual(self.statuses(), [av.DONE, av.DONE, av.ERROR])
        self.assertEqual(self.v.steps[1].result, {"file": "2.pdf"})

    def test_cancel_naming_a_later_step_finishes_the_current_one(self):
        self.v.step(1, "analysis.buffer")
        self.v.cancelled(2, {1: self.BUFFER})
        self.assertEqual(self.statuses(), [av.DONE, av.CANCELLED, av.PENDING])

    def test_error_without_step_still_fails_the_current_one(self):
        self.v.step(1, "analysis.buffer")
        self.v.exec_failed(None, None)
        self.assertEqual(self.statuses(), [av.ERROR, av.PENDING, av.PENDING])

    def test_new_proposal_forgets_results(self):
        self.v.done([{"step": 1, "result": self.BUFFER}])
        self.v.proposed("plan-2", PLAN)
        self.assertEqual(self.v.step_results(), {})

    def statuses(self):
        return [s.status for s in self.v.steps]


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


INTERSECTIONS = [{"layer_id": "Intersection_%d" % k, "name": "Intersection"} for k in range(1, 6)]


class LeftOutTest(unittest.TestCase):
    """Ticket 5.33 : couches de résultat que le budget laisse hors de la vue du modèle."""

    def setUp(self):
        self.v = av.AgentView()

    def test_nothing_left_out_shows_nothing(self):
        self.v.proposed("plan-1", PLAN)
        self.assertEqual(self.v.left_out, [])
        self.assertIsNone(av.left_out_text(self.v, "fr"))
        self.v.question("Laquelle ?", "model", [])
        self.assertIsNone(av.left_out_text(self.v, "en"))

    def test_repeated_names_are_counted(self):
        self.v.proposed("plan-1", PLAN, INTERSECTIONS)
        self.assertEqual(
            av.left_out_text(self.v, "fr"),
            "Le modèle ne voit plus 5 couches de résultat de cette session : "
            "Intersection ×5.",
        )
        self.assertEqual(
            av.left_out_text(self.v, "en"),
            "The model no longer sees 5 result layers from this session: Intersection ×5.",
        )

    def test_distinct_names_keep_their_order_and_one_is_singular(self):
        layers = [{"layer_id": "a", "name": "Tampon"}, {"layer_id": "b", "name": "Intersection"},
                  {"layer_id": "c", "name": "Tampon"}]
        self.v.question("Laquelle ?", "model", layers)
        self.assertEqual(
            av.left_out_text(self.v, "fr"),
            "Le modèle ne voit plus 3 couches de résultat de cette session : "
            "Tampon ×2, Intersection.",
        )
        self.v.question("Laquelle ?", "model", layers[:1])
        self.assertEqual(
            av.left_out_text(self.v, "fr"),
            "Le modèle ne voit plus 1 couche de résultat de cette session : Tampon.",
        )
        self.assertEqual(
            av.left_out_text(self.v, "en"),
            "The model no longer sees 1 result layer from this session: Tampon.",
        )

    def test_a_new_proposal_a_refusal_or_an_error_clears_the_mention(self):
        for clear in (lambda: self.v.proposed("plan-2", PLAN),
                      lambda: self.v.question("Laquelle ?"),
                      lambda: self.v.rejected(),
                      lambda: self.v.failed("agent.refused", "Non."),
                      lambda: self.v.generating()):
            self.v.proposed("plan-1", PLAN, INTERSECTIONS)
            self.assertIsNotNone(av.left_out_text(self.v, "fr"))
            clear()
            self.assertIsNone(av.left_out_text(self.v, "fr"))

    def test_malformed_left_out_never_raises(self):
        for given in (None, "Intersection", 3, {"name": "x"},
                      [None, 3, "x", {}, {"name": ""}, {"name": 4}, {"layer_id": "a"}]):
            self.v.proposed("plan-1", PLAN, given)
            self.assertEqual(self.v.left_out, [], given)
            self.assertIsNone(av.left_out_text(self.v, "fr"))
        self.v.proposed("plan-1", PLAN, [{"name": "Tampon"}, "x", {"layer_id": 3, "name": "Zone"}])
        self.assertEqual(self.v.left_out,
                         [{"layer_id": None, "name": "Tampon"}, {"layer_id": None, "name": "Zone"}])
        self.assertTrue(av.left_out_text(self.v, "fr").endswith(": Tampon, Zone."))
        # Une vue d'avant, sans l'attribut : rien, sans exception.
        self.assertIsNone(av.left_out_text(object(), "fr"))

    def test_both_languages_have_the_keys(self):
        for key in ("left_out_one", "left_out_many"):
            self.assertIn(key, texts.TEXTS["fr"])
            self.assertIn(key, texts.TEXTS["en"])


if __name__ == "__main__":
    unittest.main()
