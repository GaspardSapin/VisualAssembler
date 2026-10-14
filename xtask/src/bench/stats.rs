//! Pure statistics over bench samples: median, nearest-rank p95, min and max (§6.2).

use serde::Serialize;

/// Summary of one metric over the measured runs.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Summary {
    /// Middle value, or the mean of the two middle values when the count is even.
    pub median: f64,
    /// 95th percentile, nearest-rank method.
    pub p95: f64,
    /// Smallest value.
    pub min: f64,
    /// Largest value.
    pub max: f64,
}

/// Returns the 1-based nearest rank of `percentile` (0..=100) among `n` sorted values:
/// `ceil(percentile / 100 × n)`, clamped to `[1, n]`. Computed in integers, so it is exact.
///
/// Returns 0 only when `n == 0`.
pub fn nearest_rank(n: usize, percentile: u32) -> usize {
    if n == 0 {
        return 0;
    }
    let product = (n as u128) * u128::from(percentile.min(100));
    let rank = product.div_ceil(100);
    usize::try_from(rank).unwrap_or(n).clamp(1, n)
}

/// Summarizes `values`. The input order does not matter.
///
/// Returns `None` for an empty input or when any value is NaN or infinite.
pub fn summarize(values: &[f64]) -> Option<Summary> {
    if values.is_empty() || values.iter().any(|value| !value.is_finite()) {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let n = sorted.len();
    let middle = n / 2;
    let median = if n % 2 == 1 {
        sorted[middle]
    } else {
        f64::midpoint(sorted[middle - 1], sorted[middle])
    };
    Some(Summary {
        median,
        p95: sorted[nearest_rank(n, 95) - 1],
        min: sorted[0],
        max: sorted[n - 1],
    })
}

/// Summarizes a metric that may be missing: `None` as soon as one value is `None`
/// (§6.2: a summary metric is null when any sample has null for it).
pub fn summarize_optional(values: impl IntoIterator<Item = Option<f64>>) -> Option<Summary> {
    let values: Option<Vec<f64>> = values.into_iter().collect();
    summarize(&values?)
}
