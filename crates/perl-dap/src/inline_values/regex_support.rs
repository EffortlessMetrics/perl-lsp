use once_cell::sync::Lazy;
use regex::Regex;

/// Regex for matching Perl variables (scalars, arrays, hashes).
/// Stored as Option to avoid panics; if compilation fails, inline values are skipped.
pub(super) static PERL_VAR_RE: Lazy<Option<Regex>> = Lazy::new(|| {
    Regex::new(r"[$@%](?:::)?[A-Za-z_][A-Za-z0-9_]*(?:(?:::|')[A-Za-z_][A-Za-z0-9_]*)*").ok()
});

/// Regex for braced Perl variables (`${name}`, `@{Pkg::arr}`, `%{cfg}`).
pub(super) static BRACED_PERL_VAR_RE: Lazy<Option<Regex>> = Lazy::new(|| {
    Regex::new(r"([$@%])\{((?:::)?[A-Za-z_][A-Za-z0-9_]*(?:(?:::|')[A-Za-z_][A-Za-z0-9_]*)*)\}")
        .ok()
});

/// Legacy regex for scalar-only matching (used by `DapDispatcher`).
pub(super) static SCALAR_VAR_RE: Lazy<Option<Regex>> = Lazy::new(|| {
    Regex::new(r"\$(?:::)?[A-Za-z_][A-Za-z0-9_]*(?:(?:::|')[A-Za-z_][A-Za-z0-9_]*)*").ok()
});

/// Special Perl variables that should not be shown inline.
const SPECIAL_VARS: &[&str] = &[
    "$_", "$!", "$@", "$/", "$\\", "$,", "$;", "$\"", "$^", "$~", "$=", "$-", "$%", "$^W", "$^O",
    "$0", "$^T", "%ENV", "%SIG", "@ARGV", "@ISA", "@INC",
];

/// Check if a variable name is a special variable that should be excluded.
pub(super) fn is_special_variable_name(name: &str) -> bool {
    SPECIAL_VARS.contains(&name)
}
