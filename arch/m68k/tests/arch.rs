//! Tests for `src/arch.rs` and `src/registers.rs` (m1-core-isa-lexer.md §3.6, §9.1): the 68000
//! seen through `va_core::arch::Architecture` — its id, the lexer and tables it hands out (every
//! keyword and every spelling lexes as its kind), its diagnostic codes, its register file and
//! the fields of SR.
#![allow(clippy::unwrap_used)]

use std::collections::BTreeSet;

use va_arch_m68k::{M68K, M68k, isa, syntax};
use va_core::arch::Architecture;
use va_core::regs::{RegId, RegisterGroup};
use va_core::syntax::{KeywordKind, LineState, Token, TokenKind};

/// Lexes `line` through the architecture and returns the kind and text of each token.
fn lex(line: &str) -> Vec<(TokenKind, String)> {
    let mut tokens: Vec<Token> = Vec::new();
    let state = M68K
        .syntax()
        .lex_line(line, LineState::INITIAL, &mut tokens);
    assert_eq!(state, LineState::INITIAL, "{line:?}");
    tokens
        .iter()
        .map(|token| (token.kind, token.range.slice(line).unwrap().to_owned()))
        .collect()
}

/// Returns the kinds of the tokens of `line`.
fn kinds(line: &str) -> Vec<TokenKind> {
    lex(line).into_iter().map(|(kind, _)| kind).collect()
}

#[test]
fn the_architecture_is_m68k() {
    let arch: &dyn Architecture = &M68K;
    assert_eq!(arch.id(), "m68k");
    assert_eq!(M68K, M68k);
}

#[test]
fn it_hands_out_the_crate_lexer_and_tables() {
    assert_eq!(M68K.instruction_set().spellings(), &isa::SPELLINGS[..]);
    assert_eq!(M68K.instruction_set().spellings().len(), 126);
    assert_eq!(
        M68K.syntax().keywords(),
        va_core::syntax::SyntaxProvider::keywords(&syntax::M68kSyntax)
    );
}

#[test]
fn diagnostic_codes_are_the_lexer_codes() {
    let codes = M68K.diagnostic_codes();
    assert_eq!(codes, &syntax::DIAG_CODES[..]);
    assert_eq!(codes.len(), 11);
    let unique: BTreeSet<_> = codes.iter().collect();
    assert_eq!(unique.len(), codes.len(), "duplicate code");
    for code in codes {
        assert!(code.is_valid(), "{code:?} is not kebab-case");
        assert!(code.0.starts_with("lex-"), "{code:?}");
    }
}

#[test]
fn registers_are_the_68000_register_file_in_panel_order() {
    use RegisterGroup::{Address, Control, General, Status};
    let expected: [(&str, &[&str], u8, RegisterGroup); 20] = [
        ("D0", &[], 32, General),
        ("D1", &[], 32, General),
        ("D2", &[], 32, General),
        ("D3", &[], 32, General),
        ("D4", &[], 32, General),
        ("D5", &[], 32, General),
        ("D6", &[], 32, General),
        ("D7", &[], 32, General),
        ("A0", &[], 32, Address),
        ("A1", &[], 32, Address),
        ("A2", &[], 32, Address),
        ("A3", &[], 32, Address),
        ("A4", &[], 32, Address),
        ("A5", &[], 32, Address),
        ("A6", &[], 32, Address),
        ("A7", &["SP"], 32, Address),
        ("USP", &[], 32, Control),
        ("SSP", &[], 32, Control),
        ("PC", &[], 32, Control),
        ("SR", &[], 16, Status),
    ];
    let registers = M68K.registers();
    assert_eq!(registers.len(), expected.len());
    for (index, (register, (name, aliases, bits, group))) in
        registers.iter().zip(expected).enumerate()
    {
        assert_eq!(
            register.id,
            RegId(u16::try_from(index).unwrap()),
            "{name}: stable id"
        );
        assert_eq!(register.name, name);
        assert_eq!(register.aliases, aliases, "{name}");
        assert_eq!(register.bits, bits, "{name}");
        assert_eq!(register.group, group, "{name}");
        assert_eq!(register.fields.is_empty(), name != "SR", "{name}: fields");
    }
    let mut names = BTreeSet::new();
    for register in registers {
        for name in std::iter::once(&register.name).chain(register.aliases) {
            assert!(names.insert(*name), "{name} names two registers");
        }
    }
}

#[test]
fn sr_has_the_trace_supervisor_mask_and_condition_fields() {
    let sr = M68K
        .registers()
        .iter()
        .find(|register| register.name == "SR")
        .unwrap();
    let fields: Vec<(&str, u8, u8)> = sr
        .fields
        .iter()
        .map(|field| (field.name, field.bit, field.width))
        .collect();
    assert_eq!(
        fields,
        [
            ("T", 15, 1),
            ("S", 13, 1),
            ("I", 8, 3),
            ("X", 4, 1),
            ("N", 3, 1),
            ("Z", 2, 1),
            ("V", 1, 1),
            ("C", 0, 1),
        ]
    );
    let mut used = 0u32;
    for field in sr.fields {
        assert!(
            field.width > 0 && field.bit + field.width <= sr.bits,
            "{field:?}"
        );
        let mask = ((1u32 << field.width) - 1) << field.bit;
        assert_eq!(used & mask, 0, "{} overlaps another field", field.name);
        used |= mask;
    }
}

#[test]
fn register_keywords_are_the_assembler_names_of_the_registers() {
    let keywords: BTreeSet<&str> = M68K
        .syntax()
        .keywords()
        .iter()
        .filter(|keyword| keyword.kind == KeywordKind::Register)
        .map(|keyword| keyword.name)
        .collect();
    assert_eq!(
        keywords.len(),
        21,
        "D0-D7, A0-A7, SP, USP, CCR, SR, PC (m1 §5.3)"
    );
    // SSP has no assembler name (only MOVE USP exists); CCR is the low byte of SR.
    let mut expected: BTreeSet<&str> = M68K
        .registers()
        .iter()
        .flat_map(|register| std::iter::once(register.name).chain(register.aliases.iter().copied()))
        .filter(|name| *name != "SSP")
        .collect();
    expected.insert("CCR");
    assert_eq!(keywords, expected);
}

#[test]
fn every_keyword_lexes_as_its_kind() {
    let keywords = M68K.syntax().keywords();
    let count = |kind: KeywordKind| keywords.iter().filter(|k| k.kind == kind).count();
    assert_eq!(count(KeywordKind::Mnemonic), 126, "mnemonics (m1 §5.3)");
    assert_eq!(count(KeywordKind::Directive), 49, "directives (m1 §5.3)");
    assert_eq!(count(KeywordKind::Register), 21, "registers (m1 §5.3)");
    assert_eq!(count(KeywordKind::SizeSuffix), 4, "size suffixes (m1 §5.3)");
    for keyword in keywords {
        let name = keyword.name;
        assert_eq!(
            name,
            name.to_ascii_uppercase(),
            "keyword names are uppercase"
        );
        for spelled in [name.to_owned(), name.to_ascii_lowercase()] {
            match keyword.kind {
                KeywordKind::Mnemonic => assert_eq!(
                    kinds(&format!("\t{spelled}")),
                    [TokenKind::Whitespace, TokenKind::Mnemonic],
                    "{spelled}"
                ),
                KeywordKind::Directive => assert_eq!(
                    kinds(&format!("\t{spelled}")),
                    [TokenKind::Whitespace, TokenKind::Directive],
                    "{spelled}"
                ),
                KeywordKind::Register => assert_eq!(
                    kinds(&format!("\tmove {spelled}")),
                    [
                        TokenKind::Whitespace,
                        TokenKind::Mnemonic,
                        TokenKind::Whitespace,
                        TokenKind::Register
                    ],
                    "{spelled}"
                ),
                KeywordKind::SizeSuffix => {
                    let suffix = format!(".{}", spelled.trim_start_matches('.'));
                    let tokens = lex(&format!("\tmove{suffix}"));
                    assert_eq!(tokens.len(), 3, "{suffix}: {tokens:?}");
                    assert_eq!(tokens[2], (TokenKind::SizeSuffix, suffix.clone()));
                }
            }
        }
    }
}

#[test]
fn mnemonic_keywords_are_the_spellings_with_their_doc_ids() {
    let isa = M68K.instruction_set();
    let mnemonics: Vec<_> = M68K
        .syntax()
        .keywords()
        .iter()
        .filter(|keyword| keyword.kind == KeywordKind::Mnemonic)
        .collect();
    let names: BTreeSet<&str> = mnemonics.iter().map(|keyword| keyword.name).collect();
    let spellings: BTreeSet<&str> = isa.spellings().iter().copied().collect();
    assert_eq!(names, spellings);
    for keyword in mnemonics {
        let info = isa.describe(keyword.name, None).unwrap();
        assert_eq!(keyword.doc_id, Some(info.doc_id), "{}", keyword.name);
    }
}

#[test]
fn every_spelling_lexes_as_a_mnemonic_with_each_accepted_suffix() {
    let isa = M68K.instruction_set();
    assert!(!isa.spellings().is_empty());
    for spelling in isa.spellings() {
        let info = isa.describe(spelling, None).unwrap();
        let sizes: BTreeSet<&str> = info
            .forms
            .iter()
            .flat_map(|form| form.sizes.iter().copied())
            .collect();
        for spelled in [spelling.to_string(), spelling.to_ascii_lowercase()] {
            assert_eq!(
                kinds(&format!("\t{spelled}")),
                [TokenKind::Whitespace, TokenKind::Mnemonic],
                "{spelled}"
            );
            for size in &sizes {
                for size in [size.to_string(), size.to_ascii_lowercase()] {
                    let line = format!("\t{spelled}.{size}");
                    assert_eq!(
                        lex(&line),
                        [
                            (TokenKind::Whitespace, "\t".to_owned()),
                            (TokenKind::Mnemonic, spelled.clone()),
                            (TokenKind::SizeSuffix, format!(".{size}")),
                        ],
                        "{line:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn locale_resources_exist_for_english_and_french() {
    assert_eq!(M68K.locale_resources("en").len(), 2);
    assert_eq!(M68K.locale_resources("fr").len(), 2);
    assert_ne!(M68K.locale_resources("en"), M68K.locale_resources("fr"));
    assert!(M68K.locale_resources("de").is_empty());
}
