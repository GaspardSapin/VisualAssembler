//! Shared support of the `va-ui` kittest suite (m4-editor-shell.md §10, §10.2): a fake
//! architecture and lexer, so no test needs a CPU crate (rule R2), and harness helpers.
//!
//! Each test file includes it with `mod support;` and uses what it needs. WP-E owns this file
//! after WP0; the shell tests (WP-F) keep their own helpers in their `shell_*.rs` files.
#![allow(clippy::unwrap_used)]
#![allow(
    dead_code,
    reason = "each test file uses a different subset of the helpers"
)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use egui_kittest::Harness;
use va_core::arch::Architecture;
use va_core::diag::DiagCode;
use va_core::isa::{EaTiming, InstructionInfo, InstructionSet, LookupError};
use va_core::regs::RegisterSpec;
use va_core::source::TextRange;
use va_core::syntax::{KeywordInfo, LineState, SyntaxProvider, Token, TokenKind};
use va_editor::document::Document;
use va_editor::encoding::{Encoding, LineEnding};
use va_editor::text::Text;
use va_ui::ArchSupport;

// -------------------------------------------------------------------------------------------
// Fake lexer and architecture
// -------------------------------------------------------------------------------------------

/// A small, stateless lexer that honours the `SyntaxProvider` contract (contiguous, non-empty,
/// char-aligned tokens covering the line) and counts its calls.
///
/// Rules, left to right: a run of spaces or tabs is `Whitespace`; `;` starts a `Comment` to the
/// end of the line; a word (alphanumeric, `_`, `.`, `@`) is a `Number` when it starts with a
/// digit, a `Label` when it starts at column 0, the first other word is a `Mnemonic`, and later
/// words are `Symbol`s; any other char is one `Punctuation` token.
#[derive(Debug, Default)]
pub struct FakeSyntax {
    /// Number of `lex_line` calls so far.
    calls: AtomicUsize,
}

impl FakeSyntax {
    /// A lexer with a zero call count.
    pub const fn new() -> Self {
        Self {
            calls: AtomicUsize::new(0),
        }
    }

    /// Number of `lex_line` calls so far.
    pub fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

/// Tells whether `c` belongs to a word of [`FakeSyntax`].
fn is_word(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '.' | '@')
}

/// Byte offset of the first char of `line[start..]` that `keep` rejects (or the line length).
fn run_end(line: &str, start: usize, keep: impl Fn(char) -> bool) -> usize {
    line[start..]
        .char_indices()
        .find(|&(_, c)| !keep(c))
        .map_or(line.len(), |(offset, _)| start + offset)
}

/// Converts a line offset to the `u32` of a token range (lines are at most 64 KiB in `va-core`).
fn offset(at: usize) -> u32 {
    u32::try_from(at).unwrap()
}

impl SyntaxProvider for FakeSyntax {
    fn lex_line(&self, line: &str, state: LineState, out: &mut Vec<Token>) -> LineState {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let mut at = 0;
        let mut seen_mnemonic = false;
        while let Some(c) = line[at..].chars().next() {
            let (kind, end) = if c == ' ' || c == '\t' {
                (
                    TokenKind::Whitespace,
                    run_end(line, at, |c| c == ' ' || c == '\t'),
                )
            } else if c == ';' {
                (TokenKind::Comment, line.len())
            } else if is_word(c) {
                let kind = if c.is_ascii_digit() {
                    TokenKind::Number
                } else if at == 0 {
                    TokenKind::Label
                } else if !seen_mnemonic {
                    seen_mnemonic = true;
                    TokenKind::Mnemonic
                } else {
                    TokenKind::Symbol
                };
                (kind, run_end(line, at, is_word))
            } else {
                (TokenKind::Punctuation, at + c.len_utf8())
            };
            out.push(Token {
                kind,
                range: TextRange::new(offset(at), offset(end)),
            });
            at = end;
        }
        state
    }

    fn keywords(&self) -> &[KeywordInfo] {
        &[]
    }
}

/// An instruction set with no instruction.
#[derive(Debug, Default)]
pub struct FakeIsa;

impl InstructionSet for FakeIsa {
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

/// A CPU with the [`FakeSyntax`] lexer, no instruction, no register, no diagnostic code and no
/// Fluent resource, registered with the line comment prefix `;` by [`fake_arch_support`].
#[derive(Debug, Default)]
pub struct FakeArch {
    /// Its lexer.
    pub syntax: FakeSyntax,
    /// Its instruction set.
    pub isa: FakeIsa,
}

impl FakeArch {
    /// A fake CPU whose lexer has a zero call count.
    pub const fn new() -> Self {
        Self {
            syntax: FakeSyntax::new(),
            isa: FakeIsa,
        }
    }
}

impl Architecture for FakeArch {
    fn id(&self) -> &'static str {
        "fake"
    }

    fn syntax(&self) -> &dyn SyntaxProvider {
        &self.syntax
    }

    fn instruction_set(&self) -> &dyn InstructionSet {
        &self.isa
    }

    fn registers(&self) -> &'static [RegisterSpec] {
        &[]
    }

    fn diagnostic_codes(&self) -> &'static [DiagCode] {
        &[]
    }

    fn locale_resources(&self, _lang: &str) -> &'static [&'static str] {
        &[]
    }
}

/// The fake CPU shared by a test binary. Its lexer's call count is shared by every test of that
/// binary (they run in parallel): a test that counts calls builds its own [`FakeSyntax`].
pub static FAKE_ARCH: FakeArch = FakeArch::new();

/// [`FAKE_ARCH`] as the shell registers it, with the line comment prefix `;`.
pub fn fake_arch_support() -> ArchSupport {
    ArchSupport {
        arch: &FAKE_ARCH,
        line_comment: ";",
    }
}

// -------------------------------------------------------------------------------------------
// Documents
// -------------------------------------------------------------------------------------------

/// `count` LF-separated lines of fake assembly, without a final terminator, so the text has
/// exactly `count` lines; line `i` (0-based) starts with the unique label `l<i>:`.
pub fn numbered_text(count: usize) -> String {
    let mut text = String::new();
    for i in 0..count {
        if i > 0 {
            text.push('\n');
        }
        text.push_str(&format!(
            "l{i}:\tmove.l\td{},d{}\t; line {i}",
            i % 8,
            (i + 1) % 8
        ));
    }
    text
}

/// A clean UTF-8 document of `text`, with LF for new line breaks.
pub fn document(text: &str) -> Document {
    Document::new(Text::new(text), Encoding::Utf8, LineEnding::Lf)
}

// -------------------------------------------------------------------------------------------
// Harness
// -------------------------------------------------------------------------------------------

/// Size of the test window, in logical pixels (§10.2: 800×600).
pub const HARNESS_SIZE: [f32; 2] = [800.0, 600.0];

/// Longest wait of [`wait_until`] for a worker result (`va-io`, `va-search`).
pub const WAIT_TIMEOUT: Duration = Duration::from_secs(5);

/// A headless harness of [`HARNESS_SIZE`] running `app` over `state` (egui only, no GPU).
pub fn harness<'a, S>(app: impl FnMut(&mut egui::Ui, &mut S) + 'a, state: S) -> Harness<'a, S> {
    Harness::builder()
        .with_size(egui::Vec2::from(HARNESS_SIZE))
        .build_ui_state(app, state)
}

/// The wake-up the last frame asked for: `Duration::MAX` when none (reactive mode at rest).
pub fn repaint_delay<S>(harness: &Harness<'_, S>) -> Duration {
    harness
        .output()
        .viewport_output
        .get(&egui::ViewportId::ROOT)
        .map_or(Duration::MAX, |viewport| viewport.repaint_delay)
}

/// Sets the input time of the next frame, in seconds (blink and timer tests).
pub fn set_time<S>(harness: &mut Harness<'_, S>, seconds: f64) {
    harness.input_mut().time = Some(seconds);
}

/// Steps the harness until `done` holds for its state; panics after [`WAIT_TIMEOUT`] of real
/// time, naming `what`.
pub fn wait_until<S>(harness: &mut Harness<'_, S>, what: &str, mut done: impl FnMut(&S) -> bool) {
    let deadline = Instant::now() + WAIT_TIMEOUT;
    loop {
        harness.step();
        if done(harness.state()) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "timed out after {WAIT_TIMEOUT:?} waiting for {what}"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}

// -------------------------------------------------------------------------------------------
// Files
// -------------------------------------------------------------------------------------------

/// Creates a fresh, empty folder `<CARGO_TARGET_TMPDIR>/<tag>-<pid>-<n>` (CLAUDE.md test rule).
pub fn temp_dir(tag: &str) -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("{tag}-{}-{n}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    dir
}
