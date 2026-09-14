use ohmygrep_core::{CancelToken, OhMyGrepError, SearchMatch, SearchRequest, SearchResult};
use std::path::PathBuf;
use std::sync::OnceLock;

/// A small tree covering the inputs the sink special-cases: CRLF, NUL bytes,
/// a UTF-8 BOM, invalid UTF-8, long lines and ignore files.
pub fn corpus_dir() -> &'static str {
    static DIR: OnceLock<String> = OnceLock::new();
    DIR.get_or_init(|| {
        let dir: PathBuf =
            std::env::temp_dir().join(format!("ohmygrep-fuzz-{}", std::process::id()));
        let files: &[(&str, &[u8])] = &[
            ("plain.txt", b"alpha\nbeta HIT\ngamma\n\nHIT HIT\nend"),
            ("crlf.txt", b"one\r\nHIT two\r\nthree\r\n"),
            ("binary.dat", b"head HIT\n\x00\x01tail HIT\n"),
            ("bom.txt", b"\xEF\xBB\xBFHIT bom\nnext\n"),
            ("latin1.txt", b"caf\xE9 HIT\n\xFF\xFE\n"),
            (".gitignore", b"ignored.txt\n"),
            ("ignored.txt", b"HIT ignored\n"),
            ("sub/nested.rs", b"fn main() { /* HIT */ }\n"),
        ];
        for (name, content) in files {
            let path = dir.join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, content).unwrap();
        }
        std::fs::write(
            dir.join("long.txt"),
            [vec![b'x'; 70_000], b" HIT\n".to_vec()].concat(),
        )
        .unwrap();
        dir.to_string_lossy().into_owned()
    })
}

/// A panic caught inside the library is a bug even though it is reported as an error.
pub fn check(result: Result<SearchResult, OhMyGrepError>, request_max: Option<u32>) {
    match result {
        Err(OhMyGrepError::InternalPanic(message)) => panic!("internal panic: {message}"),
        Err(_) => {}
        Ok(res) => {
            if let Some(max) = request_max {
                assert!(
                    res.matches.len() <= max as usize,
                    "{} matches over limit {max}",
                    res.matches.len()
                );
            }
            res.matches.iter().for_each(check_match);
        }
    }
}

pub fn check_match(m: &SearchMatch) {
    for s in &m.submatches {
        assert!(s.start <= s.end, "{s:?}");
        // Offsets index the raw bytes; lossy decoding only makes `line` longer.
        assert!(
            (s.end as usize) <= m.line.len(),
            "{s:?} beyond {:?}",
            m.line
        );
    }
}

pub fn search(request: SearchRequest) -> Result<SearchResult, OhMyGrepError> {
    ohmygrep_core::search_blocking(request, CancelToken::new(Some(2_000)))
}
