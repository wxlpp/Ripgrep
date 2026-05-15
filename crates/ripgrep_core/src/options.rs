#[derive(Debug, Clone)]
pub struct SearchRequest {
    pub pattern: String,
    pub paths: Vec<String>,
    pub case_insensitive: bool,
    pub smart_case: bool,
    pub multiline: bool,
    pub include_globs: Vec<String>,
    pub exclude_globs: Vec<String>,
    pub file_types: Vec<String>,
    pub respect_gitignore: bool,
    pub include_hidden: bool,
    pub before_context: u32,
    pub after_context: u32,
    pub max_matches: Option<u32>,
    pub max_files: Option<u32>,
    pub max_file_size_bytes: Option<u64>,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Submatch {
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchMatch {
    pub path: String,
    pub line_number: u64,
    pub line: String,
    pub before_context: Vec<String>,
    pub after_context: Vec<String>,
    pub submatches: Vec<Submatch>,
}

#[derive(Debug, Clone, Default)]
pub struct SearchResult {
    pub matches: Vec<SearchMatch>,
    pub truncated: bool,
    pub cancelled: bool,
    pub files_searched: u64,
    #[allow(dead_code)]
    pub elapsed_ms: u64, // Exported to Swift via UniFFI
}
