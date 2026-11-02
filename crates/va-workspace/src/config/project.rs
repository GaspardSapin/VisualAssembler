//! `.visualasm/project.toml` (m4-editor-shell.md §4.5, frozen): the project of a folder opened
//! with "Open folder", and [`parse_project`].
//!
//! Every path in the file must be relative to the project root without a `..` component, on
//! every host (`/` and `\` are both separators), so a downloaded project cannot point outside its
//! folder. The tables of later milestones are kept raw.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use toml::Spanned;
use toml::de::{DeTable, DeValue, ValueDeserializer};

use super::problems::{
    ConfigProblem, Report, SCHEMA_VERSION_KEY, expected, is_contained_relative_path,
    parse_document, strip_bom,
};

/// Path of the project file, relative to the project root.
pub const PROJECT_FILE: &str = ".visualasm/project.toml";

/// The table of this milestone.
const PROJECT: &str = "project";

/// Tables reserved for later milestones (M3, M5, M6), kept raw without warnings.
const RESERVED_TABLES: [&str; 4] = ["assembler", "output", "run", "debug"];

/// A project: a folder opened with "Open folder", and its optional `project.toml`.
#[derive(Clone, Debug, PartialEq)]
pub struct Project {
    /// The project root.
    pub root: PathBuf,
    /// Id of a registered architecture; `None` means the app's default.
    pub arch: Option<String>,
    /// The I/O profile (M3 meaning; stored as is in M4).
    pub io_profile: Option<String>,
    /// The entry file, relative to the root without `..`.
    pub entry: Option<PathBuf>,
    /// Include folders, relative to the root without `..` (used from M5).
    pub include_dirs: Vec<PathBuf>,
    /// Tables reserved for later milestones (`assembler`, `output`, `run`, `debug`), kept raw.
    pub reserved: BTreeMap<String, toml::Table>,
}

/// Parses the `project.toml` of `root` (`text` is `None` when the file is missing); problems
/// name the file `<root>/.visualasm/project.toml`, in file order.
///
/// `[project]` holds `arch` and `io_profile` (strings, stored as written: the app checks `arch`
/// against its registry, see [`project_arch_problem`]), `entry` (a path string) and
/// `include_dirs` (an array of path strings). A path that is absolute, rooted, has a drive
/// prefix, contains `..` or is empty is `cfg-bad-path` and dropped. The `[assembler]`,
/// `[output]`, `[run]` and `[debug]` tables are kept raw. A syntax error or a `schema_version`
/// other than 1 gives the empty project and that one problem.
pub fn parse_project(root: &Path, text: Option<&str>) -> (Project, Vec<ConfigProblem>) {
    let mut project = Project {
        root: root.to_path_buf(),
        arch: None,
        io_profile: None,
        entry: None,
        include_dirs: Vec::new(),
        reserved: BTreeMap::new(),
    };
    let Some(text) = text else {
        return (project, Vec::new());
    };
    let text = strip_bom(text);
    let mut report = Report::new(root.join(PROJECT_FILE), text);
    if let Some(document) = parse_document(&mut report, text) {
        for (key, value) in &document {
            let name: &str = key.get_ref();
            if name == SCHEMA_VERSION_KEY {
                continue;
            }
            if name == PROJECT {
                if let Some(table) = report.table(name, value) {
                    read_project(&mut report, table, &mut project);
                }
            } else if RESERVED_TABLES.contains(&name) {
                if report.table(name, value).is_some()
                    && let Ok(raw) =
                        toml::Table::deserialize(ValueDeserializer::from(value.clone()))
                {
                    project.reserved.insert(name.to_owned(), raw);
                }
            } else {
                report.unknown_key(name, key.span());
            }
        }
    }
    (project, report.finish())
}

/// Reports the `arch` of a project file that names no registered architecture: the
/// `cfg-invalid-value` problem of m4 §4.5 (key `project.arch`, `expected` the `known` ids joined
/// by `, `), placed on the value. `None` when the file has no `arch`, names a known one, or does
/// not parse (`parse_project` reports that).
///
/// The app calls it with the text it gave to [`parse_project`], then uses its default
/// architecture.
pub fn project_arch_problem(
    root: &Path,
    text: Option<&str>,
    known: &[&str],
) -> Option<ConfigProblem> {
    let text = strip_bom(text?);
    let mut report = Report::new(root.join(PROJECT_FILE), text);
    let document = parse_document(&mut report, text)?;
    let arch = document.get(PROJECT)?.get_ref().as_table()?.get("arch")?;
    let id = arch.get_ref().as_str()?;
    if known.contains(&id) {
        return None;
    }
    report.invalid_value("project.arch", id, &known.join(", "), arch.span());
    report.finish().into_iter().next()
}

/// The `[project]` table.
fn read_project(report: &mut Report<'_>, table: &DeTable<'_>, project: &mut Project) {
    for (key, value) in table {
        let name: &str = key.get_ref();
        let dotted = format!("{PROJECT}.{name}");
        match name {
            "arch" => {
                if let Some(text) = report.string(&dotted, value) {
                    project.arch = Some(text.to_owned());
                }
            }
            "io_profile" => {
                if let Some(text) = report.string(&dotted, value) {
                    project.io_profile = Some(text.to_owned());
                }
            }
            "entry" => {
                if let Some(path) = relative_path(report, &dotted, value) {
                    project.entry = Some(path);
                }
            }
            "include_dirs" => {
                if let Some(array) = report.array(&dotted, value) {
                    project.include_dirs = array
                        .iter()
                        .filter_map(|element| relative_path(report, &dotted, element))
                        .collect();
                }
            }
            _ => report.unknown_key(&dotted, key.span()),
        }
    }
}

/// A path string that stays inside the project root, or `None` after its problem
/// (`cfg-wrong-type` for a non-string, `cfg-bad-path` otherwise).
fn relative_path(
    report: &mut Report<'_>,
    key: &str,
    value: &Spanned<DeValue<'_>>,
) -> Option<PathBuf> {
    let Some(text) = value.get_ref().as_str() else {
        report.wrong_type(key, expected::STRING, value.span());
        return None;
    };
    if is_contained_relative_path(text) {
        Some(PathBuf::from(text))
    } else {
        report.bad_path(key, text, value.span());
        None
    }
}
