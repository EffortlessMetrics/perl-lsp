# perl-dap-shell

[![Crates.io](https://img.shields.io/crates/v/perl-dap-shell.svg)](https://crates.io/crates/perl-dap-shell)
[![Documentation](https://docs.rs/perl-dap-shell/badge.svg)](https://docs.rs/perl-dap-shell)

Shell-oriented launch helpers for Perl DAP.

## When to use this crate

Use `perl-dap-shell` when you need to prepare command-line arguments or launch
environment state for the DAP process layer.

It is useful when you want to:

- format shell arguments safely for launch paths
- keep shell-specific quoting separate from path normalization
- build the process inputs consumed by `perl-dap`

## Quick example

```rust
use perl_dap_shell::{format_command_args, setup_environment};
use std::path::PathBuf;

let args = format_command_args(&["file with spaces.pl".into()]);
assert_eq!(args.len(), 1);

let env = setup_environment(&[PathBuf::from("lib")]);
assert!(env.contains_key("PERL5LIB"));
```

## Public API

- `format_command_args`: shell-quote launch arguments where needed
- `setup_environment`: re-exported platform helper for `PERL5LIB`

## Workspace role

This crate keeps shell-specific concerns out of `perl-dap-platform`. Use it for
launch-path assembly; use the platform crate for path discovery and normalization.
