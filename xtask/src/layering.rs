//! Layering and source checks run by `cargo xtask ci` (m0-foundations.md §6.3, amended by §17.2,
//! §17.4 and §18.5).
//!
//! - [`check_layering`] enforces the crate map of architecture.md §2 (rules R1–R5 and R7) on the
//!   output of `cargo metadata --format-version 1 --no-deps`, for every dependency kind, and
//!   matches GUI crates by family ([`is_gui_crate`]).
//! - [`scan_sources`] applies the source rules S1 and S3–S9 of m0-foundations.md §17.4, as amended
//!   by §18.5, to the folder of every workspace member: the offline rule (architecture §9.3), the
//!   unsafe allow-list (architecture §9.4), the inclusion rule and the symlink ban. Rule S2 (the
//!   data exemption of this file and its tests) is removed: every file is scanned under every
//!   rule.
//!
//! Threat model (m0-foundations.md §18.1, binding): the scan is a tripwire against *accidental or
//! careless* introduction of network or unsafe code, not a sandbox against a deliberately
//! malicious contributor. Deliberate obfuscation (renamed or aliased built-in macros, raw-identifier
//! attributes, out-of-tree target paths, crate aliasing, input-ignoring macros, `std::r#net`) is an
//! accepted limitation recorded in SECURITY.md; code review, cargo-deny and branch protection cover
//! it.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt, fs, io,
    path::{Component, Path, PathBuf},
};

use anyhow::{Context as _, Result, anyhow};
use serde_json::Value;

use crate::locate;

/// Files allowed to contain `unsafe` and to allow `unsafe_code` (paths relative to the root, `/`).
pub const UNSAFE_ALLOWLIST: &[&str] = &[
    "apps/visual-assembler/src/bench/process_windows.rs",
    "apps/visual-assembler/src/bench/process_linux.rs",
    "apps/visual-assembler/src/bench/platform.rs",
];

/// Files allowed to use network APIs. Empty in M0; the planned loopback entry for `va-gdb`
/// needs Security's review and a `SECURITY.md` record (architecture §10.2).
pub const NET_ALLOWLIST: &[&str] = &[];

/// Files allowed a computed `include!`/`#[path]` target (rule S8). Empty in M0; a future build
/// script that generates code needs an entry here, reviewed like the other allow-lists.
pub const INCLUDE_ALLOWLIST: &[&str] = &[];

/// Network paths matched on the code view with all whitespace removed (rule S7a).
pub const NET_PATHS: &[&str] = &["std::net", "core::net"];

/// Crate roots whose brace groups `<root>::{…}` must not import `net` (rule S7a).
pub const NET_GROUP_ROOTS: &[&str] = &["std", "core"];

/// Whole-word identifiers that reveal network I/O (rule S7b).
pub const NET_IDENTIFIERS: &[&str] = &[
    "TcpStream",
    "TcpListener",
    "UdpSocket",
    "ToSocketAddrs",
    "to_socket_addrs",
    "lookup_host",
];

/// Lint-level attributes allowed to name `unsafe_code` (rule S6b).
pub const UNSAFE_CODE_GUARDS: &[&str] = &["deny", "forbid"];

/// Folder under the root that is never scanned, besides the cargo target folder (rule S3).
pub const ROOT_TARGET_DIR: &str = "target";

/// Families of the GUI stack (rule R3, m0-foundations.md §18.5): a direct dependency whose
/// normalized name ([`normalized_crate_name`]) starts with one of these is a GUI crate, e.g.
/// `egui_extras`, `epaint`, `wgpu-hal` or `accesskit_winit`.
pub const GUI_CRATE_FAMILIES: &[&str] = &[
    "egui",
    "eframe",
    "epaint",
    "emath",
    "ecolor",
    "wgpu",
    "naga",
    "winit",
    "glutin",
    "glow",
    "accesskit",
];

/// GUI crates matched by their whole normalized name rather than by family (rule R3).
pub const GUI_CRATES_EXACT: &[&str] = &["raw-window-handle"];

/// The only package allowed to depend on any GUI crate.
pub const GUI_APP: &str = "visual-assembler";

/// The UI crate, allowed to depend on [`VA_UI_GUI_CRATES`] only.
pub const VA_UI: &str = "va-ui";

/// The exact GUI crates `va-ui` may depend on (compared by normalized name).
pub const VA_UI_GUI_CRATES: &[&str] = &["egui", "egui_kittest"];

/// Rule name of a forbidden internal dependency or a package without a rule.
pub const RULE_LAYERING: &str = "layering";

/// Rule name of a GUI crate used outside the GUI packages.
pub const RULE_GUI: &str = "gui-crate";

/// Rule name of a network API in a source file (rule S7).
pub const RULE_NETWORK: &str = "network";

/// Rule name of `unsafe`, or of `unsafe_code` outside `deny`/`forbid`, outside the allow-list
/// (rule S6).
pub const RULE_UNSAFE: &str = "unsafe";

/// Rule name of an `include!` or `#[path = …]` whose target is not a plain `.rs` literal (rule S8).
pub const RULE_INCLUDE: &str = "include";

/// Rule name of a symbolic link under a scanned folder (rule S4).
pub const RULE_SYMLINK: &str = "symlink";

/// Rule name of a `cargo metadata` document that cannot be checked.
pub const RULE_METADATA: &str = "metadata";

/// One broken rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// Short rule name, one of the `RULE_*` constants.
    pub rule: &'static str,
    /// Package name, `path:line` or `path`, relative to the workspace root.
    pub location: String,
    /// Human-readable explanation.
    pub detail: String,
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}: {}", self.rule, self.location, self.detail)
    }
}

// ---------------------------------------------------------------------------------------------
// Layering (§17.2)
// ---------------------------------------------------------------------------------------------

/// Internal dependencies a package may have (§17.2: every package lists what it may use).
struct Allowed {
    /// These workspace packages.
    names: &'static [&'static str],
    /// Plus every CPU crate `va-arch-*`.
    arch: bool,
}

impl Allowed {
    /// Tells whether `dependency` (a workspace package) is allowed.
    fn permits(&self, dependency: &str) -> bool {
        self.names.contains(&dependency) || (self.arch && is_arch_package(dependency))
    }
}

/// The non-UI library crates the apps may use.
const LIBRARY_CRATES: &[&str] = &[
    "va-core",
    "va-editor",
    "va-lang",
    "va-debug",
    "va-workspace",
];

/// Returns the layering rule of `package` (§17.2 table), or `None` when the table has no rule.
fn allowed_internal(package: &str) -> Option<Allowed> {
    const GUI_APP_DEPS: &[&str] = &[
        "va-core",
        "va-editor",
        "va-lang",
        "va-debug",
        "va-workspace",
        "va-ui",
    ];
    let only = |names| Some(Allowed { names, arch: false });
    match package {
        "va-core" | "xtask" => only(&[]),
        "va-editor" | "va-debug" | "va-workspace" => only(&["va-core"]),
        "va-lang" => only(&["va-core", "va-editor"]),
        "va-ui" => only(LIBRARY_CRATES),
        "va" => Some(Allowed {
            names: LIBRARY_CRATES,
            arch: true,
        }),
        "visual-assembler" => Some(Allowed {
            names: GUI_APP_DEPS,
            arch: true,
        }),
        _ if is_arch_package(package) => only(&["va-core"]),
        _ => None,
    }
}

/// Tells whether `package` is a CPU crate `va-arch-<cpu>`.
fn is_arch_package(package: &str) -> bool {
    package
        .strip_prefix("va-arch-")
        .is_some_and(|cpu| !cpu.is_empty())
}

/// Normalizes a crate name for comparison: crates.io treats `-` and `_` as the same character
/// and names as case-insensitive, so `Egui_Extras` becomes `egui-extras`.
pub fn normalized_crate_name(name: &str) -> String {
    name.to_ascii_lowercase().replace('_', "-")
}

/// Tells whether `dependency` belongs to `set`, comparing normalized names.
fn in_set(set: &[&str], dependency: &str) -> bool {
    let dependency = normalized_crate_name(dependency);
    set.iter()
        .any(|name| normalized_crate_name(name) == dependency)
}

/// Tells whether the crate `name` belongs to the GUI stack (rule R3, m0-foundations.md §18.5):
/// its normalized name starts with one of [`GUI_CRATE_FAMILIES`] or equals one of
/// [`GUI_CRATES_EXACT`].
pub fn is_gui_crate(name: &str) -> bool {
    let name = normalized_crate_name(name);
    GUI_CRATE_FAMILIES
        .iter()
        .any(|family| name.starts_with(&normalized_crate_name(family)))
        || in_set(GUI_CRATES_EXACT, &name)
}

/// Describes the dependency kind of a `cargo metadata` dependency entry.
fn kind_of(dependency: &Value) -> &str {
    dependency
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("normal")
}

/// Returns the workspace packages of a `cargo metadata` document, or a metadata violation.
fn workspace_packages(metadata: &Value) -> Result<Vec<&Value>, Violation> {
    let malformed = |detail: &str| Violation {
        rule: RULE_METADATA,
        location: "cargo metadata".to_owned(),
        detail: detail.to_owned(),
    };
    let packages = metadata
        .get("packages")
        .and_then(Value::as_array)
        .ok_or_else(|| malformed("no `packages` array"))?;
    let members: Option<Vec<&str>> = match metadata.get("workspace_members") {
        None => None,
        Some(value) => Some(
            value
                .as_array()
                .ok_or_else(|| malformed("`workspace_members` is not an array"))?
                .iter()
                .map(|id| {
                    id.as_str()
                        .ok_or_else(|| malformed("`workspace_members` holds a non-string id"))
                })
                .collect::<Result<_, _>>()?,
        ),
    };
    Ok(packages
        .iter()
        .filter(|package| match &members {
            None => true,
            Some(ids) => package
                .get("id")
                .and_then(Value::as_str)
                .is_some_and(|id| ids.contains(&id)),
        })
        .collect())
}

/// Checks rules R1–R5 and R7 on `cargo metadata --format-version 1 --no-deps` output.
///
/// A dependency is internal when its name is a workspace package. Every dependency kind
/// (normal, dev, build) is checked, under the real package name (a `package =` rename does not
/// hide it). A direct GUI dependency ([`is_gui_crate`]) is allowed in `visual-assembler`, and
/// exactly `egui`/`egui_kittest` in `va-ui`. A malformed document yields a single `metadata`
/// violation.
pub fn check_layering(metadata: &Value) -> Vec<Violation> {
    let packages = match workspace_packages(metadata) {
        Ok(packages) => packages,
        Err(violation) => return vec![violation],
    };
    let mut names = Vec::with_capacity(packages.len());
    let mut violations = Vec::new();
    for package in &packages {
        match package.get("name").and_then(Value::as_str) {
            Some(name) => names.push(name),
            None => violations.push(Violation {
                rule: RULE_METADATA,
                location: "cargo metadata".to_owned(),
                detail: "a workspace package has no string `name`".to_owned(),
            }),
        }
    }
    for package in &packages {
        let Some(name) = package.get("name").and_then(Value::as_str) else {
            continue;
        };
        let rule = allowed_internal(name);
        if rule.is_none() {
            violations.push(Violation {
                rule: RULE_LAYERING,
                location: name.to_owned(),
                detail: format!(
                    "no layering rule for {name} (add it to xtask/src/layering.rs and architecture.md §2)"
                ),
            });
        }
        let dependencies = match package.get("dependencies").and_then(Value::as_array) {
            Some(dependencies) => dependencies,
            None => {
                violations.push(Violation {
                    rule: RULE_METADATA,
                    location: name.to_owned(),
                    detail: "package has no `dependencies` array".to_owned(),
                });
                continue;
            }
        };
        for dependency in dependencies {
            let Some(dep_name) = dependency.get("name").and_then(Value::as_str) else {
                violations.push(Violation {
                    rule: RULE_METADATA,
                    location: name.to_owned(),
                    detail: "a dependency has no string `name`".to_owned(),
                });
                continue;
            };
            let kind = kind_of(dependency);
            if names.contains(&dep_name)
                && let Some(rule) = &rule
                && !rule.permits(dep_name)
            {
                violations.push(Violation {
                    rule: RULE_LAYERING,
                    location: name.to_owned(),
                    detail: format!(
                        "depends on `{dep_name}` ({kind}), which its layering rule forbids"
                    ),
                });
            }
            if is_gui_crate(dep_name) && !gui_dependency_allowed(name, dep_name) {
                violations.push(Violation {
                    rule: RULE_GUI,
                    location: name.to_owned(),
                    detail: format!(
                        "depends on GUI crate `{dep_name}` ({kind}); the GUI crates ({}*, {}) are for {GUI_APP} only, plus exactly egui/egui_kittest in {VA_UI}",
                        GUI_CRATE_FAMILIES.join("*, "),
                        GUI_CRATES_EXACT.join(", ")
                    ),
                });
            }
        }
    }
    violations
}

/// Tells whether `package` may depend on the GUI crate `dependency` (rule R3, §18.5):
/// `visual-assembler` may use any, `va-ui` exactly [`VA_UI_GUI_CRATES`], no other package any.
fn gui_dependency_allowed(package: &str, dependency: &str) -> bool {
    package == GUI_APP || (package == VA_UI && in_set(VA_UI_GUI_CRATES, dependency))
}

// ---------------------------------------------------------------------------------------------
// Code view (rule S5)
// ---------------------------------------------------------------------------------------------

/// A comment or literal found in source text.
struct Skip {
    /// Bytes replaced by spaces in the code view.
    blank: (usize, usize),
    /// Offset where scanning resumes.
    resume: usize,
}

/// Returns the *code view* of `text` (rule S5): every comment (nested block comments included)
/// and the contents of every string, raw-string, byte-string and character literal are replaced
/// by spaces, byte for byte. A shebang line that rustc strips ([`shebang_line`]) is blanked as a
/// comment, so a quote on it opens nothing. Line breaks are kept and the literal delimiters stay,
/// so line numbers and byte offsets are the same as in `text`.
pub fn code_view(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut view = bytes.to_vec();
    let mut index = 0;
    if let Some((start, end)) = shebang_line(text) {
        blank(&mut view, start, end);
        index = end;
    }
    while index < bytes.len() {
        match skip_at(text, index) {
            Some(skip) => {
                let (start, end) = skip.blank;
                blank(&mut view, start, end);
                index = skip.resume;
            }
            None => index += 1,
        }
    }
    // Every blanked range starts and ends next to an ASCII delimiter (or at the end of the
    // shebang line or of the text), so whole characters are replaced and the bytes stay UTF-8.
    // The lossy fallback only guards against a scanner bug.
    String::from_utf8(view)
        .unwrap_or_else(|error| String::from_utf8_lossy(error.as_bytes()).into_owned())
}

/// Replaces the bytes `start..end` of `view` by spaces, keeping line breaks.
fn blank(view: &mut [u8], start: usize, end: usize) {
    for byte in view.get_mut(start..end).unwrap_or_default() {
        if *byte != b'\n' && *byte != b'\r' {
            *byte = b' ';
        }
    }
}

/// The UTF-8 byte-order mark, which rustc removes before it looks for a shebang.
const BOM: &str = "\u{feff}";

/// Returns the byte range `(start, end)` of the shebang line that rustc strips from the source
/// `text`, or `None` when it has none (rule S5, m0-foundations.md §18.5).
///
/// Same rule as `rustc_lexer::strip_shebang`: after an optional UTF-8 byte-order mark, the text
/// starts with `#!`, and the first token after `#!` is not `[`. Whitespace (rustc's set,
/// [`is_rustc_whitespace`]) and line or block comments that are not doc comments are skipped
/// while looking for that token, across lines. The range then covers the whole first line, from
/// `#!` to the line feed (excluded). So `#![deny(unsafe_code)]`, `#! [allow(…)]` and `#!` with a
/// `[` on a later line are inner attributes, i.e. code.
pub fn shebang_line(text: &str) -> Option<(usize, usize)> {
    let start = if text.starts_with(BOM) { BOM.len() } else { 0 };
    let body = &text[start..];
    let mut rest = body.strip_prefix("#!")?;
    loop {
        rest = rest.trim_start_matches(is_rustc_whitespace);
        if let Some(comment) = rest.strip_prefix("//") {
            // `//!` and `///` (not `////`) are doc comments: a token that is not `[`.
            let doc = comment.starts_with('!')
                || (comment.starts_with('/') && !comment[1..].starts_with('/'));
            if doc {
                break;
            }
            rest = comment.find('\n').map_or("", |offset| &comment[offset..]);
        } else if let Some(comment) = rest.strip_prefix("/*") {
            // `/*!` and `/**` (not `/***` or `/**/`) are doc comments.
            let doc = comment.starts_with('!')
                || (comment.starts_with('*') && !comment[1..].starts_with(['*', '/']));
            if doc {
                break;
            }
            rest = &comment[block_comment_end(comment.as_bytes(), 0)..];
        } else {
            break;
        }
    }
    if rest.starts_with('[') {
        return None;
    }
    let end = body.find('\n').map_or(text.len(), |offset| start + offset);
    Some((start, end))
}

/// Tells whether rustc's lexer treats `c` as whitespace (`rustc_lexer::is_whitespace`: the
/// Unicode `Pattern_White_Space` set, which differs from [`char::is_whitespace`]).
pub fn is_rustc_whitespace(c: char) -> bool {
    matches!(
        c,
        '\u{0009}'
            | '\u{000A}'
            | '\u{000B}'
            | '\u{000C}'
            | '\u{000D}'
            | '\u{0020}'
            | '\u{0085}'
            | '\u{200E}'
            | '\u{200F}'
            | '\u{2028}'
            | '\u{2029}'
    )
}

/// Returns the comment or literal starting at `index`, if any.
fn skip_at(text: &str, index: usize) -> Option<Skip> {
    let bytes = text.as_bytes();
    let next = bytes.get(index + 1).copied();
    match (bytes[index], next) {
        (b'/', Some(b'/')) => {
            let end = bytes[index..]
                .iter()
                .position(|&byte| byte == b'\n')
                .map_or(bytes.len(), |offset| index + offset);
            Some(Skip {
                blank: (index, end),
                resume: end,
            })
        }
        (b'/', Some(b'*')) => {
            let end = block_comment_end(bytes, index + 2);
            Some(Skip {
                blank: (index, end),
                resume: end,
            })
        }
        (b'"', _) => {
            let end = string_end(bytes, index + 1);
            let content_end = if end > index + 1 && bytes.get(end - 1) == Some(&b'"') {
                end - 1
            } else {
                end
            };
            Some(Skip {
                blank: (index + 1, content_end),
                resume: end,
            })
        }
        (b'r', Some(b'"' | b'#')) if raw_prefix_allowed(bytes, index) => raw_string(bytes, index),
        (b'\'', _) => {
            let end = char_literal_end(text, index);
            (end > index + 1).then_some(Skip {
                blank: (index + 1, end - 1),
                resume: end,
            })
        }
        _ => None,
    }
}

/// Tells whether a byte can belong to an identifier (non-ASCII bytes count as identifier bytes).
fn is_ident_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || !byte.is_ascii()
}

/// Tells whether the `r` at `index` can start a raw string: it starts a token, or follows a
/// `b`/`c` prefix that starts one. `ident"` is not Rust, so this only avoids misreading an
/// identifier that ends with `r` before `#`.
fn raw_prefix_allowed(bytes: &[u8], index: usize) -> bool {
    let before = |back: usize| index.checked_sub(back).map(|at| bytes[at]);
    match before(1) {
        None => true,
        Some(b'b' | b'c') => !before(2).is_some_and(is_ident_byte),
        Some(byte) => !is_ident_byte(byte),
    }
}

/// Returns the raw string literal whose `r` is at `index`, or `None` for a raw identifier
/// (`r#name`).
fn raw_string(bytes: &[u8], index: usize) -> Option<Skip> {
    let hashes = bytes[index + 1..]
        .iter()
        .take_while(|&&byte| byte == b'#')
        .count();
    let quote = index + 1 + hashes;
    if bytes.get(quote) != Some(&b'"') {
        return None;
    }
    let end = raw_string_end(bytes, index + 1);
    let terminator = hashes + 1;
    let terminated = end >= quote + 1 + terminator && bytes.get(end - terminator) == Some(&b'"');
    let content_end = if terminated { end - terminator } else { end };
    Some(Skip {
        blank: (quote + 1, content_end),
        resume: end,
    })
}

/// Returns the offset just past the `"` closing a string literal whose body starts at `index`.
fn string_end(bytes: &[u8], mut index: usize) -> usize {
    while let Some(&byte) = bytes.get(index) {
        match byte {
            b'\\' => index += 2,
            b'"' => return index + 1,
            _ => index += 1,
        }
    }
    bytes.len()
}

/// Returns the offset just past a raw string literal whose `#`s or `"` start at `index` (just
/// after the `r`). When no `"` follows the `#`s, it is not a raw string: returns `index`.
fn raw_string_end(bytes: &[u8], index: usize) -> usize {
    let hashes = bytes[index..]
        .iter()
        .take_while(|&&byte| byte == b'#')
        .count();
    let body = index + hashes;
    if bytes.get(body) != Some(&b'"') {
        return index;
    }
    let mut terminator = vec![b'"'];
    terminator.resize(hashes + 1, b'#');
    bytes[body + 1..]
        .windows(terminator.len())
        .position(|window| window == terminator.as_slice())
        .map_or(bytes.len(), |offset| body + 1 + offset + terminator.len())
}

/// Returns the offset just past a character literal starting with the `'` at `index`, or just
/// past the `'` when it starts a lifetime or a label instead.
fn char_literal_end(text: &str, index: usize) -> usize {
    let bytes = text.as_bytes();
    if bytes.get(index + 1) == Some(&b'\\') {
        return bytes
            .get(index + 3..)
            .and_then(|rest| rest.iter().position(|&byte| byte == b'\''))
            .map_or(bytes.len(), |offset| index + 3 + offset + 1);
    }
    let width = text
        .get(index + 1..)
        .and_then(|rest| rest.chars().next())
        .map_or(0, char::len_utf8);
    if width > 0 && bytes.get(index + 1 + width) == Some(&b'\'') {
        index + 2 + width
    } else {
        index + 1
    }
}

/// Returns the offset just past the `*/` closing a block comment whose body starts at `index`.
/// Block comments nest, as in Rust.
fn block_comment_end(bytes: &[u8], mut index: usize) -> usize {
    let mut depth = 1_usize;
    while index < bytes.len() {
        match (bytes[index], bytes.get(index + 1)) {
            (b'/', Some(b'*')) => {
                depth += 1;
                index += 2;
            }
            (b'*', Some(b'/')) => {
                depth -= 1;
                index += 2;
                if depth == 0 {
                    return index;
                }
            }
            _ => index += 1,
        }
    }
    bytes.len()
}

// ---------------------------------------------------------------------------------------------
// Token helpers on the code view
// ---------------------------------------------------------------------------------------------

/// Tells whether `c` can belong to an identifier.
fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Returns the offsets of every whole-word occurrence of `word` in `view`.
fn word_offsets(view: &str, word: &str) -> Vec<usize> {
    view.match_indices(word)
        .map(|(offset, _)| offset)
        .filter(|&offset| {
            let before = view[..offset].chars().next_back();
            let after = view[offset + word.len()..].chars().next();
            !before.is_some_and(is_ident_char) && !after.is_some_and(is_ident_char)
        })
        .collect()
}

/// Returns the first offset at or after `index` that is not whitespace.
fn skip_whitespace(view: &str, index: usize) -> usize {
    view.get(index..)
        .and_then(|rest| {
            rest.char_indices()
                .find(|(_, c)| !c.is_whitespace())
                .map(|(offset, _)| index + offset)
        })
        .unwrap_or(view.len())
}

/// Returns the last character before `index` that is not whitespace.
fn previous_non_whitespace(view: &str, index: usize) -> Option<char> {
    view[..index].trim_end().chars().next_back()
}

/// Returns the identifier that ends just before `index`, whitespace skipped.
fn identifier_before(view: &str, index: usize) -> &str {
    let head = view[..index].trim_end();
    let start = head
        .char_indices()
        .rev()
        .take_while(|(_, c)| is_ident_char(*c))
        .last()
        .map_or(head.len(), |(offset, _)| offset);
    &head[start..]
}

/// Returns the offset of the delimiter closing the group opened at `open` in the code view, or
/// the length of the view when it is unclosed. Literal contents are blank in the view, so plain
/// counting is exact.
fn group_end(view: &str, open: usize) -> usize {
    let bytes = view.as_bytes();
    let mut depth = 0_usize;
    for (offset, &byte) in bytes.iter().enumerate().skip(open) {
        match byte {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return offset;
                }
            }
            _ => {}
        }
    }
    bytes.len()
}

/// Returns the offset of the delimiter opening the innermost group (of any kind) that encloses
/// `index` in the code view.
fn enclosing_group(view: &str, index: usize) -> Option<usize> {
    let bytes = view.as_bytes();
    let mut depth = 0_usize;
    for offset in (0..index).rev() {
        match bytes[offset] {
            b')' | b']' | b'}' => depth += 1,
            b'(' | b'[' | b'{' => {
                if depth == 0 {
                    return Some(offset);
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    None
}

/// Returns the byte range of every attribute `#[...]` or `#![...]` of the code view, from its
/// `#` to its matching `]` included. Whitespace may separate `#`, `!` and `[`.
fn attribute_spans(view: &str) -> Vec<(usize, usize)> {
    let bytes = view.as_bytes();
    let mut spans = Vec::new();
    for (start, _) in view.match_indices('#') {
        let mut index = skip_whitespace(view, start + 1);
        if bytes.get(index) == Some(&b'!') {
            index = skip_whitespace(view, index + 1);
        }
        if bytes.get(index) == Some(&b'[') {
            spans.push((start, (group_end(view, index) + 1).min(bytes.len())));
        }
    }
    spans
}

// ---------------------------------------------------------------------------------------------
// Source rules S6–S8 on one file
// ---------------------------------------------------------------------------------------------

/// One pattern found at a byte offset of a file.
struct Hit {
    /// Byte offset in the file.
    offset: usize,
    /// Rule name.
    rule: &'static str,
    /// What was found, quoted for the message.
    what: String,
}

/// Rules S7: network paths, brace groups importing `net`, network identifiers.
fn network_hits(view: &str, hits: &mut Vec<Hit>) {
    let mut compact = String::with_capacity(view.len());
    let mut origin = Vec::with_capacity(view.len());
    for (offset, c) in view.char_indices().filter(|(_, c)| !c.is_whitespace()) {
        compact.push(c);
        origin.extend((0..c.len_utf8()).map(|byte| offset + byte));
    }
    for path in NET_PATHS {
        for (index, _) in compact.match_indices(path) {
            hits.push(Hit {
                offset: origin[index],
                rule: RULE_NETWORK,
                what: (*path).to_owned(),
            });
        }
    }
    for root in NET_GROUP_ROOTS {
        for offset in word_offsets(view, root) {
            let colons = skip_whitespace(view, offset + root.len());
            if !view[colons..].starts_with("::") {
                continue;
            }
            let open = skip_whitespace(view, colons + 2);
            if view.as_bytes().get(open) != Some(&b'{') {
                continue;
            }
            let close = group_end(view, open);
            for net in word_offsets(&view[..close], "net") {
                if net > open && is_net_segment(view, net) {
                    hits.push(Hit {
                        offset: net,
                        rule: RULE_NETWORK,
                        what: format!("{root}::{{… net …}}"),
                    });
                }
            }
        }
    }
    for identifier in NET_IDENTIFIERS {
        for offset in word_offsets(view, identifier) {
            hits.push(Hit {
                offset,
                rule: RULE_NETWORK,
                what: (*identifier).to_owned(),
            });
        }
    }
}

/// Tells whether the word `net` at `offset` is a path segment of a brace group: it follows `{`
/// or `,`, and `::`, `,`, `}` or `as` follows it (whitespace ignored).
fn is_net_segment(view: &str, offset: usize) -> bool {
    let after = skip_whitespace(view, offset + "net".len());
    let rest = &view[after..];
    let renamed = rest
        .strip_prefix("as")
        .is_some_and(|tail| tail.chars().next().is_none_or(|c| !is_ident_char(c)));
    matches!(previous_non_whitespace(view, offset), Some('{' | ','))
        && (rest.starts_with("::") || rest.starts_with(',') || rest.starts_with('}') || renamed)
}

/// Rule S6: `unsafe` anywhere, `unsafe_code` outside a `deny(…)`/`forbid(…)` group.
fn unsafe_hits(view: &str, hits: &mut Vec<Hit>) {
    for offset in word_offsets(view, "unsafe") {
        hits.push(Hit {
            offset,
            rule: RULE_UNSAFE,
            what: "unsafe".to_owned(),
        });
    }
    for offset in word_offsets(view, "unsafe_code") {
        let guarded = enclosing_group(view, offset).is_some_and(|open| {
            view.as_bytes()[open] == b'('
                && UNSAFE_CODE_GUARDS.contains(&identifier_before(view, open))
        });
        if !guarded {
            hits.push(Hit {
                offset,
                rule: RULE_UNSAFE,
                what: "unsafe_code (outside deny(…)/forbid(…))".to_owned(),
            });
        }
    }
}

/// Rule S8: `include!(…)` and `#[path = …]` take a plain `.rs` string literal without `..`.
fn include_hits(text: &str, view: &str, hits: &mut Vec<Hit>) {
    let bytes = view.as_bytes();
    for offset in word_offsets(view, "include") {
        let bang = skip_whitespace(view, offset + "include".len());
        if bytes.get(bang) != Some(&b'!') {
            continue;
        }
        let open = skip_whitespace(view, bang + 1);
        let accepted = matches!(bytes.get(open), Some(b'(' | b'[' | b'{'))
            && literal_argument_is_plain(text, view, open);
        if !accepted {
            hits.push(Hit {
                offset,
                rule: RULE_INCLUDE,
                what: "include!".to_owned(),
            });
        }
    }
    for (start, end) in attribute_spans(view) {
        for relative in word_offsets(&view[start..end], "path") {
            let offset = start + relative;
            if !matches!(previous_non_whitespace(view, offset), Some('[' | '(' | ',')) {
                continue;
            }
            let equals = skip_whitespace(view, offset + "path".len());
            if bytes.get(equals) != Some(&b'=') || bytes.get(equals + 1) == Some(&b'=') {
                continue;
            }
            let value = skip_whitespace(view, equals + 1);
            let accepted = plain_rs_literal(text, value).is_some();
            if !accepted {
                hits.push(Hit {
                    offset,
                    rule: RULE_INCLUDE,
                    what: "#[path = …]".to_owned(),
                });
            }
        }
    }
}

/// Tells whether the macro group opened at `open` holds exactly one plain `.rs` string literal
/// (an optional trailing comma aside).
fn literal_argument_is_plain(text: &str, view: &str, open: usize) -> bool {
    let bytes = view.as_bytes();
    let close = group_end(view, open);
    let argument = skip_whitespace(view, open + 1);
    let Some(literal_end) = plain_rs_literal(text, argument) else {
        return false;
    };
    let mut next = skip_whitespace(view, literal_end);
    if bytes.get(next) == Some(&b',') {
        next = skip_whitespace(view, next + 1);
    }
    next == close && close < bytes.len()
}

/// When a plain string literal `"…"` starts at `index` of `text` and names an allowed include
/// target, returns the offset just past it.
///
/// Allowed: ends in `.rs`, no `..` component, not absolute, no escape sequence (an escape could
/// spell `..` in disguise).
pub fn plain_rs_literal(text: &str, index: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    if bytes.get(index) != Some(&b'"') {
        return None;
    }
    let end = string_end(bytes, index + 1);
    if end <= index + 1 || bytes.get(end - 1) != Some(&b'"') {
        return None;
    }
    let content = text.get(index + 1..end - 1)?;
    include_target_allowed(content).then_some(end)
}

/// Tells whether `target` (the value of an `include!` or `#[path]` literal) is allowed by S8: it
/// ends in `.rs` ignoring ASCII case (m0-foundations.md §18.5), has no backslash, is not
/// absolute and has no `..` segment. Such a file is scanned by the walk ([`has_rs_suffix`]).
pub fn include_target_allowed(target: &str) -> bool {
    let absolute = target.starts_with('/') || target.as_bytes().get(1) == Some(&b':');
    has_rs_suffix(target.as_bytes())
        && !target.contains('\\')
        && !absolute
        && !target.split('/').any(|segment| segment == "..")
}

/// Tells whether a file name (or include target) ends in `.rs`, ignoring ASCII case (rules S8
/// and S9, m0-foundations.md §18.5): `x.rs`, `x.RS` and `x.Rs` all do, on every OS.
pub fn has_rs_suffix(name: &[u8]) -> bool {
    const SUFFIX: &[u8] = b".rs";
    name.len() >= SUFFIX.len() && name[name.len() - SUFFIX.len()..].eq_ignore_ascii_case(SUFFIX)
}

/// Line numbers (1-based) of byte offsets.
struct LineIndex {
    /// Offsets of every `\n`.
    newlines: Vec<usize>,
}

impl LineIndex {
    /// Indexes `text`.
    fn new(text: &str) -> Self {
        Self {
            newlines: text.match_indices('\n').map(|(offset, _)| offset).collect(),
        }
    }

    /// Returns the 1-based line of `offset`.
    fn line_of(&self, offset: usize) -> usize {
        self.newlines.partition_point(|&newline| newline < offset) + 1
    }
}

/// Order of the rules inside one line of the report.
fn rule_rank(rule: &str) -> u8 {
    match rule {
        RULE_NETWORK => 0,
        RULE_UNSAFE => 1,
        _ => 2,
    }
}

/// Builds the message of a rule from the patterns found on one line.
fn detail_of(rule: &str, found: &[String]) -> String {
    match rule {
        RULE_NETWORK => format!(
            "network API `{}` is forbidden (offline product, architecture §9.3)",
            found.join("`, `")
        ),
        RULE_UNSAFE => format!(
            "`{}` outside the unsafe allow-list (architecture §9.4)",
            found.join("`, `")
        ),
        _ => format!(
            "`{}` must take a plain string literal that ends in `.rs` (any ASCII case) and has no `..` (m0-foundations.md §17.4 S8, §18.5)",
            found.join("`, `")
        ),
    }
}

/// Scans the text of one `.rs` file (rules S5–S8). `relative_path` uses `/` and is relative to
/// the root; it selects the allow-lists. No file is exempt (rule S2 is removed, §18.5): the
/// checker and its tests are scanned like any other file.
///
/// Patterns are matched on the [`code_view`]: comments, literal contents and a shebang line never
/// match. One violation is reported per line and rule, listing every pattern found on that line.
pub fn scan_text(relative_path: &str, text: &str) -> Vec<Violation> {
    let view = code_view(text);
    let mut hits = Vec::new();
    if !NET_ALLOWLIST.contains(&relative_path) {
        network_hits(&view, &mut hits);
    }
    if !UNSAFE_ALLOWLIST.contains(&relative_path) {
        unsafe_hits(&view, &mut hits);
    }
    if !INCLUDE_ALLOWLIST.contains(&relative_path) {
        include_hits(text, &view, &mut hits);
    }

    let lines = LineIndex::new(text);
    let mut grouped: BTreeMap<(usize, u8), (&'static str, Vec<String>)> = BTreeMap::new();
    hits.sort_by_key(|hit| hit.offset);
    for hit in hits {
        let entry = grouped
            .entry((lines.line_of(hit.offset), rule_rank(hit.rule)))
            .or_insert_with(|| (hit.rule, Vec::new()));
        if !entry.1.contains(&hit.what) {
            entry.1.push(hit.what);
        }
    }
    grouped
        .into_iter()
        .map(|((line, _), (rule, found))| Violation {
            rule,
            location: format!("{relative_path}:{line}"),
            detail: detail_of(rule, &found),
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Walking the member folders (rules S1–S4, S9)
// ---------------------------------------------------------------------------------------------

/// What the walk found under the member folders.
#[derive(Default)]
struct Found {
    /// Regular files whose name ends in `.rs`, ignoring ASCII case ([`has_rs_suffix`]).
    sources: BTreeSet<PathBuf>,
    /// Symbolic links (files, folders or junctions), never followed.
    links: BTreeSet<PathBuf>,
}

/// Scans the folder of every workspace member (rules S1–S9).
///
/// - `members` are the package folders (parents of each `manifest_path`); tests pass synthetic
///   trees. A file under several members is scanned once.
/// - `target_directory` (cargo's `target_directory`) and `<root>/target` are skipped; every
///   other folder named `target` is scanned (S3).
/// - Any symbolic link is a `symlink` violation and is never followed (S4).
/// - Only `.rs` files are pattern-scanned (S9), through [`scan_text`]. The extension is compared
///   ignoring ASCII case on every OS (`net.RS` is scanned, §18.5), so the result never depends on
///   the host.
///
/// Unreadable folders or files, and non-UTF-8 `.rs` files, are errors whose message names the
/// path; nothing is silently ignored.
pub fn scan_sources(
    root: &Path,
    members: &[PathBuf],
    target_directory: &Path,
) -> io::Result<Vec<Violation>> {
    let skipped = skipped_folders(root, target_directory)?;
    let mut found = Found::default();
    for member in members {
        let meta = fs::symlink_metadata(member).map_err(|error| with_path(error, member))?;
        if meta.file_type().is_symlink() {
            found.links.insert(member.clone());
        } else if meta.is_dir() {
            walk(member, &skipped, &mut found)?;
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("member folder {} is not a folder", member.display()),
            ));
        }
    }
    let mut by_path: BTreeMap<PathBuf, Vec<Violation>> = BTreeMap::new();
    for link in found.links {
        let relative = relative_slash_path(root, &link);
        by_path.insert(
            link,
            vec![Violation {
                rule: RULE_SYMLINK,
                location: relative,
                detail: "symbolic link (or junction) under a member folder; links are never followed (m0-foundations.md §17.4 S4)".to_owned(),
            }],
        );
    }
    for file in found.sources {
        let text = fs::read_to_string(&file).map_err(|error| with_path(error, &file))?;
        let violations = scan_text(&relative_slash_path(root, &file), &text);
        if !violations.is_empty() {
            by_path.insert(file, violations);
        }
    }
    Ok(by_path.into_values().flatten().collect())
}

/// Returns the canonical forms of the existing folders skipped by the scan (rule S3).
fn skipped_folders(root: &Path, target_directory: &Path) -> io::Result<Vec<PathBuf>> {
    let mut skipped = Vec::new();
    for candidate in [target_directory.to_path_buf(), root.join(ROOT_TARGET_DIR)] {
        match fs::canonicalize(&candidate) {
            Ok(canonical) => {
                if !skipped.contains(&canonical) {
                    skipped.push(canonical);
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(with_path(error, &candidate)),
        }
    }
    Ok(skipped)
}

/// Collects the `.rs` files (any ASCII case) and the symlinks under the real folder `dir`,
/// skipping the folders of [`skipped_folders`]. Uses `fs::symlink_metadata`, so links are never
/// followed.
fn walk(dir: &Path, skipped: &[PathBuf], found: &mut Found) -> io::Result<()> {
    if !skipped.is_empty() {
        let canonical = fs::canonicalize(dir).map_err(|error| with_path(error, dir))?;
        if skipped.contains(&canonical) {
            return Ok(());
        }
    }
    let entries = fs::read_dir(dir).map_err(|error| with_path(error, dir))?;
    for entry in entries {
        let path = entry.map_err(|error| with_path(error, dir))?.path();
        let meta = fs::symlink_metadata(&path).map_err(|error| with_path(error, &path))?;
        let file_type = meta.file_type();
        if file_type.is_symlink() {
            found.links.insert(path);
        } else if file_type.is_dir() {
            walk(&path, skipped, found)?;
        } else if file_type.is_file()
            && path
                .file_name()
                .is_some_and(|name| has_rs_suffix(name.as_encoded_bytes()))
        {
            found.sources.insert(path);
        }
    }
    Ok(())
}

/// Returns `path` relative to `root` with `/` separators, or the whole path (with `/`) when it
/// lies outside `root`.
fn relative_slash_path(root: &Path, path: &Path) -> String {
    match path.strip_prefix(root) {
        Ok(relative) => relative
            .components()
            .map(|component| component.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/"),
        Err(_) => path.to_string_lossy().replace('\\', "/"),
    }
}

/// Adds the path to an I/O error message.
fn with_path(error: io::Error, path: &Path) -> io::Error {
    io::Error::new(error.kind(), format!("{}: {error}", path.display()))
}

// ---------------------------------------------------------------------------------------------
// Live workspace
// ---------------------------------------------------------------------------------------------

/// Returns the package folder (parent of `manifest_path`) of every workspace member of a
/// `cargo metadata --no-deps` document (rule S1).
pub fn member_folders(metadata: &Value) -> Result<Vec<PathBuf>> {
    let packages = workspace_packages(metadata).map_err(|violation| anyhow!("{violation}"))?;
    packages
        .iter()
        .map(|package| {
            let name = package
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("<unnamed>");
            let manifest = package
                .get("manifest_path")
                .and_then(Value::as_str)
                .with_context(|| {
                    format!("workspace package {name} has no string `manifest_path`")
                })?;
            Path::new(manifest)
                .parent()
                .filter(|folder| folder.components().next_back().is_some_and(is_normal))
                .map(Path::to_path_buf)
                .with_context(|| format!("manifest path {manifest} of {name} has no folder"))
        })
        .collect()
}

/// Tells whether a path component is a plain name.
fn is_normal(component: Component<'_>) -> bool {
    matches!(component, Component::Normal(_))
}

/// Runs both checks on the real workspace at `root`: `cargo metadata` layering, then the source
/// scan of every member folder.
pub fn check_workspace(root: &Path) -> Result<Vec<Violation>> {
    let metadata = locate::cargo_metadata(root)?;
    let mut violations = check_layering(&metadata);
    let members = member_folders(&metadata)?;
    let target_directory = locate::target_directory_from(&metadata)?;
    violations.extend(
        scan_sources(root, &members, &target_directory)
            .with_context(|| format!("cannot scan the sources under {}", root.display()))?,
    );
    Ok(violations)
}
