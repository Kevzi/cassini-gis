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
            self.assertIn("QFrame#cassiniStep:focus", thread.styles(thread.colors(False))[key])


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


if __name__ == "__main__":
    unittest.main()
