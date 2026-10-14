//! Covers `src/bench/report.rs`: the `va.bench.run/1` JSON line (docs/specs/m0-foundations.md
//! §4.6, §11).
#![allow(clippy::unwrap_used)]

use std::collections::BTreeSet;

use visual_assembler::bench::report::{BenchRun, RUN_SCHEMA};

/// The 18 keys of the schema, in their normative order.
const KEYS: [&str; 18] = [
    "schema",
    "scenario",
    "renderer",
    "backend",
    "adapter",
    "driver",
    "window_system",
    "wsl",
    "os",
    "arch",
    "version",
    "startup_ms",
    "frames_painted",
    "process_cpu_ms",
    "peak_rss_bytes",
    "peak_private_bytes",
    "settle_ms",
    "idle_window_ms",
];

/// A startup run with fixed values; the idle-only fields and the driver are `None`.
fn startup_run() -> BenchRun {
    BenchRun {
        schema: RUN_SCHEMA,
        scenario: "startup",
        renderer: "wgpu",
        backend: "dx12",
        adapter: Some("Test Adapter\nwith a newline".to_owned()),
        driver: None,
        window_system: "win32",
        wsl: false,
        os: "windows",
        arch: "x86_64",
        version: "0.1.0",
        startup_ms: 182.5,
        frames_painted: 1,
        process_cpu_ms: Some(15.625),
        peak_rss_bytes: Some(91_226_112),
        peak_private_bytes: None,
        settle_ms: None,
        idle_window_ms: None,
    }
}

#[test]
fn run_line_has_exact_schema() {
    let line = startup_run().to_json_line();
    assert!(!line.contains('\n'), "not a single line: {line}");
    assert!(!line.ends_with('\n'));

    let value: serde_json::Value = serde_json::from_str(&line).unwrap();
    let object = value.as_object().unwrap();
    let keys: BTreeSet<&str> = object.keys().map(String::as_str).collect();
    assert_eq!(keys, KEYS.into_iter().collect::<BTreeSet<_>>());

    assert_eq!(RUN_SCHEMA, "va.bench.run/1");
    assert_eq!(object["schema"], "va.bench.run/1");
    for key in [
        "driver",
        "peak_private_bytes",
        "settle_ms",
        "idle_window_ms",
    ] {
        assert!(object[key].is_null(), "{key} should be null: {line}");
    }
    assert_eq!(object["frames_painted"], 1);
    assert_eq!(object["peak_rss_bytes"], 91_226_112_u64);
    assert_eq!(object["startup_ms"], 182.5);
    assert_eq!(object["wsl"], false);
}

#[test]
fn run_line_keys_follow_schema_order() {
    let line = startup_run().to_json_line();
    let positions: Vec<usize> = KEYS
        .iter()
        .map(|key| line.find(&format!("\"{key}\":")).unwrap())
        .collect();
    assert!(
        positions.windows(2).all(|pair| pair[0] < pair[1]),
        "keys out of order: {line}"
    );
}

// ---------------------------------------------------------------------------------------------
// QA edge cases (audit round 1): the §4.6 example byte for byte (key order and number format),
// string escaping, and integer extremes.
// ---------------------------------------------------------------------------------------------

/// The example line of docs/specs/m0-foundations.md §4.6, verbatim.
const SPEC_EXAMPLE_LINE: &str = r#"{"schema":"va.bench.run/1","scenario":"idle","renderer":"wgpu","backend":"dx12","adapter":"AMD Radeon(TM) 860M Graphics","driver":"…","window_system":"win32","wsl":false,"os":"windows","arch":"x86_64","version":"0.1.0","startup_ms":182.4,"frames_painted":0,"process_cpu_ms":15.6,"peak_rss_bytes":91226112,"peak_private_bytes":40960000,"settle_ms":1000,"idle_window_ms":10000.7}"#;

#[test]
fn spec_example_line_is_reproduced_byte_for_byte() {
    let run = BenchRun {
        schema: RUN_SCHEMA,
        scenario: "idle",
        renderer: "wgpu",
        backend: "dx12",
        adapter: Some("AMD Radeon(TM) 860M Graphics".to_owned()),
        driver: Some("…".to_owned()),
        window_system: "win32",
        wsl: false,
        os: "windows",
        arch: "x86_64",
        version: "0.1.0",
        startup_ms: 182.4,
        frames_painted: 0,
        process_cpu_ms: Some(15.6),
        peak_rss_bytes: Some(91_226_112),
        peak_private_bytes: Some(40_960_000),
        settle_ms: Some(1000),
        idle_window_ms: Some(10000.7),
    };
    assert_eq!(run.to_json_line(), SPEC_EXAMPLE_LINE);
}

#[test]
fn driver_strings_are_escaped_and_unicode_survives() {
    let tricky = "Radeon™ \"860M\" \\ C:\\GPU\ttab\r\n\u{0}nul é 文字 🙂";
    let run = BenchRun {
        adapter: Some(tricky.to_owned()),
        driver: Some(tricky.to_owned()),
        ..startup_run()
    };
    let line = run.to_json_line();
    assert!(
        line.bytes().all(|byte| byte >= 0x20),
        "raw control byte in the line: {line:?}"
    );
    let value: serde_json::Value = serde_json::from_str(&line).unwrap();
    assert_eq!(value["adapter"], tricky);
    assert_eq!(value["driver"], tricky);
}

#[test]
fn integer_extremes_serialize_exactly() {
    let run = BenchRun {
        frames_painted: u64::MAX,
        peak_rss_bytes: Some(u64::MAX),
        peak_private_bytes: Some(0),
        startup_ms: 0.0,
        process_cpu_ms: Some(0.0),
        ..startup_run()
    };
    let line = run.to_json_line();
    let value: serde_json::Value = serde_json::from_str(&line).unwrap();
    assert_eq!(value["frames_painted"].as_u64(), Some(u64::MAX));
    assert_eq!(value["peak_rss_bytes"].as_u64(), Some(u64::MAX));
    assert_eq!(value["peak_private_bytes"].as_u64(), Some(0));
    assert_eq!(value["startup_ms"].as_f64(), Some(0.0));
    assert!(value["startup_ms"].is_number(), "{line}");
}
