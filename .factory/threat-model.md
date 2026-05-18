# Security Threat Model for perl-lsp

**Generated:** 2026-05-18  
**Repository:** EffortlessMetrics/perl-lsp  
**Version:** 1.0

---

## Executive Summary

perl-lsp is a Language Server Protocol (LSP) implementation for Perl written in Rust. The system processes user-controlled input (Perl source files, workspace configuration) and executes external Perl processes for linting and formatting.

### Trust Boundaries

1. **External → LSP Protocol**: Network-facing LSP client connections
2. **Workspace Filesystem**: User-controlled source files read from disk
3. **External Tools**: Perl interpreter, perlcritic, perltidy invocations
4. **Configuration**: .perl-lsp/config.json and editor settings

---

## System Architecture

### Core Components

| Component | Path | Trust Level | Notes |
|-----------|------|-------------|-------|
| LSP Server | `crates/perl-lsp-rs/` | High | Entry point, protocol handling |
| Input Validation | `crates/perl-lsp-rs-core/src/runtime/input_validation/` | High | All external data passes through here |
| Sandbox | `crates/perl-lsp-rs/src/security/sandbox.rs` | Critical | Process isolation |
| Semantic Analyzer | `crates/perl-semantic-analyzer/` | Medium | Analysis only, no execution |
| Parser | `crates/perl-parser/` | Medium | Parsing only, no execution |

### Data Flow

```
LSP Client (VSCode/Editor)
    ↓ JSON-RPC over stdio/TCP
perl-lsp Server
    ↓ validate_lsp_request()
Input Validation Layer
    ↓ validate_file_path() / validate_file_content()
Workspace Filesystem
    ↓
perl-lsp Core
    ↓ [optional] SafeExecutor.execute_perl_script()
Sandbox (firejail/sandbox-exec/Windows Job Objects)
    ↓
External Perl Tools (perl, perlcritic, perltidy)
```

---

## STRIDE Threat Analysis

### S - Spoofing

| Threat | Likelihood | Impact | Mitigation |
|--------|------------|--------|------------|
| LSP client impersonation | Low | High | TLS/client certificates (deployed at network level) |
| Workspace root spoofing | Medium | Medium | Path validation via `validate_workspace_path()` |

### T - Tampering

| Threat | Likelihood | Impact | Mitigation |
|--------|------------|--------|------------|
| Path traversal via workspace path | Low | High | `validate_workspace_path()` with canonicalization |
| Path traversal via URI (file:///) | Low | High | URI scheme allowlist in `validate_lsp_request()` |
| Content injection via file content | Medium | High | File extension allowlist, suspicious pattern detection |
| Configuration injection | Low | Medium | JSON Schema validation |
| Git output manipulation | Low | Low | Sandboxed git operations |

### R - Repudiation

| Threat | Likelihood | Impact | Mitigation |
|--------|------------|--------|------------|
| No audit logging | Medium | Medium | tracing-based structured logs |
| Command execution without confirmation | Low | Medium | Sandbox warnings, user configuration |

### I - Information Disclosure

| Threat | Likelihood | Impact | Mitigation |
|--------|------------|--------|------------|
| Path traversal file read | Low | Medium | Path validation prevents directory traversal |
| Error message disclosure | Medium | Low | Error messages sanitized in production |
| Memory exposure via panic | Low | Medium | Panic handlers use safe error returns |
| Workspace file content in logs | Low | Low | Content sanitization before logging |

### D - Denial of Service

| Threat | Likelihood | Impact | Mitigation |
|--------|------------|--------|------------|
| Large workspace DoS | Medium | Medium | `max_file_size_bytes()` limit, workspace scanning limits |
| Algorithmic complexity attack | Low | Medium | Time limits on operations, workspace index limits |
| LSP message flooding | Medium | Medium | `max_notification_buffer_size`, request throttling |
| Infinite loop in Perl code | Medium | Medium | Timeout on subprocess execution |
| XML BOM in source files | Low | Low | Detected and handled in parser |

### E - Elevation of Privilege

| Threat | Likelihood | Impact | Mitigation |
|--------|------------|--------|------------|
| Arbitrary command execution | Low | Critical | Sandbox with firejail/sandbox-exec/Job Objects |
| Sandbox escape | Low | Critical | Fail-closed if sandbox unavailable |
| Taint mode bypass | Low | High | Perl -T flag always used |
| Configuration override | Low | Medium | Schema validation on config files |

---

## Security Controls Inventory

### Input Validation

- **Path Validation**: `validate_workspace_path()` with canonicalization prevents `../` traversal
- **URI Scheme Allowlist**: Only `file://`, `untitled:`, `opencode:` schemes permitted
- **File Extension Allowlist**: Only `.pm`, `.pl`, `.pod`, `.t`, `.tml`, `.tt` allowed
- **Content Size Limits**: 1MB max file size, 100,000 char max line length
- **Suspicious Pattern Detection**: Script tags, null bytes, control characters
- **Method/Command Allowlist**: Only whitelisted LSP methods and commands

### Sandboxing

- **Linux**: firejail with `--net=none`, `--private-tmp`, memory/CPU limits
- **macOS**: sandbox-exec with explicit profile, deny network by default
- **Windows**: Job Objects with sandbox.enabled=false fallback (fails closed)
- **Fail-Closed**: Returns error if sandbox unavailable rather than running unsandboxed

### Process Execution

- **Perl Taint Mode**: All Perl scripts run with `-T` flag
- **Timeout Protection**: Configurable CPU time limits (default 30s)
- **Memory Limits**: 512MB default
- **Environment Isolation**: Only PATH, HOME, TMPDIR passed to sandboxed processes

---

## Key Security Files

| File | Purpose |
|------|---------|
| `crates/perl-lsp-rs/src/security/sandbox.rs` | Process isolation |
| `crates/perl-lsp-rs/src/security/validation.rs` | Input validation facade |
| `crates/perl-lsp-rs-core/src/runtime/input_validation/mod.rs` | Validation implementation |
| `crates/perl-lsp-rs-core/src/runtime/input_validation/file_validation.rs` | Path/content validation |
| `crates/perl-lsp-rs-core/src/runtime/input_validation/lsp_validation.rs` | LSP request validation |
| `crates/perl-lsp-rs-core/src/runtime/limits/mod.rs` | Resource limits |

---

## Recommendations

### High Priority

1. **Continue fail-closed sandbox policy** - Critical security control
2. **Maintain Perl taint mode** - Defense in depth for Perl execution
3. **Keep path canonicalization** - Prevents all path traversal variants

### Medium Priority

1. **Add security audit logging** - Improves repudiation posture
2. **Document sandbox configuration** - Helps users understand protections
3. **Rate limiting on LSP requests** - DoS protection improvement

### Low Priority

1. **Consider memory-hard sandboxing** - Defense in depth
2. **Network denylist option** - For air-gapped environments
3. **Security event aggregation** - Centralized security monitoring

---

## Threat Prioritization Summary

| Severity | Count | Priority Threats |
|----------|-------|-------------------|
| Critical | 2 | Sandbox escape, arbitrary command execution |
| High | 3 | Path traversal, sandbox unavailable bypass, taint bypass |
| Medium | 5 | Path traversal (workspace), DoS (workspace size), error disclosure |
| Low | 4 | Git output manipulation, log disclosure, XML BOM DoS |

**Overall Security Verdict**: GOOD - Defense in depth with fail-closed design. Multiple overlapping controls for critical paths.

---

*Last Updated: 2026-05-18*
