# M4 — Editor and GUI shell: file-level specification

| | |
|---|---|
| Status | Ready for development. The new dependencies, the font download and the UX defaults were approved by the user on 2026-10-24 (§8.3, §8.4, §13); the architecture amendments this spec needs are ratified in `architecture.md` (2026-10-24); the merged wave order with M2 is §14. The settings types of §4.3 and the session types of §4.7 were frozen on 2026-10-24 after WP0 (WP0 follow-up, §1). The wave-1 implementation (WP0 `b4e7aae`, follow-up `570afb1`) is ratified in §15 (2026-10-25), which wins over the body; wave-2 batch A (WP-A, WP-B, WP-D) is ratified in §16 (2026-11-02), which wins over §15 and the body; wave-2 batch B (WP-C, WP-E, WP-H) is ratified in §17 (2026-11-04), which wins over §16, §15 and the body, adds the fix packages M4 FX-1 and M4 FX-2 and orders batch C |
| Owner | Architect role |
| Date | 2026-10-24 |
| Depends on | `architecture.md` (§2 R1–R8, §4 T1–T6, §5, §6, §7, §9.6, §9.8); `m0-foundations.md` §4, §6, §17.10, §19 (renderer, fallback, bench v2); `m1-core-isa-lexer.md` §3 (frozen `va-core`), §5 (lexer), §6.2 (Localizer) |
| Parallel milestone | M2 (`docs/specs/m2-assembler.md`). M4 never edits `arch/*`, `apps/va` or `crates/va-core`; the only shared files are the Makefile and the closing documents (§14). |
| Milestone goal | A usable editor shell: rope buffer with multi-cursor and grouped undo, regex find/replace, TOML settings and keybindings, portable mode, hot exit with crash restore, Fluent UI in EN/FR, and a virtualized, lexer-coloured editor with tabs, explorer, palette and find bar, measured by an extended `cargo xtask bench` |

Normative words: **must** = required; *rejected* = considered and refused, with the reason in one line. Blocks marked *frozen* are
binding signatures: implementations may add private items, derives and extra public helpers, never remove or rename a listed item. A
change goes back to the Architect.

## 0. Exit criteria (all required)

| # | Criterion | How it is checked |
|---|---|---|
| E1 | `make test`, `make lint`, `make docs`, `make security`, `make ci` exit 0 on Windows and in WSL Ubuntu 26.04 | QA, both hosts |
| E2 | CI green on the 4 runners (jobs unchanged) | `gh run list --workflow ci.yml` |
| E3 | `va-editor` tests of §10.1 pass, including the ropey line count equal to `va_core::source::LineIndex` on LF, CRLF, CR and mixed text (m1 F7) | `cargo test -p va-editor` |
| E4 | `va-workspace` tests pass, including the **kill test**: 20 kills of a process writing backups, each restore yields one complete earlier version, never a torn or missing one (§4.8) | `cargo test -p va-workspace` |
| E5 | `va-ui` kittest suite (§10.2): on a 100,000-line document only visible lines are laid out; one keystroke re-lexes exactly one line; at rest no repaint is requested (blink off), and with blink on the wake-ups stop after 10 s; palette, find bar, tabs, explorer, zoom, theme and FR/EN switching work headless | `cargo test -p va-ui` |
| E6 | UI i18n completeness: every referenced `ui-*`/`cmd-*`/`diag-cfg-*` id exists in `en` and `fr`, both have identical id and attribute sets, no orphan (§4.6) | `cargo test -p va-ui --test locales`, `cargo test -p va-workspace --test i18n_ui` |
| E7 | Renderer precedence `--renderer` > `VA_RENDERER` > `render.renderer` > `auto` (§6.3); every M0 headless renderer test still passes; the ignored fallback tests pass on Windows and WSLg X11 (m0 §19.9 S19-5) | `cargo test -p visual-assembler`; QA locally |
| E8 | Performance on the reference machine (on mains, "Best performance") meets every target of §11; report `docs/perf/<date>-m4-editor.md`. A miss blocks M4 acceptance (never CI) and returns to the Architect | Performance agent |
| E9 | Manual checklist §10.4 completed on Windows 11 and WSLg X11, including a kill by Task Manager / `kill -9` and the restore | QA |
| E10 | Security review: no new `unsafe`, network or allow-list entry; §9 items hold; `SECURITY.md` updated | Security agent |
| E11 | Legal: the font's OFL text is shipped (embedded and in `assets/fonts/`), `DEPENDANCE.md`/`IMPORT.md` list every new crate and the font; French strings in our own words | QA, Documentarian |

## 1. Work packages and order

All interfaces are frozen here, so wave 2 runs in parallel. Write scopes are disjoint. Developers write no README; each WP hands the
Documentarian its new files with a one-line purpose.

| WP | Wave | Write scope | Needs (frozen APIs only) |
|---|---|---|---|
| WP0 scaffold | 1 (∥ M2 WP-A, §14) | Root `Cargo.toml`, `Cargo.lock`, `Makefile` (§8); manifests of `va-editor`, `va-workspace`, `va-ui`, `visual-assembler`; compiling stubs that wave 2 then owns, one for every module the crate roots declare: `crates/va-editor/src/lib.rs`, `crates/va-ui/src/{lib.rs,theme.rs,fonts.rs,editor/mod.rs,shell/mod.rs}`, `crates/va-workspace/src/{lib.rs,paths.rs,config/{mod.rs,default_settings.toml},persist/mod.rs}`; `crates/va-ui/tests/support/mod.rs` (the fake `SyntaxProvider`/`Architecture` and harness helpers, §10); `locales/{en,fr}/{ui,commands,config}.ftl` (header comment only); `assets/fonts/{JetBrainsMono-Regular.ttf,OFL.txt}` (§8.4) | user approvals (given 2026-10-24) |
| WP-A editor | 2 | `crates/va-editor/src/**`, `crates/va-editor/tests/**` (§3) | — |
| WP-B config | 2 | `crates/va-workspace/src/{paths.rs,config/**}`, `tests/{paths,config_settings,config_keybindings,config_project}.rs`, `locales/{en,fr}/config.ftl` (§4.1–§4.5) | — |
| WP-C persist | 2 | `crates/va-workspace/src/persist/**`, `tests/persist_*.rs` (§4.7–§4.8) | WP-B `Paths`; `ThemeKind` (§4.3) |
| WP-D i18n | 2 | `crates/va-workspace/src/i18n/**` (adds `ui.rs`, `system.rs`), `tests/i18n_ui.rs`, `crates/va-ui/tests/locales.rs` (§4.6) | — |
| WP-E view | 2 | `crates/va-ui/src/{editor/**,theme.rs,fonts.rs}`, `tests/{editor_view,editor_input,editor_cache,theme,fonts}.rs`, `tests/support/mod.rs` after WP0 (§5.2–§5.6) | WP-A |
| WP-F shell | 2 | `crates/va-ui/src/{lib.rs,shell/**}`, `tests/shell_*.rs`, `locales/{en,fr}/{ui,commands}.ftl` (§5.1, §5.7–§5.13) | WP-A–E |
| WP-G app | 2, finishes after E/F | `apps/visual-assembler/src/**`, `apps/visual-assembler/tests/**` (§6) | all of the above |
| WP-H bench | 2 | `xtask/src/bench/**`, `xtask/tests/bench_*.rs` (§7) | §7 contract only |
| — | 3 | Security, QA | `SECURITY.md`; QA reports |
| — | 4 | Performance | `docs/perf/<date>-m4-editor.md` (§11) |
| WP-I docs | 5 | Documentarian | Every README (new folders included), `CLAUDE.md`, `AGENCEMENT.md`, `DEPENDANCE.md`, `IMPORT.md` |

Wave 1 is one developer, beside M2's WP-A (§14). The user approved the font download on 2026-10-24 (§8.4). WP-E and WP-F can start
on day 1 with a stub `Document` built from §3; WP-G starts with stub shells and wires the real ones as they land. WP-H needs nothing
from the GUI until the end-to-end run, which uses WP-G's recorder (§6.8). Crate roots (`lib.rs`) and `tests/support/mod.rs` already
declare every module after WP0, so no wave-2 package edits a file another one owns: WP-F keeps its own test helpers inside its
`shell_*.rs` files and asks WP-E for a change to `support/mod.rs`.

**WP0 follow-up (frozen 2026-10-24 (after WP0); wave 1, after commit `b4e7aae`, before the wave-1c docs checkpoint and wave 2).**
WP0 stubbed §4.3 and `Session` as empty structs because they were not frozen yet. WP0 now writes into its own stubs: the §4.3
types into `config/mod.rs` plus the new `config/default_settings.toml` and `DEFAULT_SETTINGS_TOML`, and the session types of §4.7
into `persist/mod.rs`. The `Default` impls, the enums' `id`/`from_id` and the file content are written in full (pure data, no
`todo!()`); `parse_settings`, `parse_session` and `session_to_toml` stay `todo!()` for WP-B and WP-C.

## 2. File tree (new or changed at the end of M4)

```
Cargo.toml, Cargo.lock, Makefile                                   # §8
assets/README.md, assets/fonts/{README.md,JetBrainsMono-Regular.ttf,OFL.txt}
locales/README.md, locales/{en,fr}/{ui,commands,config}.ftl        # §4.6
crates/va-editor/src/{lib,text,encoding,selection,edit,history,search,document}.rs, tests/{same names but lib}.rs
crates/va-workspace/src/{paths.rs, config/{mod,settings,keybindings,project,problems}.rs,
                         persist/{mod,atomic,worker,session,backup,lock}.rs,
                         i18n/{mod,lang,localizer,ui,system}.rs}
crates/va-workspace/src/config/default_settings.toml              # embedded commented default
crates/va-workspace/tests/{paths,config_settings,config_keybindings,config_project,
                           persist_atomic,persist_session,persist_backup,persist_lock,persist_kill,i18n_ui}.rs
crates/va-ui/src/{lib,theme,fonts}.rs, crates/va-ui/src/editor/{mod,view,layout,cache,input,caret,gutter,plain}.rs
crates/va-ui/src/shell/{mod,state,tabs,explorer,palette,fuzzy,find,status,commands,keymap,prompts}.rs
crates/va-ui/tests/{editor_view,editor_input,editor_cache,theme,fonts,locales,
                    shell_tabs,shell_explorer,shell_palette,shell_find,shell_status,shell_hot_exit}.rs
crates/va-ui/tests/support/mod.rs                                  # fake SyntaxProvider/Architecture, harness helpers
apps/visual-assembler/src/{lib,main,cli,shell_app,arch_registry,host}.rs   # spike.rs removed
apps/visual-assembler/src/bench/{mod,recorder,report,script,platform,process,process_windows,process_linux}.rs
apps/visual-assembler/tests/{cli,binary,bench_report,bench_recorder,bench_script,bench_process,
                             renderer_backend,fallback,shell_app,arch_registry}.rs   # spike.rs removed
xtask/src/bench/generate.rs, xtask/tests/bench_generate.rs (new); runner, report, mod and their tests (changed)
```

- `assets/` and `locales/` are the root folders of architecture §11; no root file is added. Every new folder that holds a file
  (`locales/en/`, `locales/fr/`, `assets/fonts/`, the new `src/` sub-folders, `tests/support/`) gets its README from the
  Documentarian at the docs checkpoint after wave 1b (§14), then in WP-I.
- `va-ui/src/shell/` holds 11 files plus its README (limit 12). `tests/support/` is a module folder of shared test code (`mod
  support;` in each test file).
- `apps/visual-assembler/src/spike.rs` and `tests/spike.rs` are deleted: the M0 i18n exception ends (CLAUDE.md conventions).
  `WINDOW_TITLE = "VisualAssembler"` (a product name, not translated) moves to `lib.rs`.

## 3. `va-editor` (WP-A): buffer, selections, undo, search

Pure library: no threads, no I/O beyond `std::io::Write` sinks, no UI, depends on `va-core`, `ropey`, `regex`. All offsets are
**bytes** into the document text (after a stripped BOM), on char boundaries; lines are **0-based** `usize` in `va-editor` and
`va-ui`; 1-based `LineCol` appears only in display and in `va-core` types. Time is passed in as `Duration` since an arbitrary
origin, so tests control it.

### 3.1 `text.rs`, `encoding.rs` (frozen)

```rust
pub const MAX_DOCUMENT_BYTES: usize = va_core::source::MAX_SOURCE_BYTES;     // 16 MiB
pub struct Text { /* private: ropey::Rope */ }        // Clone is O(1) (shared nodes); Send + Sync
impl Text {
    pub fn new(text: &str) -> Self;
    pub fn len_bytes(&self) -> usize;  pub fn line_count(&self) -> usize;   // >= 1, = LineIndex::line_count
    pub fn line_start(&self, line: usize) -> usize;   pub fn line_end(&self, line: usize) -> usize; // content end, before the terminator; line clamped
    pub fn line_of(&self, byte: usize) -> usize;                            // byte clamped to len
    pub fn line_text_into(&self, line: usize, out: &mut String);           // clears `out`, copies the content
    pub fn slice_to_string(&self, start: usize, end: usize) -> String;
    pub fn chunks(&self) -> impl Iterator<Item = &str> + '_;
    pub fn write_to(&self, out: &mut dyn std::io::Write) -> std::io::Result<()>;
    pub fn widest_line_chars(&self) -> usize;                              // O(n): load and replace only
}
pub fn display_col(line: &str, byte_in_line: usize, tab_width: u8) -> u32;      // tab → next multiple of tab_width
pub fn byte_at_display_col(line: &str, col: u32, tab_width: u8) -> usize;      // char boundary at or before col

pub enum Encoding { Utf8, Utf8Bom, Windows1252 }      // id(): "utf-8" | "utf-8-bom" | "windows-1252"
pub enum LineEnding { Lf, CrLf, Cr }                  // id(): "lf" | "crlf" | "cr"; as_str(): "\n" …
pub enum DecodeError { TooLarge { bytes: usize }, NotUtf8 { offset: usize }, Binary { offset: usize } }
pub fn decode(bytes: &[u8], forced: Option<Encoding>) -> Result<(String, Encoding), DecodeError>;
pub fn first_unencodable(text: &Text, encoding: Encoding) -> Option<(usize, char)>;
pub fn encode_to(text: &Text, encoding: Encoding, out: &mut dyn std::io::Write) -> std::io::Result<()>;
pub fn detect_line_ending(text: &Text) -> Option<LineEnding>;            // first terminator in the text
```

- ropey is built with `default-features = false, features = ["simd", "cr_lines"]` (m1 §3.2): line breaks are LF, CRLF and lone CR,
  never U+0085/U+2028. Terminators are stored verbatim, so a mixed file keeps every original terminator; a newly inserted break uses
  `Document::line_ending`.
- `decode(bytes, None)`: over 16 MiB → `TooLarge`; a NUL byte in the first 8 KiB → `Binary`; a UTF-8 BOM → `Utf8Bom` (stripped);
  valid UTF-8 → `Utf8`; else `NotUtf8` (first invalid byte). `forced = Some(Windows1252)` decodes every byte (0x80–0x9F through the
  WHATWG table; the five undefined bytes map to U+0081, U+008D, U+008F, U+0090, U+009D, so the round trip is lossless). The table is
  hand-written (32 entries). *Rejected:* `encoding_rs`, ~1 MB of tables for one code page.
- `encode_to` writes the BOM for `Utf8Bom`; for `Windows1252` it fails with `InvalidData` on the first char outside the code page;
  callers check `first_unencodable` first (§5.7 save flow).

### 3.2 `selection.rs`, `edit.rs` (frozen)

```rust
pub const MAX_CARETS: usize = 10_000;
pub struct Selection { pub anchor: usize, pub head: usize, pub goal: Option<u32> }  // head = caret; goal = display column of vertical moves
impl Selection { pub fn caret(at: usize) -> Self; pub fn start(&self) -> usize; pub fn end(&self) -> usize; pub fn is_empty(&self) -> bool; }
pub struct Selections { /* private: non-empty, sorted, non-overlapping, one primary */ }
impl Selections { pub fn single(s: Selection) -> Self; pub fn from_vec(v: Vec<Selection>, primary: usize) -> Self; // sorts, merges, caps
    pub fn iter(&self) -> impl Iterator<Item = &Selection>; pub fn len(&self) -> usize; pub fn primary(&self) -> &Selection;
    pub fn primary_index(&self) -> usize; pub fn push(&mut self, s: Selection) /* merged, becomes primary */; pub fn collapse_to_primary(&mut self); }
pub struct Change { pub start: usize, pub end: usize, pub insert: String }   // old-text bytes
pub struct ChangeSet { /* private: sorted, non-overlapping changes */ }
pub enum ChangeError { Overlap, OutOfBounds, NotCharBoundary }
pub enum Assoc { Before, After }
impl ChangeSet { pub fn new(changes: Vec<Change>, text: &Text) -> Result<Self, ChangeError>; pub fn changes(&self) -> &[Change];
                 pub fn map_offset(&self, byte: usize, assoc: Assoc) -> usize; }
pub enum EditCommand { InsertText(String), Paste(String), NewLine, Tab, Indent, Outdent, Backspace, Delete, DeleteWordLeft,
    DeleteWordRight, DeleteSelection, DeleteLines, DuplicateLines, MoveLinesUp, MoveLinesDown, ToggleLineComment { prefix: &'static str } }
pub enum Motion { Left, Right, WordLeft, WordRight, Up, Down, LineStart, LineEnd,
                  PageUp(u32), PageDown(u32), DocStart, DocEnd, To(usize) }
pub struct EditOptions { pub tab_width: u8, pub insert_spaces: bool }
pub fn is_word_char(c: char) -> bool;     // alphanumeric, '_', '.', '@' (assembly identifiers, A68k §5.2)
```

Command semantics (each applies at every selection, back to front, in one `ChangeSet`):

| Command | Rule |
|---|---|
| `InsertText` | Replaces each selection with the text |
| `Paste` | If the text has exactly N lines and there are N > 1 carets, line i goes to caret i; else as `InsertText` |
| `NewLine` | Inserts the document line ending plus the leading whitespace (spaces/tabs) of the current line |
| `Tab` | Empty selections: a `\t`, or spaces to the next tab stop when `insert_spaces`; a selection spanning lines: `Indent` |
| `Indent` / `Outdent` | Adds / removes one level (`\t`, or `tab_width` spaces, or up to that many spaces) at each touched line start |
| `Backspace` / `Delete` | Deletes the selection, else one char before / after the caret (a `\r\n` pair counts as one) |
| `DeleteWord*` | Deletes to the previous / next word boundary (`is_word_char` runs, then whitespace runs) |
| `DeleteLines`, `DuplicateLines`, `MoveLines*` | On every line touched by a selection; selections follow the moved text |
| `ToggleLineComment` | If every non-blank touched line starts (after indentation) with `prefix`, removes it and one following space; else inserts `prefix` + space after the smallest common indentation. The prefix comes from the architecture registry (§6.7), never hard-coded in `va-editor` |

Motions: `Left`/`Right` collapse a non-empty selection to its start/end unless `extend`; `Up`/`Down` keep the display column in
`goal`; `LineStart` is smart home (first non-blank, then column 0); `Page*(n)` moves n lines. Word motions use `is_word_char`.

### 3.3 `history.rs`, `document.rs` (frozen)

```rust
pub const UNDO_GROUP_MS: u64 = 1_000;  pub const MAX_UNDO_GROUPS: usize = 10_000;
pub const MAX_UNDO_BYTES: usize = 64 * 1024 * 1024;
pub enum EditKind { Typing, Deleting, Other }
pub struct Snapshot { pub text: Text, pub revision: u64, pub encoding: Encoding, pub line_ending: LineEnding } // Send + Sync, O(1)
pub struct Document { /* private */ }
impl Document {
    pub fn new(text: Text, encoding: Encoding, default_eol: LineEnding) -> Self;   // eol = detected, else default
    pub fn text(&self) -> &Text;  pub fn selections(&self) -> &Selections;  pub fn set_selections(&mut self, s: Selections);
    pub fn revision(&self) -> u64;                       // +1 per applied change, undo and redo included
    pub fn is_dirty(&self) -> bool;  pub fn mark_saved(&mut self);
    pub fn encoding(&self) -> Encoding;  pub fn set_encoding(&mut self, e: Encoding);   // makes the document dirty
    pub fn line_ending(&self) -> LineEnding;  pub fn widest_line_hint(&self) -> usize;
    pub fn edit(&mut self, cmd: EditCommand, opts: &EditOptions, at: Duration) -> bool;  // false: nothing changed
    pub fn apply(&mut self, changes: ChangeSet, kind: EditKind, at: Duration);         // external edits (M5 quick fix)
    pub fn replace_text(&mut self, text: Text);          // one undo group holding both texts: replace-all, reload
    pub fn move_carets(&mut self, motion: Motion, extend: bool, tab_width: u8);  pub fn add_caret_vertical(&mut self, up: bool, tab_width: u8);
    pub fn select_next_occurrence(&mut self) -> bool;    // Ctrl+D: word under caret, then next match of the primary selection
    pub fn select_word_at(&mut self, byte: usize);  pub fn select_line(&mut self, line: usize);  pub fn selected_texts(&self) -> Vec<String>;
    pub fn undo(&mut self) -> bool;  pub fn redo(&mut self) -> bool;  pub fn snapshot(&self) -> Snapshot;
}
```

- **Undo groups.** An edit merges into the open group only when all hold: same kind (`Typing` for `InsertText` of one char per
  caret, `Deleting` for `Backspace`/`Delete`), each caret's edit starts where its previous edit ended, less than `UNDO_GROUP_MS`
  since the previous edit, and the inserted text is not a line break. Anything else (selection change by the user, `Other` commands,
  paste, `replace_text`) closes the group. Undo restores the text and the selections from before the group; any new edit clears the
  redo stack.
- **Limits.** Past `MAX_UNDO_GROUPS` or `MAX_UNDO_BYTES` (inserted plus deleted text kept), the oldest groups are dropped. A
  `replace_text` group costs two O(1) `Text` clones.
- **Dirty state.** Each content state has an id; undo and redo restore the ids, so undoing back to the saved state makes the
  document clean again. `set_encoding` and a changed line ending count as content changes.
- **No O(n) per keystroke.** `edit`, `move_carets`, `undo` and `snapshot` must be O(log n + size of the change) for a single caret;
  only `Text::new`, `widest_line_chars`, `replace_text` and search are O(n). `widest_line_hint` only grows on edits (exact again
  after `replace_text`).

### 3.4 `search.rs` (frozen)

```rust
pub struct Query { pub pattern: String, pub regex: bool, pub case_sensitive: bool, pub whole_word: bool }
pub enum QueryError { Empty, Invalid { detail: String }, TooBig }   // detail: the regex crate's text, shown raw
pub struct Matcher { /* private: regex::Regex */ }                  // Clone + Send + Sync
impl Matcher { pub fn new(query: &Query) -> Result<Self, QueryError>; }
pub const MAX_MATCHES: usize = 100_000;  pub const REGEX_SIZE_LIMIT: usize = 1 << 20;  pub const REGEX_NEST_LIMIT: u32 = 64;
pub struct Matches { pub ranges: Vec<(usize, usize)>, pub truncated: bool }
pub fn find_all(text: &Text, m: &Matcher, cancel: &AtomicBool) -> Option<Matches>;     // None when cancelled
pub fn find_from(text: &Text, m: &Matcher, from: usize, backwards: bool) -> Option<(usize, usize)>; // wraps once
pub fn expand_replacement(m: &Matcher, line: &str, found: (usize, usize), template: &str) -> String;
pub fn replace_all(text: &Text, m: &Matcher, template: &str, cancel: &AtomicBool) -> Option<(Text, usize)>;
```

- **Line by line.** Each line's content (no terminator) is searched separately: no match spans a line break, `^`/`$` are line edges,
  empty matches are skipped. *Rejected:* matching the whole text, which needs a contiguous copy of it and makes `.` and `\s` cross
  lines unexpectedly.
- A literal query is `regex::escape`d; `case_sensitive = false` adds `(?i)` (Unicode case folding, so `É` finds `é`); `whole_word`
  keeps a match only if the chars before and after it are not `is_word_char`. `RegexBuilder` gets `size_limit(REGEX_SIZE_LIMIT)`,
  `dfa_size_limit(2 * REGEX_SIZE_LIMIT)` and `nest_limit(REGEX_NEST_LIMIT)`; exceeding one gives `TooBig`. The `regex` crate runs in
  linear time, so no input can hang the search (§9).
- Templates: regex mode expands `$1`, `${name}` and `$$`; literal mode inserts the template as is.
- `find_all` and `replace_all` check `cancel` every 1,024 lines; both stop collecting at `MAX_MATCHES` (`truncated`). All three
  search functions and `Matcher::new` run on the `va-search` worker (§5.10), never on the UI thread: `find_from` can scan the whole
  document when the next match is far away, and a pattern near the size limits compiles in more than 1 ms (architecture T1).

## 4. `va-workspace` (WP-B, WP-C, WP-D)

No UI, depends on `va-core`, `fluent-bundle`, `unic-langid`, `toml`, `serde`, and `sys-locale` on Windows. It knows nothing of ropey
or egui: text crosses its API as `&str` or as a writer closure.

### 4.1 Paths and portable mode (`paths.rs`, WP-B; frozen)

```rust
pub struct Paths { pub config_dir: PathBuf, pub data_dir: PathBuf, pub portable: bool }
pub struct PathInputs<'a> { pub os: &'a str, pub exe_dir: Option<&'a Path>, pub var: &'a dyn Fn(&str) -> Option<OsString>,
                            pub is_file: &'a dyn Fn(&Path) -> bool }
pub enum PathWarning { NotAbsolute { var: &'static str }, Empty { var: &'static str } }   // English Display, for stderr
pub fn resolve_paths(inputs: &PathInputs<'_>) -> (Option<Paths>, Vec<PathWarning>);   // None: no usable location
impl Paths { pub fn settings(&self) -> PathBuf; pub fn keybindings(&self) -> PathBuf; pub fn session(&self) -> PathBuf;
             pub fn session_lock(&self) -> PathBuf; pub fn backups_dir(&self) -> PathBuf; pub fn backups_index(&self) -> PathBuf; }
```

| Level (highest first) | Config dir | Data dir |
|---|---|---|
| Environment | `VA_CONFIG_DIR` | `VA_DATA_DIR` |
| Portable: a regular file named exactly `portable` next to the executable | `<exe dir>/data/config` | `<exe dir>/data/data` |
| Windows | `%APPDATA%\VisualAssembler\config` (else `%USERPROFILE%\AppData\Roaming\…`) | `%LOCALAPPDATA%\VisualAssembler\data` (else `%USERPROFILE%\AppData\Local\…`) |
| Linux | `$XDG_CONFIG_HOME/visualassembler` (else `$HOME/.config/visualassembler`) | `$XDG_DATA_HOME/visualassembler` (else `$HOME/.local/share/visualassembler`) |

- Each variable is used only when non-empty and absolute; otherwise it is skipped with a `PathWarning` (the XDG rule for relative
  values). Each directory is chosen independently.
- Folders are created on first write only (`create_dir_all`); on Unix the data dir and `backups/` get mode `0o700` (std
  `DirBuilderExt`). Files live at `settings.toml`, `keybindings.toml` (config) and `session.toml`, `session.lock`,
  `backups/index.toml`, `backups/<id>.txt` (data).
- **Decision: std-only resolution.** *Rejected:* `directories` 6.0.0 (3 crates, one MPL-2.0 that needs a cargo-deny exception) for
  two environment lookups; the folders are the ones architecture §6 names. A portable folder that cannot be written makes the
  session read-only (§4.8).

### 4.2 Config files and problems (`config/{mod,problems}.rs`, WP-B; frozen)

```rust
pub const MAX_CONFIG_BYTES: usize = 1024 * 1024;
pub struct ConfigProblem { pub path: PathBuf, pub code: DiagCode, pub range: TextRange, pub at: LineCol,
                           pub args: Vec<(&'static str, DiagArg)> }
impl ConfigProblem { pub fn to_diagnostic(&self, file: FileId) -> Diagnostic; }   // Severity::Warning, non-blocking
pub static CONFIG_CODES: [DiagCode; 12];
pub fn read_config_text(path: &Path) -> Result<Option<String>, ConfigProblem>;     // Ok(None): file missing
```

| Code | When | Args |
|---|---|---|
| `cfg-unreadable` | I/O error other than "not found" | `error` (English io kind) |
| `cfg-too-large` | more than `MAX_CONFIG_BYTES` | `bytes` |
| `cfg-not-utf8` | invalid UTF-8 | `offset` |
| `cfg-syntax` | TOML syntax error (whole file falls back to defaults) | `detail` (toml's text) |
| `cfg-schema-version` | `schema_version` present and not `1` (file ignored) | `found` |
| `cfg-unknown-key` | key not in the schema (ignored) | `key` |
| `cfg-wrong-type` | value of the wrong TOML type (default kept) | `key`, `expected` |
| `cfg-out-of-range` | number outside its range (default kept) | `key`, `min`, `max` |
| `cfg-invalid-value` | string not among the allowed values (default kept) | `key`, `value`, `expected` |
| `cfg-unknown-command` | keybinding names an unknown command (binding skipped) | `command` |
| `cfg-bad-key` | keybinding key string not parsable (binding skipped) | `key` |
| `cfg-bad-path` | path value absolute where a relative one is required, or containing `..` (default kept) | `key`, `value` |

- Parsing uses `toml::de::DeTable::parse`, which keeps spans, then walks the schema by hand: a bad key never discards its
  neighbours. *Rejected:* `serde` derive with `deny_unknown_fields`, where one bad value rejects the whole file and spans are lost.
- A missing `schema_version` means 1, silently. Problems are shown in the UI (§5.11), printed on stderr as `warning:
  <path>:<line>:<col>: <message>` (English, path and values through `escape_debug` as m0 §19.9 S19-2), and become M5 Problems-panel
  entries via `to_diagnostic`.

### 4.3 `settings.toml` schema (`config/settings.rs`, WP-B; frozen 2026-10-24 (after WP0))

```rust
// The six structs: derive Clone, Debug, PartialEq; `Default` is hand-written and equals the Default column below.
pub struct Settings { pub ui: UiSettings, pub editor: EditorSettings, pub files: FileSettings,
                      pub explorer: ExplorerSettings, pub render: RenderSettings }
pub struct UiSettings { pub language: Option<Lang>, pub theme: Option<ThemeKind>, pub native_dialogs: bool }
pub struct EditorSettings { pub font_size: f32, pub font_file: Option<PathBuf>, pub tab_width: u8,
                            pub insert_spaces: bool, pub cursor_blink: bool, pub line_numbers: bool,
                            pub highlight_current_line: bool, pub diagnostics_delay_ms: u32 }
pub struct FileSettings { pub hot_exit: bool, pub backup_delay_ms: u32, pub autosave: AutosaveMode,
                          pub autosave_delay_ms: u32, pub eol: Option<EolKind> }
pub struct ExplorerSettings { pub exclude: Vec<String> }
pub struct RenderSettings { pub renderer: Option<RendererKind> }
// The four enums: derive Clone, Copy, Debug, PartialEq, Eq, Hash; AutosaveMode also derives Default (`Off`).
pub enum ThemeKind { Light, Dark }                            // ids "light" | "dark"
pub enum EolKind { Lf, CrLf }                                 // ids "lf" | "crlf"
pub enum RendererKind { Glow, Wgpu }                          // ids "glow" | "wgpu"
pub enum AutosaveMode { Off, AfterDelay, OnFocusChange }      // ids "off" | "after-delay" | "on-focus-change"
impl ThemeKind { pub const fn id(self) -> &'static str; pub fn from_id(id: &str) -> Option<Self>; } // same on the other three
pub fn parse_settings(path: &Path, text: Option<&str>) -> (Settings, Vec<ConfigProblem>);
pub const DEFAULT_SETTINGS_TOML: &str = include_str!("default_settings.toml");
```

- **Names.** Each struct is one table (`[ui]`, `[editor]`, `[files]`, `[explorer]`, `[render]`) and each field is named exactly as
  its key. `Lang` is `crate::i18n::Lang`. Public paths: `va_workspace::config::*` (re-exported if WP-B moves them to `settings.rs`).
- **`None` means `"auto"`** for `language`, `theme`, `eol` and `renderer` (absent, `"auto"` and an invalid value all give `None`),
  so the values pass unchanged to `resolve_ui_lang(…, setting, …)` (§4.6) and to `resolve_renderer(…, setting, …)` (§6.3).
  `font_file = ""` is `None`.
- **Values.** String values are exact lowercase ids (`from_id`, or `Lang::parse`; `"auto"` is handled by the parser, so `"Glow"` and
  `" glow"` give `cfg-invalid-value`). A float key also takes a TOML integer (`font_size = 16`), and `nan`/`inf` are out of range;
  an integer key never takes a float. Ranges are inclusive. `font_file` must end in `.ttf` or `.otf` (ASCII case-insensitive), else
  `cfg-invalid-value`; it is stored as written, and a relative path is joined to `Paths::config_dir` where the font is read (§5.5).
  Each `exclude` element must be a non-empty string without `/` or `\`: a bad element is dropped (`cfg-wrong-type` or
  `cfg-invalid-value`) and the others are kept; `exclude = []` hides nothing.

| Key | Type | Default | Allowed | Effect |
|---|---|---|---|---|
| `ui.language` | string | `"auto"` | `auto`, `en`, `fr` | UI and diagnostics language (§4.6); read before the window, applied at the next start |
| `ui.theme` | string | `"auto"` | `auto` (follows the OS), `light`, `dark` | Base theme; the palette toggle overrides it in `session.toml`. Default: user decision 2026-10-24 (§13) |
| `ui.native_dialogs` | bool | `true` | — | Windows: native open/save dialogs; `false` (and always on Linux) uses the internal path prompt (§6.6) |
| `editor.font_size` | float | `14.0` | 8.0–40.0 | Code font size in points, before zoom |
| `editor.font_file` | string | `""` | path to a `.ttf`/`.otf`, absolute or relative to the config dir | Replaces the embedded font (§5.5) |
| `editor.tab_width` | integer | `8` | 1–16 | Tab stops. Default: user decision 2026-10-24 (§13) |
| `editor.insert_spaces` | bool | `false` | — | Tab inserts spaces; `false` = a tab character (user decision 2026-10-24) |
| `editor.cursor_blink` | bool | `true` | — | §5.6. Default: user decision 2026-10-24 (§13) |
| `editor.line_numbers` | bool | `true` | — | Gutter numbers |
| `editor.highlight_current_line` | bool | `true` | — | Current-line background |
| `editor.diagnostics_delay_ms` | integer | `300` | 50–5000 | Parsed now, used by M5 (architecture T3); no effect in M4 |
| `files.hot_exit` | bool | `true` | — | §4.8. Default: user decision 2026-10-24 (§13) |
| `files.backup_delay_ms` | integer | `1500` | 250–60000 | Delay after the last edit before a backup |
| `files.autosave` | string | `"off"` | `off`, `after-delay`, `on-focus-change` | Writes the student's file; only on the user's opt-in (§4.8) |
| `files.autosave_delay_ms` | integer | `1000` | 250–60000 | For `after-delay` |
| `files.eol` | string | `"auto"` | `auto` (CRLF on Windows, LF elsewhere), `lf`, `crlf` | Line ending of new files only |
| `explorer.exclude` | array of strings | `[".git"]` | non-empty names (no `/` or `\`) | Entries hidden in the explorer and quick open; case-insensitive on Windows |
| `render.renderer` | string | `"auto"` | `auto`, `glow`, `wgpu` | §6.3; read before the window, applied at the next start |

- **`DEFAULT_SETTINGS_TOML`.** The file is `crates/va-workspace/src/config/default_settings.toml`. The `include_str!` above sits in
  `config/mod.rs` (WP0 follow-up), or in `config/settings.rs` once WP-B moves it: same folder, same argument. Its content is exactly
  the block below (ASCII, LF, one final newline): `schema_version = 1`, then each table header, then each key commented out with
  its default under one English comment line. Tests (`config_settings.rs`): the text parses to `Settings::default()` with no
  problem; it has exactly one line matching `^# [a-z_]+ = ` per row of the table above, in table order; with all of them
  uncommented it still parses to `Settings::default()` with no problem.

```toml
# VisualAssembler settings. Every key is listed commented out, with its default value.
# To change a value, remove the "# " in front of its key. A bad value is reported and ignored.
# The app creates this file once, when it is missing, and never rewrites it.
schema_version = 1

[ui]
# Interface language: "auto" (from the system), "en" or "fr"; applies at the next start.
# language = "auto"
# Colour theme: "auto" (follows the system), "light" or "dark".
# theme = "auto"
# Windows: use the native open and save dialogs; false uses the built-in path prompt.
# native_dialogs = true

[editor]
# Code font size in points, 8.0 to 40.0, before zoom.
# font_size = 14.0
# A .ttf or .otf file replacing the embedded font, absolute or relative to this folder.
# font_file = ""
# Width of a tab stop in columns, 1 to 16.
# tab_width = 8
# true: the Tab key inserts spaces; false: it inserts a tab character.
# insert_spaces = false
# Blinking caret; it stays solid after 10 s without input.
# cursor_blink = true
# Show line numbers in the gutter.
# line_numbers = true
# Highlight the line of the caret.
# highlight_current_line = true
# Delay in ms before diagnostics refresh after an edit, 50 to 5000 (not used yet).
# diagnostics_delay_ms = 300

[files]
# Hot exit: closing the window keeps unsaved changes and restores them at the next start.
# hot_exit = true
# Delay in ms after the last edit before unsaved changes are backed up, 250 to 60000.
# backup_delay_ms = 1500
# Save your files automatically: "off", "after-delay" or "on-focus-change".
# autosave = "off"
# Delay in ms of the "after-delay" autosave, 250 to 60000.
# autosave_delay_ms = 1000
# Line ending of new files: "auto" (CRLF on Windows, LF elsewhere), "lf" or "crlf".
# eol = "auto"

[explorer]
# Names hidden in the explorer and in quick open.
# exclude = [".git"]

[render]
# Renderer: "auto", "glow" or "wgpu"; applies at the next start (--renderer and VA_RENDERER win).
# renderer = "auto"
```

- The app writes `settings.toml` only once, when it is missing, by writing `DEFAULT_SETTINGS_TOML` with `write_atomic` (architecture
  §6): at §6.2 step 7 (WP-G) and in `app.open-settings` (§5.12, WP-F); never in bench mode or a read-only session.
- Reload: on window focus gain, and right after the IDE itself saves the file, settings and keybindings are re-read if their length
  or mtime changed; every key applies live except `render.renderer` and `ui.language` (status note "applies at next start"):
  `ShellConfig::lang` (§5.1) does not record whether `--lang` or `VA_LANG` chose it, so a live switch could override them.
  *Rejected:* a live `ui.language`, which needs that source in the frozen `ShellConfig`. `ui.native_dialogs` applies live because
  the shell checks it before each `ShellHost::native_dialog` call (§6.6).
- **Ownership.**

| Piece | Lands with | Owner in wave 2 | Read by |
|---|---|---|---|
| §4.3 types, `Default` impls, enum `id`/`from_id`, `default_settings.toml`, `DEFAULT_SETTINGS_TOML` | WP0 follow-up (§1), `config/mod.rs` | WP-B (content frozen: a change goes back to the Architect) | all |
| `parse_settings` and `config_settings.rs` | WP0 stub (`todo!()`) | WP-B | WP-G at start (§6.2 step 3), WP-F on reload |
| Session types of §4.7 and their `Default` | WP0 follow-up, `persist/mod.rs` | WP-C (may move them to `persist/session.rs`, re-exported) | WP-G, WP-F |
| `parse_session`, `session_to_toml`, `persist_session.rs` | WP0 stub (`todo!()`) | WP-C | WP-G at start, WP-F |

  Keys by reader: `ui.language` and `render.renderer`: WP-G only (`resolve_ui_lang`, `resolve_renderer`). Every other key: WP-F,
  which builds `EditorStyle` and `EditOptions` from `editor.*`, the effective theme (§5.4) from `ui.theme`, the font request (§5.5)
  from `editor.font_file`, and runs `files.*` (§4.8) and `explorer.exclude` (§5.8, §5.9). WP-E reads `EditorStyle` and
  `EditorContext`, never `Settings`. `editor.diagnostics_delay_ms` has no reader in M4.

### 4.4 `keybindings.toml` schema (`config/keybindings.rs`, WP-B; frozen)

```toml
schema_version = 1
[[binding]]
key = "Ctrl+Shift+P"                   # [Ctrl+][Shift+][Alt+]<key>; modifier names case-insensitive
command = "palette.commands"           # an id of §5.12
when = "global"                        # optional: global (default) | editor | explorer
[[binding]]
key = "Ctrl+D"
command = "-edit.select-next-occurrence"   # leading '-': removes that default binding of the key
```

```rust
pub struct KeyChord { pub ctrl: bool, pub shift: bool, pub alt: bool, pub key: &'static str } // canonical name
pub enum When { Global, Editor, Explorer }
pub struct Binding { pub chord: KeyChord, pub command: String, pub when: When }
pub static KEY_NAMES: [&str; 83];      // canonical egui 0.36 `Key::name()` spellings, see below
pub fn parse_chord(text: &str) -> Option<KeyChord>;
pub fn parse_keybindings(path: &Path, text: Option<&str>, defaults: &[Binding],
                         is_command: &dyn Fn(&str) -> bool) -> (Vec<Binding>, Vec<ConfigProblem>);
```

- Key names: `A`–`Z`, `0`–`9`, `F1`–`F20`, `Up`, `Down`, `Left`, `Right`, `Home`, `End`, `PageUp`, `PageDown`, `Insert`, `Delete`,
  `Backspace`, `Enter`, `Tab`, `Escape`, `Space`, `Comma`, `Period`, `Slash`, `Backslash`, `Semicolon`, `Quote`, `Backtick`,
  `Minus`, `Plus`, `Equals`, `OpenBracket`, `CloseBracket` (83). Aliases accepted on input: `ArrowUp`/`ArrowDown`/
  `ArrowLeft`/`ArrowRight`, `Esc`, `Return`. Key names are case-insensitive. `Ctrl` is egui's `Modifiers::COMMAND` (macOS is a
  non-goal).
- Resolution: defaults, then removals (`-id`), then user bindings in file order; a user binding replaces any binding of the same
  chord and `when`. Unknown commands and bad keys are skipped with a problem. Single chords only. *Rejected:* chords such as `Ctrl+K
  Ctrl+S` (state machine and timeout for a handful of commands).
- Not rebindable in M4: caret motion and basic editing keys (arrows with Shift/Ctrl, Home, End, Page keys, Backspace, Delete, Enter,
  Tab, Shift+Tab, Escape) and the clipboard keys, which egui-winit turns into `Event::Copy/Cut/Paste`.

### 4.5 `.visualasm/project.toml` (`config/project.rs`, WP-B; frozen)

```toml
schema_version = 1
[project]
arch = "m68k"            # id of a registered architecture; default: the app's default architecture
io_profile = "easy68k"   # M3 meaning; M4 stores the string
entry = "main.asm"       # relative to the project root, no ".." component
include_dirs = ["inc"]   # same rule; used by the GUI assembler from M5 (`va asm` takes -I instead)
# [assembler], [output], [run], [debug]: reserved for later milestones (M3, M5, M6), kept raw without warnings
```

```rust
pub struct Project { pub root: PathBuf, pub arch: Option<String>, pub io_profile: Option<String>,
                     pub entry: Option<PathBuf>, pub include_dirs: Vec<PathBuf>,
                     pub reserved: BTreeMap<String, toml::Table> }
pub fn parse_project(root: &Path, text: Option<&str>) -> (Project, Vec<ConfigProblem>);
pub const PROJECT_FILE: &str = ".visualasm/project.toml";
```

- "Open folder" makes that folder the project root; `project.toml` is read if present and never created implicitly (only the
  "Project settings" command, `project.settings`, creates it from a commented template and opens it). An unknown `arch` is reported
  by the app (`cfg-invalid-value`) and the default architecture is used.
- Paths are relative without `..` (else `cfg-bad-path`), so a downloaded project cannot point the assembler's `INCLUDE` search
  outside its root before the M7 sandbox. M2's `va asm` never reads `project.toml` (its include folders are `-I` arguments, m2 §8.1);
  the M5 GUI provider reads `include_dirs` and also applies m2 §6.2.

### 4.6 UI localization (`i18n/{ui,system}.rs`, WP-D)

- **Files.** `locales/<lang>/ui.ftl` (`ui-*`: shell text), `commands.ftl` (`cmd-*`: command titles, one per command id with `.` and
  `-` both mapped to `-`, e.g. `cmd-edit-toggle-comment`), `config.ftl` (`diag-cfg-*`, with `.help` where the fix is not obvious).
  Embedded with `include_str!` by `va-workspace`. User overrides in `<config>/locales/<lang>/*.ftl` stay deferred (architecture §5
  allows them; M4 does not load them).
- **API (frozen).**

```rust
pub fn ui_resources(lang: Lang) -> &'static [&'static str];         // ui, commands, config of that language
impl Localizer { pub fn for_ui(lang: Lang, arch: Option<&dyn Architecture>) -> Result<Self, LocalizerError>; }
pub enum LangSource { Flag, VaLang, Environment, Default, Setting, System }      // Setting and System are new
pub fn resolve_ui_lang(flag: Option<Lang>, setting: Option<Lang> /* None: auto or absent */,
                       var: &dyn Fn(&str) -> Option<String>, system: &dyn Fn() -> Option<String>) -> LangChoice;
pub fn system_ui_language() -> Option<String>;   // Windows: first sys_locale::get_locales() entry; elsewhere None
```

- **Precedence (GUI):** `--lang` > `VA_LANG` (`en`/`fr`/`auto`; invalid reported as in m1) > `ui.language` > `auto`. `auto` =
  `LC_ALL`, `LC_MESSAGES`, `LANG` (first non-empty, m1 §6.2), then on Windows the user's UI language (`fr…` → Fr, else En).
  `resolve_lang` (used by `va`) is unchanged: the CLI keeps M1 semantics until a later spec decides otherwise.
- `for_ui` loads `ui_resources(lang)` plus `arch.locale_resources(lang)` as primary and the English sets as fallback (m1 §6.2
  per-pattern fallback, isolation marks off).
- **Id namespaces (architecture §5).** The root resources own `ui-*`, `cmd-*` and `diag-cfg-*` and define no Fluent term; a CPU owns
  `diag-<its codes>` (68000: `diag-lex-*`, `diag-asm-*`) and `isa-<arch>-*`. Both sets share one bundle per language, where a
  duplicate id is `LocalizerError::Duplicate`: a build defect, caught by `apps/visual-assembler/tests/arch_registry.rs` (§10.3).
- **M2's texts.** Once M2 lands, `arch.locale_resources` also returns the assembler diagnostics and the directive docs (m2 §7.1);
  `for_ui` loads them with the rest and M4 renders none of them. Assembler diagnostics reach the GUI in M5 (background assembly,
  Problems panel, underline). `apps/va` keeps using `Localizer::for_architecture` and `resolve_lang`, whose signatures M4 keeps
  (§14.2).
- **Completeness tests.** `va-workspace/tests/i18n_ui.rs`: every resource parses; `en`/`fr` id and attribute sets are equal; every
  `CONFIG_CODES` entry has `diag-<code>`. `va-ui/tests/locales.rs`: extracts every string literal matching `"(ui|cmd)-[a-z0-9-]+"`
  from `crates/va-ui/src/**` and `apps/visual-assembler/src/**` (a plain text scan), adds the `cmd-*` id of every command of the
  registry, and asserts each exists in both languages and that no `ui-*`/`cmd-*` id is unused.
- French text is written in our own words, with the m1 §12.2 typography (regular spaces before `:` `;` `»`). Labels stay short and
  imperative, e.g. `ui-find-replace-all` = "Replace all" / "Tout remplacer".

### 4.7 I/O worker, atomic writes, session, lock (`persist/{worker,atomic,session,lock}.rs`, WP-C; frozen)

```rust
pub type Waker = Arc<dyn Fn() + Send + Sync>;                      // architecture T2
pub struct IoWorker { /* private: Arc of thread "va-io", mpsc of jobs, waker slot */ }   // Clone = shared handle
impl IoWorker {
    pub fn spawn() -> std::io::Result<Self>;  pub fn set_waker(&self, waker: Waker);  pub fn waker(&self) -> Waker; // calls the slot
    pub fn submit(&self, job: Box<dyn FnOnce() + Send>);             // runs in order; the job sends its own result
    pub fn flush(&self, timeout: Duration) -> bool;                  // waits until queued jobs ran (shutdown only)
}
pub type WriteFn = Box<dyn FnOnce(&mut dyn std::io::Write) -> std::io::Result<()> + Send>;
pub fn write_atomic(target: &Path, write: WriteFn) -> std::io::Result<()>;
pub fn read_limited(path: &Path, limit: usize) -> std::io::Result<Vec<u8>>;   // Read::take(limit + 1); over → InvalidData
pub struct DiskStamp { pub len: u64, pub modified: Option<SystemTime> }
pub fn stamp(path: &Path) -> std::io::Result<DiskStamp>;
pub struct SessionLock { /* private: File */ }
pub fn try_lock_session(paths: &Paths) -> std::io::Result<Option<SessionLock>>;   // None: another instance holds it
pub struct Session { /* frozen 2026-10-24 (after WP0): session types below */ }
pub fn parse_session(text: Option<&str>) -> Session;              // never fails: bad parts are dropped
pub fn session_to_toml(session: &Session) -> String;
```

- **Atomic write.** `target` is resolved through `fs::canonicalize` when it exists (a symlink is never replaced by a regular file).
  The temp file is `<name>.<pid>.<n>.tmp` in the target's folder, created with `create_new(true)`; the writer runs into a
  `BufWriter`; then `flush`, `sync_all`, the original's permissions copied (Unix), and `fs::rename` over the target. On error the
  temp file is removed. *Rejected:* a fixed `<name>.tmp` (the architecture §6 wording before its 2026-10-24 amendment), which could
  clobber a user file of that name and collides between two instances. M2's `va asm` follows the same procedure (m2 §8.1).
- **Lock.** `session.lock` is held with `File::try_lock` (std, stable since 1.89) for the life of the process. Without it (second
  instance, unwritable data dir), the session is **read-only**: no `session.toml`, backup or default-settings write; the status bar
  says so (§5.11).
- **`session.toml`** (written by the app, atomically, 2 s after a change and at exit):

```toml
schema_version = 1
window = { width = 1280.0, height = 800.0, maximized = false }   # logical px, 400–10000 × 300–10000
ui = { zoom = 1.0, theme = "dark", explorer_visible = true, explorer_width = 240.0 }   # theme: optional override
workspace = { root = "C:/Users/me/tp1" }                            # optional project root
[[tab]]                    # in tab order; path, or backup = "<id>" for an untitled buffer
path = "C:/Users/me/tp1/main.asm"
line = 12                  # caret, 1-based
column = 5
top_line = 1
active = true
```

  Paths are stored as UTF-8; a path that is not valid Unicode is not stored (its tab is not restored; documented). `window` and `ui`
  are read synchronously before the window opens (§6.2).
- **Session types** (`persist/mod.rs` from the WP0 follow-up, owned by WP-C; frozen 2026-10-24 (after WP0)):

```rust
// Session, SessionWindow, SessionUi, SessionWorkspace: derive Clone, Debug, PartialEq; `Default` hand-written (values in comments).
pub struct Session { pub window: SessionWindow, pub ui: SessionUi, pub workspace: SessionWorkspace,
                     pub tabs: Vec<SessionTab> }                                   // the [[tab]] tables, in tab order; empty
pub struct SessionWindow { pub width: f32, pub height: f32, pub maximized: bool }  // 1280.0, 800.0 (M0's size), false
pub struct SessionUi { pub zoom: f32, pub theme: Option<ThemeKind>, pub explorer_visible: bool,
                       pub explorer_width: f32 }                                  // 1.0, None, true, 240.0
pub struct SessionWorkspace { pub root: Option<PathBuf> }                          // None
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionTab { pub source: TabSource, pub line: u32, pub column: u32, pub top_line: u32, pub active: bool }
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TabSource { File(PathBuf), Backup(BackupId) }       // `path = "…"` | `backup = "<id>"` (untitled buffer)
```

- Fields are named as the keys (`[[tab]]` → `tabs`); `ThemeKind` is the §4.3 enum (`crate::config::ThemeKind`); `theme: None` = no
  override. `line`, `column` (in chars) and `top_line` are 1-based like `LineCol`; the shell clamps them to the document.
- `parse_session` reports nothing. A missing file, a syntax error or a `schema_version` other than 1 gives `Session::default()`.
  Otherwise a value of the wrong type or out of range takes its default (`width` 400–10000, `height` 300–10000, `zoom` 0.5–3.0 =
  §5.5 `ZOOM_MIN`–`ZOOM_MAX`, `explorer_width` 120–2000, `line`/`column`/`top_line` ≥ 1); a `theme` other than `light`/`dark` and
  a relative `root` are `None`; a `[[tab]]` is dropped unless it has exactly one of an absolute `path` or a well-formed `backup` id
  (§4.8); only the first `active = true` tab stays active; unknown keys are ignored.
- `session_to_toml` writes `schema_version = 1`, `window`, `ui` and `workspace` as inline tables, then one `[[tab]]` per tab. It
  omits `theme` and `root` when `None`, and a path that is not valid Unicode (with its tab); paths are written verbatim (the `/` of
  the example are illustrative). `persist_session.rs` asserts `parse_session(Some(&session_to_toml(&s))) == s` for in-range
  sessions with absolute UTF-8 paths.
- WP-G reads `window` (`with_window_geometry`, §6.2) and hands the whole `Session` to `ShellConfig::session`; WP-F applies `ui`,
  `workspace` and `tabs`, keeps them current (zoom, `view.toggle-theme` sets `theme`, `view.theme-from-settings` clears it,
  explorer, tabs and carets) and writes the file.

### 4.8 Hot exit, backups, autosave, restore (`persist/backup.rs`, WP-C; frozen)

```rust
pub struct BackupId(pub String);                 // "b" + 16 lowercase hex digits (time and counter)
pub struct BackupEntry { pub id: BackupId, pub file: Option<PathBuf>, pub title: Option<String>,
                         pub encoding: String, pub disk: Option<DiskStamp>, pub updated_utc: String }
pub struct BackupStore { /* private: dir, entries */ }
pub enum RestoreItem { Entry { entry: BackupEntry, text: String }, Orphan { id: BackupId, text: String } }
impl BackupStore {
    pub fn open(paths: &Paths) -> std::io::Result<(Self, Vec<RestoreItem>, Vec<BackupProblem>)>;
    pub fn write(&mut self, entry: BackupEntry, text: WriteFn) -> std::io::Result<()>;   // <id>.txt, then the index
    pub fn remove(&mut self, id: &BackupId) -> std::io::Result<()>;                       // index first, then the file
}
pub enum BackupProblem { MissingText(BackupId), NewerSchema, Unreadable(String) }
```

`backups/index.toml` = `schema_version = 1` plus one `[[backup]]` table per entry with the `BackupEntry` fields (`disk` as
`disk_len` and `disk_modified_ms`, absent for untitled buffers).

- **What is backed up.** Every dirty buffer (files and untitled), `files.backup_delay_ms` after its last edit and once more at exit.
  The text is always written as UTF-8; the entry records the original encoding and the file's `DiskStamp` at load or last save.
- **Order.** `write`: `<id>.txt` atomically, then `index.toml` atomically. `remove`: index first, then the file. So a kill at any
  point leaves either a complete earlier state or an orphan `<id>.txt`, never an index entry pointing at a torn file. An index of a
  newer `schema_version` is never rewritten: backups are disabled for that session with a notice.
- **Student files are never touched without consent.** Backups live only in the data dir. A student's file is written only by Save,
  Save As, Save All, or autosave after the user set `files.autosave` (that setting is the consent). Default `off`.
- **Hot exit (VS Code style, plan §4.3; default on by user decision, 2026-10-24).** With `files.hot_exit = true`, closing the window
  never prompts: the shell submits the final backups and the session, and `IoWorker::flush(2 s)` runs at exit (the one place the UI thread waits, at shutdown). With
  `false`, closing with dirty buffers cancels the close and asks Save all / Don't save / Cancel; "Don't save" deletes their backups.
  Closing one dirty tab always asks Save / Don't save / Cancel.
- **Restore** (after the first frame, on `va-io`): each entry reopens as a dirty tab with the backup text, in the session's tab
  order (`Session::tabs`: a `TabSource::File` tab takes the entry whose `file` is its path, a `TabSource::Backup` tab the entry of
  its id; clean `File` tabs reload from disk; entries and orphans that no tab names come last). If the file's current `DiskStamp`
  differs from the recorded one, the tab shows a banner: "The file changed on disk after your last unsaved edit" with **Keep my
  version** / **Load the disk version** (the latter discards the backup). A missing file keeps the buffer, titled with its old path. An orphan becomes an untitled "Recovered" tab. A backup is deleted only after its
  buffer is saved, reverted to clean, or explicitly discarded.
- **Autosave.** `after-delay`: dirty file buffers are saved `autosave_delay_ms` after the last edit; `on-focus-change`: when the
  window loses focus or the tab changes. Untitled buffers, Windows-1252 buffers with unencodable chars, and files changed on disk
  are never autosaved (they keep their backup and show the usual banner).
- **Kill test** (`tests/persist_kill.rs`, CI): the test re-runs its own binary (`std::env::current_exe()`, `--exact kill_helper
  --ignored`, env `VA_KILL_HELPER_DIR`) as a helper that loops writing versions `v0, v1, …` (64 KiB to 1 MiB, each line holding its
  version number) through `BackupStore::write`. The parent kills it 20 times at delays drawn from a fixed-seed xorshift in 5–200 ms,
  re-opens the store after each kill and asserts: `open` succeeds, every restored text is one complete version, versions never go
  backwards across kills, and no `*.tmp` file is ever read. Temp folders follow the `CARGO_TARGET_TMPDIR` rule.

## 5. `va-ui` (WP-E, WP-F)

Depends on `egui` (never eframe), `va-core`, `va-editor`, `va-workspace`. Dev-dependency `egui_kittest`. CPU specifics arrive only
as `&dyn Architecture` (rule R2); tests use a fake architecture and syntax in `tests/support/`.

### 5.1 Shell API (`lib.rs`, `shell/mod.rs`, WP-F; frozen)

```rust
pub struct ArchSupport { pub arch: &'static dyn Architecture, pub line_comment: &'static str }
pub struct RendererNote { pub running: &'static str, pub fallback_from: Option<&'static str>,
                          pub fallback_reason: Option<String>, pub test_hook: Option<String> }
pub enum ReadOnlyReason { OtherInstance, NotWritable, NoLocation }
pub enum Persistence { ReadWrite, ReadOnly { reason: ReadOnlyReason }, Off }   // Off: bench mode
pub struct ShellConfig {
    pub localizer: Arc<Localizer>, pub lang: Lang, pub settings: Settings, pub bindings: Vec<Binding>,
    pub problems: Vec<ConfigProblem>, pub session: Session, pub paths: Option<Paths>,
    pub persistence: Persistence, pub arch: Option<ArchSupport>, pub renderer: RendererNote,
    pub io: IoWorker, pub open_at_start: Option<PathBuf>,
}
pub enum DialogRequest { OpenFile { start: Option<PathBuf> }, OpenFolder { start: Option<PathBuf> },
                         SaveAs { start: Option<PathBuf>, name: String } }
pub struct DialogReply { /* private: Sender + Waker */ }
impl DialogReply { pub fn send(self, path: Option<PathBuf>); }
pub trait ShellHost { fn native_dialog(&mut self, request: DialogRequest, reply: DialogReply) -> bool; } // false: unavailable
pub struct Shell { /* private */ }
impl Shell {
    pub fn new(config: ShellConfig, host: Box<dyn ShellHost>) -> Self;  /* no I/O, < 1 ms */  pub fn ui(&mut self, ui: &mut egui::Ui);
    pub fn on_close_requested(&mut self, ctx: &egui::Context) -> bool;  // false: close cancelled, prompt shown
    pub fn shutdown(&mut self);  /* final backups + session, flush(2 s) */  pub fn execute(&mut self, command: &str) -> bool; // §5.12 id
    pub fn go_to_line(&mut self, line: usize);  pub fn open_path(&mut self, path: &Path);  pub fn bench_probe(&self) -> BenchProbe;
}
pub struct BenchProbe { pub pending_io: usize, pub open_tabs: usize, pub active_lines: Option<usize>,
                        pub active_painted: bool, pub total_lines: usize }
```

Layout: menu-less top bar (tabs), left explorer panel (resizable, `Ctrl+B`), central editor, bottom status bar; an empty bottom/side
slot is reserved for M5/M6 panels (not drawn in M4). The window title is `<file name>[ ●] — VisualAssembler`, set with
`ViewportCommand::Title` only when it changes. With no tab, the central panel shows a welcome page (`ui-welcome-*`: open file, open
folder, recent folder from the session, the palette shortcut).

### 5.2 Editor view (`editor/{mod,view,layout,gutter}.rs`, WP-E; frozen)

```rust
pub struct EditorStyle { pub font: egui::FontId, pub tab_width: u8, pub line_numbers: bool,
                         pub highlight_current_line: bool, pub cursor_blink: bool, pub style_generation: u64 }
pub enum DecorationKind { SearchMatch, CurrentSearchMatch }        // M5 adds Diagnostic(Severity)
pub struct Decoration { pub start: usize, pub end: usize, pub kind: DecorationKind }
pub struct EditorContext<'a> { pub syntax: &'a dyn SyntaxProvider, pub theme: &'a Theme, pub style: &'a EditorStyle,
                               pub options: EditOptions, pub decorations: &'a [Decoration], pub read_only: bool,
                               pub line_comment: Option<&'static str>, pub label: &'a str }
pub struct EditorResponse { pub response: egui::Response, pub changed: bool, pub caret_moved: bool }
pub struct RenderStats { pub lines_drawn: usize, pub lexed_lines: usize, pub cache_hits: usize, pub cache_misses: usize }
pub struct EditorView { /* private: id, cache, blink state, IME state, stats of the last frame */ }
impl EditorView { pub fn new(id: egui::Id) -> Self; pub fn stats(&self) -> RenderStats; pub fn top_line(&self) -> usize;
    pub fn show(&mut self, ui: &mut egui::Ui, doc: &mut Document, ctx: &EditorContext<'_>) -> EditorResponse;
    pub fn scroll_to_line(&mut self, line: usize);  pub fn request_focus(&self, ui: &egui::Ui); }
pub struct PlainSyntax;                 // impl SyntaxProvider: whitespace runs + one Symbol per other run
```

- **No egui `TextEdit` for the buffer.** The view is a custom widget inside `egui::ScrollArea::both().show_viewport`. Content height
  = `line_count × row_height`; content width = `(widest_line_hint + 2) × char_advance + gutter`. Only lines intersecting the
  viewport, plus one above and below, are fetched (`line_text_into` into one reused `String`), laid out and painted. **Every
  per-frame cost is O(visible lines)**; nothing walks the whole document.
- Rows have one fixed height (the code font's row height); no wrap, no folding in M4.
- **Long lines.** At most `MAX_DRAWN_CHARS = 4_096` chars of a line are laid out; longer lines show a truncation mark; the caret and
  selections still move through the whole line, and positions past the cap are drawn at the cap.
- **Tabs** are expanded to spaces for display (tab stops from `tab_width`); the line cache keeps the display↔byte map for lines that
  contain a tab. Positions use the galley (`Galley::pos_from_cursor`, `cursor_from_pos`), never a fixed grid, so fallback-font
  glyphs (CJK, emoji in comments) stay hit-testable.
- **Gutter**: right-aligned 1-based numbers, width from the digit count of `line_count` (min 3), painted at the viewport's left edge
  (it does not scroll horizontally); the caret's line uses `gutter_active`. A marker column of one char width is reserved for M5
  (empty in M4).
- **Painting order per visible line**: current-line background, selection rectangles, decorations (search matches as a filled
  background), text galley, carets (2 px wide, `caret` colour), IME pre-edit underline.
- Accessibility: the widget reports `WidgetInfo` of type `TextEdit` with `label` = the file name and the current line's text as
  value (full AccessKit text ranges: M7). Tests find it by label.
- `PlainSyntax` is used when the build has no architecture (single-renderer CI builds, §6.7).

### 5.3 Colouring and the per-line cache (`editor/cache.rs`, WP-E)

- Each visible line is lexed with `syntax.lex_line(line, LineState::INITIAL, &mut tokens)` (the returned state is ignored: the only
  M4 provider is stateless; a stateful dialect needs a per-line state index, deferred §13), turned into an `egui::text::LayoutJob`
  with one section per token (colour from §5.4), and laid out with `ui.fonts(|f| f.layout_job(job))`.
- **Cache**: `HashMap<Box<str>, CachedLine>` keyed by the line's exact text (no hash collision can mix lines), holding the
  `Arc<Galley>` and the tab map; capacity `LINE_CACHE_CAPACITY = 4_096` entries, least-recently-used eviction by frame stamp. The
  whole cache is cleared when `style_generation` changes (theme, font, size, zoom, `pixels_per_point`, tab width), and the app bumps
  it on each of those events.
- **Keystroke path**: an edit changes only the touched line's text, so the next frame has exactly one miss (one `lex_line`, one
  layout) and hits for every other visible line. `RenderStats` exposes this for tests (E5).

### 5.4 Themes (`theme.rs`, WP-E; frozen)

```rust
pub enum ThemeId { Light, Dark }
pub struct Theme { pub id: ThemeId, pub background: Color32, pub current_line: Color32, pub selection: Color32,
                   pub caret: Color32, pub text: Color32, pub gutter: Color32, pub gutter_active: Color32,
                   pub search_match: Color32, pub comment: Color32, pub mnemonic: Color32, pub directive: Color32,
                   pub macro_name: Color32, pub label: Color32, pub register: Color32, pub number: Color32,
                   pub string: Color32, pub macro_param: Color32, pub invalid: Color32 }
impl Theme { pub fn light() -> Self; pub fn dark() -> Self; pub fn token_color(&self, kind: TokenKind) -> Color32;
             pub fn visuals(&self) -> egui::Visuals; }
pub fn contrast_ratio(a: Color32, b: Color32) -> f32;     // WCAG 2.x relative-luminance ratio
```

| Role | Token kinds | Light | Dark |
|---|---|---|---|
| background / current line / selection | — | `#FFFFFF` / `#F2F4F7` / `#C8DDF5` | `#1E1F22` / `#2A2C30` / `#264F78` |
| text | Whitespace, Symbol, Operator, Punctuation | `#1F1F1F` | `#D4D4D4` |
| comment | Comment, SuspectComment | `#5F6B76` | `#8B96A1` |
| mnemonic | Mnemonic, SizeSuffix | `#0B57D0` | `#6CA8E8` |
| directive | Directive | `#8E24AA` | `#C792EA` |
| macro_name | MacroName | `#795500` | `#E0D08A` |
| label | Label, LocalLabel | `#00796B` | `#5CC8B4` |
| register | Register | `#005A9E` | `#9DD2F5` |
| number | Number | `#2E7D32` | `#A9D18E` |
| string | String | `#A1460B` | `#E09A78` |
| macro_param | MacroParam | `#9C4F00` | `#E6B060` |
| invalid | Invalid | `#C62828` | `#FF6B6B` |
| gutter / gutter_active / caret / search_match | — | `#6E7681` / `#1F1F1F` / `#1F1F1F` / `#FFE58F` | `#8C8C8C` / `#C8C8C8` / `#E8E8E8` / `#5A4A12` |

- `SuspectComment` is drawn exactly like a comment in M4; its error underline is M5 (m1 §5.7).
- **Test (`tests/theme.rs`)**: every token colour has a contrast ratio ≥ 4.5 against `background` and against `current_line`,
  `gutter` ≥ 4.5 against `background`, `text` ≥ 4.5 against `selection`; the match over `TokenKind` is exhaustive (17 kinds, no
  wildcard arm).
- `ui.theme = "auto"` (the default, user decision 2026-10-24) follows `ctx.system_theme()` (dark when unknown); the effective theme is the session override when set, else
  the setting. egui visuals come from `Visuals::light()/dark()` with the theme's background on panels.

### 5.5 Fonts and zoom (`fonts.rs`, WP-E; frozen)

```rust
pub const CODE_FONT_FAMILY: &str = "va-code";
pub static EMBEDDED_CODE_FONT: &[u8];      // include_bytes!("../../../assets/fonts/JetBrainsMono-Regular.ttf")
pub static EMBEDDED_FONT_LICENSE: &str;    // include_str!("../../../assets/fonts/OFL.txt")
pub fn font_definitions(custom: Option<Arc<Vec<u8>>>) -> egui::FontDefinitions;  // egui defaults + "va-code"
pub fn validate_font(bytes: Vec<u8>) -> Result<Arc<Vec<u8>>, FontError>;  // size, then a trial layout in a scratch Context under catch_unwind
pub enum FontError { TooLarge { bytes: usize }, Invalid }  pub const MAX_FONT_BYTES: usize = 32 * 1024 * 1024;
pub const ZOOM_MIN: f32 = 0.5;  pub const ZOOM_MAX: f32 = 3.0;  pub const ZOOM_STEP: f32 = 0.1;
```

- `va-code` = [custom font if valid, else JetBrains Mono Regular (`FontData::from_static`), then egui's monospace and emoji fonts as
  fallbacks]. The UI keeps egui's default proportional set (architecture §9.6: decided now). Only Regular is embedded (§13).
- A custom `editor.font_file` is read (`read_limited`, `MAX_FONT_BYTES`) and validated on `va-io` after the first frame; any failure
  keeps the embedded font and shows a notice. *Rejected:* system font enumeration (`fontdb`/`font-kit`): a dependency for a path
  setting covers the need.
- **Zoom** is egui's `set_zoom_factor` (the whole UI), persisted in `session.toml`; egui's own `zoom_with_keyboard` is turned off so
  the keys go through the keymap (`view.zoom-in`, …) and `Ctrl` + wheel over the editor. The code size is `editor.font_size`.
- The "Third-party licences" command (`app.licences`) opens `EMBEDDED_FONT_LICENSE` and the egui font notices in a read-only tab, so
  every binary carries the OFL text.

### 5.6 Input, caret and blink (`editor/{input,caret}.rs`, WP-E)

- The editor takes keyboard focus on click and locks Tab, arrows and Escape (`Memory::set_focus_lock_filter`), so egui's focus
  navigation never steals them.
- Keys: arrows (Shift extends, Ctrl = word), Home/End (Ctrl = document), PageUp/PageDown (visible rows − 1), Backspace/Delete (Ctrl
  = word), Enter, Tab/Shift+Tab, Escape (collapse to the primary caret). `Event::Text` inserts; `Event::Copy/Cut` put
  `selected_texts` joined by the document line ending on the clipboard (`ctx.copy_text`); `Event::Paste` runs `Paste`.
  `Event::Ime(Commit)` inserts; pre-edit text is drawn underlined at the caret and `PlatformOutput::ime` reports the caret rect.
- Mouse: click places the primary caret; drag selects; double-click selects a word, triple-click a line; Shift+click extends;
  Alt+click adds a caret; dragging past the viewport edge autoscrolls (one `request_repaint_after(16 ms)` per frame only while the
  button is held).
- After any edit or caret move, the primary caret is scrolled into view (`scroll_to_rect`).
- **Blink (reactive-mode rule; on by default, user decision 2026-10-24).** With `cursor_blink` on, the editor focused and the window focused, the caret alternates every
  `BLINK_HALF_PERIOD = 530 ms` and the view requests exactly one `request_repaint_after(time to the next phase change)`. After
  `BLINK_TIMEOUT = 10 s` without input the caret stays solid and no wake-up is requested. With blink off, unfocused, or the window
  unfocused, the editor requests nothing. Time is `ctx.input(|i| i.time)`, which kittest controls.
- **Idle rule for the whole shell.** No widget calls `request_repaint` unconditionally. Delayed work (backup and autosave timers,
  find debounce, session write) requests one wake-up at its deadline only while that work is pending; workers wake the UI through
  the `Waker` only when a result is ready. Test `shell_status::idle_requests_no_repaint` (E5).

### 5.7 Tabs and documents (`shell/{state,tabs,prompts}.rs`, WP-F)

- A tab holds a `Document`, its `EditorView`, the file path (or untitled name `Untitled-<n>`), the `DiskStamp` at load/save, its
  `BackupId` once backed up, and a `read_only` flag.
- **Open** (explorer, quick open, dialog, command line, restore): the job runs on `va-io`: `read_limited(16 MiB)`, `decode`,
  `Text::new` and `widest_line_chars` all happen off the UI thread; the tab appears at once with a "Loading" placeholder and is
  filled when the result arrives. Opening an already open path focuses its tab. Errors: too large, binary, unreadable → a notice, no
  tab; `NotUtf8` → a read-only tab with the banner "This file is not UTF-8 (byte <n>)" and **Reopen as Windows-1252** / **Close**.
- **Save**: if the file's `DiskStamp` changed since load/save, ask Overwrite / Cancel first; for Windows-1252 with an unencodable
  char, ask to save as UTF-8 instead (consent) or cancel. The snapshot is written by `write_atomic` on `va-io` with `encode_to`; on
  success the document is marked saved at the snapshot's revision (edits made meanwhile keep it dirty) and its backup is removed.
  Read-only file or permission error → notice and offer Save As.
- Tab bar: one button per tab with the name, `●` when dirty, a close `×`; middle-click closes; overflow scrolls horizontally;
  `Ctrl+Tab`/`Ctrl+Shift+Tab` cycle in visual order. Dirty-tab close and window close follow §4.8.
- External changes: on window focus gain, the stamps of open files are refreshed on `va-io`. A clean tab whose file changed reloads
  silently (caret line kept); a dirty one shows the banner **Reload** / **Keep my version**; a deleted file marks the tab "deleted
  on disk".

### 5.8 Explorer (`shell/explorer.rs`, WP-F)

- Tree of the project root, folders first, then files, each sorted case-insensitively; entries named in `explorer.exclude` are
  hidden. A folder's entries are listed on `va-io` when it is first expanded (at most `MAX_DIR_ENTRIES = 10_000`, then a "… <n>
  more" row); symlinked folders are shown but never followed automatically (expanding one lists it once, no recursion).
- Actions: click opens a file, folder rows expand; context menu: New file, New folder, Rename (F2), Refresh. Rename refuses an
  existing target. No delete in M4 (§13).
- Without a project, the panel shows **Open folder**. Refresh also runs on focus gain for expanded folders.

### 5.9 Command palette and quick open (`shell/{palette,fuzzy}.rs`, WP-F)

- One modal popup at the top centre with a text field; the prefix chooses the mode: `>` commands (`Ctrl+Shift+P`, `F1`), no prefix
  files of the project (`Ctrl+P`), `:` go to line (`Ctrl+G`).
- Commands: localized title from `cmd-*`, its key chord shown at the right, disabled commands hidden. Files: names from a project
  walk on `va-io` (BFS, depth ≤ 16, at most 20,000 files, excluded names skipped, symlinked folders not followed), displayed as
  relative paths.
- **Fuzzy match** (hand-written): case-insensitive subsequence; score = +3 per char matched right after the previous one, +2 for a
  match at a word start (after `space . _ - /` or a case change), −1 per skipped char; ties: recently used first (in-memory list of
  20), then alphabetical. At most 50 rows drawn. Up/Down move, Enter runs, Escape closes.
- In internal-dialog mode (§6.6) the same popup takes a path (absolute or project-relative) for Open, Open folder and Save As, with
  completion of the last segment from a `va-io` directory listing.

### 5.10 Find bar (`shell/find.rs`, WP-F)

- `Ctrl+F` opens a bar above the editor with the primary selection as the initial pattern; `Ctrl+H` adds the replace row. Toggles:
  case (`Alt+C`), whole word (`Alt+W`), regex (`Alt+R`). Enter / `F3` next, Shift+Enter / `Shift+F3` previous, Escape closes and
  returns focus.
- One persistent `va-search` thread compiles the query (`Matcher::new`) and runs `find_all` 150 ms after the last change of the
  pattern or of the document (debounce), with a per-request cancel flag. The reply carries the document revision, the `Matcher`
  (kept by the bar for Replace) and the matches, or the `QueryError`, which shows `ui-find-invalid` with the raw detail; a stale
  reply is dropped. The bar shows "<i> of <n>" (or "<n>+" when truncated); matches become `SearchMatch` decorations.
- F3 / Shift+F3 with a fresh match list move at once. With a stale list (an edit within the last 150 ms), the bar sends an
  undebounced `find_from` request to `va-search` and moves the selection when the reply arrives, if the revision is unchanged.
- **Replace** replaces the current match (one undo group) and moves to the next. **Replace all** runs `replace_all` on `va-search`
  and applies the result with `Document::replace_text` (one undo group), only if the revision did not change meanwhile.

### 5.11 Status bar and notices (`shell/status.rs`, WP-F)

- Left to right: `Ln <l>, Col <c>` (1-based, columns in chars, tab = 1 as `LineCol`), the selection count when > 1, encoding, line
  ending, language, zoom % (when ≠ 100 %), then notices.
- Notices (each a short label; click shows the detail in a popup):
  - **renderer fallback** (m0 §19.9 S19-1): "<from> → <running>" with the reason;
  - **test hook armed**: `VA_TEST_FAIL_RENDERER=<value>` when `RendererNote::test_hook` is set;
  - **read-only session** (§4.7) with its reason; **config problems**: "<n> settings problems", the popup lists them and opens the
    file at the problem's line;
  - transient notices (save errors, load errors, font errors) shown for 8 s (one delayed wake-up).

### 5.12 Commands and default keymap (`shell/{commands,keymap}.rs`, WP-F)

Ids are `<group>.<action>` (kebab-case); the Fluent title is `cmd-<group>-<action>`. Bindings are matched with
`InputState::consume_shortcut`, longer modifier sets first, before the editor sees the events; `when = editor` bindings apply only
while an editor has focus, `explorer` likewise.

| Command | Default key | Command | Default key |
|---|---|---|---|
| `file.new` | Ctrl+N | `edit.add-cursor-below` | Ctrl+Alt+Down |
| `file.open` | Ctrl+O | `edit.toggle-comment` | Ctrl+Slash |
| `file.open-folder` | Ctrl+Shift+O | `edit.delete-lines` | Ctrl+Shift+K |
| `file.save` | Ctrl+S | `edit.duplicate-lines` | Shift+Alt+Down |
| `file.save-as` | Ctrl+Shift+S | `edit.move-lines-up` / `-down` | Alt+Up / Alt+Down |
| `file.save-all` | — | `find.find` / `find.replace` | Ctrl+F / Ctrl+H |
| `file.close-tab` | Ctrl+W | `find.next` / `find.previous` | F3 / Shift+F3 |
| `file.revert` | — | `go.line` / `go.file` | Ctrl+G / Ctrl+P |
| `file.reopen-windows-1252` | — | `palette.commands` | Ctrl+Shift+P, F1 |
| `edit.undo` | Ctrl+Z | `view.zoom-in` | Ctrl+Equals, Ctrl+Plus |
| `edit.redo` | Ctrl+Y, Ctrl+Shift+Z | `view.zoom-out` / `view.zoom-reset` | Ctrl+Minus / Ctrl+0 |
| `edit.select-all` | Ctrl+A | `view.toggle-explorer` | Ctrl+B |
| `edit.select-next-occurrence` | Ctrl+D | `view.toggle-theme` / `view.theme-from-settings` | — |
| `edit.add-cursor-above` | Ctrl+Alt+Up | `tab.next` / `tab.previous` | Ctrl+Tab / Ctrl+Shift+Tab |
| `explorer.new-file` / `explorer.new-folder` | — | `explorer.rename` | F2 (explorer) |
| `explorer.refresh` | — | `project.settings` | — |
| `app.open-settings` | Ctrl+Comma | `app.open-keybindings` | — |
| `app.licences` | — | `app.quit` | Ctrl+Q |

`app.open-settings` opens `settings.toml` in a tab (creating the commented default first when missing and the session is writable).
`app.quit` behaves as the window close button.

### 5.13 Hooks for M5 (not implemented beyond these points)

`Document::snapshot()` and `revision()` (inputs of the M5 background assembler; `Text::write_to` gives contiguous text),
`Decoration`/`DecorationKind` (M5 adds `Diagnostic(Severity)` and the underline), the gutter marker column, the reserved panel slot,
`ConfigProblem::to_diagnostic`, and `Shell::execute` with the command registry.

## 6. `visual-assembler` integration (WP-G)

### 6.1 Command line (`cli.rs`)

```
visual-assembler [--renderer <auto|glow|wgpu>] [--lang <en|fr>] [PATH]
visual-assembler [--renderer …] --bench <startup|idle|open|scroll|type> [--bench-input <PATH>] [--duration-secs <N>]
```

- `PATH` (file or folder) is opened after the first frame. `--lang` follows §4.6 (`auto` is not a value, as for `va`).
- Usage errors (exit 2, added to `UsageError`): `--bench-input` without `--bench`, or with `--bench startup`; `--bench
  open|scroll|type` without `--bench-input`; `PATH` together with `--bench`. `--duration-secs` stays idle-only. Exit codes are
  unchanged (m0 §17.10, §19.4).

### 6.2 Startup sequence (normative)

1. `main` takes `started`; `run` parses the command line.
2. `resolve_paths` (warnings to stderr); `try_lock_session` is **not** taken yet.
3. Synchronously, before any window, each capped at `MAX_CONFIG_BYTES`: `settings.toml`, `keybindings.toml`, `session.toml` (window
   geometry, zoom, theme override). Problems go to stderr and to the shell. This is the only file I/O on the main thread, and it is
   inside the measured startup.
4. Resolve the renderer (§6.3), the language (§4.6), the window system (m0 §19.1); print warnings.
5. `Localizer::for_ui`, the architecture registry, `IoWorker::spawn` (the app creator calls `set_waker` with the first frame's
   `egui::Context`).
6. The fallback loop of m0 §19.2 is unchanged. `native_options` keeps its signature; two new pure helpers apply the session geometry
   (`with_window_geometry`) and the bench vsync rule (`with_bench_vsync`, §6.8). The app creator builds `ShellApp` (fonts
   registered, no I/O).
7. After the first frame (on `va-io`): take the session lock, open the backup store, restore the session's project and tabs, write
   the default `settings.toml` if missing (writable session, not bench), open `PATH`, load a custom font.

### 6.3 Renderer precedence with the settings level

`resolve_renderer(cli, env, setting, compiled, os, arch)` gains `setting: Option<RendererChoice>` between the environment and
`auto`; `ChoiceSource` gains `Settings` (`as_str` = `"settings"`). Precedence: `--renderer` > `VA_RENDERER` > `render.renderer` >
`auto` (m0 §19.1). WP-G maps `Settings::render.renderer` (§4.3): `Some(RendererKind::Glow)` → `Some(RendererChoice::Glow)`,
`Some(RendererKind::Wgpu)` → `Some(RendererChoice::Wgpu)`, `None` (absent, `"auto"` or invalid) → `None`. So `"auto"` in the file
acts as the built-in `auto` (source `default`) and `ChoiceSource::Settings` appears only for a forced setting. The invalid-value
rule of m0 §19.9 WP2-5 applies to the setting through its `cfg-invalid-value` problem (an invalid setting is ignored, the next
level applies). Only the winning level is checked against the build: a forced setting naming a renderer that is not compiled in is
a warning (`render.renderer names a renderer that is not compiled in; using auto`) and falls back to `auto`, never exit 2 (a
settings file must not make the app unlaunchable).
`--renderer` keeps exit 2 and `VA_RENDERER` keeps its M0 behaviour. A forced setting that fails to initialize exits 3 with `(forced
by settings.toml render.renderer; try --renderer auto)`.

### 6.4 Startup failure without a console (architecture §9.8, m0 §19.9 S19-1)

When the app is about to exit 3 (or 1 for a runtime failure before the window), it prints the §19.4 lines on stderr as today and,
**on Windows only**, when `std::io::stderr().as_raw_handle()` is null (GUI subsystem started without redirection, i.e. from Explorer
or the Start menu) and no `--bench` is given, also shows a native error box (`rfd::MessageDialog`, level Error) with
`ui-startup-failed-title`, `ui-startup-failed-body` (the error text and the hint "start it with --renderer glow or --renderer wgpu
from a terminal") in the resolved language. The decision is the pure function `should_show_startup_dialog(windows: bool,
stderr_missing: bool, bench: bool)`. Linux keeps stderr only (desktop launchers log it). *Rejected:* a log file, which nobody
without a console would know to open.

### 6.5 `ShellApp` (`shell_app.rs`)

- Owns the `Shell`, the borrowed bench recorder and the `ui_started` flag (m0 §19.2 unchanged). `logic` forwards to the recorder;
  `ui` sets `ui_started`, calls `Shell::ui`, then `recorder.on_ui`; `raw_input_hook` forwards to `recorder.on_raw_input` (§6.8).
- Close: when the root viewport reports `close_requested`, `Shell::on_close_requested` decides; `false` sends
  `ViewportCommand::CancelClose`. `on_exit` calls `Shell::shutdown`.
- `pub fn draw(&mut self, ui: &mut egui::Ui)` (the frame-independent part) is public so `tests/shell_app.rs` drives it with kittest
  without eframe.

### 6.6 Dialog host (`host.rs`)

`NativeHost` implements `ShellHost`; the shell calls it only while `ui.native_dialogs = true` (§4.3). On Windows it runs
`rfd::FileDialog` (`pick_file`, `pick_folder`, `save_file`, filters `*.asm;*.s;*.i;*.inc` and all files) on a short-lived thread
`va-dialog` and calls `DialogReply::send`; a second request while one is open is refused. Elsewhere it returns `false`, and the shell uses the internal
path prompt (§5.9). Dialogs are not parented to the window (not modal). *Rejected:* rfd on Linux: under WSLg there is no portal,
native Linux is not a manual tier (architecture §9.1), and its portal backend falls back to spawning `zenity` from `PATH`.

### 6.7 Architecture registry (`arch_registry.rs`)

`pub fn architectures() -> Vec<ArchSupport>`; feature `arch-m68k` (default) adds `ArchSupport { arch: &va_arch_m68k::M68K,
line_comment: ";" }`. The project's `arch` id picks the entry; without any registered architecture the shell uses `PlainSyntax` and
no comment toggle. The comment prefix lives here because `va-core` has no such query (§13: candidate `va-core` addition, not needed
for M4, not added by M2). The registry and the shell call only the M1 methods of `Architecture`; M2's provided methods
`assembler()`/`disassembler()` (default `None`, m2 §3.4) are first called by M5.

### 6.8 Bench recorder (`bench/{recorder,script,report}.rs`)

- Bench mode sets `Persistence::Off` (no session, backup, default-settings or autosave write; settings are still read), turns blink
  off, and for `open`/`scroll`/`type` sets `NativeOptions::vsync = false` so a sample measures the app's frame, not the display's
  refresh wait (architecture §1: latency to the physical display is a non-goal). `vsync` is reported.
- **Present proxy** (m0 §4.5 definition, extended): a frame counts as presented when the next frame starts; the recorder timestamps
  the start of every frame in `raw_input_hook`, which eframe runs before `logic`. While a scenario is active the recorder calls
  `ctx.request_repaint()` each frame so the next frame follows immediately.

| Scenario | Steps (after the first presented frame) | Reported |
|---|---|---|
| `startup` | as M0 (empty shell, no input) | as v2 |
| `idle` | with `--bench-input <dir>`: open it as project and open its `.asm` files (sorted) as tabs; wait until `pending_io == 0` and the active tab is painted; then the M0 idle algorithm (3 s settle, N s window, no wake-up) | as v2 + `input`, `input_lines` |
| `open` | t0 = `Shell::open_path(input)`; t1 = start of the frame after the first frame that painted the document | `open_ms = t1 − t0` |
| `scroll` | open as above; then 300 frames, each injecting one `PageDown` key press | `open_ms`; 300 samples = frame start → next frame start |
| `type` | open; `go_to_line(50_000)`; one quiet frame; then 500 frames, each injecting one event of the script below | `open_ms`; 500 samples, same definition |

- **Type script** (`script.rs`, deterministic): the 25-event cycle `\t m o v e . l \t d 0 , ( a 1 ) + Space ; Space o k Enter
  Backspace Backspace Enter` where single chars are `Event::Text`, `\t` a `Tab` key, `Space` a text " ", and `Enter`/`Backspace` key
  presses; 20 cycles = 500 events. Events go to the focused editor of the opened file.
- Each sample is `next_frame_start − injection_instant`, i.e. keystroke → presented frame for `type` and PageDown → presented frame
  for `scroll`. The GUI reports all samples and their summary (median, nearest-rank p95, max).
- Watchdog: `open`, `scroll`, `type` use `STARTUP_WATCHDOG_SECS` (60 s). Unknown input path or a load error: `error: bench input
  <path>: <reason>`, exit 1.

## 7. Bench runner (WP-H, `xtask/src/bench/`)

### 7.1 Command and inputs

```
cargo xtask bench [startup|idle|open|scroll|type|all] [--renderer auto|glow|wgpu|all] [--runs N] [--warmup N] [--duration-secs N] [--json]
```

- `all` = startup, idle, open, scroll, type, in that order. Default runs: startup 20, idle 3, open 5, scroll 3, type 3 (`--runs`
  overrides all). Each run passes `--renderer <r> --bench <scenario> [--bench-input <path>] [--duration-secs N]`.
- **Isolation:** every GUI run gets `VA_CONFIG_DIR` and `VA_DATA_DIR` pointing to
  `<target>/bench/profile-<utc>-<pid>/{config,data}`, created once per command with `fs::create_dir` (the user's settings never
  affect a report).
- **Generated inputs** (`generate.rs`, frozen): `bench/generated/100k.asm` (100,000 lines) for open/scroll/type, and
  `bench/generated/5k/` for idle: `.visualasm/project.toml`, `main.asm` (500 lines, ending with `INCLUDE` lines for the parts) and
  `part01.asm`…`part09.asm` (500 lines each), 5,000 lines in all. `bench/generated/VERSION` holds `GENERATOR_VERSION` (= `1`); files
  are regenerated when it is missing or different. Content: 20 fixed template lines of our own, cycled, LF only, with the line
  number in every label (`l<n>:`) so labels are unique; the set covers labels and local labels, every addressing mode, directives
  with `DC.B` strings, comments, one `SuspectComment` line (`dc.b 1, 2`) and one invalid line, for ~33 bytes per line.

```rust
pub const GENERATOR_VERSION: u32 = 1;
pub fn template_line(n: usize) -> String;          // line n (0-based) of every generated file
pub struct BenchInputs { pub large: PathBuf, pub project: PathBuf }
pub fn ensure_inputs(root: &Path) -> anyhow::Result<BenchInputs>;
```

### 7.2 Run line `va.bench.run/3` (GUI) and validation (xtask)

The 22 keys of `va.bench.run/2` in their order, then 7 new keys: `input` (string|null, last path component of `--bench-input`),
`input_lines` (integer|null, lines opened), `vsync` (bool), `open_ms` (number|null), `event_count` (integer|null), `event_ms` (array
of numbers|null, in injection order), `event_summary_ms` (`{"median","p95","max"}`|null). Example (values illustrative):

```json
{"schema":"va.bench.run/3","scenario":"type","renderer_requested":"auto","renderer":"glow","fallback_from":null,"fallback_reason":null,"injected_failure":null,"backend":"gl","adapter":"AMD Radeon(TM) 860M Graphics","driver":"…","window_system":"win32","wsl":false,"os":"windows","arch":"x86_64","version":"0.1.0","startup_ms":290.1,"frames_painted":503,"process_cpu_ms":2210.4,"peak_rss_bytes":151000000,"peak_private_bytes":98000000,"settle_ms":null,"idle_window_ms":null,"input":"100k.asm","input_lines":100000,"vsync":false,"open_ms":61.2,"event_count":500,"event_ms":[4.1,3.9],"event_summary_ms":{"median":4.0,"p95":6.3,"max":11.8}}
```

(`event_ms` shortened here; a real line has `event_count` entries.) Validation adds to m0 §19.5: `open`/`scroll`/`type` need
`open_ms` non-null and `vsync == false`; `scroll`/`type` need `event_count` equal to 300/500 and as many samples; `input` must equal
the requested input's name.

### 7.3 Report `va.bench.report/3`

- Each result adds `input` (string|null) after `duration_secs`. `summary` adds `open_ms` (median/p95/min/max over runs, or null) and
  `event_ms` (median, nearest-rank p95 and max over the **pooled** samples of all measured runs, plus `count`; null for
  startup/idle). Other keys and rules are those of v2.
- Markdown header (one row per result):

```
| scenario | requested | renderer | fallbacks | backend | window | spawn→frame p50/p95 ms | startup p50/p95 ms | idle CPU p50/p95 % | frames p50/max | open p50/p95 ms | event p50/p95/max ms | peak RSS p50 MiB | binary MiB |
```

- The xtask `BenchRun` mirror adds the 7 fields; `RUN_SCHEMA`/`REPORT_SCHEMA` become `/3`. *Rejected:* keeping `/2` with extra keys:
  the key set and order are normative and tested.

## 8. Dependencies, manifests and shared files (WP0)

### 8.1 Root `Cargo.toml` `[workspace.dependencies]` additions (normative, sorted with the existing keys)

```toml
egui = "=0.36.2"
regex = { version = "=1.13.1", default-features = false, features = ["std", "perf", "unicode-case", "unicode-perl"] }
rfd = { version = "=0.17.2", default-features = false }
ropey = { version = "=1.6.1", default-features = false, features = ["simd", "cr_lines"] }
sys-locale = "=0.3.2"
toml = { version = "=1.1.8", default-features = false, features = ["std", "parse", "display", "serde"] }
va-editor = { path = "crates/va-editor" }
va-ui = { path = "crates/va-ui" }
```

The header comment of `Cargo.toml` gains "and amended by m4-editor-shell.md §8". `Cargo.lock` is committed with the manifests (`make
security` is `--locked`).

### 8.2 Member manifests

| Member | `[dependencies]` | Other |
|---|---|---|
| `va-editor` | `ropey`, `regex`, `va-core` | — |
| `va-workspace` | + `serde`, `toml` | `[target.'cfg(windows)'.dependencies] sys-locale` |
| `va-ui` | `egui`, `va-core`, `va-editor`, `va-workspace` | `[dev-dependencies] egui_kittest` |
| `visual-assembler` | + `va-core`, `va-editor`, `va-ui`, `va-workspace`, `va-arch-m68k = { workspace = true, optional = true }` | `[features] default = ["renderer-glow", "renderer-wgpu", "arch-m68k"]`, `arch-m68k = ["dep:va-arch-m68k"]`; `[target.'cfg(windows)'.dependencies] rfd` |

All entries are `{ workspace = true }` (R6). The layering table of `xtask/src/layering.rs` needs no change: every edge is allowed
(R1–R3; `egui`/`egui_kittest` are the two GUI crates `va-ui` may use; `rfd` is not a GUI family and stays in the app). `ci.yml`,
`.cargo/deny.toml` and `.gitignore` are unchanged (`bench/generated/` is already ignored); WP0 confirms with `make security` that no
licence or ban exception is needed.

**Makefile** (normative): the `bench` recipe is unchanged and the help line becomes:

```make
	$(info   bench     measure the GUI - SCENARIO=startup|idle|open|scroll|type|all RENDERER=auto|glow|wgpu|all RUNS=n)
```

The header comment's last two lines become (WP0, wave 1):

```make
# (security target) and 19.5 (RENDERER default), and amended by m1-core-isa-lexer.md section 8
# (bench-lexer target) and m4-editor-shell.md section 8 (bench help line). Comments may be added;
# values must not change.
```

M2's WP0 then edits the same comment in wave 1b (m2 §9: `bench-asm`, `compare-a68k`), so the two diffs are applied in that order,
never in parallel (§14). Its final text is the one m2 §9 gives.

### 8.3 New dependencies — **approved by the user on 2026-10-24**

Resolved on 2026-10-24 against crates.io; every MSRV ≤ 1.85, so all build on rustc 1.99. **User decision (2026-10-24, in chat):**
ropey 1.6.1, regex 1.13.1, toml 1.1.8, sys-locale 0.3.2, rfd 0.17.2 (Windows only) and egui 0.36.2 as a direct dependency of
`va-ui` are approved exactly as this table and §8.1–§8.2 list them (versions, features, targets). Any other crate, version or feature
goes back to the Architect and the user.

| Crate | Version | Licence | Used by | New packages in `Cargo.lock` | Reason / why not std |
|---|---|---|---|---|---|
| `ropey` | 1.6.1 (latest stable; 2.0 is beta) | MIT | va-editor | ropey, str_indices 0.4.4 (MIT OR Apache-2.0); smallvec already present | O(log n) edits and line index on 100,000 lines, O(1) snapshots for workers (plan decision) |
| `regex` | 1.13.1 (latest) | MIT OR Apache-2.0 | va-editor | regex, regex-automata 0.4.18, regex-syntax 0.8.11 (MIT OR Apache-2.0), aho-corasick 1.1.5 (Unlicense OR MIT); memchr present | Regex find/replace with guaranteed linear time and size limits; unused Unicode tables left out |
| `toml` | 1.1.8 (latest; already in the graph through egui_kittest) | MIT OR Apache-2.0 | va-workspace | toml_writer 1.1.3 (MIT OR Apache-2.0); toml, toml_parser, winnow, toml_datetime, serde_spanned present | TOML is imposed (plan); spans for precise config problems |
| `sys-locale` | 0.3.2 (latest) | MIT OR Apache-2.0 | va-workspace, Windows only | sys-locale (no dependency, its own FFI declarations) | Windows UI language for `auto` without an `unsafe` allow-list entry in our code (m1 §6.2) |
| `rfd` | 0.17.2 (latest), no default features | MIT | visual-assembler, Windows only | rfd only (windows-sys 0.61.2, raw-window-handle, log present) | Native Windows open/save dialogs and the startup error box |
| `egui` (direct) | 0.36.2 (same as eframe) | MIT OR Apache-2.0 | va-ui | none | `va-ui` builds widgets on egui, never eframe (R3) |
| JetBrains Mono (asset) | 2.304, Regular TTF only (~0.3 MB) | OFL-1.1 | va-ui (`include_bytes!`) | — | Embedded code font (architecture §9.6) |

Not added (*rejected*, one line each): `directories` (§4.1, std suffices, MPL-2.0 child); `notify` (focus-gain stat checks cover M4,
CC0 + watcher threads); `encoding_rs` (one code page, §3.1); `fontdb`/`font-kit` (§5.5); `thiserror` (std `Error` impls suffice);
`tracing` (no log consumer in M4); `crossbeam`/`tokio` (architecture T5); `criterion` (M7 decides).

### 8.4 Font asset (WP0; download approved by the user on 2026-10-24)

- **User decision (2026-10-24, in chat):** download `JetBrainsMono-2.304.zip` (5,622,857 bytes) from
  `https://github.com/JetBrains/JetBrainsMono/releases/download/v2.304/JetBrainsMono-2.304.zip` and embed only the Regular TTF and its
  OFL licence file. The approval covers exactly that file from that URL; WP0 checks the size before extracting and stops on a
  mismatch.
- The archive is untrusted data: it goes into its own new, empty folder outside the repository, and only two members are copied
  out, unmodified: `fonts/ttf/JetBrainsMono-Regular.ttf` and `OFL.txt` into `assets/fonts/`. If either path is missing from the
  archive, WP0 stops and reports; no other member is taken. Record the SHA-256 of the archive and of both files in
  `assets/fonts/README.md` with the source URL and the date.
- The binary embeds the TTF and the licence text (§5.5); M8 packaging also copies `OFL.txt` next to the executable. The font is not
  a crate, so cargo-deny does not see it: `IMPORT.md` and `DEPENDANCE.md` carry it.

## 9. Security notes (for the Security agent)

| # | Topic | Decision |
|---|---|---|
| S4-1 | Untrusted config and project files (a downloaded project, a copied settings file) | Every app-read file is capped (`MAX_CONFIG_BYTES`, 16 MiB sources, 32 MiB font); TOML recursion stays bounded (`toml` without `unbounded`); project paths must be relative without `..` (§4.5); a bad file never panics or blocks start (tests) |
| S4-2 | Writes next to the student's files | Only on Save/Save As/opt-in autosave; `write_atomic` with `create_new` temp names, canonicalized targets (symlinks preserved), permissions copied; no delete feature in M4; temp files left by a kill are never deleted automatically in user folders |
| S4-3 | Backups hold student code | Data dir only, Unix mode `0o700`; read-only session when the lock is held elsewhere, so two instances never clobber each other; newer index schema never rewritten |
| S4-4 | Regex from the user | Linear-time engine, size/DFA/nest limits, matches capped, cancellable worker (§3.4) |
| S4-5 | Custom font file | Size cap; trial layout in a scratch `egui::Context` under `catch_unwind` on `va-io` before use (§5.5) |
| S4-6 | Text echoed to stderr | Config problems and path warnings use `escape_debug` (m0 §19.9 S19-2 rule) |
| S4-7 | External processes and network | None added. rfd is Windows-only (COM dialogs, MessageBoxW); no `zenity` spawn; no new `unsafe` (the stderr-handle check uses the safe `AsRawHandle`); `UNSAFE_ALLOWLIST`, `NET_ALLOWLIST`, `INCLUDE_ALLOWLIST` unchanged; `include_bytes!`/`include_str!` are data (m0 §17.4 S8) |
| S4-8 | Network paths | The GUI opens what the user opens, including UNC paths or mapped network drives, and the session restores them: SMB traffic is the OS acting on the user's path, not product network I/O. Recorded as a limitation until the M7 path sandbox (same class as M1-S6) |
| S4-9 | Environment influence | `VA_CONFIG_DIR`, `VA_DATA_DIR`, `VA_LANG`, `VA_RENDERER` steer the process; whoever sets the environment already controls it (m0 §19.3 analysis) |
| S4-10 | Bidi or control chars in file names shown in the UI | egui draws them as text (no terminal); Fluent isolation stays off (m1 §6.2); accepted |

## 10. Tests and acceptance

Every test file starts with `//!` and `#![allow(clippy::unwrap_used)]`; temp folders follow the `CARGO_TARGET_TMPDIR` rule. kittest
tests use `Harness::new_ui_state` (egui only), drive input with `key_press_modifiers`, `event(Event::Text(..))`, `input_mut().time`,
and read `harness.output().viewport_output[&ViewportId::ROOT].repaint_delay` for wake-ups. Tests that wait on `va-io`/`va-search`
loop `harness.step()` until a condition holds, with a 5 s timeout.

### 10.1 Library crates

| File | Covers |
|---|---|
| `va-editor/tests/text.rs` | Line count and line ranges equal `va_core::source::LineIndex` on LF, CRLF, CR, mixed, empty, trailing terminator, 10,000 random texts (fixed seed); `display_col`/`byte_at_display_col` with tabs and multi-byte chars; `widest_line_chars` |
| `…/encoding.rs` | BOM detection and strip; NUL → `Binary`; `NotUtf8` offset; all 256 bytes round-trip through Windows-1252; `first_unencodable`; `encode_to` per encoding; 16 MiB + 1 → `TooLarge` |
| `…/selection.rs`, `edit.rs` | Normalization and merge; `MAX_CARETS`; every command of §3.2 with one and three carets; paste distribution; auto-indent; tab/indent/outdent with both `insert_spaces`; toggle comment both ways and with blank lines; move/duplicate/delete lines at document edges; CRLF as one char for Backspace |
| `…/history.rs` | Grouping rules with injected time (typing "move.l" = one group; a pause of 1 s, a line break, a caret jump each split); undo restores selections; redo cleared by edits; dirty after undo to the save point = clean; limits drop oldest; `replace_text` is one group |
| `…/search.rs` | Literal, case, whole word with `.`/`_` words, regex, `^`/`$` per line, empty matches, `MAX_MATCHES` truncation, cancel, `find_from` wrap both ways, templates `$1`/`${name}`/`$$`, invalid regex, `TooBig` on a huge counted repetition, `replace_all` count and result |
| `…/document.rs` | `revision` increments; `snapshot` shares storage (clone timing independent of size: 16 MiB snapshot < 1 ms in debug); `widest_line_hint` growth; no-op edits return false |
| `va-workspace/tests/paths.rs` | Every row of §4.1 with fake `var`/`is_file`: env > portable > OS, relative and empty values warned and skipped, Windows `USERPROFILE` fallback, Linux `HOME` fallback, no location → `None` |
| `…/config_settings.rs` | Defaults; the three `DEFAULT_SETTINGS_TOML` checks of §4.3; integer for a float key; `exclude` elements; each key valid, wrong type, out of range, invalid value (with line and column); unknown key keeps neighbours; schema 2 ignored; syntax error → defaults + one problem; 1 MiB + 1; non-UTF-8; renderer values incl. `Glow`, `" glow"` |
| `…/config_keybindings.rs` | Chord parsing (modifiers any case, aliases, every `KEY_NAMES` entry), removal, replacement, file order, unknown command, bad key, `when` |
| `…/config_project.rs` | Keys, defaults, reserved tables kept raw, `..` and absolute paths → `cfg-bad-path` |
| `…/persist_atomic.rs` | Temp naming and `create_new`; failure removes the temp; target content all-or-nothing; symlink target preserved (Unix; Windows when symlinks are permitted, else a printed note); permissions kept (Unix) |
| `…/persist_session.rs`, `persist_lock.rs` | Session round trip and bad-part tolerance; second `try_lock_session` → `None`, released on drop |
| `…/persist_backup.rs` | Write/remove order; orphan adoption; missing text reported; newer schema → disabled; restore order |
| `…/persist_kill.rs` | §4.8 kill test |
| `…/i18n_ui.rs` | `resolve_ui_lang` precedence (flag, `VA_LANG`, setting, env, system, default) with fakes; resources parse; en/fr id sets equal; `CONFIG_CODES` covered |

### 10.2 `va-ui` (kittest, headless, no GPU)

| File | Covers |
|---|---|
| `editor_view.rs` | 100,000-line document in a 800×600 harness: `lines_drawn` ≤ visible rows + 2 at top, middle (after `scroll_to_line(50_000)`) and end; content height = lines × row height; long line truncation; gutter width for 9, 99,999 lines |
| `editor_cache.rs` | After a warm frame, typing one char gives `lexed_lines == 1`, `cache_misses == 1`; scrolling back gives only hits; `style_generation` change clears; capacity eviction |
| `editor_input.rs` | Text, Enter with indent, Backspace, arrows, Shift selection, Ctrl word moves, Tab focus lock (focus stays in the editor), Escape collapses carets, Copy/Cut/Paste events with 2 carets, Alt+click adds a caret, double/triple click, IME commit; blink: off → `repaint_delay == Duration::MAX`; on and focused → delay ≤ 530 ms; after 10 s of fake time without input → `MAX` |
| `theme.rs`, `fonts.rs` | §5.4 contrast rules and exhaustive mapping; `font_definitions` registers `va-code` first with the embedded font; `validate_font` rejects junk bytes and an oversize input without panicking; `KEY_NAMES` all map through `egui::Key::from_name` back to the same name |
| `shell_tabs.rs` | Open two files (temp folder), dirty marker, Ctrl+Tab, close dirty tab → prompt with Save/Don't save/Cancel, save writes the file atomically and clears the marker, NotUtf8 banner → reopen as Windows-1252, disk change banner |
| `shell_explorer.rs` | Tree listing, exclude, sort, lazy expansion, rename, open on click |
| `shell_palette.rs` | Ctrl+Shift+P opens with `>`, typing filters (fuzzy order pinned on 6 titles), Enter runs `view.toggle-theme`; Ctrl+P lists project files; `:120` goes to line 120; user keybinding override changes the shown chord |
| `shell_find.rs` | Ctrl+F with a selection, "1 of 3", F3/Shift+F3, regex error message, replace one, replace all = one undo |
| `shell_status.rs` | Ln/Col, encoding, EOL; renderer fallback and test-hook notices from `RendererNote`; read-only session notice; config problem count; `idle_requests_no_repaint` (shell with two open tabs, blink off, after loads: `repaint_delay == MAX`); French labels with `Lang::Fr` |
| `shell_hot_exit.rs` | With a temp data dir: edit → after `backup_delay_ms` (fake time) a backup exists; `on_close_requested` with hot exit → true, no prompt; with `hot_exit = false` → false and the prompt; a new shell over the same data dir restores the dirty tab with its text; disk-changed restore banner |
| `locales.rs` | §4.6 completeness scan (WP-D) |

### 10.3 `visual-assembler` and `xtask`

| File | Covers |
|---|---|
| `apps/…/tests/cli.rs` | M0 tests plus: setting level in `resolve_renderer` (each source, `Settings` source, not-compiled setting → warning and `auto`); `--lang`; the §6.1 usage errors |
| `…/binary.rs` | Linux: `--bench open` without input → 2; `PATH` with `--bench` → 2; M0 cases unchanged |
| `…/bench_report.rs`, `bench_script.rs`, `bench_recorder.rs` | v3 key set, order and the §7.2 example byte for byte; the type script is 500 events, cycle of 25; summary statistics; scenario state machine with fake times |
| `…/shell_app.rs`, `arch_registry.rs` | `ShellApp::draw` under kittest with fake config; `should_show_startup_dialog` truth table; m68k registered with `;`; `Localizer::for_ui` builds for every registered architecture in `en` and `fr` (no duplicate id between `locales/` and the CPU's resources, §4.6; this also covers M2's additions once merged); no-arch build uses `PlainSyntax` (compiled in the glow-only clippy step) |
| `…/renderer_backend.rs`, `fallback.rs` | Unchanged assertions; `with_window_geometry` and `with_bench_vsync` options; fallback tests (ignored, local) still pass |
| `xtask/tests/bench_generate.rs` | 100,000 and 5,000 lines, LF only, deterministic, unique labels, regenerated on version change, kept otherwise |
| `…/bench_runner.rs`, `bench_report.rs` | Run arguments per scenario with `--bench-input`; isolation variables; v3 validation rules; pooled event summary; markdown header; `all` order and default run counts |

### 10.4 Manual checklist (QA, Windows 11 and WSLg X11)

Open a folder, edit, Save; type in a 100k file; multi-cursor; find/replace; palette; zoom; theme toggle; switch to French; native
dialogs (Windows) and the internal prompt (WSLg); kill the process (Task Manager / `kill -9`) with unsaved edits, restart, check
restore and the disk-change banner; second instance shows the read-only notice; startup failure box from Explorer
(`VA_TEST_FAIL_RENDERER=glow,wgpu` set in the user environment, then removed); IME input of `é` on a French keyboard; the renderer
fallback notice (`VA_TEST_FAIL_RENDERER=glow`).

### 10.5 Acceptance commands (QA, Windows and WSL)

```bash
make test && make lint && make docs && make security && make ci
make build
cargo test -p va-workspace --test persist_kill
cargo test -p visual-assembler --test fallback -- --ignored --test-threads=1   # display needed (S19-5)
target/release/visual-assembler --bench open --bench-input bench/generated/100k.asm | cat   # Git Bash; inputs exist after one xtask bench run
cargo xtask bench all --renderer auto          # Performance, reference machine (§11)
```

## 11. Bench plan and targets

| Metric | Scenario and definition | Target (E8) |
|---|---|---|
| Startup | `startup`, outer `spawn_to_first_frame_ms` p95 over 20 runs | < 500 ms |
| Idle frames / CPU | `idle` with the 5k project open: `frames_painted` max = 0; idle CPU median | 0; ≤ 1 % |
| Memory | `idle` with the 5k project: `peak_rss_bytes` median | < 500 MB |
| Open 100k | `open`, `open_ms` p95 over 5 runs | < 1,000 ms |
| Scroll 100k | `scroll`, pooled `event_ms` p95 (vsync off) | < 16 ms |
| Keystroke 100k | `type`, pooled `event_ms` p95 over 1,500 keystrokes (vsync off); max reported | p95 < 16 ms |

Performance agent runs, shipped binary, on mains in "Best performance": P1 Windows `cargo xtask bench all --renderer auto`; P2 WSLg
X11 same; P3 Windows `cargo xtask bench startup --renderer all` (compare with M0's glow p95 302 ms: the shell's cost); P4 Windows
`cargo xtask bench type --renderer wgpu` (informational). The report gives each target, the gap if any with suspected causes, the
binary size against M0, and the exact commands.

## 12. Fragile points

| # | Risk | Mitigation |
|---|---|---|
| F1 | An O(n) step sneaks into the keystroke path (gutter width, content width, dirty check, backups) | §3.3 complexity rule, `widest_line_hint`, `RenderStats` tests, `type` bench |
| F2 | Vsync quantizes frame samples to 16.7 ms | Bench scenarios run with vsync off and report it (§6.8) |
| F3 | Caret blink or a timer keeps egui awake | Blink timeout, one-deadline rule, `idle_requests_no_repaint`, idle bench frames = 0 |
| F4 | egui Tab focus navigation or built-in zoom steals keys | Focus lock filter; `zoom_with_keyboard = false` |
| F5 | ropey line counting differs from `LineIndex` (m1 F7) | Only `cr_lines`; equality test on random texts |
| F6 | Torn backups or a lost index after a kill | Write order, atomic rename, kill test (E4) |
| F7 | Two instances clobber the session | `File::try_lock`, read-only session |
| F8 | M2 runs at the same time. M2 never edits `Cargo.toml`/`Cargo.lock` (m2 §9) but edits the Makefile header comment too | M4 alone owns the manifests and the lock; WP0 (wave 1) lands before M2's WP0 (wave 1b), which applies its Makefile diff on top (§8.2, §14) |

## 13. Deferred, rejected, user decisions

**Deferred** (decided not in M4): diagnostics underline, problems panel, quick-fix application, background assembly (M5); find in
files and file watching (`notify`, M7 if focus-gain checks prove insufficient); explorer delete (needs a trash API); chords; column
(box) selection; word wrap, folding, minimap, split view; embedding the Bold face (when a style needs it); user `.ftl` overrides; a
per-line `LineState` index for stateful dialects; full AccessKit text ranges (M7); scroll precision past ~900,000 lines (f32
offsets); `va` using the OS UI language on Windows (CLI keeps M1 `auto`; a later spec decides); rfd on Linux.

**Candidate `va-core` addition (not required by M4):** `SyntaxProvider::line_comment(&self) -> Option<&'static str>` with default
`None`, which would move the comment prefix out of the app registry (§6.7). M2's WP-A does not add it (its `va-core` scope is fixed by
m2 §3); M5 decides.

**User decisions (2026-10-24, in chat).** Nothing remains open for the user.

| Topic | Decision | Where |
|---|---|---|
| New dependencies | ropey 1.6.1, regex 1.13.1, toml 1.1.8, sys-locale 0.3.2, rfd 0.17.2 (Windows only), egui 0.36.2 direct in `va-ui`, exactly as listed | §8.1–§8.3 |
| Font | Download `JetBrainsMono-2.304.zip` (5,622,857 bytes) from the v2.304 release URL; embed only the Regular TTF and `OFL.txt` | §8.4, §5.5 |
| Caret blink | On by default: 530 ms half-period, solid after 10 s without input, so an idle IDE draws 0 frames | §4.3, §5.6 |
| Close behaviour | VS Code-style hot exit: closing the window never prompts (`files.hot_exit = true`) | §4.3, §4.8 |
| Tabs | A tab character (`editor.insert_spaces = false`) of width 8 (`editor.tab_width = 8`) | §4.3 |
| Theme | Follows the OS (`ui.theme = "auto"`) | §4.3, §5.4 |

## 14. Coordination with M2 (2026-10-24)

M2 ∥ M4 run at the same time. This section is identical in substance to m2 §14; if they ever differ, the Architect fixes both.

### 14.1 Merged wave order

No two work packages of the same wave edit the same file. "Owns" lists the files a package may write; everything else is read-only
for it.

| Wave | WP | Milestone | Owns | Depends on |
|---|---|---|---|---|
| 1 | WP-A core | M2 | `crates/va-core/src/{lib,arch,asm,disasm}.rs`, `crates/va-core/tests/{asm,disasm}.rs` | — |
| 1 | WP0 scaffold | M4 | `Cargo.toml`, `Cargo.lock`, Makefile (header comment and `bench` help line, §8.2), the manifests of `va-editor`, `va-workspace`, `va-ui`, `visual-assembler`, the stubs of §1, `crates/va-ui/tests/support/mod.rs`, `locales/{en,fr}/*.ftl` (headers), `assets/fonts/*` | user approvals (given 2026-10-24) |
| 1b | WP0 scaffold | M2 | `arch/m68k/src/{lib,arch}.rs`, `arch/m68k/src/isa/mod.rs` (one line), stubs under `arch/m68k/src/{asm,disasm}/` and `isa/fields.rs`, `arch/m68k/tests/{asm,disasm}/main.rs` and their empty module files, `apps/va/src/{cli.rs,commands/mod.rs}`, Makefile (m2 §9 diff) | M2 WP-A (trait), M4 WP0 (Makefile, lock) |
| 1b | WP0 follow-up | M2 | `arch/m68k/tests/arch.rs` (one test), `apps/va/tests/lex.rs` (one assertion) (m2 §15.2 W1) | M2 WP0; lands before wave 2, may run beside 1c (disjoint files) |
| 1c | Docs checkpoint | both | READMEs of the folders created in waves 1–1b, `AGENCEMENT.md` (annotations) | 1, 1b |
| 2 | WP-A … WP-H | M4 | the §1 scopes | §1 |
| 2 | WP-B … WP-H | M2 | the m2 §1 scopes (`asm/codes.rs` belongs to WP-F, m2 §15.2 W1) | m2 §1; M2 WP0 follow-up |
| 3 | Security, then QA | M2, M4 | `SECURITY.md` (one milestone at a time); QA reports outside the repository | the milestone's wave 2 |
| 4 | Performance | M2, M4 | `docs/perf/<date>-m2-assembler.md`, `docs/perf/<date>-m4-editor.md`, `docs/perf/README.md` (one milestone at a time) | the milestone's QA |
| 5 | WP-I docs | M2, M4 | READMEs, `CLAUDE.md`, `AGENCEMENT.md`, `DEPENDANCE.md`, `IMPORT.md` (one milestone at a time) | the milestone's waves 3–4 |

- **Wave 2 runs in three batches (§16.4, §17.8, m2 §16.7).** A (merged): M2 WP-B, WP-C, WP-F; M4 WP-A, WP-B, WP-D. B: M2 FX-1
  (first), WP-D, WP-E, WP-G; M4 WP-C, WP-E, WP-H (M4's merged, §17). C: M2 WP-H (after D); M4 FX-1 and M4 FX-2 (fixes, first, beside
  WP-F), WP-F (with `tests/support/mod.rs`, §17.4 F9), then WP-G, whose parts that call no `Shell` method may start beside WP-F
  (§17.8). m2 §15.3 G1 still orders the cross-package tests.
- Waves 3–5 run per milestone: the first milestone to finish wave 2 may enter wave 3 while the other is still in wave 2, because
  wave-2 packages never write the files of waves 3–5. The same phase of the two milestones never runs at the same time:
  `SECURITY.md`, `docs/perf/README.md`, `CLAUDE.md`, `AGENCEMENT.md`, the root and `crates/`/`apps/` READMEs are shared.
- Wave-2 developers write no README and never commit `AGENCEMENT.md`; a local `make docs` rewrite of it is discarded. Each work
  package uses its own worktree and cargo target folder (two `cargo xtask ci` runs on one Windows target folder fail, CLAUDE.md).
  Performance measures with no build or test running on the machine.

### 14.2 Contracts between the two milestones

| Topic | Rule |
|---|---|
| `va-core` | Only M2 edits it (WP-A, wave 1). Every trait method it adds is provided with a neutral default (m2 §3.1, §3.4), so `tests/support/mod.rs` and `va-workspace/tests/i18n_localizer.rs` compile before and after it. M4 relies on no M2 item. `va_editor::encoding::DecodeError` and M2's `va_core::disasm::DecodeError` share a name: import them by path, never by glob. |
| `Architecture` in the GUI | The registry (§6.7) stores `&'static dyn Architecture` and calls only M1 methods; `assembler()`/`disassembler()` are first called by M5. |
| 68000 facts in M4 tests | M4 tests (`apps/visual-assembler/tests/arch_registry.rs` and any other that uses `va_arch_m68k::M68K`) read what M2 owns (the 68000's resources, codes, keyword docs, token streams) through `Architecture` and never pin a count or a text: in wave 2, M2's WP-F takes the resources from two to three per language and the codes from 11 to 61 (m2 §15.2 W1). |
| Lexer | M2 changes how a `MacroName` operand field lexes `<…>` (one `String` token, m2 §4.3); `TokenKind` is unchanged, so the colouring of §5.3–§5.4 needs no change. |
| Fluent | M4 adds `ui-*`, `cmd-*`, `diag-cfg-*` in the root `locales/`; M2 adds `diag-asm-*` and `isa-m68k-dir-*` to `arch/m68k/locales/`. `for_ui` loads both; M4 renders no assembler diagnostic (M5 does, §4.6). |
| `va-workspace` | Only M4 edits it. `apps/va` (M2) uses `Lang`, `LangChoice`, `resolve_lang`, `Localizer::{for_architecture, message, render}`, `RenderedDiagnostic`: M4 keeps their signatures and only adds items (new `LangSource` variants are safe: `va` never matches on it). Through `va-workspace`, `va` links `toml` and, on Windows, `sys-locale` (§8.2); M2's `make security` runs on the merged lock. |
| Project files | `va asm` does not read `project.toml`; `include_dirs` (§4.5) is for the M5 GUI provider, which applies m2 §6.2. |
| Writes | The GUI and `va asm` follow the same atomic-write rule (architecture §6; §4.7, m2 §8.1). |
| Bench inputs | M4 owns `bench/generated/` (§7.1); M2's assembler bench generates its lines in memory (m2 §10.5). Neither reads the other's inputs. |
| Shared files | Makefile (waves 1 then 1b), the closing documents (waves 3–5, one milestone at a time). M4 never edits `arch/*`, `apps/va` or `crates/va-core`. |

## 15. Ratified wave-1 implementation (WP0 and its follow-up, 2026-10-25)

Same precedence as the body it amends; where §15 and an earlier passage differ, §15 wins. Source: the notes of WP0 (`b4e7aae`)
and its follow-up (`570afb1`), checked against the code at `c500c74`.

### 15.1 Ratified as implemented

| # | Decision (the spec now says what the code does) |
|---|---|
| M1 | The `va-editor` stubs are inline modules of `lib.rs` (`text`, `encoding`, `selection`, `edit`, `history`, `document`, `search`), each with the frozen items of §3. Ratified for the scaffold. **WP-A moves** each module to the file §2 names (`src/text.rs`, …) as it implements it, so that `lib.rs` ends with the crate documentation and seven `pub mod` lines and the test files of §10.1 mirror `src/` (CLAUDE.md); the module paths (`va_editor::text::Text`, `va_editor::encoding::DecodeError`, …) never change. |
| M2 | WP0 stubbed every frozen block of the modules it declares, beyond the minimum `ShellConfig` needs: §4.4 and §4.5 in `config/mod.rs`, §4.7 and §4.8 in `persist/mod.rs`, §5.2 in `editor/mod.rs`. Ratified: that is what "compiling stubs that wave 2 then owns" means, and every wave-2 package compiles against the frozen signatures from day 1. **Public-path rule** (generalizes §4.3): the path of a stub item is its public path. An owner who splits a module into the files of §2 (`config/{settings,keybindings,project,problems}.rs`, `persist/{atomic,worker,session,backup,lock}.rs`, `editor/{view,layout,cache,input,caret,gutter,plain}.rs`, the `shell/` files) makes them private modules and re-exports each frozen item by name from `mod.rs` (`pub use`, no glob). So `va_workspace::config::KeyChord`, `va_workspace::persist::IoWorker`, `va_ui::editor::EditorView`, … never move. The split is required by the end of each package, for the same mirroring reason as M1. |
| M3 | Each stub file carries `#![expect(unused_variables, [dead_code,] clippy::todo, [clippy::ptr_arg,] reason = "WP0 stub: …")]` (the workspace sets `clippy::todo = "warn"` and CI denies warnings). Ratified, with this rule, binding for M2's stubs too (m2 §15.1 W6). The list holds exactly the lints the file's stubs trigger. `#[expect]` raises `unfulfilled_lint_expectations` as soon as a listed lint stops firing, so the owner removes each lint in the commit that makes it stop firing in that file (`dead_code` once the placeholder field is used, `unused_variables` once every parameter is read, `clippy::ptr_arg` once the buffer is written) and the attribute with the last `todo!()`. Never `allow` instead, never widened to cover new code. Lint levels are inherited by child modules: a `todo!()` in a new `editor/view.rs` would be absorbed by the expectation of `editor/mod.rs` and fail only when that attribute goes, so a new file that holds a stub carries its own expectation. |
| M4 | Bodies that are not `todo!()` where a signature cannot compile otherwise (an `impl Iterator` return, or a private field that would be dead code): `Text::chunks` (the rope's chunks), `Selections::iter` (the list in order), `ChangeSet::changes` (the sorted changes), `PlainSyntax::keywords` (`&[]`: it has no keywords, §5.2). They are final. `Selections::is_empty` (always `false`; clippy `len_without_is_empty`) is an extra public helper, which the frozen-block rule allows. |
| M5 | The §5.1 items are defined in `shell/mod.rs` and re-exported by name at the crate root: `ArchSupport`, `BenchProbe`, `DialogReply`, `DialogRequest`, `Persistence`, `ReadOnlyReason`, `RendererNote`, `Shell`, `ShellConfig`, `ShellHost`. Both `va_ui::Shell` and `va_ui::shell::Shell` are public paths and stay (WP-F may move the definitions into its `shell/` files under the M2 rule). WP-G imports them from the root. |
| M6 | Since WP0, `va` links `toml` and, on Windows, `sys-locale` through `va-workspace` (`cargo tree -p va -e normal`, checked at `c500c74`), as §8.2 and §14.2 expect. Ratified: no new crate or version; m2 §9 now says "since M4's WP0" instead of "after M4's WP-B". `DEPENDANCE.md`/`IMPORT.md` record it (Documentarian). |

### 15.2 Wave-2 order and cross-milestone notes

- **No change to M4's wave-2 order.** §14.1 gains M2's WP0 follow-up row (wave 1b, two M2 test files, m2 §15.2 W1); it touches no
  M4 file and may run beside the 1c docs checkpoint.
- **New contract (§14.2, mirrored in m2 §14.2):** M4 tests never pin a count or a text that M2 owns; they read the 68000 through
  `Architecture`. The registry test of §10.3 asserts "no duplicate id" and "builds in `en` and `fr`" over whatever
  `locale_resources` returns.
- **Cross-package tests inside M4 follow m2 §15.3 G1:** no merged test calls a `todo!()` of another package. WP-E's and WP-F's tests
  that need a real `Document` merge after WP-A's implementation of what they call; until then they use the stub-free parts or wait.

## 16. Ratified wave-2 batch A (2026-11-02)

Precedence: §16 > §15 > body. Source: the notes of WP-D (`1966bd7`), WP-A (`7324608`) and WP-B (`1cf9729`), checked against the code
at `6553939`. Nothing here is fix-now: batch A needs no code change before batch B.

### 16.1 `va-editor` (WP-A)

| # | Decision |
|---|---|
| A1 | **Ratified (gap filled).** `Document::mark_saved_at(&Snapshot) -> bool` implements §5.7's "marked saved at the snapshot's revision": edits made since keep the document dirty, undoing back to the snapshot's content makes it clean, and it closes the undo group only when the snapshot is the current revision. It knows the last 256 revisions; for an older one it returns `false` and changes nothing. **WP-F** calls it after a successful save; on `false` the tab stays dirty and keeps its backup (never data loss). |
| A2 | **Ratified.** `Document::new` is O(n) (it measures the widest line), which §3.3's O(n) list now includes. **WP-F** builds the whole `Document` on `va-io` with the text (§5.7 open, restore, reload) and sends it to the UI thread; `Document: Send + Sync` is pinned by `tests/document.rs`. |
| A3 | **Ratified** as extra public helpers (allowed by the frozen-block rule): `Text::{is_empty, is_char_boundary}` and `Display`, `Eq`, `Default`; `Encoding::ALL`/`from_id`, `LineEnding::ALL`/`from_id`; `Selection::{new, is_reversed}`; `Selections::as_slice`; `Change::is_noop`; `Document::{can_undo, can_redo, add_caret, mark_saved_at}`. WP-E uses `add_caret` for Alt+click (§5.6). |
| A4 | **Ratified** (silent-spec choices): a line break inside inserted or pasted text becomes the document's line ending; an `edit` whose result would pass `MAX_DOCUMENT_BYTES` and grows the text is refused (returns `false`); a different encoding or line ending makes the document dirty; `mark_saved` closes the undo group; `DuplicateLines` inserts the copy above, so the selections end on the lower copy (VS Code's Shift+Alt+Down); Ctrl+D matches case-sensitively, whole words when the selection came from its own word expansion; `find_from` forward = first match starting at or after `from`, backward = last match ending at or before it, wrapping once; nest-limit errors are told from other syntax errors by recompiling (`TooBig`); `\p{Greek}` and other script classes are `Invalid` (the approved `regex` features have no script tables). **Gaps, WP-F:** F3 passes the primary selection's end (Shift+F3 its start), so the current match is not found again; Replace all refuses (notice, nothing changed) a result whose `len_bytes()` passes `MAX_DOCUMENT_BYTES`, since `replace_text` does not check it. |

### 16.2 `va-workspace` i18n (WP-D) and config (WP-B)

| # | Decision |
|---|---|
| D1 | **Ratified.** `VA_LANG=auto` selects the `auto` resolution and skips `ui.language`. General rule: an environment variable set to `auto` is an explicit choice of the automatic rule, so the settings level below it is skipped. **WP-G** applies it to §6.3: `VA_RENDERER=auto` skips `render.renderer`. |
| D2 | **Ratified.** The first non-empty of `LC_ALL`, `LC_MESSAGES`, `LANG` decides, as in M1: `LANG=C` or `de_DE` gives English (`Default`) and the system language is not asked. The system language is asked only when none is set and non-empty (the GUI started from Explorer or the Start menu). |
| D3 | **Ratified.** A system tag follows M1's case-sensitive prefix rule: `fr…` French and `en…` English (source `System`), anything else English (`Default`); Windows returns lowercase language subtags (`fr-FR`). |
| D4 | **Ratified.** `crates/va-ui/tests/locales.rs` keeps its own `REGISTRY` copy of the 44 commands of §5.12 and checks it against `shell/commands.rs` once that file exists. WP-F keeps its registry equal to §5.12 (a difference fails that test); a change of §5.12 goes back to the Architect. |
| B1 | **Ratified.** `config/problems.rs` is tested through `tests/config_settings.rs` (§2 lists no `config_problems.rs`); the Documentarian says so in `tests/README.md`. |
| B2 | **Ratified.** "Absolute" means absolute on the running OS (on Windows `/home/x` is not); every `os` other than `windows` uses the Linux rows; each variable is read lazily, at most once; `Paths::portable` is true only when both folders come from the portable level. Extra helpers `Paths::{create_config_dir, create_data_dir, create_backups_dir}` (the last two `0o700` on Unix). **WP-C** creates folders only through them, before its first write (`try_lock_session` and `BackupStore::open` included). |
| B3 | **Ratified.** `cfg-wrong-type` `expected` = a type id (`boolean`, `integer`, `number`, `string`, `array`, `table`, selectors of `config.ftl`); `cfg-invalid-value` `expected` = the accepted values joined by `", "`, or `font-file` / `file-name` (selectors); `cfg-out-of-range` `min`/`max` are `Int`; text taken from the file is cut to 64 chars plus `…`; a `schema_version` of `"1"` or `1.0` is another version (`cfg-schema-version`). |
| B4 | **Ratified.** `is_reserved_chord` (extra helper) implements §4.4's list: the arrows, Home, End, PageUp, PageDown, Backspace, Delete without Alt; Enter, Escape, Tab without Ctrl and Alt (Shift+Tab included); Ctrl+C/X/V with any Shift/Alt, Ctrl+Insert, Shift+Insert. A reserved chord is `cfg-bad-key`. Every default of §5.12 stays bindable (Alt+Up, Ctrl+Alt+Up, Shift+Alt+Down, Ctrl+Tab are free). A missing or non-string `key`/`command` is `cfg-wrong-type` on the `[[binding]]`; an invalid `when` is `cfg-invalid-value` and the binding is global; removals apply before user additions. |
| B5 | **Ratified.** `project.toml` paths are refused (`cfg-bad-path`) when empty, starting with `/` or `\` (UNC included), with a drive prefix, or with a `..` component, on every OS. Extra helper `project_arch_problem` builds the unknown-`arch` problem of §4.5 (WP-G calls it). The M5 GUI provider still applies m2 §6.2 on top (colon, device names, symlink escape). |
| B6, B7 | `data_folders_are_private_on_unix` has not run yet: QA runs it under WSL (E1). The `cargo fmt -p va-workspace` run in the shared checkout rewrote only WP-B's files and CI is green: no action; wave-2 packages keep their own worktrees (§14.1). |

### 16.3 APIs batch B builds on (all exist with a decided shape)

| WP | Frozen items it reads (file: item) |
|---|---|
| WP-C persist | `va-workspace/src/paths.rs`: `Paths` and its six path accessors, `create_data_dir`, `create_backups_dir` (B2); `config::ThemeKind` (§4.3); the §4.7–§4.8 items it owns in `persist/mod.rs` (stubs, split under §15.1 M2) |
| WP-E view | `va-editor` (§3): `Document` (+ `add_caret`), `Text`, `display_col`, `byte_at_display_col`, `Selection`, `Selections`, `EditCommand`, `Motion`, `EditOptions`, `is_word_char`; `va-core::syntax::{SyntaxProvider, Token, TokenKind, LineState}`; `va_workspace::config::KEY_NAMES` (fonts test); the stubs it owns: `editor/mod.rs` (§5.2), `theme.rs` (§5.4), `fonts.rs` (§5.5), `tests/support/mod.rs` |
| WP-H bench | §7 only: `xtask` stays free of workspace crates; `generate.rs` (§7.1) is new; the GUI side of `va.bench.run/3` is WP-G's (batch C) |

### 16.4 Batch-B order and Security

- **Order.** M4 WP-C, WP-E and WP-H run in parallel in batch B; all their inputs are merged (WP-A for WP-E's tests, m2 §15.3 G1).
  WP-F (needs WP-C and WP-E) and then WP-G run in batch C. Mirrored in §14.1 and m2 §16.7.
- **For Security (wave 3):** B2 (`0o700` data and backup folders; portable only when both folders are portable), B5 (project path
  refusals on every OS, S4-1), A4 (the 16 MiB cap of `edit` and WP-F's Replace-all check), B3 (64-char excerpts in problem
  arguments). No new `unsafe`, network API or allow-list entry in batch A.

## 17. Ratified wave-2 batch B (2026-11-04)

Precedence: §17 > §16 > §15 > body. Source: the notes of WP-H (`8f60ccf`), WP-C (`0d17640`) and WP-E (`2cc6e9e`), checked against
the code at `2cc6e9e`. Two findings are risky and get small fix packages (M4 FX-1, M4 FX-2, §17.6); everything else is ratified as
implemented. Body passages amended here: §1 (who edits `tests/support/mod.rs` in batch C), §5.2–§5.3 (`MAX_DRAWN_CHARS` counts
display columns, the line cache has a size budget), §5.5 (the font trial text), §6.2 steps 6–7 (fonts and zoom in the app creator,
the lock taken by the shell), §7.1 (labels are unique in each file), §10.1 (the worker tests live in `persist_atomic.rs`).

### 17.1 Ratified as implemented: persist (WP-C)

| # | Decision (the spec now says what the code does) |
|---|---|
| C1 | **Ratified.** The worker tests live in `tests/persist_atomic.rs` (§2 lists no `persist_worker.rs`; the Documentarian says so in `tests/README.md`, as B1). `va-io` waits on `recv()`: it has no periodic work, because the shell keeps every deadline (F11). This is architecture T5's loop with no deadline; T5's choice (std `mpsc`, no async runtime) holds. A panicking job is caught (T6) and drops its result channel, so its receiver sees a disconnection (F2). |
| C2 | **Ratified.** `write_atomic`: an existing target whose permissions are read-only is refused with `PermissionDenied` before anything is written, on every OS (Windows refuses that rename anyway; one rule for §5.7's "notice and offer Save As"). An existing path goes through `canonicalize`, so a dangling symlink is an error and a link is never replaced. A temp name that already exists is skipped (100 names, then `AlreadyExists`); a drop guard removes the temp file on every error and on a panic of the writer; on Unix the folder is synced after the rename, best effort (the write has already succeeded). |
| C3 | **Ratified.** `parse_session`: a missing `schema_version` means 1 (as §4.2), a BOM is skipped, a float key also takes a TOML integer (as §4.3), and a `[[tab]]` with both `path` and `backup` is dropped (§4.7: exactly one). `session_to_toml` writes `workspace = {}` when there is no root. |
| C4 | **Ratified (gap filled).** `backups/index.toml` stores `disk_modified_ms`, so a restored stamp is exact to the millisecond only. Extra helper `DiskStamp::matches(&self, recorded: &DiskStamp) -> bool` compares the length and the time to the millisecond; it is the only way stamps are compared (F7), never `==`. |
| C5 | **Ratified.** Damage in `BackupStore::open`: an index that is not valid TOML, not UTF-8, or has an invalid `schema_version` (not an integer, or below 1) is `Unreadable` and its texts come back as orphans; a newer `schema_version` gives `NewerSchema`, no item, the texts untouched and a disabled store (`write` and `remove` return `Unsupported`); a malformed or repeated `[[backup]]` entry is skipped as `Unreadable`; a `MissingText` entry leaves the index at its next write; a text that cannot be read (over 16 MiB, not UTF-8, an I/O error) is reported, not restored, and kept with its entry. Caps: the index `MAX_CONFIG_BYTES` (1 MiB), a text `MAX_SOURCE_BYTES` (16 MiB). An index over its cap fails `open` today: M4 FX-2 makes it damage like the others. |
| C6 | **Ratified.** `write` and `remove` refuse an id that is not well formed (`InvalidInput`): `BackupId`'s field is public, so this check is what keeps every name inside `backups/`. Each write re-creates `backups/` through `Paths::create_backups_dir` (B2). A `file` that is not valid Unicode is stored as an untitled entry titled with the lossy path, so its text is still restored. |
| C7 | **Ratified** as extra public helpers: `BackupId::{generate, parse, is_well_formed}` (48 bits of milliseconds and a 16-bit counter, always above every id `open` has seen, so ids sort in creation order), `BackupStore::{entries, is_disabled}`, `restore_order(items, &session)` (the §4.8 tab order, stable), `utc_timestamp(SystemTime)` (RFC 3339 UTC to the second, the `updated_utc` format) and `DiskStamp::matches` (C4). |
| C8 | **Ratified.** Temp files that a kill leaves in `backups/` (`<id>.txt.<pid>.<n>.tmp`, `index.toml.<pid>.<n>.tmp`) are never read and never deleted; a kill leaves at most one per write in flight, so at most two. *Rejected for M4:* deleting them in `open`, which cannot know whether the lock is held (and the kill test pins that planted temp files survive). M7 decides a clean-up of the data folder together with the path sandbox. |
| C9 | **Ratified.** The kill delays (5–200 ms) count from the helper's ready line (its store is open), so every kill lands in the write loop, not in the process start. The `persist_*` tests have run on Linux only: QA runs them on the Windows host in wave 3 (E1). |

### 17.2 Ratified as implemented: editor view, themes, fonts (WP-E)

| # | Decision (the spec now says what the code does) |
|---|---|
| E1 | **Ratified.** The line cache is cleared when its key changes (`style_generation`, the font, `tab_width`, `pixels_per_point`, the theme) and when egui recreates its font atlas (new definitions, a full atlas), because galleys hold atlas coordinates. The recreation is detected in O(1) per frame: egui hands back a new `Arc` for a probe galley laid out every frame. The view therefore clears itself on every §5.3 event; WP-F still bumps `style_generation` (F10). An egui bump that changed its galley cache would show as misses in `editor_cache.rs`, never as wrong text. |
| E2 | **Ratified, amended by M4 FX-1.** A line is fetched up to `4 × MAX_DRAWN_CHARS` bytes (16 KiB) and cut to `MAX_DRAWN_CHARS` chars; only that prefix is lexed and keys the cache, with a `truncated` flag (the mark ` …` in the comment colour). Content width = `(min(widest_line_hint, MAX_DRAWN_CHARS) + 2) × advance + gutter`, widened to the widest galley drawn so far. |
| E3 | **Ratified, amended by M4 FX-1 (trial text).** `validate_font` checks the size, then lays out text with the font alone in its family, in a scratch `Context`, under `catch_unwind`; the text must get a non-zero width. No glyph-coverage check: egui's `has_glyph` gives false negatives in a family of one font. The default panic hook still prints a parse panic on stderr (invisible in the Windows release build). |
| E4 | **Ratified.** Copy and Cut with every selection empty leave the clipboard unchanged (*rejected:* VS Code's copy of the whole line, not in §5.6); Cut in a read-only editor only copies. Escape takes two steps: drop the secondary carets, then collapse the primary selection. The editor ignores Alt chords and Ctrl with Up, Down, PageUp, PageDown, Enter and Tab, so those reach the keymap; Shift+Enter is Enter. |
| E5 | **Ratified.** `scroll_to_line(line)` makes `line` the top line at the next `show`, clamped so that the document end stays at the bottom. An edit or caret move made outside the view (keymap command, find, undo) is noticed by an O(1) fingerprint (revision, selection count, primary, first and last selection): the primary caret is then scrolled into view and the blink restarts, as after the view's own input. The first `show` of a view never scrolls to the caret, so a restored top line holds (F4). |
| E6 | **Ratified.** Ctrl + wheel (or a pinch) over the editor changes `ctx.zoom_factor()` itself (egui's `zoom_delta`, clamped by `clamp_zoom` to 0.5–3.0); the keyboard zoom belongs to the keymap (F6). |
| E7 | **Ratified.** Carets are drawn only while both the editor and the window have the focus. The current-line background and `gutter_active` apply to every line that holds a caret. Gutter columns: one padding column, the digits (at least 3), the M5 marker, one gap; 2 columns (marker, gap) with line numbers off. |
| E8 | **Ratified.** A `FontId` whose family is not registered falls back to `Monospace` of the same size, so the view never panics before `setup_context` takes effect. IME pre-edit text is drawn at the primary caret (hidden meanwhile) and never inserted; only `Commit` inserts. |
| E9 | **Ratified** as extra public helpers: `EditorView::{id, metrics, cached_lines}`, `ViewMetrics`, `gutter_digits`, `gutter_columns`; `Theme::from_id`, `effective_theme(session_override, setting, system)` (the §5.4 rule); `fonts::{code_font, setup_context, clamp_zoom, step_zoom, EMBEDDED_FONT_NAME, CUSTOM_FONT_NAME}`, where `setup_context(ctx, custom)` installs `font_definitions(custom)` (active from the next pass) and sets `zoom_with_keyboard = false`. `tests/support/mod.rs` adds `settle`, `click_at`, `EditorState`, `editor_harness`, `editor_style`, `EDITOR_LABEL` and keeps `#![allow(dead_code)]` with its reason: shared test code, of which each test binary uses a subset (the `expect` rule of §15.1 M3 is for stubs). |

### 17.3 Ratified as implemented: bench runner (WP-H)

| # | Decision (the spec now says what the code does) |
|---|---|
| H1 | **Ratified for M4.** `template_line(n)` numbers labels by the line of each file, so `main.asm` and the nine identical `partNN.asm` all define `l0:`, `l5:`, …: labels are unique in each file, not across the project (§7.1 now says so). Nothing assembles in M4. **M5**, whose background assembly would see these duplicates, raises `GENERATOR_VERSION` to 2 with project-wide unique labels (part k numbers its lines from 500 × k); the signature of `template_line` stays. |
| H2 | **Ratified.** Only `startup` prints its run line at the first frame, so `spawn_to_first_frame_ms` is null for `idle`, `open`, `scroll` and `type`, whose lines arrive after the measurement (m0 §17.8). §11's startup target reads the `startup` scenario only. |
| H3 | **Ratified.** `input` is null for a run without `--bench-input` (startup); otherwise it is the last path component, lossy UTF-8 (`5k`, `100k.asm`). |
| H4 | **Ratified.** In `summary`, `open_ms` (median, p95, min, max over runs) is null unless every measured run has one, and `event_ms` is null for startup, idle and open. Both follow `idle_cpu_percent`; the pooled keys are `median`, `p95`, `max`, `count`, in that order. |
| H5 | **Ratified.** The inputs are regenerated when `VERSION` is missing or different, or any generated file is missing. Generation removes `VERSION` first, rewrites `100k.asm`, replaces the `5k/` folder (a symlink there is removed as a link, never followed) and writes `VERSION` last, so an interrupted generation is redone. |
| H6 | **Ratified.** The profile folder `<target>/bench/profile-<utc>-<pid>/` stays after the command, for inspection (`make clean` removes it). The report's `env` map keeps the eight v2 variables: `VA_CONFIG_DIR` and `VA_DATA_DIR` are the runner's own and always point into the profile. |
| H7 | **Ratified.** Run deadlines stay 75 s (startup, open, scroll, type) and N + 75 s (idle), strictly above the GUI's watchdogs of 60 s and 60 + N + 3 s (G4). |
| H8 | **Ratified.** xtask checks neither `input_lines` nor `event_summary_ms`: the report recomputes the pooled statistics from `event_ms`, and WP-G's tests pin both (G2). |

### 17.4 Decided for WP-F (batch C)

| # | Rule |
|---|---|
| F1 | **Lock and persistence.** `ShellConfig::persistence` arrives as WP-G found it before the window (G1): `Off`, `ReadOnly { reason: NoLocation }` or `ReadWrite`. With `ReadWrite`, the shell's first-frame job on `va-io` (§6.2 step 7) calls `try_lock_session` once: `Ok(Some(lock))` keeps the `SessionLock` in the shell until it is dropped, after `shutdown`'s flush; `Ok(None)` makes the session `ReadOnly { reason: OtherInstance }` and `Err` makes it `ReadOnly { reason: NotWritable }`, for the rest of its life, each with its status notice (§5.11). Only a held lock opens the `BackupStore`, restores the session's project and tabs, and writes the default `settings.toml`; a read-only or bench session does none of these (they belong to the instance holding the lock) and starts with the welcome page plus `open_at_start`. `shell_hot_exit.rs` checks that a `ReadWrite` shell whose lock the test holds becomes `ReadOnly { OtherInstance }` and, like an `Off` shell, writes nothing into its temp data folder. *Rejected:* WP-G taking the lock before the window (main-thread I/O beyond step 3's list, T1). |
| F2 | **Hot exit needs a working store.** `on_close_requested` with `files.hot_exit = true` closes without a prompt only when every dirty buffer is protected: the lock is held, the store opened and is not disabled, and the buffer's latest backup write did not fail. Otherwise it behaves as with `hot_exit = false` (Save all / Don't save / Cancel). A `va-io` job whose result channel disconnects without a result panicked (C1): the shell shows a notice, and after a backup job the buffer counts as unprotected. |
| F3 | **Restore.** `BackupStore::open`, then `restore_order(items, &session)`, gives the tabs in order. An `Entry` with a `file`: a dirty tab with the backup text, its encoding from `Encoding::from_id(&entry.encoding)` (unknown: UTF-8), the §4.8 banner when `stamp(file)` does not `matches` the entry's `disk` (or the entry has none), the old path as title when the file is gone. An `Entry` without a file: an untitled tab titled `entry.title`. An `Orphan`: an untitled "Recovered" tab. Every restored tab keeps the item's `BackupId`, so its next `write` replaces or adopts that backup, never a second copy; a new buffer takes `BackupId::generate()` at its first backup, with `updated_utc = utc_timestamp(now)`. `MissingText` and `Unreadable` give one notice that lists them (the texts stay on disk); `NewerSchema` gives a lasting notice and F2 applies. The store is used only inside `va-io` jobs, never on the UI thread (T1). |
| F4 | **Scrolling.** A restored tab gets its selection from `line` and `column` (clamped, §4.7) and `scroll_to_line(top_line − 1)` before its view's first `show` (E5); `SessionTab::top_line` is written as `EditorView::top_line() + 1`. `Shell::go_to_line(line)` takes a 0-based line (§3; the palette's `:n` passes n − 1), puts the primary caret at its start and calls `scroll_to_line(line)`. Find, undo and the other commands only change the document or the selections: the view follows the caret itself (E5). |
| F5 | **Fonts.** `Shell::new` has no `Context`, so the startup call `fonts::setup_context(ctx, None)` is WP-G's (G1); WP-F's shell tests make it on their first frame, as `support::editor_harness` does. After the first frame the shell resolves `editor.font_file` (a relative path joins `Paths::config_dir`), reads it with `read_limited(path, MAX_FONT_BYTES)` and runs `validate_font`, both on `va-io`. On `Ok` it calls `setup_context(ctx, Some(font))` and bumps `style_generation`; on an error it keeps the embedded font and shows a transient notice. A settings reload that changes `font_file` repeats this; one that clears it calls `setup_context(ctx, None)`. |
| F6 | **Zoom.** `view.zoom-in` and `view.zoom-out` call `ctx.set_zoom_factor(step_zoom(ctx.zoom_factor(), 1))` and `(…, -1)`; `view.zoom-reset` sets 1.0. Because the editor zooms too (E6), the shell reads `ctx.zoom_factor()` every frame: a value that differs from `session.ui.zoom` is stored there (which schedules the 2 s session write) and shown in the status bar. The startup zoom is WP-G's (G1): the shell never applies `session.ui.zoom` itself. |
| F7 | **Disk stamps.** Every stamp comparison (the save check, the focus-gain reload banner, the restore banner) goes through `DiskStamp::matches`. The open job takes `stamp` before `read_limited`, so a change during the read shows up later as a change, never silently; the save job takes it after the rename. |
| F8 | **kittest.** With blink on, the editor keeps asking for a wake-up within kittest's `predicted_dt` (0.25 s), so `Harness::run()` exceeds its `max_steps` (4) and panics. Shell tests set `cursor_blink = false` unless they test the blink, and drive frames with `step()`, `support::settle` and `support::wait_until`; they call `run()` only when nothing is pending. |
| F9 | **`tests/support/mod.rs` in batch C.** WP-E is closed, so the file joins WP-F's scope for batch C; no other batch-C package writes `crates/va-ui/tests/` except M4 FX-1, which keeps its helpers in `editor_cache.rs`. Additions only: WP-F may move there the shell helpers that several `shell_*.rs` files share; existing items keep their signatures and behaviour, and `allow(dead_code)` stays. This replaces §1's "asks WP-E for a change". |
| F10 | **`style_generation`.** WP-F still bumps it on every §5.3 event (theme, font, size, zoom, `pixels_per_point`, tab width, a custom font installed or dropped); the view's own key (E1) makes a missed bump harmless. |
| F11 | **Timers.** `va-io` has no timer (C1). The backup delay, the 2 s session write, autosave and the transient notices are deadlines kept by the shell: each requests one `request_repaint_after` while it is pending (§5.6 idle rule) and submits its job when it passes. |
| F12 | **Escape.** While the find bar is open, Escape closes it wherever the focus is (the shell consumes it before the editor, in the §5.12 order); otherwise the editor's two-step Escape applies (E4). |

### 17.5 Decided for WP-G (batch C)

| # | Rule |
|---|---|
| G1 | **Startup.** WP-G sets `Persistence::Off` in bench mode, `ReadOnly { reason: NoLocation }` when `resolve_paths` gives `None`, else `ReadWrite`; the lock is the shell's (F1), so §6.2 step 7's "take the session lock" runs in the shell's first-frame job. The app creator (§6.2 step 6, "fonts registered") calls `fonts::setup_context(&cc.egui_ctx, None)` and `cc.egui_ctx.set_zoom_factor(fonts::clamp_zoom(session.ui.zoom))` before it builds `ShellApp`, so the first frame already has the code font, no egui keyboard zoom and the restored zoom. |
| G2 | **Run line `va.bench.run/3`.** The 29 keys in §7.2 order. `input`: the last path component of `--bench-input`, lossy UTF-8 (`5k`, `100k.asm`), null without one. `input_lines`: the lines opened (idle 5,000 over its ten files; open, scroll and type 100,000), null for startup. `vsync`: the value used (`false` for open, scroll, type). `open_ms`: non-null exactly for open, scroll and type. `event_count`, `event_ms`, `event_summary_ms`: non-null exactly for scroll (300) and type (500). `apps/visual-assembler/tests/bench_report.rs` pins this table; xtask checks only part of it (§7.2, H8). |
| G3 | **When the line is printed.** `startup` at its first frame, as in M0; `idle`, `open`, `scroll` and `type` after their measurement, so xtask records `spawn_to_first_frame_ms = null` for them (H2). |
| G4 | **Watchdogs.** `STARTUP_WATCHDOG_SECS` (60 s) for startup, open, scroll and type; idle keeps 60 + N + 3 s, with the project load and the wait for `pending_io == 0` inside its first 60 s. xtask's 75 s and N + 75 s (H7) must stay strictly larger: changing either GUI limit goes back to the Architect, who changes the xtask constant in the same decision. |
| G5 | **Profile.** Every bench run gets `VA_CONFIG_DIR` and `VA_DATA_DIR` set to fresh, empty, absolute folders, which `resolve_paths` takes at the environment level (§4.1): `settings.toml`, `keybindings.toml` and `session.toml` are absent, so defaults apply. `Persistence::Off` creates nothing in either folder: no default `settings.toml`, no `session.lock`, no `backups/` (F1). |
| G6 | **Idle input.** `bench/generated/5k` is a project folder (`.visualasm/project.toml` with `arch = "m68k"` and `entry = "main.asm"`); idle opens it as the project and its ten `.asm` files, sorted, as tabs (§6.8). Its labels repeat across files (H1), which M4 never assembles. |

### 17.6 Decided: fix packages M4 FX-1, M4 FX-2 and the docs debt

**M4 FX-1 — `va-ui` bounds (fix-now; one developer, one commit; first in batch C, beside WP-F).** The cap of §5.2 counts source chars
and the cache of §5.3 counts entries, so neither bounds memory: a galley costs about 170 bytes per glyph (glyph data, four vertices,
six indices), so 4,096 cached lines of 4,096 chars hold about 2.8 GB, and one line of 4,096 tabs at `tab_width = 16` lays out 65,536
glyphs. §5.2's `MAX_DRAWN_CHARS` now counts **display columns** (glyphs laid out), and the cache gets a size budget. The font trial
covers the text a student types, not one line, since a font that passes the trial and panics later does so on the UI thread.
- `crates/va-ui/src/editor/cache.rs`: `build_job` stops before the first char that would end past display column `MAX_DRAWN_CHARS`
  (a tab counts its expanded width; a tab that crosses the cap is left out), and such a line shows the truncation mark. The cache
  holds at most `LINE_CACHE_CHAR_BUDGET = 1 << 19` display chars in all, besides `LINE_CACHE_CAPACITY` entries; an insertion that would
  pass either limit evicts the least recently used entries down to three quarters of both, never an entry of the current frame (about
  90 MB of galleys at most, plus the frame on screen). The module documentation says so.
- `crates/va-ui/src/editor/mod.rs`: `pub use cache::{LINE_CACHE_CAPACITY, LINE_CACHE_CHAR_BUDGET};` (by name, §15.1 M2).
- `crates/va-ui/src/fonts.rs`: the trial of `validate_font` lays out every printable ASCII char (U+0020–U+007E) and U+00A0–U+00FF
  instead of one line of code, still requiring a non-zero width; `tests/fonts.rs` keeps its assertions (the embedded font passes).
- `crates/va-ui/tests/editor_cache.rs`: scrolling page by page through 600 lines of 4,096 chars keeps `cached_lines()` ≤
  `LINE_CACHE_CHAR_BUDGET / MAX_DRAWN_CHARS` + the last frame's `lines_drawn`; a line of 4,096 tabs with `tab_width = 16` gives a
  content width ≤ gutter + (`MAX_DRAWN_CHARS` + 4) × advance. Its helpers stay in this file (F9).

**M4 FX-2 — two one-place fixes outside every batch-C scope (one developer, one commit; batch C, beside WP-F).**
- `crates/va-workspace/src/persist/backup.rs`: in `BackupStore::open`, an index that `read_limited` refuses as too large (`InvalidData`)
  is damage like a syntax error: `Unreadable("backups/index.toml is larger than 1 MiB")`, its texts come back as orphans, and the next
  write replaces the index. Every other I/O error still fails `open`. The documentation of `open` says so (C5).
- `crates/va-workspace/tests/persist_backup.rs`: an index of `MAX_CONFIG_BYTES` + 1 bytes gives that problem, the texts as orphans and a
  store that writes.
- `xtask/src/main.rs`: the help of `Bench` becomes "Build the shipped GUI once, measure the startup, idle, open, scroll and type
  scenarios per renderer choice, write a JSON report." and the help of `Ci` lists what `cargo xtask ci` runs: "Run the CI checks:
  build configuration, fmt, clippy (all features, glow only, wgpu only, va without any CPU), layering and sources, tests (workspace,
  then va without any CPU)." This file is outside every M4 wave-2 scope, and its help text is code, so it is not the Documentarian's.

**Docs debt (owner: the Documentarian, WP-I's role).** At the M4 batch-B docs checkpoint, which runs beside batch C as `5139d9f` did
after batch A (it writes only READMEs and `AGENCEMENT.md`, which no batch-C package touches), then again in WP-I (wave 5):
`crates/va-workspace/src/persist/README.md` (still the WP0 text), `crates/va-workspace/tests/README.md` (the `persist_*` files, the
worker tests in `persist_atomic.rs`, C1), `crates/va-ui/src/README.md`, `crates/va-ui/src/editor/README.md`,
`crates/va-ui/tests/README.md`, `crates/va-ui/tests/support/README.md`, `xtask/src/bench/README.md` and `bench/README.md` (still v2:
now v3, generated inputs, profiles), `xtask/tests/README.md` (`bench_generate.rs`), and the `AGENCEMENT.md` annotations of the new
files; `make docs` exits 0 at the end. WP-I also updates the m4 precedence line of `CLAUDE.md` (§17 > §16 > §15 > body).

### 17.7 APIs batch C builds on (all exist with a decided shape)

| WP | Items it reads (file: item) |
|---|---|
| WP-F | `va-workspace/src/persist/`: `IoWorker`, `Waker`, `write_atomic`, `read_limited`, `stamp`, `DiskStamp::matches`, `try_lock_session`, `SessionLock`, the session types, `parse_session`, `session_to_toml`, `BackupStore::{open, write, remove, entries, is_disabled}`, `BackupEntry`, `BackupId::generate`, `RestoreItem`, `BackupProblem`, `restore_order`, `utc_timestamp`; `va-ui/src/editor/`: the §5.2 items and E9's helpers; `theme.rs`: `Theme`, `ThemeId`, `effective_theme`; `fonts.rs`: `setup_context`, `validate_font`, `step_zoom`, `MAX_FONT_BYTES`; `va-editor` (§3, §16.1); `va_workspace::config` and `i18n` (§4.2–§4.6, §16.2) |
| WP-G | The `va_ui` root items (§5.1, §15.1 M5); `fonts::{setup_context, clamp_zoom}`; `va_workspace::{paths::resolve_paths, config::{parse_settings, parse_keybindings, project_arch_problem, MAX_CONFIG_BYTES}, persist::{read_limited, parse_session, IoWorker}, i18n::{resolve_ui_lang, Localizer::for_ui}}`; the run-line contract of §7.2 and G2 (xtask is no dependency) |
| M4 FX-1, M4 FX-2 | Their own files only (§17.6) |

### 17.8 Batch-C order

- **M4 FX-1 and M4 FX-2 first**, one commit each, beside WP-F: their files (`editor/{cache,mod}.rs`, `fonts.rs`,
  `tests/editor_cache.rs`; `persist/backup.rs`, `tests/persist_backup.rs`, `xtask/src/main.rs`) belong to no batch-C package. The M4
  batch-B docs checkpoint (§17.6) runs beside them.
- **WP-F**, with `tests/support/mod.rs` added to its scope (F9).
- **WP-G** may start beside WP-F on what calls no `Shell` method (`cli.rs`, the renderer setting, `host.rs`, the startup dialog,
  `arch_registry.rs`, `bench/{script,report}.rs` and their tests); `shell_app.rs`, the recorder's open, scroll and type steps and every
  test that calls a `Shell` method merge after WP-F (m2 §15.3 G1). This refines §16.4's "WP-F, then WP-G".
- **M2 WP-H** follows M2's order (after M2 WP-D) and shares no file with M4. Mirrored in §14.1 and in m2 §14.1.

### 17.9 For Security (wave 3)

| # | What to audit |
|---|---|
| S17-1 | `write_atomic` (C2): canonicalized targets (a symlink is written through, a dangling one refused; a hard link is broken by the rename, as with every atomic save), read-only targets refused, `create_new` temp names with 100 tries, the temp removed on error and on a writer panic, Unix permissions copied (not the owner), the best-effort folder sync. On Unix "read-only" means the mode bits: a file the student may not write, in a folder they may write, is replaced (the OS rule for `rename`); check it against S4-2. |
| S17-2 | Backups (C5, C6, M4 FX-2): the id check is the only guard that keeps names inside `backups/`; only regular files named `b<16 hex>.txt` are read (`DirEntry::file_type`, so a symlink is skipped); caps of 1 MiB (index) and 16 MiB (text); a damaged index gives orphans, a newer schema is never rewritten, a text is kept on every doubt, a non-Unicode path is stored as untitled. |
| S17-3 | Lock and folders (F1, B2): `File::try_lock` is advisory on Unix and may not hold on a network file system (a limitation, with S4-8); a read-only or bench session writes nothing; `0o700` applies to the folders the app creates (the umask can only narrow it), while an existing data folder keeps its mode; the files inside get the default mode in that private folder. |
| S17-4 | Temp files left by kills (C8): never read, never deleted in M4, at most two per kill, each up to 16 MiB (a text) or 1 MiB (the index); the clean-up is M7's. |
| S17-5 | Custom fonts (E3, F5, M4 FX-1): the 32 MiB cap, the trial layout of printable ASCII and Latin-1 under `catch_unwind` on `va-io` (`panic = "unwind"` in every profile, architecture T6). Residual risk to record: a font that passes the trial but panics on another glyph does so on the UI thread (exit 101), at every start until `editor.font_file` is removed. |
| S17-6 | Editor memory (E2, M4 FX-1): at most 16 KiB fetched per drawn line (`4 × MAX_DRAWN_CHARS`), at most `MAX_DRAWN_CHARS` glyphs laid out per line, the cache under `LINE_CACHE_CAPACITY` entries and `LINE_CACHE_CHAR_BUDGET` glyphs; the per-frame work stays O(visible lines). |
| S17-7 | Bench (H5, H6): xtask deletes and rewrites only `bench/generated/{VERSION,100k.asm,5k/}` (a symlinked `5k` is unlinked, never followed) and creates its profiles under `<target>/bench/` with `fs::create_dir`, never shared between commands. |

No new `unsafe` block, network API or allow-list entry in batch B (checked on the three commits; `xtask/src/layering.rs` is unchanged).
