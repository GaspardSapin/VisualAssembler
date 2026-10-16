//! Tests for `src/bench/report.rs`: report file name, markdown table and the
//! `va.bench.report/2` document built from synthetic samples (idle spawn→frame is `null`,
//! §17.8; requested and used renderer, fallback count, shipped binary and renderer variables,
//! §19.5), and the `va.bench.run/2` line (§19.5 example). Temporary folders live under
//! `CARGO_TARGET_TMPDIR` (§17.9).
#![allow(clippy::unwrap_used)]

use std::{
    fmt,
    path::PathBuf,
    sync::atomic::{AtomicU32, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::{
    Deserializer as _,
    de::{IgnoredAny, MapAccess, Visitor},
};
use serde_json::Value;
use xtask::bench::{
    RendererChoice, Scenario,
    report::{
        BenchResult, BenchRun, ENV_VARS, EnvSnapshot, GitInfo, HostInfo, MARKDOWN_HEADER,
        MARKDOWN_SEPARATOR, MIXED, REPORT_SCHEMA, RUN_SCHEMA, Report, ResultConfig, SHIPPED_BINARY,
        Sample, UNKNOWN, report_file_name, write_report,
    },
    stats::Summary,
};

/// The run line example of m0-foundations.md §19.5 (it replaces the §4.6 example).
const SPEC_RUN_LINE: &str = r#"{"schema":"va.bench.run/2","scenario":"startup","renderer_requested":"auto","renderer":"wgpu","fallback_from":"glow","fallback_reason":"injected renderer failure (VA_TEST_FAIL_RENDERER)","injected_failure":"glow","backend":"dx12","adapter":"AMD Radeon(TM) 860M Graphics","driver":"…","window_system":"win32","wsl":false,"os":"windows","arch":"x86_64","version":"0.1.0","startup_ms":512.3,"frames_painted":1,"process_cpu_ms":437.5,"peak_rss_bytes":358612992,"peak_private_bytes":120586240,"settle_ms":null,"idle_window_ms":null}"#;

/// Bytes per MiB.
const MIB: u64 = 1_048_576;

/// Returns the instant `seconds` after the Unix epoch.
fn at(seconds: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(seconds)
}

/// A startup run line forced to wgpu, on DX12 / Win32.
fn startup_run() -> BenchRun {
    BenchRun {
        schema: "va.bench.run/2".to_owned(),
        scenario: "startup".to_owned(),
        renderer_requested: "wgpu".to_owned(),
        renderer: "wgpu".to_owned(),
        fallback_from: None,
        fallback_reason: None,
        injected_failure: None,
        backend: "dx12".to_owned(),
        adapter: Some("Test Adapter".to_owned()),
        driver: Some("1.0".to_owned()),
        window_system: "win32".to_owned(),
        wsl: false,
        os: "windows".to_owned(),
        arch: "x86_64".to_owned(),
        version: "0.1.0".to_owned(),
        startup_ms: 150.0,
        frames_painted: 1,
        process_cpu_ms: Some(120.0),
        peak_rss_bytes: Some(100 * MIB),
        peak_private_bytes: Some(50 * MIB),
        settle_ms: None,
        idle_window_ms: None,
    }
}

/// An idle run line with the given CPU time over a 1000 ms window.
fn idle_run(process_cpu_ms: Option<f64>) -> BenchRun {
    BenchRun {
        scenario: "idle".to_owned(),
        frames_painted: 0,
        process_cpu_ms,
        settle_ms: Some(1000),
        idle_window_ms: Some(1000.0),
        ..startup_run()
    }
}

/// Wraps a run line into a sample measured `spawn_to_first_frame_ms` after the spawn.
fn sample(spawn_to_first_frame_ms: f64, run: BenchRun) -> Sample {
    Sample {
        spawn_to_first_frame_ms: Some(spawn_to_first_frame_ms),
        run,
    }
}

/// A result configuration for `scenario` requested with `--renderer wgpu`, 10 MiB binary.
fn config(scenario: Scenario) -> ResultConfig {
    ResultConfig {
        scenario,
        renderer_requested: RendererChoice::Wgpu,
        binary_bytes: 10 * MIB,
        warmup: 1,
        runs: 2,
        duration_secs: (scenario == Scenario::Idle).then_some(10),
        env: EnvSnapshot::default(),
    }
}

/// A report created at 1700000000 s holding `results`.
fn report(results: Vec<BenchResult>) -> Report {
    Report::new(
        at(1_700_000_000),
        GitInfo {
            sha: "1a2b3c4d".to_owned(),
            dirty: false,
        },
        HostInfo {
            os: "windows".to_owned(),
            arch: "x86_64".to_owned(),
            wsl: false,
            logical_cpus: Some(16),
        },
        results,
    )
}

/// A startup result with two samples (spawn→frame 200 and 210 ms).
fn startup_result() -> BenchResult {
    BenchResult::new(
        config(Scenario::Startup),
        vec![sample(200.0, startup_run()), sample(210.0, startup_run())],
    )
}

/// Returns a fresh, empty folder `<CARGO_TARGET_TMPDIR>/<tag>-<pid>-<n>` (§17.9): created with
/// `create_dir`, which fails if the path already exists.
fn temp_dir(tag: &str) -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("{tag}-{}-{n}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    dir
}

/// Asserts that `keys` appear in this order in `text`.
fn assert_key_order(text: &str, keys: &[&str]) {
    let positions: Vec<usize> = keys
        .iter()
        .map(|key| {
            text.find(&format!("\"{key}\""))
                .unwrap_or_else(|| panic!("key {key} missing"))
        })
        .collect();
    assert!(
        positions.windows(2).all(|pair| pair[0] < pair[1]),
        "keys out of order: {keys:?} at {positions:?}"
    );
}

#[test]
fn report_file_name_uses_compact_utc_and_sha() {
    assert_eq!(
        report_file_name(at(1_700_000_000), "1a2b3c4d"),
        "20231114T221320Z-1a2b3c4d.json"
    );
}

/// §19.5 replaced the header: `requested`, `renderer` and `fallbacks` columns.
#[test]
fn markdown_header_matches_the_spec_exactly() {
    let spec = "| scenario | requested | renderer | fallbacks | backend | window | spawn→frame p50/p95 ms | startup p50/p95 ms | idle CPU p50/p95 % | frames p50/max | peak RSS p50 MiB | binary MiB |";
    assert_eq!(MARKDOWN_HEADER, spec);
    let table = report(vec![startup_result()]).markdown_table();
    assert_eq!(table.lines().next(), Some(spec));
}

/// §19.5: 12 columns; the requested choice, the used renderer and the fallback count follow
/// the scenario.
#[test]
fn markdown_row_formats_units_and_missing_values() {
    let table = report(vec![startup_result()]).markdown_table();
    let lines: Vec<&str> = table.lines().collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(
        lines[1],
        "|---|---|---|---|---|---|---|---|---|---|---|---|"
    );
    assert_eq!(
        lines[2],
        "| startup | wgpu | wgpu | 0 | dx12 | win32 | 205.0/210.0 | 150.0/150.0 | n/a | 1/1 | 100.0 | 10.0 |"
    );
}

#[test]
fn synthetic_report_has_schema_and_null_idle_cpu_for_startup() {
    let value: Value =
        serde_json::from_str(&report(vec![startup_result()]).to_json().unwrap()).unwrap();
    assert_eq!(value["schema"], "va.bench.report/2");
    assert_eq!(value["created_utc"], "2023-11-14T22:13:20Z");
    assert_eq!(value["profile"], "release");
    assert_eq!(value["git"]["sha"], "1a2b3c4d");
    assert_eq!(value["host"]["logical_cpus"], 16);
    let result = &value["results"][0];
    assert_eq!(result["scenario"], "startup");
    assert_eq!(result["renderer_requested"], "wgpu");
    assert_eq!(result["renderer"], "wgpu");
    assert_eq!(result["fallbacks"], 0);
    assert_eq!(result["binary"], "shipped");
    assert!(result["duration_secs"].is_null());
    assert!(result["summary"]["idle_cpu_percent"].is_null());
    assert_eq!(
        result["summary"]["spawn_to_first_frame_ms"]["median"],
        205.0
    );
    assert_eq!(result["samples"].as_array().unwrap().len(), 2);
    assert_eq!(result["samples"][0]["run"]["schema"], "va.bench.run/2");
    for name in [
        "WGPU_BACKEND",
        "WGPU_ADAPTER_NAME",
        "WGPU_POWER_PREF",
        "WAYLAND_DISPLAY",
        "DISPLAY",
        "VA_RENDERER",
        "VA_TEST_FAIL_RENDERER",
        "VA_WAYLAND",
    ] {
        assert!(result["env"][name].is_null(), "{name}");
    }
}

#[test]
fn report_keys_follow_the_schema_order() {
    let text = report(vec![startup_result()]).to_json().unwrap();
    assert_key_order(
        &text,
        &["schema", "created_utc", "git", "host", "profile", "results"],
    );
    assert_key_order(
        &text,
        &[
            "scenario",
            "renderer_requested",
            "renderer",
            "fallbacks",
            "binary",
            "binary_bytes",
            "warmup",
            "runs",
            "duration_secs",
            "env",
            "backend",
            "adapter",
            "driver",
            "window_system",
            "summary",
            "samples",
        ],
    );
    assert_key_order(
        &text,
        &[
            "spawn_to_first_frame_ms",
            "startup_ms",
            "process_cpu_ms",
            "peak_rss_bytes",
            "peak_private_bytes",
            "frames_painted",
            "idle_cpu_percent",
        ],
    );
}

#[test]
fn idle_result_computes_idle_cpu_percent() {
    let result = BenchResult::new(
        config(Scenario::Idle),
        vec![
            sample(200.0, idle_run(Some(10.0))),
            sample(200.0, idle_run(Some(20.0))),
        ],
    );
    let idle = result.summary.idle_cpu_percent.unwrap();
    assert_eq!(idle.median, 1.5);
    assert_eq!(idle.max, 2.0);
    assert_eq!(result.duration_secs, Some(10));
}

#[test]
fn summary_metric_is_null_when_any_sample_lacks_it() {
    let mut linux_like = startup_run();
    linux_like.peak_private_bytes = None;
    let result = BenchResult::new(
        config(Scenario::Startup),
        vec![sample(200.0, startup_run()), sample(200.0, linux_like)],
    );
    assert_eq!(result.summary.peak_private_bytes, None);
    assert!(result.summary.peak_rss_bytes.is_some());

    let idle = BenchResult::new(
        config(Scenario::Idle),
        vec![
            sample(200.0, idle_run(Some(10.0))),
            sample(200.0, idle_run(None)),
        ],
    );
    assert_eq!(idle.summary.process_cpu_ms, None);
    assert_eq!(idle.summary.idle_cpu_percent, None);
}

#[test]
fn differing_descriptions_become_mixed() {
    let mut other = startup_run();
    other.backend = "vulkan".to_owned();
    other.driver = None;
    let result = BenchResult::new(
        config(Scenario::Startup),
        vec![sample(200.0, startup_run()), sample(200.0, other)],
    );
    assert_eq!(result.backend, MIXED);
    assert_eq!(result.driver.as_deref(), Some(MIXED));
    assert_eq!(result.adapter.as_deref(), Some("Test Adapter"));
    assert_eq!(result.window_system, "win32");

    let mut no_adapter = startup_run();
    no_adapter.adapter = None;
    let all_null = BenchResult::new(
        config(Scenario::Startup),
        vec![sample(200.0, no_adapter.clone()), sample(200.0, no_adapter)],
    );
    assert_eq!(all_null.adapter, None);
}

/// §19.5 replaced the §4.6 example with the v2 fallback example.
#[test]
fn spec_run_line_round_trips_byte_for_byte() {
    let run: BenchRun = serde_json::from_str(SPEC_RUN_LINE).unwrap();
    assert_eq!(run.schema, RUN_SCHEMA);
    assert_eq!(run.scenario, "startup");
    assert_eq!(run.renderer_requested, "auto");
    assert_eq!(run.renderer, "wgpu");
    assert_eq!(run.fallback_from.as_deref(), Some("glow"));
    assert_eq!(
        run.fallback_reason.as_deref(),
        Some("injected renderer failure (VA_TEST_FAIL_RENDERER)")
    );
    assert_eq!(run.injected_failure.as_deref(), Some("glow"));
    assert_eq!(run.peak_private_bytes, Some(120_586_240));
    assert_eq!(serde_json::to_string(&run).unwrap(), SPEC_RUN_LINE);
}

/// §19.5 added `VA_RENDERER`, `VA_TEST_FAIL_RENDERER` and `VA_WAYLAND` to the five variables.
#[test]
fn env_snapshot_reads_the_eight_variables() {
    let snapshot =
        EnvSnapshot::from_lookup(|name| (name == "WGPU_BACKEND").then(|| "gl".to_owned()));
    assert_eq!(snapshot.wgpu_backend.as_deref(), Some("gl"));
    assert_eq!(snapshot.display, None);
    let value = serde_json::to_value(&snapshot).unwrap();
    assert_eq!(value.as_object().unwrap().len(), 8);
    assert_eq!(value["WGPU_BACKEND"], "gl");

    let renderer_vars = EnvSnapshot::from_lookup(|name| match name {
        "VA_RENDERER" => Some("wgpu".to_owned()),
        "VA_TEST_FAIL_RENDERER" => Some("glow,wgpu".to_owned()),
        "VA_WAYLAND" => Some("1".to_owned()),
        _ => None,
    });
    assert_eq!(renderer_vars.va_renderer.as_deref(), Some("wgpu"));
    assert_eq!(
        renderer_vars.va_test_fail_renderer.as_deref(),
        Some("glow,wgpu")
    );
    assert_eq!(renderer_vars.va_wayland.as_deref(), Some("1"));
    let value = serde_json::to_value(&renderer_vars).unwrap();
    assert_eq!(value["VA_RENDERER"], "wgpu");
    assert_eq!(value["VA_TEST_FAIL_RENDERER"], "glow,wgpu");
    assert_eq!(value["VA_WAYLAND"], "1");
    assert!(value["WGPU_BACKEND"].is_null());
}

#[test]
fn env_snapshot_asks_for_each_variable_once_in_report_order() {
    let mut asked = Vec::new();
    let _ = EnvSnapshot::from_lookup(|name| {
        asked.push(name.to_owned());
        None
    });
    assert_eq!(asked, ENV_VARS);
    assert_eq!(
        ENV_VARS,
        [
            "WGPU_BACKEND",
            "WGPU_ADAPTER_NAME",
            "WGPU_POWER_PREF",
            "WAYLAND_DISPLAY",
            "DISPLAY",
            "VA_RENDERER",
            "VA_TEST_FAIL_RENDERER",
            "VA_WAYLAND",
        ]
    );
}

#[test]
fn write_report_never_overwrites_and_adds_a_suffix() {
    let dir = temp_dir("report-suffix");
    let created = at(1_700_000_000);
    let report = report(vec![startup_result()]);
    let first = write_report(&dir, created, &report).unwrap();
    let second = write_report(&dir, created, &report).unwrap();
    let third = write_report(&dir, created, &report).unwrap();
    assert_eq!(first, dir.join("20231114T221320Z-1a2b3c4d.json"));
    assert_eq!(second, dir.join("20231114T221320Z-1a2b3c4d-2.json"));
    assert_eq!(third, dir.join("20231114T221320Z-1a2b3c4d-3.json"));
    let written: Value = serde_json::from_str(&std::fs::read_to_string(&first).unwrap()).unwrap();
    assert_eq!(written["schema"], "va.bench.report/2");
    std::fs::remove_dir_all(&dir).unwrap();
}

// ---------------------------------------------------------------------------------------------
// QA edge cases (audit round 1): exact key sets and order of every object of the report (a
// position search alone would miss an extra or duplicated key), markdown rows for idle and
// missing metrics, unicode paths with spaces, write failures and malformed run lines.
// ---------------------------------------------------------------------------------------------

/// The 22 keys of a `va.bench.run/2` line, in their normative order (§19.5).
const RUN_KEYS: [&str; 22] = [
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

/// Serde visitor returning the keys of a JSON object in document order.
struct KeyOrder;

impl<'de> Visitor<'de> for KeyOrder {
    type Value = Vec<String>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON object")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut keys = Vec::new();
        while let Some(key) = map.next_key::<String>()? {
            map.next_value::<IgnoredAny>()?;
            keys.push(key);
        }
        Ok(keys)
    }
}

/// Returns the top-level keys of the JSON object `text`, in document order.
fn keys_in_order(text: &str) -> Vec<String> {
    serde_json::Deserializer::from_str(text)
        .deserialize_map(KeyOrder)
        .unwrap()
}

/// Serializes `value` and returns its top-level keys in document order.
fn keys_of<T: serde::Serialize>(value: &T) -> Vec<String> {
    keys_in_order(&serde_json::to_string(value).unwrap())
}

/// The keys of a `va.bench.report/2` result object, in order: §6.2 with `renderer` replaced by
/// `renderer_requested` + `renderer`, then `fallbacks` and `binary` (§19.5).
const RESULT_KEYS: [&str; 16] = [
    "scenario",
    "renderer_requested",
    "renderer",
    "fallbacks",
    "binary",
    "binary_bytes",
    "warmup",
    "runs",
    "duration_secs",
    "env",
    "backend",
    "adapter",
    "driver",
    "window_system",
    "summary",
    "samples",
];

#[test]
fn every_report_object_has_exactly_the_schema_keys_in_order() {
    let idle = BenchResult::new(
        config(Scenario::Idle),
        vec![sample(200.0, idle_run(Some(10.0)))],
    );
    let report = report(vec![startup_result(), idle]);
    assert_eq!(
        keys_in_order(&report.to_json().unwrap()),
        ["schema", "created_utc", "git", "host", "profile", "results"]
    );
    assert_eq!(keys_of(&report.git), ["sha", "dirty"]);
    assert_eq!(keys_of(&report.host), ["os", "arch", "wsl", "logical_cpus"]);
    for result in &report.results {
        assert_eq!(keys_of(result), RESULT_KEYS);
        assert_eq!(keys_of(&result.env), ENV_VARS);
        assert_eq!(
            keys_of(&result.summary),
            [
                "spawn_to_first_frame_ms",
                "startup_ms",
                "process_cpu_ms",
                "peak_rss_bytes",
                "peak_private_bytes",
                "frames_painted",
                "idle_cpu_percent",
            ]
        );
        let startup: Summary = result.summary.startup_ms.unwrap();
        assert_eq!(keys_of(&startup), ["median", "p95", "min", "max"]);
        for sample in &result.samples {
            assert_eq!(keys_of(sample), ["spawn_to_first_frame_ms", "run"]);
            assert_eq!(keys_of(&sample.run), RUN_KEYS);
        }
    }
}

/// §17.8 changed the spawn→frame cell of idle rows: it is `n/a` now.
#[test]
fn idle_markdown_row_uses_two_decimals_for_percentages() {
    let idle = BenchResult::new(
        config(Scenario::Idle),
        vec![
            sample(200.0, idle_run(Some(10.0))),
            sample(220.0, idle_run(Some(20.0))),
        ],
    );
    assert_eq!(
        idle.markdown_row(),
        "| idle | wgpu | wgpu | 0 | dx12 | win32 | n/a | 150.0/150.0 | 1.50/2.00 | 0/0 | 100.0 | 10.0 |"
    );
}

/// §17.8 changed the spawn→frame cell of idle rows: it is `n/a` now.
#[test]
fn markdown_row_shows_n_a_for_every_missing_metric() {
    let mut linux_like = idle_run(None);
    linux_like.peak_rss_bytes = None;
    linux_like.peak_private_bytes = None;
    let result = BenchResult::new(config(Scenario::Idle), vec![sample(200.0, linux_like)]);
    assert_eq!(
        result.markdown_row(),
        "| idle | wgpu | wgpu | 0 | dx12 | win32 | n/a | 150.0/150.0 | n/a | 0/0 | n/a | 10.0 |"
    );
}

#[test]
fn a_missing_adapter_in_one_sample_makes_the_adapter_mixed() {
    let mut no_adapter = startup_run();
    no_adapter.adapter = None;
    let result = BenchResult::new(
        config(Scenario::Startup),
        vec![sample(200.0, startup_run()), sample(200.0, no_adapter)],
    );
    assert_eq!(result.adapter.as_deref(), Some(MIXED));
}

#[test]
fn report_file_name_without_git_and_around_the_epoch() {
    assert_eq!(
        report_file_name(at(0), "nogit"),
        "19700101T000000Z-nogit.json"
    );
    assert_eq!(
        report_file_name(UNIX_EPOCH - Duration::from_secs(1), "nogit"),
        "19691231T235959Z-nogit.json"
    );
}

#[test]
fn created_utc_and_file_name_describe_the_same_instant() {
    let instant = at(1_700_000_000) + Duration::from_millis(999);
    let report = Report::new(
        instant,
        GitInfo {
            sha: "nogit".to_owned(),
            dirty: true,
        },
        HostInfo {
            os: "linux".to_owned(),
            arch: "aarch64".to_owned(),
            wsl: true,
            logical_cpus: None,
        },
        vec![],
    );
    assert_eq!(report.created_utc, "2023-11-14T22:13:20Z");
    assert_eq!(
        report_file_name(instant, &report.git.sha),
        "20231114T221320Z-nogit.json"
    );
    let value: Value = serde_json::from_str(&report.to_json().unwrap()).unwrap();
    assert!(value["host"]["logical_cpus"].is_null());
    assert_eq!(value["results"], Value::Array(vec![]));
}

#[test]
fn write_report_creates_missing_folders_under_a_unicode_path_with_spaces() {
    let base = temp_dir("report-unicode");
    let dir = base
        .join("dossier é 文件 ✓ with spaces")
        .join("bench")
        .join("results");
    let path = write_report(&dir, at(1_700_000_000), &report(vec![startup_result()])).unwrap();
    assert_eq!(path, dir.join("20231114T221320Z-1a2b3c4d.json"));
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.ends_with("}\n"), "the file must end with one newline");
    let written: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        written["results"][0]["samples"][0]["run"]["adapter"],
        "Test Adapter"
    );
    std::fs::remove_dir_all(&base).unwrap();
}

#[test]
fn write_report_into_a_path_that_is_a_file_fails_with_context() {
    let base = temp_dir("report-blocked");
    let blocker = base.join("results");
    std::fs::write(&blocker, "not a folder").unwrap();
    let error = write_report(&blocker, at(0), &report(vec![startup_result()])).unwrap_err();
    let message = format!("{error:#}");
    assert!(message.contains("results"), "{message}");
    assert_eq!(std::fs::read_to_string(&blocker).unwrap(), "not a folder");
    std::fs::remove_dir_all(&base).unwrap();
}

#[test]
fn run_lines_with_wrong_types_are_rejected() {
    for (key, bad) in [
        ("frames_painted", "-1"),
        ("frames_painted", "1.5"),
        ("startup_ms", "null"),
        ("startup_ms", "\"fast\""),
        ("wsl", "0"),
        ("peak_rss_bytes", "-5"),
        ("schema", "1"),
        ("scenario", "null"),
        ("renderer_requested", "null"),
        ("renderer_requested", "1"),
        ("renderer", "null"),
        ("fallback_from", "1"),
        ("fallback_reason", "true"),
        ("injected_failure", "[\"glow\"]"),
    ] {
        let mut value: Value = serde_json::from_str(SPEC_RUN_LINE).unwrap();
        value[key] = serde_json::from_str(bad).unwrap();
        assert!(
            serde_json::from_value::<BenchRun>(value).is_err(),
            "{key} = {bad} must be rejected"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Fix wave (§17.8): idle results carry no spawn→frame time.
// ---------------------------------------------------------------------------------------------

#[test]
fn idle_results_have_null_spawn_to_first_frame_in_samples_and_summary() {
    let idle = BenchResult::new(
        config(Scenario::Idle),
        vec![
            sample(200.0, idle_run(Some(10.0))),
            Sample {
                spawn_to_first_frame_ms: None,
                run: idle_run(Some(20.0)),
            },
        ],
    );
    assert_eq!(idle.summary.spawn_to_first_frame_ms, None);
    assert!(
        idle.samples
            .iter()
            .all(|s| s.spawn_to_first_frame_ms.is_none())
    );
    assert!(idle.summary.startup_ms.is_some(), "startup_ms stays valid");
    let value: Value = serde_json::from_str(&report(vec![idle]).to_json().unwrap()).unwrap();
    let result = &value["results"][0];
    assert!(result["summary"]["spawn_to_first_frame_ms"].is_null());
    for sample in result["samples"].as_array().unwrap() {
        assert!(sample["spawn_to_first_frame_ms"].is_null(), "{sample}");
        assert!(sample["run"]["startup_ms"].is_number(), "{sample}");
    }
}

#[test]
fn startup_results_keep_numeric_spawn_to_first_frame() {
    let value: Value =
        serde_json::from_str(&report(vec![startup_result()]).to_json().unwrap()).unwrap();
    let result = &value["results"][0];
    assert_eq!(result["samples"][0]["spawn_to_first_frame_ms"], 200.0);
    assert_eq!(result["summary"]["spawn_to_first_frame_ms"]["p95"], 210.0);
}

#[test]
fn a_startup_sample_without_spawn_time_makes_the_summary_null() {
    let result = BenchResult::new(
        config(Scenario::Startup),
        vec![
            sample(200.0, startup_run()),
            Sample {
                spawn_to_first_frame_ms: None,
                run: startup_run(),
            },
        ],
    );
    assert_eq!(result.summary.spawn_to_first_frame_ms, None);
    assert!(
        result
            .markdown_row()
            .starts_with("| startup | wgpu | wgpu | 0 | dx12 | win32 | n/a |")
    );
}

// ---------------------------------------------------------------------------------------------
// Renderer decision (§19.5): report v2 on the shipped binary.
// ---------------------------------------------------------------------------------------------

/// A startup run line requested with `auto`, running `renderer` after a fallback from
/// `fallback_from` when it is `Some`.
fn auto_run(renderer: &str, fallback_from: Option<&str>) -> BenchRun {
    BenchRun {
        renderer_requested: "auto".to_owned(),
        renderer: renderer.to_owned(),
        fallback_from: fallback_from.map(str::to_owned),
        fallback_reason: fallback_from
            .map(|_| "injected renderer failure (VA_TEST_FAIL_RENDERER)".to_owned()),
        injected_failure: fallback_from.map(str::to_owned),
        backend: if renderer == "glow" { "gl" } else { "dx12" }.to_owned(),
        ..startup_run()
    }
}

/// A startup result requested with `auto` holding `runs`.
fn auto_result(runs: Vec<BenchRun>) -> BenchResult {
    BenchResult::new(
        ResultConfig {
            renderer_requested: RendererChoice::Auto,
            ..config(Scenario::Startup)
        },
        runs.into_iter().map(|run| sample(200.0, run)).collect(),
    )
}

#[test]
fn report_v2_schema_ids_and_shipped_binary() {
    assert_eq!(REPORT_SCHEMA, "va.bench.report/2");
    assert_eq!(RUN_SCHEMA, "va.bench.run/2");
    assert_eq!(SHIPPED_BINARY, "shipped");
    assert_eq!(startup_result().binary, "shipped");
}

#[test]
fn auto_result_takes_the_used_renderer_from_the_samples() {
    let result = auto_result(vec![auto_run("glow", None), auto_run("glow", None)]);
    assert_eq!(result.renderer_requested, RendererChoice::Auto);
    assert_eq!(result.renderer, "glow");
    assert_eq!(result.fallbacks, 0);
    assert_eq!(result.backend, "gl");
    assert_eq!(
        result.markdown_row(),
        "| startup | auto | glow | 0 | gl | win32 | 200.0/200.0 | 150.0/150.0 | n/a | 1/1 | 100.0 | 10.0 |"
    );
}

#[test]
fn fallbacks_count_the_samples_with_a_fallback_from() {
    let result = auto_result(vec![
        auto_run("wgpu", Some("glow")),
        auto_run("wgpu", Some("glow")),
        auto_run("wgpu", Some("glow")),
    ]);
    assert_eq!(result.renderer, "wgpu");
    assert_eq!(result.fallbacks, 3);
    let value: Value = serde_json::from_str(&report(vec![result]).to_json().unwrap()).unwrap();
    let result = &value["results"][0];
    assert_eq!(result["renderer_requested"], "auto");
    assert_eq!(result["renderer"], "wgpu");
    assert_eq!(result["fallbacks"], 3);
    assert!(result["fallbacks"].is_u64(), "an integer, not a float");
    assert_eq!(result["samples"][0]["run"]["fallback_from"], "glow");
    assert_eq!(result["samples"][0]["run"]["injected_failure"], "glow");
}

#[test]
fn samples_that_ran_different_renderers_make_the_renderer_mixed() {
    let result = auto_result(vec![
        auto_run("glow", None),
        auto_run("wgpu", Some("glow")),
        auto_run("glow", None),
    ]);
    assert_eq!(result.renderer, MIXED);
    assert_eq!(result.fallbacks, 1);
    assert_eq!(result.backend, MIXED);
    assert!(
        result
            .markdown_row()
            .starts_with("| startup | auto | mixed | 1 | mixed | win32 |"),
        "{}",
        result.markdown_row()
    );
}

#[test]
fn a_result_without_samples_has_an_unknown_renderer_and_no_fallback() {
    let result = BenchResult::new(config(Scenario::Startup), vec![]);
    assert_eq!(result.renderer, UNKNOWN);
    assert_eq!(result.fallbacks, 0);
    assert_eq!(result.renderer_requested, RendererChoice::Wgpu);
}

#[test]
fn markdown_separator_has_one_cell_per_header_column() {
    let columns = MARKDOWN_HEADER.matches(" | ").count() + 1;
    assert_eq!(columns, 12);
    assert_eq!(MARKDOWN_SEPARATOR.matches("---").count(), columns);
    for line in report(vec![startup_result()]).markdown_table().lines() {
        assert_eq!(line.matches('|').count(), columns + 1, "{line}");
    }
}

#[test]
fn every_renderer_choice_serializes_as_its_cli_value() {
    for (choice, text) in [
        (RendererChoice::Auto, "auto"),
        (RendererChoice::Glow, "glow"),
        (RendererChoice::Wgpu, "wgpu"),
    ] {
        let result = BenchResult::new(
            ResultConfig {
                renderer_requested: choice,
                ..config(Scenario::Startup)
            },
            vec![],
        );
        let value = serde_json::to_value(&result).unwrap();
        assert_eq!(value["renderer_requested"], text);
        assert!(
            result
                .markdown_row()
                .starts_with(&format!("| startup | {text} | unknown | 0 |"))
        );
    }
}
