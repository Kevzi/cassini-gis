"""Tests du fichier d'exemples externe de la chaîne d'entraînement (ticket 5.23), sans torch.
Un test par ligne de la matrice du ticket, sur des fichiers synthétiques écrits par le test.

    python3 -I -m unittest discover -s training/entrainement/tests
"""

import argparse
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))

import data  # noqa: E402
import pipeline  # noqa: E402
from data import Example  # noqa: E402

CALLS = '{"kind":"calls","calls":[{"command":"essai.synthetique","params":{"n":%d}}]}<|im_end|>'


def synthetic(n: int, start: int = 0) -> list[dict]:
    return [{"prompt": f"<|im_start|>user\nexterne {i}<|im_end|>\n", "completion": CALLS % i}
            for i in range(start, start + n)]


def write(path: Path, records: list) -> Path:
    path.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in records), encoding="utf-8")
    return path


def options(**values) -> argparse.Namespace:
    base = {"output": Path("/nulle-part"), "resume": False, "examples": 8, "validation": 4, "max_steps": 1,
            "external": None, "name": None, "external_only": False, "external_validation": None}
    return argparse.Namespace(**(base | values))


CIVIL_TRAINING = [Example("civil a", CALLS % 100), Example("civil b", CALLS % 101)]
CIVIL_VALIDATION = [Example("civil v", CALLS % 102)]


class ExternalExamples(unittest.TestCase):
    def setUp(self):
        self.folder = tempfile.TemporaryDirectory()
        self.dir = Path(self.folder.name)

    def tearDown(self):
        self.folder.cleanup()

    def test_default_mix_trains_on_civil_and_external_under_the_given_name(self):
        external = pipeline.read_external(write(self.dir / "ext.jsonl", synthetic(20)), None)
        mixed = pipeline.mix(CIVIL_TRAINING, CIVIL_VALIDATION, external)
        self.assertEqual(mixed["civil_training"], CIVIL_TRAINING)
        self.assertEqual(len(mixed["external_training"]), 18)
        self.assertEqual(len(mixed["external_validation"]), 2)
        self.assertEqual(mixed["civil_validation"], CIVIL_VALIDATION)  # le score civil reste mesuré
        outputs = pipeline.check_options(options(external=self.dir / "ext.jsonl", name="domaine-essai"))
        self.assertEqual(outputs.files, [pipeline.BASE_NAME, "domaine-essai-adaptateur-f16.gguf"])
        self.assertIsNone(outputs.merged)  # sans fusion
        names = pipeline.write_declarations(self.dir, outputs.declarations)
        self.assertEqual(names, ["domaine-essai.modele.json"])
        declaration = json.loads((self.dir / "domaine-essai.modele.json").read_text(encoding="utf-8"))
        self.assertEqual(declaration, {"model": pipeline.BASE_NAME,
                                       "adapter": "domaine-essai-adaptateur-f16.gguf", "prompt": "short"})
        # La part de validation est la même à chaque lecture (graine fixe).
        again = pipeline.read_external(self.dir / "ext.jsonl", None)
        self.assertEqual(again.validation, external.validation)
        # Le rapport n'en garde que l'empreinte et les comptes, jamais un prompt.
        self.assertEqual(external.info["exemples"], 20)
        self.assertRegex(external.info["empreinte"], r"^[0-9a-f]{64}$")
        self.assertNotIn("externe 1", json.dumps(external.info))

    def test_external_only_trains_on_the_external_file_alone(self):
        path = write(self.dir / "ext.jsonl", synthetic(10))
        pipeline.check_options(options(external=path, name="domaine", external_only=True))
        external = pipeline.read_external(path, None)
        mixed = pipeline.mix([], [], external)
        self.assertEqual(mixed["civil_training"], [])
        self.assertEqual(mixed["civil_validation"], [])
        self.assertEqual(len(mixed["external_training"]) + len(mixed["external_validation"]), 10)
        self.assertEqual(pipeline.mode_label(False), "externe seul")
        # L'essai prend un plan d'appels externe, dont le contenu n'est pas consigné.
        example, private = pipeline.trial_example(mixed["external_validation"], [])
        self.assertTrue(private)
        self.assertIn(example, external.validation)

    def test_a_given_validation_file_serves_the_external_score_and_nothing_is_taken_from_the_first(self):
        first = write(self.dir / "ext.jsonl", synthetic(10))
        second = write(self.dir / "val.jsonl", synthetic(3, start=50))
        external = pipeline.read_external(first, second)
        self.assertEqual(len(external.training), 10)
        self.assertEqual([e.prompt for e in external.validation], [r["prompt"] for r in synthetic(3, start=50)])
        self.assertEqual(external.info["validation_source"], "fichier")
        self.assertNotIn("part_validation", external.info)

    def test_a_malformed_line_stops_before_the_gpu_and_names_the_file_and_line(self):
        records = synthetic(3)
        records[1] = {"prompt": "sans réponse"}
        path = write(self.dir / "ext.jsonl", records)
        args = options(external=path, name="domaine")
        with mock.patch.object(pipeline, "gpu_memory") as gpu:
            with self.assertRaises(pipeline.PipelineStop) as stop:
                pipeline.launch(args)
        gpu.assert_not_called()
        self.assertIn(str(path), str(stop.exception))
        self.assertIn("ligne 2", str(stop.exception))

    def test_a_completion_that_is_not_a_plan_stops_before_the_gpu_and_names_the_file_and_line(self):
        for bad in ["bonjour<|im_end|>", '{"kind":"autre","x":1}<|im_end|>']:
            records = synthetic(3)
            records[2] = {"prompt": "réponse illisible", "completion": bad}
            path = self.dir / "ext.jsonl"
            path.write_text(json.dumps(records[0]) + "\n\n" + json.dumps(records[1]) + "\n"
                            + json.dumps(records[2], ensure_ascii=False) + "\n", encoding="utf-8")
            with mock.patch.object(pipeline, "gpu_memory") as gpu:
                with self.assertRaises(pipeline.PipelineStop, msg=bad) as stop:
                    pipeline.launch(options(external=path, name="domaine"))
            gpu.assert_not_called()
            self.assertIn(str(path), str(stop.exception))
            self.assertIn("ligne 4", str(stop.exception))  # la ligne vide compte

    def test_a_name_that_would_overwrite_the_civil_outputs_is_refused_before_any_work(self):
        path = write(self.dir / "ext.jsonl", synthetic(3))
        for name in ["cassini-composition", "cassini-composition-4b", "cassini-composition-4b-adaptateur",
                     "../ailleurs", "Domaine", "a/b", ""]:
            with mock.patch.object(pipeline, "gpu_memory") as gpu:
                with self.assertRaises(pipeline.PipelineStop, msg=name) as stop:
                    pipeline.launch(options(external=path, name=name))
            gpu.assert_not_called()
            self.assertIn("nom", str(stop.exception))
        civil = {pipeline.ADAPTER_NAME, pipeline.MERGED_NAME, *pipeline.DECLARATIONS}
        self.assertFalse(civil & set(pipeline.external_outputs("domaine").files[1:]))
        self.assertFalse(civil & set(pipeline.external_outputs("domaine").declarations))
        # Les options du fichier externe n'ont pas de sens sans lui.
        for extra in [{"name": "domaine"}, {"external_only": True}, {"external_validation": path}]:
            with self.assertRaises(pipeline.PipelineStop):
                pipeline.check_options(options(**extra))

    def test_a_validation_prompt_also_in_training_is_dropped_and_counted(self):
        first = write(self.dir / "ext.jsonl", synthetic(5))
        second = write(self.dir / "val.jsonl", synthetic(1, start=3) + synthetic(1, start=60))
        external = pipeline.read_external(first, second)
        mixed = pipeline.mix(CIVIL_TRAINING, CIVIL_VALIDATION + [CIVIL_TRAINING[0]], external)
        self.assertEqual(mixed["external_overlap"], 1)
        self.assertEqual([e.prompt for e in mixed["external_validation"]], [synthetic(1, start=60)[0]["prompt"]])
        self.assertEqual(mixed["civil_overlap"], 1)

    def test_without_the_option_outputs_and_data_are_those_of_5_17(self):
        outputs = pipeline.check_options(options())
        self.assertIs(outputs, pipeline.CIVIL_OUTPUTS)
        self.assertEqual(outputs.files, [pipeline.BASE_NAME, pipeline.ADAPTER_NAME, pipeline.MERGED_NAME])
        self.assertEqual(outputs.declarations, pipeline.DECLARATIONS)
        training = CIVIL_TRAINING
        validation = CIVIL_VALIDATION + [CIVIL_TRAINING[1]]
        mixed = pipeline.mix(training, validation, None)
        self.assertEqual((mixed["civil_validation"], mixed["civil_overlap"]), data.drop_overlap(validation, training))
        self.assertEqual(mixed["external_training"], [])
        self.assertEqual(pipeline.write_declarations(self.dir), list(pipeline.DECLARATIONS))
        # Sans option, aucune lecture externe et aucun contrôle de reprise ne s'en mêle.
        pipeline.check_resume({"donnees": {"entrainement": 2}}, None, True)


class Helpers(unittest.TestCase):
    def test_the_validation_split_is_deterministic_and_keeps_both_sides(self):
        examples = [Example(f"p{i}", CALLS % i) for i in range(30)]
        training, validation = data.split_validation(examples, 0.1, 2)
        self.assertEqual((len(training), len(validation)), (27, 3))
        self.assertEqual(data.split_validation(examples, 0.1, 2), (training, validation))
        self.assertNotEqual(data.split_validation(examples, 0.1, 3)[1], validation)
        self.assertEqual(sorted(training + validation, key=lambda e: int(e.prompt[1:])), examples)
        self.assertEqual([len(s) for s in data.split_validation(examples[:2], 0.1, 2)], [1, 1])
        with self.assertRaises(ValueError):
            data.split_validation(examples, 1.0, 2)

    def test_resuming_with_other_external_examples_is_refused(self):
        with tempfile.TemporaryDirectory() as folder:
            external = pipeline.read_external(write(Path(folder) / "ext.jsonl", synthetic(4)), None)
            other = pipeline.read_external(write(Path(folder) / "autre.jsonl", synthetic(5)), None)
        report = {"donnees": {"externe": external.info | {"melange": "civil + externe"}}}
        pipeline.check_resume(report, external, True)
        for args in [(other, True), (external, False), (None, True)]:
            with self.assertRaises(pipeline.PipelineStop):
                pipeline.check_resume(report, *args)
        with self.assertRaises(pipeline.PipelineStop):
            pipeline.check_resume({"donnees": {"entrainement": 2}}, external, True)
        pipeline.check_resume({}, external, True)  # premier lancement

    def test_a_private_trial_keeps_no_content(self):
        example = Example("<|im_start|>user\nexterne secret<|im_end|>\n", CALLS % 7)
        answer = CALLS.removesuffix("<|im_end|>") % 7
        done = mock.Mock(stdout=answer + "[end of text]")
        with tempfile.TemporaryDirectory() as folder, mock.patch.object(pipeline, "run", return_value=done):
            trial = pipeline.try_model(Path(folder), ["-m", "x"], example, Path(folder), "essai", private=True)
            public = pipeline.try_model(Path(folder), ["-m", "x"], example, Path(folder), "essai")
        self.assertTrue(trial["juste"])
        self.assertNotIn("sortie", trial)
        self.assertNotIn("attendu", trial)
        self.assertNotIn("synthetique", json.dumps(trial))
        self.assertIn("sortie", public)


if __name__ == "__main__":
    unittest.main()
