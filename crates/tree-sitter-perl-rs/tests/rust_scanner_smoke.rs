#![cfg(feature = "rust-scanner")]

use tree_sitter_perl::scanner::{PerlScanner, RustScanner};

#[test]
fn rust_scanner_emits_tokens() {
    // Basic Perl statement should yield at least one token
    let mut scanner = RustScanner::new();
    let token =
        scanner.scan(b"my $x = 1;").expect("scan should succeed").expect("expected a token");
    assert_ne!(token, 0, "scanner returned a zero token");
}
