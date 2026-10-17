# Diagnostics of the 68000 lexer, English (m1-core-isa-lexer.md §5.4, §5.7, §6.1).
#
# One message per code returned by M68K.diagnostic_codes(), with the id diag-<code>. Every
# message receives $text, the offending source text (truncated to 32 characters plus “…”).
# diag-lex-operands-in-comment also receives $cause (after-comma, inside-expression or
# operand-list), which selects its .help.
# Written in our own words; typographic quotes and apostrophes (’) only.

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
