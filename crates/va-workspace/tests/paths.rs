//! Tests of `va_workspace::paths` (m4-editor-shell.md §4.1, §10.1): every level of the folder
//! table with a fake environment and a fake `is_file`, the warnings, and folder creation in a
//! test temp folder. No test reads the real environment or the real user folders.

#![allow(clippy::unwrap_used)]

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use va_workspace::paths::{PathInputs, PathWarning, Paths, resolve_paths};

/// An absolute folder on this host (never created by these tests, except under the temp
/// folder): the package folder, which cargo always gives as an absolute path.
fn abs(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fake-root")
        .join(name)
}

/// Joins `parts` under `base`.
fn under(base: &Path, parts: &[&str]) -> PathBuf {
    parts
        .iter()
        .fold(base.to_path_buf(), |dir, part| dir.join(part))
}

/// A fake world: the environment, the executable folder, and whether a `portable` file is there.
struct World {
    /// `std::env::consts::OS` as seen by the resolver.
    os: &'static str,
    /// The environment variables.
    vars: HashMap<&'static str, OsString>,
    /// The executable folder, if known.
    exe_dir: Option<PathBuf>,
    /// Whether `<exe_dir>/portable` is a regular file.
    portable_file: bool,
}

/// What one resolution returned and observed.
struct Outcome {
    /// The folders, if any.
    paths: Option<Paths>,
    /// The warnings, in order.
    warnings: Vec<PathWarning>,
    /// Every variable read, in order.
    reads: Vec<String>,
    /// How many times `is_file` was called.
    is_file_calls: usize,
}

impl World {
    /// An empty environment on `os`, without an executable folder.
    fn new(os: &'static str) -> Self {
        Self {
            os,
            vars: HashMap::new(),
            exe_dir: None,
            portable_file: false,
        }
    }

    /// Sets the variable `name` to `value`.
    fn var(mut self, name: &'static str, value: impl Into<OsString>) -> Self {
        self.vars.insert(name, value.into());
        self
    }

    /// Sets the executable folder and whether a `portable` file sits in it.
    fn exe(mut self, dir: PathBuf, portable_file: bool) -> Self {
        self.exe_dir = Some(dir);
        self.portable_file = portable_file;
        self
    }

    /// Runs `resolve_paths` on this world.
    fn resolve(&self) -> Outcome {
        let reads = RefCell::new(Vec::new());
        let is_file_calls = Cell::new(0);
        let var = |name: &str| {
            reads.borrow_mut().push(name.to_owned());
            self.vars.get(name).cloned()
        };
        let marker = self.exe_dir.as_ref().map(|dir| dir.join("portable"));
        let is_file = |path: &Path| {
            is_file_calls.set(is_file_calls.get() + 1);
            self.portable_file && marker.as_deref() == Some(path)
        };
        let inputs = PathInputs {
            os: self.os,
            exe_dir: self.exe_dir.as_deref(),
            var: &var,
            is_file: &is_file,
        };
        let (paths, warnings) = resolve_paths(&inputs);
        Outcome {
            paths,
            warnings,
            reads: reads.into_inner(),
            is_file_calls: is_file_calls.get(),
        }
    }
}

/// The folders of an outcome that must have them.
fn folders(outcome: &Outcome) -> (PathBuf, PathBuf, bool) {
    let paths = outcome.paths.as_ref().unwrap();
    (
        paths.config_dir.clone(),
        paths.data_dir.clone(),
        paths.portable,
    )
}

#[test]
fn environment_variables_win_over_portable_mode_and_the_os() {
    for os in ["windows", "linux"] {
        let outcome = World::new(os)
            .var("VA_CONFIG_DIR", abs("env-config"))
            .var("VA_DATA_DIR", abs("env-data"))
            .var("APPDATA", abs("appdata"))
            .var("LOCALAPPDATA", abs("localappdata"))
            .var("HOME", abs("home"))
            .exe(abs("exe"), true)
            .resolve();
        assert_eq!(
            folders(&outcome),
            (abs("env-config"), abs("env-data"), false),
            "{os}"
        );
        assert!(outcome.warnings.is_empty(), "{os}");
        // Lazy reads: the lower levels are never consulted.
        assert_eq!(outcome.reads, ["VA_CONFIG_DIR", "VA_DATA_DIR"], "{os}");
        assert_eq!(outcome.is_file_calls, 0, "{os}");
    }
}

#[test]
fn a_portable_file_next_to_the_executable_wins_over_the_os() {
    for os in ["windows", "linux"] {
        let outcome = World::new(os)
            .var("APPDATA", abs("appdata"))
            .var("LOCALAPPDATA", abs("localappdata"))
            .var("XDG_CONFIG_HOME", abs("xdg-config"))
            .var("XDG_DATA_HOME", abs("xdg-data"))
            .exe(abs("exe"), true)
            .resolve();
        assert_eq!(
            folders(&outcome),
            (
                under(&abs("exe"), &["data", "config"]),
                under(&abs("exe"), &["data", "data"]),
                true
            ),
            "{os}"
        );
        assert!(outcome.warnings.is_empty(), "{os}");
        assert_eq!(outcome.is_file_calls, 1, "{os}: the marker is checked once");
    }
}

#[test]
fn portable_mode_needs_a_regular_file_and_a_known_executable_folder() {
    // A missing marker (or a folder named `portable`: `is_file` is false) and an unknown
    // executable folder both fall through to the OS folders.
    for world in [
        World::new("linux").exe(abs("exe"), false),
        World::new("linux"),
    ] {
        let outcome = world
            .var("XDG_CONFIG_HOME", abs("xdg-config"))
            .var("XDG_DATA_HOME", abs("xdg-data"))
            .resolve();
        assert_eq!(
            folders(&outcome),
            (
                abs("xdg-config").join("visualassembler"),
                abs("xdg-data").join("visualassembler"),
                false
            )
        );
    }
}

#[test]
fn windows_uses_appdata_and_localappdata() {
    let outcome = World::new("windows")
        .var("APPDATA", abs("roaming"))
        .var("LOCALAPPDATA", abs("local"))
        .var("USERPROFILE", abs("profile"))
        .resolve();
    assert_eq!(
        folders(&outcome),
        (
            under(&abs("roaming"), &["VisualAssembler", "config"]),
            under(&abs("local"), &["VisualAssembler", "data"]),
            false
        )
    );
    assert!(outcome.warnings.is_empty());
    assert!(!outcome.reads.iter().any(|name| name == "USERPROFILE"));
}

#[test]
fn windows_falls_back_to_userprofile() {
    let outcome = World::new("windows")
        .var("USERPROFILE", abs("profile"))
        .resolve();
    assert_eq!(
        folders(&outcome),
        (
            under(
                &abs("profile"),
                &["AppData", "Roaming", "VisualAssembler", "config"]
            ),
            under(
                &abs("profile"),
                &["AppData", "Local", "VisualAssembler", "data"]
            ),
            false
        )
    );
    assert!(outcome.warnings.is_empty());
    // Each folder falls back on its own: APPDATA set, LOCALAPPDATA missing.
    let outcome = World::new("windows")
        .var("APPDATA", abs("roaming"))
        .var("USERPROFILE", abs("profile"))
        .resolve();
    assert_eq!(
        folders(&outcome),
        (
            under(&abs("roaming"), &["VisualAssembler", "config"]),
            under(
                &abs("profile"),
                &["AppData", "Local", "VisualAssembler", "data"]
            ),
            false
        )
    );
}

#[test]
fn linux_uses_the_xdg_variables() {
    let outcome = World::new("linux")
        .var("XDG_CONFIG_HOME", abs("xdg-config"))
        .var("XDG_DATA_HOME", abs("xdg-data"))
        .var("HOME", abs("home"))
        .resolve();
    assert_eq!(
        folders(&outcome),
        (
            abs("xdg-config").join("visualassembler"),
            abs("xdg-data").join("visualassembler"),
            false
        )
    );
    assert!(outcome.warnings.is_empty());
}

#[test]
fn linux_falls_back_to_home() {
    let outcome = World::new("linux").var("HOME", abs("home")).resolve();
    assert_eq!(
        folders(&outcome),
        (
            under(&abs("home"), &[".config", "visualassembler"]),
            under(&abs("home"), &[".local", "share", "visualassembler"]),
            false
        )
    );
    assert!(outcome.warnings.is_empty());
    assert_eq!(
        outcome.reads.iter().filter(|name| *name == "HOME").count(),
        1,
        "HOME is read once for both folders"
    );
}

#[test]
fn other_systems_use_the_xdg_rows() {
    let outcome = World::new("freebsd").var("HOME", abs("home")).resolve();
    assert_eq!(
        folders(&outcome),
        (
            under(&abs("home"), &[".config", "visualassembler"]),
            under(&abs("home"), &[".local", "share", "visualassembler"]),
            false
        )
    );
}

#[test]
fn relative_and_empty_values_are_warned_and_skipped() {
    let outcome = World::new("linux")
        .var("VA_CONFIG_DIR", "")
        .var("VA_DATA_DIR", "relative/data")
        .var("XDG_CONFIG_HOME", "relative/config")
        .var("XDG_DATA_HOME", "")
        .var("HOME", abs("home"))
        .resolve();
    assert_eq!(
        folders(&outcome),
        (
            under(&abs("home"), &[".config", "visualassembler"]),
            under(&abs("home"), &[".local", "share", "visualassembler"]),
            false
        )
    );
    assert_eq!(
        outcome.warnings,
        [
            PathWarning::Empty {
                var: "VA_CONFIG_DIR"
            },
            PathWarning::NotAbsolute {
                var: "XDG_CONFIG_HOME"
            },
            PathWarning::NotAbsolute { var: "VA_DATA_DIR" },
            PathWarning::Empty {
                var: "XDG_DATA_HOME"
            },
        ]
    );
}

#[test]
fn a_bad_shared_variable_is_warned_once() {
    let outcome = World::new("windows")
        .var("USERPROFILE", "profile")
        .resolve();
    assert!(outcome.paths.is_none());
    assert_eq!(
        outcome.warnings,
        [PathWarning::NotAbsolute { var: "USERPROFILE" }]
    );
}

#[cfg(windows)]
#[test]
fn a_unix_style_path_is_not_absolute_on_windows() {
    let outcome = World::new("windows")
        .var("VA_CONFIG_DIR", "/home/me/config")
        .var("APPDATA", abs("roaming"))
        .var("LOCALAPPDATA", abs("local"))
        .resolve();
    assert_eq!(
        outcome.warnings,
        [PathWarning::NotAbsolute {
            var: "VA_CONFIG_DIR"
        }]
    );
    assert_eq!(
        folders(&outcome).0,
        under(&abs("roaming"), &["VisualAssembler", "config"])
    );
}

#[test]
fn each_folder_is_chosen_independently() {
    // Config from the environment, data from portable mode: not a portable installation.
    let outcome = World::new("windows")
        .var("VA_CONFIG_DIR", abs("env-config"))
        .exe(abs("exe"), true)
        .resolve();
    assert_eq!(
        folders(&outcome),
        (
            abs("env-config"),
            under(&abs("exe"), &["data", "data"]),
            false
        )
    );
    // Data from the environment, config from the OS.
    let outcome = World::new("linux")
        .var("VA_DATA_DIR", abs("env-data"))
        .var("HOME", abs("home"))
        .resolve();
    assert_eq!(
        folders(&outcome),
        (
            under(&abs("home"), &[".config", "visualassembler"]),
            abs("env-data"),
            false
        )
    );
}

#[test]
fn no_usable_location_gives_none() {
    for os in ["windows", "linux"] {
        let outcome = World::new(os).resolve();
        assert!(outcome.paths.is_none(), "{os}");
        assert!(
            outcome.warnings.is_empty(),
            "{os}: unset variables are silent"
        );
    }
    // One folder without a location is enough.
    let outcome = World::new("linux")
        .var("VA_CONFIG_DIR", abs("env-config"))
        .resolve();
    assert!(outcome.paths.is_none());
}

#[test]
fn warnings_have_an_english_text_that_names_only_the_variable() {
    assert_eq!(
        PathWarning::NotAbsolute { var: "VA_DATA_DIR" }.to_string(),
        "VA_DATA_DIR is not an absolute path; it is ignored"
    );
    assert_eq!(
        PathWarning::Empty { var: "HOME" }.to_string(),
        "HOME is set but empty; it is ignored"
    );
}

#[test]
fn file_locations_follow_the_folders() {
    let paths = Paths {
        config_dir: abs("config"),
        data_dir: abs("data"),
        portable: false,
    };
    assert_eq!(paths.settings(), abs("config").join("settings.toml"));
    assert_eq!(paths.keybindings(), abs("config").join("keybindings.toml"));
    assert_eq!(paths.session(), abs("data").join("session.toml"));
    assert_eq!(paths.session_lock(), abs("data").join("session.lock"));
    assert_eq!(paths.backups_dir(), abs("data").join("backups"));
    assert_eq!(
        paths.backups_index(),
        under(&abs("data"), &["backups", "index.toml"])
    );
}

/// Creates a fresh, empty folder `<CARGO_TARGET_TMPDIR>/<tag>-<pid>-<n>` (CLAUDE.md test rule).
fn temp_dir(tag: &str) -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("{tag}-{}-{n}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    dir
}

#[test]
fn folders_are_created_on_demand_with_their_parents() {
    let root = temp_dir("paths-create");
    let paths = Paths {
        config_dir: under(&root, &["deep", "config"]),
        data_dir: under(&root, &["deep", "data"]),
        portable: false,
    };
    assert!(!paths.config_dir.exists() && !paths.data_dir.exists());
    paths.create_config_dir().unwrap();
    assert!(paths.config_dir.is_dir());
    assert!(
        !paths.data_dir.exists(),
        "each folder is created on its own"
    );
    paths.create_backups_dir().unwrap();
    assert!(paths.data_dir.is_dir() && paths.backups_dir().is_dir());
    // Creating again is harmless.
    paths.create_config_dir().unwrap();
    paths.create_data_dir().unwrap();
    paths.create_backups_dir().unwrap();
}

#[cfg(unix)]
#[test]
fn data_folders_are_private_on_unix() {
    use std::os::unix::fs::PermissionsExt;
    let root = temp_dir("paths-mode");
    let paths = Paths {
        config_dir: root.join("config"),
        data_dir: root.join("data"),
        portable: false,
    };
    paths.create_data_dir().unwrap();
    paths.create_backups_dir().unwrap();
    for dir in [paths.data_dir.clone(), paths.backups_dir()] {
        let mode = std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o700, "{}", dir.display());
    }
}
