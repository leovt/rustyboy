; set stack pointer
	LD SP, 0xfffe

; clear 0x8000 to 0x9fff
	XOR A
	LD HL, 0x9fff
ClearLoop:
	LD (HL-), A
	BIT 7, H
	JR NZ, ClearLoop

; setup audio registers
	LD HL, 0xff26
	LD C, 0x11
	LD A, 0x80
	LD (HL-), A
	LD (0xFF00+C), A
	INC C
	LD A, 0xf3
	LD (0xFF00+C), A
	LD (HL-), A
	LD A,0x77
	LD (HL), A

; setup graphics palette
	LD A, 0xfc
	LD (0xFF00+0x47), A

; decompress logo into video RAM (tile 0 = empty, load tiles 1 to 34)
	LD DE, LogoData
	LD HL, 0x8010 ; Tile #0 (0x8000) remains zero, start with tile #1
DecompressLoop:
	LD A, (DE)
	CALL DecompressTile1
	CALL DecompressTile2
	INC DE
	LD A, E
	CP 0xdd ; data from a9 to dd
	JR NZ, DecompressLoop

; decompress (R) symbol
	LD DE, SignData
	LD B, 0x08
DecompressLoop2:
	LD A,(DE)
	INC DE
	LD (HL+),A
	INC HL
	DEC B
	JR NZ, DecompressLoop2

; write tile map
	LD A,0x1b
	LD (0x9910),A
	LD HL,0x992f
WriteRow:
	LD C,0x0d
WriteRowLoop:
	DEC A
	JR Z, ScrollLogo
	LD (HL-),A
	DEC C
	JR NZ, WriteRowLoop
	LD HL,0x990f
	JR WriteRow

; === Scroll logo on screen, and play logo sound===

ScrollLogo:
	LD H,A ; here A==0
	LD A,0x64
	LD D,A
	LD (0xFF00+0x42),A	; 0xFF42=SCY  vertical scroll register
	LD A,0x91
	LD (0xFF00+0x40),A	; 0xFF40=LCDC lcd control register: Turn on LCD, showing Background

; the loop is run twice, once with B=1 (the logo actually scrolls) and
; then with B=0 (logo in final position without scrolling)
	INC B

ScrollLogoLoop:
	LD E,0x02		; 0x0060
ScrollInner:
	LD C,0x0c		; 0x0062
WaitVSync:
	LD A,(0xFF00+0x44) ; 0xFF44=LY current y-coordinate of lcd; 0x90 is start of VSYNC
	CP 0x90
	JR NZ, WaitVSync
	DEC C
	JR NZ, WaitVSync

	DEC E
	JR NZ, ScrollInner	; 0x006e

	LD C, 0x13
	INC H
	LD A, H
	LD E, 0x83
	CP 0x62
	; when H==62 then play sound (E=62)
	JR Z, PlaySound
	LD E,0xc1
	CP 0x64
	; when H==64 then play sound (E=C1)
	; i.e. when H<>64 skip play sound
	JR NZ, SkipSound

PlaySound:
	LD A, E
	LD (0xFF00+C), A
	INC C
	LD A, 0x87
	LD (0xFF00+C), A

SkipSound:
  ; if B==1 then decrement SCY
	LD A, (0xFF00+0x42)	; SCY
	SUB B
	LD (0xFF00+0x42), A
	DEC D
	JR NZ, ScrollLogoLoop

	DEC B
	; when D==0 and B==-1 then end the logo scroll loop.
	JR NZ, CardridgeChecksum
  ; otherwise restart loop with B=0; D=20 for static logo
	LD D,0x20
	JR ScrollLogoLoop

; ==========================================================
; decopress logo subroutines
; DecompressTile2 is called immediately after Decompress1 has returned.
DecompressTile1:
	LD C,A
DecompressTile2:
	LD B,0x04
DecompressTileLoop:
	PUSH BC
	RL C
	RLA
	POP BC
	RL C
	RLA
	DEC B
	JR NZ, DecompressTileLoop
	LD (HL+), A
	INC HL
	LD (HL+), A
	INC HL
	RET


LogoData:
	;Rustyboy Logo 26 tiles = 52 bytes = 0x34
	.DB 118, 102, 198, 102, 0, 12, 0, 12, 0, 15, 3, 115, 0, 134, 0, 6, 204, 207, 0, 8, 0, 15, 0, 3, 0, 3
  .DB 118, 102, 198, 102, 204, 199, 220, 200, 143, 31, 51, 179, 99, 7, 110, 108, 204, 207, 221, 216, 153, 159, 185, 131, 63, 62

SignData:
	.DB 0x3C,0x42,0xA5,0x81,0xA5,0x99,0x42,0x3C

; Filler
  .DB 0,0,0,0,0,0,0,0,0

CardridgeChecksum:
  LD HL, 0x0134
	LD B, 0x19
	LD A, B
ChecksumLoop:
	ADD (HL)
	INC HL
	DEC B
	JR NZ, ChecksumLoop	;
	ADD (HL)

LockUp:
	JR NZ, LockUp	; if 0x19 + bytes from 0x0134-0x014D  don't add to 0x00 lock up

	LD A, 0x01
	LD (0xFF00+0x50), A	;turn off boot rom
; game starts here

Main:
