//! Configuration for the Perl lexer.

use crate::symbol_table::LocalSymbolTable;

/// Configuration options for the Perl lexer.
///
/// Controls interpolation handling, position tracking, lookahead limits, and
/// the optional file-local symbol table used for bareword/regex disambiguation.
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
///     symbol_table: None,
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
    /// Optional file-local subroutine symbol table for bareword/regex disambiguation.
    ///
    /// When `Some`, the lexer treats identifiers that appear in this table as
    /// known subroutine names and sets `LexerMode::ExpectTerm` after them, so
    /// that a subsequent `/` is lexed as a regex rather than division.
    ///
    /// Build this with [`LocalSymbolTable::scan_subs`] before constructing the
    /// lexer. When `None` (the default), the existing heuristic is used:
    /// only built-in functions trigger `ExpectTerm`; all other bare identifiers
    /// trigger `ExpectOperator`.
    pub symbol_table: Option<LocalSymbolTable>,
}

impl Default for LexerConfig {
    fn default() -> Self {
        Self {
            parse_interpolation: true,
            track_positions: true,
            max_lookahead: 1024,
            symbol_table: None,
        }
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
    fn default_symbol_table_is_none() {
        let config = LexerConfig::default();

        assert!(config.symbol_table.is_none());
    }

    #[test]
    fn clone_preserves_field_values() {
        let config = LexerConfig {
            parse_interpolation: false,
            track_positions: false,
            max_lookahead: 256,
            symbol_table: None,
        };

        let cloned = config.clone();

        assert!(!cloned.parse_interpolation);
        assert!(!cloned.track_positions);
        assert_eq!(cloned.max_lookahead, 256);
        assert!(cloned.symbol_table.is_none());
    }
}
