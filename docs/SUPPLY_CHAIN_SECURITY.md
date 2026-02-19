# Supply Chain Security

This document describes the supply chain security measures implemented for perl-lsp, including SBOM generation and SLSA provenance attestation.

## Overview

Perl-lsp implements enterprise-grade supply chain security practices to ensure transparency, integrity, and verifiability of all released artifacts. This includes:

- **SBOM (Software Bill of Materials)**: Complete dependency inventory in industry-standard formats
- **SLSA Provenance**: Cryptographic attestation of build provenance (SLSA Level 2)
- **Artifact Verification**: SHA256 checksums and GitHub attestation verification

## SBOM (Software Bill of Materials)

### What is an SBOM?

An SBOM is a complete inventory of all components, libraries, and dependencies used in the software. It enables:

- Vulnerability tracking and management
- License compliance verification
- Dependency auditing
- Supply chain risk assessment

### SBOM Formats

Perl-lsp provides SBOMs in two industry-standard formats:

1. **SPDX (v2.3)**: Software Package Data Exchange format
   - Industry standard maintained by the Linux Foundation
   - Wide tool support and ecosystem adoption
   - File: `sbom-spdx.json`

2. **CycloneDX (v1.6)**: Lightweight SBOM format
   - OWASP standard designed for security use cases
   - Optimized for vulnerability management
   - File: `sbom-cyclonedx.json`

### Generating SBOMs Locally

```bash
# Install cargo-sbom
cargo install cargo-sbom

# Generate SPDX format
just sbom-spdx

# Generate CycloneDX format
just sbom-cyclonedx

# Generate both formats
just sbom

# Verify generation
just sbom-verify
```

### SBOM Contents

Each SBOM includes:

- **Package Information**: Name, version, license for all dependencies
- **Relationships**: Dependency graph and package relationships
- **Checksums**: SHA-1 hashes for package verification
- **Metadata**: Build timestamp, tool information, document namespace

### Using SBOMs

```bash
# View SBOM structure
jq '.packages[] | {name: .name, version: .versionInfo}' sbom-spdx.json | head -20

# List all licenses
jq '.packages[].licenseDeclared' sbom-spdx.json | sort -u

# Count total dependencies
jq '.packages | length' sbom-spdx.json

# Export to CSV for analysis
jq -r '.packages[] | [.name, .versionInfo, .licenseDeclared] | @csv' sbom-spdx.json > dependencies.csv
```

## SLSA Provenance

### What is SLSA?

SLSA (Supply chain Levels for Software Artifacts) is a security framework for ensuring the integrity of software artifacts throughout the software supply chain.

### SLSA Level 2 Compliance

Perl-lsp achieves SLSA Level 2 compliance through:

1. **Version Control**: Source code hosted on GitHub with full history
2. **Build Service**: Automated builds via GitHub Actions
3. **Provenance Generation**: Cryptographic attestation of build process
4. **Provenance Distribution**: Attestations attached to release artifacts

### Build Provenance

Every release artifact includes a cryptographically signed attestation that records:

- Source repository and commit hash
- Build platform and environment
- Build timestamps
- Builder identity (GitHub Actions)
- Artifact checksums

### Verifying Provenance

```bash
# Install GitHub CLI
# https://cli.github.com/

# Verify an artifact's provenance
gh attestation verify perl-lsp-v0.9.0-x86_64-unknown-linux-gnu.tar.gz \
  --owner EffortlessMetrics

# Verify SBOM provenance
gh attestation verify sbom-spdx.json --owner EffortlessMetrics

# View attestation details
gh attestation verify perl-lsp-v0.9.0-x86_64-unknown-linux-gnu.tar.gz \
  --owner EffortlessMetrics \
  --format json | jq
```

### What Provenance Guarantees

- **Build Integrity**: Artifact was built from specified source code
- **Build Platform**: Build occurred on GitHub-hosted infrastructure
- **Non-Repudiation**: Cryptographic proof of build origin
- **Tamper Detection**: Any modification after build will fail verification

## Security Audit Tools

### cargo-deny

Comprehensive supply chain auditing:

```bash
# Run full security audit
cargo deny check

# Check for security advisories
cargo deny check advisories

# Verify license compliance
cargo deny check licenses

# Check for banned crates
cargo deny check bans
```

Configuration: `deny.toml`

### cargo-audit

Vulnerability scanning against RustSec advisory database:

```bash
# Install cargo-audit
cargo install cargo-audit

# Scan for vulnerabilities
cargo audit

# Generate JSON report
cargo audit --json > audit-report.json
```

## Release Process Integration

### Automated SBOM Generation

SBOMs are automatically generated during the release process:

1. **Trigger**: Release workflow runs on version tags (e.g., `v0.9.0`)
2. **Generation**: Both SPDX and CycloneDX SBOMs generated
3. **Checksums**: SHA256 hashes computed for all artifacts including SBOMs
4. **Provenance**: SLSA attestations generated and signed
5. **Publishing**: SBOMs attached to GitHub release alongside binaries

### Release Artifacts

Each release includes:

- Binary packages (`.tar.gz`, `.zip`)
- SBOM files (`sbom-spdx.json`, `sbom-cyclonedx.json`)
- Checksums file (`SHA256SUMS`)
- SLSA provenance attestations (via GitHub Attestations)

### Workflow: `.github/workflows/release.yml`

Key steps:
1. Build binaries for all platforms
2. Generate SBOMs for the workspace
3. Create checksums for all artifacts
4. Attest build provenance (SLSA Level 2)
5. Publish to GitHub Releases

## Compliance and Standards

### Industry Standards

- **SPDX 2.3**: ISO/IEC 5962:2021 standard
- **CycloneDX 1.6**: OWASP standard for SBOM
- **SLSA**: Supply chain security framework
- **GitHub Attestations**: Sigstore-based provenance

### Enterprise Compliance

Supply chain security features support:

- **NIST SP 800-161**: Supply chain risk management
- **Executive Order 14028**: Improving nation's cybersecurity (SBOM requirements)
- **ISO 27001**: Information security management
- **SOC 2**: Supply chain transparency controls

## Security Policy

### Vulnerability Disclosure

See [SECURITY.md](../SECURITY.md) for vulnerability reporting procedures.

### Advisory Tracking

Security advisories are tracked via:
- RustSec Advisory Database
- GitHub Security Advisories
- Automated cargo-audit scans in CI

### Update Policy

- **Critical vulnerabilities**: Emergency patch release within 24-48 hours
- **High severity**: Patch release within 1 week
- **Medium/Low severity**: Included in next scheduled release

## Best Practices

### For Users

1. **Verify Downloads**: Always verify SHA256 checksums
2. **Check Provenance**: Use `gh attestation verify` for critical deployments
3. **Review SBOMs**: Understand dependencies before deployment
4. **Monitor Advisories**: Subscribe to security notifications

### For Contributors

1. **Minimize Dependencies**: Avoid unnecessary dependencies
2. **Audit Changes**: Review dependency updates carefully
3. **Update Regularly**: Keep dependencies current
4. **Document Security**: Note security implications in PRs

## Tools and Resources

### Required Tools

- `cargo-sbom`: SBOM generation (`cargo install cargo-sbom`)
- `cargo-audit`: Vulnerability scanning (`cargo install cargo-audit`)
- `cargo-deny`: Supply chain policy enforcement (`cargo install cargo-deny`)
- `gh`: GitHub CLI for attestation verification (https://cli.github.com/)

### Documentation

- SPDX Specification: https://spdx.dev/
- CycloneDX Standard: https://cyclonedx.org/
- SLSA Framework: https://slsa.dev/
- GitHub Attestations: https://docs.github.com/en/actions/security-guides/using-artifact-attestations

### Support

- Issue Tracker: https://github.com/EffortlessMetrics/perl-lsp/issues
- Security Policy: See SECURITY.md
- Discussions: GitHub Discussions

## Continuous Improvement

### Planned Enhancements

- [ ] SLSA Level 3 compliance (hermetic builds)
- [ ] GPG signing of release artifacts
- [ ] Automated SBOM diffing between releases
- [ ] Integration with vulnerability databases
- [ ] SBOM-based license compliance automation

### Metrics

Current supply chain security posture:

- **SLSA Level**: 2
- **SBOM Coverage**: 100% of dependencies
- **SBOM Formats**: SPDX 2.3, CycloneDX 1.6
- **Provenance**: All release artifacts attested
- **Verification**: GitHub Attestations (Sigstore)

---

*Last Updated: 2026-01-28*
*Version: 0.9.0*
