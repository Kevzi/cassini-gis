"""Le plafond de valeurs lu par l'extension suit celui du noyau.

Le garde-fou des attributs du noyau (`core/src/attribute_guard.rs`, ticket 5.57) tient pour
complète toute liste `values` montrée au modèle. Elle ne l'est que parce que l'extension lit une
valeur de plus que `MAX_VALUES` (`VALUES_LIMIT = MAX_VALUES + 1`) : un champ qui en a trop perd
alors sa clé `values` au lieu d'être montré coupé.
"""

import os
import re
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "cassini_qgis"))

import inventory  # noqa: E402

CORE_INVENTORY = os.path.join(HERE, "..", "..", "core", "src", "agent_inventory.rs")


class ValuesLimitTest(unittest.TestCase):
    def test_the_extension_reads_one_value_more_than_the_core_shows(self):
        with open(CORE_INVENTORY, encoding="utf-8") as f:
            found = re.search(r"pub const MAX_VALUES: usize = (\d+);", f.read())
        self.assertIsNotNone(found, "MAX_VALUES introuvable dans agent_inventory.rs")
        self.assertEqual(inventory.VALUES_LIMIT, int(found.group(1)) + 1)


if __name__ == "__main__":
    unittest.main()
