//! Covers `src/bench/report.rs`: the `va.bench.run/2` JSON line (docs/specs/m0-foundations.md
//! §19.5, which extends the §4.6 keys; §11).
#![allow(clippy::unwrap_used)]

use std::collections::BTreeSet;

use visual_assembler::bench::report::{BenchRun, RUN_SCHEMA};

/// The 22 keys of the schema, in their normative order (§19.5: the 18 keys of §4.6 plus
/// `renderer_requested`, `fallback_from`, `fallback_reason` and `injected_failure`).
const KEYS: [&str; 22] = [
    "schema",
    "scenario",
    "renderer_requested",
    "renderer",
    "fallback_from",
    "fallback_reason",
    "injected_failure",
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

/// A startup run with fixed values, without fallback or injection; the idle-only fields and the
/// driver are `None`.
fn startup_run() -> BenchRun {
    BenchRun {
        schema: RUN_SCHEMA,
        scenario: "startup",
        renderer_requested: "auto",
        renderer: "wgpu",
        fallback_from: None,
        fallback_reason: None,
        injected_failure: None,
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

    assert_eq!(RUN_SCHEMA, "va.bench.run/2");
    assert_eq!(object["schema"], "va.bench.run/2");
    assert_eq!(object["renderer_requested"], "auto");
    for key in [
        "fallback_from",
        "fallback_reason",
        "injected_failure",
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
// QA edge cases (audit round 1): the spec example byte for byte (key order and number format),
// string escaping, and integer extremes. §19.5 replaces the §4.6 example with the one below.
// ---------------------------------------------------------------------------------------------

/// The normative example line of docs/specs/m0-foundations.md §19.5, verbatim.
const SPEC_EXAMPLE_LINE: &str = r#"{"schema":"va.bench.run/2","scenario":"startup","renderer_requested":"auto","renderer":"wgpu","fallback_from":"glow","fallback_reason":"injected renderer failure (VA_TEST_FAIL_RENDERER)","injected_failure":"glow","backend":"dx12","adapter":"AMD Radeon(TM) 860M Graphics","driver":"…","window_system":"win32","wsl":false,"os":"windows","arch":"x86_64","version":"0.1.0","startup_ms":512.3,"frames_painted":1,"process_cpu_ms":437.5,"peak_rss_bytes":358612992,"peak_private_bytes":120586240,"settle_ms":null,"idle_window_ms":null}"#;

#[test]
fn spec_example_line_is_reproduced_byte_for_byte() {
    let run = BenchRun {
        schema: RUN_SCHEMA,
        scenario: "startup",
        renderer_requested: "auto",
        renderer: "wgpu",
        fallback_from: Some("glow"),
        fallback_reason: Some("injected renderer failure (VA_TEST_FAIL_RENDERER)".to_owned()),
        injected_failure: Some("glow".to_owned()),
        backend: "dx12",
        adapter: Some("AMD Radeon(TM) 860M Graphics".to_owned()),
        driver: Some("…".to_owned()),
        window_system: "win32",
        wsl: false,
        os: "windows",
        arch: "x86_64",
        version: "0.1.0",
        startup_ms: 512.3,
        frames_painted: 1,
        process_cpu_ms: Some(437.5),
        peak_rss_bytes: Some(358_612_992),
        peak_private_bytes: Some(120_586_240),
        settle_ms: None,
        idle_window_ms: None,
    };
    assert_eq!(run.to_json_line(), SPEC_EXAMPLE_LINE);
}

/// An idle line keeps the §4.6 idle values (settle and window) next to the §19.5 keys, and the
/// injected value is reported raw even when it simulated nothing (`vulkan` is ignored).
#[test]
fn idle_line_with_a_forced_renderer_and_an_unknown_injection() {
    let run = BenchRun {
        scenario: "idle",
        renderer_requested: "glow",
        renderer: "glow",
        injected_failure: Some("vulkan".to_owned()),
        backend: "gl",
        frames_painted: 0,
        settle_ms: Some(3000),
        idle_window_ms: Some(10_000.7),
        ..startup_run()
    };
    let value: serde_json::Value = serde_json::from_str(&run.to_json_line()).unwrap();
    assert_eq!(value["renderer_requested"], "glow");
    assert_eq!(value["renderer"], "glow");
    assert!(value["fallback_from"].is_null());
    assert!(value["fallback_reason"].is_null());
    assert_eq!(value["injected_failure"], "vulkan");
    assert_eq!(value["settle_ms"], 3000);
    assert_eq!(value["idle_window_ms"], 10_000.7);
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
