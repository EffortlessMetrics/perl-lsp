# Security Policy

## Supported Versions

| Version | Supported |
| ------- | --------- |
| 0.11.x  | Yes -- current release line |
| 0.10.x  | Security fixes only |
| < 0.10  | No |

## Reporting a Vulnerability

**Please do not open public issues for security vulnerabilities.**

We prefer reports through [GitHub Security Advisories](https://github.com/EffortlessMetrics/perl-lsp/security/advisories/new). If that is not possible, email the maintainers listed in the root `Cargo.toml`.

### What to include

- Description of the vulnerability
- Steps to reproduce
- Potential impact assessment
- Suggested fix, if you have one

### Response timeline

| Milestone | Target |
|-----------|--------|
| Acknowledge report | 48 hours |
| Severity assessment | 72 hours |
| Fix for critical/high | 30 days |
| Coordinated disclosure | After fix is released |

We follow coordinated disclosure. You will be notified before any public advisory, and we will credit you unless you prefer to remain anonymous.

## Scope

The following are considered security issues for this project:

- **Path traversal** in file operations (e.g., workspace file access escaping configured roots)
- **Arbitrary code execution** triggered by LSP protocol messages or parsed input
- **LSP protocol injection** (malformed messages causing unintended server behavior)
- **Denial of service** through crafted input that causes unbounded resource consumption
- **Dependency vulnerabilities** in crates shipped as part of perl-lsp

### Out of scope

- Bugs in the Perl source code being analyzed (perl-lsp is a read-only tool; it does not execute Perl)
- Editor or client-side issues (report those to the editor/extension maintainer)
- Vulnerabilities that require local shell access beyond what the LSP client already provides

## Security Practices

- All production code is written in safe Rust. Fatal constructs (`unwrap`, `panic!`, `process::abort`) are banned outside of tests.
- Dependencies are audited with `cargo-audit` and `cargo-deny`; see `deny.toml` for policy.
- Fuzz testing covers the parser and lexer.
- LSP communication uses stdio by default (no network listener).
- File system access is limited to workspace roots configured by the client.

---

*Last updated: 2026-03-11*
