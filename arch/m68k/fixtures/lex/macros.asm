* macros.asm: macro definitions, parameters and calls
swap2	macro
	move.\0	\1,d7
	move.\0	\2,\1
	move.\0	d7,\2
	endm
count	macro
lbl\@:	dbf	\1,lbl\@
	mexit
	endm
	swap2.w	d0,d1
	swap2	(a0),(a1)
	count	d2
\1	nop
	b\1	target
	ifeq	1
	else
	endc
	rept	3
	opt	x
	dc.b	\@,\9
