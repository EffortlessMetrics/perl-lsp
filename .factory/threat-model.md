# Security Threat Model - perl-lsp

**Generated:** 2026-05-25  
**Repository:** EffortlessMetrics/perl-lsp  
**Model Type:** STRIDE-based

## Overview

perl-lsp is a Language Server Protocol (LSP) implementation for Perl, providing IDE features like auto-completion, goto definition, and diagnostics. The server communicates via JSON-RPC over stdio or TCP.

## Core Components

| Component | Purpose | Risk Profile |
|-----------|---------|--------------|
| `perl-lsp-rs` | LSP server binary | HIGH - network-facing |
| `perl-dap` | Debug Adapter Protocol | HIGH - process spawning |
| `perl-parser` | Perl recursive descent parser | MEDIUM - file parsing |
| `perl-workspace` | Workspace symbol indexing | MEDIUM - file system access |
| `perl-semantic-analyzer` | Semantic analysis | LOW - in-memory processing |

## STRIDE Analysis

### Spoofing (Identity Impersonation)

| Threat | Likelihood | Impact | Mitigation |
|--------|------------|--------|------------|
| Malicious LSP client connecting to server | LOW | HIGH | Editor-mediated connections |
| TCP socket hijacking | LOW | HIGH | Local socket only |
| URI scheme spoofing in file paths | MEDIUM | MEDIUM | Path validation in `validate_workspace_path()` |

### Tampering (Data Modification)

| Threat | Likelihood | Impact | Mitigation |
|--------|------------|--------|------------|
| Path traversal via malicious file paths | MITIGATED | HIGH | `validate_workspace_path()` in perl-parser-core |
| External tool injection via LSP requests | MITIGATED | CRITICAL | Path validation, SafeExecutor |
| DAP payload tampering | MITIGATED | HIGH | Expression validation in `eval/validator.rs` |

### Repudiation (Denial of Actions)

| Threat | Likelihood | Impact | Status |
|--------|------------|--------|--------|
| Lack of audit logging | MEDIUM | MEDIUM | Recommendation: Add structured request logging |
| No request correlation IDs | MEDIUM | LOW | Recommendation: Add correlation to all LSP requests |

### Information Disclosure (Data Exposure)

| Threat | Likelihood | Impact | Mitigation |
|--------|------------|--------|------------|
| File enumeration via workspace symbols | LOW | LOW | Scoped to workspace roots |
| Verbose error messages exposing paths | LOW | LOW | Error message sanitization |
| API key or credential exposure | LOW | CRITICAL | No storage of secrets |

### Denial of Service (Availability)

| Threat | Likelihood | Impact | Mitigation |
|--------|------------|--------|------------|
| Large file parsing | MITIGATED | MEDIUM | Timeout enforcement (MAX_TIMEOUT_MS=300s) |
| ReDoS (Regular Expression DoS) | MITIGATED | MEDIUM | Bounded regex operations |
| Parse storms via rapid requests | MITIGATED | MEDIUM | Request throttling |
| Memory exhaustion | MITIGATED | HIGH | Memory limits in corpus handling |

### Elevation of Privilege (Authorization Bypass)

| Threat | Likelihood | Impact | Mitigation |
|--------|------------|--------|------------|
| Arbitrary command execution via tools | MITIGATED | CRITICAL | Taint mode (-T), path validation |
| DAP evaluate() arbitrary code | MITIGATED | CRITICAL | SafeExecutor with restricted eval |
| File system access outside workspace | MITIGATED | HIGH | `validate_workspace_path()` boundary checks |

## Key Security Controls

1. **Path Traversal Prevention**: `perl-parser-core::path_security::validate_workspace_path()`
2. **Sandboxing**: Firejail (Linux), sandbox-exec (macOS), fail-closed (Windows)
3. **Perl Taint Mode**: `-T` flag on all script executions
4. **Input Validation**: `validate_lsp_request()`, `validate_expression()`, `validate_path()`
5. **Binary Validation**: `is_valid_perl_interpreter()` for tool paths
6. **Timeout Enforcement**: MAX_TIMEOUT_MS (300s), DEFAULT_TIMEOUT_MS (5s)
7. **Security Context**: In-memory violation tracking via `SecurityContext`

## Risk Summary

| Category | Overall Risk |
|----------|--------------|
| Spoofing | LOW |
| Tampering | LOW (mitigated) |
| Repudiation | MEDIUM (recommendations only) |
| Information Disclosure | LOW |
| Denial of Service | LOW (mitigated) |
| Elevation of Privilege | LOW (mitigated) |

**Overall Assessment: LOW** - The server implements multiple defense layers and follows secure-by-default principles.

## Recommendations

1. Add structured audit logging for all LSP requests
2. Implement request correlation IDs for traceability
3. Consider adding rate limiting at the transport layer
4. Regular dependency updates via Dependabot (already configured)
