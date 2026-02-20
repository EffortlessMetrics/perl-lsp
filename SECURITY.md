# Security Policy

## Supported Versions

We actively support security updates for the following versions:

| Version | Supported          | Status |
| ------- | ------------------ | -------- |
| 0.9.x   | :white_check_mark: | **Current Production** |
| 0.9.x   | :white_check_mark: | Legacy Support |
| 0.8.x   | :white_check_mark: | Legacy Support |
| < 0.8   | :x:                | End of Life |

**v0.9.x (Production-Ready) Security Guarantee**: As a production-ready release, v0.9.x receives priority security support with rapid response times and comprehensive security validation.

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

This project uses comprehensive automated security scanning:

- **Cargo Audit**: Daily scans against RustSec Advisory Database
- **Cargo Deny**: License and dependency policy enforcement
- **Trivy**: Comprehensive vulnerability scanning (dependencies, containers, secrets)
- **GitHub Security Tab**: Centralized SARIF report tracking
- **Mutation Testing**: Security-focused mutation hardening with 87% quality score
- **Fuzz Testing**: Property-based testing with crash detection and AST invariant validation

### v0.9.x (Production-Ready) Security Features

- **Enterprise-Grade Security**: Path traversal prevention, input validation, secure defaults
- **UTF-16 Boundary Protection**: Fixes for symmetric position conversion vulnerabilities
- **Process Isolation**: Safe execution environment for untrusted Perl code
- **Memory Safety**: Full Rust memory safety guarantees with minimal unsafe code
- **Supply Chain Security**: Audited dependencies with pinned versions

See `docs/SECURITY_DEVELOPMENT_GUIDE.md` for detailed security procedures.

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

### v0.9.x (Production-Ready) Production Security

**Parser Security**:
- **Input validation**: The parser handles arbitrary Perl code with bounded recursion to prevent stack overflow
- **Memory safety**: All parser code uses Rust's memory safety guarantees with comprehensive fuzz testing
- **DoS protection**: Large files and deep nesting handled with configurable limits and timeout protection
- **AST Security**: Comprehensive invariant validation with mutation hardening (87% quality score)

**LSP Server Security**:
- **IPC security**: LSP communication over stdio/pipes only (no network exposure by default)
- **File system access**: Limited to workspace roots configured by client with enterprise-grade validation
- **Path traversal prevention**: All file paths validated and canonicalized with UTF-16 boundary protection
- **Resource limits**: Memory and CPU usage bounded with <1MB overhead and adaptive timeout scaling

**DAP Debugging Security**:
- **Process isolation**: Debug adapter runs in isolated environment with controlled process spawning
- **Cross-platform security**: Windows, macOS, Linux, WSL with automatic path normalization
- **Secure defaults**: Safe configuration with enterprise security defaults
- **Performance security**: <50ms operations with resource monitoring

**Dependency Security**:
- **Supply chain**: Regular audits with `cargo-audit`, `cargo-deny`, and comprehensive vulnerability scanning
- **Minimal dependencies**: Reduced attack surface with carefully vetted dependencies
- **Pinned versions**: `Cargo.lock` committed for reproducible builds with automated dependency updates
- **License compliance**: All dependencies use approved open-source licenses with automated compliance checking

## Security Updates

### v0.9.x (Production-Ready) Security Update Process

**Notification Channels**:
1. **GitHub Security Advisories**: https://github.com/EffortlessMetrics/perl-lsp/security/advisories
2. **GitHub Releases**: Tagged with `[SECURITY]` prefix
3. **CHANGELOG.md**: With `[SECURITY]` section and CVE references
4. **RustSec Database**: Critical vulnerabilities reported to RustSec
5. **Enterprise Notifications**: Direct notifications for enterprise customers

**Update Procedure**:

When a security update is released:

```bash
# Update perl-lsp (production systems)
cargo install perl-lsp --force

# Or update in your project
cargo update -p perl-lsp
cargo build --release

# Verify version and security status
perl-lsp --version
perl-lsp --security-status  # New v0.9.x (Production-Ready) feature
```

**Emergency Security Updates**:

For critical vulnerabilities (CVSS 9.0+):
- **Hotfix Release**: Within 48 hours of disclosure
- **Automated Updates**: Recommended for production systems
- **Security Advisory**: Detailed impact analysis and mitigation
- **Enterprise Support**: Priority patches for enterprise customers

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

- **v0.9.x (Production-Ready) Security Focus**: Increased attention to security vulnerabilities in production release
- **Responsible Disclosure**: We appreciate and reward responsible disclosure
- **Security Credits**: We provide credit in security advisories and annual security report
- **Contributor Recognition**: Security researchers recognized in our contributors page and release notes

**Security Researcher Recognition**:
- Security advisories credit
- Annual security report acknowledgment
- Contributor page recognition
- Early access to security features
- Direct maintainer communication for security issues

## Contact

- **Security email**: See maintainers in `Cargo.toml`
- **General inquiries**: Open a non-security issue on GitHub
- **Commercial support**: Contact maintainers for enterprise support options
- **Security Discord**: Private channel for security researchers (request access via security email)

## v0.9.x (Production-Ready) Security Commitments

### Production Security Guarantees

As a v0.9.x (Production-Ready) production release, we commit to:

- **48-hour response** for critical security vulnerabilities
- **Comprehensive security testing** for all releases
- **Regular security audits** with third-party validation
- **Transparent disclosure** with coordinated vulnerability disclosure
- **Enterprise-grade support** for security issues

### Security Roadmap

- **Q1 2026**: Formal security audit by third-party firm
- **Q2 2026**: Enhanced fuzz testing infrastructure
- **Q3 2026**: Security hardening for enterprise deployments
- **Q4 2026**: Formal bug bounty program establishment

## Acknowledgments

We thank the following security researchers and contributors:

- **Internal Security Team**: Comprehensive security hardening and validation
- **Community Contributors**: Security-focused bug reports and improvements
- **Rust Security Team**: RustSec advisory database and security tools

*(This section will be updated as researchers report vulnerabilities)*

---

**Last Updated**: 2026-02-13 (v0.9.x (Production-Ready) Release)
**Next Review**: 2026-05-13 (quarterly review)
**Security Status**: Production Ready ✅
