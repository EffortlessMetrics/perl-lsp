//! Tests for memory profiling infrastructure.
//!
//! Verifies that `MemorySnapshot` captures meaningful per-component sizes
//! after indexing synthetic workspaces at varying scales.
//!
//! Requires the `memory-profiling` feature.

#![cfg(feature = "memory-profiling")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use perl_workspace::workspace::memory::{MemorySnapshot, ScaleReport};
use perl_workspace::workspace_index::WorkspaceIndex;
use url::Url;

/// Generate a synthetic Perl module with a known number of symbols (5 per module).
fn generate_module(idx: usize) -> (Url, String) {
    let uri = Url::parse(&format!("file:///lib/Profile/Module{}.pm", idx)).expect("valid uri");
    let src = format!(
        r#"package Profile::Module{idx};
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
    return "{idx}";
}}

sub _private_{idx} {{
    return {idx};
}}

1;
"#
    );
    (uri, src)
}

#[test]
fn memory_snapshot_is_zero_for_empty_index() {
    let index = WorkspaceIndex::new();
    let snap = MemorySnapshot::capture(&index);

    assert_eq!(snap.file_count, 0, "empty index should have 0 files");
    assert_eq!(snap.symbol_count, 0, "empty index should have 0 symbols");
    assert_eq!(
        snap.files_bytes, 0,
        "empty index should report 0 bytes for files map"
    );
    assert_eq!(
        snap.symbols_bytes, 0,
        "empty index should report 0 bytes for symbols map"
    );
    assert_eq!(
        snap.total_estimated_bytes(),
        0,
        "total should be 0 for empty index"
    );
}

#[test]
fn memory_snapshot_grows_with_files() {
    let index = WorkspaceIndex::new();

    // Index 10 files
    for i in 0..10 {
        let (uri, src) = generate_module(i);
        index.index_file(uri, src).ok();
    }

    let snap = MemorySnapshot::capture(&index);

    assert_eq!(snap.file_count, 10, "should report 10 indexed files");
    assert!(snap.symbol_count > 0, "should have symbols after indexing");
    assert!(
        snap.files_bytes > 0,
        "files_bytes should be positive after indexing"
    );
    assert!(
        snap.symbols_bytes > 0,
        "symbols_bytes should be positive after indexing"
    );
    assert!(
        snap.total_estimated_bytes() > 0,
        "total should be positive after indexing"
    );
}

#[test]
fn memory_snapshot_scales_linearly_with_file_count() {
    let index_small = WorkspaceIndex::new();
    let index_large = WorkspaceIndex::new();

    // Index 10 files in small
    for i in 0..10 {
        let (uri, src) = generate_module(i);
        index_small.index_file(uri, src).ok();
    }

    // Index 100 files in large
    for i in 0..100 {
        let (uri, src) = generate_module(i);
        index_large.index_file(uri, src).ok();
    }

    let snap_small = MemorySnapshot::capture(&index_small);
    let snap_large = MemorySnapshot::capture(&index_large);

    // Large should be at least 5x the size of small (rough linear check)
    // Allows for header/overhead that doesn't scale
    assert!(
        snap_large.total_estimated_bytes() > snap_small.total_estimated_bytes() * 5,
        "memory should scale with file count: small={} large={}",
        snap_small.total_estimated_bytes(),
        snap_large.total_estimated_bytes()
    );

    assert!(
        snap_large.file_count == 100,
        "large index should have 100 files"
    );
    assert!(
        snap_small.file_count == 10,
        "small index should have 10 files"
    );
}

#[test]
fn memory_snapshot_display_is_human_readable() {
    let index = WorkspaceIndex::new();
    for i in 0..5 {
        let (uri, src) = generate_module(i);
        index.index_file(uri, src).ok();
    }

    let snap = MemorySnapshot::capture(&index);
    let display = snap.to_report_string();

    // Display should contain component names
    assert!(
        display.contains("files"),
        "display should mention files component"
    );
    assert!(
        display.contains("symbols"),
        "display should mention symbols component"
    );
    assert!(display.contains("total"), "display should mention total");
}

#[test]
fn scale_report_captures_multiple_checkpoints() {
    let mut report = ScaleReport::new();

    for scale in [10usize, 50, 100] {
        let index = WorkspaceIndex::new();
        for i in 0..scale {
            let (uri, src) = generate_module(i);
            index.index_file(uri, src).ok();
        }
        let snap = MemorySnapshot::capture(&index);
        report.add_checkpoint(scale, snap);
    }

    assert_eq!(report.checkpoints().len(), 3, "should have 3 checkpoints");

    // Memory should increase monotonically across scales
    let mems: Vec<usize> = report
        .checkpoints()
        .iter()
        .map(|(_, s)| s.total_estimated_bytes())
        .collect();

    assert!(
        mems[0] < mems[1] && mems[1] < mems[2],
        "memory should increase monotonically: {:?}",
        mems
    );
}

#[test]
fn memory_snapshot_bytes_per_symbol_is_reasonable() {
    let index = WorkspaceIndex::new();

    // Index 100 files, each with ~5 symbols => ~500 symbols
    for i in 0..100 {
        let (uri, src) = generate_module(i);
        index.index_file(uri, src).ok();
    }

    let snap = MemorySnapshot::capture(&index);
    assert!(snap.symbol_count > 0, "should have symbols");

    // bytes_per_symbol should be between 10 and 10,000 bytes
    // (sanity check - not zero, not absurdly large)
    let bytes_per_symbol = snap.bytes_per_symbol();
    assert!(
        bytes_per_symbol >= 10,
        "bytes per symbol should be at least 10: got {}",
        bytes_per_symbol
    );
    assert!(
        bytes_per_symbol <= 10_000,
        "bytes per symbol should be at most 10,000: got {}",
        bytes_per_symbol
    );
}
