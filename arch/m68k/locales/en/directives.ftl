# 68000 assembler directives, English (m2-assembler.md §4.5–§4.8, §7.1).
#
# One message per directive spelling of the A68k dialect (m1-core-isa-lexer.md §5.3), with the
# id isa-m68k-dir-<name> in lowercase (= is isa-m68k-dir-equals), the doc id that
# SyntaxProvider::keywords() gives the directive keyword: a one-line summary and .details (2 to 6
# sentences) saying what VisualAssembler does with the directive, including those it refuses as
# a68k -s does and those it does not support.
# Written in our own words: no sentence comes from the a68k documentation or from the help of
# another program. Style: no braces in the text, no continuation line starting with a dot, a star
# or a bracket, typographic apostrophes (’) only; quoted strings are described, not shown.

isa-m68k-dir-equals = Gives the label a constant value, like EQU.
    .details = Syntax: name = expression, with the name in column 1, for example size = 64. It
        is the same as EQU: the value is fixed once, and defining the name again is an error.
        The expression may use symbols defined further down in the file. Like an ordinary label,
        the line starts a new block of local labels.

isa-m68k-dir-bss = Opens a section for zero-filled data; refused, as by a68k -s.
    .details = In object files meant for a linker, BSS starts a section of memory that is
        reserved but not stored. VisualAssembler builds programs as S-records, like a68k with
        its -s option, and a68k refuses BSS in that mode, so the line is refused with an error.
        Reserve the memory with DS at an address chosen with ORG.

isa-m68k-dir-cnop = Pads with zero bytes up to an address with a chosen remainder.
    .details = Syntax: CNOP offset,alignment, for example CNOP 0,4 to reach the next multiple of 4.
        Both values must be known at this line: the alignment is a power of two from 2 up, and
        the offset goes from 0 to the alignment minus 1. Zero bytes are added until the location
        counter, divided by the alignment, leaves the offset as remainder. Nothing is added when
        the counter already fits.

isa-m68k-dir-code = Opens a code section; refused, as by a68k -s.
    .details = In object files meant for a linker, CODE starts a section of instructions.
        VisualAssembler builds programs as S-records, like a68k with its -s option, and a68k
        refuses CODE in that mode, so the line is refused with an error. Place the code at a
        chosen address with ORG.

isa-m68k-dir-cseg = Another name for CODE; refused, as by a68k -s.
    .details = CSEG starts a code section in object files meant for a linker, exactly like CODE.
        A68k refuses it when it produces S-records, and VisualAssembler, which always builds
        S-records, refuses the line with an error. Place the code at a chosen address with ORG.

isa-m68k-dir-data = Opens a data section; refused, as by a68k -s.
    .details = In object files meant for a linker, DATA starts a section of constants and
        variables. VisualAssembler builds programs as S-records, like a68k with its -s option,
        and a68k refuses DATA in that mode, so the line is refused with an error. Place the data
        at a chosen address with ORG, then write it with DC or reserve it with DS.

isa-m68k-dir-dc = Stores constants: bytes, words or long words.
    .details = Syntax: DC.B, DC.W or DC.L followed by values separated by commas and no space;
        without a suffix the size is word. Each value is an expression; in DC.B, a value made of
        one quoted string stores its characters as bytes. DC.W and DC.L first align the location
        counter to an even address. A byte holds -128 to 255 and a word -32768 to 65535, and a
        value outside these ranges is an error. A space ends the operand field, so DC.B 1, 2
        stores only the 1.

isa-m68k-dir-dcb = Stores one value repeated a given number of times.
    .details = Syntax: DCB.B, DCB.W or DCB.L count,value; without a suffix the size is word. The
        count must be known at this line and be 0 or more. DCB.W and DCB.L first align to an
        even address. For example, DCB.W 4,-1 stores four words of $FFFF.

isa-m68k-dir-ds = Reserves memory filled with zeros.
    .details = Syntax: DS.B, DS.W or DS.L count; without a suffix the size is word. It reserves
        that many bytes, words or long words, which VisualAssembler fills with zeros in the
        program and in the S-record, as a68k does. The count must be known at this line and be 0
        or more. DS.W and DS.L first align to an even address, so DS.W 0 only aligns.

isa-m68k-dir-dseg = Another name for DATA; refused, as by a68k -s.
    .details = DSEG starts a data section in object files meant for a linker, exactly like DATA.
        A68k refuses it when it produces S-records, and VisualAssembler, which always builds
        S-records, refuses the line with an error. Place the data at a chosen address with ORG.

isa-m68k-dir-end = Ends the program, optionally with its start address.
    .details = Syntax: END, or END expression to give the entry point, which the last record of
        the S-record holds. Assembly stops at this line: the lines after it are ignored, in this
        file and in the files that included it, and text after END other than a comment gives a
        warning. END is optional; without it the program ends with its last line and has no
        entry point.

isa-m68k-dir-endc = Closes a conditional block opened by an IF directive.
    .details = Every IFEQ, IFNE, IFGT, IFGE, IFLT, IFLE, IFD, IFND, IFC or IFNC needs its ENDC.
        Blocks may be nested, and each ENDC closes the innermost open block. An ENDC without an
        open block, or a block still open at the end of the program, is an error. A68k 2.71 has
        no ELSE: to choose between two cases, write a second block with the opposite test.

isa-m68k-dir-endif = Another name for ENDC: closes a conditional block.
    .details = ENDIF and ENDC are the same directive, and either one closes the innermost block
        opened by an IF directive. Blocks may be nested. An ENDIF without an open block is an
        error.

isa-m68k-dir-endm = Ends the body of a macro.
    .details = The lines between name MACRO and ENDM are stored as the body of the macro; they
        are assembled only where the macro is called. ENDM outside a macro, or a macro whose ENDM
        is missing, is an error. To leave an expansion before its end, use MEXIT.

isa-m68k-dir-equ = Gives the label a constant value.
    .details = Syntax: name EQU expression, with the name in column 1, for example size EQU 64.
        The value is fixed once: defining the name again is an error, while a name defined with
        SET may change. The expression may use symbols defined further down in the file. Symbols
        are case-sensitive, so Size and SIZE are two different names. Like an ordinary label, EQU
        starts a new block of local labels.

isa-m68k-dir-equr = Names a register; not supported by VisualAssembler.
    .details = In a68k, name EQUR D3 lets a program write name instead of D3. VisualAssembler does
        not support register names and refuses the line with an error, because it recognizes
        registers line by line, without reading the rest of the program. Write the register
        itself, with a comment saying what it holds.

isa-m68k-dir-even = Moves to the next even address.
    .details = If the location counter is odd, EVEN adds one zero byte; otherwise it does nothing.
        Instructions, and DC, DS and DCB in word or long size, are aligned automatically, so EVEN
        is mostly needed before a label that stands alone on its line after byte data. The 68000
        reads and writes words and long words only at even addresses.

isa-m68k-dir-far = An a68k addressing model directive; not supported.
    .details = In a68k, NEAR and FAR turn on and off a model in which data is reached relative to
        an address register. VisualAssembler has no such model and refuses the line with an
        error. Remove the line: VisualAssembler chooses absolute or PC-relative addressing for
        each operand itself.

isa-m68k-dir-idnt = Names the program module; accepted with no effect.
    .details = Syntax: IDNT name. In object files, the name identifies the module for a linker.
        VisualAssembler accepts the line and ignores it: the S-record it writes has no module
        name, as with a68k -s.

isa-m68k-dir-ifc = Assembles a block if two strings are equal.
    .details = Syntax: IFC followed by two quoted strings separated by a comma, then the block,
        then ENDC. The strings are compared byte for byte, so the case of the letters matters. It
        is mostly used in macros, for example to test whether the argument \1 is empty by
        comparing it with an empty string. The test is made once, when the line is read; the
        lines of a false block are skipped and give no error.

isa-m68k-dir-ifd = Assembles a block if a symbol is defined above.
    .details = Syntax: IFD name, then the block, then ENDC. The block is assembled when the name
        is defined on an earlier line, as a label or with EQU, SET or =, and skipped otherwise. A
        definition further down does not count, because the test is made once, when the line is
        read. IFND is the opposite test.

isa-m68k-dir-ifeq = Assembles a block if a value is zero.
    .details = Syntax: IFEQ expression, then the block, then ENDC. The expression must be known at
        this line: a symbol defined further down is refused. The lines of a false block are
        skipped and give no error, so a constant such as DEBUG EQU 0 can switch parts of a
        program on and off. Blocks may be nested.

isa-m68k-dir-ifge = Assembles a block if a value is zero or more.
    .details = Syntax: IFGE expression, then the block, then ENDC. The expression must be known at
        this line, and its value is compared with zero. The lines of a false block are skipped
        and give no error. IFLT is the opposite test.

isa-m68k-dir-ifgt = Assembles a block if a value is greater than zero.
    .details = Syntax: IFGT expression, then the block, then ENDC. The expression must be known at
        this line, and its value is compared with zero. The lines of a false block are skipped
        and give no error. IFLE is the opposite test.

isa-m68k-dir-ifle = Assembles a block if a value is zero or less.
    .details = Syntax: IFLE expression, then the block, then ENDC. The expression must be known at
        this line, and its value is compared with zero. The lines of a false block are skipped
        and give no error. IFGT is the opposite test.

isa-m68k-dir-iflt = Assembles a block if a value is less than zero.
    .details = Syntax: IFLT expression, then the block, then ENDC. The expression must be known at
        this line, and its value is compared with zero. The lines of a false block are skipped
        and give no error. IFGE is the opposite test.

isa-m68k-dir-ifnc = Assembles a block if two strings differ.
    .details = Syntax: IFNC followed by two quoted strings separated by a comma, then the block,
        then ENDC. The strings are compared byte for byte, so the case of the letters matters. In
        a macro it can check that an argument was given, by comparing it with an empty string.
        IFC is the opposite test.

isa-m68k-dir-ifnd = Assembles a block if a symbol is not defined above.
    .details = Syntax: IFND name, then the block, then ENDC. The block is assembled when the name
        is not defined on an earlier line; a definition further down does not count. It can guard
        a file of definitions: IFND a name, then that name EQU 1 and the definitions, then ENDC,
        so that a file included twice defines its symbols once. IFD is the opposite test.

isa-m68k-dir-ifne = Assembles a block if a value is not zero.
    .details = Syntax: IFNE expression, then the block, then ENDC. The expression must be known at
        this line, and its value is compared with zero. The lines of a false block are skipped
        and give no error. IFEQ is the opposite test.

isa-m68k-dir-incbin = Inserts the bytes of a file at this point.
    .details = Syntax: INCBIN path, with the path written as it is or between quotes. The bytes
        of the file are copied at the location counter, without alignment; an instruction placed
        after them is aligned automatically. The path is looked up as for INCLUDE, and the same
        paths are refused. The file may hold at most 16 MiB.

isa-m68k-dir-include = Assembles the lines of another source file at this point.
    .details = Syntax: INCLUDE path, with the path written as it is or between quotes. The path
        is looked up in the folder of the main file, then in each include folder; a68k looks in
        its working folder instead, so keep the files of a program together. For safety, an
        absolute path, a drive, a network share, a device name or a path that leads out of its
        folder is refused. Included files may include others, up to 16 levels, and a file that
        includes itself is an error. A problem in an included file is reported in that file,
        with a note on the INCLUDE line.

isa-m68k-dir-list = Turns the listing back on; accepted with no effect.
    .details = In a68k, LIST turns the listing back on after NOLIST. VisualAssembler accepts the
        line and ignores it, because its listing always shows every line.

isa-m68k-dir-macro = Starts the definition of a macro.
    .details = Syntax: name MACRO, with the name in column 1, then the body lines, then ENDM. A
        call writes the name as an operation, followed by arguments separated by commas: in the
        body, \1 to \9 stand for them, \0 for the size suffix of the call (W when there is none)
        and \@ for a number unique to each call, which makes labels unique. NARG gives the number
        of arguments, and an argument holding spaces or commas is written between < and >. Macro
        names are case-sensitive, must be defined before their first call and cannot be the name
        of an instruction or a directive.

isa-m68k-dir-mexit = Leaves the current macro expansion.
    .details = MEXIT stops the expansion of the macro being called: the rest of its body is
        skipped for this call. It is usually placed in a conditional block, to end the macro
        early depending on an argument. MEXIT outside a macro is an error.

isa-m68k-dir-near = An a68k addressing model directive; not supported.
    .details = In a68k, NEAR and FAR turn on and off a model in which data is reached relative to
        an address register. VisualAssembler has no such model and refuses the line with an
        error. Remove the line: VisualAssembler chooses absolute or PC-relative addressing for
        each operand itself.

isa-m68k-dir-nol = Another name for NOLIST; accepted with no effect.
    .details = In a68k, NOL stops the listing until LIST, exactly like NOLIST. VisualAssembler
        accepts the line and ignores it, because its listing always shows every line.

isa-m68k-dir-nolist = Turns the listing off; accepted with no effect.
    .details = In a68k, NOLIST stops the listing until LIST. VisualAssembler accepts the line and
        ignores it, because its listing always shows every line.

isa-m68k-dir-org = Sets the address of the code and data that follow.
    .details = Syntax: ORG address, for example ORG $1000. The address must be known at this line
        and stay below $1000000, the end of the 68000 address space; without ORG, the program
        starts at address 0. An ORG that leaves a gap starts a new segment of the program. Placing
        bytes over bytes already placed, for example with an ORG back to a used address, is an
        error.

isa-m68k-dir-page = Starts a new page of a printed listing; accepted with no effect.
    .details = VisualAssembler accepts PAGE and ignores it, because its listing is plain text with
        no pages. It changes nothing in the program.

isa-m68k-dir-public = Another name for XDEF; refused, as by a68k -s.
    .details = PUBLIC makes symbols visible to other files linked with this one, exactly like
        XDEF. A68k refuses it when it produces S-records, and VisualAssembler, which always builds
        S-records, refuses the line with an error. Remove the line: every symbol is already
        visible in the whole program, included files too.

isa-m68k-dir-reg = Names a register list; not supported by VisualAssembler.
    .details = In a68k, name REG D0-D3/A0 lets MOVEM use name instead of the list. VisualAssembler
        does not support register names and refuses the line with an error, because it recognizes
        registers line by line, without reading the rest of the program. Write the list itself in
        each MOVEM.

isa-m68k-dir-rorg = Sets a relocatable origin; not supported by VisualAssembler.
    .details = In a68k, RORG moves the location counter relative to the start of a relocatable
        section, in programs meant for a linker. VisualAssembler does not support it and refuses
        the line with an error. Place the code at an address with ORG.

isa-m68k-dir-section = Opens a named section; refused, as by a68k -s.
    .details = In object files meant for a linker, SECTION starts a named section of code or data.
        VisualAssembler builds programs as S-records, like a68k with its -s option, and a68k
        refuses SECTION in that mode, so the line is refused with an error. Place each part of
        the program at a chosen address with ORG.

isa-m68k-dir-set = Gives the label a value that may change later.
    .details = Syntax: name SET expression, with the name in column 1. Unlike EQU, the name may be
        set again with SET, and each use sees the value set last above it. The expression must be
        known at this line. A name defined with SET cannot also be defined with EQU or as a label.
        Like an ordinary label, SET starts a new block of local labels.

isa-m68k-dir-spc = Adds blank lines to a printed listing; accepted with no effect.
    .details = Syntax: SPC count. VisualAssembler accepts the line and ignores it, because its
        listing has no layout control. It changes nothing in the program.

isa-m68k-dir-title = Sets the title of a printed listing; accepted with no effect.
    .details = Syntax: TITLE text. VisualAssembler accepts the line and ignores it: its listing
        starts with a header line of its own. It changes nothing in the program.

isa-m68k-dir-ttl = Another name for TITLE; accepted with no effect.
    .details = Syntax: TTL text, exactly like TITLE. VisualAssembler accepts the line and ignores
        it: its listing starts with a header line of its own.

isa-m68k-dir-xdef = Makes symbols visible to other files; refused, as by a68k -s.
    .details = In object files, XDEF makes symbols of this file visible to the other files linked
        with it. A program built as an S-record is a single unit with no linker, and a68k refuses
        XDEF when it produces S-records, so VisualAssembler refuses the line with an error. Remove
        the line: every symbol is already visible in the whole program, included files too.

isa-m68k-dir-xref = Uses symbols of another file; refused, as by a68k -s.
    .details = In object files, XREF names symbols that a linker takes from another file. A
        program built as an S-record is a single unit with no linker, and a68k refuses XREF when
        it produces S-records, so VisualAssembler refuses the line with an error. Bring the
        definitions into the program instead, for example with INCLUDE.
