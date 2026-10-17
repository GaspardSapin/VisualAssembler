* basic.asm: a small complete program, every common field
; a semicolon comment line
start	move.l	#$1234,d0	load a constant
	moveq	#0,d1
loop:	add.w	d0,d1
	dbf	d2,loop
	lea	table(pc),a0
	move.b	(a0)+,d3
	move.w	-(a1),d4
	move.l	4(a2),d5
	move.l	2(a3,d6.w),d7
	move.w	$1000,d0
	move.l	$12345678,d1
	bsr	sub
	bra.s	done
sub	nop
	rts
done	illegal

table	dc.b	1,2,3
	dc.w	$1234
	dc.l	table
	even
	end	start
