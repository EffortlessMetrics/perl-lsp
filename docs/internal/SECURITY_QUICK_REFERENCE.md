# Security Scanning - Quick Reference

> **TL;DR:** Run `just security-scan` before committing. Check GitHub Security tab for findings.

## Commands

```bash
# Comprehensive scan (recommended before PR)
just security-scan

# Quick pre-commit check
just security-quick

# Individual tools
just security-audit-strict   # cargo-audit
just security-deny           # cargo-deny
just security-trivy          # Trivy

# Reporting
just security-report         # Generate detailed reports
just security-trivy-sarif    # SARIF output for GitHub

# Docker scanning
just security-trivy-docker perl-lsp:dev
```

## CI Triggers

| Event | Jobs | Fail On |
|-------|------|---------|
| **PR** (Cargo changes) | audit, deny, trivy-repo | CRITICAL, HIGH |
| **Push to main** | audit, deny, trivy-repo | Report only |
| **Daily 2AM UTC** | All + Docker scan | Report only |
| **Manual** | All + Docker scan | Configurable |

## Severity Levels

| Level | Action | Example |
|-------|--------|---------|
| 🔴 **CRITICAL** | **Block merge** | Remote code execution, auth bypass |
| 🟠 **HIGH** | **Block merge** | Privilege escalation, data leak |
| 🟡 **MEDIUM** | **Warn** | DoS, info disclosure |
| 🟢 **LOW** | **Info** | Theoretical issues |

## GitHub Security Tab

View findings:
```
https://github.com/YOUR_ORG/perl-lsp/security/code-scanning
```

Filter by:
- Tool: `trivy-repository`, `trivy-docker`
- Severity: CRITICAL, HIGH, MEDIUM, LOW
- Status: Open, Fixed, Dismissed

## Remediation Workflow

### 1. Direct Dependency
```bash
cargo update -p vulnerable-crate
cargo test --workspace
git commit -m "security: update vulnerable-crate to fix CVE-2024-XXXXX"
```

### 2. Transitive Dependency
```bash
cargo tree -p vulnerable-crate  # Find parent
cargo update -p parent-crate
cargo test --workspace
```

### 3. No Fix Available
```yaml
# Add to deny.toml temporarily
[advisories]
ignore = [
    { id = "RUSTSEC-2024-XXXXX", reason = "No fix available; tracking in issue #123" }
]
```

### 4. False Positive
```bash
# Add to .trivyignore
echo "CVE-2024-12345  # False positive: reason, issue #123, review: 2026-06-01" >> .trivyignore
```

## Tools Installation

```bash
# Rust tools (auto-installed by justfile)
cargo install cargo-audit --locked
cargo install cargo-deny --locked

# Trivy (platform-specific)
# macOS
brew install aquasecurity/trivy/trivy

# Linux (Debian/Ubuntu)
wget -qO - https://aquasecurity.github.io/trivy-repo/deb/public.key | sudo apt-key add -
echo "deb https://aquasecurity.github.io/trivy-repo/deb $(lsb_release -sc) main" | sudo tee -a /etc/apt/sources.list.d/trivy.list
sudo apt-get update && sudo apt-get install trivy

# Windows (Chocolatey)
choco install trivy
```

## Configuration Files

| File | Purpose |
|------|---------|
| `.github/workflows/security-scan.yml` | CI workflow |
| `deny.toml` | cargo-deny policy |
| `.trivyignore` | False positive management |
| `SECURITY.md` | Vulnerability disclosure policy |

## Support

- 📖 **Full Guide**: `docs/SECURITY_SCANNING.md`
- 📋 **Summary**: `docs/SECURITY_INTEGRATION_SUMMARY.md`
- 🔒 **Policy**: `SECURITY.md`
- 🐛 **Issues**: GitHub Issues with `security` label

## Response SLA

| Severity | Response | Fix Target |
|----------|----------|------------|
| CRITICAL | 24 hours | 48 hours |
| HIGH | 48 hours | 1 week |
| MEDIUM | 1 week | 1 month |
| LOW | 1 month | Next release |

---

**Last Updated:** 2026-01-28
**Quick Links:** [Security Tab](https://github.com/YOUR_ORG/perl-lsp/security) | [Advisories](https://rustsec.org/)
