* literals.asm: numbers, strings, registers, operators, paths
	dc.b	$1F,%1010,@17,42
	dc.b	'abc','it''s',"say ""hi"""
	dc.b	'a\nb'
	dc.b	'é',"日本"
	dc.b	''
	move.w	($1000).w,d0
	move.l	$1000.l,d1
	move.w	(a0,d0.l),d1
	move.l	(sp)+,usp
	andi	#$1F,ccr
	move	sr,d0
	lea	8(pc),a0
	dc.l	1$,\local
	dc.l	1+2-3*4/5
	dc.l	~1&2!3|4
	dc.l	1<<2>>3
	dc.l	*
	dc.l	*-start
	move.l	#-1,d0
	include	"file.i"
	include	dir\sub:file.i
	incbin	'data.bin'
	dc.b	CODE,NARG,CHIP
