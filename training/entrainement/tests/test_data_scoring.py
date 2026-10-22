"""Tests de la lecture du jeu, de la notation et des gardes de la chaîne (ticket 5.10, tâche 2),
sans torch.

    python3 -I -m unittest discover -s training/entrainement/tests
"""

import json
import re
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))

import data  # noqa: E402
import pipeline  # noqa: E402
import scoring  # noqa: E402
from data import Example  # noqa: E402

PDF_PLAN = (
    '{"kind":"calls","calls":[{"command":"analysis.buffer","params":{"distance":'
    '{"unit":"m","value":500},"layer_id":"gares_1"}},{"command":"layout.export_pdf",'
    '"params":{"title":"Gares"}}]}'
)


def line(prompt="<|im_start|>system\nX<|im_end|>\n", completion='{"kind":"question","question":"Laquelle ?"}<|im_end|>'):
    return json.dumps({"prompt": prompt, "completion": completion}, ensure_ascii=False)


class DatasetReading(unittest.TestCase):
    def test_a_well_formed_file_is_read_as_is(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "jeu.jsonl"
            path.write_text(line() + "\n" + line(prompt="autre") + "\n", encoding="utf-8")
            examples = data.read_jsonl(path)
        self.assertEqual(len(examples), 2)
        self.assertEqual(examples[0].prompt, "<|im_start|>system\nX<|im_end|>\n")
        self.assertEqual(
            data.expected_answer(examples[0]), '{"kind":"question","question":"Laquelle ?"}'
        )

    def test_a_malformed_line_is_refused(self):
        for bad in [
            "pas du json",
            json.dumps({"prompt": "p"}),
            json.dumps({"prompt": "p", "completion": "c<|im_end|>", "famille": "x"}),
            json.dumps({"prompt": "p", "completion": "sans fin"}),
            json.dumps({"prompt": "", "completion": "c<|im_end|>"}),
            json.dumps({"prompt": "p", "completion": "<|im_end|>"}),
            json.dumps(["p", "c"]),
        ]:
            with self.assertRaises(data.UnreadableDataset, msg=bad):
                data.read_line(bad)

    def test_a_too_long_example_is_dropped_and_counted_never_truncated(self):
        examples = [Example("a" * n, "x<|im_end|>") for n in (10, 50, 20, 51)]
        kept, dropped = data.drop_too_long(
            examples, lambda e: len(e.prompt) + len(e.completion), 61
        )
        self.assertEqual(dropped, 1)
        self.assertEqual([len(e.prompt) for e in kept], [10, 50, 20])
        self.assertTrue(all(e.completion == "x<|im_end|>" for e in kept))

    def test_validation_keeps_no_training_prompt(self):
        training = [Example("a", "x<|im_end|>"), Example("b", "y<|im_end|>")]
        validation = [Example("b", "y<|im_end|>"), Example("c", "z<|im_end|>")]
        kept, removed = data.drop_overlap(validation, training)
        self.assertEqual(removed, 1)
        self.assertEqual([e.prompt for e in kept], ["c"])


class Scoring(unittest.TestCase):
    def test_an_identical_plan_matches(self):
        self.assertTrue(scoring.plan_matches(PDF_PLAN + "<|im_end|>", PDF_PLAN))
        self.assertTrue(scoring.plan_matches("  " + PDF_PLAN + "\n", PDF_PLAN))

    def test_the_pdf_title_only_needs_to_be_present(self):
        other_title = PDF_PLAN.replace('"Gares"', '"Carte des gares"')
        self.assertTrue(scoring.plan_matches(other_title, PDF_PLAN))
        self.assertFalse(scoring.plan_matches(PDF_PLAN.replace('"Gares"', '"  "'), PDF_PLAN))
        no_title = PDF_PLAN.replace('"params":{"title":"Gares"}', '"params":{}')
        self.assertFalse(scoring.plan_matches(no_title, PDF_PLAN))

    def test_numbers_compare_by_value_but_not_types(self):
        self.assertTrue(scoring.plan_matches(PDF_PLAN.replace('"value":500', '"value":500.0'), PDF_PLAN))
        self.assertFalse(scoring.plan_matches(PDF_PLAN.replace('"value":500', '"value":501'), PDF_PLAN))
        self.assertFalse(scoring.plan_matches(PDF_PLAN.replace('"value":500', '"value":"500"'), PDF_PLAN))
        true = '{"kind":"calls","calls":[{"command":"a.b","params":{"dissolve":true}}]}'
        one = '{"kind":"calls","calls":[{"command":"a.b","params":{"dissolve":1}}]}'
        self.assertFalse(scoring.plan_matches(one, true))

    def test_step_order_and_parameters_count(self):
        plan = json.loads(PDF_PLAN)
        plan["calls"].reverse()
        self.assertFalse(scoring.plan_matches(json.dumps(plan), PDF_PLAN))
        plan = json.loads(PDF_PLAN)
        plan["calls"][0]["params"]["dissolve"] = True
        self.assertFalse(scoring.plan_matches(json.dumps(plan), PDF_PLAN))

    def test_a_question_or_refusal_expects_the_same_non_empty_kind(self):
        question = '{"kind":"question","question":"Quelle couche ?"}'
        refusal = '{"kind":"refusal","reason":"Aucune commande ne le fait."}'
        self.assertTrue(scoring.plan_matches('{"kind":"question","question":"Laquelle ?"}', question))
        self.assertFalse(scoring.plan_matches(refusal, question))
        self.assertFalse(scoring.plan_matches('{"kind":"question","question":" "}', question))
        self.assertTrue(scoring.plan_matches('{"kind":"refusal","reason":"Non."}', refusal))
        self.assertFalse(scoring.plan_matches(question, refusal))

    def test_an_unreadable_output_is_wrong(self):
        for output in ["", "Voici le plan : " + PDF_PLAN, PDF_PLAN[:-2], "[]",
                       '{"kind":"calls","calls":[]}',
                       '{"kind":"question","question":"Q ?","extra":1}']:
            self.assertFalse(scoring.plan_matches(output, PDF_PLAN), output)
        self.assertIsNone(scoring.read_plan('{"kind":"autre"}'))

    def test_the_score_counts_by_kind(self):
        score = scoring.Score()
        question = '{"kind":"question","question":"Quelle couche ?"}'
        score.record(PDF_PLAN, PDF_PLAN)
        score.record("charabia", PDF_PLAN)
        score.record(question, question)
        summary = score.to_dict()
        self.assertEqual((summary["justes"], summary["total"]), (2, 3))
        self.assertEqual(summary["par_sorte"]["calls"], {"justes": 1, "total": 2})
        self.assertEqual(summary["par_sorte"]["question"], {"justes": 1, "total": 1})
        self.assertEqual(summary["sorties_illisibles"], 1)


class PipelineGuards(unittest.TestCase):
    """Les gardes de `pipeline.py` qui se testent sans torch ni GPU."""

    def test_a_filled_output_folder_is_refused_without_resume(self):
        with tempfile.TemporaryDirectory() as folder:
            output = Path(folder) / "sortie"
            pipeline.prepare_output(output, resume=False)  # absent : créé
            (output / "rapport.json").write_text("{}", encoding="utf-8")
            with self.assertRaises(pipeline.PipelineStop):
                pipeline.prepare_output(output, resume=False)
            self.assertEqual((output / "rapport.json").read_text(encoding="utf-8"), "{}")
            pipeline.prepare_output(output, resume=True)  # accepté

    def test_an_output_folder_inside_the_repository_is_refused(self):
        with self.assertRaises(pipeline.PipelineStop):
            pipeline.prepare_output(pipeline.ROOT / "target" / "modele-essai", resume=False)

    def test_a_missing_or_too_small_gpu_stops_the_pipeline(self):
        with self.assertRaises(pipeline.PipelineStop):
            pipeline.check_gpu(None)
        with self.assertRaises(pipeline.PipelineStop):
            pipeline.check_gpu(4 * 1024**3)
        pipeline.check_gpu(8 * 1024**3)

    def test_the_base_and_llama_cpp_are_pinned(self):
        self.assertRegex(pipeline.BASE_REVISION, r"^[0-9a-f]{40}$")
        self.assertRegex(pipeline.LLAMA_CPP_COMMIT, r"^[0-9a-f]{40}$")
        self.assertNotEqual(pipeline.TRAINING_SEED, pipeline.VALIDATION_SEED)

    def test_the_llama_cpp_commit_matches_the_locked_llama_cpp_sys_2(self):
        # Le commit épinglé est le sous-module de cette version de llama-cpp-sys-2 : si le
        # noyau change de version, la chaîne doit changer de commit avec lui.
        lock = (pipeline.ROOT / "Cargo.lock").read_text(encoding="utf-8")
        versions = re.findall(r'name = "llama-cpp-sys-2"\r?\nversion = "([^"]+)"', lock)
        self.assertEqual(versions, [pipeline.LLAMA_CPP_SYS_VERSION])

    def test_a_failed_tool_reports_the_tail_of_its_stderr(self):
        command = [sys.executable, "-c", "import sys; sys.stderr.write('x' * 3000 + 'FIN'); sys.exit(3)"]
        with self.assertRaises(pipeline.PipelineStop) as stop:
            pipeline.run(command, capture_output=True, text=True)
        message = str(stop.exception)
        self.assertIn("FIN", message)
        self.assertLess(len(message), 2600)

    def test_a_tool_output_takes_its_final_name_only_on_success(self):
        with tempfile.TemporaryDirectory() as folder:
            final = Path(folder) / "modele.gguf"
            partial = pipeline.temporary(final)
            writes = f"open({str(partial)!r}, 'w').write('ok')"
            with self.assertRaises(pipeline.PipelineStop):
                pipeline.write_atomically(final, [sys.executable, "-c", writes + "; raise SystemExit(1)"])
            self.assertFalse(final.exists())
            pipeline.write_atomically(final, [sys.executable, "-c", writes])
            self.assertEqual(final.read_text(), "ok")
            self.assertFalse(partial.exists())

    def test_the_trial_uses_an_example_expecting_calls(self):
        question = Example("q", '{"kind":"question","question":"Laquelle ?"}<|im_end|>')
        calls = Example("c", PDF_PLAN + "<|im_end|>")
        self.assertEqual(pipeline.first_calls_example([question, calls]), calls)
        with self.assertRaises(pipeline.PipelineStop):
            pipeline.first_calls_example([question])

    def test_the_two_declarations_name_the_outputs_by_file_name_with_the_short_prompt(self):
        with tempfile.TemporaryDirectory() as folder:
            output = Path(folder)
            (output / pipeline.ADAPTED_DECLARATION_NAME).write_text("ancienne", encoding="utf-8")
            names = pipeline.write_declarations(output)
            again = pipeline.write_declarations(output)  # reprise : réécrites à l'identique
            self.assertEqual(names, again)
            self.assertEqual(
                sorted(p.name for p in output.iterdir()),
                sorted([pipeline.ADAPTED_DECLARATION_NAME, pipeline.MERGED_DECLARATION_NAME]),
            )
            adapted = json.loads((output / pipeline.ADAPTED_DECLARATION_NAME).read_text(encoding="utf-8"))
            merged = json.loads((output / pipeline.MERGED_DECLARATION_NAME).read_text(encoding="utf-8"))
        self.assertEqual(adapted, {"model": pipeline.BASE_NAME, "adapter": pipeline.ADAPTER_NAME, "prompt": "short"})
        self.assertEqual(merged, {"model": pipeline.MERGED_NAME, "prompt": "short"})
        for name in names:
            self.assertTrue(name.endswith(".modele.json"), name)
        for declaration in (adapted, merged):
            for file in [declaration["model"], declaration.get("adapter")]:
                if file is not None:
                    self.assertNotRegex(file, r"[/\\:]|\.\.")
                    self.assertTrue(file.endswith(".gguf"), file)


if __name__ == "__main__":
    unittest.main()
