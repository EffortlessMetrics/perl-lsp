//! Demo of high-performance incremental parsing with <1ms updates
//!
//! Run with: cargo run -p perl-parser --example incremental_parsing_demo --features incremental
//!
//! Note: This demo requires the 'incremental' feature and the 'yansi' dependency which
//! are not included by default. To run this demo, add yansi to dev-dependencies.

fn main() {
    println!("This demo requires the 'incremental' feature and yansi dependency.");
    println!("To run it:");
    println!("1. Add to Cargo.toml: yansi = \"0.5\"");
    println!("2. Run: cargo run --example incremental_parsing_demo --features incremental");
}

// Original code below - uncomment after adding yansi dependency
/*
#[cfg(feature = "incremental")]
use perl_parser::{
    incremental_document::IncrementalDocument,
    incremental_edit::IncrementalEdit,
};
use std::time::Instant;
use yansi::Paint;

fn main() {
    println!("🚀 {} {}", "Incremental Parsing Demo".bold().cyan(), "- Achieving <1ms updates".yellow());
    println!("{}", "=".repeat(60));

    // ... rest of the original demo code ...
}
*/
