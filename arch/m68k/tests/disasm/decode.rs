//! Decoding (`src/disasm/decode.rs`; m2 §5.1, §10.1): golden decodes per mode and family,
//! `Truncated` and `Illegal`, and all 65,536 words: no panic, valid iff some form matches (WP-E).
//! Also the golden files `fixtures/disasm/{modes,illegal}.{s68,dis}` and the helpers the other
//! modules share (decoding words, the table oracle, the golden-file renderer).
//!
//! **Golden files** (m2 §10.2): each `<name>.s68` is read with `asm::srec::parse_srec`; every
//! segment is decoded from its start, as `va disasm` does (§8.3): an `Illegal` or `Truncated`
//! word is written `dc.w $XXXX` and decoding resumes 2 bytes later, a trailing odd byte is
//! `dc.b $XX`. `<name>.dis` holds one `segment $AAAAAA <n> bytes` line per segment, then one
//! line per item: `AAAAAA  <words, padded to 24>  <text, padded to 32>  ; <note>`, the note being
//! the flow id (with ` return=$XXXXXXXX` for a call) and ` target=$XXXXXXXX` when there is a
//! target, or `illegal`, or `truncated needed=<n>`. `VA_BLESS=1 cargo test -p va-arch-m68k
//! --test disasm` rewrites the `.dis` files; the diff is reviewed in git.
#![allow(clippy::unwrap_used)]

use std::fmt::Write as _;
use std::path::PathBuf;

use va_arch_m68k::M68K;
use va_arch_m68k::asm::srec::parse_srec;
use va_arch_m68k::disasm::M68kDisassembler;
use va_arch_m68k::disasm::decode::form_of;
use va_arch_m68k::isa::{Form, INSTRUCTIONS, Instruction};
use va_core::arch::Architecture as _;
use va_core::disasm::{DecodeError, DecodedInsn, Disassembler as _, FlowKind};
use va_core::image::ProgramImage;
use va_core::syntax::check_line_tokens;

/// The address the inline cases are decoded at.
pub const ORIGIN: u64 = 0x1000;

/// Returns the big-endian bytes of `words`.
pub fn bytes_of(words: &[u16]) -> Vec<u8> {
    words.iter().flat_map(|word| word.to_be_bytes()).collect()
}

/// Decodes `words` at `address`.
pub fn decode_words(words: &[u16], address: u64) -> Result<DecodedInsn, DecodeError> {
    M68kDisassembler.decode(&bytes_of(words), address)
}

/// Decodes `words` at [`ORIGIN`] and returns the instruction; panics on an error.
pub fn insn(words: &[u16]) -> DecodedInsn {
    decode_words(words, ORIGIN).unwrap_or_else(|error| panic!("{words:04X?}: {error:?}"))
}

/// Decodes `words` at [`ORIGIN`] and returns the text.
pub fn text(words: &[u16]) -> String {
    insn(words).text
}

/// One (entry, form index, form, condition) of the tables: a family form once per condition
/// code of its family (as the M1 uniqueness test counts them).
pub type Candidate = (&'static Instruction, usize, &'static Form, Option<u8>);

/// Every candidate of the tables.
pub fn candidates() -> Vec<Candidate> {
    let mut out = Vec::new();
    for entry in &INSTRUCTIONS {
        for (index, form) in entry.forms.iter().enumerate() {
            match entry.family {
                Some(family) => {
                    for code in family.first_condition()..16 {
                        out.push((entry, index, form, Some(code)));
                    }
                }
                None => out.push((entry, index, form, None)),
            }
        }
    }
    out
}

/// The candidates `opword` matches, found by `Form::matches` alone (the oracle of the table).
pub fn matching(candidates: &[Candidate], opword: u16) -> Vec<Candidate> {
    candidates
        .iter()
        .filter(|(_, _, form, condition)| form.matches(opword, *condition))
        .copied()
        .collect()
}

/// The number of valid 68000 opwords (`tests/isa/encoding.rs`, `VALID_OPWORDS`).
pub const VALID_OPWORDS: usize = 45_816;

/// Returns the folder of the disassembler fixtures, `arch/m68k/fixtures/disasm/`.
pub fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("disasm")
}

/// Formats bytes as big-endian words (`4E71 0102`; a last odd byte as `4E`).
fn hex_words(bytes: &[u8]) -> String {
    let words: Vec<String> = bytes
        .chunks(2)
        .map(|chunk| chunk.iter().map(|byte| format!("{byte:02X}")).collect())
        .collect();
    words.join(" ")
}

/// Writes one item line of a `.dis` file.
fn item_line(out: &mut String, address: u64, bytes: &[u8], text: &str, note: &str) {
    writeln!(
        out,
        "{address:06X}  {:<24}  {text:<32}  ; {note}",
        hex_words(bytes)
    )
    .unwrap();
}

/// The note of a decoded instruction: its flow id, return address and target.
fn note(insn: &DecodedInsn) -> String {
    let mut note = insn.flow.id().to_owned();
    if let FlowKind::Call { return_address } = insn.flow {
        write!(note, " return=${return_address:08X}").unwrap();
    }
    if let Some(target) = insn.target {
        write!(note, " target=${target:08X}").unwrap();
    }
    note
}

/// Renders the decoded items of every segment of `image` in the `.dis` format, checking on the
/// way that each instruction's tokens cover its text and that `flow_kind` gives its flow.
pub fn render(image: &ProgramImage) -> String {
    let disassembler = M68K.disassembler().unwrap();
    let mut out = String::new();
    for segment in image.segments() {
        let bytes = &segment.bytes;
        writeln!(
            out,
            "segment ${:06X} {} bytes",
            segment.address,
            bytes.len()
        )
        .unwrap();
        let mut offset = 0;
        while offset < bytes.len() {
            let address = segment.address + offset as u64;
            let rest = &bytes[offset..];
            match disassembler.decode(rest, address) {
                Ok(insn) => {
                    let len = insn.len as usize;
                    assert_eq!(insn.address, address);
                    assert_eq!(disassembler.flow_kind(&insn), insn.flow);
                    check_line_tokens(&insn.text, &insn.tokens)
                        .unwrap_or_else(|error| panic!("{:?}: {error:?}", insn.text));
                    item_line(&mut out, address, &rest[..len], &insn.text, &note(&insn));
                    offset += len;
                }
                Err(error) => {
                    let note = match error {
                        DecodeError::Illegal { .. } => "illegal".to_owned(),
                        DecodeError::Truncated { needed } => format!("truncated needed={needed}"),
                    };
                    if let [byte] = rest {
                        item_line(&mut out, address, rest, &format!("dc.b ${byte:02X}"), &note);
                        offset += 1;
                    } else {
                        let word = u16::from_be_bytes([rest[0], rest[1]]);
                        item_line(
                            &mut out,
                            address,
                            &rest[..2],
                            &format!("dc.w ${word:04X}"),
                            &note,
                        );
                        offset += 2;
                    }
                }
            }
        }
    }
    out
}

/// Checks that `fixtures/disasm/<name>.s68` renders to `<name>.dis` (or rewrites the `.dis`
/// file with `VA_BLESS=1`).
pub fn check_golden(name: &str) {
    let source_path = fixture_dir().join(format!("{name}.s68"));
    let source = std::fs::read_to_string(&source_path)
        .unwrap_or_else(|error| panic!("{}: {error}", source_path.display()));
    assert!(!source.contains('\r'), "{name}.s68 must be LF-only");
    let image = parse_srec(&source).unwrap_or_else(|error| panic!("{name}.s68: {error:?}"));
    let rendered = render(&image);
    let path = fixture_dir().join(format!("{name}.dis"));
    if std::env::var_os("VA_BLESS").is_some_and(|value| value == "1") {
        std::fs::write(&path, &rendered).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{}: {error} (run with VA_BLESS=1)", path.display()));
    if rendered != expected {
        let first = rendered
            .lines()
            .zip(expected.lines())
            .position(|(got, want)| got != want)
            .unwrap_or_else(|| rendered.lines().count().min(expected.lines().count()));
        panic!(
            "{name}.dis differs at line {}: got {:?}, expected {:?}\nreview, then rerun with \
             VA_BLESS=1",
            first + 1,
            rendered.lines().nth(first),
            expected.lines().nth(first)
        );
    }
}

/// Golden decodes at `$1000`: (words, text). The opwords of m1 §4.7 (observed in a68k listings)
/// first, then every addressing mode as a source and as a destination.
#[rustfmt::skip]
const GOLDEN: &[(&[u16], &str)] = &[
    (&[0x4E71], "nop"),
    (&[0x4E75], "rts"),
    (&[0x4AFC], "illegal"),
    (&[0x747F], "moveq.l #127,d2"),
    (&[0x2200], "move.l d0,d1"),
    (&[0x3040], "movea.w d0,a0"),
    (&[0x51C8, 0xFFFE], "dbra.w d0,$00001000"),
    (&[0x64D8], "bcc.s $00000FDA"),
    (&[0x6500, 0x0102], "bcs.w $00001104"),
    (&[0x60DC], "bra.s $00000FDE"),
    (&[0x61C4], "bsr.s $00000FC6"),
    (&[0x55C0], "scs.b d0"),
    (&[0xB308], "cmpm.b (a0)+,(a1)+"),
    (&[0xC308], "abcd.b -(a0),-(a1)"),
    (&[0xD380], "addx.l d0,d1"),
    (&[0xC141], "exg.l d0,d1"),
    (&[0xC188], "exg.l d0,a0"),
    (&[0x4840], "swap.w d0"),
    (&[0x4880], "ext.w d0"),
    (&[0x48C0], "ext.l d0"),
    (&[0x43D0], "lea.l (a0),a1"),
    (&[0x4E68], "move.l usp,a0"),
    (&[0x4E60], "move.l a0,usp"),
    (&[0x44C0], "move.w d0,ccr"),
    (&[0x40C0], "move.w sr,d0"),
    (&[0x007C, 0x2700], "ori.w #$2700,sr"),
    (&[0x023C, 0x00FE], "andi.b #$FE,ccr"),
    (&[0x0A3C, 0x0001], "eori.b #$01,ccr"),
    (&[0x4190], "chk.w (a0),d0"),
    (&[0x48E7, 0xFFFE], "movem.l d0-d7/a0-a6,-(a7)"),
    (&[0x4CDF, 0x7FFF], "movem.l (a7)+,d0-d7/a0-a6"),
    (&[0x4C9F, 0x0003], "movem.w (a7)+,d0-d1"),
    (&[0x0108, 0x0004], "movep.w 4(a0),d0"),
    (&[0x5240], "addq.w #1,d0"),
    (&[0x5181], "subq.l #8,d1"),
    (&[0x0681, 0x1234, 0x5678], "addi.l #$12345678,d1"),
    (&[0xD0C0], "adda.w d0,a0"),
    (&[0x0C50, 0x1234], "cmpi.w #$1234,(a0)"),
    (&[0xE348], "lsl.w #1,d0"),
    (&[0xE3D0], "lsl.w (a0)"),
    (&[0xE362], "asl.w d1,d2"),
    (&[0x0800, 0x0005], "btst.l #5,d0"),
    (&[0x0310], "btst.b d1,(a0)"),
    (&[0x08D0, 0x0012], "bset.b #18,(a0)"),
    (&[0x4E56, 0xFFF0], "link.w a6,#-16"),
    (&[0x4E4F], "trap #15"),
    (&[0x4E72, 0x2700], "stop #$2700"),
    (&[0x4A40], "tst.w d0"),
    (&[0x4250], "clr.w (a0)"),
    // Every mode as a source.
    (&[0x3001], "move.w d1,d0"),
    (&[0x3009], "move.w a1,d0"),
    (&[0x3012], "move.w (a2),d0"),
    (&[0x301B], "move.w (a3)+,d0"),
    (&[0x3024], "move.w -(a4),d0"),
    (&[0x302D, 0xFFFE], "move.w -2(a5),d0"),
    (&[0x3036, 0x1010], "move.w 16(a6,d1.w),d0"),
    (&[0x3038, 0x1234], "move.w ($1234).w,d0"),
    (&[0x3039, 0x0001, 0x2345], "move.w $00012345,d0"),
    (&[0x303A, 0x0102], "move.w $00001104(pc),d0"),
    (&[0x303B, 0xA8F0], "move.w $00000FF2(pc,a2.l),d0"),
    (&[0x303C, 0x1234], "move.w #$1234,d0"),
    (&[0x103C, 0x00FF], "move.b #$FF,d0"),
    (&[0x203C, 0x1234, 0x5678], "move.l #$12345678,d0"),
    // Every alterable mode as a destination.
    (&[0x3200], "move.w d0,d1"),
    (&[0x3080], "move.w d0,(a0)"),
    (&[0x30C0], "move.w d0,(a0)+"),
    (&[0x3100], "move.w d0,-(a0)"),
    (&[0x3140, 0x0008], "move.w d0,8(a0)"),
    (&[0x3180, 0x7004], "move.w d0,4(a0,d7.w)"),
    (&[0x31C0, 0x0400], "move.w d0,($0400).w"),
    (&[0x33C0, 0x00FF, 0xFFFE], "move.w d0,$00FFFFFE"),
    // Both extension words of a MOVE, in operand order.
    (&[0x23FC, 0x1234, 0x5678, 0x0001, 0x2345], "move.l #$12345678,$00012345"),
    (&[0x31BA, 0x0102, 0x1002], "move.w $00001104(pc),2(a0,d1.w)"),
];

#[test]
fn golden_decodes_per_mode_and_family() {
    for &(words, expected) in GOLDEN {
        let insn = insn(words);
        assert_eq!(insn.text, expected, "{words:04X?}");
        assert_eq!(insn.len as usize, 2 * words.len(), "{expected}");
        assert_eq!(insn.address, ORIGIN, "{expected}");
    }
}

#[test]
fn fewer_than_two_bytes_are_truncated_at_two() {
    for bytes in [&[][..], &[0x4E][..], &[0xFF][..]] {
        assert_eq!(
            M68kDisassembler.decode(bytes, ORIGIN),
            Err(DecodeError::Truncated { needed: 2 }),
            "{bytes:02X?}"
        );
    }
}

#[test]
fn cut_extension_words_are_truncated_at_the_full_length() {
    // jsr $00012345: 6 bytes.
    let jsr = bytes_of(&[0x4EB9, 0x0001, 0x2345]);
    for cut in 2..6 {
        assert_eq!(
            M68kDisassembler.decode(&jsr[..cut], ORIGIN),
            Err(DecodeError::Truncated { needed: 6 }),
            "{cut} bytes"
        );
    }
    assert_eq!(M68kDisassembler.decode(&jsr, ORIGIN).unwrap().len, 6);
    // move.l #$12345678,$00012345: 10 bytes, the longest 68000 instruction.
    let longest = bytes_of(&[0x23FC, 0x1234, 0x5678, 0x0001, 0x2345]);
    assert_eq!(
        M68kDisassembler.decode(&longest[..9], ORIGIN),
        Err(DecodeError::Truncated { needed: 10 })
    );
    // An empty MOVEM mask still counts its words: opword, mask, ($1234).w.
    assert_eq!(
        decode_words(&[0x4CB8, 0x0000], ORIGIN),
        Err(DecodeError::Truncated { needed: 6 })
    );
}

#[test]
fn unmatched_opwords_are_illegal_even_when_bytes_are_missing() {
    // Line F, line A, TAS #imm, RTD and MOVEC (68010), EXTB.L and MULU.L (68020).
    for word in [
        0xFFFF, 0xF000, 0xA000, 0x4AFB, 0x4E74, 0x4E7A, 0x49C0, 0x4C00,
    ] {
        for words in [&[word][..], &[word, 0x1234, 0x5678][..]] {
            assert_eq!(
                decode_words(words, ORIGIN),
                Err(DecodeError::Illegal { word }),
                "{word:04X}"
            );
        }
        assert_eq!(
            M68kDisassembler.decode(&word.to_be_bytes(), 0),
            Err(DecodeError::Illegal { word })
        );
    }
}

#[test]
fn every_opword_decodes_and_is_valid_exactly_when_a_form_matches() {
    let candidates = candidates();
    let mut valid = 0;
    for opword in 0..=u16::MAX {
        let found = matching(&candidates, opword);
        assert!(found.len() <= 1, "{opword:04X} matches twice");
        let mut words = vec![opword];
        words.extend([0x1002; 4]);
        let decoded = decode_words(&words, ORIGIN);
        let Some(&(entry, form, _, condition)) = found.first() else {
            assert_eq!(decoded, Err(DecodeError::Illegal { word: opword }));
            assert_eq!(form_of(opword), None, "{opword:04X}");
            continue;
        };
        valid += 1;
        let at = format!("{opword:04X} ({} form {form})", entry.name);
        let reference = form_of(opword).unwrap_or_else(|| panic!("{at}: no form"));
        assert_eq!(reference.entry.name, entry.name, "{at}");
        assert_eq!(reference.form, form, "{at}");
        assert_eq!(reference.condition, condition, "{at}");
        assert!(std::ptr::eq(reference.form().unwrap(), &entry.forms[form]));
        let insn = decoded.unwrap_or_else(|error| panic!("{at}: {error:?}"));
        assert!(
            insn.len >= 2 && insn.len <= 10 && insn.len.is_multiple_of(2),
            "{at}"
        );
        assert!(!insn.text.is_empty(), "{at}");
        check_line_tokens(&insn.text, &insn.tokens)
            .unwrap_or_else(|error| panic!("{at}: {:?}: {error:?}", insn.text));
        // The length follows from the opword alone; cut words are `Truncated` at it.
        let bytes = bytes_of(&words);
        let len = insn.len as usize;
        for cut in [2, len - 1] {
            if cut < len {
                assert_eq!(
                    M68kDisassembler.decode(&bytes[..cut], ORIGIN),
                    Err(DecodeError::Truncated { needed: insn.len }),
                    "{at}: {cut} bytes"
                );
            }
        }
        assert_eq!(
            M68kDisassembler.decode(&bytes[..len], ORIGIN),
            Ok(insn.clone())
        );
        // Arbitrary extension words never change the length nor make the word illegal.
        for fill in [0x0000, 0xFFFF, 0x8F7F] {
            let mut other = vec![opword];
            other.extend([fill; 4]);
            let decoded = decode_words(&other, ORIGIN).unwrap_or_else(|e| panic!("{at}: {e:?}"));
            assert_eq!(decoded.len, insn.len, "{at}: fill {fill:04X}");
            check_line_tokens(&decoded.text, &decoded.tokens).unwrap();
        }
    }
    assert_eq!(valid, VALID_OPWORDS);
}

#[test]
fn far_addresses_never_panic() {
    // A branch and a PC-relative operand near both ends of the address space.
    for address in [0, 1, 0xFFFF_FFFE, u64::MAX - 1, u64::MAX] {
        for words in [
            &[0x60FE][..],
            &[0x6000, 0x8000][..],
            &[0x41FA, 0x7FFF][..],
            &[0x4EBB, 0x80FF][..],
            &[0x61FE][..],
        ] {
            let insn = decode_words(words, address).unwrap();
            assert_eq!(insn.address, address);
            assert!(insn.target.is_some_and(|target| target <= 0xFFFF_FFFF));
        }
    }
    // At address 0, bra.s -4 lands below 0: the 32-bit program counter wraps.
    assert_eq!(text_at(&[0x60FC], 0), "bra.s $FFFFFFFE");
    assert_eq!(text_at(&[0x41FA, 0x8000], 0), "lea.l $FFFF8002(pc),a0");
}

/// Decodes `words` at `address` and returns the text.
fn text_at(words: &[u16], address: u64) -> String {
    decode_words(words, address).unwrap().text
}

#[test]
fn non_canonical_words_are_read_as_the_68000_reads_them() {
    // The high byte of a .B immediate, of a CCR immediate and of a bit number is ignored.
    assert_eq!(text(&[0x0600, 0xFF12]), "addi.b #$12,d0");
    assert_eq!(text(&[0x103C, 0xAB80]), "move.b #$80,d0");
    assert_eq!(text(&[0x023C, 0xFF1F]), "andi.b #$1F,ccr");
    assert_eq!(text(&[0x0800, 0xFF05]), "btst.l #5,d0");
    // Bits 10-8 of a brief index word are ignored (68020 scale and full-format bits).
    assert_eq!(text(&[0x3030, 0x1702]), "move.w 2(a0,d1.w),d0");
    assert_eq!(text(&[0x3030, 0x1002]), "move.w 2(a0,d1.w),d0");
    assert_eq!(text(&[0x41FB, 0x9FFE]), "lea.l $00001000(pc,a1.l),a0");
}

#[test]
fn an_empty_movem_mask_is_written_as_its_words() {
    for (words, expected) in [
        (&[0x4C90, 0x0000][..], "dc.w $4C90,$0000"),
        (&[0x48A7, 0x0000][..], "dc.w $48A7,$0000"),
        (&[0x4CB8, 0x0000, 0x1234][..], "dc.w $4CB8,$0000,$1234"),
        (
            &[0x48F9, 0x0000, 0x0001, 0x2345][..],
            "dc.w $48F9,$0000,$0001,$2345",
        ),
    ] {
        let insn = insn(words);
        assert_eq!(insn.text, expected);
        assert_eq!(insn.len as usize, 2 * words.len());
        assert_eq!(insn.flow, FlowKind::Sequential);
        assert_eq!(insn.target, None);
        check_line_tokens(&insn.text, &insn.tokens).unwrap();
    }
    // A mask of one register is a one-register list.
    assert_eq!(text(&[0x4C90, 0x0001]), "movem.w (a0),d0");
    assert_eq!(text(&[0x48A7, 0x0001]), "movem.w a7,-(a7)");
}

#[test]
fn the_modes_golden_file_matches() {
    check_golden("modes");
}

#[test]
fn the_illegal_golden_file_matches() {
    check_golden("illegal");
}
