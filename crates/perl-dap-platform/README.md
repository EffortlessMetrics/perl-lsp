# perl-dap-platform

[![Crates.io](https://img.shields.io/crates/v/perl-dap-platform.svg)](https://crates.io/crates/perl-dap-platform)
[![Documentation](https://docs.rs/perl-dap-platform/badge.svg)](https://docs.rs/perl-dap-platform)

Cross-platform runtime helpers for Perl DAP process setup.

## When to use this crate

Use `perl-dap-platform` when you need the platform-specific pieces of Perl DAP
launch/attach setup without pulling in the whole debugger runtime.

It covers three common problems:

- finding the `perl` executable on `PATH`
- normalizing platform-specific file paths for debugging
- building the environment map used to launch Perl with `PERL5LIB`

## Quick example

```rust
use perl_dap_platform::{normalize_path, setup_environment};
use std::path::PathBuf;

let env = setup_environment(&[PathBuf::from("lib"), PathBuf::from("local/lib/perl5")]);
assert!(env.contains_key("PERL5LIB"));

let normalized = normalize_path(std::path::Path::new("/mnt/c/work/script.pl"));
assert!(!normalized.as_os_str().is_empty());
```

## Public API

- `resolve_perl_path`: locate `perl` on the current `PATH`
- `normalize_path`: normalize Windows, WSL, and Unix path shapes
- `setup_environment`: construct debugger launch environment variables

## Workspace role

This is a small utility crate inside the `perl-dap` family. It is useful on its
own for tooling that needs Perl path handling, but it is primarily a support
crate for the debugger runtime.
