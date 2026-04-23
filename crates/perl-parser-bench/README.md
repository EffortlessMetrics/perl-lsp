# perl-parser-bench

A small benchmarking binary for the `perl-parser` v3 native recursive-descent
Perl parser. It reads a single Perl source file (or walks a directory of them),
parses each one with [`perl_parser::Parser`], and prints microsecond-level
timings plus success statistics.

This crate is part of the [perl-lsp](https://github.com/EffortlessMetrics/perl-lsp)
ecosystem and is used by internal tooling (for example, `perl-ci-hygiene`) to
compare parser performance across test files.

## Installation

```bash
cargo install perl-parser-bench
```

Or, from a clone of the workspace:

```bash
cargo build --release -p perl-parser-bench
```

## Usage

Benchmark a single Perl file:

```bash
perl-parser-bench path/to/script.pl
```

Output:

```
status=success error=false duration_us=1234
```

Benchmark every file under a directory (recursively):

```bash
perl-parser-bench path/to/project/
```

Output:

```
total_files=42 error_files=0 success_rate=100.0 total_duration_us=567890
```

The `error` flag reports whether the parse returned an `Err` (as distinct from
a successful parse that contains recoverable errors — those are still counted
as successful, matching real-world usage).

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
