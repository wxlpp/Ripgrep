#[derive(Debug, Clone, Default, uniffi::Record)]
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
    /// Apply `.gitignore` rules only inside a git repository (rg's default).
    pub require_git: bool,
    pub include_hidden: bool,
    pub before_context: u32,
    pub after_context: u32,
    pub max_matches: Option<u32>,
    pub max_files: Option<u32>,
    pub max_file_size_bytes: Option<u64>,
    pub search_binary: bool,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Submatch {
    // Byte offsets within one match; u32 is enough for any realistic line.
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct SearchMatch {
    pub path: String,
    pub line_number: u64,
    pub line: String,
    pub before_context: Vec<String>,
    pub after_context: Vec<String>,
    pub submatches: Vec<Submatch>,
}

/// A per-path problem that did not stop the search (unreadable file, binary file, ...).
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct SearchWarning {
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, uniffi::Record)]
pub struct SearchResult {
    pub matches: Vec<SearchMatch>,
    pub truncated: bool,
    pub cancelled: bool,
    pub files_searched: u64,
    pub elapsed_ms: u64,
    pub warnings: Vec<SearchWarning>,
}
