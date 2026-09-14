use crate::options::Submatch;

/// Continuation bytes skipped at most when aligning a cut to a character start;
/// bounded so invalid UTF-8 cannot move the cut arbitrarily.
const MAX_UTF8_BACKOFF: usize = 3;

fn is_continuation(b: u8) -> bool {
    b & 0xC0 == 0x80
}

fn forward_to_char_start(bytes: &[u8], mut i: usize) -> usize {
    let limit = (i + MAX_UTF8_BACKOFF).min(bytes.len());
    while i < limit && is_continuation(bytes[i]) {
        i += 1;
    }
    i
}

fn back_to_char_start(bytes: &[u8], mut i: usize) -> usize {
    let limit = i.saturating_sub(MAX_UTF8_BACKOFF);
    while i > limit && i < bytes.len() && is_continuation(bytes[i]) {
        i -= 1;
    }
    i
}

/// A line as returned to callers after applying `max_columns`.
#[derive(Debug, PartialEq)]
pub struct CutLine {
    pub line: String,
    pub offset: u32,
    pub truncated: bool,
    pub submatches: Vec<Submatch>,
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Prefix of at most `max` bytes, for context lines.
pub fn cut_prefix(content: &[u8], max: Option<usize>) -> String {
    match max {
        Some(max) if content.len() > max => lossy(&content[..back_to_char_start(content, max)]),
        _ => lossy(content),
    }
}

/// Applies `max_columns` to a match line. Single lines keep a window around the
/// first submatch; multiline matches cut each row to a prefix so rows stay aligned
/// with line numbers.
pub fn cut_match(
    content: &[u8],
    submatches: Vec<Submatch>,
    max: Option<usize>,
    multi_line: bool,
) -> CutLine {
    let Some(max) = max.filter(|&m| content.len() > m) else {
        return CutLine {
            line: lossy(content),
            offset: 0,
            truncated: false,
            submatches,
        };
    };
    if multi_line && content.contains(&b'\n') {
        cut_rows(content, submatches, max)
    } else {
        cut_window(content, submatches, max)
    }
}

fn cut_window(content: &[u8], submatches: Vec<Submatch>, max: usize) -> CutLine {
    let anchor = submatches.first().map_or(0, |s| s.start as usize);
    let start = forward_to_char_start(
        content,
        anchor.saturating_sub(max / 4).min(content.len() - max),
    );
    let end = back_to_char_start(content, (start + max).min(content.len())).max(start);
    let submatches = submatches
        .into_iter()
        .filter(|s| {
            let (s_start, s_end) = (s.start as usize, s.end as usize);
            (s_start < end && s_end > start)
                || (s_start == s_end
                    && (s_start == start || (s_start == end && end == content.len())))
        })
        .map(|s| Submatch {
            start: ((s.start as usize).max(start) - start) as u32,
            end: ((s.end as usize).min(end) - start) as u32,
        })
        .collect();
    CutLine {
        line: lossy(&content[start..end]),
        offset: start as u32,
        truncated: true,
        submatches,
    }
}

/// Offset in `content` where `cut_rows` makes its first cut, if any row exceeds `max`.
pub fn first_row_cut(content: &[u8], max: usize) -> Option<usize> {
    let mut row_start = 0;
    for row in content.split(|&b| b == b'\n') {
        if row.len() > max {
            return Some(row_start + back_to_char_start(row, max));
        }
        row_start += row.len() + 1;
    }
    None
}

fn cut_rows(content: &[u8], submatches: Vec<Submatch>, max: usize) -> CutLine {
    let mut out = Vec::with_capacity(content.len().min(max * 4));
    let mut first_cut: Option<usize> = None;
    let mut row_start = 0;
    for (i, row) in content.split(|&b| b == b'\n').enumerate() {
        if i > 0 {
            out.push(b'\n');
        }
        if row.len() > max {
            let keep = back_to_char_start(row, max);
            first_cut.get_or_insert(row_start + keep);
            out.extend_from_slice(&row[..keep]);
        } else {
            out.extend_from_slice(row);
        }
        row_start += row.len() + 1;
    }
    let Some(cut) = first_cut else {
        return CutLine {
            line: lossy(content),
            offset: 0,
            truncated: false,
            submatches,
        };
    };
    // Bytes before the first cut are unchanged, so offsets there stay valid.
    let submatches = submatches
        .into_iter()
        .filter(|s| (s.start as usize) < cut)
        .map(|s| Submatch {
            start: s.start,
            end: s.end.min(cut as u32),
        })
        .collect();
    CutLine {
        line: lossy(&out),
        offset: 0,
        truncated: true,
        submatches,
    }
}
