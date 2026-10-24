//! The light and dark themes (m4-editor-shell.md §5.4, frozen): editor colours, one per token
//! role, and the egui visuals.
//!
//! Every token colour has a WCAG contrast ratio of at least 4.5 against the background and the
//! current-line colour (`tests/theme.rs`).
//!
//! WP0 scaffold: frozen signatures with `todo!()` bodies; WP-E implements them.

#![expect(
    unused_variables,
    clippy::todo,
    reason = "WP0 stub: WP-E implements m4 §5.4"
)]

use egui::Color32;
use va_core::syntax::TokenKind;

/// Which base theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ThemeId {
    /// Light background.
    Light,
    /// Dark background.
    Dark,
}

/// The colours of one theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Theme {
    /// Which theme this is.
    pub id: ThemeId,
    /// Editor background.
    pub background: Color32,
    /// Background of the caret's line.
    pub current_line: Color32,
    /// Selection background.
    pub selection: Color32,
    /// Caret colour.
    pub caret: Color32,
    /// Whitespace, symbols, operators and punctuation.
    pub text: Color32,
    /// Line numbers.
    pub gutter: Color32,
    /// Line number of the caret's line.
    pub gutter_active: Color32,
    /// Background of search matches.
    pub search_match: Color32,
    /// Comments and suspect comments.
    pub comment: Color32,
    /// Mnemonics and size suffixes.
    pub mnemonic: Color32,
    /// Directives.
    pub directive: Color32,
    /// Macro names.
    pub macro_name: Color32,
    /// Labels and local labels.
    pub label: Color32,
    /// Registers.
    pub register: Color32,
    /// Numbers.
    pub number: Color32,
    /// Strings.
    pub string: Color32,
    /// Macro parameters.
    pub macro_param: Color32,
    /// Invalid text.
    pub invalid: Color32,
}

impl Theme {
    /// The light theme (§5.4 table).
    pub fn light() -> Self {
        todo!()
    }

    /// The dark theme (§5.4 table).
    pub fn dark() -> Self {
        todo!()
    }

    /// The colour of a token kind (an exhaustive match, no wildcard arm).
    pub fn token_color(&self, kind: TokenKind) -> Color32 {
        todo!()
    }

    /// egui visuals for this theme: `Visuals::light()`/`dark()` with the theme's background on
    /// panels.
    pub fn visuals(&self) -> egui::Visuals {
        todo!()
    }
}

/// The WCAG 2.x contrast ratio of two colours (relative luminance), from 1 to 21.
pub fn contrast_ratio(a: Color32, b: Color32) -> f32 {
    todo!()
}
