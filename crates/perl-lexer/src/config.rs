/// Configuration options for the Perl lexer.
///
/// Controls interpolation handling, position tracking, and lookahead limits.
/// Use [`Default::default`] for sensible defaults.
///
/// # Examples
///
/// ```rust
/// use perl_lexer::LexerConfig;
///
/// let config = LexerConfig {
///     parse_interpolation: true,
///     track_positions: true,
///     max_lookahead: 1024,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct LexerConfig {
    /// Enable interpolation parsing in strings.
    pub parse_interpolation: bool,
    /// Track token positions for error reporting.
    pub track_positions: bool,
    /// Maximum lookahead for disambiguation.
    pub max_lookahead: usize,
}

impl Default for LexerConfig {
    fn default() -> Self {
        Self { parse_interpolation: true, track_positions: true, max_lookahead: 1024 }
    }
}
