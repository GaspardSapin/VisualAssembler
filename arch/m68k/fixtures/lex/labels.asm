* labels.asm: column-1 labels, indented labels, local labels, label errors
lab
lab:
lab	nop
lab:	nop
nop
  name: nop
	other:
  1$: bra.s 1$
1$	nop
12$:	bra	12$
\loop	nop
	bra	\loop
_under.score@1	nop
.dot	nop
x = 5
y	equ	6
	=	7
lab:nop
9lab	nop
-x	nop
  name:nop
lab::	nop
lab:::
  other:: nop
