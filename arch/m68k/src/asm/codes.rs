//! Diagnostic codes of the assembler (m2-assembler.md §7.2): one constant per code, named after
//! it (`asm-bad-operand` is [`ASM_BAD_OPERAND`]), and [`ASM_DIAG_CODES`], the list that
//! `Architecture::diagnostic_codes` of the 68000 returns after the lexer codes (§7.1).
//!
//! Each code is documented as the Fluent message `diag-<code>` in
//! `arch/m68k/locales/{en,fr}/diagnostics.ftl` (WP-F). The `asm-related-*` codes name the
//! related notes of a diagnostic (`va_core::diag::Related`), not diagnostics of their own.
//!
//! WP0 scaffold: the constants exist from wave 1b so that the front end (WP-B), the encoder
//! (WP-C) and the driver (WP-D) share them. [`ASM_DIAG_CODES`] stays empty until WP-D fills it,
//! together with WP-F's Fluent messages: the M1 tests (`tests/arch.rs`, `tests/locales.rs`,
//! `apps/va/tests/lex.rs`) check every listed code against the `.ftl` files and the lexer
//! fixtures.

use va_core::diag::DiagCode;

/// An operation that is neither an instruction, a directive nor a defined macro.
pub const ASM_UNKNOWN_OPERATION: DiagCode = DiagCode("asm-unknown-operation");

/// `ELSE`, `REPT` or `OPT`, which a68k 2.71 does not know, without a macro of that name.
pub const ASM_NOT_A68K_DIRECTIVE: DiagCode = DiagCode("asm-not-a68k-directive");

/// `EQUR`, `REG`, `RORG`, `NEAR` or `FAR`: a68k directives that VisualAssembler refuses.
pub const ASM_UNSUPPORTED_DIRECTIVE: DiagCode = DiagCode("asm-unsupported-directive");

/// A section or linkage directive, which a68k refuses when it writes S-records (§13.3 Q1).
pub const ASM_NOT_IN_S_FORMAT: DiagCode = DiagCode("asm-not-in-s-format");

/// An operand that no operand syntax reads.
pub const ASM_BAD_OPERAND: DiagCode = DiagCode("asm-bad-operand");

/// An expression that does not parse.
pub const ASM_BAD_EXPRESSION: DiagCode = DiagCode("asm-bad-expression");

/// Too few or too many operands for the instruction.
pub const ASM_OPERAND_COUNT: DiagCode = DiagCode("asm-operand-count");

/// An addressing mode the selected operand position does not accept.
pub const ASM_MODE_NOT_ALLOWED: DiagCode = DiagCode("asm-mode-not-allowed");

/// A size suffix no form of the instruction accepts.
pub const ASM_SIZE_NOT_ALLOWED: DiagCode = DiagCode("asm-size-not-allowed");

/// Operands that fit no form of the instruction.
pub const ASM_NO_MATCHING_FORM: DiagCode = DiagCode("asm-no-matching-form");

/// A size after a register outside the index of `d8(An,Xn)`.
pub const ASM_REGISTER_SIZE_SUFFIX: DiagCode = DiagCode("asm-register-size-suffix");

/// `(An,Xn)` without the displacement a68k requires (quick fix `0(`).
pub const ASM_MISSING_DISPLACEMENT: DiagCode = DiagCode("asm-missing-displacement");

/// `$400.w`, which a68k does not read as a sized address (quick fix `($400).w`).
pub const ASM_ABSOLUTE_SIZE_SYNTAX: DiagCode = DiagCode("asm-absolute-size-syntax");

/// A value outside the range of its field.
pub const ASM_VALUE_OUT_OF_RANGE: DiagCode = DiagCode("asm-value-out-of-range");

/// A branch of explicit size whose target is too far.
pub const ASM_BRANCH_OUT_OF_RANGE: DiagCode = DiagCode("asm-branch-out-of-range");

/// An explicit short branch to the next instruction (displacement 0).
pub const ASM_SHORT_BRANCH_ZERO: DiagCode = DiagCode("asm-short-branch-zero");

/// A branch to an odd address (warning).
pub const ASM_ODD_BRANCH_TARGET: DiagCode = DiagCode("asm-odd-branch-target");

/// A symbol that is never defined.
pub const ASM_UNDEFINED_SYMBOL: DiagCode = DiagCode("asm-undefined-symbol");

/// A local label that its block does not define.
pub const ASM_UNDEFINED_LOCAL_LABEL: DiagCode = DiagCode("asm-undefined-local-label");

/// A second definition of a symbol or macro.
pub const ASM_DUPLICATE_SYMBOL: DiagCode = DiagCode("asm-duplicate-symbol");

/// A macro named like an instruction or a directive.
pub const ASM_RESERVED_NAME: DiagCode = DiagCode("asm-reserved-name");

/// A column-1 label spelled like an instruction or directive, alone on its line (warning).
pub const ASM_LABEL_LIKE_KEYWORD: DiagCode = DiagCode("asm-label-like-keyword");

/// `EQU`, `SET`, `=` or `MACRO` without a label.
pub const ASM_LABEL_REQUIRED: DiagCode = DiagCode("asm-label-required");

/// A directive that needs a value known at its line, given a symbol defined further down.
pub const ASM_VALUE_NOT_KNOWN: DiagCode = DiagCode("asm-value-not-known");

/// A division by zero in an expression.
pub const ASM_DIVISION_BY_ZERO: DiagCode = DiagCode("asm-division-by-zero");

/// A negative `DS` or `DCB` count.
pub const ASM_NEGATIVE_COUNT: DiagCode = DiagCode("asm-negative-count");

/// Bytes past the end of the 68000 address space (`$FFFFFF`).
pub const ASM_ADDRESS_OVERFLOW: DiagCode = DiagCode("asm-address-overflow");

/// Bytes placed over bytes already placed (an `ORG` back).
pub const ASM_OVERLAP: DiagCode = DiagCode("asm-overlap");

/// A quoted string combined with an operator.
pub const ASM_STRING_IN_EXPRESSION: DiagCode = DiagCode("asm-string-in-expression");

/// A string longer than a `.W` or `.L` value holds.
pub const ASM_STRING_TOO_LONG: DiagCode = DiagCode("asm-string-too-long");

/// Text after `END`, which is ignored (warning).
pub const ASM_TEXT_AFTER_END: DiagCode = DiagCode("asm-text-after-end");

/// An `ENDC` without `IF`, or an `IF` block never closed.
pub const ASM_UNBALANCED_CONDITIONAL: DiagCode = DiagCode("asm-unbalanced-conditional");

/// An `ENDM` or `MEXIT` outside a macro, or a `MACRO` without `ENDM`.
pub const ASM_UNBALANCED_MACRO: DiagCode = DiagCode("asm-unbalanced-macro");

/// A macro parameter (`\1`, `\@`, …) outside a macro body.
pub const ASM_MACRO_PARAM_OUTSIDE: DiagCode = DiagCode("asm-macro-param-outside");

/// Macro expansions nested deeper than `AsmLimits::max_macro_depth`.
pub const ASM_MACRO_TOO_DEEP: DiagCode = DiagCode("asm-macro-too-deep");

/// More statements than `AsmLimits::max_statements` after expansion; assembly stops.
pub const ASM_EXPANSION_LIMIT: DiagCode = DiagCode("asm-expansion-limit");

/// INCLUDE files nested deeper than `AsmLimits::max_include_depth`.
pub const ASM_INCLUDE_TOO_DEEP: DiagCode = DiagCode("asm-include-too-deep");

/// A file that includes itself, directly or not.
pub const ASM_INCLUDE_CYCLE: DiagCode = DiagCode("asm-include-cycle");

/// An INCLUDE or INCBIN path that names no file.
pub const ASM_INCLUDE_NOT_FOUND: DiagCode = DiagCode("asm-include-not-found");

/// An INCLUDE or INCBIN path refused by the path policy (m2 §6.2).
pub const ASM_INCLUDE_REFUSED: DiagCode = DiagCode("asm-include-refused");

/// An INCLUDE or INCBIN file that cannot be used (too large, not UTF-8, unreadable, …).
pub const ASM_INCLUDE_FAILED: DiagCode = DiagCode("asm-include-failed");

/// The note that ends the diagnostics after `AsmLimits::max_diagnostics`.
pub const ASM_TOO_MANY_DIAGNOSTICS: DiagCode = DiagCode("asm-too-many-diagnostics");

/// Operand-like text in the comment of an instruction or `DC`/`DCB` line (non-blocking error);
/// replaces the lexer's `lex-operands-in-comment` on those lines (§4.3).
pub const ASM_OPERANDS_IN_COMMENT: DiagCode = DiagCode("asm-operands-in-comment");

/// A one-register MOVEM assembled as MOVE, whose effect differs (warning, §13.3 Q2).
pub const ASM_MOVEM_SINGLE: DiagCode = DiagCode("asm-movem-single");

/// A pass-1 decision that pass 2 contradicts (a safety net).
pub const ASM_PHASE_ERROR: DiagCode = DiagCode("asm-phase-error");

/// Related note: the first definition of a symbol defined twice.
pub const ASM_RELATED_FIRST_DEFINITION: DiagCode = DiagCode("asm-related-first-definition");

/// Related note: the start of the block of a local label.
pub const ASM_RELATED_SCOPE_START: DiagCode = DiagCode("asm-related-scope-start");

/// Related note: the INCLUDE statement of an included file, one per include level.
pub const ASM_RELATED_INCLUDED_FROM: DiagCode = DiagCode("asm-related-included-from");

/// Related note: the body line of a macro whose expansion has the problem.
pub const ASM_RELATED_IN_MACRO: DiagCode = DiagCode("asm-related-in-macro");

/// Related note: where the bytes of an overlap were placed first.
pub const ASM_RELATED_OTHER_BYTES: DiagCode = DiagCode("asm-related-other-bytes");

/// Every code the assembler lists through `Architecture::diagnostic_codes`, after the lexer
/// codes (§7.1).
///
/// Empty in the WP0 scaffold (see the module documentation); WP-D fills it.
pub static ASM_DIAG_CODES: [DiagCode; 0] = [];
