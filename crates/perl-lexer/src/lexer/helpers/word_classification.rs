use crate::keywords::is_lexer_keyword;

#[inline(always)]
pub(crate) fn is_keyword_fast(word: &str) -> bool {
    // Fast length-based rejection for most cases.
    // Lexer keywords are currently bounded to 1..=9 characters.
    matches!(word.len(), 1..=9) && is_lexer_keyword(word)
}

#[inline]
pub(crate) fn is_builtin_function(word: &str) -> bool {
    BARE_TERM_BUILTINS.binary_search(&word).is_ok()
}

#[inline(always)]
pub(crate) fn is_quote_op_word_prefix(word: &[u8]) -> bool {
    matches!(word, b"m" | b"q" | b"qq" | b"qw" | b"qx" | b"qr")
}

const BARE_TERM_BUILTINS: &[&str] = &[
    "abs", "chomp", "chop", "chr", "close", "defined", "delete", "each", "exists", "hex", "int",
    "join", "keys", "lc", "lcfirst", "length", "oct", "open", "ord", "pack", "print", "push",
    "read", "ref", "reverse", "rindex", "say", "scalar", "splice", "sprintf", "sqrt", "substr",
    "tie", "uc", "ucfirst", "unpack", "unshift", "untie", "values", "write",
];
