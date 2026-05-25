# Security Threat Model for perl-lsp-swarm

**Version:** 2026-05-25  
**Methodology:** STRIDE  
**Repository:** EffortlessMetrics/perl-lsp-swarm

## Overview

perl-lsp-swarm is a Rust-based LSP (Language Server Protocol) server implementation for Perl. The threat model covers the primary attack surfaces across 134+ workspace crates.

## Attack Surfaces

| Surface | Components | Risk Level |
|---------|------------|------------|
| LSP Protocol | perl-lsp-rs-core/src/transport/framing.rs | High |
| File System | perl-parser-core/src/syntax/path_security.rs | Critical |
| DAP Protocol | perl-dap/src/security/mod.rs | High |
| Subprocesses | perl-lsp-rs/src/security/sandbox.rs | Critical |
| Workspace Index | Symbol resolution, file indexing | Medium |

## Threat Register

### S - Spoofing (Identity Impersonation)

| ID | Threat | Component | Severity | Mitigation |
|----|--------|-----------|----------|------------|
| T001 | LSP client identity impersonation | LSP transport | Medium | Verify client capabilities, validate method requests |
| T002 | File URI validation bypass | URI parsing | High | Canonicalization + symlink detection in path_security.rs |
| T003 | Symbol resolution to external paths | Workspace index | Medium | Path containment validation |

### T - Tampering (Code/Data Modification)

| ID | Threat | Component | Severity | Mitigation |
|----|--------|-----------|----------|------------|
| T004 | Parser buffer overflow | perl-parser | Critical | MAX_FRAME_SIZE=16MB, content limits |
| T005 | Configuration injection | Config parsing | High | Schema validation, sanitization |
| T006 | File write via applyEdit | LSP applyEdit | High | Path validation, sandboxing |
| T007 | DAP message injection | Debug protocol | High | Expression validation (blocking newlines) |

### R - Repudiation

| ID | Threat | Component | Severity | Mitigation |
|----|--------|-----------|----------|------------|
| T008 | Missing audit trail | Operations | Medium | Structured logging of file operations |
| T009 | Non-repudiation of edits | applyEdit | Low | Operation receipts |

### I - Information Disclosure

| ID | Threat | Component | Severity | Mitigation |
|----|--------|-----------|----------|------------|
| T010 | Path exposure in diagnostics | Diagnostics | Medium | Path redaction in errors |
| T011 | Memory safety issues | Parser/AST | Critical | Rust memory safety, bounds checking |
| T012 | Workspace content leak | Symbol resolution | Medium | Access control on workspace boundaries |
| T013 | Error verbosity | Error handling | Low | sanitize_diagnostic_message() |

### D - Denial of Service

| ID | Threat | Component | Severity | Mitigation |
|----|--------|-----------|----------|------------|
| T014 | Parser runaway loops | perl-parser | Critical | Timeout enforcement, iteration limits |
| T015 | Memory exhaustion via large files | File reading | High | 1MB max file content, streaming |
| T016 | Infinite recursion in symbol resolution | Workspace | Medium | Cycle detection, depth limits |
| T017 | ReDoS (Regular Expression DoS) | Regex parsing | High | Regex timeouts, non-backtracking patterns |
| T018 | Message flood | LSP transport | Medium | MAX_FRAME_SIZE=16MB, rate limiting |

### E - Elevation of Privilege

| ID | Threat | Component | Severity | Mitigation |
|----|--------|-----------|----------|------------|
| T019 | Path traversal | File operations | Critical | path_security.rs canonicalization |
| T020 | Unsafe file: URI handling | URI parsing | High | Scheme validation |
| T021 | Symlink escape from sandbox | File access | High | Symlink detection in path_security.rs |
| T022 | Command injection | Subprocess execution | Critical | Command allowlist via get_supported_commands() |
| T023 | Module load injection | Perl loading | High | Taint mode (-T), sandboxing |
| T024 | DAP breakpoint injection | Debug adapter | Medium | Expression validation |

## Security Controls Summary

| Control | Location | Description |
|---------|----------|-------------|
| Path Validation | perl-parser-core/src/syntax/path_security.rs | Canonicalization with symlink detection |
| Transport Bounds | perl-lsp-rs-core/src/transport/framing.rs | MAX_FRAME_SIZE = 16MB |
| File Content Limits | File reading | 1MB max content |
| DAP Expression Sanitization | perl-dap/src/security/mod.rs | Block newlines/carriage returns |
| Command Allowlist | Command handling | get_supported_commands() |
| Subprocess Sandboxing | perl-lsp-rs/src/security/sandbox.rs | firejail/sandbox-exec with Perl -T |

## Threat Prioritization

| Priority | Threats | Action |
|----------|---------|--------|
| P0 | T004, T014, T019, T022 | Immediate mitigation required |
| P1 | T002, T006, T007, T015, T017, T021 | Next sprint |
| P2 | T001, T008, T010, T012, T016, T018, T023, T024 | Backlog |
| P3 | T009, T013 | Low priority |

## Security Testing Coverage

- **SAST**: cargo clippy with security-focused lints
- **Fuzzing**: Bound fuzz targets for parser and transport
- **Integration**: Security-specific tests in perl-dap/src/security/

## Review History

| Date | Version | Reviewer | Changes |
|------|---------|----------|---------|
| 2026-05-25 | 1.0 | Droid Security Scan | Initial threat model |

## References

- [STRIDE Threat Model](https://docs.microsoft.com/en-us/azure/security/develop/threat-modeling-tool-threats)
- [CWE Database](https://cwe.mitre.org/)
- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
