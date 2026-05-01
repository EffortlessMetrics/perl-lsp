use regex::Regex;
use std::sync::LazyLock;

pub static SECTION_DELIMITER_RE: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(?m)^=+\s*$").ok());
pub static METADATA_RE: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(?m)^#\s*@(?P<k>id|tags|perl|flags):\s*(?P<v>.*)$").ok());

pub const EXPECTED_OUTPUT_SEPARATOR: &str = "---";
