//! Tests for `src/bench/timestamp.rs`: UTC formatting without a date crate.
#![allow(clippy::unwrap_used)]

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use xtask::bench::timestamp::{utc_compact, utc_iso};

/// Returns the instant `seconds` after the Unix epoch.
fn at(seconds: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(seconds)
}

#[test]
fn compact_epoch() {
    assert_eq!(utc_compact(at(0)), "19700101T000000Z");
}

#[test]
fn compact_leap_day_2000() {
    assert_eq!(utc_compact(at(951_782_400)), "20000229T000000Z");
}

#[test]
fn compact_ordinary_instant() {
    assert_eq!(utc_compact(at(1_700_000_000)), "20231114T221320Z");
}

#[test]
fn compact_2100_is_not_a_leap_year() {
    assert_eq!(utc_compact(at(4_102_444_800)), "21000101T000000Z");
}

#[test]
fn iso_ordinary_instant() {
    assert_eq!(utc_iso(at(1_700_000_000)), "2023-11-14T22:13:20Z");
}

#[test]
fn sub_second_part_is_truncated() {
    let t = at(1_700_000_000) + Duration::from_millis(999);
    assert_eq!(utc_iso(t), "2023-11-14T22:13:20Z");
}

#[test]
fn instants_before_the_epoch_are_supported() {
    let one_second_before = UNIX_EPOCH - Duration::from_secs(1);
    assert_eq!(utc_iso(one_second_before), "1969-12-31T23:59:59Z");
    let half_second_before = UNIX_EPOCH - Duration::from_millis(500);
    assert_eq!(utc_iso(half_second_before), "1969-12-31T23:59:59Z");
}

// ---------------------------------------------------------------------------------------------
// QA edge cases (audit round 1): an independent naive calendar as oracle for every day from 1960
// to 2110, century and year-end boundaries, truncation before the epoch, extreme instants.
// ---------------------------------------------------------------------------------------------

/// Gregorian leap-year rule.
fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Days in `month` (1..=12) of `year`.
fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(year) => 29,
        _ => 28,
    }
}

/// Returns the instant `seconds` (possibly negative) after the Unix epoch.
fn at_signed(seconds: i64) -> SystemTime {
    if seconds >= 0 {
        UNIX_EPOCH + Duration::from_secs(seconds.unsigned_abs())
    } else {
        UNIX_EPOCH - Duration::from_secs(seconds.unsigned_abs())
    }
}

#[test]
fn every_day_from_1960_to_2110_matches_a_naive_calendar() {
    // 1960-01-01 is 3653 days before the epoch (three leap years: 1960, 1964, 1968).
    let (mut year, mut month, mut day) = (1960_i64, 1_i64, 1_i64);
    let mut days_since_epoch = -3653_i64;
    let mut checked = 0;
    while year <= 2110 {
        let varying = (days_since_epoch * 7919).rem_euclid(86_400);
        for second_of_day in [0, 86_399, varying] {
            let seconds = days_since_epoch * 86_400 + second_of_day;
            let expected = format!(
                "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
                second_of_day / 3600,
                second_of_day % 3600 / 60,
                second_of_day % 60
            );
            let t = at_signed(seconds);
            assert_eq!(utc_iso(t), expected, "unix seconds {seconds}");
            assert_eq!(
                utc_compact(t),
                expected.replace(['-', ':'], ""),
                "unix seconds {seconds}"
            );
            checked += 1;
        }
        days_since_epoch += 1;
        day += 1;
        if day > days_in_month(year, month) {
            day = 1;
            month += 1;
            if month > 12 {
                month = 1;
                year += 1;
            }
        }
    }
    // 2111-01-01 is 4449513600 s after the epoch, i.e. day 51499.
    assert_eq!(days_since_epoch, 51_499, "the loop must end on 2111-01-01");
    assert!(checked > 160_000, "{checked}");
}

#[test]
fn century_and_year_end_boundaries() {
    for (seconds, expected) in [
        (-2_203_891_201_i64, "1900-02-28T23:59:59Z"),
        (-2_203_891_200, "1900-03-01T00:00:00Z"),
        (-315_619_200, "1960-01-01T00:00:00Z"),
        (-86_401, "1969-12-30T23:59:59Z"),
        (-86_400, "1969-12-31T00:00:00Z"),
        (946_684_799, "1999-12-31T23:59:59Z"),
        (946_684_800, "2000-01-01T00:00:00Z"),
        (951_868_800, "2000-03-01T00:00:00Z"),
        (978_220_800, "2000-12-31T00:00:00Z"),
        (2_147_483_647, "2038-01-19T03:14:07Z"),
        (2_147_483_648, "2038-01-19T03:14:08Z"),
        (4_107_542_399, "2100-02-28T23:59:59Z"),
        (4_107_542_400, "2100-03-01T00:00:00Z"),
        (13_574_563_200, "2400-02-29T00:00:00Z"),
        (13_601_087_999, "2400-12-31T23:59:59Z"),
        (253_402_300_799, "9999-12-31T23:59:59Z"),
    ] {
        assert_eq!(
            utc_iso(at_signed(seconds)),
            expected,
            "unix seconds {seconds}"
        );
    }
}

#[test]
fn sub_second_instants_around_the_epoch_truncate_toward_the_past() {
    // 100 ns steps: the finest resolution every OS represents (Windows FILETIME ticks).
    let one_tick_before = UNIX_EPOCH - Duration::from_nanos(100);
    assert_eq!(utc_compact(one_tick_before), "19691231T235959Z");
    let just_over_one_second_before = UNIX_EPOCH - Duration::new(1, 100);
    assert_eq!(utc_iso(just_over_one_second_before), "1969-12-31T23:59:58Z");
    let just_under_one_second_after = UNIX_EPOCH + Duration::new(0, 999_999_900);
    assert_eq!(utc_iso(just_under_one_second_after), "1970-01-01T00:00:00Z");
}

#[test]
fn extreme_representable_instants_do_not_panic() {
    // The representable range depends on the OS (Windows FILETIME ends in year 30828, Linux
    // allows i64 seconds); every instant the OS can hold must format without panicking.
    for seconds in [
        u64::from(u32::MAX),
        253_402_300_800,
        900_000_000_000,
        i64::MAX.unsigned_abs(),
        u64::MAX,
    ] {
        for t in [
            UNIX_EPOCH.checked_add(Duration::from_secs(seconds)),
            UNIX_EPOCH.checked_sub(Duration::from_secs(seconds)),
            UNIX_EPOCH.checked_sub(Duration::new(seconds, 1)),
        ]
        .into_iter()
        .flatten()
        {
            let iso = utc_iso(t);
            let compact = utc_compact(t);
            assert!(iso.ends_with('Z'), "{iso}");
            assert!(compact.ends_with('Z'), "{compact}");
        }
    }
}
