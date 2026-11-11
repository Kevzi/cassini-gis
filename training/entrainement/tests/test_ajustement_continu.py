"""Tests de l'ajustement continu de la chaîne d'entraînement (ticket 5.54), sans torch.
Une ligne de la matrice du ticket par test, sur des dossiers de parent synthétiques écrits par
le test ; plus la lignée complète, la part du rappel comptée et la lignée d'un entraînement de
zéro.

    python3 -I -m unittest discover -s training/entrainement/tests
"""

import argparse
import hashlib
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))

import pipeline  # noqa: E402
from data import Example  # noqa: E402

CALLS = '{"kind":"calls","calls":[{"command":"essai.synthetique","params":{"n":%d}}]}<|im_end|>'
FAMILIES = "export_pdf_format,refus"
SNAPSHOT = "/home/x/.cache/huggingface/hub/models--Qwen--Qwen3.5-4B/snapshots/" + pipeline.BASE_REVISION


class GpuReached(Exception):
    """Levée à la place du contrôle du GPU : les gardes sont passées avant lui."""


def options(**values) -> argparse.Namespace:
    base = {"output": Path("/nulle-part"), "resume": False, "examples": None, "validation": 300, "max_steps": 2,
            "external": None, "name": None, "external_only": False, "external_validation": None,
            "parent": None, "families": None, "recall_share": None, "long_first": False,
            "pytorch_validation": False}
    return argparse.Namespace(**(base | values))


def safetensors(path: Path, keys: list[str]) -> Path:
    """Un fichier safetensors minimal : un tenseur F32 d'une valeur par clé."""
    header = {key: {"dtype": "F32", "shape": [1], "data_offsets": [4 * i, 4 * i + 4]} for i, key in enumerate(keys)}
    header["__metadata__"] = {"format": "pt"}
    raw = json.dumps(header).encode("utf-8")
    path.write_bytes(len(raw).to_bytes(8, "little") + raw + b"\0" * (4 * len(keys)))
    return path


LORA_KEYS = ["base_model.model.model.language_model.layers.0.mlp.down_proj.lora_A.weight",
             "base_model.model.model.language_model.layers.0.mlp.down_proj.lora_B.weight"]


def write_parent(folder: Path, lignee: list | None = None, report: dict | None = None,
                 config: dict | None = None, weights: bytes | None = None) -> Path:
    """Un dossier de sortie de `pipeline.py`, comme celui d'`ajuste-apres-5-49-b` : sans `lignee`
    par défaut (avant 5.54)."""
    adapter = folder / "travail" / "adaptateur-hf"
    adapter.mkdir(parents=True)
    rapport = {
        "base": {"depot": pipeline.BASE_REPO, "revision": pipeline.BASE_REVISION, "licence": "Apache-2.0"},
        "depot_cassini": {"commit": "b8c3bdc24176ab855a92aa3df4ef0e585ea8360c"},
        "graines": {"entrainement": 1, "validation": 2, "torch": 2},
        "hyperparametres": dict(pipeline.HYPERPARAMETERS),
        "donnees": {"entrainement": 8600},
        "validation": {"justes": 290, "total": 300},
    } | (report or {})
    if lignee is not None:
        rapport["lignee"] = lignee
    (folder / "rapport.json").write_text(json.dumps(rapport), encoding="utf-8")
    adapter_config = {"base_model_name_or_path": SNAPSHOT, "revision": None, "r": 16, "lora_alpha": 32,
                      "target_modules": pipeline.HYPERPARAMETERS["modules"], "lora_dropout": 0.05, "bias": "none",
                      "use_dora": False, "use_rslora": False, "modules_to_save": None} | (config or {})
    (adapter / "adapter_config.json").write_text(json.dumps(adapter_config), encoding="utf-8")
    if weights is None:
        safetensors(adapter / "adapter_model.safetensors", LORA_KEYS)
    else:
        (adapter / "adapter_model.safetensors").write_bytes(weights)
    return folder


def weights_sha(folder: Path) -> str:
    return hashlib.sha256((folder / "travail" / "adaptateur-hf" / "adapter_model.safetensors").read_bytes()).hexdigest()


def link(rank: int, sha: str, depart: str = "base") -> dict:
    return {"maillon": rank, "depart": depart, "commit": "c" * 40, "graine_torch": 1, "adaptateur_sha256": sha}


class Continuation(unittest.TestCase):
    def setUp(self):
        self.folder = tempfile.TemporaryDirectory()
        self.dir = Path(self.folder.name)
        self.output = self.dir / "sortie"

    def tearDown(self):
        self.folder.cleanup()

    def refused_before_the_gpu(self, args: argparse.Namespace) -> str:
        """Lance la chaîne ; rend le message du refus, qui doit tomber avant le GPU et avant
        toute écriture dans le dossier de sortie."""
        with mock.patch.object(pipeline, "gpu_memory", side_effect=GpuReached) as gpu:
            with self.assertRaises(pipeline.PipelineStop) as stop:
                pipeline.launch(args)
        gpu.assert_not_called()
        self.assertFalse(self.output.exists())
        return str(stop.exception)

    def test_a_session_from_b_mixes_focus_and_recall_at_the_lower_rate_with_a_two_link_lineage(self):
        parent = write_parent(self.dir / "ajuste-apres-5-49-b")
        args = options(output=self.output, parent=parent, families=FAMILIES)
        # Toutes les gardes passent avant le GPU, sans rien écrire ni dans le parent ni en sortie.
        before = {p: p.read_bytes() for p in parent.rglob("*") if p.is_file()}
        with mock.patch.object(pipeline, "gpu_memory", side_effect=GpuReached):
            with self.assertRaises(GpuReached):
                pipeline.launch(args)
        self.assertFalse(self.output.exists())
        self.assertEqual({p: p.read_bytes() for p in parent.rglob("*") if p.is_file()}, before)

        continuation = pipeline.check_continuation(args)
        self.assertEqual(continuation.families, ["export_pdf_format", "refus"])
        self.assertEqual(continuation.recall_share, 0.4)
        self.assertEqual(pipeline.effective_hyperparameters(continuation)["taux_apprentissage"], 5e-5)
        plan = pipeline.generation_plan(self.dir, pipeline.CONTINUOUS_COUNT, 300, continuation)
        self.assertEqual(list(plan), ["rappel", "cible", "validation"])
        self.assertEqual(plan["rappel"][1:], (pipeline.TRAINING_SEED, 1200 + pipeline.RECALL_MARGIN, []))
        self.assertEqual(plan["cible"][1:], (pipeline.FOCUS_SEED, 1800, ["--familles", FAMILIES]))
        self.assertEqual(plan["validation"][1:], (pipeline.VALIDATION_SEED, 300, []))
        lineage = pipeline.lineage(continuation, "d" * 40, 1)
        sha = weights_sha(parent)
        self.assertEqual([l["maillon"] for l in lineage], [1, 2])
        self.assertEqual(lineage[0], {"maillon": 1, "depart": "base", "commit": "b8c3bdc24176ab855a92aa3df4ef0e585ea8360c",
                                      "graine_torch": 2, "adaptateur_sha256": sha, "reconstruit": True})
        self.assertEqual(lineage[1], {"maillon": 2, "depart": sha, "commit": "d" * 40, "graine_torch": 1,
                                      "adaptateur_sha256": None, "familles": ["export_pdf_format", "refus"],
                                      "part_rappel": 0.4, "exemples": 3000})
        # Le parent garde sa lignée : la nouvelle est une copie.
        self.assertNotIn("lignee", json.loads((parent / "rapport.json").read_text(encoding="utf-8")))

    def test_a_parent_on_another_base_or_revision_is_refused_as_a_broken_chain(self):
        cases = [
            ("rapport-depot", {"report": {"base": {"depot": "Qwen/Qwen3-4B", "revision": pipeline.BASE_REVISION}}}),
            ("rapport-revision", {"report": {"base": {"depot": pipeline.BASE_REPO, "revision": "0" * 40}}}),
            ("config-revision", {"config": {"base_model_name_or_path": SNAPSHOT.replace(pipeline.BASE_REVISION, "1" * 40)}}),
            ("config-depot", {"config": {"base_model_name_or_path": "Qwen/Qwen3-4B", "revision": pipeline.BASE_REVISION}}),
            ("config-sans-revision", {"config": {"base_model_name_or_path": pipeline.BASE_REPO, "revision": None}}),
        ]
        for name, change in cases:
            parent = write_parent(self.dir / name, **change)
            message = self.refused_before_the_gpu(options(output=self.output, parent=parent, families=FAMILIES))
            self.assertIn("chaîne coupée", message, name)
        # Un nom de dépôt avec sa révision dans `revision` se continue.
        parent = write_parent(self.dir / "par-nom", config={"base_model_name_or_path": pipeline.BASE_REPO,
                                                            "revision": pipeline.BASE_REVISION})
        pipeline.check_continuation(options(parent=parent, families=FAMILIES))
        # Par la ligne de commande : code 1, rien d'écrit.
        with mock.patch.object(pipeline, "gpu_memory", side_effect=GpuReached) as gpu:
            code = pipeline.main(["--sortie", str(self.output), "--depuis", str(self.dir / "rapport-revision"),
                                  "--familles", FAMILIES])
        self.assertEqual(code, 1)
        gpu.assert_not_called()
        self.assertFalse(self.output.exists())

    def test_a_parent_with_other_modules_rank_or_alpha_is_refused_as_a_broken_chain(self):
        cases = [
            ("rapport-modules", {"report": {"hyperparametres": pipeline.HYPERPARAMETERS | {"modules": "q_proj"}}}),
            ("rapport-rang", {"report": {"hyperparametres": pipeline.HYPERPARAMETERS | {"rang": 8}}}),
            ("rapport-alpha", {"report": {"hyperparametres": pipeline.HYPERPARAMETERS | {"alpha": 16}}}),
            ("config-modules", {"config": {"target_modules": ["q_proj", "v_proj"]}}),
            ("config-rang", {"config": {"r": 32}}),
            ("config-alpha", {"config": {"lora_alpha": 64}}),
        ]
        for name, change in cases:
            parent = write_parent(self.dir / name, **change)
            message = self.refused_before_the_gpu(options(output=self.output, parent=parent, families=FAMILIES))
            self.assertIn("chaîne coupée", message, name)
        # Un adaptateur de domaine (5.23) reste hors de la chaîne.
        parent = write_parent(self.dir / "domaine", report={"donnees": {"externe": {"nom": "domaine"}}})
        self.assertIn("chaîne coupée",
                      self.refused_before_the_gpu(options(output=self.output, parent=parent, families=FAMILIES)))

    def test_a_fourth_link_is_refused_and_asks_for_a_recalibration(self):
        folder = write_parent(self.dir / "troisieme")
        sha = weights_sha(folder)
        lignee = [link(1, "a" * 64), link(2, "b" * 64, "a" * 64), link(3, sha, "b" * 64)]
        (folder / "rapport.json").write_text(json.dumps(
            json.loads((folder / "rapport.json").read_text(encoding="utf-8")) | {"lignee": lignee}), encoding="utf-8")
        message = self.refused_before_the_gpu(options(output=self.output, parent=folder, families=FAMILIES))
        self.assertIn("recalage : entraînement de zéro sur le jeu complet", message)

    def test_a_substituted_adapter_is_refused(self):
        folder = write_parent(self.dir / "substitue", lignee=[link(1, "e" * 64)])
        message = self.refused_before_the_gpu(options(output=self.output, parent=folder, families=FAMILIES))
        self.assertIn("substitué", message)
        self.assertIn("e" * 64, message)

    def test_an_incomplete_or_unreadable_parent_is_refused_naming_the_file(self):
        for missing in ["rapport.json", "travail/adaptateur-hf/adapter_config.json",
                        "travail/adaptateur-hf/adapter_model.safetensors"]:
            folder = write_parent(self.dir / missing.replace("/", "-"))
            (folder / missing).unlink()
            message = self.refused_before_the_gpu(options(output=self.output, parent=folder, families=FAMILIES))
            self.assertIn(str(folder / missing), message)
        unreadable = [
            ("rapport-illisible", "rapport.json", b"{pas du json"),
            ("config-illisible", "travail/adaptateur-hf/adapter_config.json", b"[1, 2]"),
            ("poids-illisibles", "travail/adaptateur-hf/adapter_model.safetensors", b"\xff" * 12),
        ]
        for name, path, content in unreadable:
            folder = write_parent(self.dir / name)
            (folder / path).write_bytes(content)
            message = self.refused_before_the_gpu(options(output=self.output, parent=folder, families=FAMILIES))
            self.assertIn(str(folder / path), message, name)
        # Des poids qui ne sont pas un adaptateur LoRA, une lignée mal formée, un dossier absent.
        folder = write_parent(self.dir / "pas-lora")
        safetensors(folder / "travail/adaptateur-hf/adapter_model.safetensors", ["model.layers.0.mlp.weight"])
        self.assertIn("LoRA", self.refused_before_the_gpu(options(output=self.output, parent=folder, families=FAMILIES)))
        folder = write_parent(self.dir / "lignee-mal-formee", lignee=[link(2, "a" * 64)])
        self.assertIn("lignée", self.refused_before_the_gpu(options(output=self.output, parent=folder, families=FAMILIES)))
        self.assertIn(str(self.dir / "absent"), self.refused_before_the_gpu(
            options(output=self.output, parent=self.dir / "absent", families=FAMILIES)))

    def test_an_unknown_family_is_refused_listing_the_known_names(self):
        parent = write_parent(self.dir / "b")
        message = self.refused_before_the_gpu(options(output=self.output, parent=parent, families="refus,nexiste_pas"))
        self.assertIn("nexiste_pas", message)
        self.assertIn("champ_de_vision_cumule", message)  # la liste des noms connus
        for families in ["refus,refus", "refus,", ""]:
            self.refused_before_the_gpu(options(output=self.output, parent=parent, families=families))
        # Une famille que le générateur refuse au jeu ciblé (FOCUS_EXCLUDED) est refusée ici aussi.
        # La liste est vide depuis 5.54, lot 2 (les cinq familles du premier relevé réparées) : lue
        # telle quelle dans lib.rs, puis le refus essayé sur une liste posée à la main.
        self.assertEqual(pipeline.focus_excluded(), [])
        with mock.patch.object(pipeline, "focus_excluded", return_value=["retrait_couche"]):
            message = self.refused_before_the_gpu(options(output=self.output, parent=parent,
                                                          families="refus,retrait_couche"))
        self.assertIn("retrait_couche", message)

    def test_a_recall_share_under_the_floor_is_refused(self):
        parent = write_parent(self.dir / "b")
        for share in [0.3, 0.39, 1.0, 1.5]:
            message = self.refused_before_the_gpu(options(output=self.output, parent=parent, families=FAMILIES,
                                                          recall_share=share))
            self.assertIn("--part-rappel", message, share)
        self.assertEqual(pipeline.check_continuation(
            options(parent=parent, families=FAMILIES, recall_share=0.5)).recall_share, 0.5)

    def test_continuation_options_without_a_parent_or_with_external_examples_are_refused(self):
        parent = write_parent(self.dir / "b")
        for values in [{"families": FAMILIES}, {"recall_share": 0.5},
                       {"parent": parent},
                       {"parent": parent, "families": FAMILIES, "external": self.dir / "ext.jsonl", "name": "domaine"}]:
            with self.assertRaises(pipeline.PipelineStop, msg=values):
                pipeline.check_continuation(options(**values))
            self.refused_before_the_gpu(options(output=self.output, **values))
        # Trop peu d'exemples pour un rappel et un jeu ciblé.
        self.refused_before_the_gpu(options(output=self.output, parent=parent, families=FAMILIES, examples=1))

    def test_resuming_with_another_parent_or_other_families_is_refused(self):
        b = pipeline.check_continuation(options(parent=write_parent(self.dir / "b"), families=FAMILIES))
        other = pipeline.check_continuation(
            options(parent=write_parent(self.dir / "autre", weights=safetensors(self.dir / "w", LORA_KEYS[:1]).read_bytes()),
                    families=FAMILIES))
        report = {"depart": b.parent.adapter_sha256, "lignee": pipeline.lineage(b, "d" * 40, 1)}
        pipeline.check_resume_parent(report, b)
        pipeline.check_resume_parent({}, b)  # premier lancement
        # L'ordre des familles ne compte pas ; le nombre d'exemples, si.
        pipeline.check_resume_parent(report, pipeline.Continuation(b.parent, ["refus", "export_pdf_format"], 0.4))
        others = [other,
                  pipeline.Continuation(b.parent, ["refus"], 0.4),
                  pipeline.Continuation(b.parent, b.families, 0.5),
                  pipeline.Continuation(b.parent, b.families, 0.4, 2500),
                  None]
        for now in others:
            with self.assertRaises(pipeline.PipelineStop):
                pipeline.check_resume_parent(report, now)
        # Un entraînement de zéro commencé (d'avant 5.54 ou d'après) ne se reprend pas en séance.
        for scratch in [{"donnees": {"entrainement": 8600}}, {"lignee": pipeline.lineage(None, "d" * 40, 1)}]:
            pipeline.check_resume_parent(scratch, None)
            with self.assertRaises(pipeline.PipelineStop):
                pipeline.check_resume_parent(scratch, b)
        # Par la chaîne : refusé avant le GPU, le rapport commencé reste tel quel.
        self.output.mkdir()
        (self.output / "rapport.json").write_text(json.dumps(report), encoding="utf-8")
        args = options(output=self.output, resume=True, parent=self.dir / "b", families="refus")
        with mock.patch.object(pipeline, "gpu_memory", side_effect=GpuReached) as gpu:
            with self.assertRaises(pipeline.PipelineStop):
                pipeline.launch(args)
        gpu.assert_not_called()
        self.assertEqual(json.loads((self.output / "rapport.json").read_text(encoding="utf-8")), report)


class Lineage(unittest.TestCase):
    def test_a_report_carries_the_whole_lineage_up_to_three_links(self):
        with tempfile.TemporaryDirectory() as folder:
            parent_dir = write_parent(Path(folder) / "deuxieme")
            sha = weights_sha(parent_dir)
            lignee = [link(1, "a" * 64), link(2, sha, "a" * 64) | {"familles": ["refus"], "part_rappel": 0.4}]
            report = json.loads((parent_dir / "rapport.json").read_text(encoding="utf-8")) | {"lignee": lignee}
            (parent_dir / "rapport.json").write_text(json.dumps(report), encoding="utf-8")
            continuation = pipeline.check_continuation(options(parent=parent_dir, families="tampon"))
        lineage = pipeline.lineage(continuation, "f" * 40, 1)
        self.assertEqual(lineage[:2], lignee)
        self.assertEqual(lineage[2] | {"adaptateur_sha256": "x"},
                         {"maillon": 3, "depart": sha, "commit": "f" * 40, "graine_torch": 1,
                          "adaptateur_sha256": "x", "familles": ["tampon"], "part_rappel": 0.4, "exemples": 3000})
        self.assertEqual(pipeline.MAX_LINKS, 3)

    def test_a_training_from_scratch_writes_a_one_link_lineage(self):
        self.assertEqual(pipeline.lineage(None, "f" * 40, 2),
                         [{"maillon": 1, "depart": "base", "commit": "f" * 40, "graine_torch": 2,
                           "adaptateur_sha256": None}])

    def test_without_a_parent_data_and_hyperparameters_are_those_of_before(self):
        self.assertIsNone(pipeline.check_continuation(options()))
        self.assertEqual(pipeline.effective_hyperparameters(None), pipeline.HYPERPARAMETERS)
        self.assertEqual(pipeline.HYPERPARAMETERS["taux_apprentissage"], 2e-4)
        folder = Path("/donnees")
        self.assertEqual(pipeline.generation_plan(folder, pipeline.TRAINING_COUNT, 300), {
            "entrainement": (folder / "entrainement.jsonl", 1, 8600, []),
            "validation": (folder / "validation.jsonl", 2, 300, []),
        })
        pipeline.check_resume_parent({"donnees": {"entrainement": 8600}}, None)


class RecallShare(unittest.TestCase):
    def test_the_recall_share_is_counted_on_the_final_set_and_never_below_the_floor(self):
        self.assertEqual(pipeline.session_counts(3000, 0.4), (1200, 1800))
        recall, focus = pipeline.session_counts(7, 0.4)
        self.assertEqual((recall, focus), (3, 4))
        self.assertGreaterEqual(recall / 7, 0.4)
        recall_set = [Example(f"r{i}", CALLS % i) for i in range(4)]
        focus_set = [Example(f"c{i}", CALLS % (100 + i)) for i in range(5)] + [recall_set[0]]
        # Même demande, autre réponse : retirée aussi, dans le rappel comme dans le jeu ciblé.
        focus_set += [Example("r1", CALLS % 999), Example("c0", CALLS % 998)]
        training, info = pipeline.session_mix(recall_set + [Example("r9", CALLS % 9)], focus_set, recall_count=4)
        self.assertEqual(training, recall_set + focus_set[:5])
        self.assertEqual(info, {"exemples_rappel": 4, "exemples_cible": 5, "rappel_marge_non_prise": 1,
                                "cible_doublons_du_rappel": 2, "cible_doublons_internes": 1,
                                "part_rappel_comptee": round(4 / 9, 4)})
        with self.assertRaises(pipeline.PipelineStop):
            pipeline.session_mix(recall_set[:3], focus_set[:5])  # 3/8 < 0,4
        for count in (1, 0):
            with self.assertRaises(pipeline.PipelineStop):
                pipeline.session_counts(count, 0.4)


class Weights(unittest.TestCase):
    def test_a_partial_load_of_the_parent_weights_is_refused(self):
        base_keys = ["base_model.model.model.language_model.layers.0.mlp.down_proj.base_layer.weight"]
        pipeline.check_loaded_keys(base_keys, [], 352)  # les poids de la base ne comptent pas
        for missing, unexpected, loaded in [([LORA_KEYS[0].replace(".weight", ".default.weight")], [], 351),
                                            ([], ["model.autre.lora_A.weight"], 352),
                                            ([], [], 0)]:
            with self.assertRaises(pipeline.PipelineStop):
                pipeline.check_loaded_keys(base_keys + missing, unexpected, loaded)

    def test_the_family_names_are_read_from_the_generator(self):
        names = pipeline.known_families()
        self.assertIn("tampon", names)
        self.assertIn("export_pdf_format", names)
        self.assertEqual(len(names), len(set(names)))
        self.assertTrue(set(pipeline.focus_excluded()) <= set(names))


class ParentIdentity(unittest.TestCase):
    """Relecture : lignée maillon par maillon, parent complet, identité de l'adaptateur."""

    def setUp(self):
        self.folder = tempfile.TemporaryDirectory()
        self.dir = Path(self.folder.name)

    def tearDown(self):
        self.folder.cleanup()

    def refused(self, folder: Path, needle: str) -> None:
        with self.assertRaises(pipeline.PipelineStop, msg=needle) as stop:
            pipeline.check_continuation(options(parent=folder, families=FAMILIES))
        self.assertIn(needle, str(stop.exception))

    def test_a_lineage_is_checked_link_by_link(self):
        sha = "0" * 64
        cases = {
            "premier-pas-de-la-base": [link(1, "a" * 64, "b" * 64)],
            "depart-pas-le-precedent": [link(1, "a" * 64), link(2, "b" * 64, "c" * 64)],
            "empreinte-courte": [link(1, "abc")],
            "empreinte-majuscules": [link(1, "A" * 64)],
        }
        for name, lignee in cases.items():
            self.refused(write_parent(self.dir / name, lignee=lignee), "lignée")
        folder = write_parent(self.dir / "bonne")
        good = [link(1, sha), link(2, weights_sha(folder), sha)]
        report = json.loads((folder / "rapport.json").read_text(encoding="utf-8")) | {"lignee": good}
        (folder / "rapport.json").write_text(json.dumps(report), encoding="utf-8")
        self.assertEqual(pipeline.check_continuation(options(parent=folder, families=FAMILIES)).parent.lineage, good)

    def test_a_short_trial_or_an_unfinished_training_is_not_a_parent(self):
        self.refused(write_parent(self.dir / "court", report={
            "hyperparametres": pipeline.HYPERPARAMETERS | {"pas_max": 2}}), "essai court")
        folder = write_parent(self.dir / "inacheve")
        report = json.loads((folder / "rapport.json").read_text(encoding="utf-8"))
        del report["validation"]
        (folder / "rapport.json").write_text(json.dumps(report), encoding="utf-8")
        self.refused(folder, "validation")

    def test_report_fields_of_the_wrong_shape_name_the_file_never_a_trace(self):
        for key in ["base", "hyperparametres", "donnees", "graines", "depot_cassini"]:
            folder = write_parent(self.dir / key, report={key: [1, 2]})
            self.refused(folder, str(folder / "rapport.json"))
        folder = write_parent(self.dir / "sans-base")
        report = json.loads((folder / "rapport.json").read_text(encoding="utf-8"))
        del report["base"]
        (folder / "rapport.json").write_text(json.dumps(report), encoding="utf-8")
        self.refused(folder, str(folder / "rapport.json"))

    def test_the_rest_of_the_adapter_identity_must_match(self):
        for key, value in [("lora_dropout", 0.1), ("bias", "all"), ("use_dora", True), ("use_rslora", True),
                           ("modules_to_save", ["lm_head"])]:
            self.refused(write_parent(self.dir / key, config={key: value}), "chaîne coupée")
        # Absents, les trois derniers valent les défauts de peft.
        folder = write_parent(self.dir / "defauts")
        path = folder / "travail/adaptateur-hf/adapter_config.json"
        config = json.loads(path.read_text(encoding="utf-8"))
        for key in ["use_dora", "use_rslora", "modules_to_save"]:
            del config[key]
        path.write_text(json.dumps(config), encoding="utf-8")
        pipeline.check_continuation(options(parent=folder, families=FAMILIES))

    def test_a_rebuilt_first_link_needs_its_commit_and_seed(self):
        folder = write_parent(self.dir / "ancien", report={"depot_cassini": {}, "commit_cassini": "9" * 40})
        continuation = pipeline.check_continuation(options(parent=folder, families=FAMILIES))
        self.assertEqual(continuation.parent.lineage[0]["commit"], "9" * 40)
        self.refused(write_parent(self.dir / "sans-commit", report={"depot_cassini": {}}), "commit")
        self.refused(write_parent(self.dir / "sans-graine", report={"graines": {"entrainement": 1}}), "graines.torch")


class Bounds(unittest.TestCase):
    def test_the_python_constants_are_those_of_the_generator(self):
        constants = pipeline.generator_constants()
        self.assertEqual(constants, {"FOCUS_SEED": pipeline.FOCUS_SEED, "FOCUS_COUNT": pipeline.FOCUS_COUNT,
                                     "DEFAULT_COUNT": pipeline.TRAINING_COUNT})
        self.assertEqual(pipeline.session_counts(pipeline.CONTINUOUS_COUNT, pipeline.RECALL_SHARE)[1],
                         pipeline.FOCUS_COUNT)
        # Une famille dont le nom suit une ligne de commentaire est lue aussi.
        self.assertIn("decompte_par_zone", pipeline.known_families())

    def test_a_session_beyond_what_recouvrement_guards_is_refused_before_the_gpu(self):
        with tempfile.TemporaryDirectory() as folder:
            parent = write_parent(Path(folder) / "b")
            for examples, share, needle in [(4000, None, "FOCUS_COUNT"), (8700, 0.999, "DEFAULT_COUNT")]:
                with mock.patch.object(pipeline, "gpu_memory", side_effect=GpuReached) as gpu:
                    with self.assertRaises(pipeline.PipelineStop) as stop:
                        pipeline.launch(options(output=Path(folder) / "sortie", parent=parent, families=FAMILIES,
                                                examples=examples, recall_share=share))
                gpu.assert_not_called()
                self.assertIn(needle, str(stop.exception))

    def test_a_shorter_generated_set_is_refused(self):
        plan = {"cible": (Path("/x/cible.jsonl"), 3, 3, [])}
        pipeline.check_counts(plan, {"cible": [Example(f"c{i}", CALLS % i) for i in range(3)]})
        with self.assertRaises(pipeline.PipelineStop) as stop:
            pipeline.check_counts(plan, {"cible": [Example("c0", CALLS % 0)]})
        self.assertIn("/x/cible.jsonl", str(stop.exception))

    def test_a_generator_refusal_names_the_bar_sentence(self):
        refused = mock.Mock(returncode=3, stderr="jeu ciblé refusé, il reprend une phrase de barre : essai réel XX9\n")
        with tempfile.TemporaryDirectory() as folder, mock.patch.object(pipeline.subprocess, "run",
                                                                        return_value=refused):
            continuation = pipeline.Continuation(parent=None, families=["refus"], recall_share=0.4)
            with self.assertRaises(pipeline.PipelineStop) as stop:
                pipeline.generate(Path(folder), 3000, 300, continuation)
        self.assertIn("XX9", str(stop.exception))

    def test_mix_never_changes_the_training_side(self):
        training = [Example(f"t{i}", CALLS % i) for i in range(4)]
        validation = [Example("t1", CALLS % 1), Example("v", CALLS % 50)]
        mixed = pipeline.mix(training, validation, None)
        self.assertEqual(mixed["civil_training"], training)
        self.assertEqual(mixed["external_training"], [])
        self.assertEqual(mixed["civil_validation"], [validation[1]])


class FakeResult:
    def __init__(self, missing=(), unexpected=()):
        self.missing_keys, self.unexpected_keys = list(missing), list(unexpected)


class ParentWeights(unittest.TestCase):
    def setUp(self):
        self.folder = tempfile.TemporaryDirectory()
        self.adapter = Path(self.folder.name)
        safetensors(self.adapter / "adapter_model.safetensors", LORA_KEYS)
        self.sha = hashlib.sha256((self.adapter / "adapter_model.safetensors").read_bytes()).hexdigest()
        self.parent = {LORA_KEYS[0]: [0.5], LORA_KEYS[1]: [0.25]}

    def tearDown(self):
        self.folder.cleanup()

    def load(self, model_state: dict, sha: str | None = None, result=None) -> int:
        def set_state(model, state):
            model.update({k: v for k, v in state.items()} if model_state is None else model_state)
            return result or FakeResult(missing=["base.weight"])

        model: dict = {}
        return pipeline.load_parent_weights(
            model, self.adapter, sha or self.sha, load_file=lambda path: dict(self.parent),
            set_state=set_state, get_state=lambda m: m, same=lambda a, b: a == b)

    def test_the_parent_weights_are_checked_after_loading(self):
        self.assertEqual(self.load(None), 1)  # une matrice lora_B, égale à celle du parent
        # Une matrice lora_B restée à zéro (chargement sans effet) est refusée.
        with self.assertRaises(pipeline.PipelineStop):
            self.load({LORA_KEYS[0]: [0.5], LORA_KEYS[1]: [0.0]})
        with self.assertRaises(pipeline.PipelineStop):
            self.load({LORA_KEYS[0]: [0.5]})
        with self.assertRaises(pipeline.PipelineStop):
            self.load(None, result=FakeResult(unexpected=["autre.lora_A.weight"]))

    def test_the_file_is_hashed_again_just_before_loading(self):
        with self.assertRaises(pipeline.PipelineStop) as stop:
            self.load(None, sha="f" * 64)
        self.assertIn("substitué", str(stop.exception))


class StopAfterTraining(Exception):
    """Levée par la préparation de llama.cpp : le rapport est écrit jusqu'à l'entraînement."""


class LaunchPastTheGpu(unittest.TestCase):
    """La chaîne au-delà du GPU, sans torch : GPU, générateur, tokenizer et entraînement simulés."""

    def setUp(self):
        self.folder = tempfile.TemporaryDirectory()
        self.dir = Path(self.folder.name)
        self.output = self.dir / "sortie"
        self.calls: list = []

    def tearDown(self):
        self.folder.cleanup()

    def fake_generate(self, folder, count, validation_count, continuation=None):
        plan = pipeline.generation_plan(folder, count, validation_count, continuation)
        folder.mkdir(parents=True, exist_ok=True)
        for name, (path, _, n, _) in plan.items():
            path.write_text("".join(json.dumps({"prompt": f"{name} {i}", "completion": CALLS % i}) + "\n"
                                    for i in range(n)), encoding="utf-8")
        return {name: path for name, (path, _, _, _) in plan.items()}

    def fake_train(self, base_dir, examples, adapter_dir, max_steps, long_first=False, hyperparameters=None,
                   start_adapter=None, start_sha256=None):
        self.calls.append({"exemples": len(examples), "taux": hyperparameters["taux_apprentissage"],
                           "depart": start_adapter, "empreinte": start_sha256})
        adapter_dir.mkdir(parents=True, exist_ok=True)
        safetensors(adapter_dir / "adapter_model.safetensors", LORA_KEYS + ["x.lora_B.weight"])
        return {"modele": None, "pas": 1}

    def launch(self, args) -> dict:
        tokenizer = mock.Mock(side_effect=lambda text, add_special_tokens=False: {"input_ids": [0] * 10})
        modules = {
            "huggingface_hub": mock.Mock(snapshot_download=lambda repo, revision: "/base"),
            "transformers": mock.Mock(AutoTokenizer=mock.Mock(from_pretrained=lambda path: tokenizer)),
            "torch": mock.Mock(),
        }
        with mock.patch.dict(sys.modules, modules), \
                mock.patch.object(pipeline, "gpu_memory", return_value=(8 * 1024**3, "carte simulée")), \
                mock.patch.object(pipeline, "USE_UNSLOTH", False), \
                mock.patch.object(pipeline, "generate", side_effect=self.fake_generate), \
                mock.patch.object(pipeline, "install_fast_kernels", return_value={}), \
                mock.patch.object(pipeline, "train", side_effect=self.fake_train), \
                mock.patch.object(pipeline, "prepare_llama_cpp", side_effect=StopAfterTraining):
            with self.assertRaises(StopAfterTraining):
                pipeline.launch(args)
        return json.loads((self.output / "rapport.json").read_text(encoding="utf-8"))

    def test_a_continuous_session_trains_from_the_parent_and_reports_its_lineage(self):
        parent = write_parent(self.dir / "b")
        report = self.launch(options(output=self.output, parent=parent, families=FAMILIES))
        produced = hashlib.sha256((self.output / "travail/adaptateur-hf/adapter_model.safetensors").read_bytes()).hexdigest()
        self.assertEqual(self.calls, [{"exemples": 3000, "taux": 5e-5, "depart": parent.resolve() / "travail/adaptateur-hf",
                                       "empreinte": weights_sha(parent)}])
        self.assertEqual([l["maillon"] for l in report["lignee"]], [1, 2])
        self.assertEqual(report["lignee"][-1]["adaptateur_sha256"], produced)
        self.assertEqual(report["lignee"][-1]["depart"], weights_sha(parent))
        self.assertEqual((report["depart"], report["depuis"]), (weights_sha(parent), str(parent.resolve())))
        self.assertEqual(report["hyperparametres"]["taux_apprentissage"], 5e-5)
        seance = report["donnees"]["seance"]
        self.assertEqual((seance["exemples_rappel"], seance["exemples_cible"], seance["rappel_marge_non_prise"]),
                         (1200, 1800, pipeline.RECALL_MARGIN))
        self.assertEqual(seance["part_rappel_comptee"], 0.4)
        self.assertEqual(report["donnees"]["entrainement"], 3000)
        # En reprise, le maillon garde le commit enregistré.
        recorded = report["lignee"][-1]["commit"]
        report["lignee"][-1]["commit"] = "1" * 40
        (self.output / "rapport.json").write_text(json.dumps(report), encoding="utf-8")
        again = self.launch(options(output=self.output, resume=True, parent=parent, families=FAMILIES))
        self.assertEqual(again["lignee"][-1]["commit"], "1" * 40)
        self.assertNotEqual(recorded, "1" * 40)
        self.assertEqual(len(self.calls), 1)  # l'adaptateur déjà écrit n'est pas réentraîné

    def test_a_training_from_scratch_reports_as_before_plus_its_lineage(self):
        report = self.launch(options(output=self.output))
        self.assertEqual(self.calls, [{"exemples": pipeline.TRAINING_COUNT, "taux": 2e-4, "depart": None,
                                       "empreinte": None}])
        self.assertEqual(set(report), {"base", "llama_cpp", "depot_cassini", "graines", "hyperparametres", "gpu",
                                       "versions", "donnees", "chemin_entrainement", "noyaux", "entrainement",
                                       "lignee"})
        self.assertEqual(report["graines"], {"entrainement": 1, "validation": 2, "torch": pipeline.TORCH_SEED})
        self.assertEqual(report["hyperparametres"], pipeline.HYPERPARAMETERS | {"pas_max": 2})
        self.assertEqual(set(report["donnees"]), {
            "entrainement", "validation", "ecartes_trop_longs", "ecartes_trop_longs_validation",
            "validation_retires_car_vus_a_l_entrainement", "jetons_mediane", "jetons_max",
            "empreinte_entrainement", "empreinte_validation"})
        produced = hashlib.sha256((self.output / "travail/adaptateur-hf/adapter_model.safetensors").read_bytes()).hexdigest()
        self.assertEqual(report["lignee"], [{"maillon": 1, "depart": "base", "commit": report["depot_cassini"]["commit"],
                                             "graine_torch": pipeline.TORCH_SEED, "adaptateur_sha256": produced}])


if __name__ == "__main__":
    unittest.main()
