//! Assembler contracts (m2-assembler.md §3.2, architecture §3): the [`Assembler`] trait, its
//! input (sources read through a [`SourceProvider`], [`AsmOptions`] with their [`AsmLimits`]),
//! its output ([`AssembleOutput`]: image, diagnostics, [`SymbolTable`], source map, [`Listing`])
//! and the [`CancelToken`] a caller trips to stop a running assembly.
//!
//! Nothing here reads a file: INCLUDE and INCBIN reach files only through the provider, which
//! owns the path policy (m2 §6.2). [`IncludeError`] and [`PathRefusal`] carry ids, never text.
//! [`MemorySources`] is the in-memory provider of tests, benches and the disassembly round trip.

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::diag::Diagnostic;
use crate::image::{ProgramImage, SourceMap};
use crate::source::{FileId, SourceSpan};

/// A cancellation flag shared by a token and all its clones: the caller keeps one clone and
/// trips it, the assembler polls the other (m2 §4.1: every 256 statements in every pass).
///
/// A token only goes from "not cancelled" to "cancelled"; there is no reset, so a new
/// assembly takes a new token.
#[derive(Clone, Debug, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    /// Builds a token that is not cancelled, shared with no other token.
    pub fn new() -> Self {
        Self::default()
    }

    /// Cancels this token and every clone of it. Calling it again changes nothing.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    /// Tells whether [`cancel`](CancelToken::cancel) was called on this token or on a clone.
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

/// Why a provider refused an include path before any file-system access (m2 §6.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PathRefusal {
    /// The path is empty.
    Empty,
    /// The path is absolute or starts with a separator.
    Absolute,
    /// The path has a drive, UNC, `\\?\` or `\\.\` prefix.
    Prefix,
    /// The path contains `:` (a drive-relative path or an alternate data stream).
    Colon,
    /// A component of the path is a Windows device name (`CON`, `NUL`, `COM1`, …), in any case
    /// and with any extension.
    DeviceName,
    /// The path leads out of its base folder, lexically or through a link.
    Escapes,
    /// The path holds a NUL character.
    Nul,
}

impl PathRefusal {
    /// Returns the identifier of the refusal, the `reason` of `asm-include-refused`:
    /// `"empty"`, `"absolute"`, `"drive"`, `"colon"`, `"device"`, `"escapes"` or `"nul"`.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Absolute => "absolute",
            Self::Prefix => "drive",
            Self::Colon => "colon",
            Self::DeviceName => "device",
            Self::Escapes => "escapes",
            Self::Nul => "nul",
        }
    }
}

/// Why a [`SourceProvider`] gives no file for the path of an INCLUDE or INCBIN (m2 §4.8).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IncludeError {
    /// No file of that name exists in any place the provider searches.
    NotFound,
    /// The path breaks the provider's path policy; nothing was accessed.
    Refused(PathRefusal),
    /// The file is larger than the provider accepts (`MAX_SOURCE_BYTES` for the CLI).
    TooLarge {
        /// Size of the file, in bytes.
        bytes: usize,
    },
    /// The file named by an INCLUDE is not valid UTF-8.
    NotUtf8 {
        /// Offset of the first invalid byte of the file.
        offset: usize,
    },
    /// The file exists but cannot be read.
    Unreadable,
    /// The provider already holds as many files as it may load ([`AsmLimits::max_files`]).
    TooManyFiles,
    /// The provider cannot give this kind of file (the default of
    /// [`SourceProvider::resolve_binary`]).
    Unsupported,
}

/// Where an assembly reads its texts and binary files. INCLUDE and INCBIN go through it, so the
/// path policy belongs to the provider (architecture §9.5, m2 §6.2).
///
/// File ids are the provider's own. Within one assembly a file keeps one id (the assembler finds
/// include cycles by id), and the texts and bytes it returns do not change while it is borrowed.
pub trait SourceProvider: Sync {
    /// Returns the text of `file`, or `None` if `file` is not a text file of this provider.
    fn text(&self, file: FileId) -> Option<&str>;

    /// Resolves `path`, the operand of an INCLUDE written in `from` without its quotes, to a
    /// text file.
    fn resolve_include(&self, from: FileId, path: &str) -> Result<FileId, IncludeError>;

    /// Resolves `path`, the operand of an INCBIN written in `from` without its quotes, to a file
    /// whose bytes [`binary`](SourceProvider::binary) gives.
    ///
    /// Default: `Err(IncludeError::Unsupported)`, for a provider without binary files.
    fn resolve_binary(&self, _from: FileId, _path: &str) -> Result<FileId, IncludeError> {
        Err(IncludeError::Unsupported)
    }

    /// Returns the bytes of `file`, or `None` if the provider has no bytes for it.
    ///
    /// Default: `None`, for a provider without binary files.
    fn binary(&self, _file: FileId) -> Option<&[u8]> {
        None
    }
}

/// Resource limits of one assembly (m2 §6.1, Security M1-S5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AsmLimits {
    /// Files loaded per assembly, the root included (default 256).
    pub max_files: u32,
    /// INCLUDE nesting depth (default 16).
    pub max_include_depth: u32,
    /// Nested macro expansions (default 64).
    pub max_macro_depth: u32,
    /// Statements after macro expansion (default 1,000,000); past it, assembly stops.
    pub max_statements: u32,
    /// Errors and warnings collected (default 1,000); then one note ends the collection.
    pub max_diagnostics: u32,
    /// Bytes of rendered listing text (default 64 MiB).
    pub max_listing_bytes: u64,
    /// Branch relaxation sweeps (default 16); the branches still relaxable after them stay long.
    pub max_relaxation_passes: u32,
}

impl AsmLimits {
    /// The limits of m2 §6.1.
    pub const DEFAULT: Self = Self {
        max_files: 256,
        max_include_depth: 16,
        max_macro_depth: 64,
        max_statements: 1_000_000,
        max_diagnostics: 1_000,
        max_listing_bytes: 64 * 1024 * 1024,
        max_relaxation_passes: 16,
    };
}

impl Default for AsmLimits {
    /// Returns [`AsmLimits::DEFAULT`].
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Options of one assembly.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AsmOptions {
    /// Whether the size optimizations run (default `true`; `false` is a68k `-n`).
    pub optimize: bool,
    /// Whether [`AssembleOutput::listing`] is produced (default `false`).
    pub listing: bool,
    /// Resource limits (default [`AsmLimits::DEFAULT`]).
    pub limits: AsmLimits,
}

impl Default for AsmOptions {
    /// Optimizations on, no listing, [`AsmLimits::DEFAULT`].
    fn default() -> Self {
        Self {
            optimize: true,
            listing: false,
            limits: AsmLimits::DEFAULT,
        }
    }
}

/// What one assembly reads.
#[derive(Clone, Copy)]
pub struct AssembleInput<'a> {
    /// The file assembly starts with, a text file of `sources`.
    pub root: FileId,
    /// Where texts and binary files come from.
    pub sources: &'a dyn SourceProvider,
    /// The options of this assembly.
    pub options: &'a AsmOptions,
}

impl fmt::Debug for AssembleInput<'_> {
    /// Shows `root` and `options`; the provider has no `Debug` and is left out.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AssembleInput")
            .field("root", &self.root)
            .field("options", self.options)
            .finish_non_exhaustive()
    }
}

/// What a [`Symbol`] is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    /// An ordinary label.
    Label,
    /// A local label, visible in the block of its [`Symbol::scope`].
    LocalLabel,
    /// A constant defined once (`EQU`, `=`).
    Equ,
    /// A redefinable value (`SET`).
    Set,
    /// A macro name.
    Macro,
}

impl SymbolKind {
    /// Returns the identifier of the kind: `"label"`, `"local-label"`, `"equ"`, `"set"` or
    /// `"macro"`.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Label => "label",
            Self::LocalLabel => "local-label",
            Self::Equ => "equ",
            Self::Set => "set",
            Self::Macro => "macro",
        }
    }
}

/// One symbol defined by an assembly.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Symbol {
    /// The name as written (symbols are case-sensitive).
    pub name: String,
    /// What the symbol is.
    pub kind: SymbolKind,
    /// Its value: a CPU word, sign-extended (32 bits on the 68000); the last value for `SET`;
    /// `None` for a macro.
    pub value: Option<i64>,
    /// For a local label, the name of the symbol that opened its block; `None` otherwise.
    pub scope: Option<String>,
    /// Where it is defined.
    pub definition: SourceSpan,
    /// Where it is used, in source order.
    pub references: Vec<SourceSpan>,
}

/// The symbols of an assembly, sorted by name then scope.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SymbolTable {
    /// Symbols sorted by `(name, scope)` (byte order; no scope first), stable.
    symbols: Vec<Symbol>,
    /// Indexes into `symbols` of every symbol that is not a local label, in table order.
    globals: Vec<usize>,
}

impl SymbolTable {
    /// Sorts `symbols` by name, then scope (byte order, `None` before any scope). The sort is
    /// stable: symbols with the same name and scope keep their input order.
    pub fn new(mut symbols: Vec<Symbol>) -> Self {
        symbols.sort_by(|a, b| {
            (a.name.as_str(), a.scope.as_deref()).cmp(&(b.name.as_str(), b.scope.as_deref()))
        });
        let globals = symbols
            .iter()
            .enumerate()
            .filter(|(_, symbol)| symbol.kind != SymbolKind::LocalLabel)
            .map(|(index, _)| index)
            .collect();
        Self { symbols, globals }
    }

    /// Returns the symbols, sorted by name then scope.
    pub fn symbols(&self) -> &[Symbol] {
        &self.symbols
    }

    /// Returns the symbol named exactly `name` (case-sensitive) that is not a local label;
    /// macros are found. With several such symbols, returns the first in table order.
    pub fn get(&self, name: &str) -> Option<&Symbol> {
        let first = self
            .globals
            .partition_point(|&index| self.symbols[index].name.as_str() < name);
        let symbol = &self.symbols[*self.globals.get(first)?];
        (symbol.name == name).then_some(symbol)
    }
}

/// One line of a [`Listing`]: a source line as assembled, or one line of a macro expansion.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ListingLine {
    /// The source file.
    pub file: FileId,
    /// The 1-based source line; for a macro expansion, the line of the call.
    pub line: u32,
    /// 0 for a file line, `n` for a line of an expansion `n` levels deep.
    pub depth: u8,
    /// The line as assembled (the expanded text when `depth > 0`), at most 256 chars.
    pub text: String,
    /// The address of the line, if it has one.
    pub address: Option<u64>,
    /// The value defined by an `EQU`, `SET` or `=` line.
    pub value: Option<i64>,
    /// The first bytes the line produced (at most 64).
    pub bytes: Vec<u8>,
    /// The number of bytes the line produced in total.
    pub byte_count: u64,
    /// Whether the line is inside a false conditional block.
    pub skipped: bool,
    /// The id of the optimization applied to the line (m2 §4.10), e.g. `"addq"`.
    pub optimization: Option<&'static str>,
    /// Errors reported on the line.
    pub errors: u16,
    /// Warnings reported on the line.
    pub warnings: u16,
}

/// The listing of an assembly: data only; the CPU crate renders it.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct Listing {
    /// The lines, in assembly order.
    pub lines: Vec<ListingLine>,
    /// Whether lines are missing because the listing reached
    /// [`AsmLimits::max_listing_bytes`].
    pub truncated: bool,
}

/// What one assembly produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssembleOutput {
    /// The program, present iff no diagnostic is blocking and the assembly was not cancelled.
    pub image: Option<ProgramImage>,
    /// Every diagnostic, in report order.
    pub diagnostics: Vec<Diagnostic>,
    /// The symbols defined.
    pub symbols: SymbolTable,
    /// The address ↔ source line mapping of the produced bytes.
    pub source_map: SourceMap,
    /// The listing, present iff [`AsmOptions::listing`] was set.
    pub listing: Option<Listing>,
    /// Whether the [`CancelToken`] stopped the assembly (then `image` is `None`).
    pub cancelled: bool,
}

/// The assembler of one CPU dialect.
pub trait Assembler: Send {
    /// Assembles `input`, polling `cancel`; returns early with
    /// [`AssembleOutput::cancelled`] set once it sees the token cancelled.
    fn assemble(&mut self, input: &AssembleInput<'_>, cancel: &CancelToken) -> AssembleOutput;
}

/// The content of one [`MemorySources`] file.
#[derive(Clone, Debug, PartialEq, Eq)]
enum MemoryContent {
    /// A text file (the root and [`MemorySources::with_text`]).
    Text(String),
    /// A binary file ([`MemorySources::with_binary`]).
    Binary(Vec<u8>),
}

/// One file of a [`MemorySources`].
#[derive(Clone, Debug, PartialEq, Eq)]
struct MemoryFile {
    /// The name as registered.
    name: String,
    /// The name with every `\` read as `/`, compared with requested paths.
    key: String,
    /// The content.
    content: MemoryContent,
}

impl MemoryFile {
    /// Builds the file `name` holding `content`.
    fn new(name: &str, content: MemoryContent) -> Self {
        Self {
            name: name.to_owned(),
            key: name.replace('\\', "/"),
            content,
        }
    }

    /// Tells whether `path`, with every `\` read as `/`, equals the name of the file.
    fn is_named(&self, path: &str) -> bool {
        self.key.len() == path.len()
            && self
                .key
                .bytes()
                .zip(path.bytes())
                .all(|(key, byte)| key == if byte == b'\\' { b'/' } else { byte })
    }

    /// Returns the bytes of the file (a text file gives its UTF-8 bytes).
    fn bytes(&self) -> &[u8] {
        match &self.content {
            MemoryContent::Text(text) => text.as_bytes(),
            MemoryContent::Binary(bytes) => bytes,
        }
    }
}

/// An in-memory [`SourceProvider`] for tests, benches and the round trip: no path policy, no
/// file system.
///
/// - Ids: the root is `FileId(0)`; each [`with_text`](MemorySources::with_text) or
///   [`with_binary`](MemorySources::with_binary) call takes the next id, `FileId(1)`,
///   `FileId(2)`, …, in call order.
/// - Names: a path matches a file iff it equals the file's name exactly (case-sensitive, no
///   other normalization) once every `\` in both is read as `/`; anything else is
///   [`IncludeError::NotFound`]. Every file is reachable from every file (`from` is not used),
///   and the first file registered under a name wins.
/// - Kinds: [`resolve_include`](SourceProvider::resolve_include) finds the text files (the root
///   included) and [`text`](SourceProvider::text) gives them;
///   [`resolve_binary`](SourceProvider::resolve_binary) and
///   [`binary`](SourceProvider::binary) take every file, a text file giving its UTF-8 bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemorySources {
    /// The files; the index of a file is its id.
    files: Vec<MemoryFile>,
}

impl MemorySources {
    /// Builds a provider whose root, `FileId(0)`, is the text file `root_name`.
    pub fn new(root_name: &str, root_text: String) -> Self {
        Self {
            files: vec![MemoryFile::new(root_name, MemoryContent::Text(root_text))],
        }
    }

    /// Adds the text file `name`, with the next id.
    #[must_use]
    pub fn with_text(mut self, name: &str, text: String) -> Self {
        self.files
            .push(MemoryFile::new(name, MemoryContent::Text(text)));
        self
    }

    /// Adds the binary file `name`, with the next id.
    #[must_use]
    pub fn with_binary(mut self, name: &str, bytes: Vec<u8>) -> Self {
        self.files
            .push(MemoryFile::new(name, MemoryContent::Binary(bytes)));
        self
    }

    /// Returns the name `file` was registered with, or `None` for an unknown id.
    pub fn name(&self, file: FileId) -> Option<&str> {
        self.file(file).map(|entry| entry.name.as_str())
    }

    /// Returns the file of id `file`.
    fn file(&self, file: FileId) -> Option<&MemoryFile> {
        self.files.get(usize::try_from(file.0).ok()?)
    }

    /// Returns the id of the first file named `path` that `accept` takes.
    fn find(&self, path: &str, accept: fn(&MemoryContent) -> bool) -> Result<FileId, IncludeError> {
        self.files
            .iter()
            .position(|entry| accept(&entry.content) && entry.is_named(path))
            .and_then(|index| u32::try_from(index).ok())
            .map(FileId)
            .ok_or(IncludeError::NotFound)
    }
}

impl SourceProvider for MemorySources {
    fn text(&self, file: FileId) -> Option<&str> {
        match &self.file(file)?.content {
            MemoryContent::Text(text) => Some(text),
            MemoryContent::Binary(_) => None,
        }
    }

    fn resolve_include(&self, _from: FileId, path: &str) -> Result<FileId, IncludeError> {
        self.find(path, |content| matches!(content, MemoryContent::Text(_)))
    }

    fn resolve_binary(&self, _from: FileId, path: &str) -> Result<FileId, IncludeError> {
        self.find(path, |_| true)
    }

    fn binary(&self, file: FileId) -> Option<&[u8]> {
        self.file(file).map(MemoryFile::bytes)
    }
}
