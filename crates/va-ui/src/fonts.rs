//! The code font and zoom (m4-editor-shell.md §5.5, frozen).
//!
//! The code family `va-code` is a custom `editor.font_file` when it validates, else the embedded
//! JetBrains Mono Regular, then egui's monospace and emoji fonts as fallbacks. The UI keeps
//! egui's default proportional fonts. The OFL licence text is embedded with the font, so every
//! binary carries it (`app.licences`).
//!
//! WP0 scaffold: the data and constants are final; the functions have `todo!()` bodies, which
//! WP-E implements.

#![expect(
    unused_variables,
    clippy::todo,
    reason = "WP0 stub: WP-E implements m4 §5.5"
)]

use std::sync::Arc;

/// Name of the egui font family of the editor.
pub const CODE_FONT_FAMILY: &str = "va-code";

/// JetBrains Mono 2.304 Regular (SIL Open Font License 1.1, `assets/fonts/`).
pub static EMBEDDED_CODE_FONT: &[u8] =
    include_bytes!("../../../assets/fonts/JetBrainsMono-Regular.ttf");

/// The licence of [`EMBEDDED_CODE_FONT`]: the OFL 1.1 text shipped with it.
pub static EMBEDDED_FONT_LICENSE: &str = include_str!("../../../assets/fonts/OFL.txt");

/// egui's default fonts plus the `va-code` family: `custom` (already validated) when given,
/// else the embedded font, then egui's monospace and emoji fonts.
pub fn font_definitions(custom: Option<Arc<Vec<u8>>>) -> egui::FontDefinitions {
    todo!()
}

/// Checks a custom font: its size, then a trial layout in a scratch `egui::Context` under
/// `catch_unwind`.
///
/// # Errors
/// [`FontError::TooLarge`] over [`MAX_FONT_BYTES`], [`FontError::Invalid`] when egui cannot use
/// it.
pub fn validate_font(bytes: Vec<u8>) -> Result<Arc<Vec<u8>>, FontError> {
    todo!()
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
