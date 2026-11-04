//! Tests of `session.toml` (m4-editor-shell.md §4.7, §10.1): `parse_session` and
//! `session_to_toml` round trip, the written layout, and the tolerance to bad parts (syntax,
//! schema version, wrong types, ranges, themes, roots, tabs, the active tab, unknown keys).

#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use va_workspace::config::ThemeKind;
use va_workspace::persist::{
    BackupId, Session, SessionTab, SessionUi, SessionWindow, SessionWorkspace, TabSource,
    parse_session, session_to_toml,
};

/// An absolute path on this host (the package folder, which cargo gives as absolute).
fn abs(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fake-root")
        .join(name)
}

/// `abs(name)` as a TOML literal string, for hand-written session texts.
fn abs_toml(name: &str) -> String {
    let path = abs(name);
    let path = path.to_str().unwrap();
    assert!(!path.contains('\''));
    format!("'{path}'")
}

/// A well-formed backup id.
fn id(text: &str) -> BackupId {
    BackupId::parse(text).unwrap()
}

/// A tab with the given source and caret.
fn tab(source: TabSource, line: u32, column: u32, top_line: u32, active: bool) -> SessionTab {
    SessionTab {
        source,
        line,
        column,
        top_line,
        active,
    }
}

/// Parses `text` and returns the session.
fn parse(text: &str) -> Session {
    parse_session(Some(text))
}

#[test]
fn a_missing_or_empty_file_gives_the_defaults() {
    let default = Session::default();
    assert_eq!(parse_session(None), default);
    assert_eq!(parse(""), default);
    assert_eq!(parse("schema_version = 1\n"), default);
    assert_eq!(default.window.width, 1280.0);
    assert_eq!(default.window.height, 800.0);
    assert!(!default.window.maximized);
    assert_eq!(default.ui.zoom, 1.0);
    assert_eq!(default.ui.theme, None);
    assert!(default.ui.explorer_visible);
    assert_eq!(default.ui.explorer_width, 240.0);
    assert_eq!(default.workspace.root, None);
    assert!(default.tabs.is_empty());
}

#[test]
fn a_syntax_error_or_another_schema_version_gives_the_defaults() {
    let body = "window = { width = 900.0, height = 700.0, maximized = true }\n";
    assert_ne!(parse(body), Session::default(), "the body alone is read");
    for header in [
        "schema_version = 2\n",
        "schema_version = 0\n",
        "schema_version = \"1\"\n",
        "schema_version = 1.0\n",
        "schema_version = true\n",
    ] {
        assert_eq!(
            parse(&format!("{header}{body}")),
            Session::default(),
            "{header:?}"
        );
    }
    for broken in [
        "schema_version = 1\nwindow = {",
        "schema_version = 1\n[[tab]\n",
        "= 3\n",
        "schema_version = 1\nwindow = 1\nwindow = 2\n",
    ] {
        assert_eq!(parse(broken), Session::default(), "{broken:?}");
    }
}

#[test]
fn a_missing_schema_version_means_1() {
    let session = parse("window = { width = 900.0, height = 700.0, maximized = true }\n");
    assert_eq!(
        session.window,
        SessionWindow {
            width: 900.0,
            height: 700.0,
            maximized: true
        }
    );
}

#[test]
fn the_example_of_the_spec_parses() {
    let text = format!(
        "schema_version = 1\n\
         window = {{ width = 1280.0, height = 800.0, maximized = false }}   # logical px\n\
         ui = {{ zoom = 1.0, theme = \"dark\", explorer_visible = true, explorer_width = 240.0 }}\n\
         workspace = {{ root = {root} }}\n\
         [[tab]]                    # in tab order\n\
         path = {main}\n\
         line = 12                  # caret, 1-based\n\
         column = 5\n\
         top_line = 1\n\
         active = true\n",
        root = abs_toml("tp1"),
        main = abs_toml("tp1/main.asm"),
    );
    let session = parse(&text);
    assert_eq!(session.window, SessionWindow::default());
    assert_eq!(
        session.ui,
        SessionUi {
            theme: Some(ThemeKind::Dark),
            ..SessionUi::default()
        }
    );
    assert_eq!(session.workspace.root, Some(abs("tp1")));
    assert_eq!(
        session.tabs,
        [tab(TabSource::File(abs("tp1/main.asm")), 12, 5, 1, true)]
    );
}

/// Sessions with in-range values and absolute UTF-8 paths.
fn sample_sessions() -> Vec<Session> {
    let mut full = Session {
        window: SessionWindow {
            width: 1920.5,
            height: 1080.0,
            maximized: true,
        },
        ui: SessionUi {
            zoom: 1.1,
            theme: Some(ThemeKind::Light),
            explorer_visible: false,
            explorer_width: 333.25,
        },
        workspace: SessionWorkspace {
            root: Some(abs("tp 1")),
        },
        tabs: vec![
            tab(TabSource::File(abs("tp 1/main.asm")), 12, 5, 1, false),
            tab(TabSource::Backup(id("b0123456789abcdef")), 1, 1, 1, true),
            tab(
                TabSource::File(abs("tp 1/quote\"back\\slash é 日本.s")),
                u32::MAX,
                80,
                4_000_000,
                false,
            ),
        ],
    };
    let bounds = Session {
        window: SessionWindow {
            width: 400.0,
            height: 300.0,
            maximized: false,
        },
        ui: SessionUi {
            zoom: 0.5,
            theme: Some(ThemeKind::Dark),
            explorer_visible: true,
            explorer_width: 120.0,
        },
        ..Session::default()
    };
    let top = Session {
        window: SessionWindow {
            width: 10_000.0,
            height: 10_000.0,
            maximized: false,
        },
        ui: SessionUi {
            zoom: 3.0,
            explorer_width: 2000.0,
            ..SessionUi::default()
        },
        ..Session::default()
    };
    let odd = Session {
        window: SessionWindow {
            width: 1234.567,
            ..SessionWindow::default()
        },
        ui: SessionUi {
            zoom: 1.234_567_9,
            ..SessionUi::default()
        },
        workspace: SessionWorkspace {
            root: Some(abs("tab\there\nnew line\u{1}\u{7f}")),
        },
        tabs: Vec::new(),
    };
    full.tabs.push(tab(
        TabSource::Backup(id("bffffffffffffffff")),
        2,
        3,
        4,
        false,
    ));
    vec![Session::default(), full, bounds, top, odd]
}

#[test]
fn sessions_round_trip_through_the_text() {
    for session in sample_sessions() {
        let text = session_to_toml(&session);
        assert_eq!(parse(&text), session, "{text}");
        assert_eq!(session_to_toml(&parse(&text)), text, "the text is stable");
    }
}

#[test]
fn the_written_layout_is_fixed() {
    assert_eq!(
        session_to_toml(&Session::default()),
        "schema_version = 1\n\
         window = { width = 1280.0, height = 800.0, maximized = false }\n\
         ui = { zoom = 1.0, explorer_visible = true, explorer_width = 240.0 }\n\
         workspace = {}\n"
    );
    let mut session = Session::default();
    session.ui.zoom = 1.1;
    session.ui.theme = Some(ThemeKind::Dark);
    session.workspace.root = Some(abs("tp1"));
    session.tabs.push(tab(
        TabSource::Backup(id("b00000000000000a1")),
        3,
        2,
        1,
        true,
    ));
    let root = abs("tp1");
    let root = root.to_str().unwrap().replace('\\', "\\\\");
    assert_eq!(
        session_to_toml(&session),
        format!(
            "schema_version = 1\n\
             window = {{ width = 1280.0, height = 800.0, maximized = false }}\n\
             ui = {{ zoom = 1.1, theme = \"dark\", explorer_visible = true, explorer_width = 240.0 }}\n\
             workspace = {{ root = \"{root}\" }}\n\
             \n\
             [[tab]]\n\
             backup = \"b00000000000000a1\"\n\
             line = 3\n\
             column = 2\n\
             top_line = 1\n\
             active = true\n"
        )
    );
}

#[test]
fn out_of_range_values_take_their_defaults() {
    let session = parse(
        "window = { width = 399.9, height = 10000.5, maximized = false }\n\
         ui = { zoom = 0.49, explorer_width = 2001.0 }\n",
    );
    assert_eq!(session, Session::default());
    let session = parse(
        "window = { width = 10001.0, height = 299.0 }\n\
         ui = { zoom = 3.01, explorer_width = 119.0 }\n",
    );
    assert_eq!(session, Session::default());
    let session = parse(
        "window = { width = nan, height = inf }\nui = { zoom = -inf, explorer_width = -240.0 }\n",
    );
    assert_eq!(session, Session::default());
    // A non-finite value in memory is written as a TOML special float and read as out of range.
    let mut odd = Session::default();
    odd.ui.zoom = f32::NAN;
    odd.window.width = f32::INFINITY;
    odd.window.height = f32::NEG_INFINITY;
    assert_eq!(parse(&session_to_toml(&odd)), Session::default());
}

#[test]
fn a_float_key_takes_an_integer() {
    let session =
        parse("window = { width = 1024, height = 768 }\nui = { zoom = 2, explorer_width = 300 }\n");
    assert_eq!(session.window.width, 1024.0);
    assert_eq!(session.window.height, 768.0);
    assert_eq!(session.ui.zoom, 2.0);
    assert_eq!(session.ui.explorer_width, 300.0);
}

#[test]
fn values_of_the_wrong_type_take_their_defaults() {
    let text = "window = { width = \"wide\", height = true, maximized = \"yes\" }\n\
                ui = { zoom = [1.0], theme = 1, explorer_visible = 0, explorer_width = {} }\n\
                workspace = { root = 7 }\n";
    assert_eq!(parse(text), Session::default());
    // A table of the wrong type is ignored as a whole; the others are still read.
    let text = "window = 5\nui = \"big\"\nworkspace = []\ntab = 3\n\
                [extra]\nzoom = 2.0\n";
    assert_eq!(parse(text), Session::default());
    let mixed = parse("window = { width = 900.0, height = \"tall\" }\n");
    assert_eq!(mixed.window.width, 900.0, "a bad key keeps its neighbours");
    assert_eq!(mixed.window.height, 800.0);
}

#[test]
fn only_light_and_dark_are_theme_overrides() {
    let theme = |value: &str| parse(&format!("ui = {{ theme = {value} }}\n")).ui.theme;
    assert_eq!(theme("\"light\""), Some(ThemeKind::Light));
    assert_eq!(theme("\"dark\""), Some(ThemeKind::Dark));
    for value in [
        "\"auto\"",
        "\"Dark\"",
        "\" dark\"",
        "\"\"",
        "\"blue\"",
        "true",
    ] {
        assert_eq!(theme(value), None, "{value}");
    }
}

#[test]
fn a_root_that_is_not_absolute_is_none() {
    let root = |value: &str| {
        parse(&format!("workspace = {{ root = {value} }}\n"))
            .workspace
            .root
    };
    assert_eq!(root(&abs_toml("tp1")), Some(abs("tp1")));
    for value in ["\"tp1\"", "\"./tp1\"", "\"\"", "\"../up\""] {
        assert_eq!(root(value), None, "{value}");
    }
    #[cfg(windows)]
    assert_eq!(root("\"/home/me\""), None, "not absolute on Windows");
}

#[test]
fn a_tab_needs_exactly_one_absolute_path_or_well_formed_backup() {
    let tabs = |body: &str| parse(&format!("[[tab]]\n{body}\n")).tabs;
    let main = abs("main.asm");
    assert_eq!(
        tabs(&format!("path = {}", abs_toml("main.asm"))),
        [tab(TabSource::File(main.clone()), 1, 1, 1, false)]
    );
    assert_eq!(
        tabs("backup = \"b0000000000000001\""),
        [tab(
            TabSource::Backup(id("b0000000000000001")),
            1,
            1,
            1,
            false
        )]
    );
    for dropped in [
        String::new(),
        "line = 4".to_owned(),
        "path = \"main.asm\"".to_owned(),
        "path = 3".to_owned(),
        format!(
            "path = {}\nbackup = \"b0000000000000001\"",
            abs_toml("main.asm")
        ),
        "path = \"main.asm\"\nbackup = \"b0000000000000001\"".to_owned(),
        "backup = \"b000000000000001\"".to_owned(),
        "backup = \"b00000000000000001\"".to_owned(),
        "backup = \"b000000000000000A\"".to_owned(),
        "backup = \"x0000000000000001\"".to_owned(),
        "backup = \"b000000000000000g\"".to_owned(),
        "backup = \"../../etc/passwd\"".to_owned(),
        "backup = 1".to_owned(),
    ] {
        assert!(tabs(&dropped).is_empty(), "{dropped:?}");
    }
    // A dropped tab keeps its neighbours, and an array element that is not a table is skipped.
    let session = parse(&format!(
        "tab = [{{ path = {a} }}, 5, {{ path = \"relative\" }}, {{ backup = \"b00000000000000ff\" }}]\n",
        a = abs_toml("a.asm")
    ));
    assert_eq!(
        session.tabs,
        [
            tab(TabSource::File(abs("a.asm")), 1, 1, 1, false),
            tab(TabSource::Backup(id("b00000000000000ff")), 1, 1, 1, false),
        ]
    );
}

#[test]
fn caret_positions_are_1_based_and_bad_ones_take_1() {
    let positions = |body: &str| {
        let session = parse(&format!(
            "[[tab]]\nbackup = \"b0000000000000001\"\n{body}\n"
        ));
        let tab = &session.tabs[0];
        (tab.line, tab.column, tab.top_line)
    };
    assert_eq!(positions("line = 7\ncolumn = 3\ntop_line = 2"), (7, 3, 2));
    assert_eq!(
        positions("line = 0\ncolumn = -4\ntop_line = 4294967296"),
        (1, 1, 1)
    );
    assert_eq!(
        positions("line = 4294967295\ncolumn = 2.0\ntop_line = \"9\""),
        (u32::MAX, 1, 1)
    );
}

#[test]
fn only_the_first_active_tab_stays_active() {
    let text = (1..=4)
        .map(|n| {
            format!(
                "[[tab]]\nbackup = \"b000000000000000{n}\"\nactive = {}\n",
                n > 1
            )
        })
        .collect::<String>();
    let active: Vec<bool> = parse(&text).tabs.iter().map(|tab| tab.active).collect();
    assert_eq!(active, [false, true, false, false]);
    let text = "[[tab]]\nbackup = \"b0000000000000001\"\nactive = \"yes\"\n";
    assert!(!parse(text).tabs[0].active);
}

#[test]
fn unknown_keys_are_ignored() {
    let text = format!(
        "schema_version = 1\nfuture = {{ a = 1 }}\nnote = \"hi\"\n\
         window = {{ width = 900.0, depth = 3 }}\n\
         ui = {{ zoom = 2.0, sparkles = true }}\n\
         workspace = {{ root = {root}, branch = \"main\" }}\n\
         [[tab]]\npath = {main}\nfolded = [1, 2]\n",
        root = abs_toml("tp1"),
        main = abs_toml("tp1/main.asm"),
    );
    let session = parse(&text);
    assert_eq!(session.window.width, 900.0);
    assert_eq!(session.ui.zoom, 2.0);
    assert_eq!(session.workspace.root, Some(abs("tp1")));
    assert_eq!(session.tabs.len(), 1);
}

#[test]
fn a_byte_order_mark_is_skipped() {
    let session = parse("\u{feff}schema_version = 1\nui = { zoom = 2.0 }\n");
    assert_eq!(session.ui.zoom, 2.0);
}

#[cfg(unix)]
#[test]
fn a_path_that_is_not_unicode_is_not_written() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    let bad = abs("tp1").join(OsStr::from_bytes(b"caf\xe9.asm"));
    let mut session = Session::default();
    session.workspace.root = Some(abs("tp1").join(OsStr::from_bytes(b"\xff")));
    session.tabs = vec![
        tab(TabSource::File(bad), 2, 2, 1, true),
        tab(TabSource::File(abs("ok.asm")), 3, 3, 1, false),
    ];
    let text = session_to_toml(&session);
    assert!(text.contains("workspace = {}\n"), "{text}");
    let parsed = parse(&text);
    assert_eq!(parsed.workspace.root, None);
    assert_eq!(
        parsed.tabs,
        [tab(TabSource::File(abs("ok.asm")), 3, 3, 1, false)]
    );
}
