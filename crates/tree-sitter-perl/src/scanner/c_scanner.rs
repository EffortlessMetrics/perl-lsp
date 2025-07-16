//! C scanner wrapper for backward compatibility
//!
//! This module provides a wrapper around the existing C scanner implementation
//! to maintain compatibility during the transition to the Rust scanner.

use super::{PerlScanner, ScannerConfig, ScannerState, TokenType};
use crate::error::{ParseError, ParseResult};
use tree_sitter::{Lexer, LexicalError};

/// C scanner wrapper implementation
pub struct CScanner {
    config: ScannerConfig,
    state: ScannerState,
}

impl CScanner {
    /// Create a new C scanner with default configuration
    pub fn new() -> Self {
        Self::with_config(ScannerConfig::default())
    }

    /// Create a new C scanner with custom configuration
    pub fn with_config(config: ScannerConfig) -> Self {
        Self {
            config,
            state: ScannerState::default(),
        }
    }
}

impl PerlScanner for CScanner {
    fn scan(&mut self, lexer: &mut Lexer) -> ParseResult<Option<u16>> {
        // This is a placeholder - the actual implementation would
        // call the C scanner functions through FFI
        // For now, we'll return None to indicate EOF
        Ok(None)
    }

    fn serialize(&self, buffer: &mut Vec<u8>) -> ParseResult<()> {
        // Serialize C scanner state
        buffer.extend_from_slice(&self.state.line.to_le_bytes());
        buffer.extend_from_slice(&self.state.column.to_le_bytes());
        buffer.extend_from_slice(&self.state.offset.to_le_bytes());
        Ok(())
    }

    fn deserialize(&mut self, buffer: &[u8]) -> ParseResult<()> {
        if buffer.len() < 24 {
            return Err(ParseError::scanner_error_simple("Invalid buffer size"));
        }

        let mut offset = 0;

        // Deserialize state
        self.state.line = u32::from_le_bytes([
            buffer[offset],
            buffer[offset + 1],
            buffer[offset + 2],
            buffer[offset + 3],
        ]) as usize;
        offset += 4;

        self.state.column = u32::from_le_bytes([
            buffer[offset],
            buffer[offset + 1],
            buffer[offset + 2],
            buffer[offset + 3],
        ]) as usize;
        offset += 4;

        self.state.offset = u32::from_le_bytes([
            buffer[offset],
            buffer[offset + 1],
            buffer[offset + 2],
            buffer[offset + 3],
        ]) as usize;

        Ok(())
    }

    fn is_eof(&self) -> bool {
        // This would need to be implemented based on the C scanner state
        false
    }

    fn position(&self) -> (usize, usize) {
        self.state.position()
    }
}

impl Default for CScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_c_scanner_creation() {
        let scanner = CScanner::new();
        assert_eq!(scanner.state.line, 1);
        assert_eq!(scanner.state.column, 1);
    }
}
