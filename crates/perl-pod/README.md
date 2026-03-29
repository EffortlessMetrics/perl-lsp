# perl-pod

[![Crates.io](https://img.shields.io/crates/v/perl-pod.svg)](https://crates.io/crates/perl-pod)
[![Documentation](https://docs.rs/perl-pod/badge.svg)](https://docs.rs/perl-pod)

POD extraction utilities for Perl modules.

## When to use this crate

Use `perl-pod` when you need lightweight access to POD sections from a Perl
source file, especially for hover text, module summaries, or local
documentation indexing.

It extracts a focused structured view:

- module name and short description
- synopsis text
- description text
- `=head2` method sections

## Quick example

```rust
use perl_pod::extract_pod;

let doc = extract_pod(
    "=head1 NAME\nMy::Module - Example module\n\n=head1 SYNOPSIS\nuse My::Module;\n=cut\n",
);

assert!(doc.name.is_some());
assert!(doc.synopsis.is_some());
```

## Public API

- `extract_pod`: extract POD from a source string
- `extract_pod_from_file`: read a file and extract POD
- `PodDoc`: structured extracted documentation

## License

MIT OR Apache-2.0
