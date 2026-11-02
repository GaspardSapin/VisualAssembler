//! Configuration problems (m4-editor-shell.md §4.2, frozen): the `cfg-*` codes, the capped
//! UTF-8 reader, and the helpers that walk a TOML document with spans.
//!
//! Every parser of this module tree reports through a [`Report`], which turns byte ranges into
//! [`ConfigProblem`]s with their display position and returns them in file order, whatever the
//! iteration order of the TOML tables.

use std::fs::File;
use std::io::{self, Read};
use std::ops::Range;
use std::path::{Path, PathBuf};

use toml::Spanned;
use toml::de::{DeArray, DeTable, DeValue};
use va_core::diag::{DiagArg, DiagCode, Diagnostic, Severity};
use va_core::source::{FileId, LineCol, LineIndex, SourceError, SourceSpan, SourceText, TextRange};

/// Largest configuration file read, in bytes (1 MiB).
pub const MAX_CONFIG_BYTES: usize = 1024 * 1024;

/// An I/O error other than "not found" (`error`: the English `io::ErrorKind` text).
pub(super) const UNREADABLE: DiagCode = DiagCode("cfg-unreadable");
/// The file is larger than [`MAX_CONFIG_BYTES`] (`bytes`).
pub(super) const TOO_LARGE: DiagCode = DiagCode("cfg-too-large");
/// The file is not valid UTF-8 (`offset` of the first invalid byte).
pub(super) const NOT_UTF8: DiagCode = DiagCode("cfg-not-utf8");
/// A TOML syntax error; the whole file falls back to the defaults (`detail`).
pub(super) const SYNTAX: DiagCode = DiagCode("cfg-syntax");
/// `schema_version` is present and not `1`; the file is ignored (`found`).
pub(super) const SCHEMA_VERSION: DiagCode = DiagCode("cfg-schema-version");
/// A key outside the schema, ignored (`key`).
pub(super) const UNKNOWN_KEY: DiagCode = DiagCode("cfg-unknown-key");
/// A value of the wrong TOML type (`key`, `expected`: a type id of [`expected`]).
pub(super) const WRONG_TYPE: DiagCode = DiagCode("cfg-wrong-type");
/// A number outside its inclusive range (`key`, `min`, `max`).
pub(super) const OUT_OF_RANGE: DiagCode = DiagCode("cfg-out-of-range");
/// A string that is not an allowed value (`key`, `value`, `expected`).
pub(super) const INVALID_VALUE: DiagCode = DiagCode("cfg-invalid-value");
/// A key binding names an unknown command (`command`).
pub(super) const UNKNOWN_COMMAND: DiagCode = DiagCode("cfg-unknown-command");
/// A key binding's key string cannot be bound (`key`).
pub(super) const BAD_KEY: DiagCode = DiagCode("cfg-bad-key");
/// A path that must be relative without `..` is not (`key`, `value`).
pub(super) const BAD_PATH: DiagCode = DiagCode("cfg-bad-path");

/// Every code a configuration problem can carry (§4.2 table, in its order).
pub static CONFIG_CODES: [DiagCode; 12] = [
    UNREADABLE,
    TOO_LARGE,
    NOT_UTF8,
    SYNTAX,
    SCHEMA_VERSION,
    UNKNOWN_KEY,
    WRONG_TYPE,
    OUT_OF_RANGE,
    INVALID_VALUE,
    UNKNOWN_COMMAND,
    BAD_KEY,
    BAD_PATH,
];

/// The `expected` ids of `cfg-wrong-type`, selected by the Fluent message `diag-cfg-wrong-type`.
pub(super) mod expected {
    /// `true` or `false`.
    pub(in crate::config) const BOOLEAN: &str = "boolean";
    /// A TOML integer.
    pub(in crate::config) const INTEGER: &str = "integer";
    /// A TOML float or integer.
    pub(in crate::config) const NUMBER: &str = "number";
    /// A TOML string.
    pub(in crate::config) const STRING: &str = "string";
    /// A TOML array.
    pub(in crate::config) const ARRAY: &str = "array";
    /// A TOML table.
    pub(in crate::config) const TABLE: &str = "table";
}

/// The top-level key of every configuration file that names its format version.
pub(super) const SCHEMA_VERSION_KEY: &str = "schema_version";

/// The only `schema_version` this build reads.
const SUPPORTED_SCHEMA: i64 = 1;

/// Longest text taken from the file into a problem argument, in chars; longer texts are cut
/// and end with `…`.
const MAX_ARG_CHARS: usize = 64;

/// One problem found in a configuration file: a non-blocking warning.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigProblem {
    /// The file.
    pub path: PathBuf,
    /// One of [`CONFIG_CODES`].
    pub code: DiagCode,
    /// Byte range of the offending text in the file (after a stripped byte order mark, as in
    /// `va_core::source::SourceText`).
    pub range: TextRange,
    /// Its display position (1-based line and column).
    pub at: LineCol,
    /// Fluent arguments of the message `diag-<code>`.
    pub args: Vec<(&'static str, DiagArg)>,
}

impl ConfigProblem {
    /// The problem as a `Severity::Warning`, non-blocking diagnostic of `file` (M5 Problems
    /// panel).
    pub fn to_diagnostic(&self, file: FileId) -> Diagnostic {
        let span = SourceSpan {
            file,
            range: self.range,
        };
        let mut diagnostic = Diagnostic::new(self.code, Severity::Warning, span).non_blocking();
        diagnostic.args = self.args.clone();
        diagnostic
    }

    /// A problem of the whole file, placed at its start (reading problems).
    fn at_start(path: &Path, code: DiagCode, args: Vec<(&'static str, DiagArg)>) -> Self {
        Self {
            path: path.to_path_buf(),
            code,
            range: TextRange::default(),
            at: LineCol { line: 1, column: 1 },
            args,
        }
    }
}

/// Reads a configuration file as UTF-8 text, capped at [`MAX_CONFIG_BYTES`]; `Ok(None)` when
/// the file is missing. One leading byte order mark is stripped, so problem ranges match the
/// editor's offsets.
///
/// # Errors
/// A `cfg-unreadable`, `cfg-too-large` or `cfg-not-utf8` problem.
pub fn read_config_text(path: &Path) -> Result<Option<String>, ConfigProblem> {
    let unreadable = |error: io::Error| {
        ConfigProblem::at_start(
            path,
            UNREADABLE,
            vec![("error", DiagArg::Str(error.kind().to_string()))],
        )
    };
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(unreadable(error)),
    };
    let too_large = |bytes: u64| {
        ConfigProblem::at_start(
            path,
            TOO_LARGE,
            vec![(
                "bytes",
                DiagArg::Int(i64::try_from(bytes).unwrap_or(i64::MAX)),
            )],
        )
    };
    let limit = MAX_CONFIG_BYTES as u64;
    let declared = file.metadata().map(|meta| meta.len()).ok();
    if let Some(len) = declared.filter(|&len| len > limit) {
        return Err(too_large(len));
    }
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(unreadable)?;
    if bytes.len() > MAX_CONFIG_BYTES {
        let read = bytes.len() as u64;
        return Err(too_large(declared.map_or(read, |len| len.max(read))));
    }
    match SourceText::from_bytes(bytes) {
        Ok(text) => Ok(Some(text.into_string())),
        Err(SourceError::TooLarge { bytes }) => Err(too_large(bytes as u64)),
        Err(SourceError::NotUtf8 { offset }) => Err(ConfigProblem::at_start(
            path,
            NOT_UTF8,
            vec![(
                "offset",
                DiagArg::Int(i64::try_from(offset).unwrap_or(i64::MAX)),
            )],
        )),
    }
}

/// Removes one leading byte order mark, as [`read_config_text`] does, for texts that did not
/// come through it.
pub(super) fn strip_bom(text: &str) -> &str {
    text.strip_prefix('\u{feff}').unwrap_or(text)
}

/// Cuts a text taken from the file to [`MAX_ARG_CHARS`] chars, ending with `…` when cut.
pub(super) fn excerpt(text: &str) -> String {
    match text.char_indices().nth(MAX_ARG_CHARS) {
        Some((cut, _)) => format!("{}…", &text[..cut]),
        None => text.to_owned(),
    }
}

/// Converts a byte offset to `u32`, saturating (texts read through [`read_config_text`] are
/// far smaller).
fn offset_u32(offset: usize) -> u32 {
    u32::try_from(offset).unwrap_or(u32::MAX)
}

/// Collects the problems of one configuration text and positions them.
pub(super) struct Report<'t> {
    /// The file the problems belong to.
    path: PathBuf,
    /// The text being checked (without a byte order mark).
    text: &'t str,
    /// Its line table, built on the first problem.
    index: Option<LineIndex>,
    /// The problems found so far, in discovery order.
    problems: Vec<ConfigProblem>,
}

impl<'t> Report<'t> {
    /// Starts an empty report on `text`, the content of `path`.
    pub(super) fn new(path: PathBuf, text: &'t str) -> Self {
        Self {
            path,
            text,
            index: None,
            problems: Vec::new(),
        }
    }

    /// The problems, in file order (by start offset, then discovery order).
    pub(super) fn finish(mut self) -> Vec<ConfigProblem> {
        self.problems.sort_by_key(|problem| problem.range.start);
        self.problems
    }

    /// Records `code` on the bytes `span` of the text.
    pub(super) fn push(
        &mut self,
        code: DiagCode,
        span: Range<usize>,
        args: Vec<(&'static str, DiagArg)>,
    ) {
        let range = TextRange::new(offset_u32(span.start), offset_u32(span.end));
        let text = self.text;
        let at = self
            .index
            .get_or_insert_with(|| LineIndex::new(text))
            .position(text, range.start);
        self.problems.push(ConfigProblem {
            path: self.path.clone(),
            code,
            range,
            at,
            args,
        });
    }

    /// `cfg-unknown-key` on the key `key` written at `span`.
    pub(super) fn unknown_key(&mut self, key: &str, span: Range<usize>) {
        self.push(UNKNOWN_KEY, span, vec![("key", DiagArg::Str(excerpt(key)))]);
    }

    /// `cfg-wrong-type` for `key`, which must be a value of the type id `expected`.
    pub(super) fn wrong_type(&mut self, key: &str, expected: &'static str, span: Range<usize>) {
        self.push(
            WRONG_TYPE,
            span,
            vec![
                ("key", DiagArg::Str(excerpt(key))),
                ("expected", DiagArg::Str(expected.to_owned())),
            ],
        );
    }

    /// `cfg-invalid-value` for the string `value` of `key`; `expected` is the list of allowed
    /// values, or a special id that the Fluent message explains (`font-file`, `file-name`).
    pub(super) fn invalid_value(
        &mut self,
        key: &str,
        value: &str,
        expected: &str,
        span: Range<usize>,
    ) {
        self.push(
            INVALID_VALUE,
            span,
            vec![
                ("key", DiagArg::Str(excerpt(key))),
                ("value", DiagArg::Str(excerpt(value))),
                ("expected", DiagArg::Str(expected.to_owned())),
            ],
        );
    }

    /// `cfg-out-of-range` for `key`, whose value must lie in `min..=max`.
    fn out_of_range(&mut self, key: &str, min: i64, max: i64, span: Range<usize>) {
        self.push(
            OUT_OF_RANGE,
            span,
            vec![
                ("key", DiagArg::Str(excerpt(key))),
                ("min", DiagArg::Int(min)),
                ("max", DiagArg::Int(max)),
            ],
        );
    }

    /// `cfg-bad-path` for the path `value` of `key`.
    pub(super) fn bad_path(&mut self, key: &str, value: &str, span: Range<usize>) {
        self.push(
            BAD_PATH,
            span,
            vec![
                ("key", DiagArg::Str(excerpt(key))),
                ("value", DiagArg::Str(excerpt(value))),
            ],
        );
    }

    /// The boolean of `value`, or `cfg-wrong-type`.
    pub(super) fn boolean(&mut self, key: &str, value: &Spanned<DeValue<'_>>) -> Option<bool> {
        let found = value.get_ref().as_bool();
        if found.is_none() {
            self.wrong_type(key, expected::BOOLEAN, value.span());
        }
        found
    }

    /// The string of `value`, or `cfg-wrong-type`.
    pub(super) fn string<'v>(
        &mut self,
        key: &str,
        value: &'v Spanned<DeValue<'_>>,
    ) -> Option<&'v str> {
        let found = value.get_ref().as_str();
        if found.is_none() {
            self.wrong_type(key, expected::STRING, value.span());
        }
        found
    }

    /// The table of `value`, or `cfg-wrong-type`.
    pub(super) fn table<'v, 'i>(
        &mut self,
        key: &str,
        value: &'v Spanned<DeValue<'i>>,
    ) -> Option<&'v DeTable<'i>> {
        let found = value.get_ref().as_table();
        if found.is_none() {
            self.wrong_type(key, expected::TABLE, value.span());
        }
        found
    }

    /// The array of `value`, or `cfg-wrong-type`.
    pub(super) fn array<'v, 'i>(
        &mut self,
        key: &str,
        value: &'v Spanned<DeValue<'i>>,
    ) -> Option<&'v DeArray<'i>> {
        let found = value.get_ref().as_array();
        if found.is_none() {
            self.wrong_type(key, expected::ARRAY, value.span());
        }
        found
    }

    /// The integer of `value` within `min..=max`; a float or another type is `cfg-wrong-type`,
    /// an integer outside the range `cfg-out-of-range`.
    pub(super) fn integer_in(
        &mut self,
        key: &str,
        value: &Spanned<DeValue<'_>>,
        min: i64,
        max: i64,
    ) -> Option<i64> {
        let DeValue::Integer(integer) = value.get_ref() else {
            self.wrong_type(key, expected::INTEGER, value.span());
            return None;
        };
        let found = i64::from_str_radix(integer.as_str(), integer.radix())
            .ok()
            .filter(|found| (min..=max).contains(found));
        if found.is_none() {
            self.out_of_range(key, min, max, value.span());
        }
        found
    }

    /// The number of `value` (a float, or an integer) within `min..=max`; `nan` and the
    /// infinities are out of range, another type is `cfg-wrong-type`.
    pub(super) fn number_in(
        &mut self,
        key: &str,
        value: &Spanned<DeValue<'_>>,
        min: i64,
        max: i64,
    ) -> Option<f64> {
        let parsed = match value.get_ref() {
            DeValue::Float(float) => float.as_str().parse::<f64>().ok(),
            DeValue::Integer(integer) => i64::from_str_radix(integer.as_str(), integer.radix())
                .ok()
                .map(|found| found as f64),
            _ => {
                self.wrong_type(key, expected::NUMBER, value.span());
                return None;
            }
        };
        let found = parsed.filter(|found| (min as f64..=max as f64).contains(found));
        if found.is_none() {
            self.out_of_range(key, min, max, value.span());
        }
        found
    }
}

/// Parses `text` with spans and checks its `schema_version`.
///
/// Returns the root table, or `None` after recording `cfg-syntax` (the whole file falls back to
/// the defaults) or `cfg-schema-version` (a version other than the integer `1`: the file is
/// ignored). A missing `schema_version` means 1. The root keeps its `schema_version` entry, which
/// the callers skip.
pub(super) fn parse_document<'t>(report: &mut Report<'t>, text: &'t str) -> Option<DeTable<'t>> {
    let root = match DeTable::parse(text) {
        Ok(root) => root.into_inner(),
        Err(error) => {
            let span = error.span().unwrap_or(0..0);
            report.push(
                SYNTAX,
                span,
                vec![("detail", DiagArg::Str(error.message().to_owned()))],
            );
            return None;
        }
    };
    if let Some(version) = root.get(SCHEMA_VERSION_KEY) {
        let supported = match version.get_ref() {
            DeValue::Integer(integer) => {
                i64::from_str_radix(integer.as_str(), integer.radix()) == Ok(SUPPORTED_SCHEMA)
            }
            _ => false,
        };
        if !supported {
            let span = version.span();
            let found = text.get(span.clone()).unwrap_or_default().trim();
            report.push(
                SCHEMA_VERSION,
                span,
                vec![("found", DiagArg::Str(excerpt(found)))],
            );
            return None;
        }
    }
    Some(root)
}

/// Tells whether `value` is a path relative to its root that stays inside it, on every host: not
/// empty, no root (`/`, `\`, UNC), no drive prefix (`C:`), and no `..` component, with both `/`
/// and `\` taken as separators.
pub(super) fn is_contained_relative_path(value: &str) -> bool {
    let bytes = value.as_bytes();
    let rooted = matches!(bytes.first(), Some(b'/' | b'\\'));
    let drive = bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
    let host_absolute = Path::new(value).has_root() || Path::new(value).is_absolute();
    let parent = value.split(['/', '\\']).any(|component| component == "..");
    !value.is_empty() && !rooted && !drive && !host_absolute && !parent
}
