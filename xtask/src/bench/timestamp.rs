//! Pure UTC formatting of `SystemTime`, without a date crate (§6.2).
//!
//! Dates come from Howard Hinnant's days-to-civil algorithm (proleptic Gregorian calendar).
//! Times before 1970 are supported; leap seconds do not exist in Unix time.

use std::time::{SystemTime, UNIX_EPOCH};

/// Seconds in one day.
const SECONDS_PER_DAY: i64 = 86_400;

/// Broken-down UTC date and time.
struct UtcDateTime {
    /// Proleptic Gregorian year.
    year: i64,
    /// Month, 1..=12.
    month: i64,
    /// Day of the month, 1..=31.
    day: i64,
    /// Hour, 0..=23.
    hour: i64,
    /// Minute, 0..=59.
    minute: i64,
    /// Second, 0..=59.
    second: i64,
}

impl UtcDateTime {
    /// Converts `t`, truncating sub-second precision toward the past.
    fn from_system_time(t: SystemTime) -> Self {
        let seconds = match t.duration_since(UNIX_EPOCH) {
            Ok(after) => i64::try_from(after.as_secs()).unwrap_or(i64::MAX),
            Err(before) => {
                let before = before.duration();
                let whole = i64::try_from(before.as_secs()).unwrap_or(i64::MAX);
                // 1.5 s before the epoch is second -2 when truncating toward the past.
                let partial = i64::from(before.subsec_nanos() > 0);
                whole.saturating_neg().saturating_sub(partial)
            }
        };
        Self::from_unix_seconds(seconds)
    }

    /// Converts a count of seconds since 1970-01-01T00:00:00Z.
    fn from_unix_seconds(seconds: i64) -> Self {
        let days = seconds.div_euclid(SECONDS_PER_DAY);
        let second_of_day = seconds.rem_euclid(SECONDS_PER_DAY);
        let (year, month, day) = civil_from_days(days);
        Self {
            year,
            month,
            day,
            hour: second_of_day / 3600,
            minute: second_of_day % 3600 / 60,
            second: second_of_day % 60,
        }
    }
}

/// Converts days since 1970-01-01 into `(year, month, day)`.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days.saturating_add(719_468);
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097); // [0, 146096]
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365; // [0, 399]
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100); // [0, 365]
    let shifted_month = (5 * day_of_year + 2) / 153; // [0, 11], March = 0
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1; // [1, 31]
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// Formats `t` as `YYYYMMDDTHHMMSSZ` (report file names).
pub fn utc_compact(t: SystemTime) -> String {
    let d = UtcDateTime::from_system_time(t);
    format!(
        "{:04}{:02}{:02}T{:02}{:02}{:02}Z",
        d.year, d.month, d.day, d.hour, d.minute, d.second
    )
}

/// Formats `t` as `YYYY-MM-DDTHH:MM:SSZ` (ISO 8601, report `created_utc`).
pub fn utc_iso(t: SystemTime) -> String {
    let d = UtcDateTime::from_system_time(t);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        d.year, d.month, d.day, d.hour, d.minute, d.second
    )
}
