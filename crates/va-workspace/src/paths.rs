//! The config and data folders and portable mode (m4-editor-shell.md §4.1, frozen).
//!
//! Resolution is std-only and pure: [`resolve_paths`] reads the OS name, the executable folder,
//! the environment and file existence through [`PathInputs`], so tests pass fakes. Each folder is
//! chosen independently, highest level first: `VA_CONFIG_DIR` / `VA_DATA_DIR`, then a regular
//! file named `portable` next to the executable (`<exe dir>/data/{config,data}`), then the OS
//! folders. Folders are created on first write only, through [`Paths::create_config_dir`],
//! [`Paths::create_data_dir`] and [`Paths::create_backups_dir`].

use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The variable that overrides the config folder.
const VA_CONFIG_DIR: &str = "VA_CONFIG_DIR";

/// The variable that overrides the data folder.
const VA_DATA_DIR: &str = "VA_DATA_DIR";

/// The regular file next to the executable that turns portable mode on.
const PORTABLE_MARKER: &str = "portable";

/// The folder under the executable's folder that holds the portable `config` and `data`.
const PORTABLE_DIR: &str = "data";

/// One OS level: a variable, then the folders appended to its value.
type OsLevel = (&'static str, &'static [&'static str]);

/// Windows config folder: `%APPDATA%\VisualAssembler\config`, else under `%USERPROFILE%`.
const WINDOWS_CONFIG: [OsLevel; 2] = [
    ("APPDATA", &["VisualAssembler", "config"]),
    (
        "USERPROFILE",
        &["AppData", "Roaming", "VisualAssembler", "config"],
    ),
];

/// Windows data folder: `%LOCALAPPDATA%\VisualAssembler\data`, else under `%USERPROFILE%`.
const WINDOWS_DATA: [OsLevel; 2] = [
    ("LOCALAPPDATA", &["VisualAssembler", "data"]),
    (
        "USERPROFILE",
        &["AppData", "Local", "VisualAssembler", "data"],
    ),
];

/// Linux (and any other non-Windows OS) config folder: `$XDG_CONFIG_HOME/visualassembler`, else
/// `$HOME/.config/visualassembler`.
const XDG_CONFIG: [OsLevel; 2] = [
    ("XDG_CONFIG_HOME", &["visualassembler"]),
    ("HOME", &[".config", "visualassembler"]),
];

/// Linux (and any other non-Windows OS) data folder: `$XDG_DATA_HOME/visualassembler`, else
/// `$HOME/.local/share/visualassembler`.
const XDG_DATA: [OsLevel; 2] = [
    ("XDG_DATA_HOME", &["visualassembler"]),
    ("HOME", &[".local", "share", "visualassembler"]),
];

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

/// English text for stderr. It names only the variable, never its value.
impl fmt::Display for PathWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAbsolute { var } => {
                write!(f, "{var} is not an absolute path; it is ignored")
            }
            Self::Empty { var } => write!(f, "{var} is set but empty; it is ignored"),
        }
    }
}

/// Resolves the config and data folders; `None` when no usable location exists. Skipped
/// variables are returned as warnings.
///
/// Each folder takes the first usable level: its `VA_*` variable, then portable mode (a regular
/// file named exactly `portable` in `exe_dir`), then the OS variables (`os == "windows"`: the
/// Windows rows of m4 §4.1; any other OS: the XDG rows). A variable is usable when it is set,
/// non-empty and an absolute path on this host; an empty or relative value is skipped with a
/// [`PathWarning`], an unset one silently. Variables are read lazily, in precedence order, and
/// each at most once, so one bad `HOME` gives one warning. `None` is returned when either folder
/// has no usable level.
pub fn resolve_paths(inputs: &PathInputs<'_>) -> (Option<Paths>, Vec<PathWarning>) {
    let mut resolver = Resolver {
        inputs,
        read: Vec::new(),
        warnings: Vec::new(),
        portable: None,
    };
    let windows = inputs.os == "windows";
    let config = resolver.folder(
        VA_CONFIG_DIR,
        "config",
        if windows {
            &WINDOWS_CONFIG
        } else {
            &XDG_CONFIG
        },
    );
    let data = resolver.folder(
        VA_DATA_DIR,
        "data",
        if windows { &WINDOWS_DATA } else { &XDG_DATA },
    );
    let paths = match (config, data) {
        (Some((config_dir, config_portable)), Some((data_dir, data_portable))) => Some(Paths {
            config_dir,
            data_dir,
            portable: config_portable && data_portable,
        }),
        _ => None,
    };
    (paths, resolver.warnings)
}

/// The state of one [`resolve_paths`] call.
struct Resolver<'i, 'a> {
    /// The injected inputs.
    inputs: &'i PathInputs<'a>,
    /// Every variable read so far, with its usable value.
    read: Vec<(&'static str, Option<PathBuf>)>,
    /// The warnings, in reading order.
    warnings: Vec<PathWarning>,
    /// The portable root (`<exe dir>/data`) once checked; `Some(None)` when not portable.
    portable: Option<Option<PathBuf>>,
}

impl Resolver<'_, '_> {
    /// The folder of one kind and whether it comes from portable mode.
    fn folder(
        &mut self,
        variable: &'static str,
        portable_name: &str,
        os_levels: &[OsLevel],
    ) -> Option<(PathBuf, bool)> {
        if let Some(dir) = self.absolute_var(variable) {
            return Some((dir, false));
        }
        if let Some(root) = self.portable_root() {
            return Some((root.join(portable_name), true));
        }
        os_levels.iter().find_map(|&(variable, components)| {
            let base = self.absolute_var(variable)?;
            Some((
                components.iter().fold(base, |dir, part| dir.join(part)),
                false,
            ))
        })
    }

    /// The value of `variable` when it is a usable folder; a skipped value is warned once.
    fn absolute_var(&mut self, variable: &'static str) -> Option<PathBuf> {
        if let Some((_, value)) = self.read.iter().find(|(name, _)| *name == variable) {
            return value.clone();
        }
        let value = (self.inputs.var)(variable).and_then(|value| {
            if value.is_empty() {
                self.warnings.push(PathWarning::Empty { var: variable });
                None
            } else if Path::new(&value).is_absolute() {
                Some(PathBuf::from(value))
            } else {
                self.warnings
                    .push(PathWarning::NotAbsolute { var: variable });
                None
            }
        });
        self.read.push((variable, value.clone()));
        value
    }

    /// `<exe dir>/data` when a regular file `portable` sits in the executable's folder; checked
    /// once.
    fn portable_root(&mut self) -> Option<PathBuf> {
        let inputs = self.inputs;
        self.portable
            .get_or_insert_with(|| {
                let exe_dir = inputs.exe_dir?;
                (inputs.is_file)(&exe_dir.join(PORTABLE_MARKER)).then(|| exe_dir.join(PORTABLE_DIR))
            })
            .clone()
    }
}

impl Paths {
    /// `<config>/settings.toml`.
    pub fn settings(&self) -> PathBuf {
        self.config_dir.join("settings.toml")
    }

    /// `<config>/keybindings.toml`.
    pub fn keybindings(&self) -> PathBuf {
        self.config_dir.join("keybindings.toml")
    }

    /// `<data>/session.toml`.
    pub fn session(&self) -> PathBuf {
        self.data_dir.join("session.toml")
    }

    /// `<data>/session.lock`.
    pub fn session_lock(&self) -> PathBuf {
        self.data_dir.join("session.lock")
    }

    /// `<data>/backups`.
    pub fn backups_dir(&self) -> PathBuf {
        self.data_dir.join("backups")
    }

    /// `<data>/backups/index.toml`.
    pub fn backups_index(&self) -> PathBuf {
        self.backups_dir().join("index.toml")
    }

    /// Creates the config folder and its missing parents (`create_dir_all`), before the first
    /// write into it; an existing folder is left as is.
    ///
    /// # Errors
    /// The error of `fs::create_dir_all`.
    pub fn create_config_dir(&self) -> io::Result<()> {
        fs::create_dir_all(&self.config_dir)
    }

    /// Creates the data folder and its missing parents, before the first write into it; on Unix
    /// every folder it creates gets mode `0o700` (less the umask). An existing folder is left as
    /// is.
    ///
    /// # Errors
    /// The error of the folder creation.
    pub fn create_data_dir(&self) -> io::Result<()> {
        create_private_dir_all(&self.data_dir)
    }

    /// Creates `<data>/backups` and its missing parents, as [`Paths::create_data_dir`] does.
    ///
    /// # Errors
    /// The error of the folder creation.
    pub fn create_backups_dir(&self) -> io::Result<()> {
        create_private_dir_all(&self.backups_dir())
    }
}

/// `create_dir_all`, with mode `0o700` for every created folder on Unix (std `DirBuilderExt`).
#[cfg(unix)]
fn create_private_dir_all(dir: &Path) -> io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)
}

/// `create_dir_all` (folders inherit the user profile's access rules on Windows).
#[cfg(not(unix))]
fn create_private_dir_all(dir: &Path) -> io::Result<()> {
    fs::create_dir_all(dir)
}
