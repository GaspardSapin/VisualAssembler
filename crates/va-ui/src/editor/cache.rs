//! Colouring and the per-line cache (m4-editor-shell.md §5.3).
//!
//! Each drawn line is lexed with `SyntaxProvider::lex_line` (state `INITIAL`, the returned state
//! ignored), turned into a `LayoutJob` with one section per token (colours from the theme, tabs
//! expanded to spaces) and laid out once. The result is cached under the line's exact drawn text,
//! so an edit gives exactly one miss on the next frame and every other visible line hits.
//!
//! The whole cache is cleared when the style changes (`style_generation`, font, tab width,
//! `pixels_per_point`, theme) and when egui recreated its font atlas, which invalidates the glyph
//! coordinates held by every galley (detected with a probe galley, see [`LineCache::begin_frame`]).

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use egui::text::{Galley, LayoutJob, TextFormat, TextWrapping};
use egui::{Color32, FontId};
use va_core::syntax::{LineState, SyntaxProvider, Token};

use super::layout::{TabSpan, byte_at_display, display_index};
use super::view::RenderStats;
use crate::theme::Theme;

/// Most lines the cache keeps; past it, the least recently used ones are evicted by frame stamp.
pub const LINE_CACHE_CAPACITY: usize = 4_096;

/// Entries kept by an eviction: three quarters of the capacity, so evictions are rare.
const KEPT_AFTER_EVICTION: usize = LINE_CACHE_CAPACITY - LINE_CACHE_CAPACITY / 4;

/// Size of the probe text of [`LineCache::begin_frame`], in points.
const PROBE_FONT_SIZE: f32 = 10.0;

/// The truncation mark drawn after a line cut at `MAX_DRAWN_CHARS`.
const TRUNCATION_MARK: &str = " …";

/// What the cached galleys depend on; a change clears the cache.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct StyleKey {
    /// `EditorStyle::style_generation`.
    pub generation: u64,
    /// The requested code font.
    pub font: FontId,
    /// Tab stops.
    pub tab_width: u8,
    /// egui's `pixels_per_point` (zoom and screen scale).
    pub pixels_per_point: f32,
    /// The colours baked into the galleys.
    pub theme: Theme,
}

/// One laid-out line: the galley and what maps its display chars back to bytes.
#[derive(Clone, Debug)]
pub(crate) struct LineLayout {
    /// The galley (one row, tabs expanded, the truncation mark included).
    pub galley: Arc<Galley>,
    /// The tabs of the line, when it has any.
    pub tabs: Option<Arc<[TabSpan]>>,
    /// Display chars of the drawn text, the truncation mark excluded.
    pub display_chars: usize,
    /// Whether the line was cut at `MAX_DRAWN_CHARS`.
    pub truncated: bool,
}

impl LineLayout {
    /// The x offset (from the line's text start) of `byte` of the drawn `line`; bytes past the
    /// drawn text are at its end.
    pub fn x_of(&self, line: &str, byte: usize) -> f32 {
        let display = display_index(line, self.tabs.as_deref(), byte).min(self.display_chars);
        self.galley
            .pos_from_cursor(egui::text::CCursor::new(display))
            .min
            .x
    }

    /// The byte of the drawn `line` nearest to the x offset `x` (from the line's text start).
    pub fn byte_at_x(&self, line: &str, x: f32) -> usize {
        let middle = self.galley.size().y * 0.5;
        let cursor = self.galley.cursor_from_pos(egui::vec2(x, middle));
        let display = cursor.index.0.min(self.display_chars);
        byte_at_display(line, self.tabs.as_deref(), display)
    }
}

/// A cache entry and the frame it was last used in.
struct CachedLine {
    /// The layout.
    layout: LineLayout,
    /// Frame of the last use.
    stamp: u64,
}

/// What [`LineCache::layout`] needs besides the line.
pub(crate) struct LexInput<'a> {
    /// The lexer.
    pub syntax: &'a dyn SyntaxProvider,
    /// The colours.
    pub theme: &'a Theme,
    /// Tab stops (at least 1).
    pub tab_width: usize,
}

/// The per-line cache of one editor view.
pub(crate) struct LineCache {
    /// Layouts keyed by the exact drawn text of their line.
    lines: HashMap<Box<str>, CachedLine>,
    /// The style the entries were laid out with.
    key: Option<StyleKey>,
    /// The galley egui returned for the probe text last frame.
    probe: Option<Arc<Galley>>,
    /// Current frame number.
    frame: u64,
    /// The font used: the requested one when its family is registered, else monospace.
    font: FontId,
    /// Row height of `font`.
    row_height: f32,
    /// Char advance of `font`.
    char_width: f32,
    /// Scratch token list.
    tokens: Vec<Token>,
}

impl fmt::Debug for LineCache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LineCache")
            .field("lines", &self.lines.len())
            .field("key", &self.key)
            .field("frame", &self.frame)
            .finish_non_exhaustive()
    }
}

impl LineCache {
    /// An empty cache.
    pub fn new() -> Self {
        Self {
            lines: HashMap::new(),
            key: None,
            probe: None,
            frame: 0,
            font: FontId::monospace(14.0),
            row_height: 1.0,
            char_width: 1.0,
            tokens: Vec::new(),
        }
    }

    /// Number of cached lines.
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// The font the lines are laid out with.
    pub fn font(&self) -> &FontId {
        &self.font
    }

    /// Row height of the font, in points.
    pub fn row_height(&self) -> f32 {
        self.row_height
    }

    /// Char advance of the font, in points.
    pub fn char_width(&self) -> f32 {
        self.char_width
    }

    /// Starts a frame. Clears the cache, resolves the font and measures it again when `key`
    /// differs from the last frame's, or when egui recreated its fonts (new definitions, a full
    /// atlas): the probe galley egui hands back is then a new allocation, since egui keeps a
    /// galley used every frame and drops its galley cache together with the font atlas.
    pub fn begin_frame(&mut self, ctx: &egui::Context, key: StyleKey) {
        self.frame += 1;
        let probe_job = LayoutJob::simple_singleline(
            "0".to_owned(),
            FontId::monospace(PROBE_FONT_SIZE),
            Color32::WHITE,
        );
        let probe = ctx.fonts_mut(|fonts| fonts.layout_job(probe_job));
        let fonts_recreated = self
            .probe
            .as_ref()
            .is_none_or(|last| !Arc::ptr_eq(last, &probe));
        self.probe = Some(probe);
        if !fonts_recreated && self.key.as_ref() == Some(&key) {
            return;
        }
        self.lines.clear();
        let family_known = ctx.fonts(|fonts| fonts.families().contains(&key.font.family));
        self.font = if family_known {
            key.font.clone()
        } else {
            FontId::monospace(key.font.size)
        };
        let font = self.font.clone();
        (self.row_height, self.char_width) = ctx.fonts_mut(|fonts| {
            (
                fonts.row_height(&font).max(1.0),
                fonts.glyph_width(&font, ' ').max(0.5),
            )
        });
        self.key = Some(key);
    }

    /// The layout of the drawn `line` text: from the cache (a hit), else lexed, laid out and
    /// cached (a miss). Updates the counters of `stats`.
    pub fn layout(
        &mut self,
        ctx: &egui::Context,
        line: &str,
        truncated: bool,
        input: &LexInput<'_>,
        stats: &mut RenderStats,
    ) -> LineLayout {
        if let Some(entry) = self.lines.get_mut(line)
            && entry.layout.truncated == truncated
        {
            entry.stamp = self.frame;
            stats.cache_hits += 1;
            return entry.layout.clone();
        }
        stats.cache_misses += 1;
        stats.lexed_lines += 1;
        self.tokens.clear();
        let _state = input
            .syntax
            .lex_line(line, LineState::INITIAL, &mut self.tokens);
        let built = build_job(line, truncated, &self.tokens, input, &self.font);
        let galley = ctx.fonts_mut(|fonts| fonts.layout_job(built.job));
        let layout = LineLayout {
            galley,
            tabs: (!built.tabs.is_empty()).then(|| Arc::from(built.tabs)),
            display_chars: built.display_chars,
            truncated,
        };
        if self.lines.len() >= LINE_CACHE_CAPACITY {
            self.evict();
        }
        self.lines.insert(
            line.into(),
            CachedLine {
                layout: layout.clone(),
                stamp: self.frame,
            },
        );
        layout
    }

    /// Drops the least recently used entries (by frame stamp) down to [`KEPT_AFTER_EVICTION`];
    /// entries used in the current frame are never dropped.
    fn evict(&mut self) {
        let excess = self.lines.len().saturating_sub(KEPT_AFTER_EVICTION);
        let Some(last_dropped) = excess.checked_sub(1) else {
            return;
        };
        let mut stamps: Vec<u64> = self.lines.values().map(|entry| entry.stamp).collect();
        let (_, &mut threshold, _) = stamps.select_nth_unstable(last_dropped);
        let frame = self.frame;
        self.lines
            .retain(|_, entry| entry.stamp > threshold || entry.stamp == frame);
    }
}

/// A layout job and the column data of its line.
struct BuiltJob {
    /// The job.
    job: LayoutJob,
    /// The tabs, sorted by byte.
    tabs: Vec<TabSpan>,
    /// Display chars of the drawn text.
    display_chars: usize,
}

/// Appends source text to a job, expanding tabs and recording them.
struct JobBuilder<'a> {
    /// The job being built.
    job: LayoutJob,
    /// Tabs found so far.
    tabs: Vec<TabSpan>,
    /// Display column reached.
    col: usize,
    /// The font of every section.
    font: &'a FontId,
    /// Tab stops.
    tab_width: usize,
    /// Scratch text of a segment holding tabs.
    expanded: String,
}

impl JobBuilder<'_> {
    /// Appends `line[start..end]` in `color`.
    fn append(&mut self, line: &str, start: usize, end: usize, color: Color32) {
        let segment = &line[start..end];
        let format = TextFormat::simple(self.font.clone(), color);
        if !segment.contains('\t') {
            self.col += segment.chars().count();
            self.job.append(segment, 0.0, format);
            return;
        }
        self.expanded.clear();
        for (offset, c) in segment.char_indices() {
            if c == '\t' {
                let width = self.tab_width - self.col % self.tab_width;
                self.tabs.push(TabSpan {
                    byte: start + offset,
                    display: self.col,
                    width,
                });
                self.expanded.extend(std::iter::repeat_n(' ', width));
                self.col += width;
            } else {
                self.expanded.push(c);
                self.col += 1;
            }
        }
        self.job.append(&self.expanded, 0.0, format);
    }
}

/// Builds the job of the drawn `line`: one section per token, the gaps a provider might leave
/// (it should not) in the text colour, then the truncation mark when `truncated`.
fn build_job(
    line: &str,
    truncated: bool,
    tokens: &[Token],
    input: &LexInput<'_>,
    font: &FontId,
) -> BuiltJob {
    let job = LayoutJob {
        wrap: TextWrapping::no_max_width(),
        break_on_newline: false,
        keep_trailing_whitespace: true,
        ..LayoutJob::default()
    };
    let mut builder = JobBuilder {
        job,
        tabs: Vec::new(),
        col: 0,
        font,
        tab_width: input.tab_width.max(1),
        expanded: String::new(),
    };
    let mut pos = 0;
    for token in tokens {
        let start = (token.range.start as usize).max(pos);
        let end = (token.range.end as usize).min(line.len());
        if end <= start || !line.is_char_boundary(start) || !line.is_char_boundary(end) {
            continue;
        }
        if start > pos {
            builder.append(line, pos, start, input.theme.text);
        }
        builder.append(line, start, end, input.theme.token_color(token.kind));
        pos = end;
    }
    if pos < line.len() {
        builder.append(line, pos, line.len(), input.theme.text);
    }
    let display_chars = builder.col;
    if truncated {
        let format = TextFormat::simple(font.clone(), input.theme.comment);
        builder.job.append(TRUNCATION_MARK, 0.0, format);
    }
    BuiltJob {
        job: builder.job,
        tabs: builder.tabs,
        display_chars,
    }
}
