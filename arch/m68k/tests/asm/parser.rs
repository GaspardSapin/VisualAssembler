//! Line fields and operands (`src/asm/parser.rs`, `src/asm/operand.rs`; m2 §4.3, §10.1): every
//! operand form and rejection with its fix, and the `<…>` lexer rows (WP-B).
#![allow(clippy::unwrap_used)]

use va_arch_m68k::asm::codes::{
    ASM_ABSOLUTE_SIZE_SYNTAX, ASM_BAD_EXPRESSION, ASM_BAD_OPERAND, ASM_LABEL_LIKE_KEYWORD,
    ASM_MACRO_PARAM_OUTSIDE, ASM_MISSING_DISPLACEMENT, ASM_REGISTER_SIZE_SUFFIX,
    ASM_STRING_TOO_LONG,
};
use va_arch_m68k::asm::expr::{BinOp, Expr, SymRef, UnOp};
use va_arch_m68k::asm::operand::{Ea, Operand, Spanned, Xn, parse_operand};
use va_arch_m68k::asm::parser::{Line, OpField, Problem, parse_line};
use va_arch_m68k::isa::Size;
use va_arch_m68k::syntax::{self, LEX_BAD_NUMBER, LEX_UNEXPECTED_CHAR};
use va_core::diag::{DiagArg, DiagCode};
use va_core::source::TextRange;
use va_core::syntax::{Token, TokenKind};

/// Lexes `line` with the M1 lexer.
fn lex(line: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    syntax::lex_line(line, &mut tokens);
    tokens
}

/// Lexes and parses the fields of `line`.
fn fields(line: &str) -> Line {
    parse_line(line, &lex(line))
}

/// Returns the text `range` covers in `line`.
fn text(line: &str, range: TextRange) -> &str {
    range.slice(line).unwrap()
}

/// The tokens of `line` from the start of its operand field.
fn operand_tokens(line: &str) -> Vec<Token> {
    let tokens = lex(line);
    let start = parse_line(line, &tokens).operand_field.start;
    let first = tokens
        .iter()
        .position(|token| token.range.start == start)
        .unwrap_or(tokens.len());
    tokens[first..].to_vec()
}

/// The line `\tmove.l\t<operand>`, whose operand field is `operand`.
fn line_of(operand: &str) -> String {
    format!("\tmove.l\t{operand}")
}

/// Parses `operand`, the whole operand field of an instruction; asserts that it uses the whole
/// field and covers it, and returns the operand.
fn operand(operand: &str) -> Operand {
    let line = line_of(operand);
    let tokens = operand_tokens(&line);
    let (parsed, used) =
        parse_operand(&line, &tokens).unwrap_or_else(|problem| panic!("{operand:?}: {problem:?}"));
    assert_eq!(used, tokens.len(), "{operand:?}");
    assert_eq!(text(&line, parsed.range), operand, "{operand:?}");
    parsed.item
}

/// Parses `operand` and returns its problem, with the line it was read in.
fn rejected(operand: &str) -> (String, Problem) {
    let line = line_of(operand);
    let tokens = operand_tokens(&line);
    match parse_operand(&line, &tokens) {
        Ok(parsed) => panic!("{operand:?} parsed as {parsed:?}"),
        Err(problem) => (line, problem),
    }
}

/// Asserts that `operand` is rejected with `code` over the whole operand and the `text`
/// argument `text` (when `Some`), and no fix.
fn check_rejected(operand: &str, code: DiagCode, text_arg: Option<&str>) {
    let (line, problem) = rejected(operand);
    assert_eq!(problem.code, code, "{operand:?}: {problem:?}");
    assert_eq!(text(&line, problem.range), operand, "{operand:?}");
    match text_arg {
        Some(expected) => assert_eq!(
            problem.args,
            [("text", DiagArg::Str(expected.to_owned()))],
            "{operand:?}"
        ),
        None => assert!(problem.args.is_empty(), "{operand:?}: {problem:?}"),
    }
    assert_eq!(problem.fix, None, "{operand:?}");
}

/// Applies the fix of `problem` to `line`.
fn apply_fix(line: &str, problem: &Problem) -> String {
    let (range, replacement) = problem.fix.as_ref().unwrap();
    format!(
        "{}{replacement}{}",
        &line[..range.start as usize],
        &line[range.end as usize..]
    )
}

/// The expression `Num(value)`.
fn num(value: i64) -> Expr {
    Expr::Num(value)
}

/// A symbol reference `name` in the operand field of [`line_of`], `at` bytes into the operand.
fn sym(name: &str, at: u32) -> Expr {
    let start = 8 + at;
    Expr::Sym(SymRef {
        name: name.to_owned(),
        local: name.ends_with('$') || name.starts_with('\\'),
        range: TextRange::new(start, start + u32::try_from(name.len()).unwrap()),
    })
}

/// An effective-address operand.
fn ea(ea: Ea) -> Operand {
    Operand::Ea(ea)
}

#[test]
fn register_direct_and_special_registers() {
    for n in 0..8_u8 {
        assert_eq!(operand(&format!("d{n}")), ea(Ea::Dn(n)));
        assert_eq!(operand(&format!("D{n}")), ea(Ea::Dn(n)));
        assert_eq!(operand(&format!("a{n}")), ea(Ea::An(n)));
    }
    assert_eq!(operand("sp"), ea(Ea::An(7)));
    assert_eq!(operand("SP"), ea(Ea::An(7)));
    assert_eq!(operand("ccr"), Operand::Ccr);
    assert_eq!(operand("SR"), Operand::Sr);
    assert_eq!(operand("Usp"), Operand::Usp);
}

#[test]
fn register_indirect_modes() {
    assert_eq!(operand("(a0)"), ea(Ea::Ind(0)));
    assert_eq!(operand("(SP)"), ea(Ea::Ind(7)));
    assert_eq!(operand("(a4)+"), ea(Ea::PostInc(4)));
    assert_eq!(operand("(sp)+"), ea(Ea::PostInc(7)));
    assert_eq!(operand("-(a5)"), ea(Ea::PreDec(5)));
    assert_eq!(operand("-(sp)"), ea(Ea::PreDec(7)));
}

#[test]
fn displacement_and_index_modes() {
    assert_eq!(operand("$10(a6)"), ea(Ea::Disp(num(16), 6)));
    assert_eq!(
        operand("-4(a0)"),
        ea(Ea::Disp(Expr::Unary(UnOp::Neg, Box::new(num(4))), 0))
    );
    assert_eq!(operand("lab(sp)"), ea(Ea::Disp(sym("lab", 0), 7)));
    assert_eq!(operand("(4)(a1)"), ea(Ea::Disp(num(4), 1)));
    assert_eq!(operand("0(a0)"), ea(Ea::Disp(num(0), 0)));
    assert_eq!(
        operand("$10(a0,d1.w)"),
        ea(Ea::Index(
            num(16),
            0,
            Xn {
                reg: 1,
                long: false
            }
        ))
    );
    assert_eq!(
        operand("0(a0,d1)"),
        ea(Ea::Index(
            num(0),
            0,
            Xn {
                reg: 1,
                long: false
            }
        ))
    );
    assert_eq!(
        operand("2(a1,a2.L)"),
        ea(Ea::Index(
            num(2),
            1,
            Xn {
                reg: 10,
                long: true
            }
        ))
    );
    assert_eq!(
        operand("-1(sp,sp.l)"),
        ea(Ea::Index(
            Expr::Unary(UnOp::Neg, Box::new(num(1))),
            7,
            Xn {
                reg: 15,
                long: true
            }
        ))
    );
}

#[test]
fn pc_relative_modes_take_the_target_address() {
    assert_eq!(operand("lab(pc)"), ea(Ea::PcDisp(sym("lab", 0))));
    assert_eq!(operand("$1010(PC)"), ea(Ea::PcDisp(num(0x1010))));
    assert_eq!(
        operand("lab(pc,d2.l)"),
        ea(Ea::PcIndex(sym("lab", 0), Xn { reg: 2, long: true }))
    );
    assert_eq!(
        operand("*+8(pc,a3)"),
        ea(Ea::PcIndex(
            Expr::Binary(BinOp::Add, Box::new(Expr::Here), Box::new(num(8))),
            Xn {
                reg: 11,
                long: false
            }
        ))
    );
}

#[test]
fn absolute_and_immediate_modes() {
    assert_eq!(operand("$400"), ea(Ea::Abs(num(0x400), None)));
    assert_eq!(operand("lab"), ea(Ea::Abs(sym("lab", 0), None)));
    assert_eq!(operand("1$"), ea(Ea::Abs(sym("1$", 0), None)));
    assert_eq!(operand("\\loop"), ea(Ea::Abs(sym("\\loop", 0), None)));
    assert_eq!(operand("*"), ea(Ea::Abs(Expr::Here, None)));
    assert_eq!(
        operand("($1234).w"),
        ea(Ea::Abs(num(0x1234), Some(Size::Word)))
    );
    assert_eq!(
        operand("($00012345).L"),
        ea(Ea::Abs(num(0x12345), Some(Size::Long)))
    );
    assert_eq!(
        operand("((1+2)).w"),
        ea(Ea::Abs(
            Expr::Binary(BinOp::Add, Box::new(num(1)), Box::new(num(2))),
            Some(Size::Word)
        ))
    );
    // A parenthesized expression without size is an ordinary absolute address.
    assert_eq!(
        operand("(1)+(2)"),
        ea(Ea::Abs(
            Expr::Binary(BinOp::Add, Box::new(num(1)), Box::new(num(2))),
            None
        ))
    );
    assert_eq!(operand("#$12"), ea(Ea::Imm(num(0x12))));
    assert_eq!(
        operand("#-1"),
        ea(Ea::Imm(Expr::Unary(UnOp::Neg, Box::new(num(1)))))
    );
    assert_eq!(operand("#'ab'"), ea(Ea::Imm(Expr::Char(0x6162))));
    assert_eq!(
        operand("#lab+4"),
        ea(Ea::Imm(Expr::Binary(
            BinOp::Add,
            Box::new(sym("lab", 1)),
            Box::new(num(4))
        )))
    );
}

#[test]
fn register_lists() {
    assert_eq!(operand("d0-d3/a0/a5-a6"), Operand::RegList(0x610F));
    assert_eq!(operand("a0-a7"), Operand::RegList(0xFF00));
    assert_eq!(operand("d0/d1"), Operand::RegList(0x0003));
    assert_eq!(operand("D7/SP"), Operand::RegList(0x8080));
    assert_eq!(operand("d0-a7"), Operand::RegList(0xFFFF));
    assert_eq!(operand("d2-d2"), Operand::RegList(0x0004));
    assert_eq!(operand("d1/d1"), Operand::RegList(0x0002));
}

#[test]
fn a_lone_quoted_string_is_a_byte_string() {
    assert_eq!(operand("'hello'"), Operand::Bytes(b"hello".to_vec()));
    assert_eq!(operand("'it''s'"), Operand::Bytes(b"it's".to_vec()));
    assert_eq!(operand("\"a'b\""), Operand::Bytes(b"a'b".to_vec()));
    assert_eq!(operand("''"), Operand::Bytes(Vec::new()));
    assert_eq!(operand("'é'"), Operand::Bytes("é".as_bytes().to_vec()));
    assert_eq!(operand("'a b ; c'"), Operand::Bytes(b"a b ; c".to_vec()));
    // Inside an expression, a string is a character constant.
    assert_eq!(
        operand("'A'+1"),
        ea(Ea::Abs(
            Expr::Binary(BinOp::Add, Box::new(Expr::Char(0x41)), Box::new(num(1))),
            None
        ))
    );
}

#[test]
fn missing_displacement_is_rejected_with_a_fix() {
    for (written, fixed) in [
        ("(a0,d0)", "0(a0,d0)"),
        ("(a1,d2.l)", "0(a1,d2.l)"),
        ("(sp,a3.w)", "0(sp,a3.w)"),
        ("(pc,d0)", "0(pc,d0)"),
    ] {
        let (line, problem) = rejected(written);
        assert_eq!(problem.code, ASM_MISSING_DISPLACEMENT, "{written:?}");
        assert_eq!(text(&line, problem.range), written);
        assert!(problem.args.is_empty());
        assert_eq!(text(&line, problem.fix.as_ref().unwrap().0), "(");
        assert_eq!(apply_fix(&line, &problem), line_of(fixed));
        // The fixed operand parses.
        operand(fixed);
    }
}

#[test]
fn a_size_after_an_unparenthesized_address_is_rejected_with_a_fix() {
    for (written, fixed) in [
        ("$400.w", "($400).w"),
        ("$400.L", "($400).L"),
        ("lab+2.l", "(lab+2).l"),
        ("(1)+(2).w", "((1)+(2)).w"),
    ] {
        let (line, problem) = rejected(written);
        assert_eq!(problem.code, ASM_ABSOLUTE_SIZE_SYNTAX, "{written:?}");
        assert_eq!(text(&line, problem.range), written);
        assert_eq!(problem.args, [("text", DiagArg::Str(written.to_owned()))]);
        assert_eq!(apply_fix(&line, &problem), line_of(fixed), "{written:?}");
        operand(fixed);
    }
    // `lab.w` is the symbol `lab.w` (identifiers may hold `.`, a68k).
    assert_eq!(operand("lab.w"), ea(Ea::Abs(sym("lab.w", 0), None)));
}

#[test]
fn a_size_after_a_register_outside_an_index_is_rejected() {
    for (written, suffixed) in [
        ("d0.w", "d0.w"),
        ("A0.L", "A0.L"),
        ("usp.w", "usp.w"),
        ("(a0.w)", "a0.w"),
        ("-(a1.l)", "a1.l"),
        ("4(a0.w)", "a0.w"),
        ("4(a0.w,d0)", "a0.w"),
        ("d0.w/d1", "d0.w"),
        ("d0-d1.l", "d1.l"),
    ] {
        let (line, problem) = rejected(written);
        assert_eq!(problem.code, ASM_REGISTER_SIZE_SUFFIX, "{written:?}");
        assert_eq!(text(&line, problem.range), suffixed, "{written:?}");
        assert_eq!(problem.args, [("text", DiagArg::Str(suffixed.to_owned()))]);
        assert_eq!(problem.fix, None);
    }
}

#[test]
fn other_shapes_are_bad_operands() {
    for written in [
        "(4,a0)",
        "(4,pc)",
        "pc",
        "(pc)",
        "(d0)",
        "-(d0)",
        "-(pc)",
        "(a0)+4",
        "(a0)-",
        "d0(a0)",
        "4(d0)",
        "4(a0,d0.b)",
        "4(a0,d0.s)",
        "4(a0,usp)",
        "4(a0",
        "4(a0,d0",
        "4(a0)x",
        "($400).b",
        "($400).s",
        "$400.b",
        "#",
        "#d0",
        "#1(a0)",
        "d3-d0",
        "a0-d7",
        "d0-",
        "d0/",
        "d0/usp",
        "d0+1",
        "+(a0)",
        "4+(a0)",
        "(0,a0,d0)",
        "(a0,d0)+",
    ] {
        check_rejected(written, ASM_BAD_OPERAND, Some(written));
    }
}

#[test]
fn expressions_that_do_not_parse() {
    for written in ["1+", "--2", "(1+2", "1|2", ")", "~"] {
        check_rejected(written, ASM_BAD_EXPRESSION, Some(written));
    }
    // After `#`, the expression is what is reported.
    let (line, problem) = rejected("#1+");
    assert_eq!(problem.code, ASM_BAD_EXPRESSION);
    assert_eq!(text(&line, problem.range), "1+");
    assert_eq!(problem.args, [("text", DiagArg::Str("1+".to_owned()))]);
    let (_, problem) = rejected("#'abcde'");
    assert_eq!(problem.code, ASM_STRING_TOO_LONG);
    assert_eq!(
        problem.args,
        [
            ("size", DiagArg::Str("L".to_owned())),
            ("max", DiagArg::Int(4)),
            ("length", DiagArg::Int(5)),
        ]
    );
}

#[test]
fn a_long_operand_text_is_truncated() {
    let long = format!("({})", "1+".repeat(40));
    let (_, problem) = rejected(&long);
    assert_eq!(problem.code, ASM_BAD_EXPRESSION);
    let DiagArg::Str(shown) = &problem.args[0].1 else {
        panic!("{problem:?}")
    };
    assert_eq!(shown.chars().count(), 33);
    assert!(shown.ends_with('…'));
    assert!(long.starts_with(shown.trim_end_matches('…')));
}

#[test]
fn lexer_errors_and_macro_parameters_in_operands() {
    // The lexer's diagnostic, repeated exactly (code, token range, text).
    let (line, problem) = rejected("$");
    assert_eq!(problem.code, LEX_BAD_NUMBER);
    assert_eq!(text(&line, problem.range), "$");
    assert_eq!(problem.args, [("text", DiagArg::Str("$".to_owned()))]);
    let (line, problem) = rejected("4(a0,$),d1");
    assert_eq!(problem.code, LEX_BAD_NUMBER);
    assert_eq!(text(&line, problem.range), "$");
    let (line, problem) = rejected("1<2");
    assert_eq!(problem.code, LEX_UNEXPECTED_CHAR);
    assert_eq!(text(&line, problem.range), "<");
    for (written, param) in [("\\1", "\\1"), ("lbl\\@", "\\@"), ("4(a0,\\2)", "\\2")] {
        let (line, problem) = rejected(written);
        assert_eq!(problem.code, ASM_MACRO_PARAM_OUTSIDE, "{written:?}");
        assert_eq!(text(&line, problem.range), param);
        assert_eq!(problem.args, [("text", DiagArg::Str(param.to_owned()))]);
    }
}

#[test]
fn operands_end_at_a_top_level_comma_or_the_field_end() {
    let line = "\tmove.l\t4(a0,d1.w),-(sp)\tsave it";
    let tokens = operand_tokens(line);
    let (first, used) = parse_operand(line, &tokens).unwrap();
    assert_eq!(text(line, first.range), "4(a0,d1.w)");
    assert_eq!(
        first.item,
        ea(Ea::Index(
            num(4),
            0,
            Xn {
                reg: 1,
                long: false
            }
        ))
    );
    assert_eq!(tokens[used].kind, TokenKind::Punctuation);
    let (second, rest) = parse_operand(line, &tokens[used + 1..]).unwrap();
    assert_eq!(text(line, second.range), "-(sp)");
    assert_eq!(second.item, ea(Ea::PreDec(7)));
    assert_eq!(tokens[used + 1 + rest].kind, TokenKind::Whitespace);

    // An operand list after a register list.
    let line = "\tmovem.l\td0-d7/a0-a6,-(sp)";
    let tokens = operand_tokens(line);
    let (list, used) = parse_operand(line, &tokens).unwrap();
    assert_eq!(list.item, Operand::RegList(0x7FFF));
    assert_eq!(text(line, list.range), "d0-d7/a0-a6");
    assert_eq!(used, 7);

    // An empty operand (a trailing comma) is a bad operand at the field end.
    let line = "\tmove.l\td0,";
    let tokens = operand_tokens(line);
    let (_, used) = parse_operand(line, &tokens).unwrap();
    let problem = parse_operand(line, &tokens[used + 1..]).unwrap_err();
    assert_eq!(problem.code, ASM_BAD_OPERAND);
    assert_eq!(problem.range, TextRange::new(11, 11));
    assert_eq!(problem.args, [("text", DiagArg::Str(String::new()))]);
}

#[test]
fn the_operand_range_and_count_ignore_the_suspect_comment() {
    let line = "\tmove.l\td0, d1";
    let tokens = operand_tokens(line);
    let (parsed, used) = parse_operand(line, &tokens).unwrap();
    assert_eq!(parsed.item, ea(Ea::Dn(0)));
    assert_eq!(used, 1);
    assert_eq!(
        parsed,
        Spanned {
            item: ea(Ea::Dn(0)),
            range: TextRange::new(8, 10)
        }
    );
}

/// The fields `(label, local, op, operand field)` of `line`, with the texts their ranges
/// cover, after checking that `line` has no problem.
fn read(line: &str) -> (Option<String>, bool, Option<OpField>, String) {
    let fields = fields(line);
    assert!(
        fields.problems.is_empty(),
        "{line:?}: {:?}",
        fields.problems
    );
    if let Some(label) = &fields.label {
        assert_eq!(text(line, label.range), label.item, "{line:?}");
    }
    (
        fields.label.map(|label| label.item),
        fields.local_label,
        fields.op.map(|op| op.item),
        text(line, fields.operand_field).to_owned(),
    )
}

/// `OpField::Mnemonic`.
fn mnemonic(spelling: &'static str, size: Option<Size>) -> Option<OpField> {
    Some(OpField::Mnemonic { spelling, size })
}

/// `OpField::Directive`.
fn directive(name: &'static str, size: Option<Size>) -> Option<OpField> {
    Some(OpField::Directive { name, size })
}

/// `OpField::Macro`.
fn macro_call(name: &str, suffix: Option<&str>) -> Option<OpField> {
    Some(OpField::Macro {
        name: name.to_owned(),
        suffix: suffix.map(str::to_owned),
    })
}

#[test]
fn label_operation_and_operand_fields() {
    let line = "start\tmove.l\t#$1234,d0\tload a constant";
    let parsed = fields(line);
    assert_eq!(
        parsed.label,
        Some(Spanned {
            item: "start".to_owned(),
            range: TextRange::new(0, 5)
        })
    );
    assert_eq!(text(line, parsed.op.as_ref().unwrap().range), "move.l");
    assert_eq!(
        read(line),
        (
            Some("start".to_owned()),
            false,
            mnemonic("MOVE", Some(Size::Long)),
            "#$1234,d0".to_owned()
        )
    );
    assert_eq!(
        read("1$:\tbra.s\t1$"),
        (
            Some("1$".to_owned()),
            true,
            mnemonic("BRA", Some(Size::Short)),
            "1$".to_owned()
        )
    );
    assert_eq!(
        read("\\loop::\tdbf\td0,\\loop"),
        (
            Some("\\loop".to_owned()),
            true,
            mnemonic("DBF", None),
            "d0,\\loop".to_owned()
        )
    );
    assert_eq!(
        read("  name: nop"),
        (
            Some("name".to_owned()),
            false,
            mnemonic("NOP", None),
            String::new()
        )
    );
    assert_eq!(
        read("\t1$:\tbra\t1$"),
        (
            Some("1$".to_owned()),
            true,
            mnemonic("BRA", None),
            "1$".to_owned()
        )
    );
    assert_eq!(
        read("lab"),
        (Some("lab".to_owned()), false, None, String::new())
    );
    assert_eq!(
        read("lab:"),
        (Some("lab".to_owned()), false, None, String::new())
    );
    assert_eq!(
        read("lab\t; just a label"),
        (Some("lab".to_owned()), false, None, String::new())
    );
    // Labels are case-sensitive and kept as written; mnemonics are uppercased.
    assert_eq!(
        read("Foo\tMoVe.W\td0,d1"),
        (
            Some("Foo".to_owned()),
            false,
            mnemonic("MOVE", Some(Size::Word)),
            "d0,d1".to_owned()
        )
    );
    assert_eq!(
        read("\tbhs\tx"),
        (None, false, mnemonic("BHS", None), "x".to_owned())
    );
    assert_eq!(
        read("\tDBRA\td0,x"),
        (None, false, mnemonic("DBRA", None), "d0,x".to_owned())
    );
}

#[test]
fn lines_without_fields() {
    for line in ["", "* comment", "; comment", "   ", "\t; indented comment"] {
        let parsed = fields(line);
        assert_eq!(parsed.label, None, "{line:?}");
        assert_eq!(parsed.op, None, "{line:?}");
        assert!(parsed.operand_field.is_empty(), "{line:?}");
        assert!(parsed.problems.is_empty(), "{line:?}");
    }
    assert_eq!(fields("").operand_field, TextRange::new(0, 0));
}

#[test]
fn operations_without_operands_have_an_empty_field_after_them() {
    let line = "\trts\td0, d1";
    let parsed = fields(line);
    assert_eq!(
        parsed.op.as_ref().unwrap().item,
        mnemonic("RTS", None).unwrap()
    );
    assert_eq!(parsed.operand_field, TextRange::new(4, 4));
    let line = "lab\teven\tcomment";
    let parsed = fields(line);
    assert_eq!(
        parsed.op.as_ref().unwrap().item,
        directive("EVEN", None).unwrap()
    );
    assert_eq!(parsed.operand_field, TextRange::new(8, 8));
    let parsed = fields("lab");
    assert_eq!(parsed.operand_field, TextRange::new(3, 3));
    let parsed = fields("\tnop;x");
    assert_eq!(parsed.operand_field, TextRange::new(4, 4));
}

#[test]
fn directives() {
    assert_eq!(
        read("x = 5"),
        (
            Some("x".to_owned()),
            false,
            directive("=", None),
            "5".to_owned()
        )
    );
    assert_eq!(
        read("\tdc.b\t1,2"),
        (
            None,
            false,
            directive("DC", Some(Size::Byte)),
            "1,2".to_owned()
        )
    );
    assert_eq!(
        read("\tDcB.L\t4,0"),
        (
            None,
            false,
            directive("DCB", Some(Size::Long)),
            "4,0".to_owned()
        )
    );
    assert_eq!(
        read("size\tequ\t*-start"),
        (
            Some("size".to_owned()),
            false,
            directive("EQU", None),
            "*-start".to_owned()
        )
    );
    assert_eq!(
        read("\tinclude\t\"file.i\""),
        (
            None,
            false,
            directive("INCLUDE", None),
            "\"file.i\"".to_owned()
        )
    );
    assert_eq!(
        read("\tIFC\t'a','b'"),
        (None, false, directive("IFC", None), "'a','b'".to_owned())
    );
    assert_eq!(
        read("swap2\tmacro"),
        (
            Some("swap2".to_owned()),
            false,
            directive("MACRO", None),
            String::new()
        )
    );
    for name in [
        "ENDM", "ENDC", "ENDIF", "MEXIT", "END", "ORG", "SECTION", "EQUR",
    ] {
        let line = format!("\t{}", name.to_ascii_lowercase());
        assert_eq!(read(&line).2, directive(name, None), "{line:?}");
    }
}

#[test]
fn macro_calls() {
    assert_eq!(
        read("\tswap2.w\td0,d1"),
        (
            None,
            false,
            macro_call("swap2", Some("w")),
            "d0,d1".to_owned()
        )
    );
    assert_eq!(
        read("\tSwap2.L\td0,d1"),
        (
            None,
            false,
            macro_call("Swap2", Some("L")),
            "d0,d1".to_owned()
        )
    );
    for not_a68k in ["ELSE", "REPT", "opt"] {
        let line = format!("\t{not_a68k}");
        assert_eq!(read(&line).2, macro_call(not_a68k, None), "{line:?}");
    }
    // `<…>` arguments (m2 §4.3): the operand field keeps their whitespace and `;`.
    assert_eq!(
        read("\tSwap2\t<4(a0,d0)>,d1"),
        (
            None,
            false,
            macro_call("Swap2", None),
            "<4(a0,d0)>,d1".to_owned()
        )
    );
    assert_eq!(
        read("lab\tcount\t<1 2>,<a;b>\tcomment"),
        (
            Some("lab".to_owned()),
            false,
            macro_call("count", None),
            "<1 2>,<a;b>".to_owned()
        )
    );
    assert_eq!(
        read("\tmac\t<x y>, <z>"),
        (None, false, macro_call("mac", None), "<x y>,".to_owned())
    );
}

#[test]
fn lexer_errors_in_the_fields_give_no_problem_and_no_field() {
    // A broken label field: no label.
    for line in ["lab:nop", "9x nop", "lab-1\tnop", "1$x"] {
        let parsed = fields(line);
        assert_eq!(parsed.label, None, "{line:?}");
        assert!(parsed.problems.is_empty(), "{line:?}");
    }
    // A broken operation field: no operation, but its operand field.
    let parsed = fields("\t123\td0");
    assert_eq!(parsed.op, None);
    assert!(parsed.problems.is_empty());
    assert_eq!(text("\t123\td0", parsed.operand_field), "d0");
    // A rejected suffix reads as no suffix.
    assert_eq!(read("\tmove.x\td0").2, mnemonic("MOVE", None));
    assert_eq!(read("\tdc.q\t1").2, directive("DC", None));
    assert_eq!(read("\tmac.foo\t1").2, macro_call("mac", None));
}

#[test]
fn macro_parameters_in_the_label_and_operation_fields() {
    for (line, params, label, op) in [
        ("lbl\\@:\tnop", vec!["\\@"], None, mnemonic("NOP", None)),
        ("\\1\tnop", vec!["\\1"], None, mnemonic("NOP", None)),
        (
            "\tmove.\\0\t\\1,d7",
            vec!["\\0"],
            None,
            mnemonic("MOVE", None),
        ),
        ("\tmac.\\0\td0", vec!["\\0"], None, macro_call("mac", None)),
        ("\tb\\1\ttarget", vec!["\\1"], None, None),
        ("\t\\1\\2\td0", vec!["\\1", "\\2"], None, None),
        ("x\tb\\1.\\0\td0", vec!["\\1", "\\0"], Some("x"), None),
    ] {
        let parsed = fields(line);
        assert_eq!(
            parsed.label.map(|label| label.item).as_deref(),
            label,
            "{line:?}"
        );
        assert_eq!(parsed.op.map(|op| op.item), op, "{line:?}");
        let found: Vec<(&str, DiagCode)> = parsed
            .problems
            .iter()
            .map(|problem| (text(line, problem.range), problem.code))
            .collect();
        let expected: Vec<(&str, DiagCode)> = params
            .iter()
            .map(|&param| (param, ASM_MACRO_PARAM_OUTSIDE))
            .collect();
        assert_eq!(found, expected, "{line:?}");
        for problem in &parsed.problems {
            let param = text(line, problem.range).to_owned();
            assert_eq!(problem.args, [("text", DiagArg::Str(param))]);
        }
    }
}

#[test]
fn a_column_one_keyword_alone_is_a_label_with_a_warning_and_a_fix() {
    for (line, name) in [
        ("nop", "nop"),
        ("RTS", "RTS"),
        ("even ; pad", "even"),
        ("Endm", "Endm"),
        ("data", "data"),
        ("MOVE.L", "MOVE.L"),
        ("bra.s\t", "bra.s"),
    ] {
        let parsed = fields(line);
        assert_eq!(parsed.label.as_ref().unwrap().item, name, "{line:?}");
        assert_eq!(parsed.op, None, "{line:?}");
        assert_eq!(parsed.problems.len(), 1, "{line:?}");
        let problem = &parsed.problems[0];
        assert_eq!(problem.code, ASM_LABEL_LIKE_KEYWORD);
        assert_eq!(text(line, problem.range), name);
        assert_eq!(problem.args, [("name", DiagArg::Str(name.to_owned()))]);
        assert_eq!(problem.fix, Some((TextRange::new(0, 0), "\t".to_owned())));
        // Applying the fix makes the line an instruction or a directive.
        let fixed = apply_fix(line, problem);
        let refixed = fields(&fixed);
        assert_eq!(refixed.label, None, "{fixed:?}");
        assert!(refixed.op.is_some(), "{fixed:?}");
        assert!(refixed.problems.is_empty(), "{fixed:?}");
    }
    // Not warned: a colon, an operation after it, an ordinary name, a local label, a size that
    // is no size suffix.
    for line in [
        "nop:", "nop nop", "rts\trts", "start", "1$", "nop.x", "loop", ".nop",
    ] {
        let parsed = fields(line);
        assert!(
            parsed.problems.is_empty(),
            "{line:?}: {:?}",
            parsed.problems
        );
    }
}
