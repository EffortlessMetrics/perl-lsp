use std::sync::Arc;

/// Specification for a pending heredoc.
#[derive(Clone)]
pub(crate) struct HeredocSpec {
    pub(crate) label: Arc<str>,
    pub(crate) body_start: usize,  // byte offset where the body begins
    pub(crate) allow_indent: bool, // true if we saw <<~ (Perl 5.26 indented heredocs)
}
