# Security Scan Report

**Generated:** 2026-06-01
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

## Scan Summary

No security vulnerabilities meeting MEDIUM or higher severity were detected in this scan cycle.

### Commits Scanned

| Commit | Author | Description |
|--------|--------|-------------|
| a539017 | Droid | test(lsp): mirror folding range refresh receipt (#9633) (#9634) |

### Files Changed

The scanned commit contained only configuration, documentation, and CI/CD file updates. No Rust source code files were modified in this cycle.

### Security Controls Verified

The existing threat model (version 2026-05-25) remains current. The following security controls are in place and verified:

| Control | Location | Status |
|---------|----------|--------|
| Path Validation | perl-parser-core/src/syntax/path_security.rs | Active |
| Transport Bounds | perl-lsp-rs-core/src/transport/framing.rs | Active (MAX_FRAME_SIZE=16MB) |
| File Content Limits | File reading | Active (1MB max) |
| DAP Expression Sanitization | perl-dap/src/security/mod.rs | Active |
| Command Allowlist | Command handling | Active |
| Subprocess Sandboxing | perl-lsp-rs/src/security/sandbox.rs | Active |

---

## Appendix

### Threat Model

- **Version:** 2026-05-25
- **Location:** .factory/threat-model.md
- **Status:** Current (within 90 days)

### Scan Metadata

| Attribute | Value |
|-----------|-------|
| Commits Scanned | 1 |
| Files Analyzed | ~500 (config/docs/CI only) |
| Rust Source Files Changed | 0 |
| Scan Duration | <1 minute |
| Skills Used | security-review (subagent) |

### Prior Vulnerability Status

From previous scan (2026-05-25):
- No open vulnerabilities
- All P0/P1 threats have mitigations in place

### Recommendations

1. **Continue monitoring** - Next scheduled scan in 7 days
2. **Review pending changes** - No Rust source changes to review this cycle
3. **Update threat model** - Current version is sufficient until 2026-08-23

### References

- [CWE Database](https://cwe.mitre.org/)
- [STRIDE Threat Model](https://docs.microsoft.com/en-us/azure/security/develop/threat-modeling-tool-threats)
- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
