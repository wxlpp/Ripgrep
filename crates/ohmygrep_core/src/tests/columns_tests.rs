use super::columns::{cut_match, cut_prefix, CutLine};
use super::options::Submatch;

fn sub(start: u32, end: u32) -> Submatch {
    Submatch { start, end }
}

#[test]
fn short_line_is_untouched() {
    let cut = cut_match(b"abc HIT", vec![sub(4, 7)], Some(10), false);
    assert_eq!(
        cut,
        CutLine {
            line: "abc HIT".into(),
            offset: 0,
            truncated: false,
            submatches: vec![sub(4, 7)]
        }
    );
}

#[test]
fn window_keeps_match_in_middle_of_long_line() {
    let mut content = vec![b'a'; 500_000];
    content.extend_from_slice(b"NEEDLE");
    content.extend(vec![b'b'; 500_000]);
    let cut = cut_match(&content, vec![sub(500_000, 500_006)], Some(4096), false);
    assert!(cut.truncated);
    assert_eq!(cut.offset, 500_000 - 1024);
    assert_eq!(cut.line.len(), 4096);
    assert_eq!(cut.submatches, vec![sub(1024, 1030)]);
    assert_eq!(&cut.line[1024..1030], "NEEDLE");
}

#[test]
fn window_near_end_is_clamped_to_fit() {
    let mut content = vec![b'a'; 1000];
    content.extend_from_slice(b"HIT");
    let cut = cut_match(&content, vec![sub(1000, 1003)], Some(100), false);
    assert_eq!(cut.offset, 903);
    assert!(cut.line.ends_with("HIT"));
    assert_eq!(cut.submatches, vec![sub(97, 100)]);
}

#[test]
fn window_edges_align_to_utf8_characters() {
    let content = "é".repeat(3000); // 2 bytes each
    let cut = cut_match(content.as_bytes(), vec![sub(3001, 3003)], Some(101), false);
    assert!(
        !cut.line.contains('\u{FFFD}'),
        "cut inside a character: {:?}",
        &cut.line[..8]
    );
    assert!(cut.line.len() <= 101);
    assert_eq!(cut.offset % 2, 0);
}

#[test]
fn invalid_utf8_runs_bound_the_backoff() {
    let content = vec![0x80u8; 100];
    let cut = cut_match(&content, vec![sub(50, 51)], Some(4), false);
    assert!(cut.truncated);
    assert!(cut.line.chars().count() <= 4, "{:?}", cut.line);
    let prefix = cut_prefix(&content, Some(10));
    assert!(!prefix.is_empty());
}

#[test]
fn multiline_rows_are_cut_separately() {
    let content = format!("short\n{}\nend", "x".repeat(100));
    let subs = vec![sub(0, 5), sub(6, 60), sub(107, 110)];
    let cut = cut_match(content.as_bytes(), subs, Some(10), true);
    assert!(cut.truncated);
    assert_eq!(cut.line, format!("short\n{}\nend", "x".repeat(10)));
    // Rows before the first cut keep their offsets; the one crossing it is clamped.
    assert_eq!(cut.submatches, vec![sub(0, 5), sub(6, 16)]);
}

#[test]
fn context_prefix_cut() {
    assert_eq!(cut_prefix(b"abcdef", Some(3)), "abc");
    assert_eq!(cut_prefix(b"abc", None), "abc");
}
