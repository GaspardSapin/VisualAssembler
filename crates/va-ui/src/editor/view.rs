//! The editor widget (m4-editor-shell.md §5.2, frozen): a custom view inside
//! `egui::ScrollArea::both().show_viewport`, never an egui `TextEdit`.
//!
//! Each frame: the style key and the font atlas are checked (a change clears the line cache),
//! the pointer and keyboard input of the frame is applied to the document, the primary caret is
//! scrolled into view after an edit or a caret move, then the lines that intersect the viewport
//! (plus one above and one below) are fetched, laid out through the cache and painted in this
//! order: current-line background, selections, decorations, text, carets, IME pre-edit. The
//! gutter is painted at the viewport's left edge. Every per-frame cost is O(visible lines).

use std::time::Duration;

use egui::containers::scroll_area::{DragScroll, ScrollSource};
use egui::output::IMEOutput;
use egui::{
    IMEPurpose, Id, Margin, Pos2, Rect, Response, ScrollArea, Sense, Stroke, StrokeKind, Ui, Vec2,
    WidgetInfo, WidgetType, pos2, vec2,
};
use va_core::syntax::SyntaxProvider;
use va_editor::document::Document;
use va_editor::edit::EditOptions;
use va_editor::selection::Selection;

use super::cache::{LexInput, LineCache, LineLayout, StyleKey};
use super::caret::{self, Blink};
use super::gutter::{self, Gutter};
use super::input;
use super::layout::{self, Geometry, MAX_DRAWN_CHARS, ViewMetrics};
use crate::fonts;
use crate::theme::Theme;

/// How the editor draws: font, tabs, gutter and caret options.
#[derive(Clone, Debug, PartialEq)]
pub struct EditorStyle {
    /// The code font (family `va-code`) and its size.
    pub font: egui::FontId,
    /// Tab stops, in columns.
    pub tab_width: u8,
    /// Whether the gutter shows line numbers.
    pub line_numbers: bool,
    /// Whether the caret's line gets the current-line background.
    pub highlight_current_line: bool,
    /// Whether the caret blinks (§5.6).
    pub cursor_blink: bool,
    /// Bumped on every change of theme, font, size, zoom, `pixels_per_point` or tab width; a
    /// change clears the line cache.
    pub style_generation: u64,
}

/// What a decoration marks. M5 adds `Diagnostic(Severity)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DecorationKind {
    /// A search match.
    SearchMatch,
    /// The current search match.
    CurrentSearchMatch,
}

/// A decorated byte range of the document.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Decoration {
    /// Start byte.
    pub start: usize,
    /// End byte.
    pub end: usize,
    /// What it marks.
    pub kind: DecorationKind,
}

/// Everything [`EditorView::show`] needs besides the document.
#[derive(Clone, Copy)]
pub struct EditorContext<'a> {
    /// The lexer colouring the lines.
    pub syntax: &'a dyn SyntaxProvider,
    /// The colours.
    pub theme: &'a Theme,
    /// The drawing options.
    pub style: &'a EditorStyle,
    /// The indentation options of the edit commands.
    pub options: EditOptions,
    /// Decorations to paint, sorted by start.
    pub decorations: &'a [Decoration],
    /// Whether edits are refused.
    pub read_only: bool,
    /// The line comment prefix of the architecture, if any.
    pub line_comment: Option<&'static str>,
    /// Accessibility label of the widget (the file name).
    pub label: &'a str,
}

/// What one [`EditorView::show`] did.
#[derive(Clone, Debug)]
pub struct EditorResponse {
    /// The widget's egui response.
    pub response: egui::Response,
    /// Whether the text changed.
    pub changed: bool,
    /// Whether a caret moved.
    pub caret_moved: bool,
}

/// Counters of the last frame, for tests and the bench.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct RenderStats {
    /// Lines laid out and painted.
    pub lines_drawn: usize,
    /// Lines passed to the lexer.
    pub lexed_lines: usize,
    /// Line cache hits.
    pub cache_hits: usize,
    /// Line cache misses.
    pub cache_misses: usize,
}

/// Delay between two autoscroll steps while a drag selection is held past the viewport edge.
const AUTOSCROLL_PERIOD: Duration = Duration::from_millis(16);

/// Largest autoscroll step, in rows, per [`AUTOSCROLL_PERIOD`].
const AUTOSCROLL_MAX_ROWS: f32 = 3.0;

/// A cheap summary of the selections and the text revision, compared between frames to notice
/// edits and caret moves made outside the view (keymap commands, find, undo).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Fingerprint {
    /// Text revision.
    revision: u64,
    /// Selection positions.
    carets: CaretPrint,
}

/// The positions part of a [`Fingerprint`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CaretPrint {
    /// Number of selections.
    count: usize,
    /// Anchor and head of the primary selection.
    primary: (usize, usize),
    /// Anchor and head of the first selection.
    first: (usize, usize),
    /// Anchor and head of the last selection.
    last: (usize, usize),
}

impl Fingerprint {
    /// The fingerprint of `doc`, in O(1).
    fn of(doc: &Document) -> Self {
        let selections = doc.selections();
        let ends = |s: Option<&Selection>| s.map_or((0, 0), |s| (s.anchor, s.head));
        let slice = selections.as_slice();
        Self {
            revision: doc.revision(),
            carets: CaretPrint {
                count: selections.len(),
                primary: ends(Some(selections.primary())),
                first: ends(slice.first()),
                last: ends(slice.last()),
            },
        }
    }
}

/// Sizes computed before the scroll area runs.
#[derive(Clone, Copy, Debug)]
struct Setup {
    /// Row height.
    row_height: f32,
    /// Char advance.
    char_width: f32,
    /// Gutter width.
    gutter_width: f32,
    /// Document lines.
    line_count: usize,
    /// Content size from the document's line count and widest-line hint.
    base_size: Vec2,
}

/// What painting found out.
#[derive(Clone, Copy, Debug, Default)]
struct Painted {
    /// Width of the widest drawn galley.
    widest: f32,
    /// Screen rectangle of the primary caret, when its line was drawn.
    primary_caret: Option<Rect>,
}

/// The editor widget state of one tab: id, line cache, blink and IME state, last frame's stats.
#[derive(Debug)]
pub struct EditorView {
    /// The widget id.
    id: egui::Id,
    /// Layouts of recently drawn lines.
    cache: LineCache,
    /// Blink clock.
    blink: Blink,
    /// IME pre-edit text, drawn at the primary caret.
    preedit: Option<String>,
    /// Counters of the last frame.
    stats: RenderStats,
    /// Geometry of the last frame.
    metrics: ViewMetrics,
    /// First visible line of the last frame.
    top_line: usize,
    /// Line to scroll to on the next frame.
    pending_scroll: Option<usize>,
    /// Anchor of the drag selection while the primary button is held.
    drag_anchor: Option<usize>,
    /// The document as the last frame left it.
    seen: Option<Fingerprint>,
    /// Whether the editor had the focus last frame.
    had_focus: bool,
    /// Reused buffer of the drawn text of one line.
    line: String,
}

impl EditorView {
    /// A view with an empty cache.
    pub fn new(id: egui::Id) -> Self {
        Self {
            id,
            cache: LineCache::new(),
            blink: Blink::default(),
            preedit: None,
            stats: RenderStats::default(),
            metrics: ViewMetrics::default(),
            top_line: 0,
            pending_scroll: None,
            drag_anchor: None,
            seen: None,
            had_focus: false,
            line: String::new(),
        }
    }

    /// The widget id (the id of [`EditorResponse::response`]).
    pub fn id(&self) -> Id {
        self.id
    }

    /// The counters of the last frame.
    pub fn stats(&self) -> RenderStats {
        self.stats
    }

    /// The geometry of the last frame.
    pub fn metrics(&self) -> ViewMetrics {
        self.metrics
    }

    /// Number of lines in the line cache (at most `LINE_CACHE_CAPACITY` after a frame).
    pub fn cached_lines(&self) -> usize {
        self.cache.len()
    }

    /// The first visible line (0-based).
    pub fn top_line(&self) -> usize {
        self.top_line
    }

    /// Draws `doc` and applies this frame's input to it.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        doc: &mut Document,
        ctx: &EditorContext<'_>,
    ) -> EditorResponse {
        self.stats = RenderStats::default();
        let key = StyleKey {
            generation: ctx.style.style_generation,
            font: ctx.style.font.clone(),
            tab_width: ctx.style.tab_width,
            pixels_per_point: ui.ctx().pixels_per_point(),
            theme: *ctx.theme,
        };
        self.cache.begin_frame(ui.ctx(), key);
        let row_height = self.cache.row_height();
        let char_width = self.cache.char_width();
        let line_count = doc.text().line_count().max(1);
        let gutter_width =
            gutter::gutter_columns(line_count, ctx.style.line_numbers) as f32 * char_width;
        let widest = doc.widest_line_hint().min(MAX_DRAWN_CHARS);
        let base_size = vec2(
            gutter_width + (widest + 2) as f32 * char_width,
            line_count as f32 * row_height,
        );
        let mut area = ScrollArea::both()
            .id_salt(self.id.with("scroll"))
            .auto_shrink(false)
            .animated(false)
            .content_margin(Margin::ZERO)
            .scroll_source(ScrollSource {
                scroll_bar: true,
                drag: DragScroll::Never,
                mouse_wheel: true,
            });
        if let Some(line) = self.pending_scroll.take() {
            let max_offset = (base_size.y - ui.available_height()).max(0.0);
            let offset = (line.min(line_count - 1) as f32 * row_height).min(max_offset);
            area = area.vertical_scroll_offset(offset);
        }
        let setup = Setup {
            row_height,
            char_width,
            gutter_width,
            line_count,
            base_size,
        };
        area.show_viewport(ui, |ui, viewport| {
            self.frame(ui, viewport, doc, ctx, &setup)
        })
        .inner
    }

    /// Scrolls so that `line` (0-based) is visible on the next frame: it becomes the top line,
    /// unless the document ends first. A caller outside a frame also requests a repaint.
    pub fn scroll_to_line(&mut self, line: usize) {
        self.pending_scroll = Some(line);
    }

    /// Gives the widget keyboard focus.
    pub fn request_focus(&self, ui: &egui::Ui) {
        ui.memory_mut(|memory| memory.request_focus(self.id));
    }

    /// One frame inside the scroll area: input, scrolling, painting, wake-ups, IME and
    /// accessibility output.
    fn frame(
        &mut self,
        ui: &mut Ui,
        viewport: Rect,
        doc: &mut Document,
        ctx: &EditorContext<'_>,
        setup: &Setup,
    ) -> EditorResponse {
        let origin = ui.max_rect().min;
        let visible = viewport.translate(origin.to_vec2());
        let geom = Geometry {
            origin,
            visible,
            row_height: setup.row_height,
            char_width: setup.char_width,
            gutter_width: setup.gutter_width,
            line_count: setup.line_count,
        };
        let mut response = ui.interact(
            visible.intersect(ui.clip_rect()),
            self.id,
            Sense::click_and_drag(),
        );
        let (now, window_focused) = ui.input(|i| (i.time, i.focused));
        let before = Fingerprint::of(doc);
        let external = self.seen.is_some_and(|seen| seen != before);

        let mut activity = self.handle_pointer(ui, &response, doc, ctx, &geom);
        let focused = response.has_focus();
        if focused {
            ui.memory_mut(|memory| memory.set_focus_lock_filter(self.id, input::FOCUS_LOCK));
            activity |= !self.had_focus;
        } else {
            self.preedit = None;
        }
        let mut changed = false;
        if focused {
            let events = ui.input(|i| i.events.clone());
            let handled = input::handle_events(
                ui.ctx(),
                &events,
                doc,
                ctx,
                geom.page_rows(),
                &mut self.preedit,
                input::edit_time(now),
            );
            changed = handled.changed;
            activity |= handled.activity;
        }
        zoom_with_wheel(ui, &response);

        let after = Fingerprint::of(doc);
        let caret_moved = after.carets != before.carets;
        if changed {
            response.mark_changed();
        }
        let follow_caret = changed || caret_moved || external;
        if follow_caret || activity {
            self.blink.restart(now);
        }
        if follow_caret {
            self.scroll_caret_into_view(ui, doc, ctx, &geom, setup.base_size);
        }
        self.had_focus = focused;
        self.seen = Some(after);

        let carets_on = focused && window_focused;
        let painted = self.paint(ui, doc, ctx, &geom, carets_on, now);
        let content_size = vec2(
            setup
                .base_size
                .x
                .max(setup.gutter_width + painted.widest + 2.0 * setup.char_width),
            setup.base_size.y,
        );
        ui.set_min_size(content_size);
        self.metrics = ViewMetrics {
            row_height: setup.row_height,
            char_width: setup.char_width,
            gutter_width: setup.gutter_width,
            content_size,
            viewport_size: viewport.size(),
        };
        self.top_line = geom.top_line();

        if carets_on
            && ctx.style.cursor_blink
            && let (_, Some(delay)) = self.blink.phase(now)
        {
            ui.ctx().request_repaint_after(delay);
        }
        if focused && !ctx.read_only {
            let to_global = ui
                .ctx()
                .layer_transform_to_global(ui.layer_id())
                .unwrap_or_default();
            let cursor = painted.primary_caret.unwrap_or_else(|| {
                caret::caret_rect(visible.min.x, visible.min.y, setup.row_height)
            });
            ui.output_mut(|output| {
                output.ime = Some(IMEOutput {
                    purpose: IMEPurpose::Normal,
                    rect: to_global * visible,
                    cursor_rect: to_global * cursor,
                    should_interrupt_composition: false,
                });
            });
        }
        let text = doc.text();
        let primary_line = text.line_of(doc.selections().primary().head);
        let label = ctx.label;
        let enabled = ui.is_enabled();
        response.widget_info(|| {
            let mut value = String::new();
            layout::fetch_line(text, primary_line, &mut value);
            let mut info = WidgetInfo::new(WidgetType::TextEdit);
            info.enabled = enabled;
            info.label = Some(label.to_owned());
            info.current_text_value = Some(value);
            info
        });
        EditorResponse {
            response,
            changed,
            caret_moved,
        }
    }

    /// The layout of `line` (through the cache); afterwards `self.line` holds its drawn text.
    fn line_layout(
        &mut self,
        egui_ctx: &egui::Context,
        doc: &Document,
        ctx: &EditorContext<'_>,
        line: usize,
    ) -> LineLayout {
        let truncated = layout::fetch_line(doc.text(), line, &mut self.line);
        let lex = LexInput {
            syntax: ctx.syntax,
            theme: ctx.theme,
            tab_width: usize::from(ctx.style.tab_width),
        };
        self.cache
            .layout(egui_ctx, &self.line, truncated, &lex, &mut self.stats)
    }

    /// The document byte under the screen position `pos` (clamped to the document).
    fn byte_at(
        &mut self,
        egui_ctx: &egui::Context,
        doc: &Document,
        ctx: &EditorContext<'_>,
        geom: &Geometry,
        pos: Pos2,
    ) -> usize {
        let line = geom.line_at_y(pos.y);
        let layout = self.line_layout(egui_ctx, doc, ctx, line);
        let in_line = layout.byte_at_x(&self.line, pos.x - geom.text_left());
        doc.text().line_start(line) + in_line
    }

    /// Mouse input: press (caret, Shift extends, Alt adds a caret), drag selection with
    /// autoscroll past the viewport edge, double-click word, triple-click line. Returns whether
    /// the pointer acted on the editor.
    fn handle_pointer(
        &mut self,
        ui: &Ui,
        response: &Response,
        doc: &mut Document,
        ctx: &EditorContext<'_>,
        geom: &Geometry,
    ) -> bool {
        let (pressed, down, pointer, modifiers) = ui.input(|i| {
            (
                i.pointer.primary_pressed(),
                i.pointer.primary_down(),
                i.pointer.interact_pos(),
                input::press_modifiers(i),
            )
        });
        let mut activity = false;
        if pressed
            && response.hovered()
            && let Some(pos) = pointer
        {
            response.request_focus();
            let byte = self.byte_at(ui.ctx(), doc, ctx, geom, pos);
            self.drag_anchor = Some(input::press(doc, byte, modifiers));
            activity = true;
        } else if down && let (Some(anchor), Some(pos)) = (self.drag_anchor, pointer) {
            let inside = clamp_to(pos, geom.visible);
            let byte = self.byte_at(ui.ctx(), doc, ctx, geom, inside);
            input::drag(doc, anchor, byte);
            let delta = autoscroll_delta(geom, pos);
            if delta != Vec2::ZERO {
                ui.scroll_with_delta(delta);
                ui.ctx().request_repaint_after(AUTOSCROLL_PERIOD);
            }
        }
        if !down {
            self.drag_anchor = None;
        }
        if let Some(pos) = response.interact_pointer_pos().or(pointer) {
            if response.triple_clicked() {
                doc.select_line(geom.line_at_y(pos.y));
                activity = true;
            } else if response.double_clicked() {
                let byte = self.byte_at(ui.ctx(), doc, ctx, geom, pos);
                doc.select_word_at(byte);
                activity = true;
            }
        }
        activity
    }

    /// Asks the scroll area to show the primary caret, with the gutter width and one char of
    /// margin on its left, so the gutter never hides it.
    fn scroll_caret_into_view(
        &mut self,
        ui: &Ui,
        doc: &Document,
        ctx: &EditorContext<'_>,
        geom: &Geometry,
        content_size: Vec2,
    ) {
        let head = doc.selections().primary().head;
        let line = doc.text().line_of(head);
        let in_line = head.saturating_sub(doc.text().line_start(line));
        let layout = self.line_layout(ui.ctx(), doc, ctx, line);
        let x = geom.text_left() + layout.x_of(&self.line, in_line);
        let top = geom.line_top(line);
        let content = geom.content_rect(content_size);
        let left = (x - geom.gutter_width - geom.char_width).max(content.min.x);
        let target = Rect::from_min_max(
            pos2(left, top),
            pos2(
                x + caret::CARET_WIDTH + geom.char_width,
                top + geom.row_height,
            ),
        );
        ui.scroll_to_rect(target, None);
    }

    /// Paints the drawn lines and the gutter; see the module documentation for the order.
    fn paint(
        &mut self,
        ui: &Ui,
        doc: &Document,
        ctx: &EditorContext<'_>,
        geom: &Geometry,
        carets_on: bool,
        now: f64,
    ) -> Painted {
        let theme = ctx.theme;
        let egui_ctx = ui.ctx().clone();
        let visible = geom.visible;
        let painter = ui.painter_at(visible);
        painter.rect_filled(visible, 0.0, theme.background);
        let text_clip = Rect::from_min_max(
            pos2(visible.min.x + geom.gutter_width, visible.min.y),
            visible.max,
        );
        let text_painter = ui.painter_at(text_clip);
        let font = self.cache.font().clone();
        let gutter = ctx.style.line_numbers.then(|| Gutter {
            left: visible.min.x,
            char_width: geom.char_width,
            digits: gutter::gutter_digits(geom.line_count),
            font: &font,
        });
        let caret_shown = carets_on && (!ctx.style.cursor_blink || self.blink.phase(now).0);

        let text = doc.text();
        let lines = geom.drawn_lines();
        let selections = doc.selections().as_slice();
        let primary_index = doc.selections().primary_index();
        let first_start = text.line_start(lines.start);
        let last_end = text.line_end(lines.end.saturating_sub(1));
        let decorations = visible_decorations(ctx.decorations, first_start, last_end);
        let mut next_selection = selections.partition_point(|s| s.end() < first_start);
        let text_left = geom.text_left();
        let row_height = geom.row_height;
        let mut painted = Painted::default();

        for line in lines {
            let start = text.line_start(line);
            let end = text.line_end(line);
            let has_terminator = line + 1 < geom.line_count;
            let layout = self.line_layout(&egui_ctx, doc, ctx, line);
            self.stats.lines_drawn += 1;
            painted.widest = painted.widest.max(layout.galley.size().x);
            let top = geom.line_top(line);
            let row_rect = |left: f32, right: f32| {
                Rect::from_min_max(pos2(left, top), pos2(right, top + row_height))
            };

            while selections
                .get(next_selection)
                .is_some_and(|s| s.end() < start)
            {
                next_selection += 1;
            }
            let on_line_count = selections[next_selection..]
                .iter()
                .take_while(|s| s.start() <= end)
                .count();
            let on_line = &selections[next_selection..next_selection + on_line_count];
            let is_caret_line = on_line.iter().any(|s| (start..=end).contains(&s.head));

            if ctx.style.highlight_current_line && is_caret_line {
                text_painter.rect_filled(
                    row_rect(text_clip.min.x, text_clip.max.x),
                    0.0,
                    theme.current_line,
                );
            }
            for selection in on_line.iter().filter(|s| !s.is_empty()) {
                let from = selection.start().max(start) - start;
                let to = selection.end().min(end) - start;
                let left = text_left + layout.x_of(&self.line, from);
                let mut right = text_left + layout.x_of(&self.line, to);
                if has_terminator && selection.end() > end {
                    right += geom.char_width * 0.5;
                }
                if right > left {
                    text_painter.rect_filled(row_rect(left, right), 0.0, theme.selection);
                }
            }
            for decoration in decorations {
                let from = decoration.start.max(start);
                let to = decoration.end.min(end);
                if to <= from {
                    continue;
                }
                let rect = row_rect(
                    text_left + layout.x_of(&self.line, from - start),
                    text_left + layout.x_of(&self.line, to - start),
                );
                text_painter.rect_filled(rect, 0.0, theme.search_match);
                if decoration.kind == DecorationKind::CurrentSearchMatch {
                    text_painter.rect_stroke(
                        rect,
                        0.0,
                        Stroke::new(1.0, theme.caret),
                        StrokeKind::Inside,
                    );
                }
            }
            text_painter.galley(pos2(text_left, top), layout.galley.clone(), theme.text);
            for (offset, selection) in on_line.iter().enumerate() {
                if !(start..=end).contains(&selection.head) {
                    continue;
                }
                let x = text_left + layout.x_of(&self.line, selection.head - start);
                let rect = caret::caret_rect(x, top, row_height);
                let is_primary = next_selection + offset == primary_index;
                if is_primary {
                    painted.primary_caret = Some(rect);
                }
                if caret_shown && !(is_primary && self.preedit.is_some()) {
                    text_painter.rect_filled(rect, 0.0, theme.caret);
                }
            }
            if let Some(gutter) = &gutter {
                let color = if is_caret_line {
                    theme.gutter_active
                } else {
                    theme.gutter
                };
                gutter.paint_number(&painter, top, line, color);
            }
        }

        if let (Some(preedit), Some(at)) = (&self.preedit, painted.primary_caret) {
            let galley = egui_ctx
                .fonts_mut(|fonts| fonts.layout_no_wrap(preedit.clone(), font.clone(), theme.text));
            caret::paint_preedit(
                &text_painter,
                at.min,
                galley,
                row_height,
                theme.background,
                theme.text,
            );
        }
        painted
    }
}

/// The decorations that may touch `start..=end`: those starting in it, plus the ones just before
/// it that reach into it (`decorations` sorted by start).
fn visible_decorations(decorations: &[Decoration], start: usize, end: usize) -> &[Decoration] {
    let mut low = decorations.partition_point(|d| d.start < start);
    while low > 0 && decorations[low - 1].end > start {
        low -= 1;
    }
    let high = decorations.partition_point(|d| d.start <= end).max(low);
    &decorations[low..high]
}

/// `pos` moved inside `rect` (half a point from its far edges).
fn clamp_to(pos: Pos2, rect: Rect) -> Pos2 {
    let far = (rect.max - Vec2::splat(0.5)).max(rect.min);
    pos2(
        pos.x.max(rect.min.x).min(far.x),
        pos.y.max(rect.min.y).min(far.y),
    )
}

/// The scroll delta of a drag held at `pos`: towards the side the pointer left the text area
/// by, at most [`AUTOSCROLL_MAX_ROWS`] rows per step; zero inside.
fn autoscroll_delta(geom: &Geometry, pos: Pos2) -> Vec2 {
    let visible = geom.visible;
    let text_left = visible.min.x + geom.gutter_width;
    let step = |outside: f32| outside.clamp(1.0, AUTOSCROLL_MAX_ROWS * geom.row_height);
    let mut delta = Vec2::ZERO;
    if pos.y < visible.min.y {
        delta.y = step(visible.min.y - pos.y);
    } else if pos.y > visible.max.y {
        delta.y = -step(pos.y - visible.max.y);
    }
    if pos.x < text_left {
        delta.x = step(text_left - pos.x);
    } else if pos.x > visible.max.x {
        delta.x = -step(pos.x - visible.max.x);
    }
    delta
}

/// `Ctrl` + wheel (or a pinch) over the editor scales egui's zoom factor, within
/// [`fonts::ZOOM_MIN`]..=[`fonts::ZOOM_MAX`] (§5.5).
fn zoom_with_wheel(ui: &Ui, response: &Response) {
    if !response.contains_pointer() {
        return;
    }
    let delta = ui.input(|i| i.zoom_delta());
    if !delta.is_finite() || (delta - 1.0).abs() <= f32::EPSILON {
        return;
    }
    let egui_ctx = ui.ctx();
    egui_ctx.set_zoom_factor(fonts::clamp_zoom(egui_ctx.zoom_factor() * delta));
}
