//! Shared support of the `va-ui` kittest suite (m4-editor-shell.md §10, §10.2): a fake
//! architecture and lexer, so no test needs a CPU crate (rule R2), and harness helpers.
//!
//! Each test file includes it with `mod support;` and uses what it needs. WP-E owns this file
//! after WP0; the shell tests (WP-F) keep their own helpers in their `shell_*.rs` files.
//!
//! The editor tests (`editor_*.rs`) drive one [`EditorView`] over an [`EditorState`] through
//! [`editor_harness`], which installs the code font on its first frame.
#![allow(clippy::unwrap_used)]
#![allow(
    dead_code,
    reason = "each test file uses a different subset of the helpers"
)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use egui::{Event, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::Harness;
use va_core::arch::Architecture;
use va_core::diag::DiagCode;
use va_core::isa::{EaTiming, InstructionInfo, InstructionSet, LookupError};
use va_core::regs::RegisterSpec;
use va_core::source::TextRange;
use va_core::syntax::{KeywordInfo, LineState, SyntaxProvider, Token, TokenKind};
use va_editor::document::Document;
use va_editor::edit::EditOptions;
use va_editor::encoding::{Encoding, LineEnding};
use va_editor::text::Text;
use va_ui::ArchSupport;
use va_ui::editor::{Decoration, EditorContext, EditorResponse, EditorStyle, EditorView};
use va_ui::fonts;
use va_ui::theme::Theme;

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

/// Steps the harness `frames` times without new input (lets fonts and focus settle).
pub fn settle<S>(harness: &mut Harness<'_, S>, frames: usize) {
    for _ in 0..frames {
        harness.step();
    }
}

/// Queues a primary click at `pos` with `modifiers`: a move there, the press and the release,
/// one frame each when stepped.
pub fn click_at<S>(harness: &Harness<'_, S>, pos: Pos2, modifiers: Modifiers) {
    harness.event(Event::PointerMoved(pos));
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers,
        });
    }
}

// -------------------------------------------------------------------------------------------
// Editor
// -------------------------------------------------------------------------------------------

/// Accessibility label of the editor under test (the file name of §5.2).
pub const EDITOR_LABEL: &str = "main.asm";

/// Size of the code font in the editor tests, in points.
pub const CODE_FONT_SIZE: f32 = 14.0;

/// Label of the button drawn above the editor when [`EditorState::other_widget`] is set.
pub const OTHER_WIDGET_LABEL: &str = "other widget";

/// The default editor style of the tests: the `va-code` font, tab width 8, numbers and the
/// current-line background on, blink off.
pub fn editor_style() -> EditorStyle {
    EditorStyle {
        font: fonts::code_font(CODE_FONT_SIZE),
        tab_width: 8,
        line_numbers: true,
        highlight_current_line: true,
        cursor_blink: false,
        style_generation: 0,
    }
}

/// Everything an editor test drives: the document, its view and what the frame needs.
pub struct EditorState {
    /// The document.
    pub doc: Document,
    /// The view under test.
    pub view: EditorView,
    /// The lexer, with its own call count.
    pub syntax: FakeSyntax,
    /// The colours.
    pub theme: Theme,
    /// The drawing options.
    pub style: EditorStyle,
    /// The edit options.
    pub options: EditOptions,
    /// Decorations to paint.
    pub decorations: Vec<Decoration>,
    /// Whether edits are refused.
    pub read_only: bool,
    /// Whether the first frame gives the editor the focus.
    pub focus_on_start: bool,
    /// Whether a button is drawn above the editor (focus tests).
    pub other_widget: bool,
    /// The response of the last frame.
    pub last: Option<EditorResponse>,
    /// Frames run so far.
    pub frames: usize,
}

impl EditorState {
    /// A focused editor over a clean document of `text`, light theme, [`editor_style`].
    pub fn new(text: &str) -> Self {
        Self {
            doc: document(text),
            view: EditorView::new(egui::Id::new("editor-under-test")),
            syntax: FakeSyntax::new(),
            theme: Theme::light(),
            style: editor_style(),
            options: EditOptions {
                tab_width: 8,
                insert_spaces: false,
            },
            decorations: Vec::new(),
            read_only: false,
            focus_on_start: true,
            other_widget: false,
            last: None,
            frames: 0,
        }
    }

    /// The document text as a string.
    pub fn text(&self) -> String {
        self.doc.text().to_string()
    }

    /// The widget rectangle of the last frame (the viewport).
    pub fn rect(&self) -> Rect {
        self.last.as_ref().unwrap().response.rect
    }

    /// Screen position of the middle of the left edge of column `col` of `line`, for an ASCII
    /// line without tabs, with the view scrolled to the top-left.
    pub fn char_pos(&self, line: usize, col: usize) -> Pos2 {
        let metrics = self.view.metrics();
        let rect = self.rect();
        egui::pos2(
            rect.min.x + metrics.gutter_width + (col as f32 + 0.2) * metrics.char_width,
            rect.min.y + (line as f32 + 0.5) * metrics.row_height,
        )
    }
}

/// One frame of the editor test app: installs the fonts on the first frame (and gives the focus
/// when asked), then shows the view.
fn editor_frame(ui: &mut egui::Ui, state: &mut EditorState) {
    if state.frames == 0 {
        fonts::setup_context(ui.ctx(), None);
        if state.focus_on_start {
            state.view.request_focus(ui);
        }
    }
    state.frames += 1;
    if state.other_widget {
        let _clicked = ui.button(OTHER_WIDGET_LABEL);
    }
    let ctx = EditorContext {
        syntax: &state.syntax,
        theme: &state.theme,
        style: &state.style,
        options: state.options,
        decorations: &state.decorations,
        read_only: state.read_only,
        line_comment: Some(";"),
        label: EDITOR_LABEL,
    };
    state.last = Some(state.view.show(ui, &mut state.doc, &ctx));
}

/// A [`HARNESS_SIZE`] harness running one editor over `state`, settled (fonts installed, the
/// focus given when asked).
pub fn editor_harness(state: EditorState) -> Harness<'static, EditorState> {
    let mut harness = harness(editor_frame, state);
    settle(&mut harness, 3);
    harness
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
