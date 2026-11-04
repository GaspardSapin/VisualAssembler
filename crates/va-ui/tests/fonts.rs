//! Tests of the code font and zoom (m4-editor-shell.md §5.5, §10.2): the `va-code` family,
//! custom font validation, the zoom steps, and the key names of `keybindings.toml`.
#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use egui::FontFamily;
use va_ui::fonts::{
    CODE_FONT_FAMILY, CUSTOM_FONT_NAME, EMBEDDED_CODE_FONT, EMBEDDED_FONT_LICENSE,
    EMBEDDED_FONT_NAME, FontError, MAX_FONT_BYTES, ZOOM_MAX, ZOOM_MIN, ZOOM_STEP, clamp_zoom,
    code_font, font_definitions, setup_context, step_zoom, validate_font,
};
use va_workspace::config::KEY_NAMES;

/// The font list of the `va-code` family.
fn code_family(definitions: &egui::FontDefinitions) -> &[String] {
    definitions
        .families
        .get(&FontFamily::Name(CODE_FONT_FAMILY.into()))
        .unwrap()
}

#[test]
fn va_code_starts_with_the_embedded_font_then_egui_fallbacks() {
    let definitions = font_definitions(None);
    let family = code_family(&definitions);
    assert_eq!(family[0], EMBEDDED_FONT_NAME);
    let data = &definitions.font_data[EMBEDDED_FONT_NAME];
    assert_eq!(&*data.font, EMBEDDED_CODE_FONT);
    let monospace = &definitions.families[&FontFamily::Monospace];
    assert!(!monospace.is_empty());
    assert_eq!(&family[1..], monospace.as_slice());
    // The UI keeps egui's default proportional set.
    assert_eq!(
        definitions.families[&FontFamily::Proportional],
        egui::FontDefinitions::default().families[&FontFamily::Proportional]
    );
}

#[test]
fn a_custom_font_comes_first() {
    let custom = Arc::new(EMBEDDED_CODE_FONT.to_vec());
    let definitions = font_definitions(Some(custom));
    let family = code_family(&definitions);
    assert_eq!(family[0], CUSTOM_FONT_NAME);
    assert_eq!(family[1], EMBEDDED_FONT_NAME);
    assert_eq!(
        &*definitions.font_data[CUSTOM_FONT_NAME].font,
        EMBEDDED_CODE_FONT
    );
}

#[test]
fn the_embedded_font_validates() {
    let bytes = validate_font(EMBEDDED_CODE_FONT.to_vec()).unwrap();
    assert_eq!(bytes.len(), EMBEDDED_CODE_FONT.len());
}

#[test]
fn junk_and_oversize_fonts_are_rejected_without_panicking() {
    assert_eq!(validate_font(Vec::new()), Err(FontError::Invalid));
    let junk: Vec<u8> = (0..4096_u32).map(|i| (i * 31 % 251) as u8).collect();
    assert_eq!(validate_font(junk), Err(FontError::Invalid));
    let mut truncated = EMBEDDED_CODE_FONT.to_vec();
    truncated.truncate(64);
    assert_eq!(validate_font(truncated), Err(FontError::Invalid));
    let oversize = vec![0; MAX_FONT_BYTES + 1];
    assert_eq!(
        validate_font(oversize),
        Err(FontError::TooLarge {
            bytes: MAX_FONT_BYTES + 1
        })
    );
}

#[test]
fn the_licence_is_embedded() {
    assert!(EMBEDDED_FONT_LICENSE.contains("SIL OPEN FONT LICENSE"));
    assert!(EMBEDDED_FONT_LICENSE.contains("JetBrains Mono"));
}

#[test]
fn setup_context_installs_va_code_and_turns_keyboard_zoom_off() {
    let ctx = egui::Context::default();
    setup_context(&ctx, None);
    assert!(!ctx.options(|options| options.zoom_with_keyboard));
    for _ in 0..2 {
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let font = code_font(14.0);
            let width = ui.fonts_mut(|fonts| fonts.glyph_width(&font, 'M'));
            assert!(width > 0.0);
        });
        output.textures_delta.clear();
    }
    let families = ctx.fonts(|fonts| fonts.families());
    assert!(families.contains(&FontFamily::Name(CODE_FONT_FAMILY.into())));
}

#[test]
fn zoom_steps_are_clamped_multiples_of_the_step() {
    assert!((step_zoom(1.0, 1) - 1.1).abs() < 1e-5);
    assert!((step_zoom(1.0, -1) - 0.9).abs() < 1e-5);
    assert!((step_zoom(1.04, 0) - 1.0).abs() < 1e-5);
    assert!((step_zoom(ZOOM_MAX, 1) - ZOOM_MAX).abs() < 1e-5);
    assert!((step_zoom(ZOOM_MIN, -1) - ZOOM_MIN).abs() < 1e-5);
    assert!((step_zoom(1.0, 100) - ZOOM_MAX).abs() < 1e-5);
    assert!((clamp_zoom(f32::NAN) - 1.0).abs() < 1e-6);
    assert!((clamp_zoom(f32::INFINITY) - 1.0).abs() < 1e-6);
    assert!((clamp_zoom(0.1) - ZOOM_MIN).abs() < 1e-6);
    assert!((clamp_zoom(9.0) - ZOOM_MAX).abs() < 1e-6);
    let mut zoom = 1.0;
    for _ in 0..5 {
        zoom = step_zoom(zoom, 1);
    }
    assert!((zoom - (1.0 + 5.0 * ZOOM_STEP)).abs() < 1e-4);
}

#[test]
fn every_key_name_maps_through_egui_and_back() {
    for name in KEY_NAMES {
        let key = egui::Key::from_name(name).unwrap_or_else(|| panic!("{name} is not an egui key"));
        assert_eq!(key.name(), name);
    }
}
