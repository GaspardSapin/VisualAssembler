//! End-to-end tests of `va isa` (m1-core-isa-lexer.md §7.1, §7.4, §7.5, §9.1 and §12.6): the text
//! and `va.isa/1` / `va.isa.list/1` outputs, lookup failures with `va.error/1`, the language
//! selection (`--lang`, `VA_LANG`, locale) and its warning, and closed output pipes.
#![allow(clippy::unwrap_used)]
#![cfg(feature = "arch-m68k")]

use std::ffi::OsString;
use std::process::{Command, Output, Stdio};

use serde_json::Value;
use va_arch_m68k::M68K;
use va_core::arch::Architecture as _;
use va_workspace::i18n::{Lang, Localizer};

/// Exit code of a command that ran and found problems.
const EXIT_PROBLEMS: i32 = 1;

/// Exit code of a usage error.
const EXIT_USAGE: i32 = 2;

/// Exit code of an I/O or environment error.
const EXIT_IO: i32 = 3;

/// Builds a command running the `va` binary under test with `args`.
///
/// The language variables are removed so the user's locale cannot change the texts, and
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

/// Decodes a captured stream as UTF-8 (which also proves French text reaches a pipe as UTF-8).
fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).unwrap()
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

/// Asserts that every needle occurs in `haystack`, each after the end of the previous one.
fn assert_in_order(haystack: &str, needles: &[&str]) {
    let mut from = 0;
    for needle in needles {
        let found = haystack[from..]
            .find(needle)
            .unwrap_or_else(|| panic!("{needle:?} not found after byte {from} in {haystack}"));
        from += found + needle.len();
    }
}

/// The summary of the documentation entry `id` in `lang`.
fn summary(lang: Lang, id: &str) -> String {
    Localizer::for_architecture(lang, &M68K)
        .unwrap()
        .message(id, None, &[])
        .unwrap()
}

/// Runs `va isa <query> --json` and returns its document, asserting exit 0 and a quiet stderr.
fn describe(query: &str) -> Value {
    let output = run_va(&["isa", query, "--json"]);
    assert_eq!(output.status.code(), Some(0), "query: {query}");
    assert_eq!(text(&output.stderr), "", "query: {query}");
    json_line(&output.stdout).1
}

/// The `syntax` of every form of a `va.isa/1` document.
fn syntaxes(document: &Value) -> Vec<&str> {
    document["forms"]
        .as_array()
        .unwrap()
        .iter()
        .map(|form| form["syntax"].as_str().unwrap())
        .collect()
}

#[test]
fn move_text_starts_with_its_summary_and_shows_flags() {
    let output = run_va(&["isa", "MOVE"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(text(&output.stderr), "");
    let stdout = text(&output.stdout);
    let first = stdout.lines().next().unwrap();
    assert_eq!(
        first,
        format!("MOVE: {}", summary(Lang::En, "isa-m68k-move"))
    );
    // The ordinary MOVE: N and Z from the result, V and C cleared, X unaffected.
    assert!(
        stdout.lines().any(|line| line == "  flags: X- N* Z* V0 C0"),
        "stdout: {stdout}"
    );
    // MOVE to CCR sets every flag from the source.
    assert!(
        stdout.lines().any(|line| line == "  flags: X* N* Z* V* C*"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("\nalso accepts: MOVEA\n"),
        "stdout: {stdout}"
    );
    let flag_lines: Vec<&str> = stdout
        .lines()
        .filter(|line| line.starts_with("  flags: "))
        .collect();
    assert_eq!(
        flag_lines.len(),
        describe("MOVE")["forms"].as_array().unwrap().len()
    );
    for line in flag_lines {
        let effects: Vec<&str> = line["  flags: ".len()..].split(' ').collect();
        assert_eq!(effects.len(), 5, "line: {line}");
        for (effect, flag) in effects.iter().zip(["X", "N", "Z", "V", "C"]) {
            assert!(effect.starts_with(flag), "line: {line}");
            assert!(
                ["-", "*", "0", "1", "U", "!"].contains(&&effect[1..]),
                "line: {line}"
            );
        }
    }
}

/// The `va.isa/1` document of MOVE: key order, schema fields, `ea_timing` present because the
/// rows add an EA time.
#[test]
fn move_json_has_the_documented_keys_in_order() {
    let output = run_va(&["isa", "MOVE", "--json"]);
    assert_eq!(output.status.code(), Some(0));
    let (line, document) = json_line(&output.stdout);
    assert_in_order(
        &line,
        &[
            r#"{"schema":"va.isa/1","arch":"m68k","lang":"en","query":"MOVE","spelling":"MOVE","#,
            r#""canonical":"MOVE","family":null,"condition":null,"aliases":[],"#,
            r#""also_accepts":["MOVEA"],"size":null,"summary":"#,
            r#","details":"#,
            r#","flags_note":null,"forms":[{"syntax":"MOVE.B <ea>,<ea>","sizes":["B"],"#,
            r#""default_size":null,"privileged":false,"operands":[{"notation":"<ea>","modes":["Dn","#,
            r#"]}],"flags":[{"flag":"X","effect":"unaffected"},{"flag":"N","effect":"result"},"#,
            r#""timing":[{"sizes":["B"],"when":[{"operand":1,"modes":["Dn"]}],"outcome":null,"clocks":"#,
            r#","reads":"#,
            r#","writes":"#,
            r#","plus_ea":[0],"per_unit":null,"bound":"exact"}"#,
            r#""ea_timing":[{"mode":"Dn","sizes":["B","W"],"clocks":"#,
        ],
    );
    assert!(line.ends_with("}]}"), "line: {line}");
    assert_eq!(document["summary"], summary(Lang::En, "isa-m68k-move"));
    assert!(document["details"].as_str().unwrap().len() > 40);
    assert_eq!(
        document["ea_timing"].as_array().unwrap().len(),
        M68K.instruction_set().ea_timing().len()
    );
    assert!(!document["ea_timing"].as_array().unwrap().is_empty());
}

/// `move.l` keeps only the forms that accept `.L`, and normalizes the size.
#[test]
fn move_l_filters_the_forms_by_size() {
    let document = describe("move.l");
    assert_eq!(document["query"], "move.l");
    assert_eq!(document["spelling"], "MOVE");
    assert_eq!(document["size"], "L");
    let forms = document["forms"].as_array().unwrap();
    assert!(!forms.is_empty());
    for form in forms {
        let sizes = form["sizes"].as_array().unwrap();
        assert!(sizes.contains(&Value::from("L")), "form: {form}");
    }
    assert!(!syntaxes(&document).contains(&"MOVE.B <ea>,<ea>"));
}

/// m1 §7.4 as amended by §12.6: `beq.s` gives the single `.S` form; without a size, `.S` and `.W`
/// are separate forms; outcome-independent rows have `outcome: null` (checked on MOVE above).
#[test]
fn beq_has_separate_short_and_word_forms() {
    let output = run_va(&["--lang", "fr", "isa", "beq.s", "--json"]);
    assert_eq!(output.status.code(), Some(0));
    let (line, document) = json_line(&output.stdout);
    assert_in_order(
        &line,
        &[
            r#"{"schema":"va.isa/1","arch":"m68k","lang":"fr","query":"beq.s","spelling":"BEQ","#,
            r#""canonical":"BEQ","family":"Bcc","condition":{"name":"EQ","test":"Z","summary":"#,
            r#"},"aliases":[],"also_accepts":[],"size":"S","summary":"#,
            r#","forms":[{"syntax":"BEQ.S <label>","sizes":["S"],"default_size":null,"#,
            r#""privileged":false,"operands":[{"notation":"<label>","modes":[]}],"#,
            r#""flags":[{"flag":"X","effect":"unaffected"},{"flag":"N","effect":"unaffected"},"#,
            r#""timing":[{"sizes":["S"],"when":[],"outcome":"taken","clocks":"#,
            r#""plus_ea":[],"per_unit":null,"bound":"exact"}"#,
            r#""outcome":"not-taken""#,
            r#"],"ea_timing":[]}"#,
        ],
    );
    assert_eq!(syntaxes(&document), ["BEQ.S <label>"]);
    assert_eq!(
        document["condition"]["summary"],
        summary(Lang::Fr, "isa-m68k-cond-eq")
    );
    assert_eq!(document["summary"], summary(Lang::Fr, "isa-m68k-bcc"));

    let without_size = describe("BEQ");
    assert_eq!(syntaxes(&without_size), ["BEQ.S <label>", "BEQ.W <label>"]);
    assert_eq!(without_size["forms"][0]["sizes"], serde_json::json!(["S"]));
    assert_eq!(without_size["forms"][1]["sizes"], serde_json::json!(["W"]));
    assert_eq!(without_size["size"], Value::Null);

    let output = run_va(&["isa", "beq"]);
    let stdout = text(&output.stdout);
    assert!(stdout.contains("\ncondition: EQ (Z) "), "stdout: {stdout}");
    assert!(stdout.contains("\nBEQ.S <label>\n"), "stdout: {stdout}");
    assert!(stdout.contains("\nBEQ.W <label>\n"), "stdout: {stdout}");
}

#[test]
fn dbra_is_an_alias_of_dbf() {
    let document = describe("DBRA");
    assert_eq!(document["spelling"], "DBRA");
    assert_eq!(document["canonical"], "DBF");
    assert_eq!(document["family"], "DBcc");
    assert!(
        document["aliases"]
            .as_array()
            .unwrap()
            .contains(&Value::from("DBF")),
        "document: {document}"
    );
}

#[test]
fn add_also_accepts_adda_and_addi() {
    let document = describe("add");
    assert_eq!(document["spelling"], "ADD");
    assert_eq!(
        document["also_accepts"],
        serde_json::json!(["ADDA", "ADDI"])
    );
}

/// `--list` gives the 126 spellings in order, with summaries, in both modes.
#[test]
fn list_gives_every_spelling_in_order() {
    let spellings = M68K.instruction_set().spellings();
    assert_eq!(spellings.len(), 126);

    let output = run_va(&["isa", "--list", "--json"]);
    assert_eq!(output.status.code(), Some(0));
    let (line, document) = json_line(&output.stdout);
    assert!(
        line.starts_with(
            r#"{"schema":"va.isa.list/1","arch":"m68k","lang":"en","mnemonics":[{"spelling":"ABCD","canonical":"ABCD","summary":"#
        ),
        "line: {line}"
    );
    let mnemonics = document["mnemonics"].as_array().unwrap();
    let listed: Vec<&str> = mnemonics
        .iter()
        .map(|entry| entry["spelling"].as_str().unwrap())
        .collect();
    assert_eq!(listed, spellings);
    let dbra = mnemonics
        .iter()
        .find(|entry| entry["spelling"] == "DBRA")
        .unwrap();
    assert_eq!(dbra["canonical"], "DBF");
    assert_eq!(dbra["summary"], summary(Lang::En, "isa-m68k-dbcc"));

    let output = run_va(&["isa", "--list"]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = text(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 126);
    for (line, spelling) in lines.iter().zip(spellings) {
        assert!(line.starts_with(&format!("{spelling}  ")), "line: {line}");
    }
}

/// An unknown spelling exits 1 with up to 3 suggestions; JSON mode adds a `va.error/1`
/// document whose message is the human line without its prefix.
#[test]
fn unknown_mnemonic_exits_1_with_suggestions() {
    let output = run_va(&["isa", "MOV"]);
    assert_eq!(output.status.code(), Some(EXIT_PROBLEMS));
    assert_eq!(text(&output.stdout), "");
    let stderr = text(&output.stderr);
    let suggestions = stderr
        .strip_prefix("error: unknown mnemonic 'MOV' (did you mean ")
        .and_then(|rest| rest.strip_suffix("?)\n"))
        .unwrap_or_else(|| panic!("stderr: {stderr}"));
    let suggestions: Vec<&str> = suggestions.split(", ").collect();
    assert!((1..=3).contains(&suggestions.len()), "stderr: {stderr}");
    assert_eq!(suggestions[0], "MOVE");
    for suggestion in &suggestions {
        assert!(suggestion.starts_with("MOV"), "stderr: {stderr}");
    }

    let output = run_va(&["--json", "isa", "MOV"]);
    assert_eq!(output.status.code(), Some(EXIT_PROBLEMS));
    let (line, document) = json_line(&output.stdout);
    assert!(
        line.starts_with(r#"{"schema":"va.error/1","code":"isa-unknown-mnemonic","message":"#),
        "line: {line}"
    );
    assert_eq!(
        text(&output.stderr),
        format!("error: {}\n", document["message"].as_str().unwrap())
    );

    // Nothing within edit distance 2: no suggestion part.
    let output = run_va(&["isa", "xyzzy"]);
    assert_eq!(output.status.code(), Some(EXIT_PROBLEMS));
    assert_eq!(text(&output.stderr), "error: unknown mnemonic 'xyzzy'\n");
}

/// A size no form accepts exits 1 with `isa-size-not-accepted`; `chk.l` stays refused until
/// M2 (m1 §12.4).
#[test]
fn size_not_accepted_exits_1() {
    for (query, spelling) in [("moveq.b", "MOVEQ"), ("chk.l", "CHK"), ("nop.w", "NOP")] {
        let output = run_va(&["isa", query, "--json"]);
        assert_eq!(output.status.code(), Some(EXIT_PROBLEMS), "query: {query}");
        let (_, document) = json_line(&output.stdout);
        assert_eq!(document["schema"], "va.error/1", "query: {query}");
        assert_eq!(document["code"], "isa-size-not-accepted", "query: {query}");
        let message = document["message"].as_str().unwrap();
        assert!(
            message.starts_with(&format!("{spelling} does not accept the size suffix")),
            "message: {message}"
        );
        assert_eq!(text(&output.stderr), format!("error: {message}\n"));
    }
}

/// `--lang fr` gives the French summary, as UTF-8 bytes on a pipe (fragile point F9).
#[test]
fn lang_fr_gives_french_texts() {
    let english = run_va(&["isa", "move.l"]);
    let french = run_va(&["--lang", "fr", "isa", "move.l"]);
    assert_eq!(french.status.code(), Some(0));
    let english = text(&english.stdout);
    let french = text(&french.stdout);
    let english_first = english.lines().next().unwrap();
    let french_first = french.lines().next().unwrap();
    assert_ne!(english_first, french_first);
    assert_eq!(
        french_first,
        format!("MOVE: {}", summary(Lang::Fr, "isa-m68k-move"))
    );
    assert!(
        french.bytes().any(|byte| byte >= 0x80),
        "French accents expected"
    );
    // Only the localized texts change: the form blocks are identical.
    let forms = |stdout: &str| stdout[stdout.find("\n\nMOVE.").unwrap()..].to_owned();
    assert_eq!(forms(&english), forms(&french));
}

/// The language precedence of m1 §6.2 and §7.1, and the warning for an ignored `VA_LANG`.
#[test]
fn va_lang_selects_the_language_and_bad_values_are_reported() {
    let english = format!("NOP: {}\n", summary(Lang::En, "isa-m68k-nop"));
    let french = format!("NOP: {}\n", summary(Lang::Fr, "isa-m68k-nop"));
    let first_line = |output: &Output| {
        let stdout = text(&output.stdout);
        format!("{}\n", stdout.lines().next().unwrap())
    };

    let output = va_command(&["isa", "nop"])
        .env("VA_LANG", "fr")
        .output()
        .unwrap();
    assert_eq!(first_line(&output), french);
    assert_eq!(text(&output.stderr), "");

    for bad in ["xx", "", "FR", "fr_FR"] {
        let output = va_command(&["isa", "nop"])
            .env("VA_LANG", bad)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(0), "VA_LANG={bad}");
        assert_eq!(first_line(&output), english, "VA_LANG={bad}");
        assert_eq!(
            text(&output.stderr),
            format!("warning: ignoring VA_LANG={bad} (expected en, fr or auto)\n")
        );
    }

    // After an ignored VA_LANG, `auto` still reads the locale.
    let output = va_command(&["isa", "nop"])
        .env("VA_LANG", "xx")
        .env("LANG", "fr_FR.UTF-8")
        .output()
        .unwrap();
    assert_eq!(first_line(&output), french);

    // `--lang` wins and no variable is read, so nothing is reported.
    let output = va_command(&["--lang", "en", "isa", "nop"])
        .env("VA_LANG", "xx")
        .env("LC_ALL", "fr_FR.UTF-8")
        .output()
        .unwrap();
    assert_eq!(first_line(&output), english);
    assert_eq!(text(&output.stderr), "");

    // `VA_LANG=auto` and the locale variables, first non-empty wins.
    let output = va_command(&["isa", "nop"])
        .env("VA_LANG", "auto")
        .env("LC_ALL", "")
        .env("LC_MESSAGES", "fr_CA.UTF-8")
        .env("LANG", "en_US.UTF-8")
        .output()
        .unwrap();
    assert_eq!(first_line(&output), french);
    assert_eq!(text(&output.stderr), "");

    // A non-Unicode VA_LANG is read lossily and reported, not silently taken as unset.
    let output = va_command(&["isa", "nop"])
        .env("VA_LANG", non_unicode_value())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(first_line(&output), english);
    assert_eq!(
        text(&output.stderr),
        "warning: ignoring VA_LANG=f\u{FFFD}r (expected en, fr or auto)\n"
    );
}

/// Returns `f<invalid>r`: an invalid UTF-8 byte on Unix, an unpaired UTF-16 surrogate on
/// Windows.
fn non_unicode_value() -> OsString {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt as _;
        OsString::from_vec(vec![b'f', 0xff, b'r'])
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt as _;
        OsString::from_wide(&[u16::from(b'f'), 0xD800, u16::from(b'r')])
    }
}

/// `va isa` takes exactly one of MNEMONIC and `--list`, and `--lang` only `en` or `fr`.
#[test]
fn malformed_isa_command_lines_are_usage_errors() {
    for args in [
        &["isa"][..],
        &["isa", "MOVE", "--list"],
        &["isa", "MOVE", "ADD"],
        &["--lang", "de", "isa", "nop"],
        &["--lang", "auto", "isa", "nop"],
        &["--lang", "FR", "isa", "nop"],
    ] {
        let output = run_va(args);
        assert_eq!(output.status.code(), Some(EXIT_USAGE), "args: {args:?}");
        assert_eq!(text(&output.stdout), "", "args: {args:?}");
        let stderr = text(&output.stderr);
        assert!(
            stderr.starts_with("error:"),
            "args: {args:?}, stderr: {stderr}"
        );
    }
}

/// A stdout pipe with no reader exits 3, in every mode.
#[test]
fn closed_stdout_pipe_is_io_error() {
    for args in [
        &["isa", "MOVE"][..],
        &["isa", "MOVE", "--json"],
        &["isa", "--list"],
        &["isa", "--list", "--json"],
    ] {
        let (reader, writer) = std::io::pipe().unwrap();
        drop(reader);
        let output = va_command(args)
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
