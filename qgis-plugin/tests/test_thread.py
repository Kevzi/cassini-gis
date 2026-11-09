"""Tests sans Qt du fil de la demande [12.1] : une classe par ligne de la matrice du ticket."""

import os
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "cassini_qgis"))

import agentview as av  # noqa: E402
import runlog  # noqa: E402
import texts  # noqa: E402
import thread  # noqa: E402

CATALOGUE = [
    {"id": "analysis.buffer", "cancellable": True, "available": True},
    {"id": "terrain.slope", "cancellable": True, "available": True},
]
PLAN = {"kind": "calls", "calls": [
    {"command": "terrain.slope", "params": {}},
    {"command": "analysis.buffer", "params": {"layer_id": "gares", "distance": 500}},
    {"command": "terrain.slope", "params": {}},
    {"command": "terrain.slope", "params": {}},
]}
REQUEST = thread.request_exchange("Trouve les parcelles à moins de 500 m d'une gare", "19:49")


def ctl(view, has_text=True, busy=False):
    return av.controls(view, True, busy, has_text, CATALOGUE)


def proposed():
    v = av.AgentView()
    v.generating()
    v.proposed("plan-1", PLAN)
    return v


def sentence(step):
    return "fait %d" % step.number


def label(step):
    return "libellé %d" % step.number


class RestTest(unittest.TestCase):
    """Repos : carte d'accueil seule, replis fermés, Envoyer principal."""

    def test_welcome_alone(self):
        v = av.AgentView()
        self.assertEqual(thread.cards(v, None, None), [thread.WELCOME])

    def test_send_is_the_primary_button_once_text_is_typed(self):
        v = av.AgentView()
        self.assertEqual(thread.primary(ctl(v)), "send")
        self.assertIsNone(thread.primary(ctl(v, has_text=False)))  # grisé : sans couleur

    def test_empty_log_fold(self):
        self.assertEqual(thread.log_fold_label(0, "fr"), "Journal de la session (vide)")
        self.assertEqual(thread.log_fold_label(1, "fr"), "Journal de la session (1 étape)")
        self.assertEqual(thread.log_fold_label(4, "fr"), "Journal de la session (4 étapes)")
        self.assertEqual(thread.log_fold_label(4, "en"), "Session log (4 steps)")

    def test_input_hint_at_rest(self):
        v = av.AgentView()
        self.assertEqual(
            thread.input_hint(v, None, False, None, "fr"), texts.tr("fr", "input_hint")
        )

    def test_welcome_names_its_examples(self):
        self.assertEqual(texts.tr("fr", "examples_title"), "Exemples de demandes")


class ComposingTest(unittest.TestCase):
    """Composition : bulle + carte « Je prépare un plan… », saisie grisée avec sa raison."""

    def test_bubble_then_composing_card(self):
        v = av.AgentView()
        v.generating()
        self.assertEqual(thread.cards(v, REQUEST, None), [thread.BUBBLE, thread.COMPOSING])

    def test_input_says_why_it_is_greyed(self):
        v = av.AgentView()
        v.generating()
        c = ctl(v)
        self.assertFalse(c["input_enabled"])
        self.assertEqual(
            thread.input_hint(v, c["input_reason"], False, REQUEST, "fr"),
            "Patientez : un plan est en préparation.",
        )
        self.assertIsNone(thread.primary(c))

    def test_failed_composition_shows_its_card_under_the_bubble(self):
        v = av.AgentView()
        v.generating()
        v.failed("agent.model_missing", "Aucun modèle.")
        self.assertEqual(thread.cards(v, REQUEST, None), [thread.BUBBLE, thread.FAILED])

    def test_bubble_keeps_the_exact_sentence_and_time(self):
        self.assertEqual(thread.bubble_text(REQUEST, "fr"), REQUEST["text"])
        self.assertEqual(thread.bubble_meta(REQUEST, "fr"), "Vous · 19:49")
        self.assertEqual(
            thread.bubble_name(REQUEST, "fr"), "Votre demande, 19:49 : " + REQUEST["text"]
        )

    def test_bubble_takes_85_percent_then_all_when_narrow(self):
        self.assertEqual(thread.bubble_width(340), 289)
        self.assertEqual(thread.bubble_width(280), 280)


class ProposedTest(unittest.TestCase):
    """Plan proposé : bulle + carte du plan, étapes « En attente », Valider principal."""

    def test_plan_card_under_the_bubble(self):
        v = proposed()
        self.assertEqual(thread.cards(v, REQUEST, None), [thread.BUBBLE, thread.PLAN])

    def test_title_counts_the_steps_and_steps_are_pending(self):
        v = proposed()
        self.assertEqual(thread.plan_title(v, "fr"), "Voici ce que je propose — 4 étapes")
        rows = thread.plan_rows(v, CATALOGUE, "fr")
        self.assertEqual([r[1] for r in rows], ["En attente"] * 4)
        self.assertTrue(all(r[2].startswith("%d. " % (i + 1)) for i, r in enumerate(rows)))
        self.assertFalse(any(r[4] for r in rows))
        self.assertEqual({r[3] for r in rows}, {""})

    def test_approve_is_the_only_primary_button(self):
        v = proposed()
        c = ctl(v, has_text=False)
        self.assertEqual(thread.primary(c), "approve")
        self.assertFalse(c["send_enabled"])
        self.assertEqual(
            thread.input_hint(v, c["input_reason"], False, REQUEST, "fr"),
            "Validez ou refusez d'abord le plan ci-dessus.",
        )


class ExecutingTest(unittest.TestCase):
    """Exécution : étape 2 marquée (marqueur + « En cours »), Arrêter."""

    def setUp(self):
        self.v = proposed()
        self.v.executing()
        self.v.step(1)
        self.v.step(2)

    def test_plan_card_stays_and_no_result_while_running(self):
        last = {"kind": runlog.RUNNING, "plan": True, "command": "x"}
        self.assertEqual(thread.cards(self.v, REQUEST, last), [thread.BUBBLE, thread.PLAN])

    def test_step_two_carries_the_marker_and_the_word(self):
        rows = thread.plan_rows(self.v, CATALOGUE, "fr")
        self.assertEqual([r[4] for r in rows], [False, True, False, False])
        self.assertEqual(rows[0][1], "Terminée")
        self.assertEqual(rows[1][1], "En cours…")
        self.assertEqual(rows[2][1], "En attente")
        self.assertEqual(thread.plan_title(self.v, "fr"), "Plan validé — 4 étapes")

    def test_stop_offered_and_input_waits(self):
        c = ctl(self.v)
        self.assertTrue(c["stop_visible"])
        self.assertTrue(c["stop_enabled"])
        self.assertEqual(
            thread.input_hint(self.v, c["input_reason"], False, REQUEST, "fr"),
            "Patientez : une étape est en cours.",
        )

    def test_current_step_style_uses_the_pair(self):
        style = thread.styles(thread.colors(False))["step_current"]
        self.assertIn("#E4F2F0", style)
        self.assertIn("border-left: 3px solid #0E8F86", style)


class ResultTest(unittest.TestCase):
    """Résultat : plan validé replié, carte de résultat, Envoyer principal."""

    def setUp(self):
        self.v = proposed()
        self.v.executing()
        for k in (1, 2, 3, 4):
            self.v.step(k)
        payload = {"steps": [{"step": k, "result": {}} for k in (1, 2, 3, 4)]}
        self.last = av.outcome(runlog.DONE, payload, self.v)
        self.v.done(payload["steps"])

    def test_folded_plan_then_result(self):
        self.assertEqual(
            thread.cards(self.v, REQUEST, self.last),
            [thread.BUBBLE, thread.PLAN_FOLDED, thread.RESULT],
        )

    def test_result_title_rows_and_closing(self):
        self.assertEqual(thread.result_title(self.last, "fr"), "Plan exécuté. Étapes terminées : 4.")
        rows = thread.result_rows(self.v.steps, self.last, "fr", sentence, label)
        self.assertEqual([r[2] for r in rows], ["fait 1", "fait 2", "fait 3", "fait 4"])
        self.assertEqual({r[1] for r in rows}, {"Terminée"})
        self.assertEqual(thread.closing(self.last, "fr"), "Les couches sont ajoutées au projet.")

    def test_input_invites_a_new_request(self):
        c = ctl(self.v)
        self.assertEqual(thread.primary(c), "send")
        self.assertEqual(
            thread.input_hint(self.v, c["input_reason"], False, REQUEST, "fr"),
            "Nouvelle demande…",
        )

    def test_folded_plan_rows_keep_their_final_status(self):
        rows = thread.plan_rows(self.v, CATALOGUE, "fr")
        self.assertEqual({r[1] for r in rows}, {"Terminée"})


class StepErrorTest(unittest.TestCase):
    """Erreur d'étape : « Erreur » + message, suivantes « Non lancée », code à part."""

    def setUp(self):
        self.v = proposed()
        self.v.executing()
        self.v.step(1)
        self.v.step(2)
        payload = {"step": 2, "code": "analysis.crs_not_metric",
                   "message": "La couche « gares » est en EPSG:4326."}
        self.last = av.outcome(runlog.ERROR, payload, self.v)
        self.v.exec_failed(2)

    def test_cards(self):
        self.assertEqual(
            thread.cards(self.v, REQUEST, self.last),
            [thread.BUBBLE, thread.PLAN_FOLDED, thread.RESULT],
        )

    def test_rows_message_and_not_run(self):
        self.assertEqual(thread.result_title(self.last, "fr"), "Plan arrêté à l'étape 2.")
        rows = thread.result_rows(self.v.steps, self.last, "fr", sentence, label)
        self.assertEqual([r[1] for r in rows], ["Terminée", "Erreur", "Non lancée", "Non lancée"])
        self.assertEqual(rows[1][2], "libellé 2")
        self.assertEqual(rows[1][3], "La couche « gares » est en EPSG:4326.")
        self.assertEqual(rows[0][3], "")
        self.assertEqual(rows[2][0], thread.ICONS[thread.NOT_RUN])

    def test_code_only_in_technical_details(self):
        rows = thread.result_rows(self.v.steps, self.last, "fr", sentence, label)
        self.assertNotIn("analysis.crs_not_metric", repr(rows))
        self.assertNotIn("analysis.crs_not_metric", thread.result_title(self.last, "fr"))
        self.assertEqual(thread.error_details(self.last), "analysis.crs_not_metric")

    def test_cancelled_plan_title(self):
        last = {"kind": runlog.CANCELLED, "plan": True, "step": 3}
        self.assertEqual(
            thread.result_title(last, "fr"), "Plan arrêté à l'étape 3 à votre demande."
        )


class QuestionOrRefusalTest(unittest.TestCase):
    """Question / refus : carte sous la bulle."""

    def test_question_card(self):
        v = av.AgentView()
        v.generating()
        v.question("Quelle couche porte le PLU ?")
        self.assertEqual(thread.cards(v, REQUEST, None), [thread.BUBBLE, thread.QUESTION])
        self.assertEqual(thread.primary(ctl(v)), "send")

    def test_rejected_card(self):
        v = proposed()
        v.start_rejecting()
        v.rejected()
        self.assertEqual(thread.cards(v, REQUEST, None), [thread.BUBBLE, thread.REJECTED])


class NewRequestTest(unittest.TestCase):
    """Nouvelle demande : le fil ne montre que la nouvelle bulle."""

    def test_second_request_replaces_the_exchange(self):
        v = proposed()
        v.executing()
        v.step(1)
        v.done([{"step": k} for k in (1, 2, 3, 4)])
        second = thread.request_exchange("Pente du MNT", "20:01")
        v.generating()  # l'envoi : le panneau remet aussi `last` à None
        self.assertEqual(thread.cards(v, second, None), [thread.BUBBLE, thread.COMPOSING])
        self.assertEqual(thread.bubble_text(second, "fr"), "Pente du MNT")

    def test_expert_bubble(self):
        exchange = thread.expert_exchange("Pente de mnt en pourcentage", "20:02")
        v = av.AgentView()
        running = {"kind": runlog.RUNNING, "command": "terrain.slope"}
        self.assertEqual(thread.cards(v, exchange, running), [thread.BUBBLE, thread.RESULT])
        self.assertEqual(
            thread.bubble_text(exchange, "fr"), "Mode expert : Pente de mnt en pourcentage"
        )
        done = {"kind": runlog.DONE, "command": "terrain.slope"}
        self.assertEqual(thread.result_title(done, "fr"), "✔ Terminée")

    def test_log_detail_without_exchange_shows_the_result_card(self):
        v = av.AgentView()
        self.assertEqual(thread.cards(v, None, None, focus=("entry", object())), [thread.RESULT])


class DarkThemeTest(unittest.TestCase):
    """Thème sombre : couleurs de Cassini en version sombre, même règle que le logo."""

    def test_pair_follows_window_lightness(self):
        self.assertTrue(thread.is_dark(43))
        self.assertFalse(thread.is_dark(240))
        self.assertEqual(thread.colors(True)["accent_strong"], "#3FD0C4")
        self.assertEqual(thread.colors(True)["on_accent_strong"], "#0F1F1E")
        self.assertEqual(thread.colors(False)["accent_strong"], "#0B7A72")

    def test_styles_use_the_dark_pair_and_inherit_qgis_surfaces(self):
        s = thread.styles(thread.colors(True))
        self.assertIn("#3FD0C4", s["primary"])
        self.assertIn("#0F1F1E", s["primary"])
        self.assertIn("#1E3B39", s["bubble"])
        self.assertIn("#2F6E68", s["bubble"])
        self.assertIn("palette(base)", s["card"])
        self.assertIn("palette(mid)", s["card"])
        self.assertNotIn("shadow", s["card"])
        self.assertEqual(s["chevron"], "#3FD0C4")

    def test_no_fixed_surface_colour_in_cards(self):
        for dark in (False, True):
            import re

            card = thread.styles(thread.colors(dark))["card"]
            self.assertIsNone(re.search(r"#[0-9A-Fa-f]{6}", card))


class FoldedPlanTest(unittest.TestCase):
    """La carte repliée dit l'issue du plan : exécuté, en erreur, arrêté."""

    def finished(self, how):
        v = proposed()
        v.executing()
        v.step(1)
        v.step(2)
        if how == "done":
            v.done([{"step": k} for k in (1, 2, 3, 4)])
            return v, {"kind": runlog.DONE, "plan": True, "n": 4}
        if how == "error":
            v.exec_failed(2)
            return v, {"kind": runlog.ERROR, "plan": True, "step": 2}
        if how == "cancelled":
            v.cancelled(2)
            return v, {"kind": runlog.CANCELLED, "plan": True, "step": 2}
        v.forget()  # noyau arrêté : plus de verdict
        return v, None

    def test_each_outcome_has_its_icon_and_word(self):
        cases = {"done": ("✔", "exécuté"), "error": ("✖", "en erreur"),
                 "cancelled": ("⊘", "arrêté"), "crash": ("✖", "en erreur")}
        for how, (icon, word) in cases.items():
            v, last = self.finished(how)
            text = thread.folded_summary(v, last, "fr")
            self.assertTrue(text.startswith(icon), (how, text))
            self.assertIn("Plan validé — 4 étapes", text)
            self.assertTrue(text.endswith(word), (how, text))

    def test_accessible_name_singular_and_plural(self):
        self.assertEqual(thread.folded_name(1, False, "fr"), "Plan validé, 1 étape, réduit")
        self.assertEqual(thread.folded_name(3, True, "fr"), "Plan validé, 3 étapes, développé")


class OpenFolderTest(unittest.TestCase):
    def test_target_of_an_expert_command_and_of_a_plan(self):
        import tempfile
        import types

        with tempfile.TemporaryDirectory() as folder:
            path = os.path.join(folder, "2-terrain.slope.tif")
            with open(path, "wb") as f:
                f.write(b"x")
            first = {"file": path}
            steps = [types.SimpleNamespace(result=first), types.SimpleNamespace(result={"n": 1})]
            plan = {"kind": runlog.DONE, "plan": True, "steps": steps}
            self.assertIs(thread.shown_result(plan), first)
            expert = {"kind": runlog.DONE, "result": first}
            self.assertIs(thread.shown_result(expert), first)
            self.assertIsNone(thread.shown_result({"kind": runlog.ERROR, "plan": True}))
            self.assertIsNone(thread.shown_result(None))
            no_file = {"kind": runlog.DONE, "plan": True, "steps": steps[1:]}
            self.assertIsNone(thread.shown_result(no_file))


class InputReasonTest(unittest.TestCase):
    def test_reason_line_only_when_text_hides_the_hint(self):
        self.assertEqual(thread.reason_line("Patientez.", True), "Patientez.")
        self.assertEqual(thread.reason_line("Patientez.", False), "")
        self.assertEqual(thread.reason_line("", True), "")


class PrimaryStatesTest(unittest.TestCase):
    def test_focus_hover_and_pressed_states(self):
        for dark in (False, True):
            c = thread.colors(dark)
            style = thread.styles(c)["primary"]
            self.assertIn("QPushButton:focus { border: 2px solid palette(window-text)", style)
            self.assertIn("QPushButton:hover { background: %s" % c["primary_hover"], style)
            self.assertIn("QPushButton:pressed { background: %s" % c["primary_pressed"], style)

    def test_focused_step_row_is_outlined(self):
        for key in ("step", "step_current"):
            self.assertRegex(thread.styles(thread.colors(False))[key],
                             r'QFrame#cassiniStep(\[current="true"\])?:focus')


class SingularTest(unittest.TestCase):
    def test_one_step_plan_reads_in_the_singular(self):
        v = av.AgentView()
        v.proposed("p", {"calls": [{"command": "terrain.slope", "params": {}}]})
        self.assertEqual(thread.plan_title(v, "fr"), "Voici ce que je propose — 1 étape")
        self.assertEqual(thread.approved_title(1, "fr"), "Plan validé — 1 étape")
        self.assertEqual(thread.approved_title(3, "en"), "Plan approved — 3 steps")


class FoldHeightTest(unittest.TestCase):
    def test_content_up_to_half_the_panel(self):
        self.assertEqual(thread.fold_heights(120, None, 900, 200), (120, 0))
        self.assertEqual(thread.fold_heights(800, None, 900, 200), (450, 0))
        self.assertEqual(thread.fold_heights(None, None, 900, 200), (0, 0))

    def test_both_folds_share_the_budget_and_the_thread_keeps_its_minimum(self):
        log, expert = thread.fold_heights(800, 800, 900, 200)
        self.assertEqual((log, expert), (225, 225))
        log, expert = thread.fold_heights(100, 800, 900, 200)
        self.assertEqual((log, expert), (100, 350))  # le reste va à l'autre repli
        # Panneau bas : le fil garde MIN_THREAD pixels.
        log, expert = thread.fold_heights(800, 800, 500, 300)
        self.assertGreaterEqual(500 - 300 - (log + expert), thread.MIN_THREAD)
        self.assertEqual(thread.fold_heights(800, 800, 300, 300), (0, 0))


class TextsTest(unittest.TestCase):
    def test_new_labels_in_both_languages(self):
        for key in ("welcome_title", "plan_approved_n", "plan_show", "log_fold_many",
                    "not_run", "input_hint_next", "composing_title"):
            for lang in ("fr", "en"):
                self.assertTrue(texts.tr(lang, key, n=2, k=1, time="", text="", label=""))


# -- guidage [12.2] : une classe par ligne de la matrice du ticket 12.2 ---------------
SENTENCE = "Trouve les parcelles à moins de 500 m d'une gare"


def guide(view, exchange=REQUEST, last=None, focus=None, rephrased=False):
    return thread.guidance(view, exchange, last, focus, rephrased)


def target(view, exchange=REQUEST, last=None, focus=None, rephrased=False):
    return thread.focus_target(view, exchange, last, focus,
                               guide(view, exchange, last, focus, rephrased))


def step_error():
    v = proposed()
    v.executing()
    v.step(1)
    v.step(2)
    payload = {"step": 2, "code": "analysis.crs_not_metric",
               "message": "La couche « gares » est en EPSG:4326."}
    last = av.outcome(runlog.ERROR, payload, v)
    v.exec_failed(2)
    return v, last


class GuidePlanProposedTest(unittest.TestCase):
    """Plan proposé : « À vous : validez… » dans la carte du plan, focus sur Valider."""

    def test_cue_in_the_plan_card(self):
        g = guide(proposed())
        self.assertEqual(g["card"], thread.PLAN)
        self.assertFalse(g["rephrase"])
        self.assertEqual(thread.cue_text(g, "fr"),
                         "À vous : validez pour lancer, ou refusez pour reformuler.")
        self.assertEqual(thread.cue_text(g, "en"),
                         "Your turn: approve to run, or reject to rephrase.")

    def test_no_cue_while_a_refusal_is_pending(self):
        v = proposed()
        v.start_rejecting()
        self.assertIsNone(thread.guidance(v, REQUEST, None, controls=ctl(v))["card"])

    def test_no_cue_when_neither_action_is_possible(self):
        v = proposed()
        busy = av.controls(v, True, False, True, CATALOGUE, external_busy=True)
        self.assertFalse(busy["approve_enabled"] or busy["reject_enabled"])
        self.assertIsNone(thread.guidance(v, REQUEST, None, controls=busy)["card"])
        self.assertEqual(thread.guidance(v, REQUEST, None, controls=ctl(v))["card"], thread.PLAN)

    def test_focus_on_approve(self):
        self.assertEqual(target(proposed()), thread.FOCUS_APPROVE)
        self.assertEqual(thread.primary(ctl(proposed()), False), "approve")


class GuideExecutingTest(unittest.TestCase):
    """Exécution : « Plan validé — 4 étapes », « Étape 2 sur 4 en cours », focus au titre."""

    def setUp(self):
        self.v = proposed()
        self.v.executing()
        self.v.step(1)
        self.v.step(2)
        self.last = {"kind": runlog.RUNNING, "plan": True, "command": "x"}

    def test_title_and_progress(self):
        self.assertEqual(thread.plan_title(self.v, "fr"), "Plan validé — 4 étapes")
        self.assertEqual(thread.progress_text(self.v, "fr"), "Étape 2 sur 4 en cours")
        self.assertEqual(thread.progress_text(self.v, "en"), "Step 2 of 4 running")
        rich = thread.progress_text(self.v, "fr", emphasis="#1f1f1f")
        self.assertIn("<b style='color:#1f1f1f'>2</b>", rich)
        self.assertIn("<b style='color:#1f1f1f'>4</b>", rich)
        self.assertEqual(thread.plan_title_name(self.v, "fr"),
                         "Plan validé — 4 étapes, étape 2 sur 4 en cours")

    def test_no_cue_while_running_and_focus_on_the_plan_title(self):
        self.assertIsNone(guide(self.v, last=self.last)["card"])
        self.assertEqual(target(self.v, last=self.last), thread.FOCUS_PLAN_TITLE)

    def test_progress_only_while_executing(self):
        self.assertEqual(thread.progress_text(proposed(), "fr"), "")
        v = proposed()
        v.executing()  # validé, aucune étape encore annoncée
        self.assertEqual(thread.progress_text(v, "fr"), "")
        self.assertEqual(thread.plan_title_name(proposed(), "fr"),
                         "Voici ce que je propose — 4 étapes")

    def test_no_cue_while_composing(self):
        v = av.AgentView()
        v.generating()
        self.assertIsNone(guide(v)["card"])
        self.assertEqual(target(v), thread.FOCUS_COMPOSING)


class GuideStepErrorTest(unittest.TestCase):
    """Erreur d'étape : « À vous : reformulez… », Reformuler principal, focus dessus."""

    def test_cue_and_rephrase_in_the_result_card(self):
        v, last = step_error()
        g = guide(v, last=last)
        self.assertEqual(g["card"], thread.RESULT)
        self.assertTrue(g["rephrase"])
        self.assertEqual(thread.cue_text(g, "fr"), "À vous : reformulez votre demande.")
        self.assertEqual(target(v, last=last), thread.FOCUS_REPHRASE)

    def test_rephrase_is_the_only_primary_and_send_is_native(self):
        v, last = step_error()
        c = ctl(v, has_text=True)
        self.assertEqual(thread.primary(c, guide(v, last=last)["rephrase"]), "rephrase")

    def test_suggestion_from_the_core_when_given(self):
        v, last = step_error()
        last["suggestion"] = "reprojette d'abord les gares en Lambert-93"
        self.assertEqual(
            thread.cue_text(guide(v, last=last), "fr"),
            "À vous : reformulez, par exemple « reprojette d'abord les gares en Lambert-93 ».",
        )

    def test_announcement_carries_the_message(self):
        _v, last = step_error()
        self.assertEqual(
            thread.outcome_announcement(last, "fr"),
            "Plan arrêté à l'étape 2. La couche « gares » est en EPSG:4326.",
        )

    def test_after_rephrase_the_button_goes_and_send_comes_back(self):
        v, last = step_error()
        g = guide(v, last=last, rephrased=True)
        self.assertEqual(g["card"], thread.RESULT)  # la consigne reste
        self.assertFalse(g["rephrase"])
        self.assertEqual(thread.primary(ctl(v, has_text=True), g["rephrase"]), "send")
        # Le focus reste dans la saisie, où la phrase est revenue.
        self.assertEqual(target(v, last=last, rephrased=True), thread.FOCUS_INPUT)

    def test_a_clicked_detail_keeps_the_cue_and_rephrase(self):
        v, last = step_error()
        g = guide(v, last=last, focus=("step", v.steps[0]))
        self.assertEqual(g["card"], thread.RESULT)
        self.assertTrue(g["rephrase"])
        self.assertEqual(thread.primary(ctl(v), g["rephrase"]), "rephrase")
        self.assertIsNone(target(v, last=last, focus=("step", v.steps[0])))  # focus laissé


class GuideRejectedTest(unittest.TestCase):
    """Refus : carte de refus, « À vous », Reformuler."""

    def test_cue_and_rephrase(self):
        v = proposed()
        v.start_rejecting()
        v.rejected()
        g = guide(v)
        self.assertEqual(g["card"], thread.REJECTED)
        self.assertTrue(g["rephrase"])
        self.assertEqual(thread.cue_text(g, "fr"), "À vous : reformulez votre demande.")
        self.assertEqual(target(v), thread.FOCUS_REPHRASE)


class GuideComposeFailedTest(unittest.TestCase):
    """Composition impossible : carte d'échec, « À vous », Reformuler."""

    def setUp(self):
        self.v = av.AgentView()
        self.v.generating()
        self.v.failed("agent.no_plan", "Aucune commande ne répond à la demande.")

    def test_cue_and_rephrase(self):
        g = guide(self.v)
        self.assertEqual(g["card"], thread.FAILED)
        self.assertTrue(g["rephrase"])
        self.assertEqual(target(self.v), thread.FOCUS_REPHRASE)

    def test_announcement_carries_the_message(self):
        self.assertEqual(
            thread.failed_announcement(self.v, "fr"),
            "Je n'ai pas pu préparer de plan. Aucune commande ne répond à la demande.",
        )


class GuideQuestionTest(unittest.TestCase):
    """Question : « À vous : répondez dans la saisie. », focus dans la saisie."""

    def test_cue_without_button_and_focus_in_the_input(self):
        v = av.AgentView()
        v.generating()
        v.question("Quelle couche porte le PLU ?", source="guard")
        g = guide(v)
        self.assertEqual(g["card"], thread.QUESTION)
        self.assertFalse(g["rephrase"])  # Envoyer reste le bouton principal
        self.assertEqual(thread.cue_text(g, "fr"), "À vous : répondez dans la saisie.")
        self.assertEqual(target(v), thread.FOCUS_INPUT)
        self.assertEqual(thread.primary(ctl(v), g["rephrase"]), "send")
        # La phrase revient seulement dans une saisie vide (règle de 5.x, inchangée).
        self.assertEqual(av.request_to_restore(v, SENTENCE, ""), SENTENCE)
        self.assertIsNone(av.request_to_restore(v, SENTENCE, "déjà tapé"))


class RephraseEmptyInputTest(unittest.TestCase):
    """Reformuler, saisie vide : la phrase revient."""

    def test_sentence_comes_back(self):
        self.assertEqual(thread.rephrase(REQUEST, None, ""),
                         {"text": SENTENCE, "note": None, "done": True})
        self.assertEqual(thread.rephrase(REQUEST, None, "  \n")["text"], SENTENCE)

    def test_the_bubble_sentence_wins_then_the_last_request(self):
        self.assertEqual(thread.rephrase(REQUEST, "ancienne", "")["text"], SENTENCE)
        self.assertEqual(thread.rephrase(None, "ancienne", "")["text"], "ancienne")
        expert = thread.expert_exchange("Pente de mnt", "10:00")
        self.assertEqual(thread.rephrase(expert, "ancienne", "")["text"], "ancienne")


class RephraseFilledInputTest(unittest.TestCase):
    """Reformuler, saisie remplie : rien n'est écrasé, le panneau le dit."""

    def test_nothing_is_overwritten(self):
        out = thread.rephrase(REQUEST, None, "autre chose")
        self.assertIsNone(out["text"])
        self.assertFalse(out["done"])
        self.assertEqual(out["note"], "rephrase_blocked")
        self.assertTrue(texts.tr("fr", out["note"]).startswith("La saisie contient déjà"))

    def test_sentence_already_there_is_not_a_refusal(self):
        # Après un échec de composition, la phrase est déjà revenue dans la saisie.
        self.assertEqual(thread.rephrase(REQUEST, None, SENTENCE + " "),
                         {"text": None, "note": None, "done": True})

    def test_no_sentence(self):
        self.assertEqual(thread.rephrase(None, None, ""),
                         {"text": None, "note": None, "done": False})


class GuideExpertErrorTest(unittest.TestCase):
    """Commande experte en erreur : carte d'erreur sans Reformuler ni « À vous »."""

    def test_no_cue_no_rephrase(self):
        v = av.AgentView()
        expert = thread.expert_exchange("Pente de mnt", "10:00")
        last = {"kind": runlog.ERROR, "command": "terrain.slope", "code": "x.y", "message": "m"}
        g = guide(v, exchange=expert, last=last)
        self.assertIsNone(g["card"])
        self.assertFalse(g["rephrase"])
        self.assertEqual(target(v, exchange=expert, last=last), thread.FOCUS_RESULT_TITLE)
        self.assertEqual(thread.outcome_announcement(last, "fr"), "Commande en erreur.")

    def test_running_expert_command_keeps_its_focus(self):
        v = av.AgentView()
        expert = thread.expert_exchange("Pente de mnt", "10:00")
        last = {"kind": runlog.RUNNING, "command": "terrain.slope"}
        self.assertIsNone(target(v, exchange=expert, last=last))


class GuideOnlyOneCueTest(unittest.TestCase):
    """La ligne « À vous » n'est jamais dans deux cartes : une seule carte par état."""

    def states(self):
        """(nom, vue, échange, dernier résultat) pour chaque état du fil."""
        rest = av.AgentView()
        composing = av.AgentView()
        composing.generating()
        question = av.AgentView()
        question.generating()
        question.question("Quelle couche ?")
        failed = av.AgentView()
        failed.generating()
        failed.failed("agent.no_plan", "m")
        rejected = proposed()
        rejected.start_rejecting()
        rejecting = proposed()
        rejecting.start_rejecting()
        rejected.rejected()
        running = proposed()
        running.executing()
        running.step(1)
        err, err_last = step_error()
        done = proposed()
        done.executing()
        done.step(1)
        payload = {"steps": [{"step": 1, "result": {}}]}
        done_last = av.outcome(runlog.DONE, payload, done)
        done.done(payload["steps"])
        cancelled = proposed()
        cancelled.executing()
        cancelled.step(1)
        cancelled_last = av.outcome(runlog.CANCELLED, {"step": 1}, cancelled)
        cancelled.cancelled(1)
        expert = thread.expert_exchange("Pente", "10:00")
        return [
            ("repos", rest, None, None),
            ("composition", composing, REQUEST, None),
            ("question", question, REQUEST, None),
            ("échec", failed, REQUEST, None),
            ("plan proposé", proposed(), REQUEST, None),
            ("refus en cours", rejecting, REQUEST, None),
            ("refusé", rejected, REQUEST, None),
            ("exécution", running, REQUEST, {"kind": runlog.RUNNING, "plan": True}),
            ("erreur d'étape", err, REQUEST, err_last),
            ("résultat", done, REQUEST, done_last),
            ("arrêté", cancelled, REQUEST, cancelled_last),
            ("expert en cours", rest, expert, {"kind": runlog.RUNNING, "command": "x"}),
            ("expert en erreur", rest, expert, {"kind": runlog.ERROR, "message": "m"}),
        ]

    def test_zero_or_one_cue_and_in_a_shown_card(self):
        expected = {"question": thread.QUESTION, "échec": thread.FAILED,
                    "plan proposé": thread.PLAN, "refusé": thread.REJECTED,
                    "erreur d'étape": thread.RESULT}
        for name, v, exchange, last in self.states():
            with self.subTest(name):
                g = thread.guidance(v, exchange, last, controls=ctl(v))
                self.assertEqual(g["card"], expected.get(name))
                self.assertEqual(bool(g["cue"]), g["card"] is not None)
                if g["card"] is not None:  # la carte qui porte la ligne est affichée
                    self.assertIn(g["card"], thread.cards(v, exchange, last))
                self.assertFalse(g["rephrase"] and g["card"] is None)

    def test_rest_and_done_have_no_cue(self):
        self.assertIsNone(guide(av.AgentView(), exchange=None)["card"])
        self.assertEqual(target(av.AgentView(), exchange=None), thread.FOCUS_INPUT)


class FocusMoveTest(unittest.TestCase):
    """Le focus ne bouge que dans le panneau, jamais pendant la frappe."""

    def test_rules(self):
        self.assertTrue(thread.may_move_focus(True, False, False))
        self.assertTrue(thread.may_move_focus(True, True, False))
        self.assertFalse(thread.may_move_focus(True, True, True))  # l'utilisateur tape
        self.assertFalse(thread.may_move_focus(False, False, False))  # ailleurs dans QGIS


class GuideStylesTest(unittest.TestCase):
    def test_cue_and_title_use_the_pair(self):
        for dark in (False, True):
            c = thread.colors(dark)
            st = thread.styles(c)
            self.assertIn(c["accent"], st["cue"])
            self.assertIn("alternate-base", st["cue"])
            self.assertIn(c["accent"], st["title"])


# -- rejeu du journal [12.3] : une classe par ligne de la matrice -------------------
import client  # noqa: E402


def button(view=None, ready=True, has_journal=True, busy=False, external=False, replay=None):
    return thread.replay_button(view or av.AgentView(), ready, has_journal, busy, external, replay)


class ReplayIdenticalTest(unittest.TestCase):
    def test_identical(self):
        state = thread.replay_outcome(client.REPLAY_DONE, {"steps": 2, "identical": True})
        self.assertEqual(thread.replay_text(state, "fr"), "Rejeu terminé : résultat identique.")
        self.assertEqual(thread.replay_text(state, "en"), "Replay done: identical result.")
        self.assertEqual(thread.replay_line(state, "fr"), "✔ Rejeu terminé : résultat identique.")
        self.assertEqual(thread.replay_details(state), "")
        self.assertTrue(button(replay=state)["enabled"])  # on peut rejouer encore

    def test_done_without_identical_is_not_called_identical(self):
        state = thread.replay_outcome(client.REPLAY_DONE, {"steps": 2})
        self.assertEqual(state["status"], thread.REPLAY_DIFFERENT)
        self.assertEqual(thread.replay_text(state, "fr"), "Rejeu terminé : résultat différent.")


class ReplayDifferentTest(unittest.TestCase):
    def test_diverged_names_the_step_and_the_reason(self):
        message = "Le rejeu diverge du journal à l'étape 2 : le fichier d'entrée a changé."
        state = thread.replay_outcome(
            client.REPLAY_FAILED, {"code": "replay.diverged", "message": message, "step": 2})
        # Le message nomme déjà l'étape : elle n'est pas répétée.
        self.assertEqual(
            thread.replay_text(state, "fr"), "Rejeu terminé : résultat différent. " + message)
        self.assertEqual(
            thread.replay_text(state, "en"), "Replay done: different result. " + message)
        other = dict(state, message="Le fichier d'entrée a changé.")
        self.assertEqual(
            thread.replay_text(other, "fr"),
            "Rejeu terminé : résultat différent à l'étape 2. Le fichier d'entrée a changé.")
        self.assertEqual(
            thread.replay_text(dict(other, message="Step 2 differs."), "en"),
            "Replay done: different result. Step 2 differs.")
        # « étape 12 » ne vaut pas « étape 1 ».
        self.assertTrue(thread.replay_text(dict(state, step=1, message="à l'étape 12"), "fr")
                        .startswith("Rejeu terminé : résultat différent à l'étape 1."))
        self.assertTrue(thread.replay_line(state, "fr").startswith("✖ "))
        self.assertEqual(thread.replay_details(state), "replay.diverged")
        self.assertNotIn("replay.diverged", thread.replay_text(state, "fr"))


class ReplayImpossibleTest(unittest.TestCase):
    def test_other_error_shows_the_core_message_and_hides_the_code(self):
        state = thread.replay_outcome(client.REPLAY_FAILED, {
            "code": "registry.unavailable", "step": 3,
            "message": "La commande layer.add n'est pas annoncée par l'extension."})
        text = thread.replay_text(state, "fr")
        self.assertEqual(
            text, "Rejeu arrêté à l'étape 3 : La commande layer.add n'est pas annoncée par l'extension.")
        self.assertNotIn("registry.unavailable", text)
        self.assertEqual(thread.replay_details(state), "registry.unavailable")

    def test_without_a_step(self):
        state = thread.replay_outcome(client.REPLAY_FAILED, {
            "code": "replay.no_journal", "step": None,
            "message": "Ce projet n'a pas de journal à rejouer."})
        self.assertEqual(thread.replay_text(state, "fr"),
                         "Rejeu impossible : Ce projet n'a pas de journal à rejouer.")
        self.assertEqual(thread.replay_text(state, "en"),
                         "Replay not possible: Ce projet n'a pas de journal à rejouer.")

    def test_core_death(self):
        state = thread.replay_outcome(client.REPLAY_FAILED, {
            "code": client.CORE_DIED, "step": None, "message": "Le noyau Cassini s'est arrêté."})
        self.assertEqual(state["status"], thread.REPLAY_STOPPED)
        self.assertFalse(thread.replay_running(state))

    def test_garbage_payload_never_raises(self):
        state = thread.replay_outcome(client.REPLAY_FAILED, None)
        self.assertEqual(state["status"], thread.REPLAY_STOPPED)
        self.assertEqual(thread.replay_text(state, "fr"), "Le rejeu n'a pas pu se faire.")
        self.assertEqual(thread.replay_text(state, "en"), "The replay could not be done.")
        state = thread.replay_outcome(client.REPLAY_FAILED, {"step": 3, "message": " "})
        self.assertEqual(thread.replay_text(state, "fr"), "Rejeu arrêté à l'étape 3.")
        state = thread.replay_outcome(client.REPLAY_FAILED, {"step": True, "code": 3})
        self.assertIsNone(state["step"])
        self.assertEqual(state["code"], "")


class ReplayRunningTest(unittest.TestCase):
    def test_running_greys_the_button_the_input_and_send(self):
        state = thread.replay_started()
        self.assertTrue(thread.replay_running(state))
        self.assertEqual(thread.replay_text(state, "fr"), "Rejeu en cours…")
        self.assertEqual(thread.replay_line(state, "fr"), "… Rejeu en cours…")
        self.assertNotIn("sur", thread.replay_text(state, "fr"))  # pas d'« étape k sur n »
        self.assertEqual(button(replay=state),
                         {"enabled": False, "reason": "replay_reason_running"})
        # Le panneau passe le rejeu comme une occupation : saisie et Envoyer grisés.
        c = av.controls(av.AgentView(), True, True, True, CATALOGUE)
        self.assertFalse(c["input_enabled"])
        self.assertFalse(c["send_enabled"])
        hint = thread.input_hint(av.AgentView(), c["input_reason"], True, None, "fr")
        self.assertEqual(hint, "Patientez : une étape est en cours.")
        self.assertEqual(texts.tr("fr", "replay_reason_running"),
                         "Patientez : le rejeu est en cours.")


class ReplayEmptyJournalTest(unittest.TestCase):
    def test_empty_journal_greys_with_its_reason(self):
        self.assertEqual(button(has_journal=False),
                         {"enabled": False, "reason": "replay_reason_empty"})
        self.assertEqual(texts.tr("fr", "replay_reason_empty"),
                         "Le journal est vide : rien à rejouer.")


class ReplayBusyTest(unittest.TestCase):
    def test_greyed_during_a_command_a_plan_or_an_external_call(self):
        self.assertEqual(button(busy=True)["reason"], "input_reason_running")
        self.assertEqual(button(external=True)["reason"], "input_reason_external")
        v = av.AgentView()
        v.generating()
        self.assertEqual(button(v)["reason"], "input_reason_composing")
        v = proposed()
        self.assertEqual(button(v)["reason"], "input_reason_plan")
        v.executing()
        self.assertEqual(button(v)["reason"], "input_reason_running")

    def test_not_ready_greys_without_reason(self):
        # Hors état prêt (lecture seule comprise aujourd'hui), le bandeau dit pourquoi.
        self.assertEqual(button(ready=False), {"enabled": False, "reason": None})

    def test_at_rest_enabled(self):
        self.assertEqual(button(), {"enabled": True, "reason": None})


class ReplayGuidanceTest(unittest.TestCase):
    def test_no_cue_and_greyed_rephrase_during_a_replay(self):
        g = {"card": thread.RESULT, "cue": "act_rephrase", "values": {}, "rephrase": True}
        shown = thread.shown_guidance(g, True)
        self.assertIsNone(shown["cue"])
        self.assertEqual(thread.cue_text(shown, "fr"), "")
        self.assertFalse(shown["rephrase_enabled"])
        self.assertIsNone(thread.primary({"send_enabled": False}, shown["rephrase_enabled"]))
        self.assertEqual(g["cue"], "act_rephrase")  # le guidage brut (focus) est intact
        shown = thread.shown_guidance(g, False)
        self.assertEqual(shown["cue"], "act_rephrase")
        self.assertTrue(shown["rephrase_enabled"])


class ReplayRefusedAndStaleTest(unittest.TestCase):
    def test_refused_text(self):
        state = thread.replay_refused()
        self.assertFalse(thread.replay_running(state))
        self.assertEqual(thread.replay_line(state, "fr"),
                         "ⓘ Le noyau est occupé : réessayez dans un instant.")
        self.assertTrue(button(replay=state)["enabled"])

    def test_current_only_for_the_same_log(self):
        done = dict(thread.replay_outcome(client.REPLAY_DONE, {"identical": True}), entries=2)
        self.assertIs(thread.replay_current(done, 2), done)
        self.assertIsNone(thread.replay_current(done, 3))
        running = thread.replay_started()
        self.assertIs(thread.replay_current(running, 5), running)
        self.assertIsNone(thread.replay_current(None, 1))


class ExpertCommandInThreadTest(unittest.TestCase):
    """Commande experte `system.about` : bulle « Mode expert : … » puis carte de résultat."""

    def test_bubble_then_result_card(self):
        expert = thread.expert_exchange("Vérification du noyau Cassini", "10:00")
        v = av.AgentView()
        running = {"kind": runlog.RUNNING, "command": "system.about"}
        self.assertEqual(thread.cards(v, expert, running), [thread.BUBBLE, thread.RESULT])
        self.assertEqual(thread.bubble_text(expert, "fr"),
                         "Mode expert : Vérification du noyau Cassini")
        done = {"kind": runlog.DONE, "command": "system.about", "result": {}}
        self.assertEqual(thread.cards(v, expert, done), [thread.BUBBLE, thread.RESULT])
        self.assertEqual(thread.result_title(done, "fr"), "✔ Terminée")

    def test_error_card_without_rephrase(self):
        expert = thread.expert_exchange("Vérification du noyau Cassini", "10:00")
        failed = {"kind": runlog.ERROR, "command": "system.about", "code": "x.y", "message": "m"}
        v = av.AgentView()
        self.assertEqual(thread.cards(v, expert, failed), [thread.BUBBLE, thread.RESULT])
        g = thread.guidance(v, expert, failed)
        self.assertFalse(g["rephrase"])
        self.assertIsNone(g["cue"])
        self.assertEqual(thread.error_details(failed), "x.y")


if __name__ == "__main__":
    unittest.main()
