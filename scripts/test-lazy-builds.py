#!/usr/bin/env python3
"""Keep the native lazy-build evidence gate strict as retained rows are added."""
import unittest
from component_probe import probe_lazy_builds


def batch(indices, count=None):
    return f"lazy-list built={len(indices) if count is None else count} first={min(indices)} last={max(indices)} total=10002 indices={indices}"


class LazyBuildEvidence(unittest.TestCase):
    def report(self, middle):
        return probe_lazy_builds("\n".join([batch(list(range(6))), middle, batch(list(range(9996, 10002)))]))

    def test_contiguous_cache_and_one_explicit_kept_row_are_accepted(self):
        result = self.report(batch([0] + list(range(9996, 10002))))
        self.assertTrue(result["passed"])
        self.assertEqual(result["kept_item_build_batches"], 1)
        self.assertEqual(result["max_built_per_pass"], 7)

    def test_nine_cached_rows_plus_one_kept_row_is_the_upper_bound(self):
        self.assertEqual(self.report(batch([0] + list(range(500, 509))))["max_built_per_pass"], 10)
        with self.assertRaises(AssertionError):
            self.report(batch([0] + list(range(500, 510))))

    def test_contiguous_ten_row_cache_is_rejected(self):
        with self.assertRaises(AssertionError):
            self.report(batch(list(range(500, 510))))

    def test_duplicate_counts_and_arbitrary_holes_are_rejected(self):
        for indices, count in [([0, 1, 1, 2], None), ([0, 1, 2], 4), ([0, 500, 502], None), ([2, 500, 501], None)]:
            with self.subTest(indices=indices, count=count), self.assertRaises(AssertionError):
                self.report(batch(indices, count))

    def test_both_endpoints_and_explicit_indices_are_required(self):
        for trace in [batch(list(range(6))), batch(list(range(9996, 10002))), "lazy-list built=6 first=0 last=5 total=10002"]:
            with self.subTest(trace=trace), self.assertRaises(AssertionError):
                probe_lazy_builds(trace)


if __name__ == "__main__":
    unittest.main()
