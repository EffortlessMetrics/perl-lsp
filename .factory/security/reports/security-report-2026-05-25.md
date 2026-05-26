# Security Scan Report

**Generated:** 2026-05-25  
**Scan Type:** Weekly Scheduled  
**Repository:** EffortlessMetrics/perl-lsp-swarm  
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

### Commits Scanned

- `ca4f295` - docs(swarm): refresh provider proof assignment gate (#8197) (#417)

### Changed Files Analysis

The weekly scan examined changes from 1 commit containing 8876 files changed. These were primarily:
- Configuration files (`.ci/`, `.cargo/`, `.config/`)
- Public API baselines
- Metrics and status files
- Agent definitions and command documentation

### Security Controls Verified

The scan verified the following security controls are in place:

| Control | Status | Location |
|---------|--------|----------|
| Path Validation | Active | perl-parser-core/src/syntax/path_security.rs |
| Transport Framing | Active | perl-lsp-rs-core/src/transport/framing.rs |
| DAP Security | Active | perl-dap/src/security/mod.rs |
| Subprocess Sandboxing | Active | perl-lsp-rs/src/security/sandbox.rs |
| Frame Size Limits | Active | MAX_FRAME_SIZE = 16MB |
| File Content Limits | Active | 1MB max file content |

## Threat Model Status

| Item | Status |
|------|--------|
| Threat Model | Created/Updated |
| Version | 2026-05-25 |
| Total Threats Documented | 24 |
| STRIDE Categories Covered | 6/6 |

## Findings by STRIDE Category

| Category | Threats | Severity Distribution |
|----------|---------|----------------------|
| Spoofing | 3 | Medium (1), High (2) |
| Tampering | 4 | Critical (1), High (3) |
| Repudiation | 2 | Low (1), Medium (1) |
| Information Disclosure | 4 | Critical (1), Medium (2), Low (1) |
| Denial of Service | 5 | Critical (2), High (2), Medium (1) |
| Elevation of Privilege | 6 | Critical (2), High (3), Medium (1) |

## Recommendations

### Priority P0 (Immediate)

No critical exploitable vulnerabilities were found in this week's changes. The existing security controls are functioning as designed.

### General Security Best Practices

1. **Continue using taint mode (-T)** for all Perl subprocess execution
2. **Maintain path canonicalization** with symlink detection
3. **Enforce command allowlisting** via `get_supported_commands()`
4. **Monitor for anomaly detection** on large file submissions

## Appendix

### Threat Model

| Item | Value |
|------|-------|
| Version | 2026-05-25 |
| Location | .factory/threat-model.md |
| Total Threats | 24 |
| P0 Priority | 4 |
| P1 Priority | 7 |
| P2 Priority | 8 |
| P3 Priority | 5 |

### Scan Metadata

| Item | Value |
|------|-------|
| Commits Scanned | 1 |
| Files Changed | 8876 |
| Scan Duration | ~5 minutes |
| Skills Used | threat-model-generation, commit-security-scan, vulnerability-validation |

### Security Controls Verification

All security controls documented in the threat model are implemented and active:

- Path validation with canonicalization and symlink detection
- 16MB frame size limit on LSP transport
- 1MB max file content
- DAP expression sanitization
- Command allowlist enforcement
- Subprocess sandboxing (firejail/sandbox-exec)
- Perl taint mode (-T) enabled

### References

- [CWE Database](https://cwe.mitre.org/)
- [STRIDE Threat Model](https://docs.microsoft.com/en-us/azure/security/develop/threat-modeling-tool-threats)
- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
