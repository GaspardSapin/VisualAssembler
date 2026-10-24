//! The window content of the IDE (m4-editor-shell.md §5.1, frozen; §5.7–§5.13): tab bar,
//! explorer, editor, status bar, palette and find bar, driven by a command registry.
//!
//! The shell owns no window: the app (`visual-assembler`) builds a [`Shell`] from a
//! [`ShellConfig`] and a [`ShellHost`], then calls [`Shell::ui`] every frame. Every file job runs
//! on the `va-io` worker, so [`Shell::new`] does no I/O.
//!
//! WP0 scaffold: the frozen signatures of §5.1 with `todo!()` bodies. WP-F implements them and
//! adds the modules of §2 (`state`, `tabs`, `explorer`, `palette`, `fuzzy`, `find`, `status`,
//! `commands`, `keymap`, `prompts`).

#![expect(
    unused_variables,
    dead_code,
    clippy::todo,
    reason = "WP0 stub: WP-F implements m4 §5.1"
)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::Sender;

use va_core::arch::Architecture;
use va_workspace::config::{Binding, ConfigProblem, Settings};
use va_workspace::i18n::{Lang, Localizer};
use va_workspace::paths::Paths;
use va_workspace::persist::{IoWorker, Session, Waker};

/// A registered architecture and what the shell needs that `va-core` does not provide.
#[derive(Clone, Copy)]
pub struct ArchSupport {
    /// The architecture.
    pub arch: &'static dyn Architecture,
    /// Its line comment prefix (e.g. `";"`), used by `edit.toggle-comment`.
    pub line_comment: &'static str,
}

/// The renderer the window runs on, for the status bar notices (m0 §19.9 S19-1).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RendererNote {
    /// The running renderer, e.g. `"glow"`.
    pub running: &'static str,
    /// The renderer that failed before it, if a fallback happened.
    pub fallback_from: Option<&'static str>,
    /// Why that renderer failed.
    pub fallback_reason: Option<String>,
    /// The value of `VA_TEST_FAIL_RENDERER` when the test hook is armed.
    pub test_hook: Option<String>,
}

/// Why the session is read-only.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReadOnlyReason {
    /// Another instance holds `session.lock`.
    OtherInstance,
    /// The data folder cannot be written.
    NotWritable,
    /// No config or data location could be resolved.
    NoLocation,
}

/// What the shell may write in the data folder.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Persistence {
    /// Session, backups and the default settings are written.
    ReadWrite,
    /// Nothing is written; the status bar says why.
    ReadOnly {
        /// Why.
        reason: ReadOnlyReason,
    },
    /// Bench mode: nothing is written and nothing is announced.
    Off,
}

/// Everything the app resolves before the window and hands to [`Shell::new`].
pub struct ShellConfig {
    /// The UI localizer.
    pub localizer: Arc<Localizer>,
    /// The resolved UI language.
    pub lang: Lang,
    /// The parsed `settings.toml`.
    pub settings: Settings,
    /// The resolved key bindings.
    pub bindings: Vec<Binding>,
    /// Problems found in the configuration files.
    pub problems: Vec<ConfigProblem>,
    /// The parsed `session.toml`.
    pub session: Session,
    /// The config and data folders, when resolved.
    pub paths: Option<Paths>,
    /// What may be written.
    pub persistence: Persistence,
    /// The architecture of the project, if the build has one.
    pub arch: Option<ArchSupport>,
    /// The running renderer.
    pub renderer: RendererNote,
    /// The `va-io` worker.
    pub io: IoWorker,
    /// The file or folder of the command line, opened after the first frame.
    pub open_at_start: Option<PathBuf>,
}

/// A native dialog the shell asks the host for.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum DialogRequest {
    /// Pick a file to open.
    OpenFile {
        /// Folder to start in.
        start: Option<PathBuf>,
    },
    /// Pick a folder to open as the project.
    OpenFolder {
        /// Folder to start in.
        start: Option<PathBuf>,
    },
    /// Pick a path to save to.
    SaveAs {
        /// Folder to start in.
        start: Option<PathBuf>,
        /// Proposed file name.
        name: String,
    },
}

/// Where the host sends the answer of a [`DialogRequest`]; it wakes the UI.
pub struct DialogReply {
    /// The shell's receiving end.
    sender: Sender<Option<PathBuf>>,
    /// Wakes the UI once the answer is sent.
    waker: Waker,
}

impl DialogReply {
    /// Sends the chosen path (`None` when the user cancelled) and wakes the UI.
    pub fn send(self, path: Option<PathBuf>) {
        todo!()
    }
}

/// What the app provides to the shell: native dialogs.
pub trait ShellHost {
    /// Shows `request` and answers through `reply`; returns false when native dialogs are
    /// unavailable, so the shell uses its internal path prompt.
    fn native_dialog(&mut self, request: DialogRequest, reply: DialogReply) -> bool;
}

/// The IDE shell.
pub struct Shell {
    /// Placeholder for the shell state (tabs, explorer, palette, find bar, status), which WP-F
    /// defines.
    state: (),
}

impl Shell {
    /// Builds the shell: no I/O, under 1 ms.
    pub fn new(config: ShellConfig, host: Box<dyn ShellHost>) -> Self {
        todo!()
    }

    /// Draws one frame and handles its input.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        todo!()
    }

    /// Handles a window close request; false when the close is cancelled and a prompt is shown.
    pub fn on_close_requested(&mut self, ctx: &egui::Context) -> bool {
        todo!()
    }

    /// Final backups and session, then waits for the worker (`IoWorker::flush`, 2 s).
    pub fn shutdown(&mut self) {
        todo!()
    }

    /// Runs the command `command` (an id of §5.12); false when it is unknown or disabled.
    pub fn execute(&mut self, command: &str) -> bool {
        todo!()
    }

    /// Moves the caret of the active tab to `line` (0-based) and scrolls to it.
    pub fn go_to_line(&mut self, line: usize) {
        todo!()
    }

    /// Opens a file in a tab, or a folder as the project.
    pub fn open_path(&mut self, path: &Path) {
        todo!()
    }

    /// The state the bench recorder watches.
    pub fn bench_probe(&self) -> BenchProbe {
        todo!()
    }
}

/// What the bench recorder reads from the shell (m4 §6.8).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct BenchProbe {
    /// Jobs submitted to `va-io` whose result has not arrived.
    pub pending_io: usize,
    /// Open tabs.
    pub open_tabs: usize,
    /// Line count of the active tab's document, once loaded.
    pub active_lines: Option<usize>,
    /// Whether the active tab's document was painted.
    pub active_painted: bool,
    /// Line count of every open document.
    pub total_lines: usize,
}
