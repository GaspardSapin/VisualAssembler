//! Selections and carets (m4-editor-shell.md §3.2).
//!
//! A [`Selections`] set is never empty, sorted by start, and holds no two overlapping
//! selections. Two non-empty selections merge when they overlap; an empty one (a caret) merges
//! with any selection it touches or lies in, as in VS Code. One selection is the primary one:
//! the last added caret (Alt+click, Ctrl+Alt+Up/Down, Ctrl+D). There is no box selection.

/// Most carets a document holds; extra selections are dropped.
pub const MAX_CARETS: usize = 10_000;

/// One selection: from `anchor` to `head` (the caret), in bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Selection {
    /// The fixed end.
    pub anchor: usize,
    /// The moving end: the caret.
    pub head: usize,
    /// Display column kept by vertical moves.
    pub goal: Option<u32>,
}

impl Selection {
    /// An empty selection (a caret) at `at`.
    pub fn caret(at: usize) -> Self {
        Self {
            anchor: at,
            head: at,
            goal: None,
        }
    }

    /// The selection from `anchor` to `head`, without a goal column.
    pub fn new(anchor: usize, head: usize) -> Self {
        Self {
            anchor,
            head,
            goal: None,
        }
    }

    /// The lower end of the selection.
    pub fn start(&self) -> usize {
        self.anchor.min(self.head)
    }

    /// The upper end of the selection.
    pub fn end(&self) -> usize {
        self.anchor.max(self.head)
    }

    /// Tells whether the selection holds no text.
    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }

    /// Tells whether the caret is before the anchor.
    pub fn is_reversed(&self) -> bool {
        self.head < self.anchor
    }

    /// The selection from `start` to `end` with the direction of `self`.
    pub(crate) fn with_range(&self, start: usize, end: usize) -> Self {
        let (anchor, head) = if self.is_reversed() {
            (end, start)
        } else {
            (start, end)
        };
        Self {
            anchor,
            head,
            goal: self.goal,
        }
    }
}

/// Tells whether `next` must merge into `previous` (`previous.start() <= next.start()`).
fn must_merge(previous: &Selection, next: &Selection) -> bool {
    if previous.is_empty() || next.is_empty() {
        next.start() <= previous.end()
    } else {
        next.start() < previous.end()
    }
}

/// The selections of a document: non-empty, sorted, non-overlapping, one of them primary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selections {
    /// The selections, sorted and non-overlapping, never empty.
    list: Vec<Selection>,
    /// Index of the primary selection in `list`.
    primary: usize,
}

impl Selections {
    /// A set holding only `s`.
    pub fn single(s: Selection) -> Self {
        Self {
            list: vec![s],
            primary: 0,
        }
    }

    /// Sorts, merges and caps `v` (at [`MAX_CARETS`]); `primary` indexes `v` (clamped). An empty
    /// `v` gives a caret at 0. A merged selection spans both parts and keeps the direction and
    /// goal of the primary one if it is involved, else of the first one; the primary one is
    /// never dropped by the cap.
    pub fn from_vec(v: Vec<Selection>, primary: usize) -> Self {
        let Some(last_index) = v.len().checked_sub(1) else {
            return Self::single(Selection::caret(0));
        };
        let primary = primary.min(last_index);
        let mut tagged: Vec<(Selection, bool)> = v
            .into_iter()
            .enumerate()
            .map(|(index, s)| (s, index == primary))
            .collect();
        tagged.sort_by_key(|(s, _)| (s.start(), s.end()));
        let mut list: Vec<Selection> = Vec::with_capacity(tagged.len());
        let mut primary_at: Option<usize> = None;
        for (s, is_primary) in tagged {
            let last_index = list.len().checked_sub(1);
            if let Some(last) = list.last_mut()
                && must_merge(last, &s)
            {
                let last_is_primary = primary_at.is_some() && primary_at == last_index;
                let keep = if is_primary && !last_is_primary {
                    s
                } else {
                    *last
                };
                let start = last.start();
                let end = last.end().max(s.end());
                *last = keep.with_range(start, end);
                if is_primary {
                    primary_at = last_index;
                }
                continue;
            }
            list.push(s);
            if is_primary {
                primary_at = Some(list.len() - 1);
            }
        }
        let mut primary_out = primary_at.unwrap_or(0);
        if list.len() > MAX_CARETS {
            if primary_out < MAX_CARETS {
                list.truncate(MAX_CARETS);
            } else {
                let kept = list[primary_out];
                list.truncate(MAX_CARETS - 1);
                list.push(kept);
                primary_out = MAX_CARETS - 1;
            }
        }
        Self {
            list,
            primary: primary_out,
        }
    }

    /// The selections, in document order.
    pub fn iter(&self) -> impl Iterator<Item = &Selection> {
        self.list.iter()
    }

    /// The selections as a slice, in document order.
    pub fn as_slice(&self) -> &[Selection] {
        &self.list
    }

    /// Number of selections, at least 1.
    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// Always false: a set holds at least one selection.
    pub fn is_empty(&self) -> bool {
        false
    }

    /// The primary selection.
    pub fn primary(&self) -> &Selection {
        &self.list[self.primary]
    }

    /// Index of the primary selection in document order.
    pub fn primary_index(&self) -> usize {
        self.primary
    }

    /// Adds `s`, merged with any selection it overlaps; it becomes the primary one.
    pub fn push(&mut self, s: Selection) {
        let mut list = std::mem::take(&mut self.list);
        list.push(s);
        let index = list.len() - 1;
        *self = Self::from_vec(list, index);
    }

    /// Keeps only the primary selection.
    pub fn collapse_to_primary(&mut self) {
        let kept = *self.primary();
        self.list = vec![kept];
        self.primary = 0;
    }

    /// Tells whether both sets hold the same anchors and heads, in order (goals ignored).
    pub(crate) fn same_positions(&self, other: &Self) -> bool {
        self.list.len() == other.list.len()
            && self
                .list
                .iter()
                .zip(&other.list)
                .all(|(a, b)| a.anchor == b.anchor && a.head == b.head)
    }

    /// Tells whether a selection equals or overlaps the range `start..end`.
    pub(crate) fn overlaps_range(&self, start: usize, end: usize) -> bool {
        let first = self.list.partition_point(|s| s.end() < start);
        self.list[first..]
            .iter()
            .take_while(|s| s.start() <= end)
            .any(|s| (s.start() == start && s.end() == end) || (s.start() < end && s.end() > start))
    }
}
