//! Memory profiling harness for large workspace symbol resolution.
//!
//! Exercises [`WorkspaceIndex`] at 100 / 500 / 1 000 / 5 000 / 10 000 synthetic
//! file scales and reports per-component memory estimates using
//! [`MemorySnapshot`].
//!
//! # Usage
//!
//! ```text
//! cargo run -p perl-workspace-index \
//!     --bin workspace_memory_profile \
//!     --features memory-profiling \
//!     -- [--scale N] [--json]
//! ```
//!
//! ## Flags
//!
//! - `--scale N`   Run a single scale point at N files (default: full suite).
//! - `--json`      Emit newline-delimited JSON records instead of a table.
//!
//! # Output
//!
//! Without flags a table is printed to stdout:
//!
//! ```text
//! files       total_B      symbols    B/sym          B/file
//! ----------------------------------------------------------------
//! 100          302451          500          604            3024
//! 500         1512255         2500          604            3024
//! 1000        3024510         5000          604            3024
//! 5000       15122550        25000          604            3024
//! 10000      30245100        50000          604            3024
//!
//! Scaling factor vs linear: 1.00x
//! ```
//!
//! Requires the `memory-profiling` feature.

#![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]

use perl_workspace::workspace::memory::{MemorySnapshot, ScaleReport};
use perl_workspace::workspace_index::WorkspaceIndex;
use std::time::Instant;
use url::Url;

/// Generate a realistic synthetic Perl module at file index `idx`.
///
/// Each module contains 5 public symbols:
/// `new`, `method_a_N`, `method_b_N`, `method_c_N`, `_private_N`.
fn generate_module(idx: usize) -> (Url, String) {
    let uri = Url::parse(&format!("file:///lib/Gen/Module{}.pm", idx))
        .expect("valid uri for synthetic module");
    let src = format!(
        r#"package Gen::Module{idx};
use strict;
use warnings;

our $VERSION = '1.00';

sub new {{
    my $class = shift;
    return bless {{}}, $class;
}}

sub method_a_{idx} {{
    my ($self, $x) = @_;
    return $x + {idx};
}}

sub method_b_{idx} {{
    my ($self, $y) = @_;
    return $y * {idx};
}}

sub method_c_{idx} {{
    my ($self) = @_;
    return "module_{idx}";
}}

sub _private_{idx} {{
    return {idx};
}}

1;
"#
    );
    (uri, src)
}

/// Build a [`WorkspaceIndex`] containing `file_count` synthetic modules.
fn build_index_at_scale(file_count: usize) -> WorkspaceIndex {
    let index = WorkspaceIndex::new();
    for i in 0..file_count {
        let (uri, src) = generate_module(i);
        index.index_file(uri, src).ok();
    }
    index
}

/// Run the profiling harness and return a populated [`ScaleReport`].
///
/// `scales` is the list of file-count checkpoints to measure.
fn run_profile(scales: &[usize]) -> ScaleReport {
    let mut report = ScaleReport::new();

    for &scale in scales {
        let t0 = Instant::now();
        let index = build_index_at_scale(scale);
        let build_ms = t0.elapsed().as_millis();

        let snap = MemorySnapshot::capture(&index);
        eprintln!(
            "[profile] scale={:>6} files indexed in {:>5}ms  \
             total={:>10}B  symbols={:>7}  B/sym={:>6}",
            scale,
            build_ms,
            snap.total_estimated_bytes(),
            snap.symbol_count,
            snap.bytes_per_symbol(),
        );

        report.add_checkpoint(scale, snap);
    }

    report
}

/// Emit the report as newline-delimited JSON to stdout.
fn print_json(report: &ScaleReport) {
    for (file_count, snap) in report.checkpoints() {
        println!(
            "{{\"file_count\":{},\"total_bytes\":{},\"files_bytes\":{},\
             \"symbols_bytes\":{},\"global_refs_bytes\":{},\
             \"document_store_bytes\":{},\"symbol_count\":{},\
             \"bytes_per_symbol\":{}}}",
            file_count,
            snap.total_estimated_bytes(),
            snap.files_bytes,
            snap.symbols_bytes,
            snap.global_refs_bytes,
            snap.document_store_bytes,
            snap.symbol_count,
            snap.bytes_per_symbol(),
        );
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let mut emit_json = false;
    let mut single_scale: Option<usize> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--json" => emit_json = true,
            "--scale" => {
                i += 1;
                if let Some(val) = args.get(i) {
                    single_scale = val.parse::<usize>().ok();
                }
            }
            _ => {}
        }
        i += 1;
    }

    let default_scales: Vec<usize> = vec![100, 500, 1_000, 5_000, 10_000];
    let scales: Vec<usize> = match single_scale {
        Some(n) => vec![n],
        None => default_scales,
    };

    let report = run_profile(&scales);

    if emit_json {
        print_json(&report);
    } else {
        println!("{}", report);
    }
}
