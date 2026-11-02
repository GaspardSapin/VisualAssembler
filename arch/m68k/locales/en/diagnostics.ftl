# Diagnostics of the 68000 lexer and assembler, English (m1-core-isa-lexer.md §5.4, §5.7,
# §6.1; m2-assembler.md §7).
#
# One message per code returned by M68K.diagnostic_codes(), with the id diag-<code>: the 11
# lexer codes, then the 50 assembler codes of arch/m68k/src/asm/codes.rs, whose documentation
# lists the variables each assembler message takes and the values of every selector. Every
# lexer message receives $text, the offending source text (truncated to 32 characters plus “…”);
# diag-lex-operands-in-comment also receives $cause (after-comma, inside-expression or
# operand-list), which selects its .help.
# Assembler messages start in lowercase and have no final period; .help and .note are full
# sentences, and .help says how to fix the problem. The diag-asm-related-* messages are the
# related notes of a diagnostic and have no attribute.
# Written in our own words; typographic quotes and apostrophes (’) only.

## Lexer (m1-core-isa-lexer.md §5.4, §5.7)

diag-lex-bad-label = “{ $text }” is not a valid label
    .help = A label starts with a letter, “_” or “.”, followed by letters, digits, “_”, “.” or “@”. A local label is digits followed by “$” (such as 1$) or a backslash followed by a name.

diag-lex-label-junk = unexpected text “{ $text }” after the label
    .help = Separate the label from the instruction with a space or a tab, for example “loop: nop”.

diag-lex-bad-operation = “{ $text }” is not an instruction, a directive or a macro name
    .help = The operation field holds a name such as MOVE, DC or a macro name, optionally followed by a size such as .W.

diag-lex-bad-size-suffix = “{ $text }” is not a valid size suffix
    .help = Use .B (byte), .W (word, 16 bits), .L (long word, 32 bits) or .S (short branch).

diag-lex-bad-number = “{ $text }” is not a valid number
    .help = Write 255 in decimal, $FF in hexadecimal, %11111111 in binary or @377 in octal; the forms 0xFF and 0FFh are not accepted.

diag-lex-unterminated-string = the string “{ $text }” is not closed
    .help = End the string with the quote that opened it. To put that quote inside the string, write it twice.

diag-lex-bad-backslash = “{ $text }” is not a valid backslash sequence
    .help = Outside strings and comments, a backslash starts a macro parameter (\0 to \9, or \@) or a local label (\name).

diag-lex-unexpected-char = unexpected character “{ $text }”
    .help = This character has no meaning here; remove it, or write it in a comment after “;”.

diag-lex-non-ascii = non-ASCII character “{ $text }” outside a comment or a string
    .help = Accented letters and other non-ASCII characters are accepted only in comments and in strings.

diag-lex-line-too-long = this line is longer than 64 KiB and is not analysed
    .help = Split it into several lines.

diag-lex-operands-in-comment = a68k will ignore “{ $text }”: the operands end at the first space.
    .help = { $cause ->
        [after-comma] Remove the spaces after the commas.
        [inside-expression] Remove the spaces inside the expression.
       *[operand-list] Join the operands, or start the comment with “;”.
    }

## Assembler (m2-assembler.md §7.2)

diag-asm-unknown-operation = unknown instruction or macro “{ $name }”
    .note = { $count ->
        [0] Macro names are case-sensitive and must be defined before their first use.
       *[other] Did you mean { $suggestions }?
    }

diag-asm-not-a68k-directive = a68k 2.71 has no { $name } directive
    .help = { $name ->
        [ELSE] Close the block with ENDC and open a second block with the opposite IF test.
        [REPT] Write the lines out one by one, or define a macro and call it once per repetition.
       *[other] Optimizations are switched off with the no-optimization option of the assembler, not in the source file.
    }

diag-asm-unsupported-directive = { $name } is an a68k directive that VisualAssembler does not support
    .help = { $name ->
        [EQUR] Write the register itself (D0 to D7, A0 to A7): VisualAssembler does not give registers other names.
        [REG] Write the register list itself, for example D0-D3/A0 in MOVEM.
        [RORG] Place the code with ORG instead.
       *[other] Remove this line: VisualAssembler has no near or far addressing model.
    }

diag-asm-not-in-s-format = a68k refuses { $name } when it produces S-records
    .help = Place code and data with ORG: a program loaded from an S-record has no sections and no external symbols.

diag-asm-bad-operand = cannot read the operand “{ $text }”
    .help = Write it in one of the forms D0, A1, (A0), (A0)+, -(A0), 8(A0), 4(A0,D1.W), loop(PC), $1000, ($1000).W, #5 or D0-D3/A0; the displacement goes before the parenthesis: 8(A0), not (8,A0).

diag-asm-bad-expression = invalid expression “{ $text }”
    .help = Check that every parenthesis is closed and that no operator is missing or doubled: two signs in a row, such as --2, are refused.

diag-asm-operand-count = { $mnemonic } takes { $expected ->
        [0] no operand
        [one] one operand
       *[other] { $expected } operands
    }, not { $found }
    .help = Check the operands: they are separated by commas and no space, because the first space ends the operand field.

diag-asm-mode-not-allowed = { $mnemonic } does not accept the mode { $mode } here
    .note = Modes accepted here: { $allowed }.

diag-asm-size-not-allowed = { $mnemonic } has no .{ $size } form
    .note = Accepted sizes: { $allowed }.

diag-asm-no-matching-form = these operands fit no form of { $mnemonic }
    .note = Accepted forms: { $forms }.

diag-asm-register-size-suffix = a size after a register (“{ $text }”) is allowed only for the index of d8(An,Xn) or d8(PC,Xn)
    .help = Remove the size after the register: the size of the operation goes after the instruction, as in MOVE.W D0,D1.

diag-asm-missing-displacement = a68k needs a displacement before (An,Xn)
    .help = Write 0 before the parenthesis, for example 0(A0,D0).

diag-asm-absolute-size-syntax = a68k does not read “{ $text }” as a sized address
    .help = Put the address in parentheses before the size, for example ($400).W.

diag-asm-value-out-of-range = { $value } does not fit here (allowed: { $min } to { $max })
    .help = Change the value, or choose a larger size or an instruction that accepts it.

diag-asm-branch-out-of-range = the target is { $distance } bytes away, too far for a .{ $size } branch
    .help = Remove the size and let the assembler choose; for a target more than 32 KiB away, use JMP or JSR.

diag-asm-short-branch-zero = a short branch cannot jump to the next instruction (its displacement would be 0)
    .help = Remove “.s”: the assembler then writes a word branch.

diag-asm-odd-branch-target = the target { $target } is at an odd address: the 68000 will stop with an address error
    .help = Put the label on the line of the instruction it names, or add EVEN before the label.

diag-asm-undefined-symbol = “{ $name }” is not defined
    .help = Define it with a label in column 1 or with EQU, or check its spelling.
    .note = { $count ->
        [0] Symbols distinguish upper and lower case.
       *[other] Did you mean { $suggestions }?
    }

diag-asm-undefined-local-label = the local label “{ $name }” is not defined in this block
    .note = A local label is visible only until the next ordinary label, EQU, SET or “=”.

diag-asm-duplicate-symbol = “{ $name }” is already defined
    .help = Rename one of the two definitions; only a symbol defined with SET can get a new value, again with SET.

diag-asm-reserved-name = “{ $name }” is an instruction or directive and cannot name a macro
    .help = Choose another name for the macro, for example by adding a prefix.

diag-asm-label-like-keyword = “{ $name }” in column 1 is a label, not an instruction
    .help = Indent the line with a space or a tab to make it an instruction.

diag-asm-label-required = { $name } needs a name in column 1
    .help = Write the name at the very start of the line, then { $name }.

diag-asm-value-not-known = { $name } needs a value known at this line, but “{ $symbol }” is defined further down
    .help = Move the definition of “{ $symbol }” above this line.

diag-asm-division-by-zero = division by zero
    .help = Check the value after “/”: here it is 0.

diag-asm-negative-count = { $name } cannot use a negative count ({ $count })

diag-asm-address-overflow = the program goes past { $address }, the end of the 68000 address space
    .help = The 68000 addresses 16 MiB, from $000000 to $FFFFFF: check the ORG values and the sizes given to DS and DCB.

diag-asm-overlap = these bytes overlap bytes already placed at { $address }
    .help = Check the ORG lines: two parts of the program use the same addresses.

diag-asm-string-in-expression = a quoted string cannot be part of a calculation (“{ $text }”)
    .help = Write the character code, for example $42.

diag-asm-string-too-long = a .{ $size } value holds at most { $max } characters, this string has { $length }
    .help = Store longer text with DC.B, which accepts strings of any length.

diag-asm-text-after-end = the lines after END are ignored
    .help = Move these lines above END, or delete them.

diag-asm-unbalanced-conditional = { $kind ->
        [close] { $name } has no matching IF
       *[open] this { $name } block is never closed by ENDC
    }
    .help = { $kind ->
        [close] Remove this { $name }, or add the IF line that opens its block.
       *[open] Add ENDC after the last line of the block.
    }

diag-asm-unbalanced-macro = { $kind ->
        [close] { $name } is outside any macro
       *[open] the macro “{ $name }” has no ENDM
    }
    .help = { $kind ->
        [close] ENDM and MEXIT belong between a MACRO line and its ENDM: remove this { $name }, or move it into the macro.
       *[open] Add ENDM after the last line of the macro.
    }

diag-asm-macro-param-outside = “{ $text }” is a macro parameter, but this line is not in a macro
    .help = \0 to \9 and \@ are replaced only in the lines of a macro body: remove the parameter, or move the line into a macro.

diag-asm-macro-too-deep = macros call each other more than { $limit } levels deep
    .help = Look for a macro that calls itself, directly or through another macro.

diag-asm-expansion-limit = the program exceeds { $limit } lines once macros are expanded; assembly stops
    .help = Look for a macro that calls itself without end.

diag-asm-include-too-deep = INCLUDE files are nested more than { $limit } levels deep
    .help = Include the files from the main file rather than from one another.

diag-asm-include-cycle = “{ $path }” includes itself
    .help = Remove the INCLUDE line that reads this file again, directly or through another file.

diag-asm-include-not-found = cannot find the file “{ $path }”
    .note = Paths are relative to the folder of the main file, then to each include folder.

diag-asm-include-refused = the path “{ $path }” is refused: { $reason ->
        [absolute] it is absolute
        [drive] it names a drive, a network share or a device
        [colon] it contains “:”
        [device] it names a Windows device
        [escapes] it leads out of its folder
       *[other] it is empty or invalid
    }
    .help = Use a relative path inside the folder of the main file, or add an include folder.

diag-asm-include-failed = cannot use “{ $path }”: { $reason ->
        [too-large] it is larger than 16 MiB
        [not-utf8] it is not UTF-8 text
        [too-many-files] the program already uses too many files
        [unsupported] this tool cannot read binary files
       *[other] it cannot be read
    }
    .help = { $reason ->
        [too-large] Split it into several smaller files.
        [not-utf8] Save it again as UTF-8 text from your editor.
        [too-many-files] Gather the included files into fewer files.
        [unsupported] Remove the INCBIN line: binary files cannot be read here.
       *[other] Check that the file still exists and that you are allowed to read it.
    }

diag-asm-too-many-diagnostics = only the first { $limit } problems are reported
    .help = Fix these problems first, then assemble again to see the next ones.

diag-asm-operands-in-comment = { $kind ->
        [data] a68k will ignore “{ $text }”: only { $bytes ->
            [one] 1 byte
           *[other] { $bytes } bytes
        } will be produced
       *[instruction] a68k will ignore “{ $text }”: only “{ $kept }” is assembled
    }
    .help = { $cause ->
        [after-comma] Remove the spaces after the commas.
        [inside-expression] Remove the spaces inside the expression.
       *[operand-list] Join the operands, or start the comment with “;”.
    }

diag-asm-movem-single = a68k assembles this one-register MOVEM as MOVE.{ $size }: { $effect ->
        [sign] the flags change and the register keeps its upper word instead of being sign-extended
       *[flags] the flags N and Z change and V and C are cleared
    }
    .help = Write MOVE yourself, or switch optimizations off to keep MOVEM.

diag-asm-phase-error = the value of “{ $name }” changed after the layout was fixed
    .help = This should not happen: please report it to the VisualAssembler developers, with the source file.

## Related notes of assembler diagnostics (no attribute)

diag-asm-related-first-definition = first definition of “{ $name }”

diag-asm-related-scope-start = the block of local labels starts here

diag-asm-related-included-from = included from here

diag-asm-related-in-macro = line of the macro “{ $name }”

diag-asm-related-other-bytes = the other bytes come from here
