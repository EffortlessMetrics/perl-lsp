//! Canonical Perl keyword inventories and allocation-free lookup helpers.
//!
//! Exports a single [`KEYWORDS`] constant containing the full set of Perl
//! reserved words, pragmas, and special tokens. The list is used by the lexer,
//! completion provider, and semantic-token highlighter to distinguish keywords
//! from user-defined identifiers.

/// Canonical union of keyword inventories used by the workspace.
pub const KEYWORDS: &[&str] = &[
    "AUTOLOAD",
    "BEGIN",
    "CHECK",
    "DESTROY",
    "END",
    "INIT",
    "UNITCHECK",
    "__FILE__",
    "__LINE__",
    "__PACKAGE__",
    "__SUB__",
    "abs",
    "and",
    "async",
    "await",
    "bless",
    "blessed",
    "break",
    "catch",
    "chomp",
    "chop",
    "chr",
    "class",
    "close",
    "cmp",
    "continue",
    "default",
    "defer",
    "defined",
    "delete",
    "die",
    "do",
    "each",
    "else",
    "elsif",
    "eq",
    "eval",
    "exists",
    "exit",
    "field",
    "finally",
    "for",
    "foreach",
    "format",
    "ge",
    "given",
    "goto",
    "grep",
    "gt",
    "hex",
    "if",
    "index",
    "int",
    "join",
    "keys",
    "last",
    "lc",
    "lcfirst",
    "le",
    "length",
    "local",
    "lt",
    "m",
    "map",
    "method",
    "my",
    "ne",
    "next",
    "no",
    "not",
    "oct",
    "open",
    "or",
    "ord",
    "our",
    "pack",
    "package",
    "pop",
    "print",
    "printf",
    "push",
    "q",
    "qq",
    "qr",
    "qw",
    "qx",
    "read",
    "redo",
    "ref",
    "require",
    "return",
    "reverse",
    "rindex",
    "s",
    "say",
    "scalar",
    "shift",
    "sort",
    "splice",
    "split",
    "sprintf",
    "sqrt",
    "state",
    "sub",
    "substr",
    "tie",
    "tr",
    "try",
    "uc",
    "ucfirst",
    "undef",
    "unless",
    "unpack",
    "unshift",
    "untie",
    "until",
    "use",
    "values",
    "wantarray",
    "warn",
    "when",
    "while",
    "write",
    "xor",
    "y",
];

/// Keywords used by `perl-lsp-completion` keyword completion.
pub const LSP_COMPLETION_KEYWORDS: &[&str] = &[
    "AUTOLOAD",
    "BEGIN",
    "CHECK",
    "DESTROY",
    "END",
    "INIT",
    "UNITCHECK",
    "__FILE__",
    "__LINE__",
    "__PACKAGE__",
    "__SUB__",
    "and",
    "async",
    "await",
    "blessed",
    "catch",
    "class",
    "cmp",
    "default",
    "defer",
    "defined",
    "die",
    "do",
    "else",
    "elsif",
    "eq",
    "eval",
    "exit",
    "field",
    "finally",
    "for",
    "foreach",
    "ge",
    "given",
    "goto",
    "gt",
    "if",
    "last",
    "le",
    "local",
    "lt",
    "method",
    "my",
    "ne",
    "next",
    "not",
    "or",
    "our",
    "package",
    "redo",
    "ref",
    "require",
    "return",
    "scalar",
    "state",
    "sub",
    "try",
    "undef",
    "unless",
    "until",
    "use",
    "wantarray",
    "warn",
    "when",
    "while",
    "xor",
];

/// Keywords used by DAP debug-console completions.
pub const DAP_COMPLETION_KEYWORDS: &[&str] = &[
    "abs", "bless", "chomp", "chop", "chr", "close", "defined", "delete", "die", "do", "each",
    "else", "elsif", "eval", "exists", "for", "foreach", "grep", "hex", "if", "index", "int",
    "join", "keys", "last", "lc", "lcfirst", "length", "local", "map", "my", "next", "oct", "open",
    "ord", "our", "pack", "package", "pop", "print", "printf", "push", "qw", "redo", "ref",
    "require", "return", "reverse", "rindex", "say", "scalar", "shift", "sort", "splice", "split",
    "sprintf", "sqrt", "sub", "substr", "tie", "uc", "ucfirst", "unless", "unpack", "unshift",
    "untie", "until", "use", "values", "warn", "while",
];

/// Keywords used by runtime fallback completion in `perl-lsp`.
pub const LSP_RUNTIME_COMPLETION_KEYWORDS: &[&str] = &[
    "close", "default", "die", "else", "elsif", "for", "foreach", "given", "goto", "grep", "if",
    "last", "local", "map", "my", "next", "open", "our", "package", "pop", "print", "push", "read",
    "redo", "require", "return", "say", "shift", "sort", "splice", "state", "sub", "unless",
    "unshift", "until", "use", "warn", "when", "while", "write",
];

/// Keywords reserved for LSP rename validation.
pub const RENAME_KEYWORDS: &[&str] = &[
    "and", "else", "elsif", "eq", "for", "foreach", "if", "last", "local", "my", "ne", "next",
    "not", "or", "our", "package", "redo", "require", "return", "state", "sub", "unless", "until",
    "use", "while",
];

/// Keywords used by parser LSP-compat completion/rename paths.
pub const PARSER_LSP_KEYWORDS: &[&str] = &[
    "async", "await", "break", "continue", "default", "die", "do", "else", "elsif", "eval", "for",
    "foreach", "given", "goto", "if", "last", "local", "my", "next", "no", "our", "package",
    "redo", "require", "return", "sub", "unless", "until", "use", "warn", "when", "while",
];

/// Keywords recognized by `perl-lexer` for token classification.
pub const LEXER_KEYWORDS: &[&str] = &[
    "BEGIN",
    "CHECK",
    "END",
    "INIT",
    "UNITCHECK",
    "and",
    "await",
    "break",
    "catch",
    "class",
    "cmp",
    "continue",
    "default",
    "defer",
    "die",
    "do",
    "else",
    "elsif",
    "eval",
    "field",
    "finally",
    "for",
    "foreach",
    "format",
    "given",
    "goto",
    "grep",
    "if",
    "last",
    "local",
    "m",
    "map",
    "method",
    "my",
    "next",
    "not",
    "or",
    "our",
    "package",
    "print",
    "q",
    "qq",
    "qr",
    "qw",
    "qx",
    "redo",
    "require",
    "return",
    "s",
    "say",
    "sort",
    "split",
    "state",
    "sub",
    "tr",
    "try",
    "undef",
    "unless",
    "until",
    "use",
    "warn",
    "when",
    "while",
    "xor",
    "y",
];

/// Return `true` when `token` exists in the canonical keyword inventory.
#[must_use]
pub fn is_keyword(token: &str) -> bool {
    KEYWORDS.binary_search(&token).is_ok()
}

/// Return `true` when `token` is recognized as a lexer keyword.
#[must_use]
pub fn is_lexer_keyword(token: &str) -> bool {
    LEXER_KEYWORDS.binary_search(&token).is_ok()
}

/// Return `true` when `token` exists in the LSP completion keyword bucket.
#[must_use]
pub fn is_lsp_completion_keyword(token: &str) -> bool {
    LSP_COMPLETION_KEYWORDS.binary_search(&token).is_ok()
}

/// Return `true` when `token` exists in the DAP completion keyword bucket.
#[must_use]
pub fn is_dap_completion_keyword(token: &str) -> bool {
    DAP_COMPLETION_KEYWORDS.binary_search(&token).is_ok()
}

/// Return `true` when `token` exists in the runtime completion keyword bucket.
#[must_use]
pub fn is_lsp_runtime_completion_keyword(token: &str) -> bool {
    LSP_RUNTIME_COMPLETION_KEYWORDS.binary_search(&token).is_ok()
}

/// Return `true` when `token` is reserved in rename validation paths.
#[must_use]
pub fn is_rename_keyword(token: &str) -> bool {
    RENAME_KEYWORDS.binary_search(&token).is_ok()
}

/// Return `true` when `token` exists in parser LSP-compat keyword bucket.
#[must_use]
pub fn is_parser_lsp_keyword(token: &str) -> bool {
    PARSER_LSP_KEYWORDS.binary_search(&token).is_ok()
}

#[cfg(test)]
mod tests {
    use super::{
        DAP_COMPLETION_KEYWORDS, KEYWORDS, LEXER_KEYWORDS, LSP_COMPLETION_KEYWORDS,
        LSP_RUNTIME_COMPLETION_KEYWORDS, PARSER_LSP_KEYWORDS, RENAME_KEYWORDS,
        is_dap_completion_keyword, is_keyword, is_lexer_keyword, is_lsp_completion_keyword,
        is_lsp_runtime_completion_keyword, is_parser_lsp_keyword, is_rename_keyword,
    };

    fn assert_sorted_unique(name: &str, items: &[&str]) {
        let mut last = "";
        for &item in items {
            assert!(item > last, "{name} must be sorted + unique: {item} after {last}");
            last = item;
        }
    }

    #[test]
    fn keyword_lists_are_sorted_and_unique() {
        assert_sorted_unique("KEYWORDS", KEYWORDS);
        assert_sorted_unique("LSP_COMPLETION_KEYWORDS", LSP_COMPLETION_KEYWORDS);
        assert_sorted_unique("DAP_COMPLETION_KEYWORDS", DAP_COMPLETION_KEYWORDS);
        assert_sorted_unique("LSP_RUNTIME_COMPLETION_KEYWORDS", LSP_RUNTIME_COMPLETION_KEYWORDS);
        assert_sorted_unique("RENAME_KEYWORDS", RENAME_KEYWORDS);
        assert_sorted_unique("PARSER_LSP_KEYWORDS", PARSER_LSP_KEYWORDS);
        assert_sorted_unique("LEXER_KEYWORDS", LEXER_KEYWORDS);
    }

    #[test]
    fn known_keywords_are_present() {
        assert!(is_keyword("my"));
        assert!(is_keyword("foreach"));
        assert!(is_keyword("print"));
        assert!(is_keyword("__PACKAGE__"));
        assert!(is_keyword("__SUB__"));
        assert!(!is_keyword("definitely_not_a_perl_keyword"));
    }

    #[test]
    fn lsp_completion_includes_modern_perl_keywords() {
        for keyword in [
            "catch", "class", "default", "defer", "field", "finally", "given", "method", "try",
            "when",
        ] {
            assert!(is_lsp_completion_keyword(keyword), "missing {keyword} in LSP completion");
        }
    }

    #[test]
    fn lookup_helpers_match_bucket_membership() {
        for &item in LSP_COMPLETION_KEYWORDS {
            assert!(is_lsp_completion_keyword(item));
            assert!(is_keyword(item));
        }
        for &item in DAP_COMPLETION_KEYWORDS {
            assert!(is_dap_completion_keyword(item));
            assert!(is_keyword(item));
        }
        for &item in LSP_RUNTIME_COMPLETION_KEYWORDS {
            assert!(is_lsp_runtime_completion_keyword(item));
            assert!(is_keyword(item));
        }
        for &item in RENAME_KEYWORDS {
            assert!(is_rename_keyword(item));
            assert!(is_keyword(item));
        }
        for &item in PARSER_LSP_KEYWORDS {
            assert!(is_parser_lsp_keyword(item));
            assert!(is_keyword(item));
        }
        for &item in LEXER_KEYWORDS {
            assert!(is_lexer_keyword(item));
            assert!(is_keyword(item));
        }
    }

    #[test]
    fn lookup_helpers_reject_unknown_tokens() {
        assert!(!is_lsp_completion_keyword("print"));
        assert!(!is_dap_completion_keyword("AUTOLOAD"));
        assert!(!is_lsp_runtime_completion_keyword("AUTOLOAD"));
        assert!(!is_rename_keyword("print"));
        assert!(!is_parser_lsp_keyword("AUTOLOAD"));
        assert!(!is_lexer_keyword("__PACKAGE__"));
    }
}
