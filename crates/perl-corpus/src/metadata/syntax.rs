use regex::Regex;
use std::sync::LazyLock;

pub static SECTION_RE: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(?m)^=+\s*$").ok());
pub static META_RE: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r"(?m)^#\s*@(?P<k>id|tags|perl|flags):\s*(?P<v>.*)$").ok()
});

pub const OUTPUT_SEPARATOR: &str = "---";
