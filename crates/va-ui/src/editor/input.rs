//! Keyboard, clipboard, IME and mouse input of the editor (m4-editor-shell.md §5.6).
//!
//! The shell's keymap consumes its shortcuts (`InputState::consume_shortcut`) before the editor
//! sees the events; the editor handles what is left: arrows (Shift extends, Ctrl = word),
//! Home/End (Ctrl = document), PageUp/PageDown, Backspace/Delete (Ctrl = word), Enter,
//! Tab/Shift+Tab, Escape, text, copy/cut/paste and IME commits. Keys held with Alt belong to the
//! keymap and are ignored here.

use std::time::Duration;

use egui::{Event, EventFilter, ImeEvent, InputState, Key, Modifiers, PointerButton};
use va_editor::document::Document;
use va_editor::edit::{EditCommand, Motion};
use va_editor::selection::{Selection, Selections};

use super::view::EditorContext;

/// The focus lock of a focused editor: Tab, the arrows and Escape act on the editor instead of
/// moving or dropping the focus (`Memory::set_focus_lock_filter`).
pub(crate) const FOCUS_LOCK: EventFilter = EventFilter {
    tab: true,
    horizontal_arrows: true,
    vertical_arrows: true,
    escape: true,
};

/// What one key press does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum KeyAction {
    /// Moves every caret, extending the selections when the flag is set.
    Move(Motion, bool),
    /// Runs an edit command.
    Edit(EditCommand),
    /// Escape: keep the primary selection only, then collapse it to its caret.
    Escape,
}

/// The action of a key pressed with `modifiers`, if the editor handles it; `page_rows` is the
/// PageUp/PageDown distance.
pub(crate) fn key_action(key: Key, modifiers: Modifiers, page_rows: u32) -> Option<KeyAction> {
    if modifiers.alt {
        return None;
    }
    let ctrl = modifiers.ctrl || modifiers.command;
    let extend = modifiers.shift;
    let pick = |plain: Motion, with_ctrl: Motion| {
        KeyAction::Move(if ctrl { with_ctrl } else { plain }, extend)
    };
    let edit = |plain: EditCommand, with_ctrl: EditCommand| {
        KeyAction::Edit(if ctrl { with_ctrl } else { plain })
    };
    let action = match key {
        Key::ArrowLeft => pick(Motion::Left, Motion::WordLeft),
        Key::ArrowRight => pick(Motion::Right, Motion::WordRight),
        Key::ArrowUp if !ctrl => pick(Motion::Up, Motion::Up),
        Key::ArrowDown if !ctrl => pick(Motion::Down, Motion::Down),
        Key::Home => pick(Motion::LineStart, Motion::DocStart),
        Key::End => pick(Motion::LineEnd, Motion::DocEnd),
        Key::PageUp if !ctrl => pick(Motion::PageUp(page_rows), Motion::PageUp(page_rows)),
        Key::PageDown if !ctrl => pick(Motion::PageDown(page_rows), Motion::PageDown(page_rows)),
        Key::Backspace => edit(EditCommand::Backspace, EditCommand::DeleteWordLeft),
        Key::Delete => edit(EditCommand::Delete, EditCommand::DeleteWordRight),
        Key::Enter if !ctrl => KeyAction::Edit(EditCommand::NewLine),
        Key::Tab if !ctrl && extend => KeyAction::Edit(EditCommand::Outdent),
        Key::Tab if !ctrl => KeyAction::Edit(EditCommand::Tab),
        Key::Escape => KeyAction::Escape,
        _ => return None,
    };
    Some(action)
}

/// Tells whether a text event is real text (not a line break, a tab or other control chars,
/// which arrive as key events).
fn is_insertable(text: &str) -> bool {
    !text.is_empty() && !text.chars().all(char::is_control)
}

/// What the events of one frame did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Handled {
    /// The text changed.
    pub changed: bool,
    /// Some event was for the editor (restarts the blink).
    pub activity: bool,
}

/// The undo-grouping time of egui input time `seconds`.
pub(crate) fn edit_time(seconds: f64) -> Duration {
    Duration::try_from_secs_f64(seconds.max(0.0)).unwrap_or(Duration::ZERO)
}

/// Applies the keyboard, clipboard and IME events of this frame to `doc` (the editor has the
/// focus). `preedit` is the IME pre-edit text, drawn by the view.
pub(crate) fn handle_events(
    egui_ctx: &egui::Context,
    events: &[Event],
    doc: &mut Document,
    ctx: &EditorContext<'_>,
    page_rows: u32,
    preedit: &mut Option<String>,
    at: Duration,
) -> Handled {
    let mut handled = Handled::default();
    let tab_width = ctx.style.tab_width.max(1);
    let edit = |doc: &mut Document, handled: &mut Handled, command: EditCommand| {
        handled.activity = true;
        if !ctx.read_only && doc.edit(command, &ctx.options, at) {
            handled.changed = true;
        }
    };
    for event in events {
        match event {
            Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => match key_action(*key, *modifiers, page_rows) {
                Some(KeyAction::Move(motion, extend)) => {
                    handled.activity = true;
                    doc.move_carets(motion, extend, tab_width);
                }
                Some(KeyAction::Edit(command)) => edit(doc, &mut handled, command),
                Some(KeyAction::Escape) => {
                    handled.activity = true;
                    escape(doc);
                }
                None => {}
            },
            Event::Text(text) if is_insertable(text) => {
                edit(doc, &mut handled, EditCommand::InsertText(text.clone()));
            }
            Event::Copy => {
                handled.activity = true;
                copy(egui_ctx, doc);
            }
            Event::Cut => {
                handled.activity = true;
                if copy(egui_ctx, doc) {
                    edit(doc, &mut handled, EditCommand::DeleteSelection);
                }
            }
            Event::Paste(text) if !text.is_empty() => {
                edit(doc, &mut handled, EditCommand::Paste(text.clone()));
            }
            Event::Ime(ImeEvent::Preedit { text, .. }) => {
                handled.activity = true;
                *preedit = (!text.is_empty()).then(|| text.clone());
            }
            Event::Ime(ImeEvent::Commit(text)) => {
                handled.activity = true;
                *preedit = None;
                if is_insertable(text) {
                    edit(doc, &mut handled, EditCommand::InsertText(text.clone()));
                }
            }
            _ => {}
        }
    }
    handled
}

/// Escape: with several selections, keeps the primary one; with one non-empty selection,
/// collapses it to its caret.
fn escape(doc: &mut Document) {
    let selections = doc.selections();
    if selections.len() > 1 {
        let mut kept = selections.clone();
        kept.collapse_to_primary();
        doc.set_selections(kept);
    } else if !selections.primary().is_empty() {
        let head = selections.primary().head;
        doc.set_selections(Selections::single(Selection::caret(head)));
    }
}

/// Puts the selected texts, joined by the document line ending, on the clipboard; returns false
/// (and leaves the clipboard alone) when every selection is empty.
fn copy(egui_ctx: &egui::Context, doc: &Document) -> bool {
    let texts = doc.selected_texts();
    if texts.iter().all(String::is_empty) {
        return false;
    }
    egui_ctx.copy_text(texts.join(doc.line_ending().as_str()));
    true
}

/// The modifiers of this frame's primary-button press: those carried by the press event, else
/// the current ones.
pub(crate) fn press_modifiers(input: &InputState) -> Modifiers {
    input
        .events
        .iter()
        .rev()
        .find_map(|event| match event {
            Event::PointerButton {
                button: PointerButton::Primary,
                pressed: true,
                modifiers,
                ..
            } => Some(*modifiers),
            _ => None,
        })
        .unwrap_or(input.modifiers)
}

/// A primary-button press at `byte`: Alt adds a caret, Shift extends the primary selection,
/// else a single caret. Returns the anchor of the drag that may follow.
pub(crate) fn press(doc: &mut Document, byte: usize, modifiers: Modifiers) -> usize {
    if modifiers.alt {
        doc.add_caret(byte);
        doc.selections().primary().head
    } else if modifiers.shift {
        let anchor = doc.selections().primary().anchor;
        doc.set_selections(Selections::single(Selection::new(anchor, byte)));
        anchor
    } else {
        doc.set_selections(Selections::single(Selection::caret(byte)));
        doc.selections().primary().head
    }
}

/// A drag of the primary button to `byte`: the primary selection spans `anchor..byte`, the
/// other selections are kept (an Alt drag adds a selection).
pub(crate) fn drag(doc: &mut Document, anchor: usize, byte: usize) {
    let primary = *doc.selections().primary();
    if primary.anchor == anchor && primary.head == byte {
        return;
    }
    let index = doc.selections().primary_index();
    let mut list = doc.selections().as_slice().to_vec();
    if let Some(slot) = list.get_mut(index) {
        *slot = Selection::new(anchor, byte);
    }
    doc.set_selections(Selections::from_vec(list, index));
}
