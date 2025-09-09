//! Compatibility wrapper that mimics the old C scanner API while delegating to
//! the new Rust implementation.  This allows benchmark code that expects a
//! "C" scanner to compile without having to maintain the original C code.

use super::{rust_scanner::RustScanner, PerlScanner, ScannerConfig};
use crate::error::ParseResult;

/// Wrapper around [`RustScanner`] that exposes the legacy `CScanner` type.
pub struct CScanner {
    inner: RustScanner,
}

impl CScanner {
    /// Create a new scanner using default configuration.
    pub fn new() -> Self {
        Self { inner: RustScanner::new() }
    }

    /// Create a new scanner with a custom configuration.
    pub fn with_config(config: ScannerConfig) -> Self {
        Self { inner: RustScanner::with_config(config) }
    }
}

impl PerlScanner for CScanner {
    fn scan(&mut self, input: &[u8]) -> ParseResult<Option<u16>> {
        self.inner.scan(input)
    }

    fn serialize(&self, buffer: &mut Vec<u8>) -> ParseResult<()> {
        self.inner.serialize(buffer)
    }

    fn deserialize(&mut self, buffer: &[u8]) -> ParseResult<()> {
        self.inner.deserialize(buffer)
    }

    fn is_eof(&self) -> bool {
        self.inner.is_eof()
    }

    fn position(&self) -> (usize, usize) {
        self.inner.position()
    }

    fn is_regex_context(&self) -> bool {
        self.inner.is_regex_context()
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
    fn test_c_scanner_delegates() {
        let mut scanner = CScanner::new();
        let token = scanner.scan(b"my $x = 1;").unwrap();
        assert!(token.is_some());
    }
}
