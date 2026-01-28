# Security Policy

## Supported Versions

We actively support security updates for the following versions:

| Version | Supported          |
| ------- | ------------------ |
| 0.9.x   | :white_check_mark: |
| 0.8.x   | :white_check_mark: |
| < 0.8   | :x:                |

## Reporting a Vulnerability

**DO NOT** open public issues for security vulnerabilities.

### How to Report

1. **Email**: Send security reports to the maintainers listed in `Cargo.toml`
2. **Subject Line**: Use "SECURITY:" prefix (e.g., "SECURITY: Buffer overflow in parser")
3. **Include**:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if available)
   - Your preferred method of contact

### What to Expect

| Timeline | Action |
|----------|--------|
| **24 hours** | Initial acknowledgment of your report |
| **72 hours** | Initial assessment and severity classification |
| **7 days** | Preliminary fix or mitigation strategy (for CRITICAL/HIGH) |
| **30 days** | Coordinated disclosure and patch release (standard timeline) |
| **90 days** | Maximum disclosure timeline (per industry standard) |

### Disclosure Policy

We follow **coordinated disclosure**:

1. **Report received** → We acknowledge within 24 hours
2. **Assessment** → We classify severity (CRITICAL/HIGH/MEDIUM/LOW)
3. **Fix development** → We develop and test a fix
4. **Private notification** → We notify you before public disclosure
5. **Public disclosure** → We publish advisory and release patch
6. **Credit** → We credit you in the security advisory (unless you prefer anonymity)

### Severity Classification

| Severity | Response Time | Public Disclosure |
|----------|---------------|-------------------|
| **CRITICAL** | 48 hours | 7-14 days after fix |
| **HIGH** | 1 week | 14-30 days after fix |
| **MEDIUM** | 2 weeks | 30-60 days after fix |
| **LOW** | 1 month | Next release cycle |

## Security Scanning

This project uses automated security scanning:

- **Cargo Audit**: Daily scans against RustSec Advisory Database
- **Cargo Deny**: License and dependency policy enforcement
- **Trivy**: Comprehensive vulnerability scanning (dependencies, containers, secrets)
- **GitHub Security Tab**: Centralized SARIF report tracking

See `docs/SECURITY_SCANNING.md` for detailed scanning procedures.

## Security Best Practices

### For Contributors

When contributing code:

1. **No `unwrap()`/`expect()`** - Use proper error handling with `Result<T, E>`
2. **No hardcoded secrets** - Use environment variables or secure vaults
3. **Validate all inputs** - Especially from LSP clients or file system
4. **Use safe defaults** - Fail closed, not open
5. **Minimize `unsafe`** - Document all unsafe blocks with safety justification
6. **Check dependencies** - Run `cargo audit` before submitting PR

### For Users

When using perl-lsp:

1. **Keep updated** - Always use the latest stable version
2. **Monitor advisories** - Subscribe to security notifications
3. **Report issues** - Follow the reporting guidelines above
4. **Isolate untrusted input** - Use appropriate sandboxing for untrusted Perl code
5. **Review dependencies** - Check `Cargo.lock` for your deployment

## Known Security Considerations

### Parser Security

- **Input validation**: The parser handles arbitrary Perl code and uses bounded recursion to prevent stack overflow
- **Memory safety**: All parser code uses Rust's memory safety guarantees
- **DoS protection**: Large files and deep nesting are handled with configurable limits

### LSP Server Security

- **IPC security**: LSP communication over stdio/pipes only (no network exposure by default)
- **File system access**: Limited to workspace roots configured by client
- **Path traversal prevention**: All file paths are validated and canonicalized
- **Resource limits**: Memory and CPU usage are bounded with timeouts

### Dependency Security

- **Supply chain**: We audit dependencies regularly with `cargo-audit` and `cargo-deny`
- **Minimal dependencies**: We minimize external dependencies to reduce attack surface
- **Pinned versions**: `Cargo.lock` is committed for reproducible builds
- **License compliance**: All dependencies use approved open-source licenses

## Security Updates

### Notification Channels

Security updates are announced via:

1. **GitHub Security Advisories**: https://github.com/YOUR_ORG/perl-lsp/security/advisories
2. **GitHub Releases**: Tagged with `[SECURITY]` prefix
3. **CHANGELOG.md**: With `[SECURITY]` section
4. **RustSec Database**: Critical vulnerabilities reported to RustSec

### Update Procedure

When a security update is released:

```bash
# Update perl-lsp
cargo install perl-lsp --force

# Or update in your project
cargo update -p perl-lsp
cargo build --release

# Verify version
perl-lsp --version
```

## Security Tooling

### Required Tools

```bash
# Install security scanning tools
cargo install cargo-audit --locked
cargo install cargo-deny --locked

# Install Trivy (platform-specific)
# See: https://aquasecurity.github.io/trivy/latest/getting-started/installation/
```

### Local Security Checks

```bash
# Run comprehensive security scan
just security-scan

# Quick pre-commit check
just security-quick

# Generate security report
just security-report
```

## Vulnerability Database

We track vulnerabilities in:

- **RustSec Advisory Database**: https://rustsec.org/
- **GitHub Security Advisories**: Project-specific advisories
- **National Vulnerability Database (NVD)**: CVE tracking

## Bug Bounty Program

We currently **do not** have a formal bug bounty program. However:

- We appreciate responsible disclosure
- We provide credit in security advisories
- We may offer recognition on our contributors page

## Contact

- **Security email**: See maintainers in `Cargo.toml`
- **General inquiries**: Open a non-security issue on GitHub
- **Commercial support**: Contact maintainers for enterprise support options

## Acknowledgments

We thank the following security researchers:

*(This section will be updated as researchers report vulnerabilities)*

---

**Last Updated**: 2026-01-28
**Next Review**: 2026-04-28 (quarterly review)
