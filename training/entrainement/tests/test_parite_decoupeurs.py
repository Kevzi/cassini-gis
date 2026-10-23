"""Tests de la comparaison des découpeurs (ticket 5.10, tâche 4b), sans torch ni transformers.

    python3 -I -m unittest discover -s training/entrainement/tests
"""

import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))

import argparse  # noqa: E402

import parite_decoupeurs as parite  # noqa: E402
import pipeline  # noqa: E402
from data import Example  # noqa: E402


class TokenizerParity(unittest.TestCase):
    def test_identical_tokens_have_no_gap(self):
        self.assertEqual(parite.compare([[1, 2, 3], [4]], [[1, 2, 3], [4]]), (0, None))

    def test_gaps_are_counted_and_the_first_is_located(self):
        count, first = parite.compare([[1, 2, 3], [4, 5, 6, 7], [8]], [[1, 2, 3], [4, 9, 6, 7], [0]], context=2)
        self.assertEqual(count, 2)
        self.assertEqual(first, parite.Gap(prompt=1, token=1, hugging_face=[5, 6], llama_cpp=[9, 6]))

    def test_a_longer_cut_is_a_gap_at_the_end_of_the_shorter(self):
        count, first = parite.compare([[1, 2]], [[1, 2, 3]])
        self.assertEqual(count, 1)
        self.assertEqual((first.token, first.hugging_face, first.llama_cpp), (2, [], [3]))

    def test_both_sides_must_cut_the_same_prompts(self):
        with self.assertRaises(ValueError):
            parite.compare([[1]], [])

    def test_the_ids_are_read_from_the_last_line_of_llama_tokenize(self):
        self.assertEqual(parite.parse_ids("chargement...\n[151644, 8948, 198]\n"), [151644, 8948, 198])
        self.assertEqual(parite.parse_ids("[]"), [])
        for bad in ("", "\n", "pas une liste", "['a']"):
            with self.subTest(bad=bad), self.assertRaises((ValueError, SyntaxError)):
                parite.parse_ids(bad)

    def test_an_unreadable_llama_tokenize_output_stops_the_command(self):
        for bad in ("", "pas une liste", "[1, 2"):
            with self.subTest(bad=bad), self.assertRaises(pipeline.PipelineStop):
                parite.read_ids(bad)

    def test_a_count_of_zero_or_less_is_refused(self):
        self.assertEqual(parite.positive("300"), 300)
        for bad in ("0", "-3"):
            with self.subTest(bad=bad), self.assertRaises(argparse.ArgumentTypeError):
                parite.positive(bad)

    def test_a_set_with_fewer_prompts_than_asked_stops_the_command(self):
        examples = [Example(prompt="a", completion="x<|im_end|>"), Example(prompt="b", completion="y<|im_end|>")]
        self.assertEqual(parite.take(examples, 1), ["a"])
        with self.assertRaises(pipeline.PipelineStop):
            parite.take(examples, 3)


if __name__ == "__main__":
    unittest.main()
