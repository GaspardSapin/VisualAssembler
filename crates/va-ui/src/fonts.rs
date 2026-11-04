//! The code font and zoom (m4-editor-shell.md §5.5, frozen).
//!
//! The code family `va-code` is a custom `editor.font_file` when it validates, else the embedded
//! JetBrains Mono Regular, then egui's monospace and emoji fonts as fallbacks. The UI keeps
//! egui's default proportional fonts. The OFL licence text is embedded with the font, so every
//! binary carries it (`app.licences`).
//!
//! Zoom is egui's zoom factor (the whole UI), changed in steps of [`ZOOM_STEP`] between
//! [`ZOOM_MIN`] and [`ZOOM_MAX`] ([`step_zoom`]); egui's own keyboard zoom is turned off by
//! [`setup_context`], so the keys go through the keymap.

use std::panic::{self, AssertUnwindSafe};
use std::sync::Arc;

use egui::{FontData, FontDefinitions, FontFamily, FontId};

/// Name of the egui font family of the editor.
pub const CODE_FONT_FAMILY: &str = "va-code";

/// Key of the embedded code font in [`egui::FontDefinitions::font_data`].
pub const EMBEDDED_FONT_NAME: &str = "va-code-embedded";

/// Key of the custom code font (`editor.font_file`) in [`egui::FontDefinitions::font_data`].
pub const CUSTOM_FONT_NAME: &str = "va-code-custom";

/// JetBrains Mono 2.304 Regular (SIL Open Font License 1.1, `assets/fonts/`).
pub static EMBEDDED_CODE_FONT: &[u8] =
    include_bytes!("../../../assets/fonts/JetBrainsMono-Regular.ttf");

/// The licence of [`EMBEDDED_CODE_FONT`]: the OFL 1.1 text shipped with it.
pub static EMBEDDED_FONT_LICENSE: &str = include_str!("../../../assets/fonts/OFL.txt");

/// Text laid out by the trial of [`validate_font`].
const TRIAL_TEXT: &str = "move.l\t#$1F,d0 ; 0123456789 ABC abc";

/// Family name of the scratch context of [`validate_font`].
const TRIAL_FAMILY: &str = "va-code-trial";

/// egui's default fonts plus the `va-code` family: `custom` (already validated) when given,
/// else the embedded font, then egui's monospace and emoji fonts.
pub fn font_definitions(custom: Option<Arc<Vec<u8>>>) -> egui::FontDefinitions {
    let mut definitions = FontDefinitions::default();
    let mut family = Vec::new();
    if let Some(bytes) = custom {
        let bytes = Arc::try_unwrap(bytes).unwrap_or_else(|shared| (*shared).clone());
        definitions.font_data.insert(
            CUSTOM_FONT_NAME.to_owned(),
            Arc::new(FontData::from_owned(bytes)),
        );
        family.push(CUSTOM_FONT_NAME.to_owned());
    }
    definitions.font_data.insert(
        EMBEDDED_FONT_NAME.to_owned(),
        Arc::new(FontData::from_static(EMBEDDED_CODE_FONT)),
    );
    family.push(EMBEDDED_FONT_NAME.to_owned());
    if let Some(monospace) = definitions.families.get(&FontFamily::Monospace) {
        family.extend(monospace.iter().cloned());
    }
    definitions
        .families
        .insert(FontFamily::Name(CODE_FONT_FAMILY.into()), family);
    definitions
}

/// The code font of `size` points (family [`CODE_FONT_FAMILY`]).
pub fn code_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(CODE_FONT_FAMILY.into()))
}

/// Installs [`font_definitions`] in `ctx` (active from the next pass) and turns egui's keyboard
/// zoom off, so `Ctrl` `+`/`-`/`0` go through the keymap (§5.5).
pub fn setup_context(ctx: &egui::Context, custom: Option<Arc<Vec<u8>>>) {
    ctx.set_fonts(font_definitions(custom));
    ctx.options_mut(|options| options.zoom_with_keyboard = false);
}

/// Checks a custom font: its size, then a trial layout in a scratch `egui::Context` under
/// `catch_unwind`.
///
/// The trial family holds only this font; the font must parse and lay out a line of code with
/// a non-zero width. egui panics on bytes it cannot parse; the panic is caught (the default hook
/// still prints it). Glyph coverage is not checked: egui's `has_glyph` cannot tell a missing
/// glyph from a present one in a family of one font.
///
/// # Errors
/// [`FontError::TooLarge`] over [`MAX_FONT_BYTES`], [`FontError::Invalid`] when egui cannot use
/// it.
pub fn validate_font(bytes: Vec<u8>) -> Result<Arc<Vec<u8>>, FontError> {
    if bytes.len() > MAX_FONT_BYTES {
        return Err(FontError::TooLarge { bytes: bytes.len() });
    }
    let bytes = Arc::new(bytes);
    let trial = Arc::clone(&bytes);
    let usable = panic::catch_unwind(AssertUnwindSafe(move || trial_layout(&trial)));
    match usable {
        Ok(true) => Ok(bytes),
        Ok(false) | Err(_) => Err(FontError::Invalid),
    }
}

/// Lays out [`TRIAL_TEXT`] with `bytes` as the only font of a family, in a scratch context;
/// tells whether the text gets a non-zero width.
fn trial_layout(bytes: &[u8]) -> bool {
    let ctx = egui::Context::default();
    let mut definitions = FontDefinitions::default();
    definitions.font_data.insert(
        TRIAL_FAMILY.to_owned(),
        Arc::new(FontData::from_owned(bytes.to_vec())),
    );
    definitions.families.insert(
        FontFamily::Name(TRIAL_FAMILY.into()),
        vec![TRIAL_FAMILY.to_owned()],
    );
    ctx.set_fonts(definitions);
    let font = FontId::new(14.0, FontFamily::Name(TRIAL_FAMILY.into()));
    let mut usable = false;
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        usable = ui.fonts_mut(|fonts| {
            let galley =
                fonts.layout_no_wrap(TRIAL_TEXT.to_owned(), font.clone(), egui::Color32::WHITE);
            galley.size().x > 0.0
        });
    });
    // No renderer takes the font atlas of the scratch context.
    output.textures_delta.clear();
    usable
}

/// Why [`validate_font`] rejected a font.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FontError {
    /// The file is larger than [`MAX_FONT_BYTES`].
    TooLarge {
        /// Its size in bytes.
        bytes: usize,
    },
    /// The bytes are not a font egui can lay out.
    Invalid,
}

/// Largest custom font file read, in bytes (32 MiB).
pub const MAX_FONT_BYTES: usize = 32 * 1024 * 1024;

/// Smallest zoom factor.
pub const ZOOM_MIN: f32 = 0.5;

/// Largest zoom factor.
pub const ZOOM_MAX: f32 = 3.0;

/// Zoom change of one zoom-in or zoom-out step.
pub const ZOOM_STEP: f32 = 0.1;

/// `factor` limited to [`ZOOM_MIN`]..=[`ZOOM_MAX`]; a NaN or infinite factor gives 1.0.
pub fn clamp_zoom(factor: f32) -> f32 {
    if factor.is_finite() {
        factor.clamp(ZOOM_MIN, ZOOM_MAX)
    } else {
        1.0
    }
}

/// The zoom factor `steps` steps of [`ZOOM_STEP`] away from `factor` (negative: zoom out),
/// rounded to a multiple of [`ZOOM_STEP`] and clamped ([`clamp_zoom`]).
pub fn step_zoom(factor: f32, steps: i32) -> f32 {
    let base = (clamp_zoom(factor) / ZOOM_STEP).round();
    clamp_zoom((base + steps as f32) * ZOOM_STEP)
}
