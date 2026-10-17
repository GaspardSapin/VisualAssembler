# 68000 instruction documentation, English (m1-core-isa-lexer.md §4.3, §6.1).
#
# Written in our own words (m1 §4.6): no sentence comes from the Motorola manuals or from the
# help of another program. Ids are English and kebab-case:
#   isa-m68k-<entry>         one-line summary; .details (2 to 6 sentences: syntax, effect,
#                            addressing modes, flags X N Z V C, timing notes); .flags when a
#                            form of the entry has an instruction-specific flag rule
#   isa-m68k-cond-<name>     meaning of a condition code
#   isa-m68k-outcome-<id>    case of a timing row (short phrase)
#   isa-m68k-unit-<id>       counted unit of a timing row (short phrase)
# Exact operand modes, flags and cycle counts come from the tables and are shown next to these
# texts, so the texts explain rather than repeat them.
# Style: no braces in the text, no continuation line starting with a dot, a star or a bracket,
# typographic apostrophes (’) only.
#
# Vocabulary used below: “data modes” are every addressing mode except An; an “alterable”
# operand can be written (no PC-relative and no immediate operand); “control modes” are (An),
# d16(An), d8(An,Xn), xxx.W, xxx.L, d16(PC) and d8(PC,Xn).

## Instructions

isa-m68k-abcd = Adds two packed-BCD bytes and the X flag.
    .details = Syntax: ABCD Dy,Dx or ABCD -(Ay),-(Ax), byte size only. Each byte holds two
        decimal digits (packed BCD), and the sum source + destination + X is computed in decimal
        and stored in the destination. The memory form steps backwards through memory, so a
        chain of ABCD adds long decimal numbers starting from their last byte. X and C receive
        the decimal carry.
    .flags = Z is cleared when the result is not zero and left unchanged otherwise. After a chain
        of ABCD it therefore tells whether the whole number is zero, provided Z was set (and X
        cleared) before the first one. N and V are undefined.

isa-m68k-add = Adds the source to the destination.
    .details = Syntax: ADD.B/W/L <ea>,Dn or ADD.B/W/L Dn,<ea>; without a suffix the size is
        word. One operand must be a data register: the source of the first form may use any
        mode (An only in word or long size), the destination of the second form is an
        alterable memory operand. A68k also accepts ADD with an address register destination
        (ADDA) or an immediate source (ADDI) and then picks that encoding. X, N, Z, V and C are
        all set from the result. Adding into a register is faster than adding into memory.

isa-m68k-adda = Adds the source to an address register.
    .details = Syntax: ADDA.W/L <ea>,An; the source may use any addressing mode. A word source is
        sign-extended to 32 bits first, and the whole address register always changes. No flag
        is affected, which makes ADDA convenient for pointer arithmetic between a test and its
        branch. A68k also selects ADDA when you write ADD with an address register destination.

isa-m68k-addi = Adds an immediate value to the destination.
    .details = Syntax: ADDI.B/W/L #<data>,<ea>; the destination is a data register or an
        alterable memory operand. The constant is stored in the words that follow the opcode,
        which costs extra memory reads. X, N, Z, V and C are set from the result. For a
        constant from 1 to 8, ADDQ is shorter and faster.

isa-m68k-addq = Adds a constant from 1 to 8 to the destination.
    .details = Syntax: ADDQ.B/W/L #<1-8>,<ea>. The constant is packed into the opcode itself, so
        it needs no extension word and runs quickly. The destination may be any alterable
        operand, including an address register in word or long size. On an address register
        the whole 32 bits change and no flag is affected; otherwise X, N, Z, V and C are set
        from the result.

isa-m68k-addx = Adds the source and the X flag to the destination.
    .details = Syntax: ADDX.B/W/L Dy,Dx or ADDX.B/W/L -(Ay),-(Ax). It builds additions wider
        than 32 bits: add the lowest parts with ADD, then each higher part with ADDX, which
        includes the carry kept in X. The memory form predecrements both pointers, so it walks
        from the least significant end of numbers stored in memory. X, N, V and C are set from
        the result.
    .flags = Z is cleared when the result is not zero and left unchanged otherwise. After a chain
        of ADDX it therefore tells whether the complete result is zero, provided Z was set
        before the chain, for example with MOVE #4,CCR.

isa-m68k-and = Bitwise AND of the source into the destination.
    .details = Syntax: AND.B/W/L <ea>,Dn or AND.B/W/L Dn,<ea>. A result bit is 1 only where both
        operands have a 1, which is how a mask clears bits. The source of the first form may use
        any data mode; the destination of the second form is an alterable memory operand. A68k
        also accepts an immediate source and then encodes ANDI. N and Z are set from the result,
        V and C are cleared and X is unchanged.

isa-m68k-andi = Bitwise AND of an immediate value into the destination, CCR or SR.
    .details = Syntax: ANDI.B/W/L #<data>,<ea> with a data register or an alterable memory
        destination, ANDI #<data>,CCR to clear chosen condition flags, or ANDI #<data>,SR to
        clear chosen bits of the whole status register. ANDI to SR is privileged: in user mode
        it raises a privilege violation exception. In the ordinary form N and Z are set from the
        result, V and C are cleared and X is unchanged.
    .flags = With CCR or SR as destination, each flag is ANDed with the matching bit of the
        immediate value: a 0 bit clears the flag and a 1 bit keeps it as it was.

isa-m68k-asl = Arithmetic shift to the left.
    .details = Syntax: ASL.B/W/L Dx,Dy (count in Dx, modulo 64), ASL.B/W/L #<1-8>,Dy, or ASL <ea>
        to shift a word in alterable memory by one bit. Zeros enter on the right and each step
        multiplies the value by 2. X and C receive the last bit shifted out, N and Z describe
        the result, and V is set if the sign bit changed at any moment, which signals a signed
        overflow. With a count of 0, C is cleared and X is unchanged. The register forms take 2
        more clock periods per bit position.

isa-m68k-asr = Arithmetic shift to the right.
    .details = Syntax: ASR.B/W/L Dx,Dy (count in Dx, modulo 64), ASR.B/W/L #<1-8>,Dy, or ASR <ea>
        to shift a word in alterable memory by one bit. The sign bit is copied into the vacated
        positions, so each step divides a signed value by 2, rounding towards minus infinity.
        X and C receive the last bit shifted out, N and Z describe the result, and V is
        cleared. With a count of 0, C is cleared and X is unchanged. The register forms take 2
        more clock periods per bit position.

isa-m68k-bcc = Branches to a label when the condition holds.
    .details = Syntax: Bcc <label>, where cc is HI, LS, CC (or HS), CS (or LO), NE, EQ, VC, VS,
        PL, MI, GE, LT, GT or LE. The condition is evaluated on the flags left by an earlier
        instruction, usually CMP or TST: if it holds, execution continues at the label,
        otherwise at the next instruction. The short size S stores an 8-bit offset (about 128
        bytes either way) and the word size W a 16-bit offset (about 32 KB); A68k reads the
        suffix B as S and L as W, and the assembler chooses when no suffix is given. No flag
        is changed. A taken branch costs 10 clock periods; a branch not taken costs 8 in the
        short size and 12 in the word size.

isa-m68k-bchg = Tests a bit, then inverts it.
    .details = Syntax: BCHG Dn,<ea> or BCHG #<number>,<ea>; the first operand gives the bit
        number. On a data register the instruction works on 32 bits (bit number modulo 32); on
        an alterable memory operand it works on one byte (bit number modulo 8). Z is set if the
        bit was 0 before the change and cleared if it was 1; the other flags are unchanged. On
        a register the time depends on the bit number, so the tables give the maximum.

isa-m68k-bclr = Tests a bit, then clears it.
    .details = Syntax: BCLR Dn,<ea> or BCLR #<number>,<ea>; the first operand gives the bit
        number. On a data register the instruction works on 32 bits (bit number modulo 32); on
        an alterable memory operand it works on one byte (bit number modulo 8). Z is set if the
        bit was 0 before it was cleared and cleared if it was 1; the other flags are unchanged.
        On a register the time depends on the bit number, so the tables give the maximum.

isa-m68k-bra = Branches to a label unconditionally.
    .details = Syntax: BRA <label>, in the short size S (8-bit offset) or the word size W
        (16-bit offset); A68k reads the suffix B as S and L as W. Execution always continues at
        the label, whose distance is stored relative to the program counter, so the code keeps
        working wherever it is loaded. No flag is changed. Both sizes take 10 clock periods;
        for a target beyond about 32 KB, use JMP.

isa-m68k-bset = Tests a bit, then sets it.
    .details = Syntax: BSET Dn,<ea> or BSET #<number>,<ea>; the first operand gives the bit
        number. On a data register the instruction works on 32 bits (bit number modulo 32); on
        an alterable memory operand it works on one byte (bit number modulo 8). Z is set if the
        bit was 0 before it was set and cleared if it was 1; the other flags are unchanged. On
        a register the time depends on the bit number, so the tables give the maximum.

isa-m68k-bsr = Calls a subroutine at a nearby label.
    .details = Syntax: BSR <label>, in the short size S or the word size W, like BRA. The address
        of the next instruction is pushed on the stack as a long word (A7 decreases by 4), then
        execution continues at the label; RTS later returns there. The target is PC-relative and
        must lie within about 32 KB; use JSR beyond that. No flag is changed, and the call
        takes 18 clock periods.

isa-m68k-btst = Tests a bit.
    .details = Syntax: BTST Dn,<ea> or BTST #<number>,<ea>. Z is set if the chosen bit is 0 and
        cleared if it is 1; neither the operand nor the other flags change, so BEQ or BNE
        usually follows. A data register is tested on 32 bits (bit number modulo 32) and a
        memory operand on one byte (bit number modulo 8). Unlike the other bit instructions,
        BTST may also read a PC-relative operand, and with the bit number in a register even
        an immediate one.

isa-m68k-chk = Checks a register against a bound and raises an exception when out of range.
    .details = Syntax: CHK <ea>,Dn, word size only on the 68000; the source holds the upper bound
        and may use any data mode. The low word of Dn is compared, as a signed number, with 0
        and with the bound. If it is below 0 or above the bound, the CPU takes the CHK
        exception (vector 6); otherwise execution simply continues. It is a cheap guard for an
        array index, and the exception case is much slower than the normal one.
    .flags = N is set if Dn was below 0 and cleared if Dn was above the bound; when no exception
        occurs it is undefined. Z, V and C are undefined and X is unchanged.

isa-m68k-clr = Sets the destination to zero.
    .details = Syntax: CLR.B/W/L <ea>, with a data register or an alterable memory destination;
        to clear an address register, use SUBA.L An,An instead. N, V and C are cleared, Z is set
        and X is unchanged. On the 68000, CLR reads its memory destination before writing it,
        which matters for some hardware registers. MOVEQ #0,Dn clears a whole data register
        faster than CLR.L Dn.

isa-m68k-cmp = Compares a data register with the source.
    .details = Syntax: CMP.B/W/L <ea>,Dn; the source may use any mode (An only in word or long
        size). The CPU subtracts the source from Dn and keeps only the flags, so that a
        conditional branch can follow: after CMP src,Dn, BGT branches if Dn is greater than src
        as signed numbers and BHI if it is greater as unsigned numbers. N, Z, V and C are set
        from the subtraction and X is unchanged. A68k also accepts CMP for CMPA, CMPI and CMPM
        and picks the encoding from the operands.

isa-m68k-cmpa = Compares an address register with the source.
    .details = Syntax: CMPA.W/L <ea>,An; the source may use any mode. A word source is
        sign-extended to 32 bits and the comparison always uses the whole address register.
        N, Z, V and C are set as by CMP and X is unchanged; for addresses, the unsigned
        conditions (HI, LS, CC, CS) are the meaningful ones.

isa-m68k-cmpi = Compares the destination with an immediate value.
    .details = Syntax: CMPI.B/W/L #<data>,<ea>; on the 68000 the destination is a data register
        or an alterable memory operand. The immediate value is subtracted from the destination
        and only the flags are kept: N, Z, V and C are set and X is unchanged. To compare an
        address register with a constant, use CMPA instead.

isa-m68k-cmpm = Compares two memory operands and advances both pointers.
    .details = Syntax: CMPM.B/W/L (Ay)+,(Ax)+. The source is subtracted from the destination
        and only the flags are kept, then both address registers move past the data they read.
        It is made for loops that compare two strings or buffers, often closed by DBNE. N, Z, V
        and C are set and X is unchanged.

isa-m68k-dbcc = Loop control: decrements a counter and branches until the condition holds or the counter runs out.
    .details = Syntax: DBcc Dn,<label>, where cc is any of the 16 conditions; DBF is also
        written DBRA. If the condition is true, the loop ends and execution falls through.
        Otherwise the low word of Dn is decremented, and the CPU branches to the label unless
        the counter has just reached -1; a DBRA loop whose counter starts at n therefore runs
        n + 1 times. Only the low word of Dn changes, and no flag is affected. The label must
        lie within about 32 KB, and each repetition (branch taken) costs 10 clock periods.

isa-m68k-divs = Signed division of a 32-bit register by a 16-bit source.
    .details = Syntax: DIVS <ea>,Dn, word size; the source may use any data mode. The 32-bit
        value of Dn is divided by the signed source word: the quotient goes to the low word of
        Dn and the remainder, which has the sign of the dividend, to the high word. If the
        quotient does not fit in 16 bits, V is set and Dn is left unchanged; a zero divisor
        raises the divide-by-zero exception (vector 5). N and Z describe the quotient (they are
        undefined after an overflow), C is always cleared and X is unchanged. It is one of the
        slowest instructions, and the tables give its worst case.

isa-m68k-divu = Unsigned division of a 32-bit register by a 16-bit source.
    .details = Syntax: DIVU <ea>,Dn, word size; the source may use any data mode. The 32-bit
        value of Dn is divided by the unsigned source word: the quotient goes to the low word
        of Dn and the remainder to the high word. If the quotient does not fit in 16 bits, V is
        set and Dn is left unchanged; a zero divisor raises the divide-by-zero exception
        (vector 5). N and Z describe the quotient (they are undefined after an overflow), C is
        always cleared and X is unchanged. It is one of the slowest instructions, and the
        tables give its worst case.

isa-m68k-eor = Bitwise exclusive OR of a data register into the destination.
    .details = Syntax: EOR.B/W/L Dn,<ea>; the source must be a data register and the destination
        is a data register or an alterable memory operand. A result bit is 1 where the two
        operands differ, so EOR with a mask inverts the chosen bits. A68k also accepts an
        immediate source and then encodes EORI. N and Z are set from the result, V and C are
        cleared and X is unchanged.

isa-m68k-eori = Bitwise exclusive OR of an immediate value into the destination, CCR or SR.
    .details = Syntax: EORI.B/W/L #<data>,<ea> with a data register or an alterable memory
        destination, EORI #<data>,CCR to invert chosen condition flags, or EORI #<data>,SR to
        invert chosen bits of the whole status register. EORI to SR is privileged: in user mode
        it raises a privilege violation exception. In the ordinary form N and Z are set from the
        result, V and C are cleared and X is unchanged.
    .flags = With CCR or SR as destination, each flag whose bit is 1 in the immediate value is
        inverted; the other flags keep their value.

isa-m68k-exg = Exchanges the contents of two registers.
    .details = Syntax: EXG Rx,Ry, with two data registers, two address registers, or one of
        each. All 32 bits are swapped, so the only size is long. No flag is changed, and it
        takes 6 clock periods.

isa-m68k-ext = Sign-extends a data register.
    .details = Syntax: EXT.W Dn copies bit 7 into bits 8 to 15 (byte to word), and EXT.L Dn
        copies bit 15 into bits 16 to 31 (word to long); without a suffix the size is word. It
        prepares a signed byte or word for arithmetic in a wider size: EXT.W followed by EXT.L
        turns a byte into a long word. N and Z are set from the result, V and C are cleared and
        X is unchanged.

isa-m68k-illegal = Always raises the illegal-instruction exception.
    .details = Syntax: ILLEGAL, with no operand and no size. Its opcode ($4AFC) is reserved so
        that it is never a valid instruction: the CPU takes the illegal-instruction exception
        (vector 4) and enters supervisor mode. Programs use it as a deliberate stop, and
        debuggers as a breakpoint. The condition flags are not changed.

isa-m68k-jmp = Jumps to an address.
    .details = Syntax: JMP <ea>, with a control addressing mode. Execution continues at the
        effective address itself, not at a value read from memory: JMP (A0) goes to the address
        held in A0. Unlike BRA it reaches the whole address space, and with an index register
        it implements jump tables. No flag is changed; the time depends on the addressing mode.

isa-m68k-jsr = Calls a subroutine at any address.
    .details = Syntax: JSR <ea>, with a control addressing mode, as for JMP. The address of the
        next instruction is pushed on the stack as a long word, then execution continues at the
        effective address; RTS later returns there. Use it for distant subroutines or for calls
        through a register, such as JSR (A0). No flag is changed; the time depends on the
        addressing mode.

isa-m68k-lea = Loads an effective address into an address register.
    .details = Syntax: LEA <ea>,An, long size, with a control addressing mode. The address of the
        operand is computed and stored in An, but memory is not read: LEA 8(A0),A1 sets A1 to
        A0 + 8. It is the usual way to point a register at a table or a string, including with
        a PC-relative operand such as LEA text(PC),A0. No flag is changed.

isa-m68k-link = Creates a stack frame for a subroutine.
    .details = Syntax: LINK An,#<displacement>, with a signed 16-bit displacement that is
        usually negative. The CPU pushes An, copies the stack pointer into An, then adds the
        displacement to the stack pointer, which reserves room for local variables. Inside the
        subroutine, the arguments are then at positive offsets from An and the locals at
        negative offsets; UNLK removes the frame. No flag is changed, and it takes 16 clock
        periods.

isa-m68k-lsl = Logical shift to the left.
    .details = Syntax: LSL.B/W/L Dx,Dy (count in Dx, modulo 64), LSL.B/W/L #<1-8>,Dy, or LSL <ea>
        to shift a word in alterable memory by one bit. Zeros enter on the right, and each step
        multiplies an unsigned value by 2. X and C receive the last bit shifted out, N and Z
        describe the result, and V is cleared. With a count of 0, C is cleared and X is
        unchanged. The register forms take 2 more clock periods per bit position.

isa-m68k-lsr = Logical shift to the right.
    .details = Syntax: LSR.B/W/L Dx,Dy (count in Dx, modulo 64), LSR.B/W/L #<1-8>,Dy, or LSR <ea>
        to shift a word in alterable memory by one bit. Zeros enter on the left, and each step
        divides an unsigned value by 2. X and C receive the last bit shifted out, N and Z
        describe the result, and V is cleared. With a count of 0, C is cleared and X is
        unchanged. The register forms take 2 more clock periods per bit position.

isa-m68k-move = Copies the source to the destination.
    .details = Syntax: MOVE.B/W/L <ea>,<ea>; without a suffix the size is word. The source may
        use any mode (An only in word or long size) and the destination is a data register or
        an alterable memory operand; A68k turns a move into an address register into MOVEA.
        Other forms load the flags or the whole status register from the source (MOVE to CCR,
        MOVE to SR), copy SR out, and move USP to or from an address register; MOVE to SR and
        both MOVE USP forms are privileged. In the ordinary form N and Z are set
        from the value moved, V and C are cleared and X is unchanged. Memory-to-memory moves
        are allowed, and the time depends on both addressing modes.

isa-m68k-movea = Copies the source to an address register.
    .details = Syntax: MOVEA.W/L <ea>,An; the source may use any mode. A word is sign-extended
        to 32 bits, so the whole register always changes. Unlike MOVE, it affects no flag, so
        loading a pointer does not disturb a pending test. A68k also selects MOVEA when you
        write MOVE with an address register destination.

isa-m68k-movem = Moves several registers to or from memory.
    .details = Syntax: MOVEM.W/L <list>,<ea> to store or MOVEM.W/L <ea>,<list> to load, where a
        list such as D0-D3/A0/A2 names the registers. A store accepts -(An) or a control mode
        other than PC-relative, and a load accepts (An)+ or any control mode, so that
        MOVEM.L D0-D7/A0-A6,-(SP) saves registers and MOVEM.L (SP)+,D0-D7/A0-A6 restores them.
        In memory the registers always appear in the order D0 to A7, from the lowest address
        up, and words loaded into registers are sign-extended to 32 bits. No flag is changed.
        The time grows with the number of registers: 4 clock periods each in word size and 8 in
        long size.

isa-m68k-movep = Moves data between a data register and every other byte of memory.
    .details = Syntax: MOVEP.W/L Dx,d16(Ay) or MOVEP.W/L d16(Ay),Dx. The bytes of the register,
        most significant first, go to or come from the addresses d16(Ay), d16+2(Ay), and so on,
        skipping one byte each time. This suits 8-bit peripherals wired to one half of the
        16-bit data bus. A word transfers 2 bytes and a long word 4. No flag is changed.

isa-m68k-moveq = Loads a small signed constant into a data register.
    .details = Syntax: MOVEQ #<data>,Dn, with a constant from -128 to 127; the size is always
        long. The 8-bit constant is stored inside the opcode and sign-extended to 32 bits, so
        MOVEQ #-1,D0 sets D0 to $FFFFFFFF. It is the fastest way to load a small value, at 4
        clock periods. N and Z are set from the value, V and C are cleared and X is unchanged.

isa-m68k-muls = Signed multiplication of two 16-bit values into a 32-bit result.
    .details = Syntax: MULS <ea>,Dn, word size; the source may use any data mode. The low word
        of Dn is multiplied by the source word, both taken as signed numbers, and the full
        32-bit product replaces Dn, so the result cannot overflow. N and Z are set from the
        product, V and C are cleared and X is unchanged. The time depends on the bit pattern of
        the source: 38 clock periods plus 2 for each pair of differing adjacent bits, at most 70.

isa-m68k-mulu = Unsigned multiplication of two 16-bit values into a 32-bit result.
    .details = Syntax: MULU <ea>,Dn, word size; the source may use any data mode. The low word
        of Dn is multiplied by the source word, both taken as unsigned numbers, and the full
        32-bit product replaces Dn, so the result cannot overflow. N and Z are set from the
        product, V and C are cleared and X is unchanged. The time depends on the source: 38
        clock periods plus 2 for each bit at 1, at most 70.

isa-m68k-nbcd = Negates a packed-BCD byte together with the X flag.
    .details = Syntax: NBCD <ea>, byte size only, with a data register or an alterable memory
        destination. The CPU computes 0 - destination - X in decimal, which gives the ten’s
        complement of a packed-BCD number, or its nine’s complement when X is set. X and C
        receive the decimal borrow.
    .flags = Z is cleared when the result is not zero and left unchanged otherwise, as with ABCD
        and SBCD. N and V are undefined.

isa-m68k-neg = Negates the destination (two’s complement).
    .details = Syntax: NEG.B/W/L <ea>, with a data register or an alterable memory destination.
        The destination is replaced by 0 minus itself, which changes the sign of a signed
        number. X and C are set unless the operand was zero, V is set when the operand is the
        most negative value (such as $80 in a byte, which stays $80), and N and Z describe the
        result.

isa-m68k-negx = Negates the destination together with the X flag.
    .details = Syntax: NEGX.B/W/L <ea>, with a data register or an alterable memory destination.
        The CPU computes 0 - destination - X, so a number stored in several parts is negated
        with NEG on its lowest part and then NEGX on each higher part. X, N, V and C are set
        from the result.
    .flags = Z is cleared when the result is not zero and left unchanged otherwise, so after a
        chain it tells whether the complete result is zero.

isa-m68k-nop = Does nothing.
    .details = Syntax: NOP, with no operand and no size. The CPU simply goes on to the next
        instruction, which takes 4 clock periods and 2 bytes. It reserves room for a later
        patch or adds a short, precise delay. No flag is changed.

isa-m68k-not = Inverts every bit of the destination (one’s complement).
    .details = Syntax: NOT.B/W/L <ea>, with a data register or an alterable memory destination.
        Each bit at 0 becomes 1 and each bit at 1 becomes 0. N and Z are set from the result,
        V and C are cleared and X is unchanged.

isa-m68k-or = Bitwise OR of the source into the destination.
    .details = Syntax: OR.B/W/L <ea>,Dn or OR.B/W/L Dn,<ea>. A result bit is 1 where at least
        one operand has a 1, which is how a mask sets bits. The source of the first form may
        use any data mode; the destination of the second form is an alterable memory operand.
        A68k also accepts an immediate source and then encodes ORI. N and Z are set from the
        result, V and C are cleared and X is unchanged.

isa-m68k-ori = Bitwise OR of an immediate value into the destination, CCR or SR.
    .details = Syntax: ORI.B/W/L #<data>,<ea> with a data register or an alterable memory
        destination, ORI #<data>,CCR to set chosen condition flags, or ORI #<data>,SR to set
        chosen bits of the whole status register, such as the interrupt mask. ORI to SR is
        privileged: in user mode it raises a privilege violation exception. In the ordinary
        form N and Z are set from the result, V and C are cleared and X is unchanged.
    .flags = With CCR or SR as destination, each flag whose bit is 1 in the immediate value is
        set; the other flags keep their value.

isa-m68k-pea = Pushes an effective address on the stack.
    .details = Syntax: PEA <ea>, with a control addressing mode. The address of the operand, not
        its content, is computed and pushed as a long word, and A7 decreases by 4. It is a
        compact way to pass a pointer to a subroutine, for example PEA buffer(PC). No flag is
        changed.

isa-m68k-reset = Resets the external devices (privileged).
    .details = Syntax: RESET, with no operand. The CPU drives its RESET output for 124 clock
        periods so that the peripherals return to their initial state; the registers of the CPU
        are not changed and execution continues with the next instruction. It is privileged:
        in user mode it raises a privilege violation exception. No flag is changed.

isa-m68k-rol = Rotates to the left, without the X flag.
    .details = Syntax: ROL.B/W/L Dx,Dy (count in Dx, modulo 64), ROL.B/W/L #<1-8>,Dy, or ROL <ea>
        to rotate a word in alterable memory by one bit. The bit leaving on the left comes back
        on the right, so no bit is lost. C receives the last bit rotated out, N and Z describe
        the result, V is cleared and X is unchanged; with a count of 0, C is cleared. The
        register forms take 2 more clock periods per bit position.

isa-m68k-ror = Rotates to the right, without the X flag.
    .details = Syntax: ROR.B/W/L Dx,Dy (count in Dx, modulo 64), ROR.B/W/L #<1-8>,Dy, or ROR <ea>
        to rotate a word in alterable memory by one bit. The bit leaving on the right comes
        back on the left, so no bit is lost. C receives the last bit rotated out, N and Z
        describe the result, V is cleared and X is unchanged; with a count of 0, C is cleared.
        The register forms take 2 more clock periods per bit position.

isa-m68k-roxl = Rotates to the left through the X flag.
    .details = Syntax: ROXL.B/W/L Dx,Dy (count in Dx, modulo 64), ROXL.B/W/L #<1-8>,Dy, or
        ROXL <ea> for one bit on a word in alterable memory; A68k also accepts the spelling
        ROLX. X acts as an extra bit: it enters on the right, while the bit leaving on the left
        goes to X and C. Shifting a number made of several parts uses LSL on the lowest part,
        then ROXL on each higher part. N and Z describe the result and V is cleared; with a
        count of 0, C takes the value of X and X is unchanged.

isa-m68k-roxr = Rotates to the right through the X flag.
    .details = Syntax: ROXR.B/W/L Dx,Dy (count in Dx, modulo 64), ROXR.B/W/L #<1-8>,Dy, or
        ROXR <ea> for one bit on a word in alterable memory; A68k also accepts the spelling
        RORX. X acts as an extra bit: it enters on the left, while the bit leaving on the right
        goes to X and C. Shifting a number made of several parts uses LSR or ASR on the highest
        part, then ROXR on each lower part. N and Z describe the result and V is cleared; with
        a count of 0, C takes the value of X and X is unchanged.

isa-m68k-rte = Returns from an exception handler (privileged).
    .details = Syntax: RTE, with no operand. The CPU pops the status register, then the program
        counter, from the supervisor stack, which restores the mode, the interrupt mask and the
        flags saved when the exception began. It is privileged, because it may switch back to
        user mode. It takes 20 clock periods.

isa-m68k-rtr = Returns from a subroutine and restores the condition flags.
    .details = Syntax: RTR, with no operand. The CPU pops a word whose low byte becomes the
        condition code register, then pops the return address into the program counter. It
        pairs with a subroutine that begins with MOVE SR,-(SP), so that the caller gets its
        flags back. It takes 20 clock periods.

isa-m68k-rts = Returns from a subroutine.
    .details = Syntax: RTS, with no operand. The CPU pops the long-word return address pushed by
        BSR or JSR and continues there, and A7 increases by 4. The stack pointer must be back
        where it was right after the call, otherwise the CPU returns to a wrong address. No
        flag is changed; it takes 16 clock periods.

isa-m68k-sbcd = Subtracts packed-BCD bytes together with the X flag.
    .details = Syntax: SBCD Dy,Dx or SBCD -(Ay),-(Ax), byte size only. The CPU computes
        destination - source - X in decimal and stores the result in the destination. As with
        ABCD, the memory form steps backwards through memory, so a chain of SBCD subtracts long
        decimal numbers starting from their last byte. X and C receive the decimal borrow.
    .flags = Z is cleared when the result is not zero and left unchanged otherwise. After a chain
        of SBCD it therefore tells whether the whole number is zero, provided Z was set (and X
        cleared) before the first one. N and V are undefined.

isa-m68k-scc = Sets a byte to all ones when the condition holds, and to zero otherwise.
    .details = Syntax: Scc <ea>, byte size, with a data register or an alterable memory
        destination; cc is any of the 16 conditions, including T (always) and F (never). The
        byte becomes $FF if the condition is true and $00 if it is false, which stores the
        outcome of a test as a boolean value. No flag is changed. On a data register, a true
        condition takes 2 clock periods more than a false one.

isa-m68k-stop = Loads the status register and waits for an interrupt (privileged).
    .details = Syntax: STOP #<data>, with a 16-bit immediate value. The value is copied into the
        status register, then the CPU stops executing instructions until an interrupt above the
        new mask, a trace exception or a reset occurs. It lets a system wait for an event
        without a busy loop. It is privileged: in user mode it raises a privilege violation
        exception.

isa-m68k-sub = Subtracts the source from the destination.
    .details = Syntax: SUB.B/W/L <ea>,Dn or SUB.B/W/L Dn,<ea>; the destination becomes
        destination minus source. One operand must be a data register: the source of the first
        form may use any mode (An only in word or long size), the destination of the second
        form is an alterable memory operand. A68k also accepts SUB with an address register
        destination (SUBA) or an immediate source (SUBI). X, N, Z, V and C are set from the
        result; C and X signal an unsigned borrow.

isa-m68k-suba = Subtracts the source from an address register.
    .details = Syntax: SUBA.W/L <ea>,An; the source may use any mode. A word source is
        sign-extended to 32 bits first, and the whole address register always changes. No flag
        is affected. SUBA.L An,An is a quick way to clear an address register.

isa-m68k-subi = Subtracts an immediate value from the destination.
    .details = Syntax: SUBI.B/W/L #<data>,<ea>; the destination is a data register or an
        alterable memory operand. The constant is stored in the words that follow the opcode.
        X, N, Z, V and C are set from the result. For a constant from 1 to 8, SUBQ is shorter
        and faster.

isa-m68k-subq = Subtracts a constant from 1 to 8 from the destination.
    .details = Syntax: SUBQ.B/W/L #<1-8>,<ea>. The constant is packed into the opcode itself, so
        it needs no extension word and runs quickly. The destination may be any alterable
        operand, including an address register in word or long size. On an address register
        the whole 32 bits change and no flag is affected; otherwise X, N, Z, V and C are set
        from the result.

isa-m68k-subx = Subtracts the source and the X flag from the destination.
    .details = Syntax: SUBX.B/W/L Dy,Dx or SUBX.B/W/L -(Ay),-(Ax). The destination becomes
        destination - source - X, which chains subtractions wider than 32 bits: SUB on the
        lowest parts, then SUBX on each higher part. The memory form predecrements both
        pointers, so it starts from the least significant end. X, N, V and C are set from the
        result.
    .flags = Z is cleared when the result is not zero and left unchanged otherwise. After a chain
        of SUBX it therefore tells whether the complete result is zero, provided Z was set
        before the chain.

isa-m68k-swap = Exchanges the two halves of a data register.
    .details = Syntax: SWAP Dn, word size. Bits 31 to 16 trade places with bits 15 to 0, which
        brings the high word within reach, for example the remainder after a division. N and Z
        are set from the whole 32-bit result, V and C are cleared and X is unchanged. It takes 4
        clock periods.

isa-m68k-tas = Tests a byte and sets its bit 7 in one indivisible operation.
    .details = Syntax: TAS <ea>, byte size, with a data register or an alterable memory
        destination. The CPU reads the byte, sets N and Z from its old value, then writes it
        back with bit 7 set, in a single read-modify-write bus cycle that no other bus master
        can split. This makes it a lock for data shared between processors. V and C are cleared
        and X is unchanged.

isa-m68k-trap = Calls the system through one of 16 trap vectors.
    .details = Syntax: TRAP #<vector>, with a number from 0 to 15. The CPU enters supervisor
        mode, saves the program counter and the status register on the supervisor stack, and
        continues at the handler whose address is in vector 32 + n. Simulators and operating
        systems use it for their services; EASy68K, for example, provides its input and output
        through TRAP #15. The condition flags are not changed, and it takes 34 clock periods.

isa-m68k-trapv = Raises an exception if the overflow flag is set.
    .details = Syntax: TRAPV, with no operand. If V is set, the CPU takes the TRAPV exception
        (vector 7); otherwise it continues with the next instruction, as fast as a NOP. Place it
        after a signed operation to catch an overflow. No flag is changed.

isa-m68k-tst = Compares the operand with zero.
    .details = Syntax: TST.B/W/L <ea>; on the 68000 the operand is a data register or an
        alterable memory operand. The operand is not changed: N and Z describe its value, V and
        C are cleared and X is unchanged, so a conditional branch such as BEQ, BMI or BPL can
        follow.

isa-m68k-unlk = Removes a stack frame created by LINK.
    .details = Syntax: UNLK An, with the register of the matching LINK. The stack pointer is set
        to An, which frees the local variables, then the saved value of An is popped back into
        An. It usually comes right before RTS. No flag is changed, and it takes 12 clock
        periods.

## Conditions (cc in Bcc, DBcc and Scc; after CMP src,dst they compare dst with src)

isa-m68k-cond-t = always true
isa-m68k-cond-f = always false
isa-m68k-cond-hi = higher: dst is greater than src as unsigned numbers
isa-m68k-cond-ls = lower or same: dst is less than or equal to src as unsigned numbers
isa-m68k-cond-cc = carry clear, also written HS (higher or same): dst is greater than or equal to src as unsigned numbers
isa-m68k-cond-cs = carry set, also written LO (lower): dst is less than src as unsigned numbers
isa-m68k-cond-ne = not equal: dst differs from src, or the result is not zero
isa-m68k-cond-eq = equal: dst equals src, or the result is zero
isa-m68k-cond-vc = overflow clear: no signed overflow occurred
isa-m68k-cond-vs = overflow set: a signed overflow occurred
isa-m68k-cond-pl = plus: the result is positive or zero
isa-m68k-cond-mi = minus: the result is negative
isa-m68k-cond-ge = greater or equal: dst is greater than or equal to src as signed numbers
isa-m68k-cond-lt = less than: dst is less than src as signed numbers
isa-m68k-cond-gt = greater than: dst is greater than src as signed numbers
isa-m68k-cond-le = less or equal: dst is less than or equal to src as signed numbers

## Timing outcomes

isa-m68k-outcome-taken = branch taken
isa-m68k-outcome-not-taken = branch not taken
isa-m68k-outcome-true = condition true
isa-m68k-outcome-false = condition false
isa-m68k-outcome-not-expired = condition false, counter not exhausted: the loop goes on
isa-m68k-outcome-expired = condition false, counter exhausted: the loop ends
isa-m68k-outcome-trap = exception taken
isa-m68k-outcome-no-trap = no exception

## Timing units (n in the per-unit costs)

isa-m68k-unit-shift-count = bit positions shifted or rotated
isa-m68k-unit-register-count = registers transferred
isa-m68k-unit-one-bits = bits at 1 in the source word
isa-m68k-unit-bit-pairs = pairs of differing adjacent bits in the source word, with a 0 appended on its right
