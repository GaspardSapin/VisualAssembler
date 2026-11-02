//! Expressions (m2-assembler.md §4.2, frozen; §4.4): their syntax tree, how symbols resolve
//! ([`Scope`], [`Lookup`]), parsing from lexer tokens and evaluation to a 32-bit value.
//!
//! Values are 32-bit two's complement, sign-extended to `i64`; every operation wraps to 32 bits,
//! and so does a literal too long for 32 bits. Precedence, highest first, left-associative
//! within a level: unary `-` `+` `~`; `<<` `>>`; `&` `!`; `*` `/`; `+` `-` (observed on a68k,
//! §4.4). A unary operator directly after another one is an error (`--2`); `|` is no operator.
//! `/` truncates toward zero, `>>` is arithmetic, a shift by 32 or more (the count read as an
//! unsigned 32-bit value) gives 0 (`<<`) or the sign fill (`>>`).
//!
//! Terms: numbers (`$` hex, `%` binary, `@` octal, decimal), symbols, local labels (`1$`,
//! `\name`), `*` (the location counter) and quoted characters (1–4 bytes of UTF-8,
//! right-justified: `'ab'` = `$6162`), and parentheses.
//!
//! **Depth.** An expression nests at most 32 parentheses and its tree is at most 256 levels deep
//! (a chain of `n` binary operators is `n` levels): deeper input is `asm-bad-expression`, so that
//! parsing, evaluation, cloning and dropping stay within a small stack.

use va_core::diag::DiagArg;
use va_core::source::TextRange;
use va_core::syntax::{Token, TokenKind};

use super::codes::{
    ASM_BAD_EXPRESSION, ASM_DIVISION_BY_ZERO, ASM_STRING_TOO_LONG, ASM_UNDEFINED_LOCAL_LABEL,
    ASM_UNDEFINED_SYMBOL,
};
use super::parser::{Problem, is_punct, operand_extent, span, start_of, token_problem, token_text};

/// An expression.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr {
    /// A number (its 32-bit value, sign-extended).
    Num(i64),
    /// A quoted character constant (1–4 characters, right-justified; its 32-bit value,
    /// sign-extended).
    Char(i64),
    /// `*`, the location counter of the statement.
    Here,
    /// A symbol or local label.
    Sym(SymRef),
    /// A unary operation.
    Unary(UnOp, Box<Expr>),
    /// A binary operation.
    Binary(BinOp, Box<Expr>, Box<Expr>),
}

/// A reference to a symbol in an expression.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymRef {
    /// The name as written (symbols are case-sensitive; a local label keeps its `$` or `\`).
    pub name: String,
    /// Whether it is a local label (`1$`, `\name`).
    pub local: bool,
    /// Where it is written, line-relative.
    pub range: TextRange,
}

/// A unary operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UnOp {
    /// `-`
    Neg,
    /// `+`
    Plus,
    /// `~`
    Not,
}

/// A binary operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BinOp {
    /// `<<`
    Shl,
    /// `>>` (arithmetic)
    Shr,
    /// `&`
    And,
    /// `!`
    Or,
    /// `*`
    Mul,
    /// `/` (truncates toward zero)
    Div,
    /// `+`
    Add,
    /// `-`
    Sub,
}

/// What a symbol resolves to at the statement being evaluated.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Lookup {
    /// Defined on an earlier statement (in expansion order), with this value.
    Known(i64),
    /// Defined further down, with this value (pass 2 only).
    Later(i64),
    /// Defined further down, value not known yet (pass 1).
    Unknown,
    /// Never defined.
    Undefined,
    /// Its definition failed: uses report nothing (§4.13).
    Poisoned,
}

/// How an expression sees the symbols and the location counter.
pub trait Scope {
    /// Resolves `sym`.
    fn lookup(&self, sym: &SymRef) -> Lookup;

    /// The location counter of the statement (`*`), after its alignment.
    fn here(&self) -> i64;
}

/// The value of an expression.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Value {
    /// The 32-bit result, sign-extended.
    pub value: i64,
    /// Whether no `Later` or `Unknown` symbol was involved.
    pub known: bool,
}

/// Most parentheses one expression may nest.
const MAX_NESTING: u32 = 32;

/// Deepest tree one expression may have.
const MAX_DEPTH: u32 = 256;

/// Most bytes a character constant may hold.
const MAX_CHAR_BYTES: usize = 4;

/// Returns `value` wrapped to 32 bits and sign-extended.
fn wrap(value: i64) -> i64 {
    i64::from(value as i32)
}

/// Returns the bytes of the quoted literal `text` (delimiters removed, a doubled delimiter
/// read as one), or `None` when `text` is not a closed quoted literal.
pub(crate) fn string_bytes(text: &str) -> Option<Vec<u8>> {
    let bytes = text.as_bytes();
    let quote = *bytes.first()?;
    if !(quote == b'\'' || quote == b'"') || bytes.len() < 2 || bytes.last() != Some(&quote) {
        return None;
    }
    let inner = bytes.get(1..bytes.len() - 1)?;
    let mut out = Vec::with_capacity(inner.len());
    let mut index = 0;
    while let Some(&byte) = inner.get(index) {
        out.push(byte);
        index += if byte == quote { 2 } else { 1 };
    }
    Some(out)
}

/// Returns the value of the `Number` token text `text`, wrapped to 32 bits.
fn number_value(text: &str) -> Option<i64> {
    let (digits, radix) = match text.as_bytes().first()? {
        b'$' => (text.get(1..)?, 16),
        b'%' => (text.get(1..)?, 2),
        b'@' => (text.get(1..)?, 8),
        _ => (text, 10),
    };
    if digits.is_empty() {
        return None;
    }
    let mut value: u32 = 0;
    for char in digits.chars() {
        let digit = char.to_digit(radix)?;
        value = value.wrapping_mul(radix).wrapping_add(digit);
    }
    Some(i64::from(value as i32))
}

/// The binary operator spelled `text`, with its precedence level (1 lowest, 4 highest).
fn binary_operator(text: &str) -> Option<(BinOp, u8)> {
    Some(match text {
        "+" => (BinOp::Add, 1),
        "-" => (BinOp::Sub, 1),
        "*" => (BinOp::Mul, 2),
        "/" => (BinOp::Div, 2),
        "&" => (BinOp::And, 3),
        "!" => (BinOp::Or, 3),
        "<<" => (BinOp::Shl, 4),
        ">>" => (BinOp::Shr, 4),
        _ => return None,
    })
}

/// The unary operator spelled `text`.
fn unary_operator(text: &str) -> Option<UnOp> {
    match text {
        "-" => Some(UnOp::Neg),
        "+" => Some(UnOp::Plus),
        "~" => Some(UnOp::Not),
        _ => None,
    }
}

/// Why parsing stopped short of an expression.
enum Failure {
    /// The tokens do not form an expression (`asm-bad-expression` over the whole extent).
    Bad,
    /// A more precise problem (a character constant too long).
    Problem(Problem),
}

/// A precedence-climbing parser over the tokens of one operand.
struct Parser<'a> {
    /// The line the token ranges point into.
    line: &'a str,
    /// The tokens of the operand (its extent only).
    tokens: &'a [Token],
    /// The next token.
    at: usize,
    /// Parentheses currently open.
    nesting: u32,
}

impl<'a> Parser<'a> {
    /// Returns the next token and its text, if any.
    fn peek(&self) -> Option<(&'a Token, &'a str)> {
        self.tokens
            .get(self.at)
            .map(|token| (token, token_text(self.line, token)))
    }

    /// Parses the binary levels from `level` up (1 is `+ -`), then the unary level; returns the
    /// expression and its depth.
    fn binary(&mut self, level: u8) -> Result<(Expr, u32), Failure> {
        if level > 4 {
            return self.unary();
        }
        let (mut left, mut depth) = self.binary(level + 1)?;
        while let Some((token, text)) = self.peek() {
            if token.kind != TokenKind::Operator {
                break;
            }
            let Some((op, op_level)) = binary_operator(text) else {
                return Err(Failure::Bad);
            };
            if op_level != level {
                break;
            }
            self.at += 1;
            let (right, right_depth) = self.binary(level + 1)?;
            depth = depth.max(right_depth) + 1;
            if depth > MAX_DEPTH {
                return Err(Failure::Bad);
            }
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok((left, depth))
    }

    /// Parses an optional unary operator and a primary term.
    fn unary(&mut self) -> Result<(Expr, u32), Failure> {
        let Some((token, text)) = self.peek() else {
            return Err(Failure::Bad);
        };
        if token.kind == TokenKind::Operator
            && let Some(op) = unary_operator(text)
        {
            self.at += 1;
            if self.peek().is_some_and(|(next, next_text)| {
                next.kind == TokenKind::Operator && unary_operator(next_text).is_some()
            }) {
                return Err(Failure::Bad);
            }
            let (operand, depth) = self.primary()?;
            return Ok((Expr::Unary(op, Box::new(operand)), depth + 1));
        }
        self.primary()
    }

    /// Parses a primary term: a number, a character constant, a symbol, `*` or a parenthesized
    /// expression.
    fn primary(&mut self) -> Result<(Expr, u32), Failure> {
        let Some((&token, text)) = self.peek() else {
            return Err(Failure::Bad);
        };
        self.at += 1;
        let term = match token.kind {
            TokenKind::Number => Expr::Num(number_value(text).ok_or(Failure::Bad)?),
            TokenKind::String => Expr::Char(self.character(&token, text)?),
            TokenKind::Symbol | TokenKind::LocalLabel => Expr::Sym(SymRef {
                name: text.to_owned(),
                local: token.kind == TokenKind::LocalLabel,
                range: token.range,
            }),
            TokenKind::Operator if text == "*" => Expr::Here,
            TokenKind::Punctuation if text == "(" => {
                self.nesting += 1;
                if self.nesting > MAX_NESTING {
                    return Err(Failure::Bad);
                }
                let inner = self.binary(1)?;
                match self.peek() {
                    Some((close, _)) if is_punct(self.line, close, ')') => self.at += 1,
                    _ => return Err(Failure::Bad),
                }
                self.nesting -= 1;
                return Ok(inner);
            }
            _ => return Err(Failure::Bad),
        };
        Ok((term, 1))
    }

    /// Returns the value of the character constant `token` (1–4 bytes, right-justified).
    fn character(&self, token: &Token, text: &str) -> Result<i64, Failure> {
        let bytes = string_bytes(text).ok_or(Failure::Bad)?;
        if bytes.is_empty() {
            return Err(Failure::Bad);
        }
        if bytes.len() > MAX_CHAR_BYTES {
            return Err(Failure::Problem(
                Problem::new(ASM_STRING_TOO_LONG, token.range)
                    .with_arg("size", DiagArg::Str("L".to_owned()))
                    .with_arg("max", DiagArg::Int(MAX_CHAR_BYTES as i64))
                    .with_arg(
                        "length",
                        DiagArg::Int(i64::try_from(bytes.len()).unwrap_or(i64::MAX)),
                    ),
            ));
        }
        let value = bytes
            .iter()
            .fold(0_u32, |value, &byte| (value << 8) | u32::from(byte));
        Ok(i64::from(value as i32))
    }
}

/// Parses an expression at the start of `tokens` (line-relative ranges into `line`); returns it
/// with the number of tokens it used.
///
/// The expression ends before the first token that cannot continue it (a `,`, a `(` after a
/// term, a size suffix, whitespace, …); the caller checks what follows. The operand it belongs
/// to runs to the first `,` outside parentheses or the end of the operand field.
///
/// # Errors
///
/// The [`Problem`] of an expression that does not parse: `asm-bad-expression` over the operand
/// (its `text` argument), `asm-string-too-long` for a character constant of more than 4 bytes,
/// `asm-macro-param-outside` for a macro parameter, or, for an `Invalid` token of the operand,
/// the lexer's own diagnostic repeated (its `lex-*` code; see `parser`).
pub fn parse_expr(line: &str, tokens: &[Token]) -> Result<(Expr, usize), Problem> {
    let extent = operand_extent(line, tokens);
    let operand = tokens.get(..extent).unwrap_or(&[]);
    if let Some(problem) = token_problem(line, operand) {
        return Err(problem);
    }
    let mut parser = Parser {
        line,
        tokens: operand,
        at: 0,
        nesting: 0,
    };
    match parser.binary(1) {
        Ok((expr, _)) => Ok((expr, parser.at)),
        Err(Failure::Problem(problem)) => Err(problem),
        Err(Failure::Bad) => Err(bad_expression(line, operand, start_of(line, tokens))),
    }
}

/// Returns the `asm-bad-expression` problem over the tokens `operand`.
pub(crate) fn bad_expression(line: &str, operand: &[Token], fallback: u32) -> Problem {
    Problem::new(ASM_BAD_EXPRESSION, span(operand, fallback)).with_text(line)
}

/// A value during evaluation.
#[derive(Clone, Copy)]
struct Partial {
    /// The 32-bit value, sign-extended (0 for an `Unknown` symbol).
    value: i64,
    /// Whether no `Later` or `Unknown` symbol was involved.
    known: bool,
    /// Whether an `Unknown` symbol was involved (the value is a placeholder).
    unknown: bool,
}

impl Partial {
    /// A value that involves no symbol.
    const fn exact(value: i64) -> Self {
        Self {
            value,
            known: true,
            unknown: false,
        }
    }
}

/// Why evaluation failed.
enum EvalError {
    /// A `Poisoned` symbol was used (the driver reports nothing for it).
    Poisoned(Problem),
    /// Any other problem.
    Other(Problem),
}

impl EvalError {
    /// Returns the problem.
    fn into_problem(self) -> Problem {
        match self {
            Self::Poisoned(problem) | Self::Other(problem) => problem,
        }
    }
}

/// Keeps the failure that wins: a poisoned use over anything else, else the leftmost one.
fn first_failure(left: EvalError, right: EvalError) -> EvalError {
    match (left, right) {
        (EvalError::Other(_), poisoned @ EvalError::Poisoned(_)) => poisoned,
        (left, _) => left,
    }
}

/// Returns the problem of the undefined (or poisoned) symbol `sym`.
fn undefined(sym: &SymRef) -> Problem {
    if sym.local {
        Problem::new(ASM_UNDEFINED_LOCAL_LABEL, sym.range)
            .with_arg("name", DiagArg::Str(sym.name.clone()))
    } else {
        Problem::new(ASM_UNDEFINED_SYMBOL, sym.range)
            .with_arg("name", DiagArg::Str(sym.name.clone()))
            .with_arg("count", DiagArg::Int(0))
            .with_arg("suggestions", DiagArg::Str(String::new()))
    }
}

/// Applies the binary operator `op` to two 32-bit values.
fn apply(op: BinOp, left: i32, right: i32) -> i32 {
    match op {
        BinOp::Shl => {
            if right as u32 >= 32 {
                0
            } else {
                left.wrapping_shl(right as u32)
            }
        }
        BinOp::Shr => {
            if right as u32 >= 32 {
                if left < 0 { -1 } else { 0 }
            } else {
                left >> right
            }
        }
        BinOp::And => left & right,
        BinOp::Or => left | right,
        BinOp::Mul => left.wrapping_mul(right),
        BinOp::Div => left.wrapping_div(right),
        BinOp::Add => left.wrapping_add(right),
        BinOp::Sub => left.wrapping_sub(right),
    }
}

/// Evaluates `expr`, looking up every symbol of it, left to right, even after a failure.
fn evaluate(expr: &Expr, scope: &dyn Scope) -> Result<Partial, EvalError> {
    match expr {
        Expr::Num(value) | Expr::Char(value) => Ok(Partial::exact(wrap(*value))),
        Expr::Here => Ok(Partial::exact(wrap(scope.here()))),
        Expr::Sym(sym) => match scope.lookup(sym) {
            Lookup::Known(value) => Ok(Partial::exact(wrap(value))),
            Lookup::Later(value) => Ok(Partial {
                value: wrap(value),
                known: false,
                unknown: false,
            }),
            Lookup::Unknown => Ok(Partial {
                value: 0,
                known: false,
                unknown: true,
            }),
            Lookup::Undefined => Err(EvalError::Other(undefined(sym))),
            Lookup::Poisoned => Err(EvalError::Poisoned(undefined(sym))),
        },
        Expr::Unary(op, operand) => {
            let mut value = evaluate(operand, scope)?;
            let bits = value.value as i32;
            value.value = i64::from(match op {
                UnOp::Neg => bits.wrapping_neg(),
                UnOp::Plus => bits,
                UnOp::Not => !bits,
            });
            Ok(value)
        }
        Expr::Binary(op, left, right) => {
            let (left, right) = match (evaluate(left, scope), evaluate(right, scope)) {
                (Ok(left), Ok(right)) => (left, right),
                (Err(left), Err(right)) => return Err(first_failure(left, right)),
                (Err(failure), Ok(_)) | (Ok(_), Err(failure)) => return Err(failure),
            };
            let known = left.known && right.known;
            let unknown = left.unknown || right.unknown;
            if *op == BinOp::Div && right.value == 0 {
                if right.unknown {
                    return Ok(Partial {
                        value: 0,
                        known,
                        unknown,
                    });
                }
                return Err(EvalError::Other(Problem::new(
                    ASM_DIVISION_BY_ZERO,
                    TextRange::default(),
                )));
            }
            Ok(Partial {
                value: i64::from(apply(*op, left.value as i32, right.value as i32)),
                known,
                unknown,
            })
        }
    }
}

/// Evaluates `expr` in `scope`.
///
/// Every symbol of `expr` is looked up once, left to right, even after a failure, so a scope
/// that records its lookups sees every use. An `Unknown` symbol counts as 0 (and a division by
/// it gives 0); the result is then not `known`.
///
/// # Errors
///
/// The [`Problem`] of an expression that has no value, the leftmost one: `asm-undefined-symbol`
/// (arguments `name`, `count` = 0 and `suggestions` = "", which the driver may complete) or
/// `asm-undefined-local-label` (`name`) over the symbol's range; `asm-division-by-zero` with an
/// **empty range at 0**, since an `Expr` carries no position: the caller sets the range of the
/// operand it evaluated. A `Poisoned` symbol fails like an undefined one and wins over any
/// other failure; the driver, whose scope answered `Poisoned`, reports nothing for it (§4.13).
pub fn eval(expr: &Expr, scope: &dyn Scope) -> Result<Value, Problem> {
    evaluate(expr, scope)
        .map(|partial| Value {
            value: partial.value,
            known: partial.known,
        })
        .map_err(EvalError::into_problem)
}
