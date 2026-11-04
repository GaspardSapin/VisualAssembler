//! Tests of the themes (m4-editor-shell.md §5.4, §10.2): WCAG contrast rules, the exhaustive
//! token mapping, the visuals, the effective theme, and theme switching headless.
#![allow(clippy::unwrap_used)]

mod support;

use egui::Color32;
use support::{EditorState, editor_harness, numbered_text};
use va_core::diag::DiagCode;
use va_core::syntax::TokenKind;
use va_ui::theme::{Theme, ThemeId, contrast_ratio, effective_theme};
use va_workspace::config::ThemeKind;

/// Smallest contrast ratio of the §5.4 rules (WCAG AA for normal text).
const MIN_CONTRAST: f32 = 4.5;

/// A diagnostic code for the kinds that carry one.
const CODE: DiagCode = DiagCode("lex-test");

/// The 17 token kinds, each with the theme role it must map to.
fn kinds_and_roles(theme: &Theme) -> [(TokenKind, Color32); 17] {
    [
        (TokenKind::Whitespace, theme.text),
        (TokenKind::Comment, theme.comment),
        (TokenKind::SuspectComment(CODE), theme.comment),
        (TokenKind::Label, theme.label),
        (TokenKind::LocalLabel, theme.label),
        (TokenKind::Mnemonic, theme.mnemonic),
        (TokenKind::Directive, theme.directive),
        (TokenKind::MacroName, theme.macro_name),
        (TokenKind::SizeSuffix, theme.mnemonic),
        (TokenKind::Register, theme.register),
        (TokenKind::Symbol, theme.text),
        (TokenKind::Number, theme.number),
        (TokenKind::String, theme.string),
        (TokenKind::Operator, theme.text),
        (TokenKind::Punctuation, theme.text),
        (TokenKind::MacroParam, theme.macro_param),
        (TokenKind::Invalid(CODE), theme.invalid),
    ]
}

/// Both themes.
fn themes() -> [Theme; 2] {
    [Theme::light(), Theme::dark()]
}

#[test]
fn every_token_colour_contrasts_with_the_backgrounds() {
    for theme in themes() {
        for (kind, _) in kinds_and_roles(&theme) {
            let color = theme.token_color(kind);
            let on_background = contrast_ratio(color, theme.background);
            let on_current = contrast_ratio(color, theme.current_line);
            assert!(
                on_background >= MIN_CONTRAST,
                "{:?} {kind:?}: {on_background} on the background",
                theme.id
            );
            assert!(
                on_current >= MIN_CONTRAST,
                "{:?} {kind:?}: {on_current} on the current line",
                theme.id
            );
        }
        assert!(contrast_ratio(theme.gutter, theme.background) >= MIN_CONTRAST);
        assert!(contrast_ratio(theme.text, theme.selection) >= MIN_CONTRAST);
    }
}

#[test]
fn every_token_kind_maps_to_its_role() {
    for theme in themes() {
        for (kind, role) in kinds_and_roles(&theme) {
            assert_eq!(theme.token_color(kind), role, "{:?} {kind:?}", theme.id);
        }
    }
}

#[test]
fn the_token_mapping_has_no_wildcard_arm() {
    let source = include_str!("../src/theme.rs");
    let start = source.find("pub fn token_color").unwrap();
    let end = start + source[start..].find("\n    }\n").unwrap();
    let body = &source[start..end];
    assert!(!body.contains("_ =>"), "wildcard arm in token_color");
    assert_eq!(body.matches("TokenKind::").count(), 17);
}

#[test]
fn the_palettes_match_the_spec_table() {
    let light = Theme::light();
    assert_eq!(light.id, ThemeId::Light);
    assert_eq!(light.background, Color32::from_rgb(0xFF, 0xFF, 0xFF));
    assert_eq!(light.selection, Color32::from_rgb(0xC8, 0xDD, 0xF5));
    assert_eq!(light.search_match, Color32::from_rgb(0xFF, 0xE5, 0x8F));
    let dark = Theme::dark();
    assert_eq!(dark.id, ThemeId::Dark);
    assert_eq!(dark.background, Color32::from_rgb(0x1E, 0x1F, 0x22));
    assert_eq!(dark.caret, Color32::from_rgb(0xE8, 0xE8, 0xE8));
    assert_eq!(dark.invalid, Color32::from_rgb(0xFF, 0x6B, 0x6B));
    assert_eq!(Theme::from_id(ThemeId::Light), light);
    assert_eq!(Theme::from_id(ThemeId::Dark), dark);
}

#[test]
fn contrast_ratio_follows_wcag() {
    let black = Color32::BLACK;
    let white = Color32::WHITE;
    assert!((contrast_ratio(black, white) - 21.0).abs() < 1e-3);
    assert!((contrast_ratio(white, black) - 21.0).abs() < 1e-3);
    assert!((contrast_ratio(white, white) - 1.0).abs() < 1e-6);
    // #777777 on white is the classic 4.48 example.
    let grey = Color32::from_rgb(0x77, 0x77, 0x77);
    assert!((contrast_ratio(grey, white) - 4.48).abs() < 0.01);
}

#[test]
fn visuals_use_the_theme_background_on_panels() {
    let light = Theme::light().visuals();
    assert!(!light.dark_mode);
    assert_eq!(light.panel_fill, Theme::light().background);
    let dark = Theme::dark().visuals();
    assert!(dark.dark_mode);
    assert_eq!(dark.panel_fill, Theme::dark().background);
}

#[test]
fn the_effective_theme_is_override_then_setting_then_system() {
    use egui::Theme as System;
    assert_eq!(effective_theme(None, None, None), ThemeId::Dark);
    assert_eq!(
        effective_theme(None, None, Some(System::Light)),
        ThemeId::Light
    );
    assert_eq!(
        effective_theme(None, None, Some(System::Dark)),
        ThemeId::Dark
    );
    assert_eq!(
        effective_theme(None, Some(ThemeKind::Light), Some(System::Dark)),
        ThemeId::Light
    );
    assert_eq!(
        effective_theme(
            Some(ThemeKind::Dark),
            Some(ThemeKind::Light),
            Some(System::Light)
        ),
        ThemeId::Dark
    );
}

#[test]
fn switching_the_theme_headless_redraws_every_line() {
    let mut harness = editor_harness(EditorState::new(&numbered_text(500)));
    harness.ctx.set_visuals(Theme::dark().visuals());
    let state = harness.state_mut();
    state.theme = Theme::dark();
    state.style.style_generation += 1;
    harness.step();
    let stats = harness.state().view.stats();
    assert!(stats.lines_drawn > 0);
    assert_eq!(stats.cache_misses, stats.lines_drawn);
    harness.step();
    assert!(harness.ctx.global_style().visuals.dark_mode);
    assert_eq!(harness.state().view.stats().cache_misses, 0);
}
