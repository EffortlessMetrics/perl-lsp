//! Lexer parse budgets and limits used for graceful degradation on pathological input.

// Budget limits to prevent hangs on pathological input
// When these limits are exceeded, the lexer gracefully truncates the token
// as UnknownRest, preserving all previously parsed symbols and allowing
// continued analysis of the remainder. LSP clients may emit a soft diagnostic
// about truncation but won't crash or hang.
pub(crate) const MAX_REGEX_BYTES: usize = 64 * 1024; // 64KB max for regex patterns
pub(crate) const MAX_HEREDOC_BYTES: usize = 256 * 1024; // 256KB max for heredoc bodies
pub(crate) const MAX_DELIM_NEST: usize = 128; // Max nesting depth for delimiters
pub(crate) const MAX_HEREDOC_DEPTH: usize = 100; // Max nesting depth for heredocs
pub(crate) const HEREDOC_TIMEOUT_MS: u64 = 5000; // 5 seconds timeout for heredoc parsing

/// Maximum scan iterations for a single regex literal.
/// This is a lexer parse budget, not regex-engine backtracking detection.
///
/// When the lexer encounters a regex literal that requires more than this
/// number of loop iterations, it
/// will emit an UnknownRest token for graceful degradation rather than
/// potentially hanging on pathological input.
///
/// The limit intentionally stays below `MAX_REGEX_BYTES` so this guard remains
/// reachable before the byte budget for very large but still bounded literals.
pub const MAX_REGEX_PARSE_STEPS: usize = 32 * 1024;
