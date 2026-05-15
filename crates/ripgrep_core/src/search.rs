use crate::error::RipgrepError;
use crate::options::SearchRequest;
use grep_regex::{RegexMatcher, RegexMatcherBuilder};

pub fn build_matcher(req: &SearchRequest) -> Result<RegexMatcher, RipgrepError> {
    let mut b = RegexMatcherBuilder::new();
    b.case_insensitive(req.case_insensitive);
    b.case_smart(req.smart_case);
    b.multi_line(req.multiline);
    b.build(&req.pattern)
        .map_err(|e| RipgrepError::InvalidPattern(e.to_string()))
}
