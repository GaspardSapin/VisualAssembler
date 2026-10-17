//! Lexer throughput benchmark of m1-core-isa-lexer.md §5.6, run by `make bench-lexer`
//! (`cargo test -p va-arch-m68k --release --test syntax_bench -- --ignored --nocapture`).
//!
//! The input is 100,000 lines made by cycling 20 fixed representative lines (labels, every
//! addressing mode, directives, strings, macro parameters, comments, one error line), about
//! 3.3 MB. Each run splits it with `va_core::source::lines` and lexes every line into one reused
//! `Vec<Token>`, cleared per line; 1 warm-up run, then 10 timed runs. The test asserts the token
//! count and prints one `va.bench.lexer/1` JSON line. Budget: median ≤ 50 ms on the reference
//! machine; the Performance agent records the result in `docs/perf/`.
#![allow(clippy::unwrap_used)]

use std::hint::black_box;
use std::time::Instant;

use va_arch_m68k::syntax::lex_line;
use va_core::source::lines;

/// The 20 representative lines, cycled to build the input.
const LINES: [&str; 20] = [
    "start:\tmove.l\t#$00FF00FF,d0\t; load the mask",
    "\tmove.w\td1,d2\t\t\tregister to register",
    "\tmovea.l\ta0,a1\t\t\taddress register",
    "\tmove.b\t(a0),d3\t\t\tindirect",
    "\tmove.w\t(a1)+,d4\t\tpost-increment",
    "\tmove.w\t-(a2),d5\t\tpre-decrement",
    "\tmove.l\t16(a3),d6\t\tdisplacement",
    "\tmove.l\t8(a4,d7.w),d0\t\tindexed",
    "\tmove.w\t($1234).w,d1\t\tabsolute short",
    "\tmove.l\t$00012345,d2\t\tabsolute long",
    "\tlea\ttable(pc),a5\t\tpc relative",
    "\tmove.w\t2(pc,d1.l),d3\t\tpc indexed",
    "loop\tdbf\td7,loop\t\t; count down",
    "table\tdc.b\t'Hello, world',13,10,0",
    "\tdc.w\t$1234,%1010,@17,42\t; bases",
    "\tmovem.l\td0-d7/a0-a6,-(sp)\t; save",
    "copy\tmacro\t\t\t\t; two operands",
    "lbl\\@:\tmove.\\0\t\\1,\\2\t\t; body",
    "* a whole-line comment before the block",
    "\tdc.b\t$12g,1, 2\t\t; error",
];

/// Number of lines of the input.
const LINE_COUNT: usize = 100_000;

/// Timed runs after the warm-up.
const RUNS: usize = 10;

/// Tokens the 20 lines produce, pinned so the benchmark measures the intended work.
const TOKENS_PER_CYCLE: usize = 222;

/// Builds the input: [`LINES`] cycled to [`LINE_COUNT`] lines, joined by LF (no final LF, so
/// `lines` yields exactly `LINE_COUNT` lines).
fn input() -> String {
    let mut text = String::with_capacity(LINE_COUNT * 34);
    for (index, line) in LINES.iter().cycle().take(LINE_COUNT).enumerate() {
        if index > 0 {
            text.push('\n');
        }
        text.push_str(line);
    }
    text
}

/// Lexes the whole text once and returns the number of tokens.
fn lex_all(text: &str) -> usize {
    let mut tokens = Vec::with_capacity(64);
    let mut count = 0;
    for (_, line) in lines(text) {
        tokens.clear();
        lex_line(line, &mut tokens);
        count += tokens.len();
    }
    count
}

#[test]
#[ignore = "benchmark: run with make bench-lexer (release build)"]
fn lexer_throughput() {
    let text = input();
    assert_eq!(lines(&text).count(), LINE_COUNT);
    let expected = TOKENS_PER_CYCLE * LINE_COUNT / LINES.len();
    assert_eq!(
        black_box(lex_all(black_box(&text))),
        expected,
        "warm-up token count"
    );
    let mut times_ms = Vec::with_capacity(RUNS);
    for _ in 0..RUNS {
        let started = Instant::now();
        let count = black_box(lex_all(black_box(&text)));
        times_ms.push(started.elapsed().as_secs_f64() * 1000.0);
        assert_eq!(count, expected, "token count");
    }
    times_ms.sort_by(f64::total_cmp);
    let median_ms = f64::midpoint(times_ms[RUNS / 2 - 1], times_ms[RUNS / 2]);
    let bytes = text.len();
    // Bytes are below 2^53, so the conversion to f64 is exact.
    #[allow(clippy::cast_precision_loss)]
    let mb_per_s = bytes as f64 / 1e6 / (median_ms / 1000.0);
    println!(
        "{{\"schema\":\"va.bench.lexer/1\",\"lines\":{LINE_COUNT},\"bytes\":{bytes},\"tokens\":{expected},\"runs\":{RUNS},\"median_ms\":{median_ms:.3},\"min_ms\":{:.3},\"max_ms\":{:.3},\"mb_per_s\":{mb_per_s:.1}}}",
        times_ms[0],
        times_ms[RUNS - 1],
    );
}
