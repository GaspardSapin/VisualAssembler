//! egui widgets and panels of the IDE (m4-editor-shell.md §5): the editor view, the shell (tabs,
//! explorer, palette, find bar, status bar), themes and fonts. Later milestones add registers,
//! memory and problems panels.
//!
//! Depends on `egui` only, never on `eframe`/`winit`/`wgpu` (rule R3); CPU specifics arrive only
//! as `&dyn Architecture` (rule R2).
//!
//! Allowed internal dependencies: `va-core`, `va-editor`, `va-lang`, `va-debug`,
//! `va-workspace`.
//!
//! Modules: [`editor`], the virtualized, lexer-coloured editor widget (m4 §5.2–§5.6);
//! [`shell`], the window content and its command registry (m4 §5.1, §5.7–§5.13); [`theme`],
//! the light and dark palettes (m4 §5.4); [`fonts`], the embedded code font and zoom (m4 §5.5).
//! The shell API of m4 §5.1 is re-exported at the crate root.

pub mod editor;
pub mod fonts;
pub mod shell;
pub mod theme;

pub use shell::{
    ArchSupport, BenchProbe, DialogReply, DialogRequest, Persistence, ReadOnlyReason, RendererNote,
    Shell, ShellConfig, ShellHost,
};
