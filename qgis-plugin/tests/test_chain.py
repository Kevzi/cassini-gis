"""Invariants de la chaîne fixe d'une ligne de la table fermée (AD-24), sans QGIS : fond en doublure.

Une ligne est une chaîne d'étapes ; seule la dernière écrit `out` ; l'annulabilité se déduit de la chaîne ;
aucune ligne ne cite un algorithme du réseau.
"""

import json
import os
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "cassini_qgis"))
sys.path.insert(0, HERE)

import analysis  # noqa: E402
import check_extension  # noqa: E402
import delegated  # noqa: E402
from test_analysis import FakeBackend, FakeLayer, start  # noqa: E402

GOLDEN = os.path.join(HERE, "table_annulable.json")


def two_step_row(**extra):
    """Ligne de test à deux étapes : la première écrit un temporaire, la seconde `out`."""
    row = {
        "chain_version": 1,
        "chain": [
            {"algorithm": "native:first", "translate": lambda params, layer: {"INPUT": layer}, "output": "OUTPUT"},
            {"algorithm": "native:second", "translate": lambda params, layer: {"INPUT": layer}, "output": "OUTPUT"},
        ],
        "input": "layer_id",
    }
    row.update(extra)
    return row


class ChainExecutionTest(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.dir.cleanup)
        self.out = os.path.join(self.dir.name, "1-test.chain.gpkg")

    def backend(self, temporary_count=1, **kw):
        backend = FakeBackend([FakeLayer("g")], **kw)
        backend.temporary_count = temporary_count
        return backend

    def params(self, **kw):
        base = {"layer_id": "g", "out": self.out}
        base.update(kw)
        return base

    def test_a_two_step_chain_writes_only_the_core_output(self):
        b = self.backend()
        _, calls = start(two_step_row(), b, self.params())
        self.assertEqual([c[0] for c in b.calls], ["native:first"])
        self.assertEqual(b.calls[0][1]["OUTPUT"], "TEMPORARY_OUTPUT")
        b.finished(True, {"OUTPUT": "temp-1"})
        self.assertEqual([c[0] for c in b.calls], ["native:first", "native:second"])
        self.assertEqual(b.calls[1][1]["INPUT"].lid, "temp-1")  # la 2e étape lit la sortie de la 1re
        self.assertEqual(b.calls[1][1]["OUTPUT"], self.out)
        b.finished(True, {"OUTPUT": self.out})
        self.assertIsNone(calls[0][1])
        self.assertEqual(calls[0][0]["layer_id"], "out_1")
        # Un seul fichier sur le disque, celui que le noyau a nommé ; aucun temporaire nommé.
        self.assertEqual(os.listdir(self.dir.name), [os.path.basename(self.out)])

    def test_a_chain_is_run_in_the_order_of_its_steps(self):
        b = self.backend()
        row = two_step_row()
        row["chain"].append({"algorithm": "native:third", "translate": lambda p, l: {"INPUT": l}, "output": "OUTPUT"})
        start(row, b, self.params())
        b.finished(True, {"OUTPUT": "t1"})
        b.finished(True, {"OUTPUT": "t2"})
        self.assertEqual([c[0] for c in b.calls], ["native:first", "native:second", "native:third"])
        self.assertEqual([c[1].get("OUTPUT") for c in b.calls[:1]], ["TEMPORARY_OUTPUT"])
        self.assertEqual(b.calls[1][1]["OUTPUT"], "TEMPORARY_OUTPUT")
        self.assertEqual(b.calls[2][1]["OUTPUT"], self.out)

    def test_cancelling_at_each_step_removes_the_output_and_answers_cancelled(self):
        for stage in (0, 1):
            with self.subTest(stage=stage):
                b = self.backend()
                g, calls = start(two_step_row(), b, self.params())
                with open(self.out, "wb") as f:  # sortie à moitié écrite
                    f.write(b"x")
                if stage == 1:
                    b.finished(True, {"OUTPUT": "temp-1"})
                handle = b.handle
                g.cancel()
                self.assertTrue(handle.cancelled)
                self.assertEqual(calls, [])  # la tâche tient encore le fichier : réponse à sa fin
                b.finished(False, {})
                self.assertEqual(calls[0][1].code, "qgis.cancelled")
                self.assertFalse(os.path.exists(self.out))
                self.assertEqual(os.listdir(self.dir.name), [])

    def test_a_guard_on_an_empty_intermediate_names_its_error(self):
        row = two_step_row()
        row["chain"][1]["guard"] = {"empty": ("analysis.origin_off_network", "Rien d'atteint.")}
        b = self.backend(temporary_count=0)
        _, calls = start(row, b, self.params())
        b.finished(True, {"OUTPUT": "temp-1"})
        self.assertEqual(calls[0][1].code, "analysis.origin_off_network")
        self.assertEqual(len(b.calls), 1)  # la 2e étape n'est pas lancée
        self.assertFalse(os.path.exists(self.out))

    def test_without_a_guard_an_empty_intermediate_goes_on(self):
        b = self.backend(temporary_count=0)
        start(two_step_row(), b, self.params())
        b.finished(True, {"OUTPUT": "temp-1"})
        self.assertEqual(len(b.calls), 2)

    def test_a_failing_step_fails_the_command_and_cleans_up(self):
        b = self.backend()
        _, calls = start(two_step_row(), b, self.params())
        b.finished(False, {})
        self.assertEqual(calls[0][1].code, "analysis.failed")
        self.assertEqual(len(b.calls), 1)

    def test_the_work_reprojection_runs_only_when_the_core_fills_work_epsg(self):
        entry = analysis.ALGORITHMS["analysis.buffer"]
        distance = {"value": 1, "unit": "km"}
        # Absent : l'algorithme seul, sur la couche d'origine, vers `out`.
        b = self.backend()
        start(entry, b, self.params(distance=distance))
        self.assertEqual([c[0] for c in b.calls], ["native:buffer"])
        self.assertEqual(b.calls[0][1]["OUTPUT"], self.out)
        # Rempli : `reprojectlayer` en temporaire d'abord, puis l'algorithme vers `out`.
        b = FakeBackend([FakeLayer("d", crs="EPSG:4326", metric=False)])
        start(entry, b, self.params(layer_id="d", distance=distance, work_epsg=32631))
        self.assertEqual([c[0] for c in b.calls], ["native:reprojectlayer"])
        self.assertEqual(b.calls[0][1]["TARGET_CRS"], "EPSG:32631")
        self.assertEqual(b.calls[0][1]["OUTPUT"], "TEMPORARY_OUTPUT")
        b.finished(True, {"OUTPUT": "temp-1"})
        self.assertEqual([c[0] for c in b.calls], ["native:reprojectlayer", "native:buffer"])
        self.assertEqual(b.calls[1][1]["OUTPUT"], self.out)

    def test_the_old_row_form_is_still_read_as_a_chain(self):
        entry = {
            "algorithm": "native:fake",
            "translate": lambda params, layer: {"INPUT": layer},
            "output": "OUTPUT",
            "input": "layer_id",
            "then": {"algorithm": "native:fake2", "translate": lambda p, l: {"INPUT": l}, "output": "OUTPUT"},
        }
        self.assertEqual([s["algorithm"] for s in analysis.chain_of(entry)], ["native:fake", "native:fake2"])


class TableInvariantsTest(unittest.TestCase):
    def test_every_row_has_a_chain_version_and_the_table_is_valid(self):
        rows = delegated.table_rows()
        self.assertEqual(analysis.validate_table(rows), [])
        # Version de chaîne attendue par ligne : 1 sauf montée décidée (AD-11, AD-25).
        raised = {"analysis.isochrone": 2}  # reprojection du réseau en tête de chaîne (entrée 10.32)
        for command_id, row in rows.items():
            with self.subTest(command=command_id):
                self.assertEqual(row["chain_version"], raised.get(command_id, 1))

    def test_layer_remove_is_a_fixed_gesture_row(self):
        row = delegated.table_rows()["layer.remove"]
        self.assertEqual((row["input"], row["output"]), ("project", "none"))
        self.assertNotIn("chain", row)
        self.assertFalse(analysis.derived_cancellable({"layer.remove": row})["layer.remove"])

    def test_the_golden_file_equals_the_derived_cancellability(self):
        with open(GOLDEN, encoding="utf-8") as f:
            golden = json.load(f)
        self.assertEqual(golden, analysis.derived_cancellable(delegated.table_rows()))

    def test_a_chain_is_cancellable_only_if_every_algorithm_is(self):
        row = two_step_row()
        row["chain"][0]["algorithm"] = "native:buffer"
        row["chain"][1]["algorithm"] = "native:convexhull"
        self.assertTrue(analysis.derived_cancellable({"x.y": row})["x.y"])
        row["chain"][1]["algorithm"] = "native:not_interruptible"
        self.assertFalse(analysis.derived_cancellable({"x.y": row})["x.y"])

    def test_a_row_citing_a_network_algorithm_is_refused_and_names_it(self):
        for algorithm in sorted(analysis.NETWORK_BLACKLIST):
            with self.subTest(algorithm=algorithm):
                row = two_step_row()
                row["chain"][0]["algorithm"] = algorithm
                errors = analysis.validate_table({"test.fetch": row})
                self.assertEqual(len(errors), 1)
                self.assertIn("test.fetch", errors[0])
                self.assertIn(algorithm, errors[0])

    def test_the_blacklist_names_the_five_network_algorithms(self):
        self.assertEqual(len(analysis.NETWORK_BLACKLIST), 5)
        self.assertIn("native:filedownloader", analysis.NETWORK_BLACKLIST)

    def test_another_provider_prefix_does_not_dodge_the_blacklist(self):
        row = two_step_row()
        row["chain"][0]["algorithm"] = "qgis:filedownloader"
        self.assertEqual(len(analysis.validate_table({"test.fetch": row})), 1)

    def test_the_recipe_refuses_a_table_with_a_network_algorithm(self):
        row = two_step_row()
        row["chain"][0]["algorithm"] = "native:filedownloader"
        errors = check_extension.check_table(check_extension.DEFAULT_PKG, {"test.fetch": row})
        self.assertEqual(len(errors), 1)
        self.assertIn("test.fetch", errors[0])
        self.assertIn("native:filedownloader", errors[0])

    def test_the_recipe_accepts_the_real_table(self):
        self.assertEqual(check_extension.check_table(check_extension.DEFAULT_PKG), [])

    def test_a_row_without_a_valid_chain_version_is_refused(self):
        for version in (None, 0, True, "1"):
            with self.subTest(version=version):
                row = two_step_row()
                if version is None:
                    del row["chain_version"]
                else:
                    row["chain_version"] = version
                errors = analysis.validate_table({"test.row": row})
                self.assertEqual(len(errors), 1)
                self.assertIn("test.row", errors[0])

    def test_a_last_step_cannot_be_conditional(self):
        row = two_step_row()
        row["chain"][1]["when"] = "other_param"
        self.assertEqual(len(analysis.validate_table({"test.row": row})), 1)

    def test_an_empty_chain_is_refused(self):
        row = two_step_row()
        row["chain"] = []
        self.assertEqual(len(analysis.validate_table({"test.row": row})), 1)

    def named(self, row, count=1):
        errors = analysis.validate_table({"test.row": row})
        self.assertEqual(len(errors), count, errors)
        self.assertTrue(all("test.row" in e for e in errors), errors)
        return errors

    def test_a_malformed_guard_is_refused_and_names_the_command(self):
        for guard in ("oops", {"empty": ("code", "message", "extra")}, {"empty": "code"}, ["empty"], None):
            with self.subTest(guard=guard):
                row = two_step_row()
                row["chain"][1]["guard"] = guard
                self.named(row)

    def test_a_guard_on_the_first_step_is_refused(self):
        row = two_step_row()
        row["chain"][0]["guard"] = {"empty": ("a.b", "message")}
        self.named(row)

    def test_a_chain_with_only_conditional_steps_is_refused_without_exception(self):
        row = two_step_row()
        for step in row["chain"]:
            step["when"] = "other_param"
        errors = self.named(row, count=2)  # aussi : la dernière étape est conditionnelle
        self.assertTrue(any("inconditionnelle" in e for e in errors))

    def test_a_row_without_chain_and_algorithm_is_reported_without_exception(self):
        self.named({"chain_version": 1, "input": "layer_id"})

    def test_a_work_crs_without_its_step_is_refused(self):
        row = two_step_row(work_crs="work_epsg")
        self.named(row)

    def test_a_work_epsg_step_without_work_crs_is_refused(self):
        row = two_step_row()
        row["chain"].insert(0, dict(analysis._REPROJECT_TO_WORK))
        self.named(row)

    def test_a_consistent_work_crs_row_is_accepted(self):
        row = two_step_row(work_crs="work_epsg")
        row["chain"].insert(0, dict(analysis._REPROJECT_TO_WORK))
        self.assertEqual(analysis.validate_table({"test.row": row}), [])

    def test_the_blacklist_ignores_case(self):
        row = two_step_row()
        row["chain"][0]["algorithm"] = "native:FileDownloader"
        errors = self.named(row)
        self.assertIn("native:FileDownloader", errors[0])

    def test_an_empty_chain_is_not_cancellable(self):
        row = two_step_row()
        row["chain"] = []
        self.assertFalse(analysis.derived_cancellable({"test.row": row})["test.row"])
        self.assertFalse(analysis.derived_cancellable({"test.row": {"chain_version": 1}})["test.row"])

    def test_the_recipe_reports_a_loading_failure_as_one_message(self):
        errors = check_extension.check_table(check_extension.DEFAULT_PKG, {"test.row": 5})
        self.assertEqual(len(errors), 1)
        self.assertIn("table fermée", errors[0])


class AnnouncementTest(unittest.TestCase):
    """Annonce fidele (AD-25) : une commande n'est annoncee que si toute sa chaine existe."""

    def announce(self, missing=()):
        backend = FakeBackend()
        backend.missing_algorithms = set(missing)
        registry = delegated.build_registry(lambda: None)
        commands, versions = delegated.announcement(registry, backend)
        return registry, set(commands), versions

    def test_a_complete_chain_is_announced_with_its_chain_version(self):
        registry, commands, versions = self.announce()
        self.assertEqual(commands, set(registry.ids()))
        self.assertEqual(versions["analysis.buffer"], analysis.ALGORITHMS["analysis.buffer"]["chain_version"])
        # Le geste fixe sans algorithme est annonce lui aussi, avec sa version.
        self.assertIn("layer.remove", versions)

    def test_the_isochrone_is_announced_in_chain_version_2_and_the_others_stay_in_1(self):
        _, _, versions = self.announce()
        self.assertEqual(versions["analysis.isochrone"], 2)
        self.assertEqual(versions["analysis.buffer"], 1)

    def test_the_shortest_path_is_announced_only_when_its_algorithm_exists(self):
        _, _, versions = self.announce()
        self.assertEqual(versions["network.shortest_path"], 1)
        backend = FakeBackend()
        backend.missing_algorithms = {"native:shortestpathpointtopoint"}
        available = analysis.available_chains(analysis.ALGORITHMS, backend)
        self.assertNotIn("network.shortest_path", available)
        self.assertIn("analysis.isochrone", available)

    def test_a_removed_algorithm_makes_its_command_unavailable(self):
        registry, commands, versions = self.announce(missing={"native:buffer"})
        self.assertNotIn("analysis.buffer", commands)
        self.assertNotIn("analysis.buffer", versions)
        self.assertIn("analysis.nearest", commands)  # une autre ligne n'en souffre pas

    def test_a_conditional_step_counts_as_required(self):
        # La reprojection de travail est une etape `when` de `analysis.buffer` : sans elle, pas d'annonce.
        self.assertIn("native:reprojectlayer", analysis.required_algorithms(analysis.ALGORITHMS["analysis.buffer"]))
        _, commands, _ = self.announce(missing={"native:reprojectlayer"})
        self.assertNotIn("analysis.buffer", commands)
        self.assertNotIn("data.reproject", commands)  # meme regle pour data.reproject

    def test_every_row_follows_the_same_rule(self):
        rows = delegated.table_rows()
        for command_id, row in rows.items():
            for algorithm in analysis.required_algorithms(row):
                _, commands, versions = self.announce(missing={algorithm})
                self.assertNotIn(command_id, commands, (command_id, algorithm))
                self.assertNotIn(command_id, versions)

    def test_the_other_gestures_are_still_announced(self):
        _, commands, _ = self.announce(missing={"native:buffer"})
        for command_id in ("layer.add", "layout.export_pdf", "layer.set_style"):
            self.assertIn(command_id, commands)

    def test_a_row_without_chain_version_is_not_announced(self):
        row = two_step_row()
        del row["chain_version"]
        self.assertEqual(analysis.available_chains({"test.row": row}, FakeBackend()), {})

    def test_an_unreadable_row_is_not_announced(self):
        self.assertEqual(analysis.available_chains({"test.row": {"chain_version": 1, "chain": [{}]}}, FakeBackend()), {})


class HelloParamsTest(unittest.TestCase):
    """`CoreClient.start` joint les versions a `session.hello` (ajout compatible, protocole 2)."""

    def hello(self, **kw):
        import client

        sent = []

        class Process:
            def start(self, program, args):
                pass

        c = client.CoreClient.__new__(client.CoreClient)
        c._p = Process()
        c._gestures = delegated.build_registry(lambda: None)
        c._request = lambda method, params, step: sent.append((method, params, step))
        c.start("cassini", r"C:\a\p.qgz", **kw)
        return sent[0][1], client.PROTOCOL_VERSION

    def test_the_versions_are_joined_when_given(self):
        params, version = self.hello(commands=["layer.add"], chain_versions={"layer.add": 1}, qgis_version="3.44.1")
        self.assertEqual(version, 2)
        self.assertEqual(params["project"], "C:/a/p.qgz")
        self.assertEqual(params["commands"], ["layer.add"])
        self.assertEqual(params["chain_versions"], {"layer.add": 1})
        self.assertEqual(params["qgis_version"], "3.44.1")

    def test_without_versions_the_hello_is_as_before(self):
        params, _ = self.hello()
        self.assertNotIn("chain_versions", params)
        self.assertNotIn("qgis_version", params)


if __name__ == "__main__":
    unittest.main()
