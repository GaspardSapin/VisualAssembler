//! Tests of `va_editor::selection`: normalization, merging, the primary selection and the
//! `MAX_CARETS` cap (m4-editor-shell.md §3.2).
#![allow(clippy::unwrap_used)]

use va_editor::selection::{MAX_CARETS, Selection, Selections};

/// The `(anchor, head)` pairs of `s`, in order.
fn pairs(s: &Selections) -> Vec<(usize, usize)> {
    s.iter().map(|sel| (sel.anchor, sel.head)).collect()
}

#[test]
fn selection_ends_and_emptiness() {
    let caret = Selection::caret(4);
    assert_eq!((caret.start(), caret.end(), caret.is_empty()), (4, 4, true));
    assert_eq!(caret.goal, None);
    let reversed = Selection::new(9, 2);
    assert_eq!((reversed.start(), reversed.end()), (2, 9));
    assert!(reversed.is_reversed() && !reversed.is_empty());
}

#[test]
fn single_holds_one_primary_selection() {
    let s = Selections::single(Selection::new(1, 3));
    assert_eq!(s.len(), 1);
    assert!(!s.is_empty());
    assert_eq!(*s.primary(), Selection::new(1, 3));
    assert_eq!(s.primary_index(), 0);
}

#[test]
fn from_vec_sorts_and_tracks_the_primary() {
    let s = Selections::from_vec(
        vec![
            Selection::caret(9),
            Selection::caret(1),
            Selection::new(5, 3),
        ],
        0,
    );
    assert_eq!(pairs(&s), vec![(1, 1), (5, 3), (9, 9)]);
    assert_eq!(s.primary_index(), 2);
    assert_eq!(s.primary().head, 9);
    assert_eq!(s.as_slice().len(), 3);
    // An out-of-range primary is clamped to the last element of the input.
    let s = Selections::from_vec(vec![Selection::caret(7), Selection::caret(2)], 99);
    assert_eq!(s.primary().head, 2);
    // An empty input gives a caret at 0.
    let s = Selections::from_vec(Vec::new(), 0);
    assert_eq!(pairs(&s), vec![(0, 0)]);
}

#[test]
fn overlapping_selections_merge_touching_ones_do_not() {
    // Strict overlap of non-empty selections merges.
    let s = Selections::from_vec(vec![Selection::new(0, 4), Selection::new(2, 6)], 0);
    assert_eq!(pairs(&s), vec![(0, 6)]);
    // Touching non-empty selections stay apart.
    let s = Selections::from_vec(vec![Selection::new(0, 4), Selection::new(4, 6)], 0);
    assert_eq!(pairs(&s), vec![(0, 4), (4, 6)]);
    // Identical selections merge.
    let s = Selections::from_vec(vec![Selection::new(1, 3), Selection::new(1, 3)], 1);
    assert_eq!(pairs(&s), vec![(1, 3)]);
}

#[test]
fn a_caret_merges_with_what_it_touches_or_lies_in() {
    let s = Selections::from_vec(vec![Selection::caret(3), Selection::caret(3)], 0);
    assert_eq!(pairs(&s), vec![(3, 3)]);
    let s = Selections::from_vec(vec![Selection::new(2, 6), Selection::caret(4)], 0);
    assert_eq!(pairs(&s), vec![(2, 6)]);
    let s = Selections::from_vec(vec![Selection::new(2, 6), Selection::caret(6)], 0);
    assert_eq!(pairs(&s), vec![(2, 6)]);
    let s = Selections::from_vec(vec![Selection::caret(2), Selection::new(2, 6)], 0);
    assert_eq!(pairs(&s), vec![(2, 6)]);
    let s = Selections::from_vec(vec![Selection::caret(1), Selection::new(2, 6)], 0);
    assert_eq!(pairs(&s), vec![(1, 1), (2, 6)]);
}

#[test]
fn a_merge_keeps_the_direction_of_the_primary() {
    // The primary (reversed) one is merged into a forward one: the result is reversed.
    let s = Selections::from_vec(vec![Selection::new(0, 4), Selection::new(6, 2)], 1);
    assert_eq!(pairs(&s), vec![(6, 0)]);
    assert_eq!(s.primary_index(), 0);
    // Without the primary involved, the first one's direction is kept.
    let s = Selections::from_vec(
        vec![
            Selection::new(4, 0),
            Selection::new(2, 6),
            Selection::caret(9),
        ],
        2,
    );
    assert_eq!(pairs(&s), vec![(6, 0), (9, 9)]);
    assert_eq!(s.primary_index(), 1);
}

#[test]
fn push_merges_and_makes_the_new_selection_primary() {
    let mut s = Selections::single(Selection::caret(1));
    s.push(Selection::caret(8));
    s.push(Selection::caret(4));
    assert_eq!(pairs(&s), vec![(1, 1), (4, 4), (8, 8)]);
    assert_eq!(s.primary().head, 4);
    assert_eq!(s.primary_index(), 1);
    // Pushing onto an existing caret merges; the merged one is primary.
    s.push(Selection::new(7, 9));
    assert_eq!(pairs(&s), vec![(1, 1), (4, 4), (7, 9)]);
    assert_eq!(s.primary_index(), 2);
}

#[test]
fn collapse_keeps_only_the_primary_selection() {
    let mut s = Selections::from_vec(
        vec![
            Selection::caret(1),
            Selection::new(3, 5),
            Selection::caret(9),
        ],
        1,
    );
    s.collapse_to_primary();
    assert_eq!(pairs(&s), vec![(3, 5)]);
    assert_eq!(s.primary_index(), 0);
}

#[test]
fn the_cap_drops_extra_selections_but_never_the_primary() {
    let many: Vec<Selection> = (0..MAX_CARETS + 50)
        .map(|i| Selection::caret(i * 2))
        .collect();
    let s = Selections::from_vec(many.clone(), 3);
    assert_eq!(s.len(), MAX_CARETS);
    assert_eq!(s.primary().head, 6);
    let last = many.len() - 1;
    let s = Selections::from_vec(many, last);
    assert_eq!(s.len(), MAX_CARETS);
    assert_eq!(s.primary().head, last * 2);
    assert_eq!(s.primary_index(), MAX_CARETS - 1);
    let heads: Vec<usize> = s.iter().map(|sel| sel.head).collect();
    assert!(heads.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn merged_sets_are_sorted_and_disjoint() {
    let mut seed = 0x1234_5678_u64;
    let mut next = |bound: usize| {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        usize::try_from((seed >> 33) % u64::try_from(bound).unwrap()).unwrap()
    };
    for _ in 0..2_000 {
        let count = 1 + next(8);
        let list: Vec<Selection> = (0..count)
            .map(|_| Selection::new(next(30), next(30)))
            .collect();
        let primary = next(count);
        let wanted = list[primary];
        let s = Selections::from_vec(list, primary);
        let all: Vec<&Selection> = s.iter().collect();
        for pair in all.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            assert!(a.start() <= b.start());
            if a.is_empty() || b.is_empty() {
                assert!(b.start() > a.end(), "{a:?} {b:?}");
            } else {
                assert!(b.start() >= a.end(), "{a:?} {b:?}");
            }
        }
        let primary = s.primary();
        assert!(primary.start() <= wanted.start() && primary.end() >= wanted.end());
    }
}
