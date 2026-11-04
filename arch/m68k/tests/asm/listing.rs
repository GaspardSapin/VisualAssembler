//! The listing (`src/asm/listing.rs`; m2 §4.12, §10.1): layout, truncation and the size cap (WP-D).
//!
//! The listings are built by hand (rule G1 of §15.3: `assemble` is another package). The rows
//! follow the columns of §4.12, `LLLLLd AAAAAA  HHHH HHHH HHHH HHHH F  source`: the flag is at
//! column 35 and the source text starts at column 38. `rendered_line_len` is crate-private:
//! `render_listing` decides with it whether a row fits, so the cap tests, which cut the listing
//! at every byte, check it against the bytes each row takes (debug builds also assert it on
//! every row rendered).
#![allow(clippy::unwrap_used)]

use va_arch_m68k::asm::listing::render_listing;
use va_core::asm::{AsmLimits, AsmOptions, Listing, ListingLine, Symbol, SymbolKind, SymbolTable};
use va_core::source::{FileId, SourceSpan, TextRange};

/// The header with optimizations on.
const HEADER: &str = "VisualAssembler listing: m68k, optimizations on\n";

/// The line that ends a cut listing.
const MARKER: &str = "listing truncated\n";

/// A line of the root file without address, bytes or value.
fn line(number: u32, text: &str) -> ListingLine {
    ListingLine {
        file: FileId(0),
        line: number,
        depth: 0,
        text: text.to_owned(),
        address: None,
        value: None,
        bytes: Vec::new(),
        byte_count: 0,
        skipped: false,
        optimization: None,
        errors: 0,
        warnings: 0,
    }
}

/// A line of the root file at `address` that produced `bytes`.
fn code(number: u32, address: u64, bytes: &[u8], text: &str) -> ListingLine {
    ListingLine {
        address: Some(address),
        bytes: bytes.to_vec(),
        byte_count: bytes.len() as u64,
        ..line(number, text)
    }
}

/// A symbol defined at the start of the root file.
fn symbol(name: &str, kind: SymbolKind, value: Option<i64>) -> Symbol {
    Symbol {
        name: name.to_owned(),
        kind,
        value,
        scope: None,
        definition: SourceSpan {
            file: FileId(0),
            range: TextRange::new(0, 0),
        },
        references: Vec::new(),
    }
}

/// The options of an assembly with the default limits and a listing cap of `cap` bytes.
fn options(optimize: bool, cap: u64) -> AsmOptions {
    AsmOptions {
        optimize,
        listing: true,
        limits: AsmLimits {
            max_listing_bytes: cap,
            ..AsmLimits::DEFAULT
        },
    }
}

/// The file names of the tests.
fn name(file: FileId) -> String {
    match file.0 {
        0 => "main.asm".to_owned(),
        n => format!("inc{n}.i"),
    }
}

/// Renders `lines` and `symbols` with optimizations on and the default cap.
fn render(lines: Vec<ListingLine>, symbols: Vec<Symbol>) -> String {
    let listing = Listing {
        lines,
        truncated: false,
    };
    render_listing(
        &listing,
        &SymbolTable::new(symbols),
        &AsmOptions::default(),
        &name,
    )
}

/// The rows `render` writes for `lines` alone, after the header and before `symbols:`.
fn rows(lines: Vec<ListingLine>) -> String {
    let text = render(lines, Vec::new());
    let body = text.strip_prefix(HEADER).unwrap();
    body[..body.rfind("symbols:\n").unwrap()].to_owned()
}

/// A row: `fields` (line number, depth mark, location) padded to the flag column, the flag,
/// two spaces and the source text, then LF.
fn row(fields: &str, flag: char, source: &str) -> String {
    format!("{fields:<35}{flag}  {source}\n")
}

#[test]
fn the_header_says_whether_optimizations_were_on() {
    let listing = Listing::default();
    let symbols = SymbolTable::default();
    let on = render_listing(&listing, &symbols, &options(true, 1 << 20), &name);
    assert_eq!(on, format!("{HEADER}symbols:\nerrors 0, warnings 0\n"));
    let off = render_listing(&listing, &symbols, &options(false, 1 << 20), &name);
    assert_eq!(
        off,
        "VisualAssembler listing: m68k, optimizations off\nsymbols:\nerrors 0, warnings 0\n"
    );
}

#[test]
fn a_whole_listing_has_its_layout() {
    let mut defined = line(4, "len\tequ\t16");
    defined.value = Some(16);
    let text = render(
        vec![
            line(1, "; demo"),
            code(2, 0x1000, &[], "start:"),
            code(
                3,
                0x1000,
                &[0x20, 0x3C, 0x12, 0x34, 0x56, 0x78],
                "\tmove.l\t#$12345678,d0",
            ),
            defined,
            code(5, 0x1006, &[0x4E, 0x75], "\trts"),
        ],
        vec![
            symbol("start", SymbolKind::Label, Some(0x1000)),
            symbol("len", SymbolKind::Equ, Some(16)),
        ],
    );
    let expected = [
        HEADER,
        "    1                                 ; demo\n",
        "    2  001000                         start:\n",
        "    3  001000  203C 1234 5678         \tmove.l\t#$12345678,d0\n",
        "    4  =00000010                      len\tequ\t16\n",
        "    5  001006  4E75                   \trts\n",
        "symbols:\n",
        "  len $00000010 equ\n",
        "  start $00001000 label\n",
        "errors 0, warnings 0\n",
    ]
    .concat();
    assert_eq!(text, expected);
}

#[test]
fn the_columns_are_fixed() {
    let text = rows(vec![
        code(3, 0x1000, &[1, 2, 3, 4, 5, 6, 7, 8], "full"),
        code(99_999, 0xFF_FFFE, &[1], "odd"),
        line(7, "none"),
    ]);
    for (row, source) in text.lines().zip(["full", "odd", "none"]) {
        assert_eq!(&row[35..38], "   ", "{row:?}");
        assert_eq!(&row[38..], source, "{row:?}");
    }
    assert!(text.starts_with("    3  001000  0102 0304 0506 0708 "));
    assert!(text.contains("\n99999  FFFFFE  01 "));
}

#[test]
fn an_expansion_line_is_marked_with_a_plus() {
    let mut expanded = code(12, 0x1004, &[0x70, 0x01], "\tmoveq\t#1,d0");
    expanded.depth = 2;
    assert_eq!(
        rows(vec![expanded]),
        row("   12+ 001004  7001", ' ', "\tmoveq\t#1,d0")
    );
}

#[test]
fn a_value_line_shows_its_32_bit_value() {
    let mut minus = line(8, "m\tset\t-1");
    minus.value = Some(-1);
    let mut wide = line(9, "w\tequ\t$123456789");
    wide.value = Some(0x1_2345_6789);
    assert_eq!(
        rows(vec![minus, wide]),
        [
            row("    8  =FFFFFFFF", ' ', "m\tset\t-1"),
            row("    9  =23456789", ' ', "w\tequ\t$123456789"),
        ]
        .concat()
    );
}

#[test]
fn the_flag_is_the_first_that_applies() {
    let flagged = |errors, warnings, optimization, skipped| ListingLine {
        errors,
        warnings,
        optimization,
        skipped,
        ..code(1, 0x1000, &[0x4E, 0x71], "x")
    };
    let cases = [
        (flagged(1, 1, Some("addq"), true), 'E'),
        (flagged(0, 2, Some("addq"), true), 'W'),
        (flagged(0, 0, Some("addq"), true), 'o'),
        (flagged(0, 0, None, true), '-'),
        (flagged(0, 0, None, false), ' '),
    ];
    for (line, flag) in cases {
        assert_eq!(rows(vec![line]), row("    1  001000  4E71", flag, "x"));
    }
}

#[test]
fn the_source_text_is_cut_after_160_characters() {
    let exact = "x".repeat(160);
    let long = "x".repeat(161);
    let accents = "\u{e9}".repeat(200);
    let text = rows(vec![line(1, &exact), line(2, &long), line(3, &accents)]);
    let sources: Vec<&str> = text.lines().map(|row| &row[38..]).collect();
    assert_eq!(sources[0], exact);
    assert_eq!(sources[1], format!("{}\u{2026}", "x".repeat(160)));
    // Characters, not bytes.
    assert_eq!(sources[2], format!("{}\u{2026}", "\u{e9}".repeat(160)));
    // Tabs are kept.
    assert_eq!(
        rows(vec![line(4, "\tnop\t; a\tb")]),
        row("    4", ' ', "\tnop\t; a\tb")
    );
}

#[test]
fn a_row_without_source_text_has_no_trailing_spaces() {
    let mut flagged = code(3, 0x1002, &[0x4E, 0x71], "");
    flagged.warnings = 1;
    assert_eq!(
        rows(vec![
            line(1, ""),
            code(2, 0x1000, &[], ""),
            flagged,
            code(4, 0x1004, &[0x4E, 0x75], ""),
        ]),
        [
            "    1\n",
            "    2  001000\n",
            "    3  001002  4E71                W\n",
            "    4  001004  4E75\n",
        ]
        .concat()
    );
}

#[test]
fn more_bytes_continue_on_rows_with_their_address() {
    let bytes: Vec<u8> = (0..20).collect();
    assert_eq!(
        rows(vec![code(10, 0x1000, &bytes, "\tdcb.b\t20,0")]),
        [
            row("   10  001000  0001 0203 0405 0607", ' ', "\tdcb.b\t20,0"),
            "       001008  0809 0A0B 0C0D 0E0F\n".to_owned(),
            "       001010  1011 1213\n".to_owned(),
        ]
        .concat()
    );
    // An odd last byte stands alone.
    let text = rows(vec![code(11, 0x2000, &bytes[..11], "x")]);
    assert!(text.ends_with("\n       002008  0809 0A\n"), "{text:?}");
}

#[test]
fn at_most_64_bytes_are_shown_then_an_ellipsis() {
    let bytes: Vec<u8> = (0..64).collect();
    let mut many = code(20, 0x1000, &bytes, "\tds.b\t100");
    many.byte_count = 100;
    let text = rows(vec![many.clone()]);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 8, "{text:?}");
    assert_eq!(lines[7], "       001038  3839 3A3B 3C3D 3E3F \u{2026}");
    // Exactly 64 bytes: no ellipsis.
    many.byte_count = 64;
    assert!(!rows(vec![many.clone()]).contains('\u{2026}'));
    // More than 64 bytes stored: the first 64 are shown.
    let mut stored = many;
    stored.bytes = (0..70).collect();
    stored.byte_count = 70;
    assert_eq!(rows(vec![stored]), text);
    // When the first row is the last one, the ellipsis gets a row at the first missing byte.
    let mut short = code(21, 0x1000, &[1, 2, 3, 4], "x");
    short.byte_count = 6;
    assert_eq!(
        rows(vec![short]),
        [
            row("   21  001000  0102 0304", ' ', "x"),
            "       001004  \u{2026}\n".to_owned(),
        ]
        .concat()
    );
}

#[test]
fn a_file_change_prints_the_file_name() {
    let included = |number, text| ListingLine {
        file: FileId(1),
        ..line(number, text)
    };
    let text = rows(vec![
        line(1, "\tinclude\tinc1.i"),
        included(1, "; included"),
        included(2, "\tnop"),
        line(2, "\tend"),
    ]);
    assert_eq!(
        text,
        [
            row("    1", ' ', "\tinclude\tinc1.i"),
            "---- inc1.i\n".to_owned(),
            row("    1", ' ', "; included"),
            row("    2", ' ', "\tnop"),
            "---- main.asm\n".to_owned(),
            row("    2", ' ', "\tend"),
        ]
        .concat()
    );
    // The first row is compared with the root file.
    assert!(rows(vec![included(5, "x")]).starts_with("---- inc1.i\n    5"));
}

#[test]
fn symbols_are_sorted_by_name_with_their_value_and_kind() {
    let text = render(
        Vec::new(),
        vec![
            symbol("zeta", SymbolKind::Set, Some(-2)),
            symbol("Alpha", SymbolKind::Macro, None),
            symbol("1$", SymbolKind::LocalLabel, Some(0x1004)),
            symbol("alpha", SymbolKind::Label, Some(0x00FF_FFFF)),
        ],
    );
    assert_eq!(
        text,
        [
            HEADER,
            "symbols:\n",
            "  1$ $00001004 local-label\n",
            "  Alpha - macro\n",
            "  alpha $00FFFFFF label\n",
            "  zeta $FFFFFFFE set\n",
            "errors 0, warnings 0\n",
        ]
        .concat()
    );
}

#[test]
fn the_count_line_sums_the_lines() {
    let counted = |errors, warnings| ListingLine {
        errors,
        warnings,
        ..line(1, "x")
    };
    let text = render(
        vec![counted(2, 0), counted(0, 1), counted(1, 3), counted(0, 0)],
        Vec::new(),
    );
    assert!(
        text.ends_with("symbols:\nerrors 3, warnings 4\n"),
        "{text:?}"
    );
}

#[test]
fn a_truncated_listing_ends_with_the_marker() {
    let listing = Listing {
        lines: vec![line(1, "a"), line(2, "b")],
        truncated: true,
    };
    let symbols = SymbolTable::new(vec![symbol("a", SymbolKind::Label, Some(0))]);
    let text = render_listing(&listing, &symbols, &options(true, 1 << 20), &name);
    assert_eq!(
        text,
        [
            HEADER.to_owned(),
            row("    1", ' ', "a"),
            row("    2", ' ', "b"),
            MARKER.to_owned(),
        ]
        .concat()
    );
}

/// A listing of every kind of row, file change, symbol and count line.
fn sample() -> (Listing, SymbolTable) {
    let mut lines = vec![
        line(1, "; sample"),
        code(2, 0x1000, &(0..20).collect::<Vec<u8>>(), "\tdcb.b\t20,0"),
        code(3, 0x1014, &[0x4E, 0x71], ""),
    ];
    let mut many = code(4, 0x1016, &[0xAB; 64], "\tdcb.b\t70,$ab");
    many.byte_count = 70;
    lines.push(many);
    lines.push(ListingLine {
        file: FileId(1),
        depth: 1,
        ..code(1, 0x105C, &[0x4E, 0x75], "\trts")
    });
    lines.push(line(5, "\tend"));
    let symbols = SymbolTable::new(vec![
        symbol("start", SymbolKind::Label, Some(0x1000)),
        symbol("m", SymbolKind::Macro, None),
    ]);
    (
        Listing {
            lines,
            truncated: false,
        },
        symbols,
    )
}

/// Splits a whole listing into the units the cap keeps or drops: the header, then each line,
/// a row with its continuation rows (which start with 7 spaces) counting as one.
fn units(text: &str) -> Vec<&str> {
    let mut units: Vec<&str> = Vec::new();
    let mut start = 0;
    for (index, _) in text.match_indices('\n') {
        let end = index + 1;
        let next = &text[end..];
        if !next.starts_with("       ") {
            units.push(&text[start..end]);
            start = end;
        }
    }
    units
}

#[test]
fn the_cap_cuts_before_the_first_line_that_does_not_fit() {
    let (listing, symbols) = sample();
    let full = render_listing(&listing, &symbols, &options(true, 1 << 20), &name);
    let units = units(&full);
    assert_eq!(units.concat(), full);
    assert_eq!(units[0], HEADER);
    assert_eq!(units.len(), 13, "{units:#?}");
    let minimum = (HEADER.len() + MARKER.len()) as u64;
    for cap in minimum..=(full.len() + MARKER.len() + 8) as u64 {
        let text = render_listing(&listing, &symbols, &options(true, cap), &name);
        assert!(text.len() as u64 <= cap, "cap {cap}: {} bytes", text.len());
        // The longest run of whole units that leaves room for the marker.
        let room = usize::try_from(cap).unwrap() - MARKER.len();
        let mut kept = 0;
        let mut length = 0;
        for unit in &units {
            if length + unit.len() > room {
                break;
            }
            length += unit.len();
            kept += 1;
        }
        let expected = if kept == units.len() {
            full.clone()
        } else {
            [&units[..kept.max(1)].concat(), MARKER].concat()
        };
        assert_eq!(text, expected, "cap {cap}");
    }
}

#[test]
fn the_header_and_the_marker_are_always_written() {
    let (listing, symbols) = sample();
    for cap in [0, 1, 18, (HEADER.len() + MARKER.len() - 1) as u64] {
        let text = render_listing(&listing, &symbols, &options(true, cap), &name);
        assert_eq!(text, [HEADER, MARKER].concat(), "cap {cap}");
    }
    // A truncated listing cut by the cap gets one marker.
    let truncated = Listing {
        truncated: true,
        ..listing
    };
    let cap = (HEADER.len() + MARKER.len() + 1) as u64;
    let text = render_listing(&truncated, &symbols, &options(true, cap), &name);
    assert_eq!(text, [HEADER, MARKER].concat());
}
