"""Recette de la table fermée (AD-24, amendement du 2026-11-05).

Les gestes de l'extension sont exactement les commandes `runs_in: qgis` du
registre, ni plus ni moins. Le registre est lu à travers le catalogue figé, que
`core/tests/catalogue.rs` confronte déjà au registre : l'égalité tient par
transitivité, sans que le Python dépende du binaire du noyau.
"""

import json
import os
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "cassini_qgis"))

import delegated  # noqa: E402

CATALOGUE = os.path.join(
    HERE, "..", "..", "_bmad-output", "initiative-cassini-mvp", "spec-cassini-mvp", "catalogue-des-commandes.md"
)

# Commandes compilées pour les tests seulement : absentes du catalogue, comme dans
# `TEST_PREFIX` de `core/tests/catalogue.rs`.
TEST_PREFIX = "test."

def read_fiches(text):
    """Rend les fiches du catalogue, une par bloc ` ```json fiche `.

    Découpage ligne à ligne, bornes comparées sans espaces autour, comme
    `parse_catalogue` de `core/tests/catalogue.rs` : les deux recettes lisent les
    mêmes fiches, sans quoi la transitivité ne tiendrait plus.
    """
    fiches = []
    body = None
    for line in text.splitlines():
        if body is None:
            if line.strip() == "```json fiche":
                body = []
        elif line.strip() == "```":
            fiches.append(json.loads("\n".join(body)))
            body = None
        else:
            body.append(line)
    if body is not None:
        raise ValueError("catalogue : bloc ```json fiche non refermé")
    return fiches


def delegated_ids(fiches):
    """Ids des commandes faites et déléguées à QGIS, sans les commandes de test."""
    return {
        f["id"]
        for f in fiches
        if f.get("status") == "faite" and f.get("runs_in") == "qgis" and not f["id"].startswith(TEST_PREFIX)
    }


def compare(fiches, table_ids):
    """Rend un message par écart entre la table et les commandes déléguées, vide si égales."""
    expected = delegated_ids(fiches)
    found = {i for i in table_ids if not i.startswith(TEST_PREFIX)}
    runs_in = {f["id"]: f.get("runs_in") for f in fiches}
    errors = []
    for command_id in sorted(found - expected):
        if runs_in.get(command_id) == "core":
            reason = "calculée par le noyau (runs_in: core)"
        elif command_id in runs_in:
            reason = "pas encore faite au catalogue"
        else:
            reason = "absente du catalogue"
        errors.append(f"{command_id} : ligne de la table fermée, mais commande {reason}")
    for command_id in sorted(expected - found):
        errors.append(f"{command_id} : commande déléguée à QGIS (runs_in: qgis) sans ligne dans la table fermée")
    return errors


def load_fiches():
    with open(CATALOGUE, encoding="utf-8") as f:
        return read_fiches(f.read())


class ClosedTableTest(unittest.TestCase):
    def setUp(self):
        self.fiches = load_fiches()
        self.table = delegated.build_registry(object).ids()

    def test_the_table_is_exactly_the_delegated_commands(self):
        self.assertEqual(compare(self.fiches, self.table), [])

    def test_the_style_gesture_announces_the_result_ramp_kind(self):
        # Décision du 2026-11-09 : sans `result_ramp` annoncée, le noyau ouvre ses rasters de
        # résultat sans style.
        kinds = delegated.build_registry(object).style_kinds()
        self.assertIn("result_ramp", kinds)
        self.assertIn("hillshade", kinds)

    def test_the_catalogue_names_delegated_commands(self):
        # Garde contre une lecture vide qui rendrait la recette vraie sans rien comparer.
        self.assertIn("analysis.buffer", delegated_ids(self.fiches))
        self.assertIn("data.transform_point", delegated_ids(self.fiches))

    def test_fences_are_read_like_the_rust_reader(self):
        text = '  ```json fiche  \n{"id": "a.b", "status": "faite", "runs_in": "qgis"}\n```  \n'
        self.assertEqual(delegated_ids(read_fiches(text)), {"a.b"})

    def test_the_test_probe_is_registered_but_not_compared(self):
        self.assertIn(delegated.PROBE_ID, self.table)

    def test_a_row_for_a_core_command_fails_and_names_it(self):
        errors = compare(self.fiches, self.table + ["terrain.slope"])
        self.assertEqual(
            errors, ["terrain.slope : ligne de la table fermée, mais commande calculée par le noyau (runs_in: core)"]
        )

    def test_a_row_for_raster_reclassify_fails_and_names_it(self):
        # Le cas d'AD-24 : `raster.reclassify`, planifiée au noyau, ne doit pas survivre en ligne.
        errors = compare(self.fiches, self.table + ["raster.reclassify"])
        self.assertEqual(
            errors,
            ["raster.reclassify : ligne de la table fermée, mais commande calculée par le noyau (runs_in: core)"],
        )

    def test_a_row_for_an_unknown_command_fails_and_names_it(self):
        errors = compare(self.fiches, self.table + ["analysis.fake"])
        self.assertEqual(errors, ["analysis.fake : ligne de la table fermée, mais commande absente du catalogue"])

    def test_a_delegated_command_without_row_fails_and_names_it(self):
        table = [i for i in self.table if i != "layer.describe"]
        errors = compare(self.fiches, table)
        self.assertEqual(
            errors,
            ["layer.describe : commande déléguée à QGIS (runs_in: qgis) sans ligne dans la table fermée"],
        )


if __name__ == "__main__":
    unittest.main()
