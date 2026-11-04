//! The light and dark themes (m4-editor-shell.md §5.4, frozen): editor colours, one per token
//! role, and the egui visuals.
//!
//! Every token colour has a WCAG contrast ratio of at least 4.5 against the background and the
//! current-line colour (`tests/theme.rs`).
//!
//! The effective theme is the session override when set, else the `ui.theme` setting, else the
//! system theme (dark when unknown): [`effective_theme`].

use egui::Color32;
use va_core::syntax::TokenKind;
use va_workspace::config::ThemeKind;

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

/// An opaque colour from its `0xRRGGBB` value.
const fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

impl Theme {
    /// The light theme (§5.4 table).
    pub fn light() -> Self {
        Self {
            id: ThemeId::Light,
            background: rgb(0xFFFFFF),
            current_line: rgb(0xF2F4F7),
            selection: rgb(0xC8DDF5),
            caret: rgb(0x1F1F1F),
            text: rgb(0x1F1F1F),
            gutter: rgb(0x6E7681),
            gutter_active: rgb(0x1F1F1F),
            search_match: rgb(0xFFE58F),
            comment: rgb(0x5F6B76),
            mnemonic: rgb(0x0B57D0),
            directive: rgb(0x8E24AA),
            macro_name: rgb(0x795500),
            label: rgb(0x00796B),
            register: rgb(0x005A9E),
            number: rgb(0x2E7D32),
            string: rgb(0xA1460B),
            macro_param: rgb(0x9C4F00),
            invalid: rgb(0xC62828),
        }
    }

    /// The dark theme (§5.4 table).
    pub fn dark() -> Self {
        Self {
            id: ThemeId::Dark,
            background: rgb(0x1E1F22),
            current_line: rgb(0x2A2C30),
            selection: rgb(0x264F78),
            caret: rgb(0xE8E8E8),
            text: rgb(0xD4D4D4),
            gutter: rgb(0x8C8C8C),
            gutter_active: rgb(0xC8C8C8),
            search_match: rgb(0x5A4A12),
            comment: rgb(0x8B96A1),
            mnemonic: rgb(0x6CA8E8),
            directive: rgb(0xC792EA),
            macro_name: rgb(0xE0D08A),
            label: rgb(0x5CC8B4),
            register: rgb(0x9DD2F5),
            number: rgb(0xA9D18E),
            string: rgb(0xE09A78),
            macro_param: rgb(0xE6B060),
            invalid: rgb(0xFF6B6B),
        }
    }

    /// The theme of `id`: [`Theme::light`] or [`Theme::dark`].
    pub fn from_id(id: ThemeId) -> Self {
        match id {
            ThemeId::Light => Self::light(),
            ThemeId::Dark => Self::dark(),
        }
    }

    /// The colour of a token kind (an exhaustive match, no wildcard arm).
    pub fn token_color(&self, kind: TokenKind) -> Color32 {
        match kind {
            TokenKind::Whitespace
            | TokenKind::Symbol
            | TokenKind::Operator
            | TokenKind::Punctuation => self.text,
            TokenKind::Comment | TokenKind::SuspectComment(_) => self.comment,
            TokenKind::Mnemonic | TokenKind::SizeSuffix => self.mnemonic,
            TokenKind::Directive => self.directive,
            TokenKind::MacroName => self.macro_name,
            TokenKind::Label | TokenKind::LocalLabel => self.label,
            TokenKind::Register => self.register,
            TokenKind::Number => self.number,
            TokenKind::String => self.string,
            TokenKind::MacroParam => self.macro_param,
            TokenKind::Invalid(_) => self.invalid,
        }
    }

    /// egui visuals for this theme: `Visuals::light()`/`dark()` with the theme's background on
    /// panels.
    pub fn visuals(&self) -> egui::Visuals {
        let mut visuals = match self.id {
            ThemeId::Light => egui::Visuals::light(),
            ThemeId::Dark => egui::Visuals::dark(),
        };
        visuals.panel_fill = self.background;
        visuals
    }
}

/// The theme to show (§5.4): the session override when set, else the `ui.theme` setting, else
/// the system theme (`None` = `auto`), dark when the system theme is unknown.
pub fn effective_theme(
    session_override: Option<ThemeKind>,
    setting: Option<ThemeKind>,
    system: Option<egui::Theme>,
) -> ThemeId {
    match session_override.or(setting) {
        Some(ThemeKind::Light) => ThemeId::Light,
        Some(ThemeKind::Dark) => ThemeId::Dark,
        None => match system {
            Some(egui::Theme::Light) => ThemeId::Light,
            Some(egui::Theme::Dark) | None => ThemeId::Dark,
        },
    }
}

/// The relative luminance of one sRGB channel (WCAG 2.x).
fn channel_luminance(channel: u8) -> f32 {
    let c = f32::from(channel) / 255.0;
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// The relative luminance of a colour, alpha ignored (WCAG 2.x).
fn relative_luminance(color: Color32) -> f32 {
    let [r, g, b, _] = color.to_srgba_unmultiplied();
    0.2126 * channel_luminance(r) + 0.7152 * channel_luminance(g) + 0.0722 * channel_luminance(b)
}

/// The WCAG 2.x contrast ratio of two colours (relative luminance), from 1 to 21.
pub fn contrast_ratio(a: Color32, b: Color32) -> f32 {
    let (la, lb) = (relative_luminance(a), relative_luminance(b));
    let (light, dark) = if la >= lb { (la, lb) } else { (lb, la) };
    (light + 0.05) / (dark + 0.05)
}
