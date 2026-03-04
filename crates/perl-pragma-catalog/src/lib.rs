//! Canonical Perl pragma inventories and lookup helpers.

/// Canonical list of well-known Perl pragma modules.
pub const PRAGMA_MODULES: &[&str] = &[
    "attributes",
    "autodie",
    "autouse",
    "base",
    "bigint",
    "bignum",
    "bigrat",
    "blib",
    "bytes",
    "charnames",
    "constant",
    "diagnostics",
    "encoding",
    "feature",
    "fields",
    "filetest",
    "if",
    "integer",
    "lib",
    "less",
    "locale",
    "open",
    "ops",
    "overload",
    "parent",
    "re",
    "sigtrap",
    "sort",
    "strict",
    "subs",
    "threads",
    "utf8",
    "vars",
    "vmsish",
    "warnings",
];

/// Returns `true` when `module` matches a well-known Perl pragma.
#[must_use]
pub fn is_pragma_module(module: &str) -> bool {
    PRAGMA_MODULES.contains(&module)
}
