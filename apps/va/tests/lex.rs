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
    let expected = format!(
        "{name}:1:10: error[lex-operands-in-comment]: {}\n  help: {}\n  fix: \tdc.b 1,2,3\n",
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
        let stderr = text(&output.stderr);
        assert!(
            stderr.starts_with("error: cannot write to stdout: "),
            "args: {args:?}, stderr: {stderr}"
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
