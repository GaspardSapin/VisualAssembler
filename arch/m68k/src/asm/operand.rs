//! Operands (m2-assembler.md §4.2, frozen; §4.3): effective addresses, register lists, the
//! special registers and byte strings, parsed from lexer tokens in the strict A68k syntax.
//!
//! Accepted forms: `Dn An SP (An) (An)+ -(An) e(An) e(An,Xn[.W|.L]) e(PC) e(PC,Xn[.W|.L]) e
//! (e).W (e).L #e`, register lists `d0-d3/a0/a5-a6` (a range runs over the bit order D0–D7,
//! A0–A7 and must not go down), `CCR SR USP`, and a lone quoted string, which is
//! [`Operand::Bytes`] (a `DC` string; a string inside an expression is a character constant).
//! A single `Dn` or `An` is an [`Ea`], never a one-register list (the encoder makes that
//! choice). The index size defaults to `.W`; in `e(PC)` and `e(PC,Xn)`, `e` is the target
//! address.
//!
//! Rejected: `(a0,d0)` and `(pc,d0)` (`asm-missing-displacement`, fix `0(`), `$400.w`
//! (`asm-absolute-size-syntax`, fix `($400).w`), a size after a register outside an index such
//! as `d0.w` (`asm-register-size-suffix`), `(4,a0)` and every other shape (`asm-bad-operand`). An
//! expression that does not parse is `asm-bad-expression`, unless the operand holds a register
//! (then the addressing mode is what is wrong: `asm-bad-operand`).

use va_core::source::TextRange;
use va_core::syntax::{Token, TokenKind};

use super::codes::{
    ASM_ABSOLUTE_SIZE_SYNTAX, ASM_BAD_EXPRESSION, ASM_BAD_OPERAND, ASM_MISSING_DISPLACEMENT,
    ASM_REGISTER_SIZE_SUFFIX,
};
use super::expr::{Expr, parse_expr, string_bytes};
use super::parser::{
    Problem, is_punct, operand_extent, slice, span, start_of, token_problem, token_text,
};
use crate::isa::Size;

/// An index register of `d8(An,Xn)` or `d8(PC,Xn)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Xn {
    /// The register: 0–7 are D0–D7, 8–15 are A0–A7.
    pub reg: u8,
    /// Whether the index is `.L` (`.W` by default).
    pub long: bool,
}

/// An effective address as written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ea {
    /// `Dn`
    Dn(u8),
    /// `An` (and `SP` for A7)
    An(u8),
    /// `(An)`
    Ind(u8),
    /// `(An)+`
    PostInc(u8),
    /// `-(An)`
    PreDec(u8),
    /// `e(An)`
    Disp(Expr, u8),
    /// `e(An,Xn)`
    Index(Expr, u8, Xn),
    /// `e(PC)`, `e` being the target address
    PcDisp(Expr),
    /// `e(PC,Xn)`, `e` being the target address
    PcIndex(Expr, Xn),
    /// `e`, `(e).W` or `(e).L`
    Abs(Expr, Option<Size>),
    /// `#e`
    Imm(Expr),
}

/// One operand.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Operand {
    /// An effective address.
    Ea(Ea),
    /// A register list: bit n is Dn, bit 8 + n is An.
    RegList(u16),
    /// `CCR`
    Ccr,
    /// `SR`
    Sr,
    /// `USP`
    Usp,
    /// The bytes of a quoted string (a `DC` operand).
    Bytes(Vec<u8>),
}

/// A value with the line-relative range it was read from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spanned<T> {
    /// The value.
    pub item: T,
    /// Where it is written, line-relative.
    pub range: TextRange,
}

/// A register name of the operand rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reg {
    /// `D0`–`D7`.
    Data(u8),
    /// `A0`–`A7`, `SP`.
    Address(u8),
    /// `PC`.
    Pc,
    /// `USP`.
    Usp,
    /// `CCR`.
    Ccr,
    /// `SR`.
    Sr,
}

impl Reg {
    /// Returns the bit of a data or address register in a register list (Dn = n, An = 8 + n).
    const fn list_bit(self) -> Option<u8> {
        match self {
            Self::Data(n) => Some(n),
            Self::Address(n) => Some(8 + n),
            _ => None,
        }
    }
}

/// Reads the register `name` (any case).
fn register_named(name: &str) -> Option<Reg> {
    let upper = |byte: &u8| byte.to_ascii_uppercase();
    match name.as_bytes() {
        [kind, digit @ b'0'..=b'7'] => match upper(kind) {
            b'D' => Some(Reg::Data(digit - b'0')),
            b'A' => Some(Reg::Address(digit - b'0')),
            _ => None,
        },
        [first, second] => match (upper(first), upper(second)) {
            (b'S', b'P') => Some(Reg::Address(7)),
            (b'P', b'C') => Some(Reg::Pc),
            (b'S', b'R') => Some(Reg::Sr),
            _ => None,
        },
        [first, second, third] => match (upper(first), upper(second), upper(third)) {
            (b'U', b'S', b'P') => Some(Reg::Usp),
            (b'C', b'C', b'R') => Some(Reg::Ccr),
            _ => None,
        },
        _ => None,
    }
}

/// Returns the `asm-bad-operand` problem over `range`.
fn bad_operand(line: &str, range: TextRange) -> Problem {
    Problem::new(ASM_BAD_OPERAND, range).with_text(line)
}

/// The tokens of one operand, being read.
struct Reader<'a> {
    /// The line the token ranges point into.
    line: &'a str,
    /// The tokens of the operand (its extent only).
    tokens: &'a [Token],
    /// The range of the whole operand.
    whole: TextRange,
}

impl Reader<'_> {
    /// Returns the token at `at`, if any.
    fn get(&self, at: usize) -> Option<&Token> {
        self.tokens.get(at)
    }

    /// Tells whether the token at `at` is the punctuation char `wanted`.
    fn punct(&self, at: usize, wanted: char) -> bool {
        self.get(at)
            .is_some_and(|token| is_punct(self.line, token, wanted))
    }

    /// Tells whether the token at `at` is the operator `wanted`.
    fn operator(&self, at: usize, wanted: &str) -> bool {
        self.get(at).is_some_and(|token| {
            token.kind == TokenKind::Operator && token_text(self.line, token) == wanted
        })
    }

    /// Returns the register at `at`, if the token there is one.
    fn register(&self, at: usize) -> Option<Reg> {
        let token = self.get(at)?;
        if token.kind != TokenKind::Register {
            return None;
        }
        register_named(token_text(self.line, token))
    }

    /// Returns the size letter of the `SizeSuffix` token at `at`, if the token there is one.
    fn suffix(&self, at: usize) -> Option<Size> {
        let token = self.get(at)?;
        if token.kind != TokenKind::SizeSuffix {
            return None;
        }
        Size::from_letter(token_text(self.line, token).get(1..)?)
    }

    /// Returns `asm-bad-operand` over the whole operand.
    fn bad(&self) -> Problem {
        bad_operand(self.line, self.whole)
    }

    /// Fails with `asm-register-size-suffix` when the register at `at` is followed by a size.
    fn no_size_after(&self, at: usize) -> Result<(), Problem> {
        match (self.get(at), self.get(at + 1)) {
            (Some(register), Some(size)) if size.kind == TokenKind::SizeSuffix => {
                let range = TextRange::new(register.range.start, size.range.end);
                Err(Problem::new(ASM_REGISTER_SIZE_SUFFIX, range).with_text(self.line))
            }
            _ => Ok(()),
        }
    }

    /// Reads the operand; returns it and the number of tokens it used.
    fn read(&self) -> Result<(Operand, usize), Problem> {
        let first = self.get(0).ok_or_else(|| self.bad())?;
        match first.kind {
            TokenKind::Register => self.register_operand(),
            TokenKind::Punctuation if self.punct(0, '#') => {
                let rest = self.tokens.get(1..).unwrap_or(&[]);
                if rest.is_empty() {
                    return Err(self.bad());
                }
                let (expr, used) = self.expression(rest)?;
                Ok((Operand::Ea(Ea::Imm(expr)), used + 1))
            }
            TokenKind::Operator
                if self.operator(0, "-") && self.punct(1, '(') && self.register(2).is_some() =>
            {
                self.no_size_after(2)?;
                match self.register(2) {
                    Some(Reg::Address(n)) if self.punct(3, ')') => {
                        Ok((Operand::Ea(Ea::PreDec(n)), 4))
                    }
                    _ => Err(self.bad()),
                }
            }
            TokenKind::Punctuation if self.punct(0, '(') && self.register(1).is_some() => {
                self.register_in_parentheses()
            }
            TokenKind::String if self.tokens.len() == 1 => {
                let bytes = string_bytes(token_text(self.line, first)).ok_or_else(|| self.bad())?;
                Ok((Operand::Bytes(bytes), 1))
            }
            _ => self.expression_operand(),
        }
    }

    /// Reads an operand that starts with a register: `Dn`, `An`, a register list, `USP`, `CCR`
    /// or `SR`.
    fn register_operand(&self) -> Result<(Operand, usize), Problem> {
        self.no_size_after(0)?;
        let register = self.register(0).ok_or_else(|| self.bad())?;
        if register.list_bit().is_some() && (self.operator(1, "-") || self.operator(1, "/")) {
            return self.register_list();
        }
        let operand = match register {
            Reg::Data(n) => Operand::Ea(Ea::Dn(n)),
            Reg::Address(n) => Operand::Ea(Ea::An(n)),
            Reg::Usp => Operand::Usp,
            Reg::Ccr => Operand::Ccr,
            Reg::Sr => Operand::Sr,
            Reg::Pc => return Err(self.bad()),
        };
        Ok((operand, 1))
    }

    /// Returns the list bit of the data or address register at `at`.
    fn list_register(&self, at: usize) -> Result<u8, Problem> {
        self.no_size_after(at)?;
        self.register(at)
            .and_then(Reg::list_bit)
            .ok_or_else(|| self.bad())
    }

    /// Reads a register list `r[-r]/…`.
    fn register_list(&self) -> Result<(Operand, usize), Problem> {
        let mut mask: u16 = 0;
        let mut at = 0;
        loop {
            let low = self.list_register(at)?;
            at += 1;
            let mut high = low;
            if self.operator(at, "-") {
                high = self.list_register(at + 1)?;
                at += 2;
                if high < low {
                    return Err(self.bad());
                }
            }
            for bit in low..=high {
                mask |= 1 << bit;
            }
            if !self.operator(at, "/") {
                return Ok((Operand::RegList(mask), at));
            }
            at += 1;
        }
    }

    /// Reads the index register `Xn[.W|.L]` at `at`; returns it and the index after it.
    fn index(&self, at: usize) -> Result<(Xn, usize), Problem> {
        let reg = match self.register(at) {
            Some(Reg::Data(n)) => n,
            Some(Reg::Address(n)) => 8 + n,
            _ => return Err(self.bad()),
        };
        match self.get(at + 1) {
            Some(token) if token.kind == TokenKind::SizeSuffix => match self.suffix(at + 1) {
                Some(Size::Word) => Ok((Xn { reg, long: false }, at + 2)),
                Some(Size::Long) => Ok((Xn { reg, long: true }, at + 2)),
                _ => Err(self.bad()),
            },
            _ => Ok((Xn { reg, long: false }, at + 1)),
        }
    }

    /// Reads `(An)`, `(An)+`, or the rejected `(An,Xn)` and `(PC,Xn)` (missing displacement,
    /// with the fix that writes `0(`).
    fn register_in_parentheses(&self) -> Result<(Operand, usize), Problem> {
        self.no_size_after(1)?;
        let register = self.register(1).ok_or_else(|| self.bad())?;
        match register {
            Reg::Address(n) if self.punct(2, ')') => {
                if self.operator(3, "+") {
                    Ok((Operand::Ea(Ea::PostInc(n)), 4))
                } else {
                    Ok((Operand::Ea(Ea::Ind(n)), 3))
                }
            }
            Reg::Address(_) | Reg::Pc if self.punct(2, ',') => {
                let (_, next) = self.index(3)?;
                if !self.punct(next, ')') || next + 1 != self.tokens.len() {
                    return Err(self.bad());
                }
                let open = self.get(0).map_or(self.whole, |token| token.range);
                Err(Problem::new(ASM_MISSING_DISPLACEMENT, self.whole)
                    .with_fix(open, "0(".to_owned()))
            }
            _ => Err(self.bad()),
        }
    }

    /// Parses the expression at the start of `tokens`; a failure in an operand that holds a
    /// register is `asm-bad-operand`.
    fn expression(&self, tokens: &[Token]) -> Result<(Expr, usize), Problem> {
        parse_expr(self.line, tokens).map_err(|problem| {
            let holds_register = self
                .tokens
                .iter()
                .any(|token| token.kind == TokenKind::Register);
            if problem.code == ASM_BAD_EXPRESSION && holds_register {
                self.bad()
            } else {
                problem
            }
        })
    }

    /// Tells whether `tokens[..end]` is one parenthesized group: its first `(` closes at its
    /// last token.
    fn is_one_group(&self, end: usize) -> bool {
        if end < 2 || !self.punct(0, '(') {
            return false;
        }
        let mut depth: u32 = 0;
        for at in 0..end {
            if self.punct(at, '(') {
                depth += 1;
            } else if self.punct(at, ')') {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return at + 1 == end;
                }
            }
        }
        false
    }

    /// Reads an operand that starts with an expression: `e`, `(e).W`, `(e).L`, `e(An)`,
    /// `e(An,Xn)`, `e(PC)` or `e(PC,Xn)`.
    fn expression_operand(&self) -> Result<(Operand, usize), Problem> {
        let (expr, used) = self.expression(self.tokens)?;
        if used == self.tokens.len() {
            return Ok((Operand::Ea(Ea::Abs(expr, None)), used));
        }
        if let Some(size) = self.suffix(used) {
            if used + 1 != self.tokens.len() {
                return Err(self.bad());
            }
            if self.is_one_group(used) {
                return match size {
                    Size::Word | Size::Long => {
                        Ok((Operand::Ea(Ea::Abs(expr, Some(size))), used + 1))
                    }
                    Size::Byte | Size::Short => Err(self.bad()),
                };
            }
            if matches!(size, Size::Byte | Size::Short) {
                return Err(self.bad());
            }
            let written = span(self.tokens.get(..used).unwrap_or(&[]), self.whole.start);
            return Err(Problem::new(ASM_ABSOLUTE_SIZE_SYNTAX, self.whole)
                .with_text(self.line)
                .with_fix(written, format!("({})", slice(self.line, written))));
        }
        if !self.punct(used, '(') {
            return Err(self.bad());
        }
        let base = used + 1;
        self.no_size_after(base)?;
        let (ea, end) = match self.register(base) {
            Some(Reg::Address(n)) if self.punct(base + 1, ')') => (Ea::Disp(expr, n), base + 2),
            Some(Reg::Pc) if self.punct(base + 1, ')') => (Ea::PcDisp(expr), base + 2),
            Some(register @ (Reg::Address(_) | Reg::Pc)) if self.punct(base + 1, ',') => {
                let (xn, next) = self.index(base + 2)?;
                if !self.punct(next, ')') {
                    return Err(self.bad());
                }
                match register {
                    Reg::Address(n) => (Ea::Index(expr, n, xn), next + 1),
                    _ => (Ea::PcIndex(expr, xn), next + 1),
                }
            }
            _ => return Err(self.bad()),
        };
        Ok((Operand::Ea(ea), end))
    }
}

/// Parses one operand at the start of `tokens` (line-relative ranges into `line`); returns it
/// with the number of tokens it used.
///
/// The operand runs to the first `,` outside parentheses or the end of the operand field
/// (whitespace, a comment or a suspect comment), and must use all of it; the caller checks
/// the `,` that may follow. The range of the result covers the tokens used.
///
/// # Errors
///
/// The [`Problem`] of an operand that does not parse (with its quick fix when §4.3 gives one):
/// `asm-bad-operand`, `asm-bad-expression`, `asm-missing-displacement`,
/// `asm-absolute-size-syntax`, `asm-register-size-suffix`, `asm-string-too-long`,
/// `asm-macro-param-outside`, or, for an `Invalid` token, the lexer's own diagnostic repeated
/// (its `lex-*` code; see `parser`).
pub fn parse_operand(line: &str, tokens: &[Token]) -> Result<(Spanned<Operand>, usize), Problem> {
    let extent = operand_extent(line, tokens);
    let operand = tokens.get(..extent).unwrap_or(&[]);
    let whole = span(operand, start_of(line, tokens));
    if operand.is_empty() {
        return Err(bad_operand(line, whole));
    }
    if let Some(problem) = token_problem(line, operand) {
        return Err(problem);
    }
    let reader = Reader {
        line,
        tokens: operand,
        whole,
    };
    let (item, used) = reader.read()?;
    if used != extent {
        return Err(reader.bad());
    }
    Ok((Spanned { item, range: whole }, used))
}
