use super::cancel::CancelToken;
use super::search::search_blocking;

/// Reproduce the per-match after_context attribution bug:
/// When two matches in the same file are close enough that grep_searcher
/// merges their context windows (no context_break between them),
/// the FIRST match must still receive the after-context lines that fall
/// within its own -A window.
///
/// Fixture `tests/fixtures/adjacent/a.txt`:
///   line 1: pre
///   line 2: MATCH one
///   line 3: MATCH two
///   line 4: tail1
///   line 5: tail2
///   line 6: tail3
///
/// With `after_context = 2` and pattern `MATCH`:
///   match @ line 2 → after_context should be ["tail1"]
///     (line 3 "MATCH two" is a match event, not an after-context line;
///      line 4 "tail1" is within the -A 2 window: 2 < 4 <= 2+2)
///   match @ line 3 → after_context should be ["tail1", "tail2"]
///     (lines 4 "tail1" and 5 "tail2" are within the -A 2 window: 3 < 4,5 <= 3+2)
#[test]
fn adjacent_matches_first_match_has_correct_after_context() {
    let mut r = super::search_e2e_tests::req("MATCH");
    r.paths = vec!["tests/fixtures/adjacent/a.txt".into()];
    r.after_context = 2;
    let res = search_blocking(r, CancelToken::new(None)).unwrap();

    assert_eq!(res.matches.len(), 2, "expected exactly 2 matches");

    let m1 = &res.matches[0];
    let m2 = &res.matches[1];

    assert_eq!(m1.line_number, 2, "first match should be on line 2");
    assert_eq!(m2.line_number, 3, "second match should be on line 3");

    // KEY assertion: first match's after_context must be non-empty
    // (the bug causes it to be empty).
    // grep_searcher emits each matching line via matched(), not as after-context,
    // so "MATCH two" (line 3) is NOT included in m1's after_context — only
    // non-matching lines within the -A window appear as after-context.
    // With -A 2: match@line2's window covers lines 3,4; line3 is a match (emitted
    // separately), line4 "tail1" is the after-context line for m1.
    assert!(
        !m1.after_context.is_empty(),
        "BUG: first match's after_context is empty; should contain 'tail1' from its -A window"
    );
    // line 5 (tail2) is outside m1's window: m1@2, after=2 → 2 < L <= 4, so 5 excluded
    assert_eq!(
        m1.after_context,
        vec!["tail1".to_string()],
        "first match should have 'tail1' (line 4) in its after-context"
    );
    assert_eq!(
        m2.after_context,
        vec!["tail1".to_string(), "tail2".to_string()],
        "second match should have 'tail1','tail2' (lines 4,5) in its after-context"
    );
}
