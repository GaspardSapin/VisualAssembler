//! UI i18n completeness (m4-editor-shell.md §4.6, §10.2, exit criterion E6): every `ui-*` and
//! `cmd-*` id the GUI references exists in `en` and `fr`, no such id is unused, and no CPU's
//! resources collide with the root ones in the GUI bundle.
//!
//! References are the string literals matching `"(ui|cmd)-[a-z0-9-]+"` in `crates/va-ui/src/**`
//! and `apps/visual-assembler/src/**` (a plain text scan of the `.rs` files) plus the title id
//! `cmd-<group>-<action>` of every command of the registry (§5.12). The registry has no public
//! list, so [`REGISTRY`] mirrors the §5.12 table and is checked against the registry's source
//! once `shell/commands.rs` exists.
//!
//! The `.ftl` content belongs to WP-F (`ui.ftl`, `commands.ftl`) and WP-B (`config.ftl`): on
//! header-only files every check still runs and holds (no literal, no id), and each one
//! tightens as they fill. The registry's titles are required as soon as `commands.ftl` defines
//! a message or `shell/commands.rs` exists. CPU resources are read from `arch/*/locales/` on
//! disk, because rule R2 forbids `va-ui` to depend on a CPU crate, dev-dependencies included;
//! `apps/visual-assembler/tests/arch_registry.rs` builds the same bundle from the registered CPUs.
#![allow(clippy::unwrap_used)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use va_core::arch::Architecture;
use va_core::diag::DiagCode;
use va_core::isa::{EaTiming, InstructionInfo, InstructionSet, LookupError};
use va_core::regs::RegisterSpec;
use va_core::syntax::{KeywordInfo, LineState, SyntaxProvider, Token};
use va_workspace::i18n::{Lang, Localizer, ui_resources};

/// The command ids of the registry, as in the §5.12 table (`<group>.<action>`).
const REGISTRY: [&str; 44] = [
    "file.new",
    "file.open",
    "file.open-folder",
    "file.save",
    "file.save-as",
    "file.save-all",
    "file.close-tab",
    "file.revert",
    "file.reopen-windows-1252",
    "edit.undo",
    "edit.redo",
    "edit.select-all",
    "edit.select-next-occurrence",
    "edit.add-cursor-above",
    "edit.add-cursor-below",
    "edit.toggle-comment",
    "edit.delete-lines",
    "edit.duplicate-lines",
    "edit.move-lines-up",
    "edit.move-lines-down",
    "find.find",
    "find.replace",
    "find.next",
    "find.previous",
    "go.line",
    "go.file",
    "palette.commands",
    "view.zoom-in",
    "view.zoom-out",
    "view.zoom-reset",
    "view.toggle-explorer",
    "view.toggle-theme",
    "view.theme-from-settings",
    "tab.next",
    "tab.previous",
    "explorer.new-file",
    "explorer.new-folder",
    "explorer.rename",
    "explorer.refresh",
    "project.settings",
    "app.open-settings",
    "app.open-keybindings",
    "app.licences",
    "app.quit",
];

/// The source folders scanned for id literals, relative to the workspace root (§4.6).
const SCANNED_SOURCES: [&str; 2] = ["crates/va-ui/src", "apps/visual-assembler/src"];

/// The folder of the command registry and the file that defines it (§2, §5.12).
const SHELL_DIR: &str = "crates/va-ui/src/shell";

/// The registry file; its existence makes the registry's titles required.
const REGISTRY_FILE: &str = "crates/va-ui/src/shell/commands.rs";

/// The prefixes of the ids this test checks, and of every id the root resources own
/// (architecture §5).
const UI_PREFIXES: [&str; 2] = ["ui-", "cmd-"];

/// Every namespace of the root resources; a CPU defines none of them.
const ROOT_PREFIXES: [&str; 3] = ["ui-", "cmd-", "diag-cfg-"];

/// Index of `commands.ftl` in `ui_resources`.
const COMMANDS_FILE: usize = 1;

/// The workspace root.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf()
}

/// The title id of a command: `cmd-` then the id with `.` mapped to `-` (`-` stays), so
/// `edit.toggle-comment` gives `cmd-edit-toggle-comment` (§4.6).
fn title_id(command: &str) -> String {
    format!("cmd-{}", command.replace('.', "-"))
}

/// The ids (messages, and terms with their leading `-`) that `source` defines: its column-0
/// `identifier =` lines. Exact for valid Fluent, whose entries start at column 0 and whose
/// continuation lines never start with an identifier followed by `=`;
/// `crates/va-workspace/tests/i18n_ui.rs` checks the root resources against the parser.
fn defined_ids(source: &str) -> Vec<String> {
    source
        .lines()
        .filter_map(|line| {
            let body = line.strip_prefix('-').unwrap_or(line);
            if !body.starts_with(|c: char| c.is_ascii_alphabetic()) {
                return None;
            }
            let end = body
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
                .unwrap_or(body.len());
            body[end..]
                .trim_start()
                .starts_with('=')
                .then(|| line[..end + line.len() - body.len()].to_owned())
        })
        .collect()
}

/// The root ids of `lang`, per `ui_resources` file.
fn root_ids(lang: Lang) -> Vec<BTreeSet<String>> {
    ui_resources(lang)
        .iter()
        .map(|source| defined_ids(source).into_iter().collect())
        .collect()
}

/// Every `.rs` file under `dir`, recursively, sorted.
fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(folder) = pending.pop() {
        for entry in fs::read_dir(&folder).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

/// The string literals of `text` whose whole content is an id with one of `prefixes` followed
/// by `[a-z0-9-]+`, i.e. `"ui-find-replace-all"`, with their 1-based line.
fn literal_ids(text: &str, prefixes: &[&str]) -> Vec<(String, usize)> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut index = 0;
    while let Some(offset) = text[index..].find('"') {
        let open = index + offset;
        index = open + 1;
        let rest = &text[open + 1..];
        let Some(prefix) = prefixes.iter().find(|prefix| rest.starts_with(**prefix)) else {
            continue;
        };
        let body = rest[prefix.len()..]
            .bytes()
            .take_while(|&b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            .count();
        let close = open + 1 + prefix.len() + body;
        if body > 0 && bytes.get(close) == Some(&b'"') {
            let line = text[..open].matches('\n').count() + 1;
            found.push((text[open + 1..close].to_owned(), line));
            index = close + 1;
        }
    }
    found
}

/// Every `ui-*`/`cmd-*` literal of the scanned sources → where it appears (`path:line`).
fn referenced_literals() -> BTreeMap<String, Vec<String>> {
    let root = root();
    let mut found: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for dir in SCANNED_SOURCES {
        for path in rust_files(&root.join(dir)) {
            let text = fs::read_to_string(&path).unwrap();
            let shown = path.strip_prefix(&root).unwrap().display().to_string();
            for (id, line) in literal_ids(&text, &UI_PREFIXES) {
                found.entry(id).or_default().push(format!("{shown}:{line}"));
            }
        }
    }
    found
}

/// Whether the registry is live: `commands.ftl` defines a message or `shell/commands.rs`
/// exists. Its titles are then required in both languages.
fn registry_live() -> bool {
    root().join(REGISTRY_FILE).is_file()
        || Lang::ALL
            .into_iter()
            .any(|lang| !root_ids(lang)[COMMANDS_FILE].is_empty())
}

#[test]
fn the_scanners_find_ids_and_literals() {
    let source = "# c\nui-a = A\n    .help = x = y\n-term = T\n[one] = no\ncmd-b=B\n  ui-c = no\n";
    assert_eq!(defined_ids(source), ["ui-a", "-term", "cmd-b"]);
    let text =
        "f(\"ui-open\", \"cmd-file-new\");\n\"ui-\" \"ui-X\" \"cmd-a b\" \"diag-x\"\n\"ui-a1-b\"";
    let ids: Vec<(String, usize)> = literal_ids(text, &UI_PREFIXES);
    assert_eq!(
        ids,
        [
            ("ui-open".to_owned(), 1),
            ("cmd-file-new".to_owned(), 1),
            ("ui-a1-b".to_owned(), 3)
        ]
    );
}

#[test]
fn registry_titles_are_distinct_and_well_formed() {
    let titles: BTreeSet<String> = REGISTRY.iter().map(|id| title_id(id)).collect();
    assert_eq!(
        titles.len(),
        REGISTRY.len(),
        "two commands share a title id"
    );
    assert_eq!(title_id("edit.toggle-comment"), "cmd-edit-toggle-comment");
    for id in REGISTRY {
        let (group, action) = id.split_once('.').unwrap();
        for part in [group, action] {
            assert!(
                !part.is_empty()
                    && part
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'),
                "{id} is not <group>.<action> in kebab-case"
            );
        }
    }
}

#[test]
fn every_referenced_id_exists_in_both_languages() {
    let literals = referenced_literals();
    let live = registry_live();
    let mut required: BTreeMap<String, Vec<String>> = literals.clone();
    if live {
        for command in REGISTRY {
            required
                .entry(title_id(command))
                .or_default()
                .push(format!("registry command {command}"));
        }
    }
    let mut missing = Vec::new();
    for lang in Lang::ALL {
        let defined: BTreeSet<String> = root_ids(lang).into_iter().flatten().collect();
        for (id, places) in &required {
            if !defined.contains(id) {
                missing.push(format!(
                    "{}: {id} (used at {})",
                    lang.code(),
                    places.join(", ")
                ));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "ids missing from locales/<lang>/:\n{}",
        missing.join("\n")
    );
    println!(
        "note: {} id literals found; registry titles {}",
        literals.len(),
        if live {
            "required"
        } else {
            "not required yet (no commands.ftl message, no shell/commands.rs)"
        }
    );
}

#[test]
fn no_ui_or_command_id_is_unused() {
    let literals = referenced_literals();
    let titles: BTreeSet<String> = REGISTRY.iter().map(|id| title_id(id)).collect();
    let mut unused = Vec::new();
    for lang in Lang::ALL {
        for id in root_ids(lang).into_iter().flatten() {
            let checked = UI_PREFIXES.iter().any(|prefix| id.starts_with(prefix));
            if checked && !literals.contains_key(&id) && !titles.contains(&id) {
                unused.push(format!("{}: {id}", lang.code()));
            }
        }
    }
    assert!(
        unused.is_empty(),
        "ids no source literal and no registry command uses (a new command goes into REGISTRY \
         of this file too):\n{}",
        unused.join("\n")
    );
}

#[test]
fn english_and_french_define_the_same_ui_and_command_ids() {
    let en = root_ids(Lang::En);
    let fr = root_ids(Lang::Fr);
    for (file, (en_ids, fr_ids)) in en.iter().zip(&fr).enumerate() {
        assert_eq!(en_ids, fr_ids, "resource {file} of ui_resources differs");
    }
}

#[test]
fn the_registry_table_matches_the_registry_source() {
    let root = root();
    if !root.join(REGISTRY_FILE).is_file() {
        println!("note: {REGISTRY_FILE} does not exist yet (WP-F): REGISTRY is the §5.12 table");
        return;
    }
    let mut source = String::new();
    for path in rust_files(&root.join(SHELL_DIR)) {
        source.push_str(&fs::read_to_string(path).unwrap());
    }
    let absent: Vec<&str> = REGISTRY
        .into_iter()
        .filter(|id| !source.contains(&format!("\"{id}\"")))
        .collect();
    assert!(
        absent.is_empty(),
        "commands of REGISTRY that no string literal of {SHELL_DIR}/ names: {absent:?}"
    );
}

// -------------------------------------------------------------------------------------------
// CPU resources
// -------------------------------------------------------------------------------------------

/// A lexer that produces no token.
struct NoSyntax;

impl SyntaxProvider for NoSyntax {
    fn lex_line(&self, _line: &str, state: LineState, _out: &mut Vec<Token>) -> LineState {
        state
    }

    fn keywords(&self) -> &[KeywordInfo] {
        &[]
    }
}

/// An instruction set with no instruction.
struct NoIsa;

impl InstructionSet for NoIsa {
    fn spellings(&self) -> &[&'static str] {
        &[]
    }

    fn describe(
        &self,
        _spelling: &str,
        _size: Option<&str>,
    ) -> Result<InstructionInfo, LookupError> {
        Err(LookupError::Unknown)
    }

    fn ea_timing(&self) -> &[EaTiming] {
        &[]
    }
}

/// A CPU whose Fluent resources are the `.ftl` files of `arch/<id>/locales/<lang>/`.
struct DiskCpu {
    /// The folder name under `arch/`.
    id: &'static str,
    /// Resources of `"en"`, file name order.
    en: &'static [&'static str],
    /// Resources of `"fr"`, file name order.
    fr: &'static [&'static str],
}

impl Architecture for DiskCpu {
    fn id(&self) -> &'static str {
        self.id
    }

    fn syntax(&self) -> &dyn SyntaxProvider {
        &NoSyntax
    }

    fn instruction_set(&self) -> &dyn InstructionSet {
        &NoIsa
    }

    fn registers(&self) -> &'static [RegisterSpec] {
        &[]
    }

    fn diagnostic_codes(&self) -> &'static [DiagCode] {
        &[]
    }

    fn locale_resources(&self, lang: &str) -> &'static [&'static str] {
        match lang {
            "en" => self.en,
            "fr" => self.fr,
            _ => &[],
        }
    }
}

/// The `.ftl` files of `dir`, file name order, leaked so they are `'static` as embedded
/// resources are.
fn leaked_resources(dir: &Path) -> &'static [&'static str] {
    let mut paths: Vec<PathBuf> = match fs::read_dir(dir) {
        Ok(entries) => entries
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "ftl"))
            .collect(),
        Err(_) => Vec::new(),
    };
    paths.sort();
    let sources: Vec<&'static str> = paths
        .iter()
        .map(|path| &*Box::leak(fs::read_to_string(path).unwrap().into_boxed_str()))
        .collect();
    Box::leak(sources.into_boxed_slice())
}

/// Every CPU of the repository that has Fluent resources (`arch/<id>/locales/`).
fn disk_cpus() -> Vec<DiskCpu> {
    let arch = root().join("arch");
    let mut dirs: Vec<PathBuf> = fs::read_dir(&arch)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.join("locales").is_dir())
        .collect();
    dirs.sort();
    dirs.into_iter()
        .map(|dir| DiskCpu {
            id: Box::leak(
                dir.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
                    .into_boxed_str(),
            ),
            en: leaked_resources(&dir.join("locales").join(Lang::En.code())),
            fr: leaked_resources(&dir.join("locales").join(Lang::Fr.code())),
        })
        .collect()
}

#[test]
fn no_cpu_resource_collides_with_the_root_resources() {
    let cpus = disk_cpus();
    assert!(
        !cpus.is_empty(),
        "no arch/*/locales/ folder found under {}",
        root().display()
    );
    for cpu in &cpus {
        for lang in Lang::ALL {
            let root_ids: BTreeSet<String> = root_ids(lang).into_iter().flatten().collect();
            let resources = cpu.locale_resources(lang.code());
            assert!(
                !resources.is_empty(),
                "{} has no {} resource",
                cpu.id,
                lang.code()
            );
            for id in resources.iter().flat_map(|source| defined_ids(source)) {
                let name = id.strip_prefix('-').unwrap_or(&id);
                assert!(
                    !ROOT_PREFIXES.iter().any(|prefix| name.starts_with(prefix)),
                    "{} {}: {id} is in a namespace of the root resources (architecture §5)",
                    cpu.id,
                    lang.code()
                );
                assert!(
                    !root_ids.contains(&id),
                    "{} {}: {id} is also a root id",
                    cpu.id,
                    lang.code()
                );
            }
            if let Err(error) = Localizer::for_ui(lang, Some(cpu)) {
                panic!(
                    "{} {}: the GUI bundle does not build: {error}",
                    cpu.id,
                    lang.code()
                );
            }
        }
    }
}
