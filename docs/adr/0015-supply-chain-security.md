# ADR-0015: Supply Chain Security (SBOM + SLSA Provenance)

**Status**: Accepted
**Date**: 2025-02-01
**Decision Makers**: Perl LSP Architecture Team
**Related**: [SUPPLY_CHAIN_SECURITY.md](../reference/SUPPLY_CHAIN_SECURITY.md)

## Context

Modern software supply chain attacks have become increasingly sophisticated, targeting the build and distribution pipeline rather than application code itself. High-profile incidents (SolarWinds, Codecov, event-stream) demonstrate the critical need for verifiable supply chain integrity.

### Problem Statement

1. **Dependency Transparency**: Users cannot easily verify what dependencies are included
2. **Vulnerability Tracking**: Security teams need accurate component inventories
3. **License Compliance**: Organizations require license verification for compliance
4. **Build Integrity**: No guarantee that released artifacts match source code
5. **Tamper Detection**: No mechanism to detect post-build modifications

### Regulatory and Standards Drivers

| Standard | Requirement | Relevance |
|----------|-------------|-----------|
| US EO 14028 | SBOM for federal software | Government contracts |
| SLSA Framework | Build provenance attestation | Industry best practice |
| NIST SSDF | Supply chain verification | Security frameworks |
| OpenSSF Scorecard | Security best practices | Community standards |

## Decision

**We implement comprehensive supply chain security with SBOM generation in dual formats (SPDX v2.3, CycloneDX v1.6) and SLSA Level 2 provenance attestation for all release artifacts.**

### SBOM (Software Bill of Materials)

#### Supported Formats

1. **SPDX (v2.3)**: Software Package Data Exchange
   - Linux Foundation standard
   - Wide tool support and ecosystem adoption
   - File: `sbom-spdx.json`

2. **CycloneDX (v1.6)**: Lightweight SBOM format
   - OWASP standard for security use cases
   - Optimized for vulnerability management
   - File: `sbom-cyclonedx.json`

#### SBOM Contents

Each SBOM includes:

- **Package Information**: Name, version, license for all dependencies
- **Relationships**: Complete dependency graph
- **Checksums**: SHA-1 hashes for package verification
- **Metadata**: Build timestamp, tool information, document namespace

### SLSA Level 2 Provenance

#### What SLSA Level 2 Provides

| Requirement | Implementation |
|-------------|----------------|
| Version Control | Source hosted on GitHub with full history |
| Build Service | Automated builds via GitHub Actions |
| Provenance Generation | Cryptographic attestation of build process |
| Provenance Distribution | Attestations attached to release artifacts |

#### Provenance Contents

Every release artifact includes cryptographically signed attestation recording:

- Source repository and commit hash
- Build platform and environment
- Build timestamps
- Builder identity (GitHub Actions)
- Artifact checksums

### Verification Workflow

```bash
# Install GitHub CLI
# https://cli.github.com/

# Verify artifact provenance
gh attestation verify perl-lsp-v0.10.0-x86_64-unknown-linux-gnu.tar.gz \
  --owner EffortlessMetrics

# Verify SBOM provenance
gh attestation verify sbom-spdx.json --owner EffortlessMetrics

# View attestation details
gh attestation verify perl-lsp-v0.10.0-x86_64-unknown-linux-gnu.tar.gz \
  --owner EffortlessMetrics \
  --format json | jq
```

## Consequences

### Positive

1. **Vulnerability Tracking**: Security teams can quickly identify affected versions
2. **License Compliance**: Automated license verification for all dependencies
3. **Build Verification**: Cryptographic proof that artifacts match source code
4. **Tamper Detection**: Any post-build modification fails verification
5. **Regulatory Compliance**: Meets US EO 14028 and NIST SSDF requirements
6. **Enterprise Adoption**: Supply chain security is often a procurement requirement

### Negative

1. **Build Complexity**: Additional steps in release process
2. **Artifact Size**: Additional SBOM and attestation files
3. **Tool Dependencies**: Requires GitHub CLI for verification
4. **Maintenance**: SBOM generation must be kept in sync with dependencies

### Mitigations

- Automated SBOM generation via `just sbom` command
- CI/CD integration ensures consistent generation
- Documentation provides clear verification steps

## Implementation

### Build Commands

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

### Security Audit Tools

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

### SBOM Analysis Examples

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

## Configuration Files

| File | Purpose |
|------|---------|
| `deny.toml` | cargo-deny configuration for security audits |
| `justfile` | SBOM generation commands |
| `.github/workflows/` | CI/CD provenance generation |

## Security Guarantees

### What Provenance Guarantees

- **Build Integrity**: Artifact was built from specified source code
- **Build Platform**: Build occurred on GitHub-hosted infrastructure
- **Non-Repudiation**: Cryptographic proof of build origin
- **Tamper Detection**: Any modification after build will fail verification

### What Provenance Does NOT Guarantee

- Correctness of the source code
- Absence of vulnerabilities in dependencies
- Security of the build environment (SLSA Level 3+)

## References

- [Supply Chain Security Reference](../reference/SUPPLY_CHAIN_SECURITY.md)
- [SLSA Specification](https://slsa.dev/)
- [SPDX Specification](https://spdx.dev/)
- [CycloneDX Specification](https://cyclonedx.org/)
- [US Executive Order 14028](https://www.whitehouse.gov/briefing-room/presidential-actions/2021/05/12/executive-order-on-improving-the-nations-cybersecurity/)
