//! The listing text (m2-assembler.md §4.12, §15.2 W5): our own format, LF line ends. It carries
//! no diagnostic text (diagnostics are rendered by the localizer). In order:
//!
//! - the header `VisualAssembler listing: m68k, optimizations on` (or `off`);
//! - one row per `va_core::asm::ListingLine`, `LLLLLd AAAAAA  HHHH HHHH HHHH HHHH F  source`:
//!   the line number right-aligned on 5 columns; `d` = `+` for a line of a macro expansion, else
//!   a space; a location field of 27 columns that holds the address (at least 6 hexadecimal
//!   digits, blank without an address), two spaces and the first 8 bytes as words (`4E75`, an
//!   odd last byte alone) — or `=VVVVVVVV`, the 32-bit value, for a line that defines one (such
//!   a line has no bytes); the flag `E` (errors), `W` (warnings), `o` (optimized) or `-`
//!   (skipped), the first that applies, else a space; then the source text, tabs kept, cut after
//!   160 characters with `…`. A row whose source text is empty ends after its last non-blank
//!   column (no trailing spaces);
//! - continuation rows for the bytes after the first 8, 8 per row with their address and the
//!   number, `d` and flag columns blank (`       001008  4E71 4E71`), up to 64 bytes; when the
//!   line produced more bytes than it shows, ` …` ends the last of its rows (or, when that is its
//!   first row, a continuation row holds `…` at the address of the first byte not shown);
//! - `---- <file name>` before a row whose file differs from the previous row's (from the root
//!   file `FileId(0)` for the first row);
//! - `symbols:`, then `  <name> $VVVVVVVV <kind id>` per symbol in table order (sorted by name;
//!   `-` in place of the value of a macro);
//! - the count line `errors <n>, warnings <n>`, the sums of the per-line counters.
//!
//! **Cap.** The text is capped by `AsmLimits::max_listing_bytes` (§15.2 W5): after the header,
//! the renderer stops before the first line that would take the text past the cap minus 18
//! bytes, a row with its continuation rows counting as one line, and then ends with
//! `listing truncated` and LF (18 bytes). The same marker follows the last row when
//! `Listing::truncated` is set; nothing follows it. Pass 2 of the assembler measures the lines it
//! collects with the crate-private `rendered_line_len`, so the memory of a listing stays
//! proportional to the cap.

use std::fmt::{self, Write as _};

use va_core::asm::{AsmOptions, Listing, ListingLine, Symbol, SymbolTable};
use va_core::source::FileId;

/// The line that ends a cut listing.
const MARKER: &str = "listing truncated\n";

/// The source characters a row shows before `…`.
const SOURCE_CHARS: usize = 160;

/// The bytes one row shows.
const ROW_BYTES: usize = 8;

/// The bytes a line shows over all its rows.
const SHOWN_BYTES: usize = 64;

/// The width of the location field: the address (6), two spaces and 8 bytes as words (19).
const LOCATION_WIDTH: usize = 27;

/// Renders `listing` and `symbols`, naming files with `name` (`---- <file name>` lines).
///
/// `options` are those of the assembly that produced `listing`: the header says
/// `optimizations on` iff `options.optimize`, and the text is capped by
/// `options.limits.max_listing_bytes` (§15.2 W5). The renderer stops before the first line that
/// would take the text past the cap minus 18 bytes and then ends with `listing truncated` and
/// LF; the same marker follows the last listing line when `listing.truncated` is set. The header
/// and the marker are always written, so the text never exceeds the cap when the cap is at
/// least their combined length.
pub fn render_listing(
    listing: &Listing,
    symbols: &SymbolTable,
    options: &AsmOptions,
    name: &dyn Fn(FileId) -> String,
) -> String {
    let marker = MARKER.len() as u64;
    let mut out = Capped {
        text: String::new(),
        limit: usize::try_from(options.limits.max_listing_bytes.saturating_sub(marker))
            .unwrap_or(usize::MAX),
    };
    out.text.push_str(if options.optimize {
        "VisualAssembler listing: m68k, optimizations on\n"
    } else {
        "VisualAssembler listing: m68k, optimizations off\n"
    });
    if !write_body(&mut out, listing, symbols, name) {
        out.text.push_str(MARKER);
    }
    out.text
}

/// The number of bytes [`render_listing`] writes for this line, its continuation lines and their
/// LF included (the `----` file-change lines are not counted).
pub(crate) fn rendered_line_len(line: &ListingLine) -> usize {
    let mut row = String::new();
    push_line(&mut row, line);
    row.len()
}

/// The text being rendered and the length it may reach before the marker.
struct Capped {
    /// The text so far.
    text: String,
    /// The cap minus the marker.
    limit: usize,
}

impl Capped {
    /// Tells whether `len` more bytes keep the text within the limit.
    fn fits(&self, len: usize) -> bool {
        self.text.len().saturating_add(len) <= self.limit
    }

    /// Appends `piece` if it fits; returns whether it did.
    fn add(&mut self, piece: &str) -> bool {
        let fits = self.fits(piece.len());
        if fits {
            self.text.push_str(piece);
        }
        fits
    }
}

/// Writes everything after the header; returns `false` where the marker must follow (a line
/// did not fit, or the listing is truncated).
fn write_body(
    out: &mut Capped,
    listing: &Listing,
    symbols: &SymbolTable,
    name: &dyn Fn(FileId) -> String,
) -> bool {
    let mut file = FileId(0);
    let mut piece = String::new();
    for line in &listing.lines {
        if line.file != file {
            file = line.file;
            piece.clear();
            piece.push_str("---- ");
            piece.push_str(&name(file));
            piece.push('\n');
            if !out.add(&piece) {
                return false;
            }
        }
        let len = rendered_line_len(line);
        if !out.fits(len) {
            return false;
        }
        let before = out.text.len();
        push_line(&mut out.text, line);
        debug_assert_eq!(
            out.text.len() - before,
            len,
            "rendered_line_len measures the row"
        );
    }
    if listing.truncated || !out.add("symbols:\n") {
        return false;
    }
    for symbol in symbols.symbols() {
        piece.clear();
        push_symbol(&mut piece, symbol);
        if !out.add(&piece) {
            return false;
        }
    }
    let errors: u64 = listing
        .lines
        .iter()
        .map(|line| u64::from(line.errors))
        .sum();
    let warnings: u64 = listing
        .lines
        .iter()
        .map(|line| u64::from(line.warnings))
        .sum();
    piece.clear();
    put(
        &mut piece,
        format_args!("errors {errors}, warnings {warnings}\n"),
    );
    out.add(&piece)
}

/// Appends the row of `line` and its continuation rows, each with its LF.
fn push_line(out: &mut String, line: &ListingLine) {
    let start = out.len();
    let depth = if line.depth > 0 { '+' } else { ' ' };
    put(out, format_args!("{:>5}{depth} ", line.line));
    let shown = &line.bytes[..line.bytes.len().min(SHOWN_BYTES)];
    let (first, rest) = shown.split_at(shown.len().min(ROW_BYTES));
    let location = out.len();
    if let Some(value) = line.value {
        put(out, format_args!("={:08X}", value & 0xFFFF_FFFF));
    } else {
        push_address(out, line.address);
        out.push_str("  ");
        push_words(out, first);
    }
    while out.len() < location + LOCATION_WIDTH {
        out.push(' ');
    }
    out.push(' ');
    out.push(flag(line));
    out.push_str("  ");
    match line.text.char_indices().nth(SOURCE_CHARS) {
        Some((cut, _)) => {
            out.push_str(&line.text[..cut]);
            out.push('…');
        }
        None => out.push_str(&line.text),
    }
    if line.text.is_empty() {
        let end = start + out[start..].trim_end_matches(' ').len();
        out.truncate(end);
    }
    out.push('\n');
    if line.value.is_some() {
        return;
    }
    let more = line.byte_count > shown.len() as u64 || line.bytes.len() > shown.len();
    let rows = rest.chunks(ROW_BYTES).len();
    for (index, bytes) in rest.chunks(ROW_BYTES).enumerate() {
        let offset = (ROW_BYTES * (index + 1)) as u64;
        out.push_str("       ");
        push_address(
            out,
            line.address.map(|address| address.wrapping_add(offset)),
        );
        out.push_str("  ");
        push_words(out, bytes);
        if more && index + 1 == rows {
            out.push_str(" …");
        }
        out.push('\n');
    }
    if more && rows == 0 {
        let offset = shown.len() as u64;
        out.push_str("       ");
        push_address(
            out,
            line.address.map(|address| address.wrapping_add(offset)),
        );
        out.push_str("  …\n");
    }
}

/// Appends an address on at least 6 hexadecimal digits, or 6 spaces without one.
fn push_address(out: &mut String, address: Option<u64>) {
    match address {
        Some(address) => put(out, format_args!("{address:06X}")),
        None => out.push_str("      "),
    }
}

/// Appends `bytes` as words of 4 hexadecimal digits separated by a space, an odd last byte
/// alone.
fn push_words(out: &mut String, bytes: &[u8]) {
    for (index, word) in bytes.chunks(2).enumerate() {
        if index > 0 {
            out.push(' ');
        }
        for byte in word {
            put(out, format_args!("{byte:02X}"));
        }
    }
}

/// The flag of a row: `E`, `W`, `o` or `-`, the first that applies, else a space.
fn flag(line: &ListingLine) -> char {
    if line.errors > 0 {
        'E'
    } else if line.warnings > 0 {
        'W'
    } else if line.optimization.is_some() {
        'o'
    } else if line.skipped {
        '-'
    } else {
        ' '
    }
}

/// Appends the line of `symbol`: its name, its 32-bit value (`-` for none) and its kind id.
fn push_symbol(out: &mut String, symbol: &Symbol) {
    out.push_str("  ");
    out.push_str(&symbol.name);
    match symbol.value {
        Some(value) => put(out, format_args!(" ${:08X} ", value & 0xFFFF_FFFF)),
        None => out.push_str(" - "),
    }
    out.push_str(symbol.kind.id());
    out.push('\n');
}

/// Appends formatted text to `out`.
fn put(out: &mut String, args: fmt::Arguments<'_>) {
    // Writing to a `String` never fails.
    let _ = out.write_fmt(args);
}
