//! The listing text (m2-assembler.md §4.12): our own format, LF line ends, one line per
//! `va_core::asm::ListingLine`, then the symbols and the error and warning counts. It carries no
//! diagnostic text (diagnostics are rendered by the localizer).
//!
//! WP0 scaffold: the §4.12 signature with a `todo!()` body; WP-D implements it.

#![expect(
    unused_variables,
    clippy::todo,
    reason = "WP0 stub: WP-D implements m2 §4.12"
)]

use va_core::asm::{Listing, SymbolTable};
use va_core::source::FileId;

/// Renders `listing` and `symbols`, naming files with `name` (`---- <file name>` lines).
pub fn render_listing(
    listing: &Listing,
    symbols: &SymbolTable,
    name: &dyn Fn(FileId) -> String,
) -> String {
    todo!()
}
