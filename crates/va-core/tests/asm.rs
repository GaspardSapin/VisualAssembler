//! Tests for `src/asm.rs`: `CancelToken` sharing, the `PathRefusal` and `SymbolKind` ids, the
//! `AsmLimits`/`AsmOptions` defaults, the provided `SourceProvider` methods, `SymbolTable` order
//! and lookup, `MemorySources` ids and name matching, an `Assembler` reached through a fake
//! `Architecture`, and the default `Architecture::assembler`.
#![allow(clippy::unwrap_used)]

use std::collections::HashSet;

use va_core::arch::Architecture;
use va_core::asm::{
    AsmLimits, AsmOptions, AssembleInput, AssembleOutput, Assembler, CancelToken, IncludeError,
    Listing, ListingLine, MemorySources, PathRefusal, SourceProvider, Symbol, SymbolKind,
    SymbolTable,
};
use va_core::diag::DiagCode;
use va_core::image::{ProgramImage, Segment, SegmentKind, SourceMap, SourceMapEntry};
use va_core::isa::{EaTiming, InstructionInfo, InstructionSet, LookupError};
use va_core::regs::RegisterSpec;
use va_core::source::{FileId, SourceSpan, TextRange};
use va_core::syntax::{KeywordInfo, LineState, SyntaxProvider, Token};

/// Tells whether `id` is a kebab-case identifier (`[a-z]+(-[a-z]+)*`).
fn is_kebab(id: &str) -> bool {
    !id.is_empty()
        && id
            .split('-')
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_lowercase()))
}

/// Compiles only if `T` can move to another thread.
fn assert_send<T: Send>() {}

/// Compiles only if `T` can be shared between threads.
fn assert_sync<T: Sync>() {}

// ---------------------------------------------------------------------------------------------
// CancelToken
// ---------------------------------------------------------------------------------------------

#[test]
fn a_new_token_is_not_cancelled() {
    assert!(!CancelToken::new().is_cancelled());
    assert!(!CancelToken::default().is_cancelled());
}

#[test]
fn clones_share_the_flag_both_ways() {
    let original = CancelToken::new();
    let clone = original.clone();
    clone.cancel();
    assert!(original.is_cancelled());
    assert!(clone.is_cancelled());

    let other = CancelToken::new();
    let other_clone = other.clone();
    other.cancel();
    assert!(other_clone.is_cancelled());
    assert!(other.clone().is_cancelled(), "a clone made after cancel");
}

#[test]
fn separate_tokens_do_not_share_the_flag() {
    let first = CancelToken::new();
    let second = CancelToken::new();
    first.cancel();
    assert!(first.is_cancelled());
    assert!(!second.is_cancelled());
    assert!(!CancelToken::default().is_cancelled());
}

#[test]
fn cancelling_twice_keeps_the_token_cancelled() {
    let token = CancelToken::new();
    token.cancel();
    token.cancel();
    assert!(token.is_cancelled());
}

#[test]
fn a_token_cancelled_on_another_thread_is_seen_here() {
    assert_send::<CancelToken>();
    assert_sync::<CancelToken>();
    let token = CancelToken::new();
    let remote = token.clone();
    std::thread::scope(|scope| scope.spawn(move || remote.cancel()).join().unwrap());
    assert!(token.is_cancelled());
}

#[test]
fn a_token_has_a_debug_form() {
    assert!(format!("{:?}", CancelToken::new()).starts_with("CancelToken"));
}

// ---------------------------------------------------------------------------------------------
// Ids
// ---------------------------------------------------------------------------------------------

#[test]
fn path_refusal_ids_are_the_reasons_of_m2() {
    let expected = [
        (PathRefusal::Empty, "empty"),
        (PathRefusal::Absolute, "absolute"),
        (PathRefusal::Prefix, "drive"),
        (PathRefusal::Colon, "colon"),
        (PathRefusal::DeviceName, "device"),
        (PathRefusal::Escapes, "escapes"),
        (PathRefusal::Nul, "nul"),
    ];
    let mut seen = HashSet::new();
    for (refusal, id) in expected {
        assert_eq!(refusal.id(), id, "{refusal:?}");
        assert!(is_kebab(id), "{id:?}");
        assert!(seen.insert(id), "duplicate id {id:?}");
    }
}

#[test]
fn symbol_kind_ids_are_kebab_case() {
    let expected = [
        (SymbolKind::Label, "label"),
        (SymbolKind::LocalLabel, "local-label"),
        (SymbolKind::Equ, "equ"),
        (SymbolKind::Set, "set"),
        (SymbolKind::Macro, "macro"),
    ];
    let mut seen = HashSet::new();
    for (kind, id) in expected {
        assert_eq!(kind.id(), id, "{kind:?}");
        assert!(is_kebab(id), "{id:?}");
        assert!(seen.insert(id), "duplicate id {id:?}");
    }
}

#[test]
fn include_errors_carry_their_details() {
    let errors = [
        IncludeError::NotFound,
        IncludeError::Refused(PathRefusal::Escapes),
        IncludeError::Refused(PathRefusal::Nul),
        IncludeError::TooLarge { bytes: 16_777_217 },
        IncludeError::NotUtf8 { offset: 3 },
        IncludeError::Unreadable,
        IncludeError::TooManyFiles,
        IncludeError::Unsupported,
    ];
    let distinct: HashSet<IncludeError> = errors.iter().copied().collect();
    assert_eq!(distinct.len(), errors.len());
    let IncludeError::Refused(reason) = errors[1] else {
        panic!("not a refusal");
    };
    assert_eq!(reason.id(), "escapes");
}

// ---------------------------------------------------------------------------------------------
// Limits and options
// ---------------------------------------------------------------------------------------------

#[test]
fn default_limits_are_those_of_m2_section_6() {
    let limits = AsmLimits::DEFAULT;
    assert_eq!(limits.max_files, 256);
    assert_eq!(limits.max_include_depth, 16);
    assert_eq!(limits.max_macro_depth, 64);
    assert_eq!(limits.max_statements, 1_000_000);
    assert_eq!(limits.max_diagnostics, 1_000);
    assert_eq!(limits.max_listing_bytes, 64 * 1024 * 1024);
    assert_eq!(limits.max_relaxation_passes, 16);
    assert_eq!(AsmLimits::default(), AsmLimits::DEFAULT);
}

#[test]
fn default_options_optimize_without_a_listing() {
    let options = AsmOptions::default();
    assert!(options.optimize);
    assert!(!options.listing);
    assert_eq!(options.limits, AsmLimits::DEFAULT);
}

#[test]
fn lowered_limits_are_kept_by_the_options() {
    let options = AsmOptions {
        optimize: false,
        limits: AsmLimits {
            max_macro_depth: 2,
            ..AsmLimits::DEFAULT
        },
        ..AsmOptions::default()
    };
    let copy = options;
    assert_eq!(copy.limits.max_macro_depth, 2);
    assert_eq!(copy.limits.max_files, 256);
    assert!(!copy.optimize);
    assert_ne!(copy, AsmOptions::default());
}

// ---------------------------------------------------------------------------------------------
// SourceProvider defaults
// ---------------------------------------------------------------------------------------------

/// A provider with one text file and none of the optional methods.
struct OneFile;

impl SourceProvider for OneFile {
    fn text(&self, file: FileId) -> Option<&str> {
        (file == FileId(0)).then_some("\tnop\n")
    }

    fn resolve_include(&self, _from: FileId, path: &str) -> Result<FileId, IncludeError> {
        if path.starts_with('/') {
            Err(IncludeError::Refused(PathRefusal::Absolute))
        } else {
            Err(IncludeError::NotFound)
        }
    }
}

#[test]
fn a_provider_without_binaries_gets_the_neutral_defaults() {
    let provider: &dyn SourceProvider = &OneFile;
    assert_eq!(
        provider.resolve_binary(FileId(0), "blob.bin"),
        Err(IncludeError::Unsupported)
    );
    assert_eq!(provider.binary(FileId(0)), None);
    assert_eq!(provider.text(FileId(0)), Some("\tnop\n"));
    assert_eq!(
        provider.resolve_include(FileId(0), "/etc/x"),
        Err(IncludeError::Refused(PathRefusal::Absolute))
    );
}

// ---------------------------------------------------------------------------------------------
// SymbolTable
// ---------------------------------------------------------------------------------------------

/// A symbol of file 0 with no reference.
fn symbol(name: &str, kind: SymbolKind, scope: Option<&str>, value: Option<i64>) -> Symbol {
    Symbol {
        name: name.to_owned(),
        kind,
        value,
        scope: scope.map(str::to_owned),
        definition: SourceSpan {
            file: FileId(0),
            range: TextRange::new(0, 4),
        },
        references: Vec::new(),
    }
}

/// The `(name, scope)` keys of `table`, in table order.
fn keys(table: &SymbolTable) -> Vec<(&str, Option<&str>)> {
    table
        .symbols()
        .iter()
        .map(|symbol| (symbol.name.as_str(), symbol.scope.as_deref()))
        .collect()
}

/// A table with labels, local labels, constants and a macro, given out of order.
fn sample_table() -> SymbolTable {
    SymbolTable::new(vec![
        symbol("loop", SymbolKind::Label, None, Some(0x1010)),
        symbol("1$", SymbolKind::LocalLabel, Some("start"), Some(0x1004)),
        symbol("Zeta", SymbolKind::Equ, None, Some(-1)),
        symbol("1$", SymbolKind::LocalLabel, Some("loop"), Some(0x1012)),
        symbol("alpha", SymbolKind::Macro, None, None),
        symbol("count", SymbolKind::Set, None, Some(3)),
        symbol("start", SymbolKind::Label, None, Some(0x1000)),
    ])
}

#[test]
fn symbols_are_sorted_by_name_then_scope() {
    let table = sample_table();
    assert_eq!(
        keys(&table),
        vec![
            ("1$", Some("loop")),
            ("1$", Some("start")),
            ("Zeta", None),
            ("alpha", None),
            ("count", None),
            ("loop", None),
            ("start", None),
        ]
    );
}

#[test]
fn no_scope_sorts_before_any_scope() {
    let table = SymbolTable::new(vec![
        symbol("x", SymbolKind::LocalLabel, Some("a"), Some(2)),
        symbol("x", SymbolKind::Label, None, Some(1)),
    ]);
    assert_eq!(keys(&table), vec![("x", None), ("x", Some("a"))]);
}

#[test]
fn the_sort_is_stable() {
    let table = SymbolTable::new(vec![
        symbol("dup", SymbolKind::Label, None, Some(1)),
        symbol("a", SymbolKind::Label, None, Some(0)),
        symbol("dup", SymbolKind::Equ, None, Some(2)),
    ]);
    let values: Vec<Option<i64>> = table.symbols().iter().map(|s| s.value).collect();
    assert_eq!(values, vec![Some(0), Some(1), Some(2)]);
    assert_eq!(table.get("dup").unwrap().value, Some(1), "first in order");
}

#[test]
fn get_finds_labels_constants_and_macros() {
    let table = sample_table();
    assert_eq!(table.get("start").unwrap().value, Some(0x1000));
    assert_eq!(table.get("loop").unwrap().kind, SymbolKind::Label);
    assert_eq!(table.get("Zeta").unwrap().kind, SymbolKind::Equ);
    assert_eq!(table.get("count").unwrap().kind, SymbolKind::Set);
    let alpha = table.get("alpha").unwrap();
    assert_eq!(alpha.kind, SymbolKind::Macro);
    assert_eq!(alpha.value, None);
}

#[test]
fn get_ignores_local_labels() {
    let table = sample_table();
    assert_eq!(table.get("1$"), None);
    let shadowed = SymbolTable::new(vec![
        symbol("x", SymbolKind::LocalLabel, Some("a"), Some(2)),
        symbol("x", SymbolKind::Label, None, Some(1)),
        symbol("y", SymbolKind::LocalLabel, Some("a"), Some(3)),
    ]);
    assert_eq!(shadowed.get("x").unwrap().kind, SymbolKind::Label);
    assert_eq!(shadowed.get("y"), None);
}

#[test]
fn get_is_case_sensitive_and_exact() {
    let table = sample_table();
    assert_eq!(table.get("ZETA"), None);
    assert_eq!(table.get("zeta"), None);
    assert_eq!(table.get("Start"), None);
    assert_eq!(table.get("star"), None);
    assert_eq!(table.get("starts"), None);
    assert_eq!(table.get(""), None);
    assert_eq!(table.get("zzz"), None, "past the last name");
}

#[test]
fn an_empty_table_is_the_default() {
    let table = SymbolTable::default();
    assert_eq!(table, SymbolTable::new(Vec::new()));
    assert!(table.symbols().is_empty());
    assert_eq!(table.get("start"), None);
}

#[test]
fn references_are_kept_in_the_given_order() {
    let mut start = symbol("start", SymbolKind::Label, None, Some(0));
    start.references = vec![
        SourceSpan {
            file: FileId(0),
            range: TextRange::new(20, 25),
        },
        SourceSpan {
            file: FileId(1),
            range: TextRange::new(3, 8),
        },
    ];
    let table = SymbolTable::new(vec![start.clone()]);
    assert_eq!(table.get("start"), Some(&start));
}

// ---------------------------------------------------------------------------------------------
// MemorySources
// ---------------------------------------------------------------------------------------------

/// A provider with a root, two text files and a binary file.
fn sample_sources() -> MemorySources {
    MemorySources::new("main.asm", "\tinclude \"inc/defs.i\"\n".to_owned())
        .with_text("inc/defs.i", "LIMIT\tequ\t8\n".to_owned())
        .with_binary("blob.bin", vec![0, 1, 0xFF])
        .with_text("nested.i", String::new())
}

#[test]
fn ids_follow_the_call_order_from_the_root() {
    let sources = sample_sources();
    assert_eq!(sources.name(FileId(0)), Some("main.asm"));
    assert_eq!(sources.name(FileId(1)), Some("inc/defs.i"));
    assert_eq!(sources.name(FileId(2)), Some("blob.bin"));
    assert_eq!(sources.name(FileId(3)), Some("nested.i"));
    assert_eq!(sources.name(FileId(4)), None);
    assert_eq!(sources.name(FileId(u32::MAX)), None);
}

#[test]
fn texts_and_bytes_are_given_by_id() {
    let sources = sample_sources();
    assert_eq!(sources.text(FileId(0)), Some("\tinclude \"inc/defs.i\"\n"));
    assert_eq!(sources.text(FileId(1)), Some("LIMIT\tequ\t8\n"));
    assert_eq!(sources.text(FileId(2)), None, "a binary file has no text");
    assert_eq!(sources.text(FileId(3)), Some(""));
    assert_eq!(sources.text(FileId(4)), None);
    assert_eq!(sources.binary(FileId(2)), Some(&[0, 1, 0xFF][..]));
    assert_eq!(
        sources.binary(FileId(1)),
        Some(&b"LIMIT\tequ\t8\n"[..]),
        "a text file gives its UTF-8 bytes"
    );
    assert_eq!(sources.binary(FileId(4)), None);
}

#[test]
fn include_names_match_exactly() {
    let sources = sample_sources();
    assert_eq!(
        sources.resolve_include(FileId(0), "inc/defs.i"),
        Ok(FileId(1))
    );
    assert_eq!(
        sources.resolve_include(FileId(1), "nested.i"),
        Ok(FileId(3))
    );
    for path in [
        "INC/defs.i",
        "inc/Defs.i",
        "./inc/defs.i",
        "inc//defs.i",
        "defs.i",
        "inc/defs.i ",
        "",
        "inc",
    ] {
        assert_eq!(
            sources.resolve_include(FileId(0), path),
            Err(IncludeError::NotFound),
            "{path:?}"
        );
    }
}

#[test]
fn a_backslash_is_read_as_a_slash() {
    let sources = sample_sources().with_text("sub\\deep\\x.i", String::new());
    assert_eq!(
        sources.resolve_include(FileId(0), "inc\\defs.i"),
        Ok(FileId(1))
    );
    assert_eq!(
        sources.resolve_include(FileId(0), "sub/deep/x.i"),
        Ok(FileId(4))
    );
    assert_eq!(
        sources.resolve_include(FileId(0), "sub\\deep/x.i"),
        Ok(FileId(4))
    );
    assert_eq!(sources.name(FileId(4)), Some("sub\\deep\\x.i"), "as given");
    assert_eq!(
        sources.resolve_include(FileId(0), "inc\\\\defs.i"),
        Err(IncludeError::NotFound)
    );
}

#[test]
fn the_root_can_be_named_by_an_include() {
    let sources = sample_sources();
    assert_eq!(
        sources.resolve_include(FileId(0), "main.asm"),
        Ok(FileId(0))
    );
}

#[test]
fn the_including_file_does_not_matter() {
    let sources = sample_sources();
    for from in [FileId(0), FileId(2), FileId(99)] {
        assert_eq!(sources.resolve_include(from, "inc/defs.i"), Ok(FileId(1)));
        assert_eq!(sources.resolve_binary(from, "blob.bin"), Ok(FileId(2)));
    }
}

#[test]
fn includes_find_text_files_and_incbin_finds_every_file() {
    let sources = sample_sources();
    assert_eq!(
        sources.resolve_include(FileId(0), "blob.bin"),
        Err(IncludeError::NotFound)
    );
    assert_eq!(sources.resolve_binary(FileId(0), "blob.bin"), Ok(FileId(2)));
    assert_eq!(
        sources.resolve_binary(FileId(0), "inc\\defs.i"),
        Ok(FileId(1))
    );
    assert_eq!(
        sources.resolve_binary(FileId(0), "missing.bin"),
        Err(IncludeError::NotFound)
    );
}

#[test]
fn the_first_file_of_a_name_wins() {
    let sources = MemorySources::new("main.asm", String::new())
        .with_binary("x.i", vec![1])
        .with_text("x.i", "first".to_owned())
        .with_text("x.i", "second".to_owned());
    assert_eq!(sources.resolve_include(FileId(0), "x.i"), Ok(FileId(2)));
    assert_eq!(sources.resolve_binary(FileId(0), "x.i"), Ok(FileId(1)));
    assert_eq!(sources.text(FileId(3)), Some("second"));
}

#[test]
fn memory_sources_work_as_a_shared_trait_object() {
    assert_sync::<MemorySources>();
    let sources = sample_sources();
    let provider: &dyn SourceProvider = &sources;
    let text = std::thread::scope(|scope| {
        scope
            .spawn(|| {
                let id = provider.resolve_include(FileId(0), "inc/defs.i").unwrap();
                provider.text(id).map(str::len)
            })
            .join()
            .unwrap()
    });
    assert_eq!(text, Some(12));
}

// ---------------------------------------------------------------------------------------------
// Assembler through an Architecture
// ---------------------------------------------------------------------------------------------

/// A lexer that appends nothing (only valid for empty lines; enough for these tests).
struct NoSyntax;

impl SyntaxProvider for NoSyntax {
    fn lex_line(&self, _line: &str, state: LineState, _out: &mut Vec<Token>) -> LineState {
        state
    }

    fn keywords(&self) -> &[KeywordInfo] {
        &[]
    }
}

/// An instruction set without instructions.
struct NoIsa;

impl InstructionSet for NoIsa {
    fn spellings(&self) -> &[&'static str] {
        &[]
    }

    fn describe(
        &self,
        _spelling: &str,
        _size: Option<&str>,
    ) -> Result<InstructionInfo, LookupError> {
        Err(LookupError::Unknown)
    }

    fn ea_timing(&self) -> &[EaTiming] {
        &[]
    }
}

/// A CPU that implements only the required (M1) methods.
struct PlainCpu;

impl Architecture for PlainCpu {
    fn id(&self) -> &'static str {
        "plain"
    }

    fn syntax(&self) -> &dyn SyntaxProvider {
        &NoSyntax
    }

    fn instruction_set(&self) -> &dyn InstructionSet {
        &NoIsa
    }

    fn registers(&self) -> &'static [RegisterSpec] {
        &[]
    }

    fn diagnostic_codes(&self) -> &'static [DiagCode] {
        &[]
    }

    fn locale_resources(&self, _lang: &str) -> &'static [&'static str] {
        &[]
    }
}

/// An assembler that copies the root text into one code segment at 0, with one `start` label
/// and, when asked, one listing line per source line.
#[derive(Default)]
struct CopyAssembler {
    /// Number of `assemble` calls (shows that `assemble` takes `&mut self`).
    runs: u32,
}

impl Assembler for CopyAssembler {
    fn assemble(&mut self, input: &AssembleInput<'_>, cancel: &CancelToken) -> AssembleOutput {
        self.runs += 1;
        let text = input.sources.text(input.root).unwrap_or("");
        let len = u32::try_from(text.len()).unwrap();
        let start = symbol("start", SymbolKind::Label, None, Some(0));
        let listing = input.options.listing.then(|| Listing {
            lines: va_core::source::lines(text)
                .zip(1..)
                .map(|((_, line), number)| ListingLine {
                    file: input.root,
                    line: number,
                    depth: 0,
                    text: line.to_owned(),
                    address: Some(0),
                    value: None,
                    bytes: Vec::new(),
                    byte_count: 0,
                    skipped: false,
                    optimization: None,
                    errors: 0,
                    warnings: 0,
                })
                .collect(),
            truncated: false,
        });
        let cancelled = cancel.is_cancelled();
        let image = (!cancelled).then(|| {
            let segment = Segment {
                address: 0,
                kind: SegmentKind::Code,
                bytes: text.as_bytes().to_vec(),
            };
            ProgramImage::new(vec![segment], None).unwrap()
        });
        AssembleOutput {
            image,
            diagnostics: Vec::new(),
            symbols: SymbolTable::new(vec![start]),
            source_map: SourceMap::new(vec![SourceMapEntry {
                address: 0,
                len,
                file: input.root,
                line: 1,
                range: TextRange::new(0, len),
            }]),
            listing,
            cancelled,
        }
    }
}

/// A CPU that also provides an assembler.
struct AsmCpu;

impl Architecture for AsmCpu {
    fn id(&self) -> &'static str {
        "asm"
    }

    fn syntax(&self) -> &dyn SyntaxProvider {
        &NoSyntax
    }

    fn instruction_set(&self) -> &dyn InstructionSet {
        &NoIsa
    }

    fn registers(&self) -> &'static [RegisterSpec] {
        &[]
    }

    fn diagnostic_codes(&self) -> &'static [DiagCode] {
        &[]
    }

    fn locale_resources(&self, _lang: &str) -> &'static [&'static str] {
        &[]
    }

    fn assembler(&self) -> Option<Box<dyn Assembler>> {
        Some(Box::new(CopyAssembler::default()))
    }
}

#[test]
fn a_cpu_without_an_assembler_returns_none() {
    let cpu: &dyn Architecture = &PlainCpu;
    assert!(cpu.assembler().is_none());
    assert!(cpu.disassembler().is_none());
}

#[test]
fn an_assembler_is_reached_through_the_architecture() {
    let cpu: &dyn Architecture = &AsmCpu;
    let mut assembler = cpu.assembler().unwrap();
    let sources = MemorySources::new("main.asm", "\tnop\n".to_owned());
    let options = AsmOptions {
        listing: true,
        ..AsmOptions::default()
    };
    let input = AssembleInput {
        root: FileId(0),
        sources: &sources,
        options: &options,
    };
    let output = assembler.assemble(&input, &CancelToken::new());
    assert!(!output.cancelled);
    let image = output.image.unwrap();
    assert_eq!(image.segments()[0].bytes, b"\tnop\n");
    assert_eq!(output.symbols.get("start").unwrap().value, Some(0));
    assert_eq!(output.source_map.at_address(4).unwrap().line, 1);
    let listing = output.listing.unwrap();
    assert_eq!(listing.lines.len(), 2);
    assert_eq!(listing.lines[0].text, "\tnop");
    assert!(!listing.truncated);
    assert!(cpu.disassembler().is_none(), "the other default stays");
}

#[test]
fn a_cancelled_assembly_has_no_image() {
    let mut assembler = AsmCpu.assembler().unwrap();
    let sources = MemorySources::new("main.asm", "\tnop\n".to_owned());
    let options = AsmOptions::default();
    let input = AssembleInput {
        root: FileId(0),
        sources: &sources,
        options: &options,
    };
    let token = CancelToken::new();
    token.clone().cancel();
    let output = assembler.assemble(&input, &token);
    assert!(output.cancelled);
    assert_eq!(output.image, None);
    assert_eq!(output.listing, None, "no listing without the option");
}

#[test]
fn an_assembly_can_run_on_a_worker_thread() {
    assert_send::<Box<dyn Assembler>>();
    assert_send::<AssembleInput<'static>>();
    let mut assembler = AsmCpu.assembler().unwrap();
    let sources = MemorySources::new("main.asm", "\trts\n".to_owned());
    let options = AsmOptions::default();
    let input = AssembleInput {
        root: FileId(0),
        sources: &sources,
        options: &options,
    };
    let token = CancelToken::new();
    let worker_token = token.clone();
    let output = std::thread::scope(|scope| {
        scope
            .spawn(move || assembler.assemble(&input, &worker_token))
            .join()
            .unwrap()
    });
    assert_eq!(output.image.unwrap().segments()[0].bytes, b"\trts\n");
    assert!(!token.is_cancelled());
}

#[test]
fn an_input_is_copied_and_debug_printed_without_its_provider() {
    let sources = MemorySources::new("main.asm", String::new());
    let options = AsmOptions::default();
    let input = AssembleInput {
        root: FileId(7),
        sources: &sources,
        options: &options,
    };
    let copy = input;
    assert_eq!(copy.root, input.root);
    let shown = format!("{input:?}");
    assert!(shown.starts_with("AssembleInput { root: FileId(7), options: AsmOptions"));
    assert!(shown.ends_with(", .. }"));
}

#[test]
fn an_empty_listing_is_the_default() {
    let listing = Listing::default();
    assert!(listing.lines.is_empty());
    assert!(!listing.truncated);
}
