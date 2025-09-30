use criterion::{Criterion, criterion_group, criterion_main};
use lsp_types::{Position, Range, TextDocumentContentChangeEvent};
use perl_parser::textdoc::{Doc, PosEnc, apply_changes, byte_to_lsp_pos, lsp_pos_to_byte};
use ropey::Rope;
use std::hint::black_box;

/// Benchmark Rope vs String for large document operations
fn benchmark_rope_vs_string_insertions(c: &mut Criterion) {
    let large_content = "x".repeat(50_000);

    let mut group = c.benchmark_group("document_insertions");

    // Benchmark Rope insertions
    group.bench_function("rope_insertion", |b| {
        b.iter(|| {
            let mut rope = Rope::from_str(&large_content);
            rope.insert(black_box(25_000), black_box(" INSERTED TEXT "));
            black_box(rope)
        })
    });

    // Benchmark String insertions
    group.bench_function("string_insertion", |b| {
        b.iter(|| {
            let mut content = large_content.clone();
            content.insert_str(black_box(25_000), black_box(" INSERTED TEXT "));
            black_box(content)
        })
    });

    group.finish();
}

/// Benchmark position conversions in large documents
fn benchmark_position_conversions(c: &mut Criterion) {
    // Create a large document with varied line lengths and Unicode
    let mut content = String::new();
    for i in 0..1000 {
        if i % 10 == 0 {
            content.push_str(&format!("# Long line {}: This is a much longer line with Unicode 🚀 characters café naïve résumé 中文\n", i));
        } else {
            content.push_str(&format!("# Line {}: Short\n", i));
        }
    }

    let rope = Rope::from_str(&content);

    let mut group = c.benchmark_group("position_conversions");

    // Benchmark UTF-16 position to byte conversion
    group.bench_function("utf16_pos_to_byte", |b| {
        b.iter(|| {
            let pos = Position::new(black_box(500), black_box(15));
            black_box(lsp_pos_to_byte(&rope, pos, PosEnc::Utf16))
        })
    });

    // Benchmark byte to UTF-16 position conversion
    group.bench_function("byte_to_utf16_pos", |b| {
        b.iter(|| black_box(byte_to_lsp_pos(&rope, black_box(25_000), PosEnc::Utf16)))
    });

    // Benchmark UTF-8 position to byte conversion (faster path)
    group.bench_function("utf8_pos_to_byte", |b| {
        b.iter(|| {
            let pos = Position::new(black_box(500), black_box(15));
            black_box(lsp_pos_to_byte(&rope, pos, PosEnc::Utf8))
        })
    });

    group.finish();
}

/// Benchmark incremental document changes
fn benchmark_incremental_edits(c: &mut Criterion) {
    let mut content = String::new();
    for i in 0..1000 {
        content.push_str(&format!("# Line {}: Some content here\n", i));
    }

    let mut group = c.benchmark_group("incremental_edits");

    // Benchmark multiple small edits (common LSP scenario)
    group.bench_function("multiple_small_edits", |b| {
        b.iter(|| {
            let mut doc = Doc { rope: Rope::from_str(&content), version: 1 };

            let edits = vec![
                TextDocumentContentChangeEvent {
                    range: Some(Range::new(Position::new(100, 0), Position::new(100, 0))),
                    range_length: None,
                    text: "# Inserted line 1\n".to_string(),
                },
                TextDocumentContentChangeEvent {
                    range: Some(Range::new(Position::new(500, 5), Position::new(500, 10))),
                    range_length: None,
                    text: "CHANGED".to_string(),
                },
                TextDocumentContentChangeEvent {
                    range: Some(Range::new(Position::new(800, 0), Position::new(800, 0))),
                    range_length: None,
                    text: "# Inserted line 2\n".to_string(),
                },
            ];

            apply_changes(&mut doc, &edits, PosEnc::Utf16);
            black_box(doc)
        })
    });

    // Benchmark single large edit
    group.bench_function("single_large_edit", |b| {
        b.iter(|| {
            let mut doc = Doc { rope: Rope::from_str(&content), version: 1 };

            let large_text = "# ".repeat(5000) + "Large insertion\n";
            let edit = TextDocumentContentChangeEvent {
                range: Some(Range::new(Position::new(500, 0), Position::new(500, 0))),
                range_length: None,
                text: large_text,
            };

            apply_changes(&mut doc, &[edit], PosEnc::Utf16);
            black_box(doc)
        })
    });

    group.finish();
}

/// Benchmark document navigation operations
fn benchmark_rope_navigation(c: &mut Criterion) {
    let mut content = String::new();
    for i in 0..2000 {
        content.push_str(&format!("Line {}: This line contains some text for benchmarking navigation operations in large documents\n", i));
    }

    let rope = Rope::from_str(&content);

    let mut group = c.benchmark_group("rope_navigation");

    // Benchmark line-based operations
    group.bench_function("line_operations", |b| {
        b.iter(|| {
            let line_idx = black_box(1000);
            let line = rope.line(line_idx);
            let line_len = line.len_chars();
            black_box((line, line_len))
        })
    });

    // Benchmark character-based operations
    group.bench_function("char_operations", |b| {
        b.iter(|| {
            let char_idx = black_box(50_000);
            let byte_idx = rope.char_to_byte(char_idx);
            let back_to_char = rope.byte_to_char(byte_idx);
            black_box((byte_idx, back_to_char))
        })
    });

    // Benchmark slicing operations
    group.bench_function("slice_operations", |b| {
        b.iter(|| {
            let start = black_box(10_000);
            let end = black_box(20_000);
            let slice = rope.byte_slice(start..end);
            let slice_str = slice.to_string();
            black_box(slice_str)
        })
    });

    group.finish();
}

criterion_group!(
    rope_benches,
    benchmark_rope_vs_string_insertions,
    benchmark_position_conversions,
    benchmark_incremental_edits,
    benchmark_rope_navigation
);
criterion_main!(rope_benches);
