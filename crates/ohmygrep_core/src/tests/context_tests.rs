use super::cancel::CancelToken;
use super::search::search_blocking;
use super::support::{req, TempDir};

// Expected values below match ripgrep 15.1 output for the same inputs.
const CTX: &[u8] = b"a\nb\nc\nd\nHIT one\ne\nHIT two\nf\n";
const GAP: &[u8] = b"l1\nHIT a\nl3\nl4\nl5\nl6\nHIT b\nl8\n";

type Row = (u64, Vec<&'static str>, Vec<&'static str>);

fn run(content: &[u8], before: u32, after: u32) -> Vec<(u64, Vec<String>, Vec<String>)> {
    let dir = TempDir::new("ctx");
    let file = dir.write("f.txt", content);
    let mut r = req("HIT", &file);
    r.before_context = before;
    r.after_context = after;
    search_blocking(r, CancelToken::new(None))
        .unwrap()
        .matches
        .into_iter()
        .map(|m| (m.line_number, m.before_context, m.after_context))
        .collect()
}

fn rows(expected: &[Row]) -> Vec<(u64, Vec<String>, Vec<String>)> {
    expected
        .iter()
        .map(|(l, b, a)| {
            (
                *l,
                b.iter().map(|s| s.to_string()).collect(),
                a.iter().map(|s| s.to_string()).collect(),
            )
        })
        .collect()
}

#[test]
fn before_context_stops_at_previous_match() {
    assert_eq!(
        run(CTX, 2, 0),
        rows(&[(5, vec!["c", "d"], vec![]), (7, vec!["e"], vec![])])
    );
}

#[test]
fn symmetric_context_does_not_duplicate_lines() {
    assert_eq!(
        run(CTX, 1, 1),
        rows(&[(5, vec!["d"], vec!["e"]), (7, vec![], vec!["f"])])
    );
}

#[test]
fn after_context_stops_at_next_match() {
    assert_eq!(
        run(CTX, 0, 3),
        rows(&[(5, vec![], vec!["e"]), (7, vec![], vec!["f"])])
    );
}

#[test]
fn adjacent_matches_leave_first_after_context_empty() {
    assert_eq!(
        run(b"pre\nHIT one\nHIT two\ntail1\ntail2\ntail3\n", 0, 2),
        rows(&[(2, vec![], vec![]), (3, vec![], vec!["tail1", "tail2"])])
    );
}

#[test]
fn touching_groups_split_lines_between_matches() {
    assert_eq!(
        run(GAP, 2, 2),
        rows(&[
            (2, vec!["l1"], vec!["l3", "l4"]),
            (7, vec!["l5", "l6"], vec!["l8"])
        ])
    );
}

#[test]
fn separated_groups_keep_own_context() {
    assert_eq!(
        run(GAP, 1, 1),
        rows(&[(2, vec!["l1"], vec!["l3"]), (7, vec!["l6"], vec!["l8"])])
    );
}

#[test]
fn multiline_pattern_matches_across_lines() {
    let dir = TempDir::new("ml");
    let file = dir.write("f.txt", b"line 1\nmatch\nline 3\nmatch\n");
    let mut r = req(r"match[\s\S]*match", &file);
    r.multiline = true;
    let res = search_blocking(r, CancelToken::new(None)).unwrap();
    assert_eq!(res.matches.len(), 1);
    assert_eq!(res.matches[0].line_number, 2);
}
