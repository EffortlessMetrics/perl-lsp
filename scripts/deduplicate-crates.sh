#!/bin/bash
set -e

# Option B: Make tree-sitter-perl-rs a thin wrapper around perl-parser-pest
# This avoids maintaining duplicate code

echo "Converting tree-sitter-perl-rs to a thin wrapper..."

# Step 1: Backup the grammar file (the only unique content we need)
cp crates/tree-sitter-perl-rs/src/grammar.pest /tmp/grammar.pest.backup 2>/dev/null || true

# Step 2: Remove all duplicate source files from tree-sitter-perl-rs
cd crates/tree-sitter-perl-rs
rm -rf src/*.rs src/*.c src/*.h src/scanner src/tree_sitter src/*.js src/*.json
rm -rf examples/*.rs tests/*.rs

# Step 3: Create a minimal lib.rs that re-exports from perl-parser-pest
cat > src/lib.rs << 'EOF'
//! Tree-sitter Perl - Compatibility wrapper for perl-parser-pest
//!
//! This crate provides backward compatibility for code using tree-sitter-perl.
//! The actual implementation is in perl-parser-pest.

#[doc(hidden)]
pub use perl_parser_pest::*;

// Re-export main types for compatibility
pub use perl_parser_pest::{
    error::{ParseError, ErrorKind},
    PureRustPerlParser,
};

#[cfg(feature = "pure-rust")]
pub use perl_parser_pest::pure_rust_parser;
EOF

# Step 4: Update Cargo.toml to depend on perl-parser-pest
cat > Cargo.toml << 'EOF'
[package]
name = "tree-sitter-perl"
version = "0.8.3"
edition = "2024"
rust-version = "1.75"
description = "Compatibility wrapper for perl-parser-pest (v2 Pest parser)"
repository = "https://github.com/EffortlessMetrics/perl-lsp"
license = "MIT OR Apache-2.0"
readme = "README.md"
keywords = ["perl", "parser", "tree-sitter", "pest"]
categories = ["parsing", "text-processing"]
publish = false  # Don't publish the wrapper

[dependencies]
perl-parser-pest = { version = "0.8.3", path = "../perl-parser-pest" }

[features]
default = []
pure-rust = ["perl-parser-pest/pure-rust"]
test-utils = ["perl-parser-pest/test-utils"]
EOF

# Step 5: Create a README explaining the wrapper
cat > README.md << 'EOF'
# tree-sitter-perl-rs

> ⚠️ **This is a compatibility wrapper.** The actual implementation is in [`perl-parser-pest`](../perl-parser-pest).

This crate exists only to maintain backward compatibility for code that imports `tree-sitter-perl`.
All functionality has been moved to `perl-parser-pest` to avoid name conflicts on crates.io.

## For New Code

Use `perl-parser-pest` directly:

```toml
[dependencies]
perl-parser-pest = "0.8.3"
```

## For Existing Code

This wrapper will continue to work, but consider migrating to `perl-parser-pest` directly.
EOF

cd ../..

echo "✅ Deduplication complete!"
echo ""
echo "tree-sitter-perl-rs is now a thin wrapper around perl-parser-pest."
echo "This eliminates the code duplication while maintaining compatibility."
echo ""
echo "Next steps:"
echo "1. Review the changes"
echo "2. Run: cargo test --all"
echo "3. Commit the deduplication"