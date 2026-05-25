# Security Scan Report

**Generated:** 2026-05-25
**Scan Type:** Weekly Scheduled
**Repository:** EffortlessMetrics/perl-lsp
**Severity Threshold:** medium

## Executive Summary

| Severity | Count | Auto-fixed | Manual Required |
|----------|-------|------------|-----------------|
| CRITICAL | 0 | 0 | 0 |
| HIGH | 0 | 0 | 0 |
| MEDIUM | 0 | 0 | 0 |
| LOW | 0 | 0 | 0 |

**Total Findings:** 0
**Auto-fixed:** 0
**Manual Review Required:** 0

## Scan Details

### Commit Analyzed
- **SHA:** 27e837c
- **Message:** workspace: gate eager indexing on initialized (#9570)
- **Author:** Steven Zimmerman, CPA
- **Date:** 2026-05-23

### Scope
Files changed in the last 7 days were scanned for STRIDE vulnerabilities:
- Spoofing
- Tampering
- Repudiation
- Information Disclosure
- Denial of Service
- Elevation of Privilege

### Analysis Performed
The scan focused on:
1. Runtime initialization and workspace indexing logic
2. Thread-safety and concurrency controls
3. Path traversal prevention
4. User input validation
5. Environment variable handling

### Key Finding
The analyzed commit implements a feature gate for eager workspace indexing. The change adds a predicate check (`should_start_workspace_indexing()`) that consults `RuntimeTuning.eager_workspace_indexing` before calling `start_workspace_indexing()`. 

Security analysis confirms:
- Proper concurrency controls via `AtomicBool` and RAII `IndexingGuard`
- Path validation via `validate_workspace_path()` in perl-parser-core
- Timeout enforcement on indexing operations
- No user-controllable injection points introduced

**No security vulnerabilities were found in the scanned code.**

## Appendix

### Threat Model
- **Version:** 2026-05-25 (newly generated)
- **Location:** .factory/threat-model.md
- **Assessment:** LOW risk - Multiple defense layers implemented

### Threat Model Summary

| STRIDE Category | Risk Level |
|-----------------|------------|
| Spoofing | LOW |
| Tampering | LOW (mitigated) |
| Repudiation | MEDIUM (recommendations only) |
| Information Disclosure | LOW |
| Denial of Service | LOW (mitigated) |
| Elevation of Privilege | LOW (mitigated) |

### Scan Metadata
- **Commits Scanned:** 1
- **Files Analyzed:** ~8800 (workspace-wide)
- **Source Files Reviewed:** Active crates only (perl-lsp-rs, perl-workspace, perl-dap)
- **Skills Used:** threat-model-generation, commit-security-scan, vulnerability-validation

### Key Security Controls in Place
1. Path traversal prevention via `validate_workspace_path()`
2. Sandboxing: Firejail (Linux), sandbox-exec (macOS)
3. Perl Taint Mode (-T) on script executions
4. Input validation: `validate_lsp_request()`, `validate_expression()`, `validate_path()`
5. Binary validation for Perl interpreter paths
6. Timeout enforcement (MAX_TIMEOUT_MS=300s, DEFAULT_TIMEOUT_MS=5s)
7. Security context tracking for violations

### Recommendations
1. Add structured audit logging for all LSP requests
2. Implement request correlation IDs for traceability
3. Consider adding rate limiting at the transport layer
4. Regular dependency updates via Dependabot (already configured)

### References
- [CWE Database](https://cwe.mitre.org/)
- [STRIDE Threat Model](https://docs.microsoft.com/en-us/azure/security/develop/threat-modeling-tool-threats)
- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
