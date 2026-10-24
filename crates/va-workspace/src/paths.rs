//! The config and data folders and portable mode (m4-editor-shell.md §4.1, frozen).
//!
//! Resolution is std-only and pure: [`resolve_paths`] reads the OS name, the executable folder,
//! the environment and file existence through [`PathInputs`], so tests pass fakes. Each folder is
//! chosen independently, highest level first: `VA_CONFIG_DIR` / `VA_DATA_DIR`, then a regular
//! file named `portable` next to the executable (`<exe dir>/data/{config,data}`), then the OS
//! folders. Folders are created on first write only.
//!
//! WP0 scaffold: frozen signatures with `todo!()` bodies; WP-B implements them.

#![expect(
    unused_variables,
    clippy::todo,
    reason = "WP0 stub: WP-B implements m4 §4.1"
)]

use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};

/// Where the IDE keeps its files.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Paths {
    /// Folder of `settings.toml` and `keybindings.toml`.
    pub config_dir: PathBuf,
    /// Folder of `session.toml`, `session.lock` and `backups/`.
    pub data_dir: PathBuf,
    /// Whether both folders come from portable mode.
    pub portable: bool,
}

/// What [`resolve_paths`] reads, injected so tests control it.
#[derive(Clone, Copy)]
pub struct PathInputs<'a> {
    /// `std::env::consts::OS` (`"windows"`, `"linux"`, …).
    pub os: &'a str,
    /// Folder of the running executable, when known.
    pub exe_dir: Option<&'a Path>,
    /// Reads one environment variable (`None` when unset).
    pub var: &'a dyn Fn(&str) -> Option<OsString>,
    /// Tells whether a path is an existing regular file.
    pub is_file: &'a dyn Fn(&Path) -> bool,
}

/// A location variable that [`resolve_paths`] skipped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PathWarning {
    /// The variable holds a relative path.
    NotAbsolute {
        /// The variable name.
        var: &'static str,
    },
    /// The variable is set but empty.
    Empty {
        /// The variable name.
        var: &'static str,
    },
}

/// English text for stderr.
impl fmt::Display for PathWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

/// Resolves the config and data folders; `None` when no usable location exists. Skipped
/// variables are returned as warnings.
pub fn resolve_paths(inputs: &PathInputs<'_>) -> (Option<Paths>, Vec<PathWarning>) {
    todo!()
}

impl Paths {
    /// `<config>/settings.toml`.
    pub fn settings(&self) -> PathBuf {
        todo!()
    }

    /// `<config>/keybindings.toml`.
    pub fn keybindings(&self) -> PathBuf {
        todo!()
    }

    /// `<data>/session.toml`.
    pub fn session(&self) -> PathBuf {
        todo!()
    }

    /// `<data>/session.lock`.
    pub fn session_lock(&self) -> PathBuf {
        todo!()
    }

    /// `<data>/backups`.
    pub fn backups_dir(&self) -> PathBuf {
        todo!()
    }

    /// `<data>/backups/index.toml`.
    pub fn backups_index(&self) -> PathBuf {
        todo!()
    }
}
