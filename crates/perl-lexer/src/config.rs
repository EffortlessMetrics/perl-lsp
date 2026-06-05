//! Configuration for the Perl lexer.

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

#[cfg(test)]
mod tests {
    use super::LexerConfig;

    #[test]
    fn default_enables_interpolation_and_position_tracking() {
        let config = LexerConfig::default();

        assert!(config.parse_interpolation);
        assert!(config.track_positions);
    }

    #[test]
    fn default_uses_expected_lookahead_limit() {
        let config = LexerConfig::default();

        assert_eq!(config.max_lookahead, 1024);
    }

    #[test]
    fn clone_preserves_field_values() {
        let config =
            LexerConfig { parse_interpolation: false, track_positions: false, max_lookahead: 256 };

        let cloned = config.clone();

        assert!(!cloned.parse_interpolation);
        assert!(!cloned.track_positions);
        assert_eq!(cloned.max_lookahead, 256);
    }
}
