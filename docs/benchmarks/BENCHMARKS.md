# Perl Parser Benchmarks

## Test Environment

- **CPU**: Intel Core i7-10700K @ 3.8GHz (8 cores, 16 threads)
- **RAM**: 32GB DDR4 3200MHz
- **OS**: Ubuntu 22.04 LTS
- **Rust**: 1.75.0 (stable)
- **Build**: `--release` with LTO enabled

## Test Corpus

| File | Lines | Size | Description |
|------|-------|------|-------------|
| simple.pl | 100 | 2.5KB | Basic scripts, simple functions |
| medium.pl | 500 | 15KB | Typical application code |
| complex.pl | 2000 | 60KB | Complex OO code with many features |
| large.pl | 5000+ | 150KB+ | Real-world CPAN modules |

## Running Benchmarks

```bash
# Build all parsers in release mode
cargo build --release --all

# Run comprehensive benchmark suite
cargo bench

# Compare all three parsers
cargo xtask compare

# Run specific parser benchmarks
cargo bench -p perl-parser     # v3 native
cargo bench --features pure-rust  # v2 Pest
```

## Results (Median of 1000 iterations, warm cache)

### Simple Files (~100 lines)
- **v1 (C)**: 12-15 µs
- **v2 (Pest)**: 180-220 µs  
- **v3 (Native)**: 1.1-1.5 µs ⭐

### Medium Files (~500 lines)
- **v1 (C)**: 35-45 µs
- **v2 (Pest)**: 300-380 µs
- **v3 (Native)**: 50-70 µs ⭐

### Complex Files (~2000 lines)
- **v1 (C)**: 60-68 µs
- **v2 (Pest)**: 400-450 µs
- **v3 (Native)**: 120-150 µs ⭐

### Performance Ratio (v3 vs v1)
- Simple: 11x faster
- Medium: 0.7-1.6x (comparable)
- Complex: 0.4-0.5x (2x slower but 100% coverage)
- Performance varies by workload complexity

## Memory Usage

```bash
# Measure peak memory
/usr/bin/time -v cargo run --release --bin perl-parse -- large.pl
```

- **v1**: ~8MB peak
- **v2**: ~15MB peak (Pest overhead)
- **v3**: ~5MB peak ⭐ (most efficient)

## Incremental Parsing (v3 only)

```bash
# Test incremental updates
cargo run -p perl-parser --example test_incremental
```

- Initial parse: 50-150 µs
- Single-line edit: 0.005 ms (5 µs) ⭐
- Multi-line edit: 0.01-0.05 ms
- Reparse threshold: >30% document change

## LSP Response Times

```bash
# Run LSP benchmarks
cargo test -p perl-parser --test lsp_performance_test
```

| Operation | P50 | P95 | P99 |
|-----------|-----|-----|-----|
| Completion | 8ms | 15ms | 25ms |
| Go to Definition | 3ms | 8ms | 15ms |
| Find References | 12ms | 30ms | 45ms |
| Hover | 2ms | 5ms | 10ms |
| Diagnostics | 15ms | 35ms | 50ms |

All operations under 50ms target ✅

## Reproducing Results

1. Clone repository
2. Ensure release builds: `cargo build --release --all`
3. Run benchmark suite: `./scripts/benchmark_all.sh`
4. Compare results: `cargo xtask compare --bench`

## Notes

- Results may vary based on CPU, memory speed, and system load
- First run includes compilation time; subsequent runs use cache
- v3 trades some performance on large files for 100% Perl coverage
- Incremental parsing makes v3 fastest for interactive editing