//! Tests for `src/bench/stats.rs`: median, nearest-rank p95, min and max of bench samples.
#![allow(clippy::unwrap_used)]

use xtask::bench::stats::{Summary, nearest_rank, summarize, summarize_optional};

/// Builds `1.0..=n` as floats.
fn one_to(n: u32) -> Vec<f64> {
    (1..=n).map(f64::from).collect()
}

#[test]
fn empty_input_gives_none() {
    assert_eq!(summarize(&[]), None);
}

#[test]
fn single_value_is_every_statistic() {
    let expected = Summary {
        median: 5.0,
        p95: 5.0,
        min: 5.0,
        max: 5.0,
    };
    assert_eq!(summarize(&[5.0]), Some(expected));
}

#[test]
fn odd_count_unsorted_input() {
    let summary = summarize(&[3.0, 1.0, 2.0]).unwrap();
    assert_eq!(summary.median, 2.0);
    assert_eq!(summary.p95, 3.0);
    assert_eq!(summary.min, 1.0);
    assert_eq!(summary.max, 3.0);
}

#[test]
fn even_count_median_is_mean_of_middle_values() {
    let summary = summarize(&[1.0, 2.0, 3.0, 4.0]).unwrap();
    assert_eq!(summary.median, 2.5);
    assert_eq!(summary.p95, 4.0);
}

#[test]
fn twenty_values_use_nearest_rank_19() {
    let summary = summarize(&one_to(20)).unwrap();
    assert_eq!(summary.median, 10.5);
    assert_eq!(summary.p95, 19.0);
    assert_eq!(summary.min, 1.0);
    assert_eq!(summary.max, 20.0);
}

#[test]
fn hundred_values_use_nearest_rank_95() {
    let summary = summarize(&one_to(100)).unwrap();
    assert_eq!(summary.median, 50.5);
    assert_eq!(summary.p95, 95.0);
}

#[test]
fn nan_gives_none() {
    assert_eq!(summarize(&[1.0, f64::NAN]), None);
}

#[test]
fn infinity_gives_none() {
    assert_eq!(summarize(&[1.0, f64::INFINITY]), None);
}

#[test]
fn input_order_does_not_matter() {
    let mut reversed = one_to(20);
    reversed.reverse();
    assert_eq!(summarize(&reversed), summarize(&one_to(20)));
}

#[test]
fn nearest_rank_is_clamped_and_exact() {
    assert_eq!(nearest_rank(0, 95), 0);
    assert_eq!(nearest_rank(1, 95), 1);
    assert_eq!(nearest_rank(3, 95), 3);
    assert_eq!(nearest_rank(20, 95), 19);
    assert_eq!(nearest_rank(100, 95), 95);
    assert_eq!(nearest_rank(5, 0), 1);
}

#[test]
fn optional_metric_is_none_when_any_value_is_missing() {
    assert_eq!(summarize_optional([Some(1.0), None, Some(3.0)]), None);
    assert_eq!(
        summarize_optional([Some(1.0), Some(3.0)]).unwrap().median,
        2.0
    );
}

// ---------------------------------------------------------------------------------------------
// QA edge cases (audit round 1): p95 rounding against an exact integer oracle, overflow-prone
// medians, signed zeros, duplicates and non-finite values anywhere in the input.
// ---------------------------------------------------------------------------------------------

/// Exact nearest rank of p95 for `n` values: `ceil(95 n / 100)` in integers, clamped to [1, n].
fn oracle_p95_rank(n: usize) -> usize {
    (95 * n).div_ceil(100).clamp(1, n)
}

#[test]
fn p95_matches_the_exact_nearest_rank_for_every_size_up_to_1000() {
    for n in 1..=1000_u32 {
        // A deterministic rotation, so the input is not sorted.
        let mut values = one_to(n);
        values.rotate_left((n as usize * 7) % n as usize);
        let rank = oracle_p95_rank(n as usize);
        assert_eq!(nearest_rank(n as usize, 95), rank, "n = {n}");
        let summary = summarize(&values).unwrap();
        assert_eq!(summary.p95, rank as f64, "n = {n}");
        assert_eq!(summary.min, 1.0, "n = {n}");
        assert_eq!(summary.max, f64::from(n), "n = {n}");
        assert_eq!(summary.median, (f64::from(n) + 1.0) / 2.0, "n = {n}");
    }
}

#[test]
fn p95_rounding_boundaries() {
    // (n, expected 1-based rank). 0.95 n is an integer only when n is a multiple of 20.
    for (n, rank) in [
        (1, 1),
        (2, 2),
        (19, 19),
        (20, 19),
        (21, 20),
        (39, 38),
        (40, 38),
        (41, 39),
        (60, 57),
        (99, 95),
        (100, 95),
        (101, 96),
        (120, 114),
        (1000, 950),
    ] {
        assert_eq!(nearest_rank(n, 95), rank, "n = {n}");
    }
}

#[test]
fn nearest_rank_extremes_neither_overflow_nor_leave_the_range() {
    assert_eq!(nearest_rank(7, 100), 7);
    assert_eq!(
        nearest_rank(7, 150),
        7,
        "percentiles above 100 clamp to 100"
    );
    assert_eq!(nearest_rank(7, 1), 1);
    let huge = usize::MAX;
    let expected = usize::try_from((huge as u128 * 95).div_ceil(100)).unwrap();
    assert_eq!(nearest_rank(huge, 95), expected);
    assert_eq!(nearest_rank(huge, 100), huge);
}

#[test]
fn median_of_extreme_values_does_not_overflow() {
    let top = summarize(&[f64::MAX, f64::MAX]).unwrap();
    assert_eq!(top.median, f64::MAX);
    let wide = summarize(&[-f64::MAX, f64::MAX]).unwrap();
    assert_eq!(wide.median, 0.0);
    assert_eq!(wide.min, -f64::MAX);
    assert_eq!(wide.max, f64::MAX);
    let tiny = summarize(&[f64::MIN_POSITIVE, 5e-324]).unwrap();
    assert!(tiny.median > 0.0);
}

#[test]
fn negative_duplicate_and_signed_zero_values() {
    let summary = summarize(&[10.0, -5.0, 0.0, -5.0]).unwrap();
    assert_eq!(summary.median, -2.5);
    assert_eq!(summary.min, -5.0);
    assert_eq!(summary.max, 10.0);
    assert_eq!(summary.p95, 10.0);

    let zeros = summarize(&[0.0, -0.0]).unwrap();
    assert_eq!(zeros.median, 0.0);
    assert_eq!(zeros.min, 0.0);
    assert_eq!(zeros.max, 0.0);

    let same = summarize(&[3.0; 7]).unwrap();
    assert_eq!(
        (same.median, same.p95, same.min, same.max),
        (3.0, 3.0, 3.0, 3.0)
    );
}

#[test]
fn every_permutation_gives_the_same_summary() {
    let base = [4.0, 1.0, 3.0, 1.0, 2.0];
    let expected = summarize(&base).unwrap();
    assert_eq!(expected.median, 2.0);
    assert_eq!(expected.p95, 4.0);
    // Heap's algorithm over the 120 orderings.
    let mut values = base;
    let mut counters = [0_usize; 5];
    let mut seen = 1;
    assert_eq!(summarize(&values), Some(expected));
    let mut i = 0;
    while i < values.len() {
        if counters[i] < i {
            if i % 2 == 0 {
                values.swap(0, i);
            } else {
                values.swap(counters[i], i);
            }
            assert_eq!(summarize(&values), Some(expected), "{values:?}");
            seen += 1;
            counters[i] += 1;
            i = 0;
        } else {
            counters[i] = 0;
            i += 1;
        }
    }
    assert_eq!(seen, 120);
}

#[test]
fn non_finite_values_anywhere_give_none() {
    for values in [
        &[f64::NAN][..],
        &[f64::NAN, 1.0, 2.0],
        &[1.0, 2.0, f64::NEG_INFINITY],
        &[f64::INFINITY, f64::NEG_INFINITY],
        &[1.0, -f64::NAN],
    ] {
        assert_eq!(summarize(values), None, "{values:?}");
    }
}

#[test]
fn optional_summary_of_nothing_or_all_missing_is_none() {
    assert_eq!(summarize_optional(std::iter::empty()), None);
    assert_eq!(summarize_optional([None, None]), None);
    assert_eq!(summarize_optional([Some(f64::NAN)]), None);
    assert_eq!(
        summarize_optional([Some(2.0)]),
        Some(Summary {
            median: 2.0,
            p95: 2.0,
            min: 2.0,
            max: 2.0
        })
    );
}
