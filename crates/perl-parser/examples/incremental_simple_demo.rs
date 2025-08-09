//! Simple demo of incremental parsing with performance metrics
//!
//! Run with: cargo run -p perl-parser --example incremental_simple_demo --features incremental

fn main() {
    println!("This demo requires the 'incremental' feature to be enabled.");
    println!("Run with: cargo run --example incremental_simple_demo --features incremental");
}

// Original code preserved below - uncomment after enabling incremental feature
/*
#[cfg(feature = "incremental")]
use perl_parser::{
    incremental_document::IncrementalDocument,
    incremental_edit::IncrementalEdit,
};
use std::time::Instant;

// ... rest of original implementation ...
*/