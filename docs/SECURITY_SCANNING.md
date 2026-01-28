# Security Scanning Guide

This document describes the security scanning infrastructure for the Perl LSP project, including tools, workflows, and remediation procedures.

## Table of Contents

- [Overview](#overview)
- [Scanning Tools](#scanning-tools)
- [Security Workflow](#security-workflow)
- [Severity Thresholds](#severity-thresholds)
- [Local Security Scanning](#local-security-scanning)
- [CI Integration](#ci-integration)
- [Remediation Guidelines](#remediation-guidelines)
- [False Positive Management](#false-positive-management)
- [Security Policy](#security-policy)

## Overview

The project uses a multi-layered security scanning approach:

1. **Cargo Audit** - RustSec advisory database for known Rust vulnerabilities
2. **Cargo Deny** - License compliance and dependency policy enforcement
3. **Trivy** - Comprehensive vulnerability scanning for dependencies and containers
4. **GitHub Security Tab** - Centralized vulnerability tracking via SARIF reports

### Security Scanning Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Security Scanning Pipeline                │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  1. Rust Dependencies (cargo-audit, cargo-deny)             │
│     └─> RustSec Advisory Database                           │
│     └─> License & Ban Policy Enforcement                    │
│                                                               │
│  2. Comprehensive Scanning (Trivy)                          │
│     └─> Filesystem Scan (Cargo.lock, configs)              │
│     └─> Docker Image Scan (containers)                      │
│     └─> Secret Detection                                    │
│                                                               │
│  3. Reporting (SARIF format)                                │
│     └─> GitHub Security Tab Integration                     │
│     └─> PR Comments with Summaries                          │
│     └─> Artifact Storage (30-day retention)                 │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

## Scanning Tools

### 1. Cargo Audit

**Purpose:** Check Rust dependencies against the RustSec Advisory Database

**Installation:**
```bash
cargo install cargo-audit --locked
```

**Usage:**
```bash
# Basic audit
cargo audit

# JSON output for processing
cargo audit --json

# Fail on warnings
cargo audit --deny warnings
```

**Configuration:** None required (uses Cargo.lock automatically)

### 2. Cargo Deny

**Purpose:** Enforce dependency policies (licenses, bans, sources)

**Installation:**
```bash
cargo install cargo-deny --locked
```

**Usage:**
```bash
# Check all policies
cargo deny check

# Check specific policy
cargo deny check advisories
cargo deny check licenses
cargo deny check bans
cargo deny check sources
```

**Configuration:** See `deny.toml` in project root

Key policies enforced:
- **Licenses:** MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Unicode-3.0
- **Sources:** Only crates.io registry allowed
- **Advisories:** All RustSec advisories reported

### 3. Trivy

**Purpose:** Comprehensive vulnerability scanning (dependencies, containers, secrets)

**Installation:**
```bash
# macOS
brew install aquasecurity/trivy/trivy

# Linux
wget -qO - https://aquasecurity.github.io/trivy-repo/deb/public.key | sudo apt-key add -
echo "deb https://aquasecurity.github.io/trivy-repo/deb $(lsb_release -sc) main" | sudo tee -a /etc/apt/sources.list.d/trivy.list
sudo apt-get update
sudo apt-get install trivy

# Windows (via Chocolatey)
choco install trivy
```

**Usage:**
```bash
# Scan filesystem (Rust dependencies)
trivy fs --severity CRITICAL,HIGH,MEDIUM .

# Scan with SARIF output
trivy fs --format sarif --output trivy-results.sarif .

# Scan Docker image
docker build -t perl-lsp:dev -f .docker/rust/Dockerfile .
trivy image --severity CRITICAL,HIGH perl-lsp:dev

# Ignore unfixed vulnerabilities
trivy fs --ignore-unfixed .

# Secret scanning
trivy fs --scanners secret .
```

**Scan Types:**
- `vuln` - Vulnerability scanning
- `config` - Configuration issues (IaC)
- `secret` - Hardcoded secrets detection

## Security Workflow

### Automated Scans

Security scans run automatically on:

1. **Pull Requests** - On changes to:
   - `Cargo.toml`, `Cargo.lock`
   - Any `**/Cargo.toml`
   - Docker files (`.docker/**`, `Dockerfile*`)
   - Security workflow itself

2. **Push to main/master** - Full security scan

3. **Daily Schedule** - Comprehensive scan at 2 AM UTC

4. **Manual Dispatch** - On-demand via GitHub Actions

### Workflow Jobs

| Job | Purpose | Fail on | Runtime |
|-----|---------|---------|---------|
| `cargo-audit` | RustSec advisories | Vulnerabilities found | ~2 min |
| `cargo-deny` | Policy enforcement | Policy violations | ~2 min |
| `trivy-repo-scan` | Filesystem scan | HIGH/CRITICAL on PR | ~5 min |
| `trivy-docker-scan` | Container scan | HIGH/CRITICAL | ~10 min |
| `security-policy` | Policy validation | Critical patterns | ~1 min |
| `security-summary` | Report aggregation | Never | ~1 min |

### GitHub Security Tab Integration

All scan results are uploaded to GitHub's Security tab in SARIF format:

```yaml
- name: Upload to GitHub Security tab
  uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: 'trivy-results.sarif'
    category: 'trivy-repository'
```

**Viewing Results:**
1. Go to: `https://github.com/YOUR_ORG/perl-lsp/security`
2. Select "Code scanning" tab
3. Filter by tool: `trivy-repository`, `trivy-docker`

## Severity Thresholds

### Severity Levels

| Severity | Definition | Action Required |
|----------|------------|-----------------|
| **CRITICAL** | Immediate threat, exploitable | **BLOCK MERGE** - Fix immediately |
| **HIGH** | Significant risk, likely exploitable | **BLOCK MERGE** - Fix before merge |
| **MEDIUM** | Moderate risk, potential exploitation | **WARN** - Review and plan fix |
| **LOW** | Minimal risk, unlikely exploitation | **INFO** - Track for future resolution |
| **NEGLIGIBLE** | Theoretical or very unlikely | **IGNORE** - No action required |

### Enforcement Policy

#### On Pull Requests:
```yaml
# Trivy fails on CRITICAL and HIGH severities
exit-code: '1'
severity: 'CRITICAL,HIGH'
```

#### On Main Branch:
```yaml
# Report all severities, don't fail (report only)
exit-code: '0'
severity: 'CRITICAL,HIGH,MEDIUM'
```

#### On Schedule:
```yaml
# Comprehensive scan, report all findings
exit-code: '0'
severity: 'CRITICAL,HIGH,MEDIUM,LOW'
```

### Ignore Unfixed Vulnerabilities

By default, we ignore unfixed vulnerabilities (no patch available):

```bash
trivy fs --ignore-unfixed .
```

**Rationale:**
- No immediate action possible
- Avoid alert fatigue
- Track in Security tab for future resolution

**Override when needed:**
```bash
trivy fs .  # Show all vulnerabilities including unfixed
```

## Local Security Scanning

### Quick Security Check

```bash
# Run all security checks locally
just security-scan

# Or manually:
cargo audit
cargo deny check
trivy fs --severity CRITICAL,HIGH .
```

### Pre-commit Security Hook

Add to `.git/hooks/pre-commit`:

```bash
#!/bin/bash
set -e

echo "Running security checks..."

# Quick cargo-audit check
if command -v cargo-audit &> /dev/null; then
    cargo audit --deny warnings || {
        echo "⚠️  Security vulnerabilities detected!"
        echo "Run 'cargo audit' for details"
        exit 1
    }
fi

echo "✅ Security checks passed"
```

Make executable:
```bash
chmod +x .git/hooks/pre-commit
```

### Interactive Security Scan

```bash
# Comprehensive local scan with interactive output
trivy fs \
  --severity CRITICAL,HIGH,MEDIUM \
  --scanners vuln,config,secret \
  --format table \
  .

# Generate HTML report
trivy fs \
  --format template \
  --template "@contrib/html.tpl" \
  --output security-report.html \
  .
```

## CI Integration

### Integration with Gate Policy

Security scanning is integrated into the merge gate (`.ci/gate-policy.yaml`):

```yaml
- name: security_audit
  tier: merge_gate
  description: "Check dependencies for known security vulnerabilities"
  required: true
  command: cargo audit --deny warnings
  timeout_seconds: 120
  tags:
    - security
    - dependencies
    - audit
```

### Adding to Justfile

```makefile
# Security scanning commands
security-scan:
    @echo "Running comprehensive security scan..."
    cargo audit --deny warnings
    cargo deny check
    trivy fs --severity CRITICAL,HIGH,MEDIUM .

security-audit:
    @echo "Running cargo-audit..."
    cargo audit

security-deny:
    @echo "Running cargo-deny..."
    cargo deny check advisories
    cargo deny check licenses
    cargo deny check bans

security-trivy:
    @echo "Running Trivy scan..."
    trivy fs --severity CRITICAL,HIGH .

security-report:
    @echo "Generating security report..."
    trivy fs --format json --output security-report.json .
    trivy fs --format sarif --output security-report.sarif .
```

### Caching Strategy

Security scans use aggressive caching:

```yaml
- name: Cache cargo dependencies
  uses: Swatinem/rust-cache@v2
  with:
    key: audit-${{ hashFiles('Cargo.lock') }}
    cache-on-failure: true
```

**Cache invalidation:** Automatic when `Cargo.lock` changes

## Remediation Guidelines

### 1. Identify Vulnerability

**Via GitHub Security Tab:**
1. Navigate to Security → Code scanning
2. Filter by severity (CRITICAL/HIGH)
3. Review vulnerability details, CVE, and affected versions

**Via CLI:**
```bash
cargo audit --json | jq '.vulnerabilities.list[]'
```

### 2. Update Dependencies

**Direct dependencies:**
```bash
# Update specific crate
cargo update -p vulnerable-crate

# Update to specific version
cargo update -p vulnerable-crate --precise 1.2.3

# Update all dependencies
cargo update
```

**Transitive dependencies:**
```bash
# Find dependency tree
cargo tree -p vulnerable-crate

# Update parent dependency
cargo update -p parent-crate
```

### 3. Verify Fix

```bash
# Re-run security scans
cargo audit
trivy fs --severity CRITICAL,HIGH .

# Verify Cargo.lock updated
git diff Cargo.lock
```

### 4. Test Impact

```bash
# Run full test suite
cargo test --workspace

# Run LSP-specific tests
RUST_TEST_THREADS=2 cargo test -p perl-lsp

# Check for regressions
just ci-gate
```

### 5. Document Change

```toml
# If downgrading or pinning a version, document why in Cargo.toml:

[dependencies]
vulnerable-crate = "=1.2.3"  # Pinned: CVE-2024-XXXXX mitigation
```

### Common Remediation Patterns

#### Pattern 1: Direct Dependency Vulnerability
```bash
# Example: tokio vulnerability
cargo update -p tokio
cargo test --workspace
git add Cargo.lock
git commit -m "security: update tokio to fix CVE-2024-XXXXX"
```

#### Pattern 2: Transitive Dependency Issue
```bash
# Example: hyper uses vulnerable openssl
cargo tree -p openssl
cargo update -p hyper  # Updates hyper which pulls in new openssl
cargo test --workspace
```

#### Pattern 3: No Fix Available
```yaml
# Add to deny.toml temporarily:
[advisories]
ignore = [
    { id = "RUSTSEC-2024-XXXXX", reason = "No fix available; tracking in issue #123" }
]
```

## False Positive Management

### Trivy False Positives

Create `.trivyignore` file in project root:

```yaml
# Ignore specific CVEs with justification
CVE-2024-12345  # False positive: affects only Windows, we're Linux-only
CVE-2024-67890  # Not exploitable in our use case (no network exposure)
```

### Cargo Audit Ignores

Add to `.cargo/audit.toml`:

```toml
[advisories]
ignore = [
    "RUSTSEC-2024-0001",  # False positive, see issue #123
]
```

Or use inline ignore in CI:

```bash
cargo audit --ignore RUSTSEC-2024-0001
```

### Cargo Deny Ignores

Update `deny.toml`:

```toml
[advisories]
ignore = [
    { id = "RUSTSEC-2024-0001", reason = "False positive - our usage is safe" },
]
```

### Documentation Requirements

When ignoring findings:
1. **Create tracking issue** with justification
2. **Document in ignore file** with reference to issue
3. **Set review date** to re-evaluate (quarterly)
4. **Include in CHANGELOG.md** if user-visible impact

## Security Policy

### Reporting Vulnerabilities

See `SECURITY.md` for vulnerability disclosure process.

Quick summary:
1. **DO NOT** open public issues for vulnerabilities
2. Email security contact (see SECURITY.md)
3. Allow 90 days for coordinated disclosure

### Security Updates

**Monitoring:**
- Daily scheduled scans
- Dependabot alerts (if enabled)
- RustSec advisory mailing list

**Response Time SLA:**
| Severity | Response Time | Fix Time Target |
|----------|---------------|-----------------|
| CRITICAL | 24 hours | 48 hours |
| HIGH | 48 hours | 1 week |
| MEDIUM | 1 week | 1 month |
| LOW | 1 month | Next release |

### Security Champions

Maintainers with security focus:
- Review security scan results weekly
- Triage security issues
- Coordinate security releases

## References

### Tools

- [Cargo Audit](https://github.com/rustsec/rustsec/tree/main/cargo-audit)
- [Cargo Deny](https://github.com/EmbarkStudios/cargo-deny)
- [Trivy](https://github.com/aquasecurity/trivy)
- [RustSec Advisory Database](https://rustsec.org/)

### Standards

- [SARIF Format](https://sarifweb.azurewebsites.net/)
- [CVE Database](https://cve.mitre.org/)
- [CVSS Scoring](https://www.first.org/cvss/)

### GitHub Security

- [Code Scanning](https://docs.github.com/en/code-security/code-scanning)
- [Security Advisories](https://docs.github.com/en/code-security/security-advisories)
- [Dependabot](https://docs.github.com/en/code-security/dependabot)

## Troubleshooting

### Issue: Trivy scan times out

**Solution:**
```bash
# Increase timeout
trivy fs --timeout 15m .

# Skip large directories
trivy fs --skip-dirs target,archive,.runs .
```

### Issue: False positives in secret scanning

**Solution:**
```bash
# Create .trivyignore.yaml with patterns
echo "test_fixtures/**" > .trivyignore.yaml
trivy fs --scanners secret .
```

### Issue: Cargo audit fails on CI but passes locally

**Solution:**
```bash
# Update advisory database
cargo audit --update

# Check for stale Cargo.lock
cargo update
git diff Cargo.lock
```

### Issue: SARIF upload fails

**Solution:**
```yaml
# Ensure proper permissions in workflow
permissions:
  contents: read
  security-events: write
```

## Changelog

| Date | Version | Changes |
|------|---------|---------|
| 2026-01-28 | 1.0.0 | Initial security scanning documentation |

---

**Last Updated:** 2026-01-28
**Maintained By:** Security Team
**Review Frequency:** Quarterly
