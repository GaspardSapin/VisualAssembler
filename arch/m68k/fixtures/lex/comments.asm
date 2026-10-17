* comments.asm: the m1 §5.7 table (rows indented), comments and suspect comments
 dc.b 1, 2, 3
 move.l d0, d1
	move.l	d0,	d1
 dc.b 1 ,2
 dc.b 1, 2 ; three
 dc.b 'a, b', 2
 dc.b 1 + 2
 move.l #1 + 2,d0
 lea 4( a0),a1
 movem.l d0-d7, -(sp)
 dc.b 1,2 3,4 swapped
 dc.b 'a b',2
 move.l d0,d1 ; real comment
 move.l d0,d1 free text
 move.l d0,d1 * comment
	move.l	d0,d1	free text
 rts d0, d1
* dc.b 1, 2
 dc.b 1 - 2
; a whole-line comment
lab ; after a label
	nop	; after a no-operand mnemonic
	even	free text after a directive
