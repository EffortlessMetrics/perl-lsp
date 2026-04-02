# perl-dap-platform

Cross-platform helpers for finding Perl and shaping the debugger launch
environment.

This crate sits below the debugger runtime and above shell quoting. It handles
the OS-specific parts of launch setup: finding the Perl executable, normalizing
filesystem paths, and building `PERL5LIB`-style environment maps.

## Boundaries

- Use `perl-dap-shell` when you need shell-safe argument formatting.
- Use `perl-dap-platform` when you need to resolve the executable or normalize
  OS paths.
- Use `perl-dap` when you want the server to actually launch and manage a
  debug session.

## Key API

- `resolve_perl_path`
- `normalize_path`
- `setup_environment`

## Example

```rust
use perl_dap_platform::{normalize_path, setup_environment};
use std::path::PathBuf;

let normalized = normalize_path(&PathBuf::from("/mnt/c/work/script.pl"));
let env = setup_environment(&[PathBuf::from("/workspace/lib")]);

assert!(normalized.to_string_lossy().len() > 0);
assert_eq!(env.get("PERL5LIB").map(String::as_str), Some("/workspace/lib"));
```
