// Benchmark for position conversion performance
use criterion::{criterion_group, criterion_main, Criterion};
use perl_parser::positions::LineStartsCache;

fn big_text() -> String {
    let mut s = String::new();
    
    // 10k ASCII characters
    for _ in 0..10_000 {
        s.push('x');
    }
    
    // 5k surrogate pairs (each is 4 UTF-8 bytes, 2 UTF-16 units)
    for _ in 0..5_000 {
        s.push('𝐀'); // Mathematical bold A
    }
    
    // 5k lines with mixed content
    for i in 0..5_000 {
        if i % 2 == 0 {
            s.push_str("\r\nline ");
            s.push_str(&i.to_string());
            s.push('\n');
        } else {
            s.push_str("\nline ");
            s.push_str(&i.to_string());
            s.push(' ');
        }
    }
    
    s
}

fn bench_position_conversions(c: &mut Criterion) {
    let text = big_text();
    let cache = LineStartsCache::new(&text);
    
    // Sample various offsets throughout the file
    let sample_offsets: Vec<usize> = (0..text.len())
        .step_by(257)  // Prime number for good distribution
        .collect();
    
    c.bench_function("offset_to_pos16_cached", |b| {
        b.iter(|| {
            for &offset in &sample_offsets {
                let _ = cache.offset_to_position(&text, offset);
            }
        })
    });
    
    c.bench_function("pos16_to_offset_cached", |b| {
        // Pre-compute positions for round-trip
        let positions: Vec<(u32, u32)> = sample_offsets
            .iter()
            .map(|&o| cache.offset_to_position(&text, o))
            .collect();
        
        b.iter(|| {
            for &(line, col) in &positions {
                let _ = cache.position_to_offset(&text, line, col);
            }
        })
    });
    
    // Benchmark round-trip (offset -> position -> offset)
    c.bench_function("position_round_trip", |b| {
        b.iter(|| {
            for &offset in &sample_offsets {
                let (line, col) = cache.offset_to_position(&text, offset);
                let _ = cache.position_to_offset(&text, line, col);
            }
        })
    });
}

criterion_group!(benches, bench_position_conversions);
criterion_main!(benches);