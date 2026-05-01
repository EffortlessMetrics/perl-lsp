use regex::Regex;
use std::sync::LazyLock;

pub(crate) static SEC_RE: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(?m)^=+\s*$").ok());
pub(crate) static META_RE: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(?m)^#\s*@(?P<k>id|tags|perl|flags):\s*(?P<v>.*)$").ok());

pub(crate) const EXPECTED_SEPARATOR: &str = "---";
