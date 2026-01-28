# Fuzzing Guide

This document describes the continuous fuzzing infrastructure for perl-lsp using cargo-fuzz.

## Overview

Fuzzing helps discover edge cases, crashes, and security vulnerabilities through automated random input generation. The perl-lsp project uses [cargo-fuzz](https://rust-fuzz.github.io/book/cargo-fuzz.html) (libFuzzer) for coverage-guided fuzzing.

## Prerequisites

```bash
# Install cargo-fuzz (requires nightly Rust)
cargo install cargo-fuzz

# Ensure nightly toolchain is available
rustup toolchain install nightly
```

## Available Fuzz Targets

The project includes multiple fuzz targets for different components:

| Target | Component | Description |
|--------|-----------|-------------|
| `parser_comprehensive` | Parser | Tests the full parser with arbitrary Perl code |
| `lexer_robustness` | Lexer | Tests tokenization with malformed inputs |
| `substitution_parsing` | Parser | Focuses on substitution operator edge cases |
| `builtin_functions` | Parser | Tests builtin function constructs (map/grep/sort) |
| `unicode_positions` | Parser | Tests Unicode handling and position tracking |
| `lsp_navigation` | LSP | Tests workspace navigation features |
| `heredoc_parsing` | Parser | Tests heredoc parsing edge cases |

## Quick Start

```bash
# List available fuzz targets
just fuzz-list

# Run fuzzing on a specific target for 60 seconds (default)
just fuzz parser_comprehensive

# Run for a custom duration (in seconds)
just fuzz parser_comprehensive 300

# Run continuous fuzzing (Ctrl+C to stop)
just fuzz-continuous parser_comprehensive
```

## CI Integration

The `fuzz-bounded` recipe runs multiple fuzz targets for 60 seconds each as part of the nightly test suite:

```bash
# Run bounded fuzzing (part of nightly gate)
just fuzz-bounded

# Run regression testing across all targets
just fuzz-regression 30
```

## Working with the Corpus

### Seed Corpus

Each fuzz target has a corpus directory at `fuzz/corpus/<target>/` containing seed inputs. These are version-controlled to ensure consistent fuzzing coverage.

The seed corpus for `parser_comprehensive` and `lexer_robustness` is automatically populated from the `examples/` directory.

### Coverage Analysis

Check code coverage achieved by the fuzzing corpus:

```bash
# Generate coverage report for a target
just fuzz-coverage parser_comprehensive

# View the HTML report
open fuzz/coverage/parser_comprehensive/coverage/index.html
```

## Crash Handling

### Finding Crashes

When fuzzing discovers a crash, the input is saved to `fuzz/artifacts/<target>/`:

```bash
# Check for any crashes
just fuzz-check-crashes

# This will exit with error code 1 if crashes are found
```

### Minimizing Crashes

Reduce a crashing input to its minimal reproducing case:

```bash
# Minimize a crash file
just fuzz-minimize parser_comprehensive fuzz/artifacts/parser_comprehensive/crash-<hash>

# The minimized input is saved back to artifacts
```

### Creating Regression Tests

When a crash is found:

1. Minimize the input: `just fuzz-minimize <target> <crash-file>`
2. Add the minimized input to `fuzz/corpus/<target>/` to prevent regression
3. Create a unit test in the appropriate test suite if the crash reveals a bug
4. Fix the underlying issue
5. Verify the crash is resolved by re-running: `just fuzz <target> 60`

## Advanced Usage

### Custom libFuzzer Arguments

Pass custom arguments directly to libFuzzer:

```bash
# Run with custom arguments
cargo +nightly fuzz run parser_comprehensive -- \
    -max_total_time=300 \
    -max_len=10000 \
    -rss_limit_mb=4096

# Or use the just recipe helper
just fuzz-custom parser_comprehensive "-max_total_time=300 -max_len=10000"
```

### Common libFuzzer Options

- `-max_total_time=N`: Run for N seconds
- `-max_len=N`: Maximum input length
- `-dict=path`: Use a fuzzing dictionary
- `-jobs=N`: Run N fuzzing jobs in parallel
- `-workers=N`: Use N worker processes
- `-rss_limit_mb=N`: Memory limit in MB

## Corpus Management

### Adding Custom Seeds

Add interesting test cases to the corpus:

```bash
# Copy a test file to the corpus
cp examples/complex_test.pl fuzz/corpus/parser_comprehensive/

# The fuzzer will use it as a seed for mutation
```

### Corpus Minimization

Remove redundant corpus entries while maintaining coverage:

```bash
# Minimize the corpus (removes duplicate coverage)
cargo +nightly fuzz cmin parser_comprehensive
```

## Integration with OSS-Fuzz

For continuous fuzzing at scale, perl-lsp can be integrated with [OSS-Fuzz](https://google.github.io/oss-fuzz/):

1. OSS-Fuzz provides free continuous fuzzing for open-source projects
2. Automatically runs fuzz targets 24/7 on Google infrastructure
3. Files bugs for crashes and provides minimized reproducers
4. Tracks code coverage and fuzzing statistics

To integrate:
1. Submit application at https://github.com/google/oss-fuzz
2. Provide build scripts and fuzz target list
3. OSS-Fuzz will run continuous fuzzing and report issues

## Best Practices

1. **Run Fuzzing Locally**: Run `just fuzz-bounded` before pushing large parser changes
2. **Monitor Corpus Growth**: Periodically check corpus size and minimize if needed
3. **Seed Interesting Inputs**: Add edge cases and regression tests to the corpus
4. **Check for Crashes**: Run `just fuzz-check-crashes` in CI
5. **Update Targets**: Add new fuzz targets when implementing new features
6. **Coverage-Guided**: Let the fuzzer explore code paths - don't over-constrain inputs

## Troubleshooting

### Out of Memory

Reduce memory usage with:
```bash
cargo +nightly fuzz run parser_comprehensive -- -rss_limit_mb=2048
```

### Slow Fuzzing

Check execution speed (execs/sec). If too slow:
- Reduce max input length: `-max_len=1000`
- Simplify the fuzz target
- Run with release mode (already default)

### No New Coverage

If fuzzing plateaus:
- Add diverse seed inputs to the corpus
- Try different mutation strategies
- Consider a fuzzing dictionary for domain-specific syntax

## Metrics

Track fuzzing effectiveness:
- **Executions/second**: Fuzzing throughput
- **Corpus size**: Number of interesting inputs found
- **Coverage**: Percentage of code exercised
- **Crashes found**: Security and robustness issues

View metrics during fuzzing - they're printed to the terminal.

## Related Resources

- [cargo-fuzz book](https://rust-fuzz.github.io/book/cargo-fuzz.html)
- [libFuzzer documentation](https://llvm.org/docs/LibFuzzer.html)
- [OSS-Fuzz documentation](https://google.github.io/oss-fuzz/)
- [Fuzzing Perl parsers research](https://lcamtuf.coredump.cx/afl/)

## Contributing

When adding new parser features:

1. Create a fuzz target if the feature has complex syntax
2. Add seed inputs covering the feature's syntax
3. Run fuzzing for at least 5 minutes: `just fuzz <target> 300`
4. Add the target to `fuzz-bounded` if critical

See [CONTRIBUTING.md](CONTRIBUTING.md) for general contribution guidelines.
