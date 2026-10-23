//! Field and token rules of m1-core-isa-lexer.md §5.1–§5.3 and the expected results of §5.7:
//! column-1 and indented labels, the operation field and its suffixes, operand policies, every
//! operand token rule, and each row of the §5.7 table with its `cause` and quick fix.
#![allow(clippy::unwrap_used)]

use va_arch_m68k::syntax::{
    LEX_BAD_BACKSLASH, LEX_BAD_LABEL, LEX_BAD_NUMBER, LEX_BAD_OPERATION, LEX_BAD_SIZE_SUFFIX,
    LEX_LABEL_JUNK, LEX_NON_ASCII, LEX_OPERANDS_IN_COMMENT, LEX_UNEXPECTED_CHAR,
    LEX_UNTERMINATED_STRING,
};
use va_core::diag::{DiagArg, Severity};
use va_core::source::{FileId, SourceSpan, TextRange};
use va_core::syntax::TokenKind;

use crate::{
    COMMENT, DIRECTIVE, LABEL, LOCAL, MACRO, MNEMONIC, NUM, OP, PARAM, PUNCT, REG, SIZE, STR,
    SUSPECT, SYM, WS, diagnostics, invalid, kinds_of, mixed_case, pieces,
};

/// Asserts that `line` lexes to exactly `expected` (kind, text) pairs.
fn check(line: &str, expected: &[(TokenKind, &str)]) {
    assert_eq!(pieces(line), expected, "{line:?}");
}

/// Asserts that the operand field `operands` (after `\tdc.l\t`) lexes to `expected`.
fn check_operands(operands: &str, expected: &[(TokenKind, &str)]) {
    let line = format!("\tdc.l\t{operands}");
    let mut full = vec![(WS, "\t"), (DIRECTIVE, "dc"), (SIZE, ".l"), (WS, "\t")];
    full.extend_from_slice(expected);
    assert_eq!(pieces(&line), full, "{line:?}");
}

#[test]
fn column_one_identifier_is_a_label() {
    check("lab", &[(LABEL, "lab")]);
    check("lab:", &[(LABEL, "lab"), (PUNCT, ":")]);
    check("lab\tnop", &[(LABEL, "lab"), (WS, "\t"), (MNEMONIC, "nop")]);
    check(
        "lab:\tnop ; done",
        &[
            (LABEL, "lab"),
            (PUNCT, ":"),
            (WS, "\t"),
            (MNEMONIC, "nop"),
            (WS, " "),
            (COMMENT, "; done"),
        ],
    );
    check("_x.y@1:", &[(LABEL, "_x.y@1"), (PUNCT, ":")]);
    check(".loop", &[(LABEL, ".loop")]);
    check("lab;c", &[(LABEL, "lab"), (COMMENT, ";c")]);
    check("lab   ", &[(LABEL, "lab"), (WS, "   ")]);
}

#[test]
fn nop_in_column_one_defines_a_label() {
    check("nop", &[(LABEL, "nop")]);
    check("nop nop", &[(LABEL, "nop"), (WS, " "), (MNEMONIC, "nop")]);
    check("MOVE.L", &[(LABEL, "MOVE.L")]);
}

#[test]
fn local_labels_in_the_label_field() {
    check("1$", &[(LOCAL, "1$")]);
    check("12$:", &[(LOCAL, "12$"), (PUNCT, ":")]);
    check("\\name", &[(LOCAL, "\\name")]);
    check(
        "\\loop:\tnop",
        &[
            (LOCAL, "\\loop"),
            (PUNCT, ":"),
            (WS, "\t"),
            (MNEMONIC, "nop"),
        ],
    );
}

#[test]
fn bad_label_and_label_junk() {
    check(
        "9x nop",
        &[(invalid(LEX_BAD_LABEL), "9x"), (WS, " "), (MNEMONIC, "nop")],
    );
    check("-x", &[(invalid(LEX_BAD_LABEL), "-x")]);
    check("é:", &[(invalid(LEX_BAD_LABEL), "é:")]);
    check("\\(", &[(invalid(LEX_BAD_LABEL), "\\(")]);
    check(
        "lab:nop",
        &[
            (LABEL, "lab"),
            (PUNCT, ":"),
            (invalid(LEX_LABEL_JUNK), "nop"),
        ],
    );
    check("1$x", &[(LOCAL, "1$"), (invalid(LEX_LABEL_JUNK), "x")]);
    check("lab-1", &[(LABEL, "lab"), (invalid(LEX_LABEL_JUNK), "-1")]);
    check(
        "lab::nop",
        &[
            (LABEL, "lab"),
            (PUNCT, "::"),
            (invalid(LEX_LABEL_JUNK), "nop"),
        ],
    );
    check(
        "lab:\\@",
        &[
            (LABEL, "lab"),
            (PUNCT, ":"),
            (invalid(LEX_LABEL_JUNK), "\\@"),
        ],
    );
}

#[test]
fn a_colon_run_after_a_label_is_one_punctuation_token() {
    // m1 §12.1 R12-2: a68k assembles `lab::` and `lab:::` like `lab:`.
    check("lab::", &[(LABEL, "lab"), (PUNCT, "::")]);
    check("lab:::", &[(LABEL, "lab"), (PUNCT, ":::")]);
    check(
        "lab::\tnop",
        &[(LABEL, "lab"), (PUNCT, "::"), (WS, "\t"), (MNEMONIC, "nop")],
    );
    check("1$::", &[(LOCAL, "1$"), (PUNCT, "::")]);
    check("lbl\\@::", &[(LABEL, "lbl"), (PARAM, "\\@"), (PUNCT, "::")]);
    check(
        "  other:: nop",
        &[
            (WS, "  "),
            (LABEL, "other"),
            (PUNCT, "::"),
            (WS, " "),
            (MNEMONIC, "nop"),
        ],
    );
    check(
        "\tother:::",
        &[(WS, "\t"), (LABEL, "other"), (PUNCT, ":::")],
    );
    // A colon run is not a label.
    check("::", &[(invalid(LEX_BAD_LABEL), "::")]);
}

#[test]
fn indented_name_with_colon_is_a_label() {
    check(
        "  name: nop",
        &[
            (WS, "  "),
            (LABEL, "name"),
            (PUNCT, ":"),
            (WS, " "),
            (MNEMONIC, "nop"),
        ],
    );
    check("\tname:", &[(WS, "\t"), (LABEL, "name"), (PUNCT, ":")]);
    check(
        "\t1$:\tbra.s\t1$",
        &[
            (WS, "\t"),
            (LOCAL, "1$"),
            (PUNCT, ":"),
            (WS, "\t"),
            (MNEMONIC, "bra"),
            (SIZE, ".s"),
            (WS, "\t"),
            (LOCAL, "1$"),
        ],
    );
    check(
        "\t\\loop: nop",
        &[
            (WS, "\t"),
            (LOCAL, "\\loop"),
            (PUNCT, ":"),
            (WS, " "),
            (MNEMONIC, "nop"),
        ],
    );
    check(
        "\tlbl\\@: nop",
        &[
            (WS, "\t"),
            (LABEL, "lbl"),
            (PARAM, "\\@"),
            (PUNCT, ":"),
            (WS, " "),
            (MNEMONIC, "nop"),
        ],
    );
}

#[test]
fn indented_label_rule_needs_a_blank_column_one_and_a_whole_word() {
    // a68k: "No such op-code" for an indented `name:nop`.
    check(
        "  name:nop",
        &[(WS, "  "), (invalid(LEX_BAD_OPERATION), "name:nop")],
    );
    // With a column-1 label, the next word is the operation field.
    check(
        "x name: nop",
        &[
            (LABEL, "x"),
            (WS, " "),
            (invalid(LEX_BAD_OPERATION), "name:"),
            (WS, " "),
            (SYM, "nop"),
        ],
    );
    // Only the first word after the indentation can be a label.
    check(
        "  a: b: nop",
        &[
            (WS, "  "),
            (LABEL, "a"),
            (PUNCT, ":"),
            (WS, " "),
            (invalid(LEX_BAD_OPERATION), "b:"),
            (WS, " "),
            (SYM, "nop"),
        ],
    );
    check(
        "  9x: nop",
        &[
            (WS, "  "),
            (invalid(LEX_BAD_OPERATION), "9x:"),
            (WS, " "),
            (SYM, "nop"),
        ],
    );
}

#[test]
fn operation_field_kinds() {
    check(
        "\tmove.l\td0,d1",
        &[
            (WS, "\t"),
            (MNEMONIC, "move"),
            (SIZE, ".l"),
            (WS, "\t"),
            (REG, "d0"),
            (PUNCT, ","),
            (REG, "d1"),
        ],
    );
    assert_eq!(kinds_of("\tMOVE.W\tD0,D1"), kinds_of("\tmove.w\td0,d1"));
    check(
        "\tdc.b\t1",
        &[
            (WS, "\t"),
            (DIRECTIVE, "dc"),
            (SIZE, ".b"),
            (WS, "\t"),
            (NUM, "1"),
        ],
    );
    check(
        "\tmymacro\td0",
        &[(WS, "\t"), (MACRO, "mymacro"), (WS, "\t"), (REG, "d0")],
    );
    for not_a68k in ["ELSE", "REPT", "OPT"] {
        check(&format!("\t{not_a68k}"), &[(WS, "\t"), (MACRO, not_a68k)]);
    }
}

#[test]
fn operation_size_suffixes() {
    for suffix in [".b", ".w", ".l", ".s", ".B", ".W", ".L", ".S"] {
        check(
            &format!("\tbra{suffix}\tx"),
            &[
                (WS, "\t"),
                (MNEMONIC, "bra"),
                (SIZE, suffix),
                (WS, "\t"),
                (SYM, "x"),
            ],
        );
    }
    check(
        "\tmove.x\td0",
        &[
            (WS, "\t"),
            (MNEMONIC, "move"),
            (invalid(LEX_BAD_SIZE_SUFFIX), ".x"),
            (WS, "\t"),
            (REG, "d0"),
        ],
    );
    check(
        "\tmove.ll",
        &[
            (WS, "\t"),
            (MNEMONIC, "move"),
            (invalid(LEX_BAD_SIZE_SUFFIX), ".ll"),
        ],
    );
    check(
        "\tnop.",
        &[
            (WS, "\t"),
            (MNEMONIC, "nop"),
            (invalid(LEX_BAD_SIZE_SUFFIX), "."),
        ],
    );
    check(
        "\tmac.\\0\td0",
        &[
            (WS, "\t"),
            (MACRO, "mac"),
            (PUNCT, "."),
            (PARAM, "\\0"),
            (WS, "\t"),
            (REG, "d0"),
        ],
    );
}

#[test]
fn bad_operation_fields() {
    check(
        "\t123\td0",
        &[
            (WS, "\t"),
            (invalid(LEX_BAD_OPERATION), "123"),
            (WS, "\t"),
            (REG, "d0"),
        ],
    );
    // A `*` after column 1 is never a comment.
    check(
        "\t*\tcomment",
        &[
            (WS, "\t"),
            (invalid(LEX_BAD_OPERATION), "*"),
            (WS, "\t"),
            (SYM, "comment"),
        ],
    );
    check(
        "\tmove+x",
        &[(WS, "\t"), (invalid(LEX_BAD_OPERATION), "move+x")],
    );
    check("\t.w", &[(WS, "\t"), (invalid(LEX_BAD_OPERATION), ".w")]);
    check("\té", &[(WS, "\t"), (invalid(LEX_BAD_OPERATION), "é")]);
}

#[test]
fn equals_directive() {
    check(
        "x = 5",
        &[
            (LABEL, "x"),
            (WS, " "),
            (DIRECTIVE, "="),
            (WS, " "),
            (NUM, "5"),
        ],
    );
    check(
        "x\t=\t$10",
        &[
            (LABEL, "x"),
            (WS, "\t"),
            (DIRECTIVE, "="),
            (WS, "\t"),
            (NUM, "$10"),
        ],
    );
    // a68k: "No such op-code" for `x =5`.
    check(
        "x =5",
        &[(LABEL, "x"), (WS, " "), (invalid(LEX_BAD_OPERATION), "=5")],
    );
    check(
        "x == 5",
        &[
            (LABEL, "x"),
            (WS, " "),
            (invalid(LEX_BAD_OPERATION), "=="),
            (WS, " "),
            (NUM, "5"),
        ],
    );
}

#[test]
fn keywords_without_operands_make_the_rest_a_comment() {
    let directives = [
        "BSS", "CODE", "CSEG", "DATA", "DSEG", "ENDC", "ENDIF", "ENDM", "EVEN", "FAR", "LIST",
        "MACRO", "MEXIT", "NOL", "NOLIST", "PAGE",
    ];
    let mnemonics = ["NOP", "RESET", "RTE", "RTR", "RTS", "TRAPV", "ILLEGAL"];
    for (kind, names) in [(DIRECTIVE, &directives[..]), (MNEMONIC, &mnemonics[..])] {
        for name in names {
            for spelling in [
                name.to_string(),
                name.to_ascii_lowercase(),
                mixed_case(name),
            ] {
                check(
                    &format!("\t{spelling} d0, d1"),
                    &[
                        (WS, "\t"),
                        (kind, spelling.as_str()),
                        (WS, " "),
                        (COMMENT, "d0, d1"),
                    ],
                );
            }
        }
    }
    check("\trts;x", &[(WS, "\t"), (MNEMONIC, "rts"), (COMMENT, ";x")]);
}

#[test]
fn include_and_incbin_take_a_path() {
    check(
        "\tinclude\t\"file.i\"",
        &[
            (WS, "\t"),
            (DIRECTIVE, "include"),
            (WS, "\t"),
            (STR, "\"file.i\""),
        ],
    );
    check(
        "\tINCLUDE\tdir\\sub:f.i",
        &[
            (WS, "\t"),
            (DIRECTIVE, "INCLUDE"),
            (WS, "\t"),
            (STR, "dir\\sub:f.i"),
        ],
    );
    check(
        "\tincbin\t'a b.bin'\tdata",
        &[
            (WS, "\t"),
            (DIRECTIVE, "incbin"),
            (WS, "\t"),
            (STR, "'a b.bin'"),
            (WS, "\t"),
            (COMMENT, "data"),
        ],
    );
    // No §5.7 rule on a path: the text after the space stays a plain comment.
    check(
        "\tinclude\ta.i, b.i",
        &[
            (WS, "\t"),
            (DIRECTIVE, "include"),
            (WS, "\t"),
            (STR, "a.i,"),
            (WS, " "),
            (COMMENT, "b.i"),
        ],
    );
    check(
        "\tinclude\t\"open",
        &[
            (WS, "\t"),
            (DIRECTIVE, "include"),
            (WS, "\t"),
            (invalid(LEX_UNTERMINATED_STRING), "\"open"),
        ],
    );
    check(
        "\tinclude\tf.i;c",
        &[
            (WS, "\t"),
            (DIRECTIVE, "include"),
            (WS, "\t"),
            (STR, "f.i"),
            (COMMENT, ";c"),
        ],
    );
}

#[test]
fn registers_and_register_suffixes() {
    check_operands(
        "2(a0,d0.w),d1",
        &[
            (NUM, "2"),
            (PUNCT, "("),
            (REG, "a0"),
            (PUNCT, ","),
            (REG, "d0"),
            (SIZE, ".w"),
            (PUNCT, ")"),
            (PUNCT, ","),
            (REG, "d1"),
        ],
    );
    check_operands(
        "(A1,D7.L),sp",
        &[
            (PUNCT, "("),
            (REG, "A1"),
            (PUNCT, ","),
            (REG, "D7"),
            (SIZE, ".L"),
            (PUNCT, ")"),
            (PUNCT, ","),
            (REG, "sp"),
        ],
    );
    for register in [
        "d0", "D7", "a0", "A7", "sp", "SP", "usp", "USP", "ccr", "CCR", "sr", "SR", "pc", "PC",
        "Usp",
    ] {
        check_operands(register, &[(REG, register)]);
    }
    for symbol in ["d8", "a8", "ssp", "d0.x", "d8.w", "a0.wl", "pcx", "d00"] {
        check_operands(symbol, &[(SYM, symbol)]);
    }
    check_operands("usp.w", &[(REG, "usp"), (SIZE, ".w")]);
}

#[test]
fn operand_size_suffix_after_a_parenthesis_or_a_number() {
    check_operands(
        "($1000).w,d0",
        &[
            (PUNCT, "("),
            (NUM, "$1000"),
            (PUNCT, ")"),
            (SIZE, ".w"),
            (PUNCT, ","),
            (REG, "d0"),
        ],
    );
    check_operands("$1000.l", &[(NUM, "$1000"), (SIZE, ".l")]);
    check_operands("1.S", &[(NUM, "1"), (SIZE, ".S")]);
    check_operands("x.w", &[(SYM, "x.w")]);
    check_operands(".x", &[(SYM, ".x")]);
    check_operands(
        "(a0).",
        &[
            (PUNCT, "("),
            (REG, "a0"),
            (PUNCT, ")"),
            (invalid(LEX_UNEXPECTED_CHAR), "."),
        ],
    );
}

#[test]
fn local_labels_in_operands() {
    check_operands("1$", &[(LOCAL, "1$")]);
    check_operands("\\loop", &[(LOCAL, "\\loop")]);
    check_operands("12$+4", &[(LOCAL, "12$"), (OP, "+"), (NUM, "4")]);
}

#[test]
fn macro_parameters_in_every_field() {
    check("lbl\\@:", &[(LABEL, "lbl"), (PARAM, "\\@"), (PUNCT, ":")]);
    check("\\@lbl", &[(PARAM, "\\@"), (LABEL, "lbl")]);
    check("\\1", &[(PARAM, "\\1")]);
    check(
        "\\name\\@x",
        &[(LOCAL, "\\name"), (PARAM, "\\@"), (LOCAL, "x")],
    );
    check(
        "\tmove.l\t\\1,\\2",
        &[
            (WS, "\t"),
            (MNEMONIC, "move"),
            (SIZE, ".l"),
            (WS, "\t"),
            (PARAM, "\\1"),
            (PUNCT, ","),
            (PARAM, "\\2"),
        ],
    );
    check(
        "\t\\1\td0",
        &[(WS, "\t"), (PARAM, "\\1"), (WS, "\t"), (REG, "d0")],
    );
    check(
        "\tb\\1\tlabel",
        &[
            (WS, "\t"),
            (MACRO, "b"),
            (PARAM, "\\1"),
            (WS, "\t"),
            (SYM, "label"),
        ],
    );
    check_operands("lbl\\@", &[(SYM, "lbl"), (PARAM, "\\@")]);
    check_operands("\\9,\\0", &[(PARAM, "\\9"), (PUNCT, ","), (PARAM, "\\0")]);
    // Not in strings or comments.
    check_operands("'\\1'", &[(STR, "'\\1'")]);
    check("; \\1", &[(COMMENT, "; \\1")]);
}

#[test]
fn bad_backslash() {
    check_operands("\\%", &[(invalid(LEX_BAD_BACKSLASH), "\\%")]);
    check_operands("\\", &[(invalid(LEX_BAD_BACKSLASH), "\\")]);
    check_operands("\\é", &[(invalid(LEX_BAD_BACKSLASH), "\\é")]);
    check_operands("\\\\", &[(invalid(LEX_BAD_BACKSLASH), "\\\\")]);
    check_operands(
        "\\ x",
        &[
            (invalid(LEX_BAD_BACKSLASH), "\\"),
            (WS, " "),
            (COMMENT, "x"),
        ],
    );
    check_operands(
        "\\;c",
        &[(invalid(LEX_BAD_BACKSLASH), "\\"), (COMMENT, ";c")],
    );
}

#[test]
fn every_number_base() {
    for number in [
        "$1F", "$abcdef", "$0", "%1010", "%0", "@17", "@0", "42", "0", "007",
    ] {
        check_operands(number, &[(NUM, number)]);
    }
    check_operands("#-1", &[(PUNCT, "#"), (OP, "-"), (NUM, "1")]);
}

#[test]
fn bad_numbers() {
    for bad in [
        "10h", "0x10", "$12g", "$", "%2", "@8", "%", "@", "1.5", "$12.wx", "1.", "12@", "%102",
    ] {
        check_operands(bad, &[(invalid(LEX_BAD_NUMBER), bad)]);
    }
    check_operands(
        "$,1",
        &[(invalid(LEX_BAD_NUMBER), "$"), (PUNCT, ","), (NUM, "1")],
    );
    check_operands("#$", &[(PUNCT, "#"), (invalid(LEX_BAD_NUMBER), "$")]);
}

#[test]
fn quoted_strings() {
    for string in [
        "'abc'",
        "'it''s'",
        "\"say \"\"hi\"\"\"",
        "'a\\nb'",
        "''",
        "''''",
        "\"'\"",
        "'\"'",
        "'é日'",
        "'a b, c ; d'",
    ] {
        check_operands(string, &[(STR, string)]);
    }
    check_operands("'a',\"b\"", &[(STR, "'a'"), (PUNCT, ","), (STR, "\"b\"")]);
}

#[test]
fn unterminated_string_runs_to_the_end_of_the_line() {
    check_operands(
        "'abc d0, d1 ; x",
        &[(invalid(LEX_UNTERMINATED_STRING), "'abc d0, d1 ; x")],
    );
    check_operands(
        "1,'it''s",
        &[
            (NUM, "1"),
            (PUNCT, ","),
            (invalid(LEX_UNTERMINATED_STRING), "'it''s"),
        ],
    );
    check_operands("\"", &[(invalid(LEX_UNTERMINATED_STRING), "\"")]);
}

#[test]
fn operators_and_punctuation() {
    check_operands(
        "1+2-3*4/5~6&7!8|9<<1>>2",
        &[
            (NUM, "1"),
            (OP, "+"),
            (NUM, "2"),
            (OP, "-"),
            (NUM, "3"),
            (OP, "*"),
            (NUM, "4"),
            (OP, "/"),
            (NUM, "5"),
            (OP, "~"),
            (NUM, "6"),
            (OP, "&"),
            (NUM, "7"),
            (OP, "!"),
            (NUM, "8"),
            (OP, "|"),
            (NUM, "9"),
            (OP, "<<"),
            (NUM, "1"),
            (OP, ">>"),
            (NUM, "2"),
        ],
    );
    check_operands("*", &[(OP, "*")]);
    check_operands("*+4", &[(OP, "*"), (OP, "+"), (NUM, "4")]);
    check_operands(
        "#1,(a0)",
        &[
            (PUNCT, "#"),
            (NUM, "1"),
            (PUNCT, ","),
            (PUNCT, "("),
            (REG, "a0"),
            (PUNCT, ")"),
        ],
    );
}

#[test]
fn unexpected_characters() {
    check_operands("^[]{}", &[(invalid(LEX_UNEXPECTED_CHAR), "^[]{}")]);
    check_operands("=?:`", &[(invalid(LEX_UNEXPECTED_CHAR), "=?:`")]);
    check_operands(
        "1<2",
        &[(NUM, "1"), (invalid(LEX_UNEXPECTED_CHAR), "<"), (NUM, "2")],
    );
    check_operands(">", &[(invalid(LEX_UNEXPECTED_CHAR), ">")]);
    check_operands(
        "1<<<2",
        &[
            (NUM, "1"),
            (OP, "<<"),
            (invalid(LEX_UNEXPECTED_CHAR), "<"),
            (NUM, "2"),
        ],
    );
    check_operands(".", &[(invalid(LEX_UNEXPECTED_CHAR), ".")]);
    check_operands(
        "x?.y",
        &[(SYM, "x"), (invalid(LEX_UNEXPECTED_CHAR), "?"), (SYM, ".y")],
    );
    check_operands(
        "\u{1}\u{7f}",
        &[(invalid(LEX_UNEXPECTED_CHAR), "\u{1}\u{7f}")],
    );
}

#[test]
fn non_ascii_outside_comments_and_strings() {
    check_operands("é", &[(invalid(LEX_NON_ASCII), "é")]);
    check_operands(
        "x日本y",
        &[(SYM, "x"), (invalid(LEX_NON_ASCII), "日本"), (SYM, "y")],
    );
    check("; é日本", &[(COMMENT, "; é日本")]);
    check_operands("'é'", &[(STR, "'é'")]);
}

#[test]
fn comments_after_the_operands() {
    check(
        "\tmove.l\td0,d1\tcopy it",
        &[
            (WS, "\t"),
            (MNEMONIC, "move"),
            (SIZE, ".l"),
            (WS, "\t"),
            (REG, "d0"),
            (PUNCT, ","),
            (REG, "d1"),
            (WS, "\t"),
            (COMMENT, "copy it"),
        ],
    );
    check("* whole line", &[(COMMENT, "* whole line")]);
    check("*", &[(COMMENT, "*")]);
    check("; whole line", &[(COMMENT, "; whole line")]);
    check_operands("';',1", &[(STR, "';'"), (PUNCT, ","), (NUM, "1")]);
    check_operands("1;c", &[(NUM, "1"), (COMMENT, ";c")]);
    check_operands("1 ;c", &[(NUM, "1"), (WS, " "), (COMMENT, ";c")]);
    check_operands("1   ", &[(NUM, "1"), (WS, "   ")]);
    check_operands(
        "1 free ; text",
        &[(NUM, "1"), (WS, " "), (COMMENT, "free ; text")],
    );
}

/// One row of the §5.7 table: the line, the suspect text, its cause and the fixed line (`None`
/// for rule R3, which has no fix).
struct SuspectRow {
    /// The source line.
    line: &'static str,
    /// The text of the `SuspectComment` token.
    suspect: &'static str,
    /// The `cause` argument.
    cause: &'static str,
    /// The quick-fix replacement and the line once it is applied.
    fix: Option<(&'static str, &'static str)>,
}

/// The rows of the §5.7 table that fire, written off column 1 (an identifier in column 1 is a
/// label), plus extra cases of each rule.
const SUSPECT_ROWS: [SuspectRow; 19] = [
    SuspectRow {
        line: " dc.b 1, 2, 3",
        suspect: "2, 3",
        cause: "after-comma",
        fix: Some(("2,3", " dc.b 1,2,3")),
    },
    SuspectRow {
        line: " move.l d0, d1",
        suspect: "d1",
        cause: "after-comma",
        fix: Some(("d1", " move.l d0,d1")),
    },
    SuspectRow {
        line: "\tmove.l\td0,\td1",
        suspect: "d1",
        cause: "after-comma",
        fix: Some(("d1", "\tmove.l\td0,d1")),
    },
    SuspectRow {
        line: " dc.b 1 ,2",
        suspect: ",2",
        cause: "after-comma",
        fix: Some((",2", " dc.b 1,2")),
    },
    SuspectRow {
        line: " dc.b 1, 2 ; three",
        suspect: "2",
        cause: "after-comma",
        fix: Some(("2", " dc.b 1,2 ; three")),
    },
    SuspectRow {
        line: " dc.b 'a, b', 2",
        suspect: "2",
        cause: "after-comma",
        fix: Some(("2", " dc.b 'a, b',2")),
    },
    SuspectRow {
        line: " dc.b 1 + 2",
        suspect: "+ 2",
        cause: "inside-expression",
        fix: Some(("+2", " dc.b 1+2")),
    },
    SuspectRow {
        line: " move.l #1 + 2,d0",
        suspect: "+ 2,d0",
        cause: "inside-expression",
        fix: Some(("+2,d0", " move.l #1+2,d0")),
    },
    SuspectRow {
        line: " lea 4( a0),a1",
        suspect: "a0),a1",
        cause: "inside-expression",
        fix: Some(("a0),a1", " lea 4(a0),a1")),
    },
    SuspectRow {
        line: " movem.l d0-d7, -(sp)",
        suspect: "-(sp)",
        cause: "after-comma",
        fix: Some(("-(sp)", " movem.l d0-d7,-(sp)")),
    },
    SuspectRow {
        line: " dc.b 1,2 3,4 swapped",
        suspect: "3,4 swapped",
        cause: "operand-list",
        fix: None,
    },
    // Extra cases: R1 on `#` and on an open parenthesis, R2 on `<<`, quoted `;`, R3 shapes.
    // m1 §12.1 R12-1: `#` joins, so the fix of a dangling `#` removes the space.
    SuspectRow {
        line: " move.l # 1,d0",
        suspect: "1,d0",
        cause: "inside-expression",
        fix: Some(("1,d0", " move.l #1,d0")),
    },
    SuspectRow {
        line: " move.l (a0 ,d0)",
        suspect: ",d0)",
        cause: "inside-expression",
        fix: Some((",d0)", " move.l (a0,d0)")),
    },
    SuspectRow {
        line: " dc.l 1 << 2",
        suspect: "<< 2",
        cause: "inside-expression",
        fix: Some(("<<2", " dc.l 1<<2")),
    },
    SuspectRow {
        line: " dc.b 1, ';', 2",
        suspect: "';', 2",
        cause: "after-comma",
        fix: Some(("';',2", " dc.b 1,';',2")),
    },
    SuspectRow {
        line: " move.l d0,d1 -(a0),d2",
        suspect: "-(a0),d2",
        cause: "operand-list",
        fix: None,
    },
    SuspectRow {
        line: " dc.b 1 #2,3 x",
        suspect: "#2,3 x",
        cause: "operand-list",
        fix: None,
    },
    SuspectRow {
        line: " mymac d0, d1",
        suspect: "d1",
        cause: "after-comma",
        fix: Some(("d1", " mymac d0,d1")),
    },
    SuspectRow {
        line: "x = 1, 2",
        suspect: "2",
        cause: "after-comma",
        fix: Some(("2", "x = 1,2")),
    },
];

#[test]
fn suspect_comment_table() {
    for row in &SUSPECT_ROWS {
        let tokens = pieces(row.line);
        let suspects: Vec<&str> = tokens
            .iter()
            .filter(|(kind, _)| *kind == SUSPECT)
            .map(|&(_, text)| text)
            .collect();
        assert_eq!(suspects, [row.suspect], "{:?}: {tokens:?}", row.line);
        let index = tokens
            .iter()
            .position(|(kind, _)| *kind == SUSPECT)
            .unwrap();
        assert_eq!(tokens[index - 1].0, WS, "{:?}", row.line);
    }
}

#[test]
fn suspect_comment_keeps_the_semicolon_comment_and_trailing_space() {
    check_operands(
        "1, 2 ; three",
        &[
            (NUM, "1"),
            (PUNCT, ","),
            (WS, " "),
            (SUSPECT, "2"),
            (WS, " "),
            (COMMENT, "; three"),
        ],
    );
    check_operands(
        "1, 2   ",
        &[
            (NUM, "1"),
            (PUNCT, ","),
            (WS, " "),
            (SUSPECT, "2"),
            (WS, "   "),
        ],
    );
    check_operands(
        "1, 2  ;x",
        &[
            (NUM, "1"),
            (PUNCT, ","),
            (WS, " "),
            (SUSPECT, "2"),
            (WS, "  "),
            (COMMENT, ";x"),
        ],
    );
    check_operands(
        "1, 'a;b'",
        &[(NUM, "1"), (PUNCT, ","), (WS, " "), (SUSPECT, "'a;b'")],
    );
}

#[test]
fn suspect_comment_diagnostics_carry_cause_and_fix() {
    let line_start = 100;
    for row in &SUSPECT_ROWS {
        let tokens = crate::lex(row.line);
        let suspect = tokens.iter().find(|token| token.kind == SUSPECT).unwrap();
        let diagnostics = diagnostics(row.line, line_start);
        assert_eq!(diagnostics.len(), 1, "{:?}: {diagnostics:?}", row.line);
        let diagnostic = &diagnostics[0];
        assert_eq!(diagnostic.code, LEX_OPERANDS_IN_COMMENT);
        assert_eq!(diagnostic.severity, Severity::Error);
        assert!(!diagnostic.blocking, "{:?}", row.line);
        assert_eq!(
            diagnostic.span,
            SourceSpan {
                file: FileId(3),
                range: suspect.range.shifted(line_start)
            }
        );
        assert_eq!(
            diagnostic.args,
            [
                ("text", DiagArg::Str(row.suspect.to_owned())),
                ("cause", DiagArg::Str(row.cause.to_owned())),
            ],
            "{:?}",
            row.line
        );
        match row.fix {
            None => assert_eq!(diagnostic.suggestion, None, "{:?}", row.line),
            Some((replacement, fixed)) => {
                let suggestion = diagnostic.suggestion.as_ref().unwrap();
                assert_eq!(suggestion.span.file, FileId(3));
                assert_eq!(suggestion.replacement, replacement, "{:?}", row.line);
                assert_eq!(suggestion.span.range.end, suspect.range.end + line_start);
                let range = TextRange::new(
                    suggestion.span.range.start - line_start,
                    suggestion.span.range.end - line_start,
                );
                let mut applied = row.line[..range.start as usize].to_owned();
                applied.push_str(replacement);
                applied.push_str(&row.line[range.end as usize..]);
                assert_eq!(applied, fixed, "{:?}", row.line);
            }
        }
    }
}

#[test]
fn text_that_does_not_fire_stays_a_comment() {
    for line in [
        " dc.b 'a b',2",
        " move.l d0,d1 ; real comment",
        " move.l d0,d1 free text",
        " move.l d0,d1 * comment",
        "\tmove.l\td0,d1\tfree text",
        " rts d0, d1",
        "* dc.b 1, 2",
        " dc.b 1 - 2",
        " dc.b 1 * 2",
        " dc.b 1 2",
        " dc.l 1 ~2",
        " move.l d0,d1 a,b",
        " move.l d0,d1 (a0,d0)",
        " move.l d0,d1 1,$",
        " even 1, 2",
        " 123 d0, d1",
        " include a.i, b.i",
        " dc.b 1 < 2",
    ] {
        let kinds = kinds_of(line);
        assert!(!kinds.contains(&SUSPECT), "{line:?}: {kinds:?}");
        assert!(
            diagnostics(line, 0)
                .iter()
                .all(|diagnostic| diagnostic.code != LEX_OPERANDS_IN_COMMENT),
            "{line:?}"
        );
    }
    check(
        " dc.b 'a b',2",
        &[
            (WS, " "),
            (DIRECTIVE, "dc"),
            (SIZE, ".b"),
            (WS, " "),
            (STR, "'a b'"),
            (PUNCT, ","),
            (NUM, "2"),
        ],
    );
    check(
        " dc.b 1 - 2",
        &[
            (WS, " "),
            (DIRECTIVE, "dc"),
            (SIZE, ".b"),
            (WS, " "),
            (NUM, "1"),
            (WS, " "),
            (COMMENT, "- 2"),
        ],
    );
}

#[test]
fn invalid_tokens_give_blocking_errors() {
    let diagnostics = diagnostics("\tdc.b\t$,1, 2", 10);
    assert_eq!(diagnostics.len(), 2, "{diagnostics:?}");
    assert_eq!(diagnostics[0].code, LEX_BAD_NUMBER);
    assert!(diagnostics[0].blocking);
    assert_eq!(
        diagnostics[0].args,
        [("text", DiagArg::Str("$".to_owned()))]
    );
    assert_eq!(diagnostics[0].span.range, TextRange::new(16, 17));
    assert_eq!(diagnostics[1].code, LEX_OPERANDS_IN_COMMENT);
    assert!(!diagnostics[1].blocking);
}

// QA (m1 §5.7, §12.1 R12-1): more firing rows (negative numbers, strings with spaces, labels
// before the statement, tab separators, every R2 joiner, R3 starting with a local label, a
// string or a parenthesis, a macro name holding a parameter), more non-firing lines, and lines
// with several diagnostics. Each firing row is checked like the §5.7 table: one suspect token
// right after a whitespace token, one non-blocking diagnostic with `text` and `cause`, the fix
// applied, and the fixed line clean. Facts checked with a68k 2.71.F3w on 2026-10-18:
// `dc.b -1, -2` gives 1 byte (FF) and the fixed `dc.b -1,-2` 2 bytes (FFFE); `dc.b 'a', 'b c'`
// gives 1 byte (61).
const QA_SUSPECT_ROWS: [SuspectRow; 20] = [
    SuspectRow {
        line: " dc.b -1, -2",
        suspect: "-2",
        cause: "after-comma",
        fix: Some(("-2", " dc.b -1,-2")),
    },
    SuspectRow {
        line: " dc.b 'a', 'b c'",
        suspect: "'b c'",
        cause: "after-comma",
        fix: Some(("'b c'", " dc.b 'a','b c'")),
    },
    SuspectRow {
        line: "  lab: dc.b 1, 2",
        suspect: "2",
        cause: "after-comma",
        fix: Some(("2", "  lab: dc.b 1,2")),
    },
    SuspectRow {
        line: "lab dc.b 1, 2",
        suspect: "2",
        cause: "after-comma",
        fix: Some(("2", "lab dc.b 1,2")),
    },
    SuspectRow {
        line: " dc.b 1,\t2",
        suspect: "2",
        cause: "after-comma",
        fix: Some(("2", " dc.b 1,2")),
    },
    SuspectRow {
        line: " dc.w 1, 2 , 3",
        suspect: "2 , 3",
        cause: "after-comma",
        fix: Some(("2,3", " dc.w 1,2,3")),
    },
    SuspectRow {
        line: " move.l d0, d1\t; c",
        suspect: "d1",
        cause: "after-comma",
        fix: Some(("d1", " move.l d0,d1\t; c")),
    },
    SuspectRow {
        line: " dc.b 1, 2 bytes total",
        suspect: "2 bytes total",
        cause: "after-comma",
        fix: Some(("2 bytes total", " dc.b 1,2 bytes total")),
    },
    SuspectRow {
        line: " dc.l 8 >> 1",
        suspect: ">> 1",
        cause: "inside-expression",
        fix: Some((">>1", " dc.l 8>>1")),
    },
    SuspectRow {
        line: " dc.w 1 | 2",
        suspect: "| 2",
        cause: "inside-expression",
        fix: Some(("|2", " dc.w 1|2")),
    },
    SuspectRow {
        line: " dc.w 6 / 2",
        suspect: "/ 2",
        cause: "inside-expression",
        fix: Some(("/2", " dc.w 6/2")),
    },
    SuspectRow {
        line: " dc.w 6 & 2",
        suspect: "& 2",
        cause: "inside-expression",
        fix: Some(("&2", " dc.w 6&2")),
    },
    SuspectRow {
        line: " dc.b 1+ 2",
        suspect: "2",
        cause: "inside-expression",
        fix: Some(("2", " dc.b 1+2")),
    },
    SuspectRow {
        line: " move.l (a0, d1),d2",
        suspect: "d1),d2",
        cause: "after-comma",
        fix: Some(("d1),d2", " move.l (a0,d1),d2")),
    },
    SuspectRow {
        line: " lea 2(a0 ),a1",
        suspect: "),a1",
        cause: "inside-expression",
        fix: Some(("),a1", " lea 2(a0),a1")),
    },
    SuspectRow {
        line: " move.l (a0) ,d0",
        suspect: ",d0",
        cause: "after-comma",
        fix: Some((",d0", " move.l (a0),d0")),
    },
    SuspectRow {
        line: " b\\1 d0, d1",
        suspect: "d1",
        cause: "after-comma",
        fix: Some(("d1", " b\\1 d0,d1")),
    },
    SuspectRow {
        line: " dc.l 1$,2$ 3$,4$",
        suspect: "3$,4$",
        cause: "operand-list",
        fix: None,
    },
    SuspectRow {
        line: " dc.b 1,2 'a','b'",
        suspect: "'a','b'",
        cause: "operand-list",
        fix: None,
    },
    SuspectRow {
        line: " move.l d0,d1 (a0),d2",
        suspect: "(a0),d2",
        cause: "operand-list",
        fix: None,
    },
];

/// Checks one firing row like `suspect_comment_table` and
/// `suspect_comment_diagnostics_carry_cause_and_fix` do, and that the fixed line is clean.
fn check_suspect_row(row: &SuspectRow) {
    let line_start = 1_000;
    let tokens = crate::lex(row.line);
    let suspects: Vec<usize> = tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| token.kind == SUSPECT)
        .map(|(index, _)| index)
        .collect();
    assert_eq!(suspects.len(), 1, "{:?}: {tokens:?}", row.line);
    let suspect = tokens[suspects[0]];
    assert_eq!(
        suspect.range.slice(row.line),
        Some(row.suspect),
        "{:?}",
        row.line
    );
    assert_eq!(tokens[suspects[0] - 1].kind, WS, "{:?}", row.line);
    let found = diagnostics(row.line, line_start);
    assert_eq!(found.len(), 1, "{:?}: {found:?}", row.line);
    let diagnostic = &found[0];
    assert_eq!(diagnostic.code, LEX_OPERANDS_IN_COMMENT, "{:?}", row.line);
    assert_eq!(diagnostic.severity, Severity::Error, "{:?}", row.line);
    assert!(!diagnostic.blocking, "{:?}", row.line);
    assert_eq!(diagnostic.span.range, suspect.range.shifted(line_start));
    assert_eq!(
        diagnostic.args,
        [
            ("text", DiagArg::Str(row.suspect.to_owned())),
            ("cause", DiagArg::Str(row.cause.to_owned())),
        ],
        "{:?}",
        row.line
    );
    match row.fix {
        None => assert_eq!(diagnostic.suggestion, None, "{:?}", row.line),
        Some((replacement, fixed)) => {
            let suggestion = diagnostic.suggestion.as_ref().unwrap();
            assert_eq!(suggestion.replacement, replacement, "{:?}", row.line);
            assert_eq!(suggestion.span.range.end, suspect.range.end + line_start);
            let start = (suggestion.span.range.start - line_start) as usize;
            let end = (suggestion.span.range.end - line_start) as usize;
            let applied = format!("{}{replacement}{}", &row.line[..start], &row.line[end..]);
            assert_eq!(applied, fixed, "{:?}", row.line);
            assert!(diagnostics(fixed, 0).is_empty(), "fixed line {fixed:?}");
        }
    }
}

#[test]
fn qa_more_suspect_comment_rows() {
    for row in &QA_SUSPECT_ROWS {
        check_suspect_row(row);
    }
}

#[test]
fn qa_more_text_that_does_not_fire() {
    for line in [
        // T starts with `;`: never fires, even after a dangling comma.
        " dc.b 1, ; c",
        // An unterminated string runs to the end of the line: no trailing text at all.
        " dc.b 'abc, d",
        // After an invalid operation field, operand rules apply but §5.7 does not.
        " mov+ d0, d1",
        // No-operand keywords and paths.
        " nop , x",
        " endm 1, 2",
        " include \"a b.i\" , x",
        // A single `>` is not a joiner, and `-`, `*` never start one.
        " dc.b 1 > 2",
        " dc.b 1 -2,3",
        " dc.b 1 *2,3",
        // R3 needs an operand-like start.
        " move.l d0,d1 see,notes",
        " move.l d0,d1 -,x",
    ] {
        let kinds = kinds_of(line);
        assert!(!kinds.contains(&SUSPECT), "{line:?}: {kinds:?}");
        assert!(
            diagnostics(line, 0)
                .iter()
                .all(|diagnostic| diagnostic.code != LEX_OPERANDS_IN_COMMENT),
            "{line:?}"
        );
    }
}

#[test]
fn qa_suspect_comment_after_an_invalid_operand_token() {
    let line = " dc.b 1^, 2";
    assert_eq!(
        pieces(line),
        [
            (WS, " "),
            (DIRECTIVE, "dc"),
            (SIZE, ".b"),
            (WS, " "),
            (NUM, "1"),
            (invalid(LEX_UNEXPECTED_CHAR), "^"),
            (PUNCT, ","),
            (WS, " "),
            (SUSPECT, "2"),
        ]
    );
    let diagnostics = diagnostics(line, 50);
    assert_eq!(diagnostics.len(), 2, "{diagnostics:?}");
    assert_eq!(diagnostics[0].code, LEX_UNEXPECTED_CHAR);
    assert!(diagnostics[0].blocking);
    assert_eq!(
        diagnostics[0].args,
        [("text", DiagArg::Str("^".to_owned()))]
    );
    assert_eq!(diagnostics[0].suggestion, None);
    assert_eq!(diagnostics[1].code, LEX_OPERANDS_IN_COMMENT);
    assert!(!diagnostics[1].blocking);
    assert_eq!(
        diagnostics[1].args,
        [
            ("text", DiagArg::Str("2".to_owned())),
            ("cause", DiagArg::Str("after-comma".to_owned())),
        ]
    );
    let suggestion = diagnostics[1].suggestion.as_ref().unwrap();
    assert_eq!(suggestion.span.range, TextRange::new(59, 61));
    assert_eq!(suggestion.replacement, "2");
}

/// A `\r` inside a line (API misuse) splits it into two statements: each suspect comment gets
/// its own cause and fix, and the `\r` its own blocking error, in token order.
#[test]
fn qa_two_suspect_comments_around_a_stray_carriage_return() {
    let line = "\tdc.b 1, 2\r\tdc.b 3, 4";
    let diagnostics = diagnostics(line, 0);
    let codes: Vec<_> = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert_eq!(
        codes,
        [
            LEX_OPERANDS_IN_COMMENT,
            LEX_UNEXPECTED_CHAR,
            LEX_OPERANDS_IN_COMMENT
        ]
    );
    for (diagnostic, (text, span, fix)) in [&diagnostics[0], &diagnostics[2]]
        .into_iter()
        .zip([("2", (9, 10), (8, 10)), ("4", (20, 21), (19, 21))])
    {
        assert_eq!(diagnostic.span.range, TextRange::new(span.0, span.1));
        assert_eq!(
            diagnostic.args,
            [
                ("text", DiagArg::Str(text.to_owned())),
                ("cause", DiagArg::Str("after-comma".to_owned())),
            ]
        );
        let suggestion = diagnostic.suggestion.as_ref().unwrap();
        assert_eq!(suggestion.span.range, TextRange::new(fix.0, fix.1));
        assert_eq!(suggestion.replacement, text);
    }
    assert!(diagnostics[1].blocking);
    assert_eq!(diagnostics[1].span.range, TextRange::new(10, 11));
}

/// `line_diagnostics` appends: diagnostics already in `out` (an earlier line) keep their
/// arguments, and only the new suspect diagnostic gets a `cause`.
#[test]
fn qa_line_diagnostics_appends_after_earlier_diagnostics() {
    use va_core::syntax::SyntaxProvider as _;
    let syntax = va_arch_m68k::syntax::M68kSyntax;
    let mut out = Vec::new();
    for (line_start, line) in [(0_u32, " dc.b 1, 2"), (11, " dc.b $,3 4,5")] {
        let tokens = crate::lex(line);
        syntax.line_diagnostics(FileId(1), line_start, line, &tokens, &mut out);
    }
    let summary: Vec<(&str, Vec<&str>)> = out
        .iter()
        .map(|diagnostic| {
            let args = diagnostic
                .args
                .iter()
                .map(|(_, value)| match value {
                    DiagArg::Str(text) => text.as_str(),
                    DiagArg::Int(_) => "int",
                })
                .collect();
            (diagnostic.code.0, args)
        })
        .collect();
    assert_eq!(
        summary,
        [
            ("lex-operands-in-comment", vec!["2", "after-comma"]),
            ("lex-bad-number", vec!["$"]),
            ("lex-operands-in-comment", vec!["4,5", "operand-list"]),
        ]
    );
    assert!(out[0].suggestion.is_some());
    assert_eq!(out[2].suggestion, None);
    assert_eq!(out[2].span.range, TextRange::new(11 + 10, 11 + 13));
}

/// m1 §12.8 N-1: rows where R1 fires but removing the joining spaces would change nothing (the
/// separator touches no `,`, `#`, `(`, `)` or operator), so no fix is offered; the diagnostic
/// and its cause stay.
const NO_OP_FIX_ROWS: [SuspectRow; 3] = [
    SuspectRow {
        line: " move.l 8(a0 d0),d1",
        suspect: "d0),d1",
        cause: "inside-expression",
        fix: None,
    },
    SuspectRow {
        line: " dc.b (1 2)",
        suspect: "2)",
        cause: "inside-expression",
        fix: None,
    },
    SuspectRow {
        line: " move.l (a0 d1",
        suspect: "d1",
        cause: "inside-expression",
        fix: None,
    },
];

#[test]
fn a_fix_that_changes_nothing_is_not_offered() {
    for row in &NO_OP_FIX_ROWS {
        check_suspect_row(row);
    }
}

/// m1 §12.8 N-7: in the trailing text, a quote opens a string only when the line closes it, for
/// the `;` search and for the fix alike.
const QUOTE_ROWS: [SuspectRow; 3] = [
    SuspectRow {
        line: " dc.b 1, 'a;b' x",
        suspect: "'a;b' x",
        cause: "after-comma",
        fix: Some(("'a;b' x", " dc.b 1,'a;b' x")),
    },
    SuspectRow {
        line: " dc.b 1, 'a , b' , 2",
        suspect: "'a , b' , 2",
        cause: "after-comma",
        fix: Some(("'a , b',2", " dc.b 1,'a , b',2")),
    },
    SuspectRow {
        line: " dc.b 1, \"x;y\"",
        suspect: "\"x;y\"",
        cause: "after-comma",
        fix: Some(("\"x;y\"", " dc.b 1,\"x;y\"")),
    },
];

#[test]
fn a_closed_quote_in_the_trailing_text_keeps_its_semicolon_and_spaces() {
    for row in &QUOTE_ROWS {
        check_suspect_row(row);
    }
}

#[test]
fn an_unclosed_quote_in_the_trailing_text_does_not_hide_a_comment() {
    let line = " dc.b 1, don't ; real comment";
    assert_eq!(
        pieces(line),
        [
            (WS, " "),
            (DIRECTIVE, "dc"),
            (SIZE, ".b"),
            (WS, " "),
            (NUM, "1"),
            (PUNCT, ","),
            (WS, " "),
            (SUSPECT, "don't"),
            (WS, " "),
            (COMMENT, "; real comment"),
        ]
    );
    let found = diagnostics(line, 0);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].span.range, TextRange::new(9, 14));
    assert_eq!(
        found[0].args,
        [
            ("text", DiagArg::Str("don't".to_owned())),
            ("cause", DiagArg::Str("after-comma".to_owned())),
        ]
    );
    let suggestion = found[0].suggestion.as_ref().unwrap();
    assert_eq!(suggestion.span.range, TextRange::new(8, 14));
    assert_eq!(suggestion.replacement, "don't");

    check_operands(
        "1, say \"hi ; c",
        &[
            (NUM, "1"),
            (PUNCT, ","),
            (WS, " "),
            (SUSPECT, "say \"hi"),
            (WS, " "),
            (COMMENT, "; c"),
        ],
    );
    // An unclosed quote does not protect the spaces of the fix either.
    let found = diagnostics(" dc.b 1, don't , 2", 0);
    assert_eq!(found[0].suggestion.as_ref().unwrap().replacement, "don't,2");
}
