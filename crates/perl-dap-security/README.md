# perl-dap-security

Security validation microcrate for the Perl DAP ecosystem.

This crate has one responsibility: enforce input and path safety policies used by
`perl-dap` and related debugger components.

## Responsibilities

- Workspace-bounded path validation (path traversal hardening)
- Expression and condition input validation (protocol-injection guardrails)
- Timeout clamping policy for request/resource protection

## Usage

```rust
use perl_dap_security::{validate_expression, validate_path, validate_timeout};
use std::path::Path;

# fn main() -> anyhow::Result<()> {
validate_expression("$x + 1")?;
let _safe = validate_path(Path::new("lib/Foo.pm"), Path::new("/workspace"))?;
assert_eq!(validate_timeout(0), 1);
# Ok(())
# }
```
