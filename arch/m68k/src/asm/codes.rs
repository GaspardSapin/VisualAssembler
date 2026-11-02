//! Diagnostic codes of the assembler (m2-assembler.md §7.2): one constant per code, named after
//! it (`asm-bad-operand` is [`ASM_BAD_OPERAND`]), and [`ASM_DIAG_CODES`], the list that
//! `Architecture::diagnostic_codes` of the 68000 returns after the lexer codes (§7.1).
//!
//! Each code is documented as the Fluent message `diag-<code>` in
//! `arch/m68k/locales/{en,fr}/diagnostics.ftl`, in English and French. The `asm-related-*` codes
//! name the related notes of a diagnostic (`va_core::diag::Related`), not diagnostics of their
//! own; their messages have no attribute.
//!
//! **Arguments.** The documentation of each constant lists the Fluent variables its message
//! takes, which the producer passes in `Diagnostic::args` (or `Related::args`), each exactly once:
//! - `Str` is [`DiagArg::Str`](va_core::diag::DiagArg::Str) and `Int` is
//!   [`DiagArg::Int`](va_core::diag::DiagArg::Int). Values and addresses are formatted text in
//!   the 68000 notation (`$1000`, `-129`, m1 §3.3); counts and sizes in bytes are `Int`, which
//!   Fluent uses for plural selection.
//! - `text` is an excerpt of the source, truncated to 32 characters plus `…` (m1 §3.4).
//! - An argument documented as a *selector* chooses a variant of the text and must be one of the
//!   listed ids, spelled exactly so (Fluent compares them byte for byte); any other value gets the
//!   last variant listed.
//! - Directive and size names are uppercase, without the dot: `ENDC`, `W`.
//!
//! `tests/locales.rs` formats every message in both languages with these arguments and checks
//! that each one is used. Severity and blocking are decided by the code that emits a diagnostic
//! (WP-D, §7.2), not here.
//!
//! WP-F owns this file in wave 2 (§15.2 W1): a new code needs its constant, its entry in
//! [`ASM_DIAG_CODES`] and its two messages in one change.

use va_core::diag::DiagCode;

/// An operation that is neither an instruction, a directive nor a defined macro.
///
/// Arguments: `name` (Str, the operation as written), `count` (Int, the number of suggestions,
/// 0 to 3), `suggestions` (Str, the suggested spellings or macro names joined by `", "`, empty
/// when `count` is 0).
pub const ASM_UNKNOWN_OPERATION: DiagCode = DiagCode("asm-unknown-operation");

/// `ELSE`, `REPT` or `OPT`, which a68k 2.71 does not know, without a macro of that name.
///
/// Arguments: `name` (Str, selector: `ELSE`, `REPT` or `OPT`, uppercase whatever the source
/// case; it is also printed).
pub const ASM_NOT_A68K_DIRECTIVE: DiagCode = DiagCode("asm-not-a68k-directive");

/// `EQUR`, `REG`, `RORG`, `NEAR` or `FAR`: a68k directives that VisualAssembler refuses.
///
/// Arguments: `name` (Str, selector: `EQUR`, `REG`, `RORG`, `NEAR` or `FAR`, uppercase; it is
/// also printed).
pub const ASM_UNSUPPORTED_DIRECTIVE: DiagCode = DiagCode("asm-unsupported-directive");

/// A section or linkage directive, which a68k refuses when it writes S-records (§13.3 Q1).
///
/// Arguments: `name` (Str, the directive, uppercase: `SECTION`, `CODE`, `DATA`, `BSS`, `CSEG`,
/// `DSEG`, `XDEF`, `XREF` or `PUBLIC`).
pub const ASM_NOT_IN_S_FORMAT: DiagCode = DiagCode("asm-not-in-s-format");

/// An operand that no operand syntax reads.
///
/// Arguments: `text` (Str, the operand).
pub const ASM_BAD_OPERAND: DiagCode = DiagCode("asm-bad-operand");

/// An expression that does not parse.
///
/// Arguments: `text` (Str, the expression).
pub const ASM_BAD_EXPRESSION: DiagCode = DiagCode("asm-bad-expression");

/// Too few or too many operands for the instruction.
///
/// Arguments: `mnemonic` (Str, the spelling as written, size suffix excluded), `expected` (Int,
/// the operand count of the closest form; 0 and 1 have their own wording), `found` (Int).
pub const ASM_OPERAND_COUNT: DiagCode = DiagCode("asm-operand-count");

/// An addressing mode the selected operand position does not accept.
///
/// Arguments: `mnemonic` (Str), `mode` (Str, the mode of the operand as `Mode::notation`
/// spells it, such as `An` or `#imm`), `allowed` (Str, the accepted modes in that notation,
/// joined by `", "`).
pub const ASM_MODE_NOT_ALLOWED: DiagCode = DiagCode("asm-mode-not-allowed");

/// A size suffix no form of the instruction accepts.
///
/// Arguments: `mnemonic` (Str), `size` (Str, the refused size: `B`, `W`, `L` or `S`), `allowed`
/// (Str, the accepted sizes, such as `.B, .W, .L`).
pub const ASM_SIZE_NOT_ALLOWED: DiagCode = DiagCode("asm-size-not-allowed");

/// Operands that fit no form of the instruction.
///
/// Arguments: `mnemonic` (Str), `forms` (Str, the operand lists of the forms joined by `"; "`,
/// such as `<ea>,Dn; Dn,<ea>`).
pub const ASM_NO_MATCHING_FORM: DiagCode = DiagCode("asm-no-matching-form");

/// A size after a register outside the index of `d8(An,Xn)`.
///
/// Arguments: `text` (Str, the register with its size, such as `d0.w`).
pub const ASM_REGISTER_SIZE_SUFFIX: DiagCode = DiagCode("asm-register-size-suffix");

/// `(An,Xn)` without the displacement a68k requires (quick fix `0(`).
///
/// Arguments: none.
pub const ASM_MISSING_DISPLACEMENT: DiagCode = DiagCode("asm-missing-displacement");

/// `$400.w`, which a68k does not read as a sized address (quick fix `($400).w`).
///
/// Arguments: `text` (Str, the operand as written).
pub const ASM_ABSOLUTE_SIZE_SYNTAX: DiagCode = DiagCode("asm-absolute-size-syntax");

/// A value outside the range of its field.
///
/// Arguments: `value`, `min`, `max` (Str each, formatted numbers such as `300`, `-128`, `255`).
pub const ASM_VALUE_OUT_OF_RANGE: DiagCode = DiagCode("asm-value-out-of-range");

/// A branch of explicit size whose target is too far.
///
/// Arguments: `distance` (Int, how far the target is, in bytes, as a positive number), `size`
/// (Str, the size written: `S`, `B`, `W` or `L`).
pub const ASM_BRANCH_OUT_OF_RANGE: DiagCode = DiagCode("asm-branch-out-of-range");

/// An explicit short branch to the next instruction (displacement 0).
///
/// Arguments: none.
pub const ASM_SHORT_BRANCH_ZERO: DiagCode = DiagCode("asm-short-branch-zero");

/// A branch to an odd address (warning).
///
/// Arguments: `target` (Str, the formatted target address, such as `$001001`).
pub const ASM_ODD_BRANCH_TARGET: DiagCode = DiagCode("asm-odd-branch-target");

/// A symbol that is never defined.
///
/// Arguments: `name` (Str), `count` (Int, the number of suggestions, 0 to 3), `suggestions`
/// (Str, the suggested symbols joined by `", "`, empty when `count` is 0).
pub const ASM_UNDEFINED_SYMBOL: DiagCode = DiagCode("asm-undefined-symbol");

/// A local label that its block does not define.
///
/// Arguments: `name` (Str, such as `1$` or `\loop`).
pub const ASM_UNDEFINED_LOCAL_LABEL: DiagCode = DiagCode("asm-undefined-local-label");

/// A second definition of a symbol or macro.
///
/// Arguments: `name` (Str).
pub const ASM_DUPLICATE_SYMBOL: DiagCode = DiagCode("asm-duplicate-symbol");

/// A macro named like an instruction or a directive.
///
/// Arguments: `name` (Str, the macro name as written).
pub const ASM_RESERVED_NAME: DiagCode = DiagCode("asm-reserved-name");

/// A column-1 label spelled like an instruction or directive, alone on its line (warning).
///
/// Arguments: `name` (Str, the label as written).
pub const ASM_LABEL_LIKE_KEYWORD: DiagCode = DiagCode("asm-label-like-keyword");

/// `EQU`, `SET`, `=` or `MACRO` without a label.
///
/// Arguments: `name` (Str, the directive, uppercase: `EQU`, `SET`, `=` or `MACRO`).
pub const ASM_LABEL_REQUIRED: DiagCode = DiagCode("asm-label-required");

/// A directive that needs a value known at its line, given a symbol defined further down.
///
/// Arguments: `name` (Str, the directive, uppercase, such as `ORG` or `IFEQ`), `symbol` (Str,
/// the symbol defined further down).
pub const ASM_VALUE_NOT_KNOWN: DiagCode = DiagCode("asm-value-not-known");

/// A division by zero in an expression.
///
/// Arguments: none.
pub const ASM_DIVISION_BY_ZERO: DiagCode = DiagCode("asm-division-by-zero");

/// A negative `DS` or `DCB` count.
///
/// Arguments: `name` (Str, `DS` or `DCB`), `count` (Int, the negative count).
pub const ASM_NEGATIVE_COUNT: DiagCode = DiagCode("asm-negative-count");

/// Bytes past the end of the 68000 address space (`$FFFFFF`).
///
/// Arguments: `address` (Str, the formatted last address, `$FFFFFF`).
pub const ASM_ADDRESS_OVERFLOW: DiagCode = DiagCode("asm-address-overflow");

/// Bytes placed over bytes already placed (an `ORG` back).
///
/// Arguments: `address` (Str, the formatted first overlapping address).
pub const ASM_OVERLAP: DiagCode = DiagCode("asm-overlap");

/// A quoted string combined with an operator.
///
/// Arguments: `text` (Str, the expression).
pub const ASM_STRING_IN_EXPRESSION: DiagCode = DiagCode("asm-string-in-expression");

/// A string longer than a `.W` or `.L` value holds.
///
/// Arguments: `size` (Str, `W` or `L`), `max` (Int, 2 or 4), `length` (Int, the characters of
/// the string).
pub const ASM_STRING_TOO_LONG: DiagCode = DiagCode("asm-string-too-long");

/// Text after `END`, which is ignored (warning).
///
/// Arguments: none.
pub const ASM_TEXT_AFTER_END: DiagCode = DiagCode("asm-text-after-end");

/// An `ENDC` without `IF`, or an `IF` block never closed.
///
/// Arguments: `name` (Str, the directive, uppercase: `ENDC` or `ENDIF` for `close`, the `IFxx`
/// that opened the block for `open`), `kind` (Str, selector: `close` or `open`).
pub const ASM_UNBALANCED_CONDITIONAL: DiagCode = DiagCode("asm-unbalanced-conditional");

/// An `ENDM` or `MEXIT` outside a macro, or a `MACRO` without `ENDM`.
///
/// Arguments: `name` (Str: `ENDM` or `MEXIT` for `close`, the macro name for `open`), `kind`
/// (Str, selector: `close` or `open`).
pub const ASM_UNBALANCED_MACRO: DiagCode = DiagCode("asm-unbalanced-macro");

/// A macro parameter (`\1`, `\@`, …) outside a macro body.
///
/// Arguments: `text` (Str, the parameter, such as `\1`).
pub const ASM_MACRO_PARAM_OUTSIDE: DiagCode = DiagCode("asm-macro-param-outside");

/// Macro expansions nested deeper than `AsmLimits::max_macro_depth`.
///
/// Arguments: `limit` (Int, `max_macro_depth`).
pub const ASM_MACRO_TOO_DEEP: DiagCode = DiagCode("asm-macro-too-deep");

/// More statements than `AsmLimits::max_statements` after expansion; assembly stops.
///
/// Arguments: `limit` (Int, `max_statements`).
pub const ASM_EXPANSION_LIMIT: DiagCode = DiagCode("asm-expansion-limit");

/// INCLUDE files nested deeper than `AsmLimits::max_include_depth`.
///
/// Arguments: `limit` (Int, `max_include_depth`).
pub const ASM_INCLUDE_TOO_DEEP: DiagCode = DiagCode("asm-include-too-deep");

/// A file that includes itself, directly or not.
///
/// Arguments: `path` (Str, the path as written in the INCLUDE line).
pub const ASM_INCLUDE_CYCLE: DiagCode = DiagCode("asm-include-cycle");

/// An INCLUDE or INCBIN path that names no file.
///
/// Arguments: `path` (Str, the path as written).
pub const ASM_INCLUDE_NOT_FOUND: DiagCode = DiagCode("asm-include-not-found");

/// An INCLUDE or INCBIN path refused by the path policy (m2 §6.2).
///
/// Arguments: `path` (Str, the path as written), `reason` (Str, selector: `PathRefusal::id`,
/// that is `absolute`, `drive`, `colon`, `device`, `escapes`, `empty` or `nul`; the last two
/// share one wording).
pub const ASM_INCLUDE_REFUSED: DiagCode = DiagCode("asm-include-refused");

/// An INCLUDE or INCBIN file that cannot be used (too large, not UTF-8, unreadable, …).
///
/// Arguments: `path` (Str, the path as written), `reason` (Str, selector: `too-large`,
/// `not-utf8`, `too-many-files`, `unsupported` or `unreadable`, §15.1 A5).
pub const ASM_INCLUDE_FAILED: DiagCode = DiagCode("asm-include-failed");

/// The note that ends the diagnostics after `AsmLimits::max_diagnostics`.
///
/// Arguments: `limit` (Int, `max_diagnostics`).
pub const ASM_TOO_MANY_DIAGNOSTICS: DiagCode = DiagCode("asm-too-many-diagnostics");

/// Operand-like text in the comment of an instruction or `DC`/`DCB` line (non-blocking error);
/// replaces the lexer's `lex-operands-in-comment` on those lines (§4.3).
///
/// Arguments: `text` (Str, the ignored text, as for the lexer code), `kind` (Str, selector:
/// `data` for `DC`/`DCB`, `instruction` otherwise), `bytes` (Int, the bytes the line produces;
/// printed for `data`), `kept` (Str, the operand field a68k uses; printed for `instruction`),
/// `cause` (Str, selector of the `.help`, as for the lexer code: `after-comma`,
/// `inside-expression` or `operand-list`). Every argument is passed whatever `kind` is.
pub const ASM_OPERANDS_IN_COMMENT: DiagCode = DiagCode("asm-operands-in-comment");

/// A one-register MOVEM assembled as MOVE, whose effect differs (warning, §13.3 Q2).
///
/// Arguments: `size` (Str, `W` or `L`), `effect` (Str, selector: `sign` for a `.W` move from
/// memory into a data register, which loses the sign extension, `flags` otherwise).
pub const ASM_MOVEM_SINGLE: DiagCode = DiagCode("asm-movem-single");

/// A pass-1 decision that pass 2 contradicts (a safety net).
///
/// Arguments: `name` (Str, the symbol whose value changed).
pub const ASM_PHASE_ERROR: DiagCode = DiagCode("asm-phase-error");

/// Related note: the first definition of a symbol defined twice.
///
/// Arguments: `name` (Str).
pub const ASM_RELATED_FIRST_DEFINITION: DiagCode = DiagCode("asm-related-first-definition");

/// Related note: the start of the block of a local label.
///
/// Arguments: none.
pub const ASM_RELATED_SCOPE_START: DiagCode = DiagCode("asm-related-scope-start");

/// Related note: the INCLUDE statement of an included file, one per include level.
///
/// Arguments: none.
pub const ASM_RELATED_INCLUDED_FROM: DiagCode = DiagCode("asm-related-included-from");

/// Related note: the body line of a macro whose expansion has the problem.
///
/// Arguments: `name` (Str, the macro name).
pub const ASM_RELATED_IN_MACRO: DiagCode = DiagCode("asm-related-in-macro");

/// Related note: where the bytes of an overlap were placed first.
///
/// Arguments: none.
pub const ASM_RELATED_OTHER_BYTES: DiagCode = DiagCode("asm-related-other-bytes");

/// Every code the assembler lists through `Architecture::diagnostic_codes`, after the lexer
/// codes (§7.1): the 45 diagnostic codes in the order of §7.2, then the 5 `asm-related-*` note
/// codes, whose messages the locale tests check too.
pub static ASM_DIAG_CODES: [DiagCode; 50] = [
    ASM_UNKNOWN_OPERATION,
    ASM_NOT_A68K_DIRECTIVE,
    ASM_UNSUPPORTED_DIRECTIVE,
    ASM_NOT_IN_S_FORMAT,
    ASM_BAD_OPERAND,
    ASM_BAD_EXPRESSION,
    ASM_OPERAND_COUNT,
    ASM_MODE_NOT_ALLOWED,
    ASM_SIZE_NOT_ALLOWED,
    ASM_NO_MATCHING_FORM,
    ASM_REGISTER_SIZE_SUFFIX,
    ASM_MISSING_DISPLACEMENT,
    ASM_ABSOLUTE_SIZE_SYNTAX,
    ASM_VALUE_OUT_OF_RANGE,
    ASM_BRANCH_OUT_OF_RANGE,
    ASM_SHORT_BRANCH_ZERO,
    ASM_ODD_BRANCH_TARGET,
    ASM_UNDEFINED_SYMBOL,
    ASM_UNDEFINED_LOCAL_LABEL,
    ASM_DUPLICATE_SYMBOL,
    ASM_RESERVED_NAME,
    ASM_LABEL_LIKE_KEYWORD,
    ASM_LABEL_REQUIRED,
    ASM_VALUE_NOT_KNOWN,
    ASM_DIVISION_BY_ZERO,
    ASM_NEGATIVE_COUNT,
    ASM_ADDRESS_OVERFLOW,
    ASM_OVERLAP,
    ASM_STRING_IN_EXPRESSION,
    ASM_STRING_TOO_LONG,
    ASM_TEXT_AFTER_END,
    ASM_UNBALANCED_CONDITIONAL,
    ASM_UNBALANCED_MACRO,
    ASM_MACRO_PARAM_OUTSIDE,
    ASM_MACRO_TOO_DEEP,
    ASM_EXPANSION_LIMIT,
    ASM_INCLUDE_TOO_DEEP,
    ASM_INCLUDE_CYCLE,
    ASM_INCLUDE_NOT_FOUND,
    ASM_INCLUDE_REFUSED,
    ASM_INCLUDE_FAILED,
    ASM_TOO_MANY_DIAGNOSTICS,
    ASM_OPERANDS_IN_COMMENT,
    ASM_MOVEM_SINGLE,
    ASM_PHASE_ERROR,
    ASM_RELATED_FIRST_DEFINITION,
    ASM_RELATED_SCOPE_START,
    ASM_RELATED_INCLUDED_FROM,
    ASM_RELATED_IN_MACRO,
    ASM_RELATED_OTHER_BYTES,
];
