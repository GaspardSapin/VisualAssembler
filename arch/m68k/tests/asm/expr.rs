//! Expressions (`src/asm/expr.rs`; m2 §4.4, §10.1): precedence examples, 32-bit wrap, `/` toward
//! zero, `>>` sign, shifts by 32 or more, characters, `*` and `--2` (WP-B).
#![allow(clippy::unwrap_used)]

use std::cell::RefCell;

use va_arch_m68k::asm::codes::{
    ASM_BAD_EXPRESSION, ASM_DIVISION_BY_ZERO, ASM_MACRO_PARAM_OUTSIDE, ASM_STRING_TOO_LONG,
    ASM_UNDEFINED_LOCAL_LABEL, ASM_UNDEFINED_SYMBOL,
};
use va_arch_m68k::asm::expr::{BinOp, Expr, Lookup, Scope, SymRef, UnOp, Value, eval, parse_expr};
use va_arch_m68k::asm::parser::{Problem, parse_line};
use va_arch_m68k::syntax::{self, LEX_BAD_NUMBER};
use va_core::diag::DiagArg;
use va_core::source::TextRange;
use va_core::syntax::Token;

/// The location counter of [`TestScope`].
const HERE: i64 = 0x1000;

/// A scope with a fixed table of symbols that records every lookup.
struct TestScope {
    /// The symbols and what they resolve to; any other name is `Undefined`.
    symbols: Vec<(&'static str, Lookup)>,
    /// The names looked up, in order.
    looked_up: RefCell<Vec<String>>,
}

impl TestScope {
    /// The scope used by most tests.
    fn new() -> Self {
        Self {
            symbols: vec![
                ("five", Lookup::Known(5)),
                ("Five", Lookup::Known(50)),
                ("minus", Lookup::Known(-1)),
                ("big", Lookup::Known(0x1_2345_6789)),
                ("later", Lookup::Later(7)),
                ("unknown", Lookup::Unknown),
                ("zero", Lookup::Known(0)),
                ("poison", Lookup::Poisoned),
                ("1$", Lookup::Known(0x2000)),
                ("\\name", Lookup::Later(0x3000)),
            ],
            looked_up: RefCell::new(Vec::new()),
        }
    }
}

impl Scope for TestScope {
    fn lookup(&self, sym: &SymRef) -> Lookup {
        self.looked_up.borrow_mut().push(sym.name.clone());
        self.symbols
            .iter()
            .find(|(name, _)| *name == sym.name)
            .map_or(Lookup::Undefined, |&(_, lookup)| lookup)
    }

    fn here(&self) -> i64 {
        HERE
    }
}

/// The line `\tdc.l\t<text>`, whose operand field is `text`.
fn line_of(text: &str) -> String {
    format!("\tdc.l\t{text}")
}

/// The tokens of `line` from the start of its operand field.
fn operand_tokens(line: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    syntax::lex_line(line, &mut tokens);
    let start = parse_line(line, &tokens).operand_field.start;
    let first = tokens
        .iter()
        .position(|token| token.range.start == start)
        .unwrap_or(tokens.len());
    tokens[first..].to_vec()
}

/// Parses `text` (a whole operand field) and asserts that the expression uses all of it.
fn parse(text: &str) -> Expr {
    let line = line_of(text);
    let tokens = operand_tokens(&line);
    let (expr, used) =
        parse_expr(&line, &tokens).unwrap_or_else(|problem| panic!("{text:?}: {problem:?}"));
    assert_eq!(used, tokens.len(), "{text:?}");
    expr
}

/// Parses and evaluates `text` in [`TestScope::new`].
fn evaluate(text: &str) -> Result<Value, Problem> {
    eval(&parse(text), &TestScope::new())
}

/// The value of `text`, which must evaluate and be known.
fn value(text: &str) -> i64 {
    let value = evaluate(text).unwrap_or_else(|problem| panic!("{text:?}: {problem:?}"));
    assert!(value.known, "{text:?}");
    value.value
}

/// The problem of parsing `text`, with the line it was read in.
fn parse_problem(text: &str) -> (String, Problem) {
    let line = line_of(text);
    let tokens = operand_tokens(&line);
    match parse_expr(&line, &tokens) {
        Ok(parsed) => panic!("{text:?} parsed as {parsed:?}"),
        Err(problem) => (line, problem),
    }
}

/// Asserts that `text` does not parse: `asm-bad-expression` over all of it.
fn check_bad(text: &str) {
    let (line, problem) = parse_problem(text);
    assert_eq!(problem.code, ASM_BAD_EXPRESSION, "{text:?}: {problem:?}");
    assert_eq!(problem.range.slice(&line), Some(text), "{text:?}");
    assert_eq!(problem.args, [("text", DiagArg::Str(shown(text)))]);
    assert_eq!(problem.fix, None);
}

/// The `text` argument of `text`: at most 32 chars, then `…` (m2 §7.2).
fn shown(text: &str) -> String {
    match text.char_indices().nth(32) {
        Some((cut, _)) => format!("{}…", &text[..cut]),
        None => text.to_owned(),
    }
}

#[test]
fn precedence_examples_observed_on_a68k() {
    assert_eq!(value("1+2<<1"), 5);
    assert_eq!(value("2*3&1"), 2);
    assert_eq!(value("1!2&0"), 0);
    assert_eq!(value("2<<1*3"), 12);
    assert_eq!(value("1!1<<1"), 3);
    assert_eq!(value("~1+1"), -1);
    assert_eq!(value("~1+1") & 0xFFFF, 0xFFFF);
    assert_eq!(value("6/2*3"), 9);
}

#[test]
fn levels_are_left_associative_and_parentheses_group() {
    assert_eq!(value("10-3-2"), 5);
    assert_eq!(value("100/10/5"), 2);
    assert_eq!(value("1<<2<<3"), 32);
    assert_eq!(value("12&10!1"), 9);
    assert_eq!(value("(1+2)*3"), 9);
    assert_eq!(value("2*(3+4)"), 14);
    assert_eq!(value("((((1))))"), 1);
    assert_eq!(
        parse("1-2-3"),
        Expr::Binary(
            BinOp::Sub,
            Box::new(Expr::Binary(
                BinOp::Sub,
                Box::new(Expr::Num(1)),
                Box::new(Expr::Num(2))
            )),
            Box::new(Expr::Num(3))
        )
    );
}

#[test]
fn unary_operators() {
    assert_eq!(value("-2"), -2);
    assert_eq!(value("+2"), 2);
    assert_eq!(value("~0"), -1);
    assert_eq!(value("-2<<1"), -4);
    assert_eq!(value("-(-2)"), 2);
    assert_eq!(value("3--2"), 5);
    assert_eq!(value("7/-2"), -3);
    assert_eq!(parse("~5"), Expr::Unary(UnOp::Not, Box::new(Expr::Num(5))));
    // A unary operator right after another one (a68k: "Error in operand format").
    for text in ["--2", "-+2", "~-1", "+~1", "3---2"] {
        check_bad(text);
    }
}

#[test]
fn every_operation_wraps_to_32_bits() {
    assert_eq!(value("$7FFFFFFF+1"), i64::from(i32::MIN));
    assert_eq!(value("-$80000000"), i64::from(i32::MIN));
    assert_eq!(value("$80000000-1"), i64::from(i32::MAX));
    assert_eq!(value("$10000*$10000"), 0);
    assert_eq!(value("$FFFFFFFF"), -1);
    assert_eq!(value("4294967295"), -1);
    // A literal longer than 32 bits keeps its low 32 bits.
    assert_eq!(value("$100000000"), 0);
    assert_eq!(value("$123456789"), 0x2345_6789);
    // So do symbol values and the location counter given by the scope.
    assert_eq!(value("big"), 0x2345_6789);
    assert_eq!(
        eval(&Expr::Num(0x1_0000_0001), &TestScope::new()).unwrap(),
        Value {
            value: 1,
            known: true
        }
    );
}

#[test]
fn division_truncates_toward_zero() {
    assert_eq!(value("7/2"), 3);
    assert_eq!(value("-7/2"), -3);
    assert_eq!(value("-7/-2"), 3);
    assert_eq!(value("$80000000/-1"), i64::from(i32::MIN));
    for text in ["1/0", "1/(2-2)", "five/zero"] {
        let problem = evaluate(text).unwrap_err();
        assert_eq!(problem.code, ASM_DIVISION_BY_ZERO, "{text:?}");
        assert_eq!(problem.range, TextRange::default(), "{text:?}");
        assert!(problem.args.is_empty());
    }
    // Dividing by a symbol not known yet (pass 1) is no error: the value is a placeholder.
    let value = evaluate("five/unknown").unwrap();
    assert!(!value.known);
}

#[test]
fn shift_right_is_arithmetic() {
    assert_eq!(value("-1>>1"), -1);
    assert_eq!(value("-1>>1") as u32, 0xFFFF_FFFF);
    assert_eq!(value("-8>>1"), -4);
    assert_eq!(value("$80000000>>31"), -1);
    assert_eq!(value("8>>1"), 4);
}

#[test]
fn shifts_by_32_or_more() {
    assert_eq!(value("1<<31"), i64::from(i32::MIN));
    assert_eq!(value("1<<32"), 0);
    assert_eq!(value("1<<100"), 0);
    assert_eq!(value("1>>32"), 0);
    assert_eq!(value("$40000000>>40"), 0);
    assert_eq!(value("-1>>32"), -1);
    assert_eq!(value("$80000000>>99"), -1);
    // The count is read as an unsigned 32-bit value: -1 is 2^32 - 1.
    assert_eq!(value("1<<-1"), 0);
    assert_eq!(value("-4>>-1"), -1);
}

#[test]
fn numbers_in_every_base() {
    assert_eq!(value("$1F"), 31);
    assert_eq!(value("$abcdef"), 0xAB_CDEF);
    assert_eq!(value("%1010"), 10);
    assert_eq!(value("@17"), 15);
    assert_eq!(value("42"), 42);
    assert_eq!(value("007"), 7);
    assert_eq!(parse("$10"), Expr::Num(16));
}

#[test]
fn quoted_characters_are_right_justified() {
    assert_eq!(value("'a'"), 0x61);
    assert_eq!(value("'ab'"), 0x6162);
    assert_eq!(value("'abcd'"), 0x6162_6364);
    assert_eq!(value("\"AB\""), 0x4142);
    assert_eq!(value("'it''s'"), 0x6974_2773);
    assert_eq!(value("'é'"), 0xC3A9);
    assert_eq!(value("'ab'+1"), 0x6163);
    assert_eq!(value("'\u{7f}\u{7f}\u{7f}\u{7f}'"), 0x7F7F_7F7F);
    assert_eq!(value("'ÿÿ'"), 0xC3BF_C3BF_u32 as i32 as i64);
    assert_eq!(parse("'ab'"), Expr::Char(0x6162));
    let (line, problem) = parse_problem("'abcde'");
    assert_eq!(problem.code, ASM_STRING_TOO_LONG);
    assert_eq!(problem.range.slice(&line), Some("'abcde'"));
    assert_eq!(
        problem.args,
        [
            ("size", DiagArg::Str("L".to_owned())),
            ("max", DiagArg::Int(4)),
            ("length", DiagArg::Int(5)),
        ]
    );
    let (_, problem) = parse_problem("1+'ééé'");
    assert_eq!(problem.code, ASM_STRING_TOO_LONG);
    assert_eq!(problem.args[2], ("length", DiagArg::Int(6)));
    check_bad("''");
}

#[test]
fn the_location_counter() {
    assert_eq!(value("*"), HERE);
    assert_eq!(value("*+4"), HERE + 4);
    assert_eq!(value("**2"), 2 * HERE);
    assert_eq!(value("2**"), 2 * HERE);
    assert_eq!(value("*-*"), 0);
    assert_eq!(value("-*"), -HERE);
    assert_eq!(parse("*"), Expr::Here);
}

#[test]
fn symbols_resolve_through_the_scope() {
    assert_eq!(value("five+1"), 6);
    // Symbols are case-sensitive: the name is passed as written.
    assert_eq!(value("Five"), 50);
    assert_eq!(value("minus*3"), -3);
    assert_eq!(value("1$+1"), 0x2001);
    assert_eq!(
        parse("1$"),
        Expr::Sym(SymRef {
            name: "1$".to_owned(),
            local: true,
            range: TextRange::new(6, 8)
        })
    );
    assert_eq!(
        parse("five"),
        Expr::Sym(SymRef {
            name: "five".to_owned(),
            local: false,
            range: TextRange::new(6, 10)
        })
    );
    // `Later` gives its value, but the result is not known.
    assert_eq!(
        evaluate("later*2").unwrap(),
        Value {
            value: 14,
            known: false
        }
    );
    assert_eq!(
        evaluate("\\name").unwrap(),
        Value {
            value: 0x3000,
            known: false
        }
    );
    // `Unknown` counts as 0 and is not known.
    assert_eq!(
        evaluate("unknown+3").unwrap(),
        Value {
            value: 3,
            known: false
        }
    );
    assert!(evaluate("five+five").unwrap().known);
}

#[test]
fn undefined_symbols_and_local_labels() {
    let line = line_of("five+undef1*2");
    let tokens = operand_tokens(&line);
    let (expr, _) = parse_expr(&line, &tokens).unwrap();
    let problem = eval(&expr, &TestScope::new()).unwrap_err();
    assert_eq!(problem.code, ASM_UNDEFINED_SYMBOL);
    assert_eq!(problem.range.slice(&line), Some("undef1"));
    assert_eq!(
        problem.args,
        [
            ("name", DiagArg::Str("undef1".to_owned())),
            ("count", DiagArg::Int(0)),
            ("suggestions", DiagArg::Str(String::new())),
        ]
    );
    // Case-sensitive: `FIVE` is not `five`.
    assert_eq!(evaluate("FIVE").unwrap_err().code, ASM_UNDEFINED_SYMBOL);
    for local in ["2$", "\\other"] {
        let line = line_of(local);
        let tokens = operand_tokens(&line);
        let (expr, _) = parse_expr(&line, &tokens).unwrap();
        let problem = eval(&expr, &TestScope::new()).unwrap_err();
        assert_eq!(problem.code, ASM_UNDEFINED_LOCAL_LABEL, "{local:?}");
        assert_eq!(problem.range.slice(&line), Some(local));
        assert_eq!(problem.args, [("name", DiagArg::Str(local.to_owned()))]);
    }
}

#[test]
fn every_symbol_is_looked_up_and_the_leftmost_failure_wins() {
    let scope = TestScope::new();
    let problem = eval(&parse("undef1+undef2/0+five"), &scope).unwrap_err();
    assert_eq!(problem.args[0], ("name", DiagArg::Str("undef1".to_owned())));
    assert_eq!(
        *scope.looked_up.borrow(),
        ["undef1", "undef2", "five"].map(str::to_owned)
    );
    let scope = TestScope::new();
    let problem = eval(&parse("(1/0)+undef1"), &scope).unwrap_err();
    assert_eq!(problem.code, ASM_DIVISION_BY_ZERO);
    assert_eq!(*scope.looked_up.borrow(), ["undef1".to_owned()]);
}

#[test]
fn a_poisoned_symbol_fails_and_wins_over_other_failures() {
    let line = line_of("undef1+(1/0)-poison");
    let tokens = operand_tokens(&line);
    let (expr, _) = parse_expr(&line, &tokens).unwrap();
    let scope = TestScope::new();
    let problem = eval(&expr, &scope).unwrap_err();
    assert_eq!(problem.code, ASM_UNDEFINED_SYMBOL);
    assert_eq!(problem.range.slice(&line), Some("poison"));
    assert_eq!(problem.args[0], ("name", DiagArg::Str("poison".to_owned())));
    assert_eq!(
        *scope.looked_up.borrow(),
        ["undef1", "poison"].map(str::to_owned)
    );
    assert_eq!(evaluate("-poison").unwrap_err().code, ASM_UNDEFINED_SYMBOL);
}

#[test]
fn the_expression_stops_where_it_cannot_continue() {
    for (text, used) in [
        ("4(a0)", 1),
        ("lab+2(pc)", 3),
        ("($400).w", 3),
        ("$400.w", 1),
        ("1+2,3", 3),
        ("(1),2", 3),
        ("1 comment", 1),
        ("1, 2", 1),
        ("lab(a0,d0)", 1),
    ] {
        let line = line_of(text);
        let tokens = operand_tokens(&line);
        let (_, found) = parse_expr(&line, &tokens).unwrap();
        assert_eq!(found, used, "{text:?}");
    }
}

#[test]
fn expressions_that_do_not_parse() {
    for text in [
        "1+", "(1+2", ")", "1|2", "5~6", "~", "#1", "d0+1", "(d0)", "1+(2,3)", ",",
    ] {
        if text == "," {
            // An empty expression: the problem is the empty range at its start.
            let (line, problem) = parse_problem(text);
            assert_eq!(problem.code, ASM_BAD_EXPRESSION);
            assert_eq!(problem.range, TextRange::new(6, 6));
            assert_eq!(problem.range.slice(&line), Some(""));
            continue;
        }
        check_bad(text);
    }
}

#[test]
fn lexer_errors_and_macro_parameters() {
    let (line, problem) = parse_problem("1+$");
    assert_eq!(problem.code, LEX_BAD_NUMBER);
    assert_eq!(problem.range.slice(&line), Some("$"));
    assert_eq!(problem.args, [("text", DiagArg::Str("$".to_owned()))]);
    let (line, problem) = parse_problem("\\1+1");
    assert_eq!(problem.code, ASM_MACRO_PARAM_OUTSIDE);
    assert_eq!(problem.range.slice(&line), Some("\\1"));
    assert_eq!(problem.args, [("text", DiagArg::Str("\\1".to_owned()))]);
}

#[test]
fn nesting_and_depth_are_bounded() {
    let nested = |depth: usize| format!("{}1{}", "(".repeat(depth), ")".repeat(depth));
    assert_eq!(value(&nested(32)), 1);
    check_bad(&nested(33));
    // A chain of n binary operators is n levels deep: 255 fit, 256 do not.
    let chain = |operators: usize| format!("1{}", "+1".repeat(operators));
    assert_eq!(value(&chain(255)), 256);
    check_bad(&chain(256));
    // A pathological line stays a bad expression (no stack overflow), however long.
    let line = line_of(&chain(30_000));
    let tokens = operand_tokens(&line);
    assert_eq!(
        parse_expr(&line, &tokens).unwrap_err().code,
        ASM_BAD_EXPRESSION
    );
    let line = line_of(&nested(20_000));
    let tokens = operand_tokens(&line);
    assert_eq!(
        parse_expr(&line, &tokens).unwrap_err().code,
        ASM_BAD_EXPRESSION
    );
}
