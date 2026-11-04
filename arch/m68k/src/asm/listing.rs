//! The listing text (m2-assembler.md §4.12): our own format, LF line ends, one line per
//! `va_core::asm::ListingLine`, then the symbols and the error and warning counts. It carries no
//! diagnostic text (diagnostics are rendered by the localizer).
//!
//! The text is capped by `AsmLimits::max_listing_bytes` and then ends with
//! `listing truncated` (§15.2 W5); pass 2 of the assembler measures the lines it collects with
//! the crate-private `rendered_line_len`, so the memory of a listing stays proportional to the
//! cap.
//!
//! WP0 scaffold, given its final signature by WP-D's day-1 commit (§16.4 G10): the §15.2 W5
//! signature and the measuring helper, with `todo!()` bodies; WP-D implements them.

#![expect(
    dead_code,
    unused_variables,
    clippy::todo,
    reason = "WP0 stub: WP-D implements m2 §4.12 and §15.2 W5"
)]

use va_core::asm::{AsmOptions, Listing, ListingLine, SymbolTable};
use va_core::source::FileId;

/// Renders `listing` and `symbols`, naming files with `name` (`---- <file name>` lines).
///
/// `options` are those of the assembly that produced `listing`: the header says
/// `optimizations on` iff `options.optimize`, and the text is capped by
/// `options.limits.max_listing_bytes` (§15.2 W5). The renderer stops before the first line that
/// would take the text past the cap minus 18 bytes and then ends with `listing truncated` and
/// LF; the same marker follows the last listing line when `listing.truncated` is set. The header
/// and the marker are always written.
pub fn render_listing(
    listing: &Listing,
    symbols: &SymbolTable,
    options: &AsmOptions,
    name: &dyn Fn(FileId) -> String,
) -> String {
    todo!()
}

/// The number of bytes [`render_listing`] writes for this line, its continuation lines and their
/// LF included (the `----` file-change lines are not counted).
pub(crate) fn rendered_line_len(line: &ListingLine) -> usize {
    todo!()
}
