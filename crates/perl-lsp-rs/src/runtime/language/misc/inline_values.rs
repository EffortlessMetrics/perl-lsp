//! Inline value variable scanning support.

use std::sync::LazyLock;

static INLINE_VALUE_REGEX: LazyLock<Result<regex::Regex, regex::Error>> =
    LazyLock::new(|| regex::Regex::new(r"([$@%])([a-zA-Z_][a-zA-Z0-9_]*)"));

pub(super) fn inline_value_regex() -> Option<&'static regex::Regex> {
    INLINE_VALUE_REGEX.as_ref().ok()
}
