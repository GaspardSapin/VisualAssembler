//! End-to-end tests of `va lex` (m1-core-isa-lexer.md §7.2, §7.3, §7.5 and §9.1): exit codes, the
//! text and `va.lex/1` outputs against the lexer's golden fixtures, stdin, line terminators, BOM,
//! input limits, `va.error/1` failures, `--lang`, and closed output pipes.
#![allow(clippy::unwrap_used)]
#![cfg(feature = "arch-m68k")]

use std::collections::BTreeSet;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::Value;
use va_arch_m68k::M68K;
use va_core::arch::Architecture as _;
use va_core::diag::DiagArg;
use va_core::source::MAX_SOURCE_BYTES;
use va_workspace::i18n::{Lang, Localizer};

/// Exit code of a command that ran and found problems.
const EXIT_PROBLEMS: i32 = 1;

/// Exit code of a usage error.
const EXIT_USAGE: i32 = 2;

/// Exit code of an I/O or environment error.
const EXIT_IO: i32 = 3;

/// The English text of the error of a write to a pipe without reader (m1 §12.8 F-1):
/// ERROR_NO_DATA on Windows, EPIPE on Unix.
const BROKEN_PIPE: &str = if cfg!(windows) {
    "broken pipe (os error 232)"
} else {
    "broken pipe (os error 32)"
};

/// Byte sequences that must never reach a terminal raw (Security M1-S1): ESC, BEL, the C1 CSI
/// U+009B and the right-to-left override U+202E, in UTF-8.
const TERMINAL_UNSAFE: [&[u8]; 4] = [b"\x1b", b"\x07", b"\xc2\x9b", b"\xe2\x80\xae"];

/// Asserts that `bytes` holds none of [`TERMINAL_UNSAFE`].
fn assert_terminal_safe(bytes: &[u8], what: &str) {
    for unsafe_bytes in TERMINAL_UNSAFE {
        assert!(
            !bytes
                .windows(unsafe_bytes.len())
                .any(|window| window == unsafe_bytes),
            "{what} holds {unsafe_bytes:02x?}: {}",
            String::from_utf8_lossy(bytes)
        );
    }
}

/// The six golden fixtures of m1 §5.5.
const FIXTURES: [&str; 6] = [
    "basic", "labels", "literals", "comments", "macros", "errors",
];

/// Builds a command running the `va` binary under test with `args`.
///
/// The language variables are removed so the user's locale cannot change the messages, and
/// `NO_COLOR` keeps ANSI escapes out of clap's messages.
fn va_command(args: &[&str]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_va"));
    command
        .args(args)
        .env("NO_COLOR", "1")
        .env_remove("VA_LANG")
        .env_remove("LC_ALL")
        .env_remove("LC_MESSAGES")
        .env_remove("LANG")
        .stdin(Stdio::null());
    command
}

/// Runs `va` with `args`, capturing stdout and stderr.
fn run_va(args: &[&str]) -> Output {
    va_command(args).output().unwrap()
}

/// Runs `va` with `args` and `input` on stdin, capturing stdout and stderr.
///
/// The input is written from a thread, so the child can exit before reading all of it (an
/// oversized input): the resulting broken-pipe error of the writer is ignored.
fn run_va_with_stdin(args: &[&str], input: Vec<u8>) -> Output {
    let mut child = va_command(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&input);
    });
    let output = child.wait_with_output().unwrap();
    writer.join().unwrap();
    output
}

/// Decodes a captured stream as UTF-8.
fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).unwrap()
}

/// Path of the fixture `<name>.<extension>` of the 68000 lexer.
fn fixture(name: &str, extension: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../arch/m68k/fixtures/lex")
        .join(format!("{name}.{extension}"))
}

/// Creates a fresh, empty folder `<CARGO_TARGET_TMPDIR>/<tag>-<pid>-<n>` (CLAUDE.md test rule).
fn temp_dir(tag: &str) -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("{tag}-{}-{n}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    dir
}

/// Writes `bytes` to a new file `name` in a fresh temp folder and returns its path.
fn temp_file(tag: &str, name: &str, bytes: &[u8]) -> PathBuf {
    let path = temp_dir(tag).join(name);
    std::fs::write(&path, bytes).unwrap();
    path
}

/// Returns the single JSON line of `stdout` (exactly one trailing newline) and its parse.
fn json_line(stdout: &[u8]) -> (String, Value) {
    let stdout = text(stdout);
    let line = stdout
        .strip_suffix('\n')
        .unwrap_or_else(|| panic!("stdout must end with one newline: {stdout:?}"));
    assert!(!line.contains('\n'), "more than one line: {stdout:?}");
    (line.to_owned(), serde_json::from_str(line).unwrap())
}

/// The English localizer of the 68000, to compute the expected messages.
fn english() -> Localizer {
    Localizer::for_architecture(Lang::En, &M68K).unwrap()
}

/// Renders the §5.5 golden format from a `va.lex/1` document:
/// `<line>:<start>..<end> <kind>[ <code>] <text as a JSON string>` per token.
fn tokens_from_json(document: &Value) -> String {
    let mut rendered = String::new();
    for line in document["lines"].as_array().unwrap() {
        for token in line["tokens"].as_array().unwrap() {
            let code = token
                .get("code")
                .map(|code| format!(" {}", code.as_str().unwrap()))
                .unwrap_or_default();
            rendered.push_str(&format!(
                "{}:{}..{} {}{} {}\n",
                line["line"],
                token["start"],
                token["end"],
                token["kind"].as_str().unwrap(),
                code,
                serde_json::to_string(&token["text"]).unwrap()
            ));
        }
    }
    rendered
}

#[test]
fn basic_fixture_exits_zero_and_prints_non_whitespace_tokens() {
    let path = fixture("basic", "asm");
    let output = run_va(&["lex", path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(text(&output.stderr), "");
    let stdout = text(&output.stdout);
    let mut lines = stdout.lines();
    assert_eq!(
        lines.next(),
        Some(r#"1:1 comment "* basic.asm: a small complete program, every common field""#)
    );
    assert_eq!(
        lines.next(),
        Some(r#"2:1 comment "; a semicolon comment line""#)
    );
    // `start<TAB>move.l<TAB>#$1234,d0<TAB>load a constant`: columns count chars, tab = 1.
    let line3: Vec<&str> = stdout.lines().filter(|l| l.starts_with("3:")).collect();
    assert_eq!(
        line3,
        [
            r#"3:1 label "start""#,
            r#"3:7 mnemonic "move""#,
            r#"3:11 size-suffix ".l""#,
            r##"3:14 punctuation "#""##,
            r#"3:15 number "$1234""#,
            r#"3:20 punctuation ",""#,
            r#"3:21 register "d0""#,
            r#"3:24 comment "load a constant""#,
        ]
    );
    assert!(
        !stdout.contains(" whitespace "),
        "whitespace tokens are not printed in text mode: {stdout}"
    );
}

/// The JSON tokens of every fixture, rendered in the golden format, equal the lexer's own golden
/// `.tokens` files: `va lex` adds nothing, drops nothing and keeps line-relative ranges.
#[test]
fn json_tokens_match_every_golden_fixture() {
    for name in FIXTURES {
        let path = fixture(name, "asm");
        let output = run_va(&["--json", "lex", path.to_str().unwrap()]);
        let golden = std::fs::read_to_string(fixture(name, "tokens")).unwrap();
        // Every m68k lexer diagnostic is an error, one per invalid or suspect-comment token.
        let has_errors = golden
            .lines()
            .any(|line| line.contains(" invalid ") || line.contains(" suspect-comment "));
        let expected_code = if has_errors { EXIT_PROBLEMS } else { 0 };
        assert_eq!(output.status.code(), Some(expected_code), "fixture {name}");
        assert_eq!(text(&output.stderr), "", "fixture {name}");
        let (_, document) = json_line(&output.stdout);
        assert_eq!(tokens_from_json(&document), golden, "fixture {name}");

        let lines = document["lines"].as_array().unwrap();
        let tokens: usize = lines
            .iter()
            .map(|line| line["tokens"].as_array().unwrap().len())
            .sum();
        assert_eq!(document["summary"]["lines"], lines.len(), "fixture {name}");
        assert_eq!(document["summary"]["tokens"], tokens, "fixture {name}");
        let diagnostics = document["diagnostics"].as_array().unwrap();
        assert_eq!(
            document["summary"]["errors"],
            diagnostics.len(),
            "fixture {name}"
        );
        for (index, line) in lines.iter().enumerate() {
            assert_eq!(line["line"], index + 1, "fixture {name}");
        }
    }
}

/// `dc.b 1, 2, 3` (m1 §5.7): a `suspect-comment` token, a non-blocking error with its fix, and
/// exit 1. The exact line also pins the `va.lex/1` key order, `-` for stdin, file offsets for
/// diagnostics and fixes, and the empty last line.
#[test]
fn suspect_comment_on_stdin_is_a_non_blocking_error_with_a_fix() {
    let output = run_va_with_stdin(&["--json", "lex", "-"], b"\tdc.b 1, 2, 3\n".to_vec());
    assert_eq!(output.status.code(), Some(EXIT_PROBLEMS));
    assert_eq!(text(&output.stderr), "");
    let (line, _) = json_line(&output.stdout);

    let localizer = english();
    let args = [
        ("text", DiagArg::Str("2, 3".to_owned())),
        ("cause", DiagArg::Str("after-comma".to_owned())),
    ];
    let id = "diag-lex-operands-in-comment";
    let message = localizer.message(id, None, &args).unwrap();
    let help = localizer.message(id, Some("help"), &args).unwrap();
    let expected = format!(
        concat!(
            r#"{{"schema":"va.lex/1","arch":"m68k","lang":"en","file":"-","lines":["#,
            r#"{{"line":1,"offset":0,"tokens":["#,
            r#"{{"kind":"whitespace","start":0,"end":1,"column":1,"text":"\t"}},"#,
            r#"{{"kind":"directive","start":1,"end":3,"column":2,"text":"dc"}},"#,
            r#"{{"kind":"size-suffix","start":3,"end":5,"column":4,"text":".b"}},"#,
            r#"{{"kind":"whitespace","start":5,"end":6,"column":6,"text":" "}},"#,
            r#"{{"kind":"number","start":6,"end":7,"column":7,"text":"1"}},"#,
            r#"{{"kind":"punctuation","start":7,"end":8,"column":8,"text":","}},"#,
            r#"{{"kind":"whitespace","start":8,"end":9,"column":9,"text":" "}},"#,
            r#"{{"kind":"suspect-comment","start":9,"end":13,"column":10,"text":"2, 3","code":"lex-operands-in-comment"}}]}},"#,
            r#"{{"line":2,"offset":14,"tokens":[]}}],"#,
            r#""diagnostics":[{{"code":"lex-operands-in-comment","severity":"error","blocking":false,"#,
            r#""line":1,"column":10,"start":9,"end":13,"message":{},"help":{},"note":null,"#,
            r#""fix":{{"start":8,"end":13,"replacement":"2,3"}}}}],"#,
            r#""summary":{{"lines":2,"tokens":8,"errors":1}}}}"#
        ),
        serde_json::to_string(&message).unwrap(),
        serde_json::to_string(&help).unwrap()
    );
    assert_eq!(line, expected);
}

/// Text mode of the same line: the token line with its code on stdout, the GNU-style
/// diagnostic with `help` and `fix` on stderr.
#[test]
fn suspect_comment_text_mode_prints_help_and_fixed_line() {
    let path = temp_file("lex-suspect", "spaces.asm", b"\tdc.b 1, 2, 3\n");
    let name = path.to_str().unwrap();
    let output = run_va(&["lex", name]);
    assert_eq!(output.status.code(), Some(EXIT_PROBLEMS));
    let stdout = text(&output.stdout);
    assert!(
        stdout.contains("1:10 suspect-comment \"2, 3\" lex-operands-in-comment\n"),
        "stdout: {stdout}"
    );
    let localizer = english();
    let args = [
        ("text", DiagArg::Str("2, 3".to_owned())),
        ("cause", DiagArg::Str("after-comma".to_owned())),
    ];
    let id = "diag-lex-operands-in-comment";
    // The fixed line is terminal-safe: its leading tab is written `\t` (Security M1-S1).
    let expected = format!(
        "{name}:1:10: error[lex-operands-in-comment]: {}\n  help: {}\n  fix: \\tdc.b 1,2,3\n",
        localizer.message(id, None, &args).unwrap(),
        localizer.message(id, Some("help"), &args).unwrap()
    );
    assert_eq!(text(&output.stderr), expected);
}

/// Comments after `;`, `*` lines and free text after complete operands never fire §5.7.
#[test]
fn real_comments_exit_zero() {
    let source = b"\tdc.b 1,2,3 ; one, two, three\n; whole line, comment\n* star, comment\n\tmove.l d0,d1 free text\n\trts d0, d1\n";
    let output = run_va_with_stdin(&["--json", "lex", "-"], source.to_vec());
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stdout));
    let (_, document) = json_line(&output.stdout);
    assert_eq!(document["diagnostics"], Value::Array(Vec::new()));
    assert_eq!(document["summary"]["errors"], 0);
}

/// `errors.asm` produces every diagnostic code of the CPU, exits 1 in both modes, and the text
/// mode prints one GNU-style line per diagnostic, prefixed with the path as given.
#[test]
fn errors_fixture_reports_every_code() {
    let path = fixture("errors", "asm");
    let name = path.to_str().unwrap();
    let output = run_va(&["--json", "lex", name]);
    assert_eq!(output.status.code(), Some(EXIT_PROBLEMS));
    let (_, document) = json_line(&output.stdout);
    assert_eq!(document["file"], name);
    let diagnostics = document["diagnostics"].as_array().unwrap();
    let codes: BTreeSet<&str> = diagnostics
        .iter()
        .map(|diagnostic| diagnostic["code"].as_str().unwrap())
        .collect();
    let expected: BTreeSet<&str> = M68K.diagnostic_codes().iter().map(|code| code.0).collect();
    assert_eq!(codes, expected);
    for diagnostic in diagnostics {
        let blocking = diagnostic["code"] != "lex-operands-in-comment";
        assert_eq!(diagnostic["blocking"], blocking, "{diagnostic}");
        assert_eq!(diagnostic["severity"], "error", "{diagnostic}");
        assert!(diagnostic["message"].is_string(), "{diagnostic}");
        assert!(diagnostic["note"].is_null(), "{diagnostic}");
        // The column is the char column of `start` on its line.
        let line = &document["lines"][diagnostic["line"].as_u64().unwrap() as usize - 1];
        let relative = diagnostic["start"].as_u64().unwrap() - line["offset"].as_u64().unwrap();
        let token = line["tokens"]
            .as_array()
            .unwrap()
            .iter()
            .find(|token| token["start"].as_u64() == Some(relative))
            .unwrap();
        assert_eq!(diagnostic["column"], token["column"], "{diagnostic}");
    }

    let output = run_va(&["lex", name]);
    assert_eq!(output.status.code(), Some(EXIT_PROBLEMS));
    let stderr = text(&output.stderr);
    let headers: Vec<&str> = stderr
        .lines()
        .filter(|line| !line.starts_with("  "))
        .collect();
    assert_eq!(headers.len(), diagnostics.len(), "stderr: {stderr}");
    assert!(
        headers[0].starts_with(&format!("{name}:2:1: error[lex-bad-label]: ")),
        "stderr: {stderr}"
    );
}

/// A program with LF, CRLF and lone-CR terminators gives the same text output (tokens, lines
/// and columns); in JSON only the line offsets differ.
#[test]
fn crlf_and_cr_give_the_same_tokens_as_lf() {
    let lf = "start\tmove.l\t#1,d0\t; go\n\tdc.b 1, 2\nloop:\tbra.s\tloop\n";
    let crlf = lf.replace('\n', "\r\n");
    let cr = lf.replace('\n', "\r");
    let outputs: Vec<Output> = [lf, &crlf, &cr]
        .into_iter()
        .map(|source| run_va_with_stdin(&["lex", "-"], source.as_bytes().to_vec()))
        .collect();
    for output in &outputs {
        assert_eq!(output.status.code(), Some(EXIT_PROBLEMS));
        assert_eq!(output.stdout, outputs[0].stdout);
        assert_eq!(output.stderr, outputs[0].stderr);
    }

    let documents: Vec<Value> = [lf, &crlf]
        .into_iter()
        .map(|source| {
            let output = run_va_with_stdin(&["--json", "lex", "-"], source.as_bytes().to_vec());
            json_line(&output.stdout).1
        })
        .collect();
    let lines_lf = documents[0]["lines"].as_array().unwrap();
    let lines_crlf = documents[1]["lines"].as_array().unwrap();
    assert_eq!(lines_lf.len(), lines_crlf.len());
    for (index, (a, b)) in lines_lf.iter().zip(lines_crlf).enumerate() {
        assert_eq!(a["tokens"], b["tokens"], "line {}", index + 1);
    }
    assert_eq!(lines_lf[1]["offset"], 24);
    assert_eq!(lines_crlf[1]["offset"], 25);
}

/// A leading BOM is stripped: offsets are relative to the text after it, so the output equals
/// the output without BOM.
#[test]
fn bom_is_stripped_before_lexing() {
    let source = b"\tnop\n\tdc.b 1, 2\n";
    let mut with_bom = b"\xEF\xBB\xBF".to_vec();
    with_bom.extend_from_slice(source);
    for args in [&["lex", "-"][..], &["--json", "lex", "-"]] {
        let plain = run_va_with_stdin(args, source.to_vec());
        let bom = run_va_with_stdin(args, with_bom.clone());
        assert_eq!(bom.status.code(), Some(EXIT_PROBLEMS), "args: {args:?}");
        assert_eq!(bom.stdout, plain.stdout, "args: {args:?}");
        assert_eq!(bom.stderr, plain.stderr, "args: {args:?}");
    }
}

/// Invalid UTF-8 exits 3 with `input-not-utf8` and the byte offset, before any output.
#[test]
fn non_utf8_input_exits_3() {
    let path = temp_file("lex-utf8", "latin1.asm", b"\tnop\n; caf\xE9\n");
    let output = run_va(&["--json", "lex", path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(EXIT_IO));
    let (_, document) = json_line(&output.stdout);
    assert_eq!(document["schema"], "va.error/1");
    assert_eq!(document["code"], "input-not-utf8");
    let message = document["message"].as_str().unwrap();
    assert!(message.contains("offset 10"), "message: {message}");
    assert_eq!(text(&output.stderr), format!("error: {message}\n"));

    let output = run_va_with_stdin(&["lex", "-"], b"\xFF".to_vec());
    assert_eq!(output.status.code(), Some(EXIT_IO));
    assert_eq!(text(&output.stdout), "");
    let stderr = text(&output.stderr);
    assert!(
        stderr.starts_with("error: standard input is not valid UTF-8"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("offset 0"), "stderr: {stderr}");
}

/// Exactly 16 MiB is accepted; one byte more exits 3 with `input-too-large`, from a file and
/// from stdin (which is not read beyond the limit).
#[test]
fn input_limit_is_16_mib() {
    let mut at_limit = Vec::with_capacity(MAX_SOURCE_BYTES + 1);
    while at_limit.len() < MAX_SOURCE_BYTES {
        let room = MAX_SOURCE_BYTES - at_limit.len();
        let run = room.min(1024);
        at_limit.extend(std::iter::repeat_n(b' ', run - 1));
        at_limit.push(b'\n');
    }
    assert_eq!(at_limit.len(), MAX_SOURCE_BYTES);
    let dir = temp_dir("lex-limit");
    let at_path = dir.join("at-limit.asm");
    std::fs::write(&at_path, &at_limit).unwrap();
    let output = run_va(&["lex", at_path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    assert_eq!(text(&output.stdout), "");

    let mut over_limit = at_limit;
    over_limit.push(b' ');
    let over_path = dir.join("over-limit.asm");
    std::fs::write(&over_path, &over_limit).unwrap();
    let output = run_va(&["--json", "lex", over_path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(EXIT_IO));
    let (_, document) = json_line(&output.stdout);
    assert_eq!(document["code"], "input-too-large");

    // Four times the limit on stdin: the child stops reading after one byte over the limit.
    over_limit.resize(4 * MAX_SOURCE_BYTES, b' ');
    let output = run_va_with_stdin(&["--json", "lex", "-"], over_limit);
    assert_eq!(output.status.code(), Some(EXIT_IO));
    let (_, document) = json_line(&output.stdout);
    assert_eq!(document["code"], "input-too-large");
}

/// A directory or a missing file exits 3 with `read-failed`.
#[test]
fn unreadable_paths_exit_3() {
    let dir = temp_dir("lex-unreadable");
    let missing = dir.join("missing.asm");
    for path in [&dir, &missing] {
        let path = path.to_str().unwrap();
        let output = run_va(&["--json", "lex", path]);
        assert_eq!(output.status.code(), Some(EXIT_IO), "path: {path}");
        let (line, document) = json_line(&output.stdout);
        assert_eq!(document["schema"], "va.error/1", "path: {path}");
        assert_eq!(document["code"], "read-failed", "path: {path}");
        assert!(
            line.starts_with(r#"{"schema":"va.error/1","code":"read-failed","message":"#),
            "key order: {line}"
        );
        let message = document["message"].as_str().unwrap();
        assert!(message.contains(path), "message: {message}");
        assert_eq!(text(&output.stderr), format!("error: {message}\n"));

        let output = run_va(&["lex", path]);
        assert_eq!(output.status.code(), Some(EXIT_IO), "path: {path}");
        assert_eq!(text(&output.stdout), "", "path: {path}");
    }
    let output = run_va(&["lex", dir.to_str().unwrap()]);
    let stderr = text(&output.stderr);
    assert!(
        stderr.ends_with(": it is a directory\n"),
        "stderr: {stderr}"
    );
}

/// `--lang fr` changes the messages and `lang`, nothing else; the French text reaches a pipe as
/// UTF-8.
#[test]
fn lang_fr_changes_only_the_messages() {
    let path = fixture("errors", "asm");
    let path = path.to_str().unwrap();
    let english = run_va(&["--json", "lex", path]);
    let french = run_va(&["--json", "--lang", "fr", "lex", path]);
    assert_eq!(french.status.code(), Some(EXIT_PROBLEMS));
    let (_, mut english) = json_line(&english.stdout);
    let (french_line, mut french) = json_line(&french.stdout);
    assert_eq!(english["lang"], "en");
    assert_eq!(french["lang"], "fr");
    assert!(
        french_line.bytes().any(|byte| byte >= 0x80),
        "French accents expected"
    );

    let mut differing = 0;
    for document in [&mut english, &mut french] {
        document["lang"] = Value::Null;
    }
    let english_diagnostics = english["diagnostics"].as_array_mut().unwrap();
    let french_diagnostics = french["diagnostics"].as_array_mut().unwrap();
    for (en, fr) in english_diagnostics.iter_mut().zip(french_diagnostics) {
        for key in ["message", "help", "note"] {
            if en[key] != fr[key] {
                differing += 1;
            }
            en[key] = Value::Null;
            fr[key] = Value::Null;
        }
    }
    assert!(differing > 0);
    assert_eq!(english, french);

    let output = run_va(&["--lang", "fr", "lex", path]);
    let stderr = text(&output.stderr);
    assert!(
        stderr.contains("error[lex-bad-label]: « 9bad »"),
        "stderr: {stderr}"
    );
}

/// A stdout pipe with no reader exits 3 with an `error:` line, in both modes; no `va.error/1`
/// document can be printed there.
#[test]
fn closed_stdout_pipe_is_io_error() {
    let path = fixture("basic", "asm");
    for args in [&["lex"][..], &["--json", "lex"]] {
        let mut args = args.to_vec();
        args.push(path.to_str().unwrap());
        let (reader, writer) = std::io::pipe().unwrap();
        drop(reader);
        let output = va_command(&args)
            .stdout(Stdio::from(writer))
            .stderr(Stdio::piped())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(EXIT_IO), "args: {args:?}");
        assert_eq!(
            text(&output.stderr),
            format!("error: cannot write to stdout: {BROKEN_PIPE}\n"),
            "args: {args:?}"
        );
    }
}

/// In text mode the diagnostics are the result on stderr: losing them exits 3, not 1.
#[test]
fn closed_stderr_pipe_with_diagnostics_is_io_error() {
    let path = fixture("errors", "asm");
    let (reader, writer) = std::io::pipe().unwrap();
    drop(reader);
    let output = va_command(&["lex", path.to_str().unwrap()])
        .stdout(Stdio::piped())
        .stderr(Stdio::from(writer))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(EXIT_IO));
}

#[test]
fn missing_or_extra_file_argument_is_a_usage_error() {
    for args in [&["lex"][..], &["lex", "a.asm", "b.asm"]] {
        let output = run_va(args);
        assert_eq!(output.status.code(), Some(EXIT_USAGE), "args: {args:?}");
        assert_eq!(text(&output.stdout), "", "args: {args:?}");
    }
}

// QA edge cases (m1 §3.2, §5.2, §7.2, §7.3, §7.5, §12.2, §12.7).

/// An empty input and a BOM-only input are one empty line: exit 0, no output in text mode,
/// `"tokens":[]` in JSON (§7.3 "every line appears"; §12.2 `lines("")` yields one line).
#[test]
fn qa_empty_and_bom_only_inputs_are_one_empty_line() {
    for input in [&b""[..], b"\xEF\xBB\xBF"] {
        let output = run_va_with_stdin(&["lex", "-"], input.to_vec());
        assert_eq!(output.status.code(), Some(0), "input: {input:?}");
        assert_eq!(text(&output.stdout), "", "input: {input:?}");
        assert_eq!(text(&output.stderr), "", "input: {input:?}");

        let output = run_va_with_stdin(&["--json", "lex", "-"], input.to_vec());
        assert_eq!(output.status.code(), Some(0), "input: {input:?}");
        let (line, _) = json_line(&output.stdout);
        assert_eq!(
            line,
            r#"{"schema":"va.lex/1","arch":"m68k","lang":"en","file":"-","lines":[{"line":1,"offset":0,"tokens":[]}],"diagnostics":[],"summary":{"lines":1,"tokens":0,"errors":0}}"#
        );
    }
}

/// A second BOM is text: U+FEFF in column 1 is not a label start (one char, three bytes).
#[test]
fn qa_a_second_bom_is_text() {
    let output = run_va_with_stdin(
        &["--json", "lex", "-"],
        b"\xEF\xBB\xBF\xEF\xBB\xBFstart nop\n".to_vec(),
    );
    assert_eq!(output.status.code(), Some(EXIT_PROBLEMS));
    let (_, document) = json_line(&output.stdout);
    let tokens = document["lines"][0]["tokens"].as_array().unwrap();
    assert_eq!(tokens[0]["kind"], "invalid");
    assert_eq!(tokens[0]["code"], "lex-bad-label");
    assert_eq!(tokens[0]["text"], "\u{FEFF}start");
    assert_eq!(numbers(&tokens[0], &["start", "end", "column"]), [0, 8, 1]);
    assert_eq!(tokens[1]["column"], 7);
    assert_eq!(tokens[2]["kind"], "mnemonic");
}

/// A line of exactly `MAX_LINE_BYTES` (64 KiB, terminator excluded) is lexed normally, with LF
/// or CRLF; one byte more is exactly one `lex-line-too-long` token over the whole line, whose
/// message carries the truncated text, never the whole line (§3.4, §5.2).
#[test]
fn qa_line_of_exactly_64_kib_is_lexed_and_one_byte_more_is_one_invalid_token() {
    let max = va_core::source::MAX_LINE_BYTES;
    let line = |len: usize| {
        let mut line = b"\tnop ;".to_vec();
        line.resize(len, b'x');
        line
    };
    for terminator in [&b"\n"[..], b"\r\n"] {
        let mut input = line(max);
        input.extend_from_slice(terminator);
        let output = run_va_with_stdin(&["--json", "lex", "-"], input);
        assert_eq!(output.status.code(), Some(0));
        let (_, document) = json_line(&output.stdout);
        let kinds: Vec<&str> = document["lines"][0]["tokens"]
            .as_array()
            .unwrap()
            .iter()
            .map(|token| token["kind"].as_str().unwrap())
            .collect();
        assert_eq!(kinds, ["whitespace", "mnemonic", "whitespace", "comment"]);
    }
    // Exactly 64 KiB of two-byte chars in the comment.
    let mut input = b"\tnop ;".to_vec();
    while input.len() < max {
        input.extend_from_slice("é".as_bytes());
    }
    assert_eq!(input.len(), max);
    let output = run_va_with_stdin(&["lex", "-"], input);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));

    let mut input = line(max + 1);
    input.extend_from_slice(b"\n\tnop\n");
    let output = run_va_with_stdin(&["--json", "lex", "-"], input);
    assert_eq!(output.status.code(), Some(EXIT_PROBLEMS));
    let (_, document) = json_line(&output.stdout);
    let tokens = document["lines"][0]["tokens"].as_array().unwrap();
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0]["code"], "lex-line-too-long");
    assert_eq!(tokens[0]["start"], 0);
    assert_eq!(tokens[0]["end"], max + 1);
    assert_eq!(document["lines"][1]["offset"], max + 2);
    assert_eq!(document["lines"][1]["tokens"].as_array().unwrap().len(), 2);
    let diagnostics = document["diagnostics"].as_array().unwrap();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0]["end"], max + 1);
    assert!(diagnostics[0]["message"].as_str().unwrap().len() < 200);
}

/// The 16 MiB limit counts the input bytes including a BOM (§12.2): BOM + content of exactly
/// 16 MiB in total is accepted, one byte more is `input-too-large`.
#[test]
fn qa_the_16_mib_limit_counts_the_bom() {
    let mut at_limit = b"\xEF\xBB\xBF".to_vec();
    while at_limit.len() < MAX_SOURCE_BYTES {
        let run = (MAX_SOURCE_BYTES - at_limit.len()).min(512);
        at_limit.extend(std::iter::repeat_n(b';', run - 1));
        at_limit.push(b'\n');
    }
    assert_eq!(at_limit.len(), MAX_SOURCE_BYTES);
    let dir = temp_dir("qa-lex-limit-bom");
    let at_path = dir.join("at-limit-bom.asm");
    std::fs::write(&at_path, &at_limit).unwrap();
    let output = run_va(&["lex", at_path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));

    at_limit.push(b';');
    let over_path = dir.join("over-limit-bom.asm");
    std::fs::write(&over_path, &at_limit).unwrap();
    let output = run_va(&["--json", "lex", over_path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(EXIT_IO));
    let (_, document) = json_line(&output.stdout);
    assert_eq!(document["code"], "input-too-large");
}

/// NUL is a control character: an error in operands, plain text in a comment.
#[test]
fn qa_nul_bytes() {
    let output = run_va_with_stdin(&["lex", "-"], b"\tmove.l d0,\0d1\n".to_vec());
    assert_eq!(output.status.code(), Some(EXIT_PROBLEMS));
    assert!(
        text(&output.stdout).contains("1:12 invalid \"\\u0000\" lex-unexpected-char\n"),
        "stdout: {}",
        text(&output.stdout)
    );
    assert!(
        text(&output.stderr).starts_with("-:1:12: error[lex-unexpected-char]: "),
        "stderr: {}",
        text(&output.stderr)
    );
    let output = run_va_with_stdin(&["lex", "-"], b"\tnop ; a\0b\n".to_vec());
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(text(&output.stderr), "");
}

/// Columns count chars and token offsets count line bytes, diagnostic offsets file bytes, with
/// 4-byte UTF-8 chars before the token (§3.2, §7.3).
#[test]
fn qa_columns_count_chars_and_offsets_count_bytes() {
    let source = "; \u{1F600}é\n\tdc.b '\u{1F600}',é\n";
    let output = run_va_with_stdin(&["lex", "-"], source.as_bytes().to_vec());
    assert_eq!(output.status.code(), Some(EXIT_PROBLEMS));
    assert_eq!(
        text(&output.stdout),
        "1:1 comment \"; \u{1F600}é\"\n2:2 directive \"dc\"\n2:4 size-suffix \".b\"\n\
         2:7 string \"'\u{1F600}'\"\n2:10 punctuation \",\"\n2:11 invalid \"é\" lex-non-ascii\n"
    );
    assert!(text(&output.stderr).starts_with("-:2:11: error[lex-non-ascii]: "));

    let output = run_va_with_stdin(&["--json", "lex", "-"], source.as_bytes().to_vec());
    let (_, document) = json_line(&output.stdout);
    let line = &document["lines"][1];
    assert_eq!(line["offset"], 9);
    let invalid = line["tokens"].as_array().unwrap().last().unwrap();
    assert_eq!(numbers(invalid, &["start", "end", "column"]), [13, 15, 11]);
    assert_eq!(
        numbers(
            &document["diagnostics"][0],
            &["line", "column", "start", "end"]
        ),
        [2, 11, 22, 24]
    );
}

/// The unsigned integers at `keys` of the JSON object `value`.
fn numbers(value: &Value, keys: &[&str]) -> Vec<u64> {
    keys.iter()
        .map(|key| {
            value[*key]
                .as_u64()
                .unwrap_or_else(|| panic!("{key}: {value}"))
        })
        .collect()
}

/// With a BOM and CRLF terminators, line offsets, diagnostic offsets and the fix are file bytes
/// of the text after the BOM, and the text-mode fixed line carries no `\r`.
#[test]
fn qa_bom_and_crlf_diagnostic_and_fix_offsets_are_file_bytes() {
    let input = b"\xEF\xBB\xBF\tnop\r\n\tdc.b 1, 2\r\n".to_vec();
    let output = run_va_with_stdin(&["--json", "lex", "-"], input.clone());
    assert_eq!(output.status.code(), Some(EXIT_PROBLEMS));
    let (_, document) = json_line(&output.stdout);
    let offsets: Vec<u64> = document["lines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|line| line["offset"].as_u64().unwrap())
        .collect();
    assert_eq!(offsets, [0, 6, 18]);
    let diagnostic = &document["diagnostics"][0];
    assert_eq!(
        numbers(diagnostic, &["start", "end", "column"]),
        [15, 16, 10]
    );
    assert_eq!(
        diagnostic["fix"],
        serde_json::json!({"start": 14, "end": 16, "replacement": "2"})
    );

    let output = run_va_with_stdin(&["lex", "-"], input);
    let stderr = text(&output.stderr);
    assert!(
        stderr.starts_with("-:2:10: error[lex-operands-in-comment]: "),
        "{stderr}"
    );
    // Security M1-S1 (wave-3 fix): the fixed line is terminal-safe, its tab written `\t`.
    assert!(stderr.ends_with("  fix: \\tdc.b 1,2\n"), "{stderr:?}");
}

/// An ignored `VA_LANG` is reported once on stderr before the diagnostics, which stay English;
/// with `--lang`, the variable is not read (§7.1, §12.2).
#[test]
fn qa_va_lang_warning_on_lex() {
    let path = fixture("comments", "asm");
    let path = path.to_str().unwrap();
    for value in ["FR", "", "fr_FR", "AUTO"] {
        let output = va_command(&["lex", path])
            .env("VA_LANG", value)
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(EXIT_PROBLEMS),
            "VA_LANG={value:?}"
        );
        let stderr = text(&output.stderr);
        let warning = format!("warning: ignoring VA_LANG={value} (expected en, fr or auto)\n");
        assert!(stderr.starts_with(&warning), "VA_LANG={value:?}: {stderr}");
        assert_eq!(stderr.matches("warning:").count(), 1, "{stderr}");
        assert!(stderr.contains("a68k will ignore"), "{stderr}");
    }
    let output = va_command(&["--lang", "fr", "lex", path])
        .env("VA_LANG", "FR")
        .output()
        .unwrap();
    let stderr = text(&output.stderr);
    assert!(!stderr.contains("warning:"), "{stderr}");
    assert!(stderr.contains("a68k ignorera"), "{stderr}");
}

/// CLI messages are English (§7.5): an OS error is rendered as its `io::ErrorKind` and number,
/// the form §12.7 ratifies because Windows localizes OS messages. A closed stdout pipe must
/// therefore read `cannot write to stdout: broken pipe (os error <n>)` on every OS language.
#[test]
fn qa_stdout_failure_message_is_english() {
    let kind = std::io::ErrorKind::BrokenPipe.to_string();
    let path = fixture("basic", "asm");
    for args in [&["lex"][..], &["--json", "lex"]] {
        let mut args = args.to_vec();
        args.push(path.to_str().unwrap());
        let (reader, writer) = std::io::pipe().unwrap();
        drop(reader);
        let output = va_command(&args)
            .stdout(Stdio::from(writer))
            .stderr(Stdio::piped())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(EXIT_IO), "args: {args:?}");
        let stderr = text(&output.stderr);
        let number = stderr
            .strip_prefix(&format!("error: cannot write to stdout: {kind} (os error "))
            .and_then(|rest| rest.strip_suffix(")\n"))
            .unwrap_or_else(|| panic!("args: {args:?}: OS-localized or unexpected: {stderr:?}"));
        assert!(
            !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit()),
            "{stderr:?}"
        );
    }
}

// Security round 1 (M1-S1, M1-S3, M1-S4): text mode never writes terminal control, C1 or bidi
// characters raw; the JSON document keeps the source text as it is.

/// A source holding a title-setting OSC (`ESC ] 0 ; x BEL`), a clear-screen CSI (`ESC [ 2 J`), an
/// OSC 52 clipboard write, a right-to-left override and a C1 CSI, in comments, strings, invalid
/// tokens and suspect comments (hence in messages and fixed lines).
const HOSTILE_SOURCE: &str = concat!(
    "* title \u{1b}]0;x\u{7} clear \u{1b}[2J\n",
    "\tdc.b 1, 2\u{1b}[2J\n",
    "\tmove.l d0,d1 ; clip \u{1b}]52;c;aGk=\u{7}\n",
    "\tdc.b \u{202e}x, \u{9b}2J\n",
    "\tdc.b 'a\u{202e}b', 1, \u{9b}31m\n",
    "\tdc.b \u{1b}[31m\n",
);

#[test]
fn text_mode_never_writes_terminal_controls_raw() {
    let output = run_va_with_stdin(&["lex", "-"], HOSTILE_SOURCE.as_bytes().to_vec());
    assert_eq!(output.status.code(), Some(EXIT_PROBLEMS));
    assert_terminal_safe(&output.stdout, "stdout");
    assert_terminal_safe(&output.stderr, "stderr");
    let stdout = text(&output.stdout);
    let stderr = text(&output.stderr);
    // The characters are shown, in their escape_debug form, after JSON quoting on stdout.
    assert!(stdout.contains(r#"string "'a\u{202e}b'""#), "{stdout}");
    assert!(stdout.contains(r"\u001b]52;c;aGk=\u0007"), "{stdout}");
    assert!(stdout.contains(r"\u{9b}"), "{stdout}");
    // On stderr, in messages and fixed lines; tabs of the fixed line are escaped too.
    assert!(stderr.contains(r"\u{1b}[2J"), "{stderr}");
    assert!(stderr.contains(r"\u{202e}"), "{stderr}");
    assert!(stderr.contains(r"\u{9b}"), "{stderr}");
    assert!(stderr.contains(r"  fix: \tdc.b 1,2\u{1b}[2J"), "{stderr}");
    for line in stderr.lines() {
        assert!(!line.contains('\t') && !line.contains('\r'), "{line:?}");
    }

    // JSON keeps the text: a JSON reader decodes the same characters the source holds.
    let output = run_va_with_stdin(&["--json", "lex", "-"], HOSTILE_SOURCE.as_bytes().to_vec());
    assert_eq!(output.status.code(), Some(EXIT_PROBLEMS));
    let (line, document) = json_line(&output.stdout);
    assert!(
        line.contains('\u{202e}') && line.contains('\u{9b}'),
        "{line}"
    );
    let texts: String = document["lines"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|line| line["tokens"].as_array().unwrap())
        .map(|token| token["text"].as_str().unwrap())
        .collect();
    assert_eq!(texts, HOSTILE_SOURCE.replace('\n', ""));
}

/// The file name of the diagnostic prefix and of an `error:` line is escaped; `va.error/1` keeps
/// it as given.
#[test]
fn echoed_file_names_are_terminal_safe() {
    // U+202E is a valid file name char everywhere; ESC is not on Windows.
    let name = if cfg!(windows) {
        "evil\u{202e}txt.asm"
    } else {
        "evil\u{202e}\u{1b}[2J.asm"
    };
    let path = temp_file("lex-hostile-name", name, b"\tdc.b 1, 2\n");
    let output = run_va(&["lex", path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(EXIT_PROBLEMS));
    assert_terminal_safe(&output.stderr, "stderr");
    let stderr = text(&output.stderr);
    assert!(stderr.contains(r"evil\u{202e}"), "{stderr}");
    assert!(
        stderr.contains(":1:10: error[lex-operands-in-comment]: "),
        "{stderr}"
    );

    let missing = temp_dir("lex-hostile-missing").join("gone\u{1b}]0;x\u{7}\u{202e}.asm");
    let missing = missing.to_str().unwrap();
    let output = run_va(&["--json", "lex", missing]);
    assert_eq!(output.status.code(), Some(EXIT_IO));
    assert_terminal_safe(&output.stderr, "stderr");
    let stderr = text(&output.stderr);
    assert!(
        stderr.contains(r"gone\u{1b}]0;x\u{7}\u{202e}.asm': "),
        "{stderr}"
    );
    let (_, document) = json_line(&output.stdout);
    assert_eq!(document["code"], "read-failed");
    assert!(
        document["message"].as_str().unwrap().contains(missing),
        "{document}"
    );
}
