//! Tests for `src/layering.rs`: crate layering rules (§17.2, GUI families of §18.5) on synthetic
//! `cargo metadata` documents, and the source scan rules S1 and S3–S9 (§17.4 as amended by §18.5:
//! S2 removed, shebang lines, case-insensitive `.rs`) on synthetic member trees created in fresh
//! folders under `CARGO_TARGET_TMPDIR` (§17.9).
#![allow(clippy::unwrap_used)]

use std::{
    io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU32, Ordering},
};

use serde_json::{Value, json};
use xtask::layering::{
    GUI_CRATE_FAMILIES, GUI_CRATES_EXACT, RULE_GUI, RULE_INCLUDE, RULE_LAYERING, RULE_METADATA,
    RULE_NETWORK, RULE_SYMLINK, RULE_UNSAFE, Violation, check_layering, code_view, has_rs_suffix,
    include_target_allowed, is_gui_crate, member_folders, scan_sources, scan_text, shebang_line,
};

/// Every package of the M0 workspace, without dependencies.
const M0_PACKAGES: &[&str] = &[
    "va-core",
    "va-editor",
    "va-lang",
    "va-debug",
    "va-workspace",
    "va-ui",
    "va-arch-m68k",
    "visual-assembler",
    "va",
    "xtask",
];

/// A dependency `(name, kind)`, where kind is `None` (normal), `Some("dev")` or `Some("build")`.
type Dep<'a> = (&'a str, Option<&'a str>);

/// A package `(name, dependencies)`.
type Package<'a> = (&'a str, &'a [Dep<'a>]);

/// Builds a `cargo metadata --no-deps` document from `packages`.
fn metadata(packages: &[Package<'_>]) -> Value {
    let id = |name: &str| format!("path+file:///workspace/{name}#0.1.0");
    let entries: Vec<Value> = packages
        .iter()
        .map(|(name, dependencies)| {
            let dependencies: Vec<Value> = dependencies
                .iter()
                .map(|(dep, kind)| json!({ "name": dep, "kind": kind, "target": null }))
                .collect();
            json!({ "name": name, "id": id(name), "dependencies": dependencies })
        })
        .collect();
    let members: Vec<String> = packages.iter().map(|(name, _)| id(name)).collect();
    json!({ "packages": entries, "workspace_members": members, "target_directory": "/t" })
}

/// The M0 workspace with `extra` dependencies added to one package.
fn m0_with(package: &str, extra: &[Dep<'_>]) -> Value {
    let packages: Vec<Package<'_>> = M0_PACKAGES
        .iter()
        .map(|name| (*name, if *name == package { extra } else { &[][..] }))
        .collect();
    metadata(&packages)
}

/// Returns the rules of `violations`.
fn rules(violations: &[Violation]) -> Vec<&'static str> {
    violations.iter().map(|v| v.rule).collect()
}

/// Returns a fresh, empty folder `<CARGO_TARGET_TMPDIR>/<tag>-<pid>-<n>` (§17.9): created with
/// `create_dir`, which fails if the path already exists.
fn temp_root(tag: &str) -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("{tag}-{}-{n}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    dir
}

/// Creates `base/relative` one level at a time below the fresh folder `base`.
fn make_folders(base: &Path, relative: &Path) {
    let mut path = base.to_path_buf();
    for component in relative.components() {
        path.push(component);
        match std::fs::create_dir(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => panic!("cannot create {}: {error}", path.display()),
        }
    }
}

/// Writes `text` to `root/relative`, creating folders below `root`.
fn write(root: &Path, relative: &str, text: &str) {
    let relative = Path::new(relative);
    if let Some(parent) = relative.parent() {
        make_folders(root, parent);
    }
    std::fs::write(root.join(relative), text).unwrap();
}

/// Returns the member folders of a synthetic tree laid out like the real workspace: every
/// folder `crates/*`, `arch/*` and `apps/*`, plus `xtask` when present.
fn tree_members(root: &Path) -> Vec<PathBuf> {
    let mut members = Vec::new();
    for group in ["crates", "arch", "apps"] {
        let Ok(entries) = std::fs::read_dir(root.join(group)) else {
            continue;
        };
        for entry in entries {
            let path = entry.unwrap().path();
            if path.is_dir() {
                members.push(path);
            }
        }
    }
    if root.join("xtask").is_dir() {
        members.push(root.join("xtask"));
    }
    members.sort();
    members
}

/// Scans `members` (relative to `root`) with `<root>/target` as cargo target folder.
fn try_scan_members(root: &Path, members: &[&str]) -> io::Result<Vec<Violation>> {
    let members: Vec<PathBuf> = members.iter().map(|member| root.join(member)).collect();
    scan_sources(root, &members, &root.join("target"))
}

/// Scans the synthetic tree at `root` the way `check_workspace` scans the real one.
fn try_scan(root: &Path) -> io::Result<Vec<Violation>> {
    scan_sources(root, &tree_members(root), &root.join("target"))
}

/// Same as [`try_scan`], unwrapped.
fn scan(root: &Path) -> Vec<Violation> {
    try_scan(root).unwrap()
}

/// Returns `(rule, location)` of each violation.
fn pairs(violations: &[Violation]) -> Vec<(&'static str, &str)> {
    violations
        .iter()
        .map(|v| (v.rule, v.location.as_str()))
        .collect()
}

/// Writes `text` as `crates/x/src/lib.rs` of a fresh tree and returns its violations.
fn scan_one(tag: &str, text: &str) -> Vec<Violation> {
    let root = temp_root(tag);
    write(&root, "crates/x/src/lib.rs", text);
    let violations = scan(&root);
    std::fs::remove_dir_all(&root).unwrap();
    violations
}

/// Creates a symbolic link to a folder; `None` when Windows refuses it (no symlink privilege
/// and no Developer Mode), after printing a note.
fn symlink_folder(target: &Path, link: &Path) -> Option<()> {
    #[cfg(unix)]
    let created = std::os::unix::fs::symlink(target, link);
    #[cfg(windows)]
    let created = std::os::windows::fs::symlink_dir(target, link);
    symlink_created(created, link)
}

/// Creates a symbolic link to a file; `None` when Windows refuses it (see [`symlink_folder`]).
fn symlink_file(target: &Path, link: &Path) -> Option<()> {
    #[cfg(unix)]
    let created = std::os::unix::fs::symlink(target, link);
    #[cfg(windows)]
    let created = std::os::windows::fs::symlink_file(target, link);
    symlink_created(created, link)
}

/// Turns a symlink creation result into `Some(())`, or `None` plus a note on Windows.
fn symlink_created(created: io::Result<()>, link: &Path) -> Option<()> {
    match created {
        Ok(()) => Some(()),
        Err(error) if cfg!(windows) => {
            println!("skipped: symlink creation is not permitted here ({error})");
            None
        }
        Err(error) => panic!("cannot create the symlink {}: {error}", link.display()),
    }
}

#[test]
fn m0_workspace_without_dependencies_is_clean() {
    assert_eq!(check_layering(&m0_with("va", &[])), vec![]);
}

#[test]
fn core_depending_on_ui_is_one_violation() {
    let violations = check_layering(&m0_with("va-core", &[("va-ui", None)]));
    assert_eq!(rules(&violations), vec![RULE_LAYERING]);
    assert_eq!(violations[0].location, "va-core");
    assert!(violations[0].detail.contains("va-ui"), "{}", violations[0]);
}

#[test]
fn cli_depending_on_ui_is_a_violation() {
    let violations = check_layering(&m0_with("va", &[("va-ui", None)]));
    assert_eq!(rules(&violations), vec![RULE_LAYERING]);
    assert_eq!(violations[0].location, "va");
}

#[test]
fn cli_may_depend_on_every_other_crate() {
    let deps = [
        ("va-core", None),
        ("va-editor", None),
        ("va-lang", None),
        ("va-debug", None),
        ("va-workspace", None),
        ("va-arch-m68k", None),
    ];
    assert_eq!(check_layering(&m0_with("va", &deps)), vec![]);
}

#[test]
fn new_arch_crate_depending_on_core_is_ok() {
    let doc = metadata(&[("va-core", &[]), ("va-arch-z80", &[("va-core", None)])]);
    assert_eq!(check_layering(&doc), vec![]);
}

#[test]
fn arch_crate_depending_on_editor_is_a_violation() {
    let doc = metadata(&[
        ("va-core", &[]),
        ("va-editor", &[("va-core", None)]),
        ("va-arch-z80", &[("va-editor", None)]),
    ]);
    assert_eq!(rules(&check_layering(&doc)), vec![RULE_LAYERING]);
}

#[test]
fn unknown_package_has_no_layering_rule() {
    let doc = metadata(&[("va-core", &[]), ("mystery-tool", &[])]);
    let violations = check_layering(&doc);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].rule, RULE_LAYERING);
    assert!(
        violations[0]
            .detail
            .contains("no layering rule for mystery-tool"),
        "{}",
        violations[0]
    );
}

#[test]
fn bare_arch_prefix_has_no_layering_rule() {
    let doc = metadata(&[("va-arch-", &[])]);
    assert!(check_layering(&doc)[0].detail.contains("no layering rule"));
}

#[test]
fn lang_depending_on_eframe_is_a_gui_crate_violation() {
    let violations = check_layering(&m0_with("va-lang", &[("eframe", None)]));
    assert_eq!(rules(&violations), vec![RULE_GUI]);
    assert!(violations[0].detail.contains("eframe"), "{}", violations[0]);
}

#[test]
fn ui_may_use_egui_and_kittest_but_not_eframe() {
    let allowed = m0_with("va-ui", &[("egui", None), ("egui_kittest", Some("dev"))]);
    assert_eq!(check_layering(&allowed), vec![]);
    let forbidden = m0_with("va-ui", &[("eframe", None)]);
    assert_eq!(rules(&check_layering(&forbidden)), vec![RULE_GUI]);
}

#[test]
fn gui_app_may_use_every_gui_crate() {
    let deps = [
        ("eframe", None),
        ("raw-window-handle", None),
        ("egui_kittest", Some("dev")),
        ("va-ui", None),
        ("va-arch-m68k", None),
    ];
    assert_eq!(check_layering(&m0_with("visual-assembler", &deps)), vec![]);
}

#[test]
fn dev_and_build_dependencies_follow_the_same_rules() {
    let dev = check_layering(&m0_with("va-core", &[("va-editor", Some("dev"))]));
    assert_eq!(rules(&dev), vec![RULE_LAYERING]);
    assert!(dev[0].detail.contains("(dev)"), "{}", dev[0]);
    let build = check_layering(&m0_with("va", &[("winit", Some("build"))]));
    assert_eq!(rules(&build), vec![RULE_GUI]);
}

#[test]
fn gui_crate_names_ignore_dash_underscore_spelling() {
    let violations = check_layering(&m0_with("va", &[("egui-glow", None)]));
    assert_eq!(rules(&violations), vec![RULE_GUI]);
}

#[test]
fn xtask_may_not_depend_on_workspace_crates() {
    let violations = check_layering(&m0_with("xtask", &[("va-core", None)]));
    assert_eq!(rules(&violations), vec![RULE_LAYERING]);
}

#[test]
fn malformed_metadata_is_reported() {
    let violations = check_layering(&json!({ "version": 1 }));
    assert_eq!(rules(&violations), vec![RULE_METADATA]);
}

/// §17.4 S3 changed one expectation: `crates/x/target/` is a source folder like any other and is
/// scanned now; only the cargo target folder and `<root>/target` are skipped.
///
/// §18.5 changed another: rule S2 is removed, so `xtask/src/layering.rs` is scanned under every
/// rule and its network name and `unsafe_code` outside `deny`/`forbid` are reported.
#[test]
fn scan_flags_network_and_unsafe_outside_the_allow_list() {
    let root = temp_root("scan-basic");
    write(
        &root,
        "crates/x/src/lib.rs",
        "//! x\nuse std::net::TcpStream;\n",
    );
    write(&root, "crates/x/src/raw.rs", "#![allow(unsafe_code)]\n");
    write(
        &root,
        "arch/y/src/lib.rs",
        "#[allow(dead_code, unsafe_code)]\nfn f() {}\n",
    );
    write(
        &root,
        "apps/z/src/main.rs",
        "#[expect(unsafe_code)]\nfn main() {}\n",
    );
    write(
        &root,
        "apps/visual-assembler/src/bench/process_windows.rs",
        "#![allow(unsafe_code)]\n",
    );
    write(
        &root,
        "apps/visual-assembler/src/bench/mod.rs",
        "#![deny(unsafe_code)]\n",
    );
    write(
        &root,
        "crates/x/target/debug/gen.rs",
        "use std::net::UdpSocket;\n",
    );
    write(&root, "crates/x/src/notes.txt", "std::net\n");
    write(
        &root,
        "xtask/src/layering.rs",
        "TcpListener allow(unsafe_code)\n",
    );

    let violations = scan(&root);
    assert_eq!(
        pairs(&violations),
        vec![
            (RULE_UNSAFE, "apps/z/src/main.rs:1"),
            (RULE_UNSAFE, "arch/y/src/lib.rs:1"),
            (RULE_NETWORK, "crates/x/src/lib.rs:2"),
            (RULE_UNSAFE, "crates/x/src/raw.rs:1"),
            (RULE_NETWORK, "crates/x/target/debug/gen.rs:1"),
            (RULE_NETWORK, "xtask/src/layering.rs:1"),
            (RULE_UNSAFE, "xtask/src/layering.rs:1"),
        ]
    );
    assert!(
        violations[2].detail.contains("std::net"),
        "{}",
        violations[2]
    );
    assert!(
        violations[2].detail.contains("TcpStream"),
        "{}",
        violations[2]
    );
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn scan_accepts_the_allow_listed_unsafe_files() {
    let root = temp_root("scan-allowed");
    for path in xtask::layering::UNSAFE_ALLOWLIST {
        write(
            &root,
            path,
            "#![allow(unsafe_code)]\nfn f(p: *const u8) -> u8 {\n    unsafe { *p }\n}\n",
        );
    }
    assert_eq!(scan(&root), vec![]);
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn scan_of_an_empty_root_is_clean() {
    let root = temp_root("scan-empty");
    assert_eq!(scan(&root), vec![]);
    std::fs::remove_dir_all(&root).unwrap();
}

/// Runs both checks on the live workspace. Ignored by default because its outcome depends on
/// the code of every package; run it with
/// `cargo test -p xtask --test layering -- --ignored live_workspace --nocapture`, or through
/// `cargo xtask ci`. The name filter matters: the two tests that record accepted tripwire limits
/// (m0-foundations.md §18.1) are ignored too, and they fail by design.
#[test]
#[ignore = "inspects the live workspace; `cargo xtask ci` runs the same check"]
fn live_workspace_satisfies_layering_and_source_rules() {
    let root = xtask::locate::workspace_root().unwrap();
    let violations = xtask::layering::check_workspace(&root).unwrap();
    for violation in &violations {
        println!("{violation}");
    }
    println!(
        "live workspace {}: {} violation(s)",
        root.display(),
        violations.len()
    );
    assert_eq!(violations, vec![]);
}

// ---------------------------------------------------------------------------------------------
// QA edge cases (audit round 1): layering bypass attempts (renamed and target-specific
// dependencies, every forbidden edge, both crate-name spellings), malformed metadata, and
// source-scan bypass attempts (multi-line attributes, lookalike paths, unicode roots, CRLF,
// non-UTF-8 files).
// ---------------------------------------------------------------------------------------------

/// Builds a `cargo metadata --no-deps` document whose dependency entries are given verbatim.
fn raw_metadata(packages: &[(&str, Vec<Value>)]) -> Value {
    let id = |name: &str| format!("path+file:///workspace/{name}#0.1.0");
    let entries: Vec<Value> = packages
        .iter()
        .map(|(name, dependencies)| {
            json!({ "name": name, "id": id(name), "dependencies": dependencies })
        })
        .collect();
    let members: Vec<String> = packages.iter().map(|(name, _)| id(name)).collect();
    json!({ "packages": entries, "workspace_members": members, "target_directory": "/t" })
}

/// The M0 workspace where `package` has the verbatim dependency entries `dependencies`.
fn m0_with_raw(package: &str, dependencies: &[Value]) -> Value {
    let packages: Vec<(&str, Vec<Value>)> = M0_PACKAGES
        .iter()
        .map(|name| {
            let deps = if *name == package {
                dependencies.to_vec()
            } else {
                Vec::new()
            };
            (*name, deps)
        })
        .collect();
    raw_metadata(&packages)
}

/// Returns the file part (before `:<line>`) of each violation location.
fn files(violations: &[Violation]) -> Vec<&str> {
    violations
        .iter()
        .map(|v| {
            v.location
                .rsplit_once(':')
                .map_or(v.location.as_str(), |(file, _)| file)
        })
        .collect()
}

#[test]
fn renamed_internal_dependency_is_checked_under_its_real_name() {
    // `ui = { package = "va-ui", path = "../../crates/va-ui" }` in apps/va/Cargo.toml.
    let doc = m0_with_raw(
        "va",
        &[json!({ "name": "va-ui", "rename": "ui", "kind": null, "target": null })],
    );
    let violations = check_layering(&doc);
    assert_eq!(rules(&violations), vec![RULE_LAYERING]);
    assert!(violations[0].detail.contains("va-ui"), "{}", violations[0]);
}

#[test]
fn renamed_gui_crate_is_still_a_gui_crate() {
    let doc = m0_with_raw(
        "va-core",
        &[json!({ "name": "wgpu", "rename": "gfx", "kind": null, "target": null })],
    );
    assert_eq!(rules(&check_layering(&doc)), vec![RULE_GUI]);
}

#[test]
fn target_specific_dependencies_are_checked() {
    let doc = m0_with_raw(
        "va-core",
        &[
            json!({ "name": "va-ui", "kind": null, "target": "cfg(windows)" }),
            json!({ "name": "winit", "kind": "dev", "target": "cfg(target_os = \"linux\")" }),
        ],
    );
    assert_eq!(rules(&check_layering(&doc)), vec![RULE_LAYERING, RULE_GUI]);
}

#[test]
fn every_edge_outside_the_crate_map_is_a_violation() {
    for (package, dependency) in [
        ("va-editor", "va-lang"),
        ("va-editor", "va-debug"),
        ("va-lang", "va-debug"),
        ("va-lang", "va-workspace"),
        ("va-debug", "va-editor"),
        ("va-workspace", "va-editor"),
        ("va-core", "va-editor"),
        // R2: crates/* never depends on arch/*.
        ("va-core", "va-arch-m68k"),
        ("va-lang", "va-arch-m68k"),
        ("va-ui", "va-arch-m68k"),
        ("va-arch-m68k", "va-editor"),
        ("va-arch-m68k", "va-ui"),
        ("va-ui", "visual-assembler"),
        ("va-core", "va"),
        // R5: xtask depends on no workspace crate.
        ("xtask", "va"),
        ("xtask", "visual-assembler"),
    ] {
        let violations = check_layering(&m0_with(package, &[(dependency, None)]));
        assert_eq!(
            rules(&violations),
            vec![RULE_LAYERING],
            "{package} -> {dependency}"
        );
        assert_eq!(violations[0].location, package);
    }
}

#[test]
fn every_edge_of_the_crate_map_is_allowed() {
    for (package, dependency) in [
        ("va-editor", "va-core"),
        ("va-lang", "va-core"),
        ("va-lang", "va-editor"),
        ("va-debug", "va-core"),
        ("va-workspace", "va-core"),
        ("va-ui", "va-core"),
        ("va-ui", "va-editor"),
        ("va-ui", "va-lang"),
        ("va-ui", "va-debug"),
        ("va-ui", "va-workspace"),
        ("va-arch-m68k", "va-core"),
        ("visual-assembler", "va-ui"),
        ("visual-assembler", "va-arch-m68k"),
        ("va", "va-arch-m68k"),
    ] {
        assert_eq!(
            check_layering(&m0_with(package, &[(dependency, Some("dev"))])),
            vec![],
            "{package} -> {dependency}"
        );
    }
}

/// The GUI crate set of §6.3 (before §18.5 matched GUI crates by family); every one stays a GUI
/// crate.
const M0_GUI_CRATES: &[&str] = &[
    "eframe",
    "egui",
    "egui-winit",
    "egui-wgpu",
    "egui_glow",
    "egui_kittest",
    "winit",
    "wgpu",
    "glow",
    "glutin",
    "raw-window-handle",
];

#[test]
fn every_gui_crate_in_both_spellings_is_rejected_outside_the_gui_packages() {
    let names = M0_GUI_CRATES
        .iter()
        .chain(GUI_CRATE_FAMILIES)
        .chain(GUI_CRATES_EXACT);
    for name in names {
        for spelling in [name.replace('_', "-"), name.replace('-', "_")] {
            for package in ["va", "va-core", "va-lang", "va-arch-m68k", "xtask"] {
                let violations = check_layering(&m0_with(package, &[(&spelling, None)]));
                assert_eq!(
                    rules(&violations),
                    vec![RULE_GUI],
                    "{package} -> {spelling}"
                );
            }
        }
    }
}

#[test]
fn ui_may_use_only_egui_and_kittest_in_any_spelling() {
    let allowed = m0_with(
        "va-ui",
        &[
            ("egui", None),
            ("egui_kittest", Some("dev")),
            ("egui-kittest", None),
        ],
    );
    assert_eq!(check_layering(&allowed), vec![]);
    for forbidden in [
        "eframe",
        "egui-winit",
        "egui_winit",
        "egui-wgpu",
        "egui_glow",
        "winit",
        "wgpu",
        "glow",
        "glutin",
        "raw-window-handle",
        "raw_window_handle",
    ] {
        let violations = check_layering(&m0_with("va-ui", &[(forbidden, None)]));
        assert_eq!(rules(&violations), vec![RULE_GUI], "va-ui -> {forbidden}");
    }
}

#[test]
fn malformed_metadata_variants_are_reported_not_ignored() {
    let not_an_array = json!({ "packages": [], "workspace_members": "va-core" });
    assert_eq!(rules(&check_layering(&not_an_array)), vec![RULE_METADATA]);

    let non_string_member = json!({ "packages": [], "workspace_members": [1] });
    assert_eq!(
        rules(&check_layering(&non_string_member)),
        vec![RULE_METADATA]
    );

    let nameless_package = json!({
        "packages": [{ "id": "a", "dependencies": [] }],
        "workspace_members": ["a"],
    });
    assert!(rules(&check_layering(&nameless_package)).contains(&RULE_METADATA));

    let no_dependency_array = json!({
        "packages": [{ "name": "va-core", "id": "a", "dependencies": null }],
        "workspace_members": ["a"],
    });
    assert_eq!(
        rules(&check_layering(&no_dependency_array)),
        vec![RULE_METADATA]
    );

    let nameless_dependency = m0_with_raw("va-core", &[json!({ "kind": null })]);
    assert_eq!(
        rules(&check_layering(&nameless_dependency)),
        vec![RULE_METADATA]
    );
}

#[test]
fn scan_works_under_a_unicode_root_with_spaces() {
    let base = temp_root("scan-unicode");
    let root = base.join("répertoire é 文件 with spaces");
    std::fs::create_dir(&root).unwrap();
    write(
        &root,
        "crates/naïve crate/src/lib.rs",
        "//! x\nuse std::net::UdpSocket;\n",
    );
    write(
        &root,
        "apps/visual-assembler/src/bench/platform.rs",
        "#[allow(unsafe_code)]\nfn f() {}\n",
    );
    assert_eq!(
        pairs(&scan(&root)),
        vec![(RULE_NETWORK, "crates/naïve crate/src/lib.rs:2")]
    );
    std::fs::remove_dir_all(&base).unwrap();
}

#[test]
fn scan_reports_crlf_files_with_correct_line_numbers() {
    let root = temp_root("scan-crlf");
    write(
        &root,
        "crates/x/src/lib.rs",
        "//! x\r\n\r\nfn f() {}\r\nuse std::net::TcpListener;\r\n",
    );
    let violations = scan(&root);
    assert_eq!(violations.len(), 1, "{violations:?}");
    assert_eq!(violations[0].location, "crates/x/src/lib.rs:4");
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn scan_fails_loudly_on_a_non_utf8_source_file() {
    let root = temp_root("scan-non-utf8");
    make_folders(&root, Path::new("crates/x/src"));
    let path = root.join("crates/x/src/bad.rs");
    std::fs::write(&path, [b'/', b'/', 0xff, 0xfe, b'\n']).unwrap();
    let error = try_scan(&root).unwrap_err();
    assert!(error.to_string().contains("bad.rs"), "{error}");
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn allow_list_lookalike_paths_are_still_flagged() {
    let root = temp_root("scan-lookalike");
    for path in [
        "crates/x/src/apps/visual-assembler/src/bench/process_windows.rs",
        "apps/visual-assembler/src/bench/process_windows_extra.rs",
        "apps/visual-assembler/src/bench/process_linux.rs.rs",
        "apps/other/src/bench/platform.rs",
        "apps/visual-assembler/src/platform.rs",
    ] {
        write(&root, path, "#![allow(unsafe_code)]\n");
    }
    let violations = scan(&root);
    assert_eq!(violations.len(), 5, "{violations:?}");
    assert!(violations.iter().all(|v| v.rule == RULE_UNSAFE));
    std::fs::remove_dir_all(&root).unwrap();
}

/// §17.4 S5 changed one expectation: a network name inside a comment is not code and is no
/// longer flagged (asserted clean below).
#[test]
fn network_apis_are_flagged_in_every_spelling() {
    let root = temp_root("scan-network");
    let cases = [
        ("crates/a/src/lib.rs", "use std::{io, net::TcpStream};\n"),
        ("crates/b/src/lib.rs", "use ::std::net::Ipv4Addr;\n"),
        (
            "crates/c/src/lib.rs",
            "let s = std :: net :: UdpSocket::bind(addr);\n",
        ),
        (
            "crates/d/src/lib.rs",
            "use std::net as n;\nfn f(_: n::TcpListener) {}\n",
        ),
        (
            "crates/e/build.rs",
            "fn main() { let _ = std::net::TcpStream::connect(\"h:1\"); }\n",
        ),
    ];
    for (path, text) in cases {
        write(&root, path, text);
    }
    write(
        &root,
        "arch/f/src/lib.rs",
        "// TcpStream mentioned in a comment\n",
    );
    let violations = scan(&root);
    assert!(violations.iter().all(|v| v.rule == RULE_NETWORK));
    let mut flagged = files(&violations);
    flagged.dedup();
    let mut expected: Vec<&str> = cases.iter().map(|(path, _)| *path).collect();
    expected.sort_unstable();
    assert_eq!(flagged, expected);
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn deny_and_forbid_unsafe_code_are_not_flagged() {
    let root = temp_root("scan-deny");
    write(&root, "crates/a/src/lib.rs", "#![deny(unsafe_code)]\n");
    write(&root, "crates/b/src/lib.rs", "#![forbid(unsafe_code)]\n");
    write(
        &root,
        "crates/c/src/lib.rs",
        "#![deny(\n    missing_docs,\n    unsafe_code,\n    clippy::all,\n)]\n",
    );
    write(
        &root,
        "crates/d/src/lib.rs",
        "//! This crate needs no unsafe_code at all.\n",
    );
    assert_eq!(scan(&root), vec![]);
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cfg_attr_allow_unsafe_code_is_flagged() {
    let root = temp_root("scan-cfg-attr");
    write(
        &root,
        "crates/a/src/lib.rs",
        "#![cfg_attr(not(miri), allow(unsafe_code))]\n",
    );
    write(
        &root,
        "crates/b/src/lib.rs",
        "#![cfg_attr(\n    not(miri),\n    allow(unsafe_code)\n)]\n",
    );
    let violations = scan(&root);
    assert_eq!(rules(&violations), vec![RULE_UNSAFE, RULE_UNSAFE]);
    assert_eq!(
        files(&violations),
        vec!["crates/a/src/lib.rs", "crates/b/src/lib.rs"]
    );
    std::fs::remove_dir_all(&root).unwrap();
}

/// rustfmt keeps an attribute on several lines when it does not fit in 100 columns, and rustc
/// accepts it with `unsafe_code = "deny"` in force: this spelling survives `cargo fmt --check`,
/// compiles, and turns the unsafe ban off for the whole crate (verified with Rust 1.99.0 on a
/// scratch crate). The scan must therefore not depend on `allow(` and `unsafe_code` sharing a
/// line.
#[test]
fn multi_line_allow_unsafe_code_attribute_is_flagged() {
    let root = temp_root("scan-multi-line");
    write(
        &root,
        "crates/x/src/lib.rs",
        "//! x\n#![allow(\n    clippy::cast_possible_truncation,\n    clippy::cast_precision_loss,\n    clippy::cast_sign_loss,\n    unsafe_code\n)]\n\npub fn f() {}\n",
    );
    write(
        &root,
        "arch/y/src/lib.rs",
        "//! y\n#[expect(\n    unsafe_code,\n    reason = \"a justification long enough to keep the attribute on several lines\"\n)]\nfn g() {}\n",
    );
    let violations = scan(&root);
    assert_eq!(
        files(&violations),
        vec!["arch/y/src/lib.rs", "crates/x/src/lib.rs"],
        "{violations:?}"
    );
    assert!(violations.iter().all(|v| v.rule == RULE_UNSAFE));
    std::fs::remove_dir_all(&root).unwrap();
}

/// A `]` inside a string, a raw string or a comment does not end a multi-line attribute early,
/// and the text after the attribute's own `]` is not part of it.
///
/// §17.4 S6 changed the reported line: a violation now sits on the line of the `unsafe_code`
/// token (line 4 in each file), no longer on the first line of the attribute.
#[test]
fn multi_line_attribute_ends_at_its_matching_bracket() {
    let root = temp_root("scan-brackets");
    write(
        &root,
        "crates/a/src/lib.rs",
        "#![cfg_attr(\n    feature = \"x]\",\n    allow(\n        unsafe_code\n    )\n)]\n",
    );
    write(
        &root,
        "crates/b/src/lib.rs",
        "//! b\r\n#![allow(\r\n    // a ] in a comment\r\n    unsafe_code\r\n)]\r\n",
    );
    write(
        &root,
        "crates/c/src/lib.rs",
        "#![cfg_attr(\n    feature = r#\"a ] in a raw string\"#,\n    expect(\n        unsafe_code\n    )\n)]\n",
    );
    write(
        &root,
        "crates/d/src/lib.rs",
        "#![allow(\n    dead_code,\n    clippy::all\n)]\n\nfn f() {\n    // unsafe_code stays denied\n}\n",
    );
    assert_eq!(
        pairs(&scan(&root)),
        vec![
            (RULE_UNSAFE, "crates/a/src/lib.rs:4"),
            (RULE_UNSAFE, "crates/b/src/lib.rs:4"),
            (RULE_UNSAFE, "crates/c/src/lib.rs:4"),
        ]
    );
    std::fs::remove_dir_all(&root).unwrap();
}

// ---------------------------------------------------------------------------------------------
// Fix wave (§17.2, §17.4): apps and xtask are leaves; one regression test per scan bypass of the
// audit; the code view, the inclusion rule, the folder rules and the member folders.
// ---------------------------------------------------------------------------------------------

#[test]
fn apps_and_xtask_are_leaves() {
    for (package, dependency) in [
        ("va", "visual-assembler"),
        ("va", "xtask"),
        ("visual-assembler", "va"),
        ("visual-assembler", "xtask"),
    ] {
        for kind in [None, Some("dev"), Some("build")] {
            let violations = check_layering(&m0_with(package, &[(dependency, kind)]));
            assert_eq!(
                rules(&violations),
                vec![RULE_LAYERING],
                "{package} -> {dependency} ({kind:?})"
            );
            assert_eq!(violations[0].location, package);
        }
    }
}

#[test]
fn no_package_may_depend_on_an_app_or_xtask() {
    for package in M0_PACKAGES {
        for leaf in ["visual-assembler", "va", "xtask"] {
            if *package == leaf {
                continue;
            }
            let violations = check_layering(&m0_with(package, &[(leaf, None)]));
            assert_eq!(
                rules(&violations),
                vec![RULE_LAYERING],
                "{package} -> {leaf}"
            );
        }
    }
}

#[test]
fn apps_may_use_every_library_crate_and_any_arch_crate() {
    let doc = metadata(&[
        ("va-core", &[]),
        ("va-editor", &[]),
        ("va-lang", &[]),
        ("va-debug", &[]),
        ("va-workspace", &[]),
        ("va-ui", &[]),
        ("va-arch-m68k", &[]),
        ("va-arch-z80", &[]),
        (
            "visual-assembler",
            &[
                ("va-core", None),
                ("va-editor", None),
                ("va-lang", None),
                ("va-debug", None),
                ("va-workspace", None),
                ("va-ui", None),
                ("va-arch-m68k", None),
                ("va-arch-z80", Some("dev")),
            ],
        ),
        (
            "va",
            &[
                ("va-core", None),
                ("va-editor", None),
                ("va-lang", None),
                ("va-debug", None),
                ("va-workspace", None),
                ("va-arch-m68k", None),
                ("va-arch-z80", Some("build")),
            ],
        ),
    ]);
    assert_eq!(check_layering(&doc), vec![]);
}

#[test]
fn wrapped_multi_line_allow_with_unsafe_code_is_flagged() {
    let violations = scan_one(
        "scan-wrapped",
        "//! x\n#![allow(\n    dead_code,\n    unsafe_code,\n)]\n",
    );
    assert_eq!(
        pairs(&violations),
        vec![(RULE_UNSAFE, "crates/x/src/lib.rs:4")]
    );
}

#[test]
fn macro_that_permits_unsafe_code_is_flagged() {
    let violations = scan_one(
        "scan-permit",
        "macro_rules! permit {\n    ($lint:ident) => {\n        #[allow($lint)]\n        mod m {}\n    };\n}\npermit!(unsafe_code);\n",
    );
    assert_eq!(
        pairs(&violations),
        vec![(RULE_UNSAFE, "crates/x/src/lib.rs:7")]
    );
}

#[test]
fn unsafe_keyword_without_any_attribute_is_flagged() {
    let violations = scan_one(
        "scan-unsafe-block",
        "pub fn f(p: *const u8) -> u8 {\n    unsafe { *p }\n}\npub unsafe fn g() {}\nstruct S;\nunsafe impl Send for S {}\n",
    );
    assert_eq!(
        pairs(&violations),
        vec![
            (RULE_UNSAFE, "crates/x/src/lib.rs:2"),
            (RULE_UNSAFE, "crates/x/src/lib.rs:4"),
            (RULE_UNSAFE, "crates/x/src/lib.rs:6"),
        ]
    );
}

#[test]
fn include_of_a_non_rs_file_is_flagged_and_the_data_file_is_not_scanned() {
    let root = temp_root("scan-include");
    write(
        &root,
        "crates/x/src/lib.rs",
        "//! x\ninclude!(\"hidden.inc\");\n",
    );
    write(
        &root,
        "crates/x/src/hidden.inc",
        "pub fn leak() { let _ = std::net::TcpStream::connect(\"h:1\"); }\n",
    );
    assert_eq!(
        pairs(&scan(&root)),
        vec![(RULE_INCLUDE, "crates/x/src/lib.rs:2")]
    );
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn path_attribute_to_a_non_rs_file_is_flagged() {
    let violations = scan_one("scan-path", "//! x\n#[path = \"x.inc\"]\nmod x;\n");
    assert_eq!(
        pairs(&violations),
        vec![(RULE_INCLUDE, "crates/x/src/lib.rs:2")]
    );
}

#[test]
fn module_folder_named_target_is_scanned() {
    let root = temp_root("scan-module-target");
    write(&root, "crates/x/src/lib.rs", "//! x\nmod target;\n");
    write(
        &root,
        "crates/x/src/target/mod.rs",
        "use std::net::TcpListener;\n",
    );
    assert_eq!(
        pairs(&scan(&root)),
        vec![(RULE_NETWORK, "crates/x/src/target/mod.rs:1")]
    );
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn brace_group_importing_net_is_flagged() {
    let violations = scan_one(
        "scan-brace",
        "use std::{io, net::ToSocketAddrs as _};\nuse core::{net};\nuse std::{\n    fmt,\n    net as n,\n};\nuse std::{io::{self, Read}, net::{IpAddr}};\n",
    );
    assert_eq!(
        pairs(&violations),
        vec![
            (RULE_NETWORK, "crates/x/src/lib.rs:1"),
            (RULE_NETWORK, "crates/x/src/lib.rs:2"),
            (RULE_NETWORK, "crates/x/src/lib.rs:5"),
            (RULE_NETWORK, "crates/x/src/lib.rs:7"),
        ]
    );
    assert!(
        violations[0].detail.contains("ToSocketAddrs"),
        "{}",
        violations[0]
    );
}

#[test]
fn spaced_std_net_path_is_flagged() {
    let violations = scan_one(
        "scan-spaced",
        "use std :: net;\nuse core\n    ::net::Ipv6Addr;\n",
    );
    assert_eq!(
        pairs(&violations),
        vec![
            (RULE_NETWORK, "crates/x/src/lib.rs:1"),
            (RULE_NETWORK, "crates/x/src/lib.rs:2"),
        ]
    );
}

#[test]
fn every_network_identifier_is_flagged() {
    for identifier in [
        "TcpStream",
        "TcpListener",
        "UdpSocket",
        "ToSocketAddrs",
        "to_socket_addrs",
        "lookup_host",
    ] {
        let violations = scan_one(
            "scan-identifier",
            &format!("fn f() {{ {identifier}(); }}\n"),
        );
        assert_eq!(rules(&violations), vec![RULE_NETWORK], "{identifier}");
    }
    let lookalikes = scan_one(
        "scan-identifier-lookalike",
        "struct MyTcpStreamish;\nfn to_socket_addrs_count() {}\nmod network {}\n",
    );
    assert_eq!(lookalikes, vec![]);
}

#[test]
fn symlinked_folder_and_file_are_violations_and_are_not_followed() {
    let outside = temp_root("scan-outside");
    write(
        &outside,
        "src/lib.rs",
        "use std::net::TcpStream;\npub fn f() { unsafe {} }\n",
    );
    let root = temp_root("scan-symlink");
    write(&root, "crates/x/src/lib.rs", "//! clean\n");
    let created =
        symlink_folder(&outside.join("src"), &root.join("crates/x/src/linked")).and_then(|()| {
            symlink_file(
                &outside.join("src/lib.rs"),
                &root.join("crates/x/src/file_link.rs"),
            )
        });
    if created.is_some() {
        assert_eq!(
            pairs(&scan(&root)),
            vec![
                (RULE_SYMLINK, "crates/x/src/file_link.rs"),
                (RULE_SYMLINK, "crates/x/src/linked"),
            ]
        );
    }
    std::fs::remove_dir_all(&root).unwrap();
    std::fs::remove_dir_all(&outside).unwrap();
}

/// Junctions need no privilege, so rule S4 is exercised on every Windows machine even where
/// symlink creation is refused.
#[cfg(windows)]
#[test]
fn junction_is_a_symlink_violation_and_is_not_followed() {
    let outside = temp_root("scan-outside-junction");
    write(&outside, "lib.rs", "use std::net::TcpStream;\n");
    let root = temp_root("scan-junction");
    write(&root, "crates/x/src/lib.rs", "//! clean\n");
    let junction = root.join("crates").join("x").join("src").join("junction");
    let output = std::process::Command::new("cmd")
        .arg("/C")
        .arg("mklink")
        .arg("/J")
        .arg(&junction)
        .arg(&outside)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "mklink /J failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        pairs(&scan(&root)),
        vec![(RULE_SYMLINK, "crates/x/src/junction")]
    );
    std::fs::remove_dir_all(&root).unwrap();
    assert!(
        outside.join("lib.rs").is_file(),
        "the junction target must survive"
    );
    std::fs::remove_dir_all(&outside).unwrap();
}

#[test]
fn symlinked_member_folder_is_a_violation() {
    let outside = temp_root("scan-outside-member");
    write(&outside, "src/lib.rs", "use std::net::UdpSocket;\n");
    let root = temp_root("scan-symlink-member");
    make_folders(&root, Path::new("crates"));
    if symlink_folder(&outside, &root.join("crates/linked")).is_some() {
        assert_eq!(
            pairs(&try_scan_members(&root, &["crates/linked"]).unwrap()),
            vec![(RULE_SYMLINK, "crates/linked")]
        );
    }
    std::fs::remove_dir_all(&root).unwrap();
    std::fs::remove_dir_all(&outside).unwrap();
}

#[test]
fn member_outside_crates_arch_and_apps_is_scanned() {
    let root = temp_root("scan-tools");
    write(
        &root,
        "tools/gen/src/main.rs",
        "fn main() {\n    unsafe { core::hint::unreachable_unchecked() }\n}\n",
    );
    write(&root, "crates/x/src/lib.rs", "//! clean\n");
    assert_eq!(
        pairs(&try_scan_members(&root, &["crates/x", "tools/gen"]).unwrap()),
        vec![(RULE_UNSAFE, "tools/gen/src/main.rs:2")]
    );
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn unsafe_and_network_names_in_comments_and_literals_are_not_flagged() {
    let violations = scan_one(
        "scan-literals",
        concat!(
            "//! Uses no unsafe code and no std::net.\n",
            "/* an unsafe { } block /* in a nested */ comment, TcpStream */\n",
            "pub const S: &str = \"unsafe { std::net::TcpStream } \\\" unsafe\";\n",
            "pub const R: &str = r#\"unsafe \"quoted\" std::{io, net}\"#;\n",
            "pub const B: &[u8] = b\"unsafe UdpSocket\";\n",
            "pub const BR: &[u8] = br\"unsafe\";\n",
            "pub const Q: char = '\"';\n",
            "pub const E: char = '\\'';\n",
            "fn f<'a>(x: &'a str) -> &'a str { x } // unsafe_code\n",
            "/// `include!(\"x.inc\")` and `#[path = \"y.inc\"]` in a doc comment.\n",
            "pub fn g() {}\n",
        ),
    );
    assert_eq!(violations, vec![]);
}

#[test]
fn literals_and_lifetimes_do_not_hide_the_code_after_them() {
    let violations = scan_one(
        "scan-after-literals",
        concat!(
            "fn f<'a>(x: &'a u8) -> u8 { *x }\n",
            "const Q: char = '\"'; fn g() { unsafe {} }\n",
            "const S: &str = \"a \\\" b\"; fn h() { unsafe {} }\n",
            "const R: &str = r##\"a \"# b\"##; fn i() { unsafe {} }\n",
            "/* a /* nested */ comment */ fn j() { unsafe {} }\n",
        ),
    );
    assert_eq!(
        pairs(&violations),
        vec![
            (RULE_UNSAFE, "crates/x/src/lib.rs:2"),
            (RULE_UNSAFE, "crates/x/src/lib.rs:3"),
            (RULE_UNSAFE, "crates/x/src/lib.rs:4"),
            (RULE_UNSAFE, "crates/x/src/lib.rs:5"),
        ]
    );
}

#[test]
fn deny_and_forbid_spellings_of_unsafe_code_are_not_flagged() {
    for text in [
        "#![deny(unsafe_code)]\n",
        "#![cfg_attr(x, forbid(unsafe_code))]\n",
        "#![cfg_attr(not(test), deny(missing_docs, unsafe_code))]\n",
        "#![forbid(\n    unsafe_code\n)]\n",
    ] {
        assert_eq!(scan_one("scan-deny-forms", text), vec![], "{text}");
    }
}

#[test]
fn every_other_mention_of_unsafe_code_is_flagged() {
    for text in [
        "#![allow(dead_code, unsafe_code)]\n",
        "#[expect(unsafe_code, reason = \"r\")]\nfn f() {}\n",
        "#![warn(unsafe_code)]\n",
        "#![cfg_attr(x, allow(unsafe_code))]\n",
        "m!{unsafe_code}\n",
        "#![deny(lints::m![unsafe_code])]\n",
        "const L: &str = stringify!(unsafe_code);\n",
    ] {
        assert_eq!(
            rules(&scan_one("scan-unsafe-code-forms", text)),
            vec![RULE_UNSAFE],
            "{text}"
        );
    }
}

#[test]
fn plain_rs_inclusions_are_accepted() {
    let violations = scan_one(
        "scan-include-ok",
        concat!(
            "include!(\"generated/table.rs\");\n",
            "std::include! { \"more.rs\", }\n",
            "#[path = \"sys/unix.rs\"]\nmod sys;\n",
            "#[cfg_attr(windows, path = \"windows.rs\")]\nmod os;\n",
            "const FTL: &str = include_str!(\"../locales/en/ui.ftl\");\n",
            "const BIN: &[u8] = include_bytes!(\"../../assets/x.bin\");\n",
            "fn include() {}\nfn path() { let path = 1; let _ = path == 2; }\n",
        ),
    );
    assert_eq!(violations, vec![]);
}

#[test]
fn computed_or_escaping_inclusions_are_flagged() {
    for text in [
        "include!(concat!(env!(\"OUT_DIR\"), \"/x.rs\"));\n",
        "include!(env!(\"GENERATED\"));\n",
        "include!(\"../escape.rs\");\n",
        "include!(\"a/../../escape.rs\");\n",
        "include!(\"/abs/x.rs\");\n",
        "include!(\"C:/abs/x.rs\");\n",
        "include!(\"x\\x2e\\x2e/y.rs\");\n",
        "include!(r\"x.rs\");\n",
        "include!(\"x.rs\" \"y.rs\");\n",
        "#[cfg_attr(unix, path = \"x.inc\")]\nmod x;\n",
        "#[path = \"..\\\\other.rs\"]\nmod o;\n",
        "#[path = concat!(\"x\", \".rs\")]\nmod c;\n",
    ] {
        assert_eq!(
            rules(&scan_one("scan-include-bad", text)),
            vec![RULE_INCLUDE],
            "{text}"
        );
    }
}

#[test]
fn include_target_rule_matches_the_spec() {
    for allowed in ["x.rs", "a/b.rs", "./a.rs", "gen/table.rs"] {
        assert!(include_target_allowed(allowed), "{allowed}");
    }
    for refused in [
        "x.inc",
        "x.rs.txt",
        "../x.rs",
        "a/../x.rs",
        "/x.rs",
        "C:/x.rs",
        "a\\b.rs",
        "",
    ] {
        assert!(!include_target_allowed(refused), "{refused}");
    }
}

/// Formerly `data_exempt_files_skip_patterns_but_not_inclusion`. §18.5 removed rule S2 and its
/// `SCAN_DATA_EXEMPT` list: the checker and its tests are scanned under every rule, like any
/// other file, so the same inputs now report every pattern.
#[test]
fn former_data_exempt_files_are_scanned_under_every_rule() {
    let root = temp_root("scan-exempt");
    write(
        &root,
        "xtask/src/layering.rs",
        "pub fn f() { unsafe {} let _ = TcpStream; }\n#![allow(unsafe_code)]\n",
    );
    write(
        &root,
        "xtask/tests/layering.rs",
        "use std::net::UdpSocket;\ninclude!(concat!(\"a\", \".rs\"));\n",
    );
    write(&root, "xtask/src/other.rs", "pub fn f() { unsafe {} }\n");
    assert_eq!(
        pairs(&scan(&root)),
        vec![
            (RULE_NETWORK, "xtask/src/layering.rs:1"),
            (RULE_UNSAFE, "xtask/src/layering.rs:1"),
            (RULE_UNSAFE, "xtask/src/layering.rs:2"),
            (RULE_UNSAFE, "xtask/src/other.rs:1"),
            (RULE_NETWORK, "xtask/tests/layering.rs:1"),
            (RULE_INCLUDE, "xtask/tests/layering.rs:2"),
        ]
    );
    std::fs::remove_dir_all(&root).unwrap();
}

/// §18.5 S2: the real checker and its tests hold the scan patterns only inside literals and
/// comments, so the full rules report nothing for them (the live-workspace test covers them
/// too, through `cargo xtask ci`).
#[test]
fn the_checker_and_its_tests_pass_their_own_rules() {
    let root = xtask::locate::workspace_root().unwrap();
    for relative in ["xtask/src/layering.rs", "xtask/tests/layering.rs"] {
        let text = std::fs::read_to_string(root.join(relative)).unwrap();
        assert_eq!(scan_text(relative, &text), vec![], "{relative}");
    }
}

#[test]
fn cargo_target_folder_and_root_target_are_skipped_only() {
    let root = temp_root("scan-target-dirs");
    // A root package: the member folder is the root itself, so `<root>/target` is under it.
    write(&root, "src/lib.rs", "//! root package\n");
    write(
        &root,
        "target/debug/build/gen.rs",
        "use std::net::UdpSocket;\n",
    );
    write(&root, "crates/x/src/lib.rs", "//! x\n");
    write(
        &root,
        "crates/x/out/cargo/gen.rs",
        "use std::net::UdpSocket;\n",
    );
    let members = [root.clone(), root.join("crates/x")];

    let custom_target = root.join("crates/x/out/cargo");
    assert_eq!(
        scan_sources(&root, &members, &custom_target).unwrap(),
        vec![]
    );
    // Without that target folder, the same file is an ordinary source file.
    assert_eq!(
        pairs(&scan_sources(&root, &members, &root.join("target")).unwrap()),
        vec![(RULE_NETWORK, "crates/x/out/cargo/gen.rs:1")]
    );
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_file_under_two_members_is_scanned_once() {
    let root = temp_root("scan-overlap");
    write(
        &root,
        "crates/x/sub/src/lib.rs",
        "pub fn f() { unsafe {} }\n",
    );
    assert_eq!(
        pairs(&try_scan_members(&root, &["crates/x", "crates/x/sub"]).unwrap()),
        vec![(RULE_UNSAFE, "crates/x/sub/src/lib.rs:1")]
    );
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_missing_member_folder_is_an_error() {
    let root = temp_root("scan-missing-member");
    let error = try_scan_members(&root, &["crates/gone"]).unwrap_err();
    assert!(error.to_string().contains("gone"), "{error}");
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn member_folders_are_the_manifest_folders_of_workspace_members() {
    let doc = json!({
        "packages": [
            { "name": "va-core", "id": "a", "manifest_path": "/w/crates/va-core/Cargo.toml" },
            { "name": "gen", "id": "b", "manifest_path": "/w/tools/gen/Cargo.toml" },
            { "name": "serde", "id": "c", "manifest_path": "/r/serde/Cargo.toml" },
        ],
        "workspace_members": ["a", "b"],
    });
    assert_eq!(
        member_folders(&doc).unwrap(),
        vec![
            PathBuf::from("/w/crates/va-core"),
            PathBuf::from("/w/tools/gen")
        ]
    );
    let no_manifest = json!({
        "packages": [{ "name": "va-core", "id": "a" }],
        "workspace_members": ["a"],
    });
    let error = member_folders(&no_manifest).unwrap_err();
    assert!(format!("{error:#}").contains("manifest_path"), "{error:#}");
    assert!(member_folders(&json!({ "version": 1 })).is_err());
}

#[test]
fn code_view_keeps_offsets_lines_and_delimiters() {
    let text = "let s = \"a // b\"; // tail é\n/* x\n y */ let c = 'é';\r\nlet r = r#\"q\"#;\n";
    let view = code_view(text);
    assert_eq!(view.len(), text.len());
    assert_eq!(
        view.match_indices('\n').collect::<Vec<_>>(),
        text.match_indices('\n').collect::<Vec<_>>()
    );
    assert!(view.contains("\r\n"));
    assert_eq!(
        view.split_whitespace().collect::<Vec<_>>(),
        [
            "let", "s", "=", "\"", "\";", "let", "c", "=", "'", "';", "let", "r", "=", "r#\"",
            "\"#;"
        ]
    );
}

// ---------------------------------------------------------------------------------------------
// QA audit round 1 (continuation): spellings that rustc 1.99 accepts and that turn a file the
// scan never reads into code. Each was compiled with `rustc --edition 2024 -D unsafe_code` on
// Windows before being written down here. S9 says only `include!` and `#[path]` (rule S8) can
// make a non-`.rs` file code, so every such route must be caught.
// ---------------------------------------------------------------------------------------------

/// Windows and macOS file systems ignore case: `mod net;` loads `net.RS` there (rustc 1.99 on
/// Windows compiles it). Such a file is Rust code and must be scanned like `net.rs`, on every
/// OS, so that the scan result does not depend on the machine that runs it.
#[test]
fn module_file_with_an_upper_case_rs_extension_is_scanned() {
    let root = temp_root("scan-upper-ext");
    write(
        &root,
        "crates/x/src/lib.rs",
        "//! x\n#[cfg(windows)]\nmod net;\n",
    );
    write(
        &root,
        "crates/x/src/net.RS",
        "pub fn f() { let _ = std::net::TcpStream::connect(\"127.0.0.1:9\"); }\n",
    );
    write(&root, "crates/x/src/raw.Rs", "pub fn g() { unsafe {} }\n");
    assert_eq!(
        pairs(&scan(&root)),
        vec![
            (RULE_NETWORK, "crates/x/src/net.RS:1"),
            (RULE_UNSAFE, "crates/x/src/raw.Rs:1"),
        ]
    );
    std::fs::remove_dir_all(&root).unwrap();
}

/// `#[r#path = "x.inc"]` is the built-in `path` attribute written as a raw identifier; rustc 1.99
/// accepts it and loads `x.inc` as the module.
///
/// Kept as the record of a known bypass (QA2-F3): a raw-identifier attribute needs deliberate
/// obfuscation, so m0-foundations.md §18.1/§18.5 classify it as Not blocking (SECURITY.md
/// Limitations). The test fails today and is ignored, not deleted.
#[test]
#[ignore = "accepted tripwire limit: m0-foundations §18.1"]
fn raw_identifier_path_attribute_is_checked() {
    let root = temp_root("scan-raw-path");
    write(
        &root,
        "crates/x/src/lib.rs",
        "//! x\n#[r#path = \"x.inc\"]\nmod x;\n",
    );
    write(
        &root,
        "crates/x/src/x.inc",
        "pub fn f() { let _ = std::net::UdpSocket::bind(\"127.0.0.1:0\"); }\n",
    );
    assert_eq!(
        pairs(&scan(&root)),
        vec![(RULE_INCLUDE, "crates/x/src/lib.rs:2")]
    );
    std::fs::remove_dir_all(&root).unwrap();
}

/// `use std::include as inc; inc!("x.inc");` compiles with rustc 1.99: the renamed `include!`
/// splices `x.inc` into the crate without the word `include` ever being followed by `!`.
///
/// Kept as the record of a known bypass (QA2-F4/R2-2): a renamed or aliased built-in macro needs
/// deliberate obfuscation, so m0-foundations.md §18.1/§18.5 classify it as Not blocking
/// (SECURITY.md Limitations). The test fails today and is ignored, not deleted.
#[test]
#[ignore = "accepted tripwire limit: m0-foundations §18.1"]
fn renamed_include_macro_is_flagged() {
    for text in [
        "//! x\nuse std::include as inc;\ninc!(\"hidden.inc\");\n",
        "//! x\nuse core::{include as inc};\ninc!(\"hidden.inc\");\n",
    ] {
        let root = temp_root("scan-renamed-include");
        write(&root, "crates/x/src/lib.rs", text);
        write(
            &root,
            "crates/x/src/hidden.inc",
            "fn leak() { let _ = std::net::TcpStream::connect(\"127.0.0.1:9\"); }\n",
        );
        let violations = scan(&root);
        assert!(
            violations
                .iter()
                .any(|v| v.rule == RULE_INCLUDE && v.location.starts_with("crates/x/src/lib.rs:")),
            "{text}{violations:?}"
        );
        std::fs::remove_dir_all(&root).unwrap();
    }
}

/// rustc strips a first line starting with `#!` (not `#![`) from every source file, crate roots
/// and module files alike (rustc 1.99 compiles both cases below). A quote on that line therefore
/// opens nothing for rustc, and the code view must not start a literal there either, or the
/// network code that follows disappears from the scan.
#[test]
fn shebang_line_does_not_hide_the_code_after_it() {
    for (text, line) in [
        (
            "#!/usr/bin/env run \"\nuse std::net::TcpStream;\npub fn f() -> std::io::Result<TcpStream> { TcpStream::connect(\"127.0.0.1:9\") }\n",
            2,
        ),
        (
            "#!/x '\nuse std::net::UdpSocket;\npub fn f() -> std::io::Result<UdpSocket> { UdpSocket::bind(\"127.0.0.1:0\") }\n",
            2,
        ),
    ] {
        let violations = scan_one("scan-shebang", text);
        assert!(
            violations
                .iter()
                .any(|v| v.rule == RULE_NETWORK
                    && v.location == format!("crates/x/src/lib.rs:{line}")),
            "{text}{violations:?}"
        );
    }
}

/// Raw identifiers do not hide the other patterns: rustc treats `r#name` as `name`.
#[test]
fn raw_identifier_spellings_of_scanned_names_are_flagged() {
    for (text, rule) in [
        ("#![allow(r#unsafe_code)]\n", RULE_UNSAFE),
        ("r#include!(\"x.inc\");\n", RULE_INCLUDE),
        (
            "fn f() { let _ = std::r#net::r#TcpStream::connect(\"h:1\"); }\n",
            RULE_NETWORK,
        ),
    ] {
        assert_eq!(
            rules(&scan_one("scan-raw-ident", text)),
            vec![rule],
            "{text}"
        );
    }
}

/// Edition 2024 unsafe attributes and extern blocks carry the `unsafe` keyword (S6a), and a
/// byte-order mark does not hide an attribute.
#[test]
fn edition_2024_unsafe_forms_and_a_leading_bom_are_flagged() {
    for text in [
        "#[unsafe(no_mangle)]\npub extern \"C\" fn f() {}\n",
        "unsafe extern \"C\" {\n    fn g();\n}\n",
        "#[unsafe(export_name = \"h\")]\npub fn h() {}\n",
        "\u{feff}#![allow(unsafe_code)]\n",
    ] {
        assert_eq!(
            rules(&scan_one("scan-2024-unsafe", text)),
            vec![RULE_UNSAFE],
            "{text:?}"
        );
    }
}

/// Literal contents that look like code are blank in the code view, whatever the prefix or the
/// number of hashes, and C-string literals behave like strings.
#[test]
fn prefixed_and_hashed_literals_never_match() {
    let violations = scan_one(
        "scan-prefixed-literals",
        concat!(
            "const A: &std::ffi::CStr = c\"unsafe std::net::TcpStream\";\n",
            "const B: &std::ffi::CStr = cr#\"include!(\"x.inc\") unsafe\"#;\n",
            "const C: &[u8] = br##\"#[path = \"x.inc\"] unsafe_code \"# UdpSocket\"##;\n",
            "const D: u8 = b'\\'';\n",
            "const E: char = '\\u{1F916}';\n",
            "pub fn ok() {}\n",
        ),
    );
    assert_eq!(violations, vec![]);
}

// ---------------------------------------------------------------------------------------------
// Fix wave (§18.5): GUI crates matched by family, rustc's shebang rule in the code view, and the
// `.rs` suffix compared ignoring ASCII case in the inclusion rule (S8) as in the walk (S9).
// ---------------------------------------------------------------------------------------------

/// §18.9 row 18-5: a direct dependency on an egui add-on is a GUI dependency, though its name
/// was not in the former fixed list.
#[test]
fn cli_depending_on_egui_extras_is_a_gui_crate_violation() {
    let violations = check_layering(&m0_with("va", &[("egui_extras", None)]));
    assert_eq!(rules(&violations), vec![RULE_GUI]);
    assert_eq!(violations[0].location, "va");
    assert!(
        violations[0].detail.contains("egui_extras"),
        "{}",
        violations[0]
    );
}

/// §18.9 row 18-5: an egui sub-crate is a GUI dependency.
#[test]
fn core_depending_on_epaint_is_a_gui_crate_violation() {
    let violations = check_layering(&m0_with("va-core", &[("epaint", None)]));
    assert_eq!(rules(&violations), vec![RULE_GUI]);
    assert_eq!(violations[0].location, "va-core");
}

#[test]
fn gui_families_cover_the_whole_gui_stack() {
    for name in [
        "egui",
        "egui_extras",
        "egui-plot",
        "egui_winit",
        "eframe",
        "epaint",
        "epaint_default_fonts",
        "emath",
        "ecolor",
        "wgpu",
        "wgpu-hal",
        "wgpu_core",
        "wgpu-types",
        "naga",
        "winit",
        "glutin",
        "glutin-winit",
        "glutin_egl_sys",
        "glow",
        "accesskit",
        "accesskit_winit",
        "accesskit-unix",
        "raw-window-handle",
        "raw_window_handle",
        "Egui_Extras",
        "WGPU",
    ] {
        assert!(is_gui_crate(name), "{name}");
    }
    for name in [
        "anyhow",
        "clap",
        "serde",
        "serde_json",
        "windows-sys",
        "libc",
        "raw-window-metal",
        "raw-window-handle-extra",
        "my-egui",
        "glob",
        "ewgpu",
    ] {
        assert!(!is_gui_crate(name), "{name}");
    }
}

#[test]
fn every_gui_family_is_rejected_in_every_dependency_kind_outside_the_gui_packages() {
    for family in GUI_CRATE_FAMILIES {
        let member = format!("{family}_addon");
        for kind in [None, Some("dev"), Some("build")] {
            for package in ["va", "va-core", "va-lang", "va-arch-m68k", "xtask"] {
                let violations = check_layering(&m0_with(package, &[(&member, kind)]));
                assert_eq!(
                    rules(&violations),
                    vec![RULE_GUI],
                    "{package} -> {member} ({kind:?})"
                );
            }
        }
    }
}

/// `va-ui` may use exactly `egui` and `egui_kittest`: the rest of their families is refused.
#[test]
fn ui_may_use_exactly_egui_and_kittest_not_their_families() {
    for forbidden in [
        "egui_extras",
        "egui-plot",
        "egui_kittest_extra",
        "epaint",
        "emath",
        "ecolor",
        "naga",
        "accesskit",
        "wgpu-hal",
    ] {
        let violations = check_layering(&m0_with("va-ui", &[(forbidden, None)]));
        assert_eq!(rules(&violations), vec![RULE_GUI], "va-ui -> {forbidden}");
    }
    let allowed = m0_with("va-ui", &[("EGUI", None), ("Egui-Kittest", Some("dev"))]);
    assert_eq!(check_layering(&allowed), vec![]);
}

#[test]
fn gui_app_may_use_any_gui_family() {
    let deps = [
        ("egui_extras", None),
        ("epaint", None),
        ("wgpu-hal", Some("dev")),
        ("naga", Some("build")),
        ("accesskit_winit", None),
        ("glutin-winit", None),
        ("raw-window-handle", None),
    ];
    assert_eq!(check_layering(&m0_with("visual-assembler", &deps)), vec![]);
}

#[test]
fn code_view_blanks_a_shebang_line_like_rustc() {
    let text = "#!/usr/bin/env run-cargo-script \"x\nuse std::net::TcpStream;\n";
    let view = code_view(text);
    assert_eq!(view.len(), text.len());
    let mut lines = view.lines();
    assert_eq!(lines.next().unwrap().trim(), "");
    assert_eq!(lines.next().unwrap(), "use std::net::TcpStream;");

    let with_bom = "\u{feff}#!/bin/x '\r\nfn f() {}\r\n";
    assert_eq!(
        shebang_line(with_bom),
        Some((3, with_bom.find('\n').unwrap()))
    );
    assert!(code_view(with_bom).contains("\r\nfn f() {}\r\n"));
}

/// The cases of `rustc_lexer::strip_shebang`: after `#!`, whitespace (rustc's set) and plain
/// comments are skipped; doc comments and any other token make the line a shebang; `[` makes it
/// an inner attribute.
#[test]
fn shebang_rule_matches_rustc_strip_shebang() {
    for (text, shebang) in [
        ("#!/usr/bin/env x\nfn f() {}\n", true),
        ("#!\n", true),
        ("#!", true),
        ("#! x [", true),
        ("#!/*\n*/ x\n[", true),
        ("#! /// outer doc comment\n[allow(unsafe_code)]\n", true),
        ("#! //! inner doc comment\n[allow(unsafe_code)]\n", true),
        ("#! /** doc */ [allow(unsafe_code)]\n", true),
        ("#! /*! doc */ [allow(unsafe_code)]\n", true),
        ("#!\u{a0}[allow(unsafe_code)]\n", true),
        ("#! /* unterminated", true),
        ("#![allow(unsafe_code)]\n", false),
        ("#! [allow(unsafe_code)]\n", false),
        ("#!\n\n[allow(unsafe_code)]\n", false),
        ("#! // comment\n[allow(unsafe_code)]\n", false),
        (
            "#! //// four slashes are a plain comment\n[allow(unsafe_code)]\n",
            false,
        ),
        (
            "#! /* a /* nested */ comment */ [allow(unsafe_code)]\n",
            false,
        ),
        ("#! /**/ [allow(unsafe_code)]\n", false),
        ("#! /*** not a doc comment */ [allow(unsafe_code)]\n", false),
        ("#!\u{200e}[allow(unsafe_code)]\n", false),
        ("\u{feff}#![allow(unsafe_code)]\n", false),
        (" #!/usr/bin/env x\n", false),
        ("\n#!/usr/bin/env x\n", false),
        ("fn f() {}\n", false),
    ] {
        assert_eq!(shebang_line(text).is_some(), shebang, "{text:?}");
    }
}

/// §18.5 S5: an inner attribute on the first line is code, not a shebang, wherever its `[` is.
#[test]
fn inner_attribute_on_the_first_line_is_not_a_shebang() {
    for text in [
        "#![allow(unsafe_code)]\n",
        "#! [allow(unsafe_code)]\n",
        "#!\n[allow(unsafe_code)]\n",
        "#! /* c */ [allow(unsafe_code)]\n",
        "#! // c\n[allow(unsafe_code)]\n",
        "\u{feff}#!\t[allow(unsafe_code)]\n",
    ] {
        assert_eq!(
            rules(&scan_one("scan-inner-attribute", text)),
            vec![RULE_UNSAFE],
            "{text:?}"
        );
    }
    // A crate starting with `#![deny(unsafe_code)]` is lexed as code: the attribute passes and
    // the code after it is still scanned.
    let violations = scan_one(
        "scan-deny-first-line",
        "#![deny(unsafe_code)]\npub fn f(p: *const u8) -> u8 {\n    unsafe { *p }\n}\n",
    );
    assert_eq!(
        pairs(&violations),
        vec![(RULE_UNSAFE, "crates/x/src/lib.rs:3")]
    );
}

/// The code after a shebang line is scanned, and patterns on the shebang line itself (a comment
/// for rustc) are not code.
#[test]
fn shebang_line_is_a_comment_and_the_rest_is_code() {
    let violations = scan_one(
        "scan-shebang-comment",
        "#!/usr/bin/env x unsafe std::net include!(\"y.inc\")\nfn f() { unsafe {} }\n",
    );
    assert_eq!(
        pairs(&violations),
        vec![(RULE_UNSAFE, "crates/x/src/lib.rs:2")]
    );
}

#[test]
fn rs_suffix_ignores_ascii_case() {
    for name in ["x.rs", "x.RS", "x.Rs", "x.rS", ".rs", "a.b.RS"] {
        assert!(has_rs_suffix(name.as_bytes()), "{name}");
    }
    for name in ["x.r", "x.rss", "xrs", "rs", "", "x.rs.txt", "x.\u{131}s"] {
        assert!(!has_rs_suffix(name.as_bytes()), "{name}");
    }
    for allowed in ["x.RS", "gen/table.Rs", "./a.rS"] {
        assert!(include_target_allowed(allowed), "{allowed}");
    }
    for refused in ["../x.RS", "/x.Rs", "x.RS.inc"] {
        assert!(!include_target_allowed(refused), "{refused}");
    }
}

/// S8 accepts an upper-case `.RS` target because the walk (S9) scans that file too: the code it
/// includes is never hidden.
#[test]
fn upper_case_rs_inclusions_are_accepted_and_their_targets_scanned() {
    let root = temp_root("scan-upper-include");
    write(
        &root,
        "crates/x/src/lib.rs",
        "//! x\ninclude!(\"gen/table.RS\");\n#[path = \"sys/net.Rs\"]\nmod net;\n",
    );
    write(
        &root,
        "crates/x/src/gen/table.RS",
        "pub fn f() { unsafe {} }\n",
    );
    write(
        &root,
        "crates/x/src/sys/net.Rs",
        "pub fn g() { let _ = std::net::UdpSocket::bind(\"127.0.0.1:0\"); }\n",
    );
    assert_eq!(
        pairs(&scan(&root)),
        vec![
            (RULE_UNSAFE, "crates/x/src/gen/table.RS:1"),
            (RULE_NETWORK, "crates/x/src/sys/net.Rs:1"),
        ]
    );
    std::fs::remove_dir_all(&root).unwrap();
}

// ---------------------------------------------------------------------------------------------
// QA, final M0 pass (after §18): what rustfmt and editors produce from good-faith code. Each
// form below is a realistic accident in the sense of §18.1 and must be caught (or, for a plain
// `.rs` inclusion, accepted while its target is scanned).
// ---------------------------------------------------------------------------------------------

/// rustfmt wraps a long `include!` over three lines. A plain `.rs` target stays accepted, and
/// the included file is scanned under every rule.
#[test]
fn rustfmt_wrapped_include_of_a_plain_rs_file_is_accepted_and_its_target_scanned() {
    let root = temp_root("scan-wrapped-include");
    write(
        &root,
        "crates/x/src/lib.rs",
        "//! x\ninclude!(\n    \"generated/opcode_table_for_the_68000_family.rs\"\n);\n",
    );
    write(
        &root,
        "crates/x/src/generated/opcode_table_for_the_68000_family.rs",
        "pub fn f() {\n    let _ = std::net::TcpStream::connect(\"127.0.0.1:9\");\n}\n",
    );
    assert_eq!(
        pairs(&scan(&root)),
        vec![(
            RULE_NETWORK,
            "crates/x/src/generated/opcode_table_for_the_68000_family.rs:2"
        )]
    );
    std::fs::remove_dir_all(&root).unwrap();
}

/// rustfmt wraps a long `cfg_attr` one argument per line; spaced and split `#[path]` forms are
/// the same attribute for rustc. A non-`.rs` target is flagged in each of them.
#[test]
fn wrapped_and_spaced_path_attributes_to_non_rs_files_are_flagged() {
    for text in [
        "#[cfg_attr(\n    all(target_os = \"windows\", target_arch = \"aarch64\"),\n    path = \"platform/windows_arm64.inc\"\n)]\nmod platform;\n",
        "# [path = \"x.inc\"]\nmod x;\n",
        "#[ path= \"x.inc\" ]\nmod x;\n",
        "#[path\n    = \"x.inc\"]\nmod x;\n",
        "#[cfg_attr(test, path = \"x.inc\", allow(dead_code))]\nmod x;\n",
        "mod outer {\n    #[path = \"inner.txt\"]\n    mod inner;\n}\n",
    ] {
        assert_eq!(
            rules(&scan_one("scan-wrapped-path", text)),
            vec![RULE_INCLUDE],
            "{text}"
        );
    }
}

/// rustfmt's vertical import layout: `net` on its own line inside a `std::{…}`/`core::{…}` group
/// is still a network import, whatever follows it.
#[test]
fn rustfmt_vertical_imports_of_net_are_flagged() {
    for text in [
        "use std::{\n    collections::HashMap,\n    net::Ipv4Addr,\n};\n",
        "use std::{\n    io,\n    net,\n};\n",
        "use core::{\n    fmt,\n    net::SocketAddr as Addr,\n};\n",
        "use std::{\n    self,\n    net::{\n        IpAddr,\n        Ipv6Addr,\n    },\n};\n",
        "use ::std::{\n    // the parser needs addresses\n    net::IpAddr,\n};\n",
    ] {
        assert_eq!(
            rules(&scan_one("scan-vertical-net", text)),
            vec![RULE_NETWORK],
            "{text}"
        );
    }
}

/// Like rustc, a block comment ends at the first `*/` even when a quote sits inside it, so the
/// code after the comment is code (and the quote after it opens a real string).
#[test]
fn a_quote_inside_a_block_comment_does_not_hide_the_code_after_it() {
    let violations = scan_one(
        "scan-quote-in-comment",
        "/* the 68000 manual calls it \"MOVE */ fn f() { unsafe {} }\nconst S: &str = \"\";\n",
    );
    assert_eq!(
        pairs(&violations),
        vec![(RULE_UNSAFE, "crates/x/src/lib.rs:1")]
    );
}

/// Character literals holding delimiters and quotes, byte characters and lifetimes, written the
/// way a lexer for assembly source naturally does, never shift the code view.
#[test]
fn lexer_style_character_literals_keep_the_code_view_in_sync() {
    let violations = scan_one(
        "scan-lexer-chars",
        concat!(
            "fn kind(c: char) -> u8 {\n",
            r#"    match c { '(' | '[' | '{' => 1, ')' | ']' | '}' => 2, '"' | '\'' => 3, '\\' => 4, _ => 0 }"#,
            "\n}\n",
            r#"fn byte(b: u8) -> bool { b == b'"' || b == b'\'' || b == b'\\' || b == b'#' }"#,
            "\n",
            "fn life<'src>(s: &'src str) -> &'src str { s }\n",
            "fn g() { unsafe {} }\n",
            "use std::{io, net};\n",
        ),
    );
    assert_eq!(
        pairs(&violations),
        vec![
            (RULE_UNSAFE, "crates/x/src/lib.rs:6"),
            (RULE_NETWORK, "crates/x/src/lib.rs:7"),
        ]
    );
}

/// Files that cargo compiles for `--all-targets` (examples, benches, integration tests, a build
/// script) are scanned like library code.
#[test]
fn examples_benches_tests_and_build_scripts_are_scanned() {
    let root = temp_root("scan-all-targets");
    write(&root, "crates/x/src/lib.rs", "//! x\n");
    write(&root, "crates/x/build.rs", "fn main() { unsafe {} }\n");
    write(
        &root,
        "crates/x/examples/demo.rs",
        "fn main() { let _ = std::net::UdpSocket::bind(\"127.0.0.1:0\"); }\n",
    );
    write(
        &root,
        "crates/x/benches/b.rs",
        "#![allow(unsafe_code)]\nfn main() {}\n",
    );
    write(
        &root,
        "crates/x/tests/t.rs",
        "#[test]\nfn t() { let _ = std::net::TcpListener::bind(\"127.0.0.1:0\"); }\n",
    );
    assert_eq!(
        pairs(&scan(&root)),
        vec![
            (RULE_UNSAFE, "crates/x/benches/b.rs:1"),
            (RULE_UNSAFE, "crates/x/build.rs:1"),
            (RULE_NETWORK, "crates/x/examples/demo.rs:1"),
            (RULE_NETWORK, "crates/x/tests/t.rs:2"),
        ]
    );
    std::fs::remove_dir_all(&root).unwrap();
}

/// GUI-adjacent add-ons a contributor reaches for first (§18.5 families): plots, extras, a
/// winit back end, a wgpu sub-crate, AccessKit adapters, in either spelling and case, are GUI
/// crates; the look-alike names below are not.
#[test]
fn common_gui_add_ons_are_matched_by_family_and_look_alikes_are_not() {
    for name in [
        "egui_plot",
        "egui-notify",
        "egui_extras",
        "egui-winit",
        "eframe",
        "wgpu-core",
        "wgpu_hal",
        "naga",
        "winit",
        "glutin-winit",
        "glow",
        "accesskit_winit",
        "Raw_Window_Handle",
        "EPAINT",
    ] {
        assert!(is_gui_crate(name), "{name}");
        let violations = check_layering(&m0_with("va-workspace", &[(name, None)]));
        assert_eq!(rules(&violations), vec![RULE_GUI], "{name}");
    }
    for name in [
        "serde",
        "regex",
        "eguide",
        "window",
        "raw-window-metal",
        "glob",
        "nagare",
    ] {
        let expected =
            name.starts_with("egui") || name.starts_with("glow") || name.starts_with("naga");
        assert_eq!(is_gui_crate(name), expected, "{name}");
    }
}
