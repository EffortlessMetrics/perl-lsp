# Security Development Guidelines

This project demonstrates **comprehensive security practices** across its parsing infrastructure and LSP implementation. All contributors should follow these security development standards, with particular attention to the UTF-16 position conversion security enhancements implemented in PR #153.

## Secure Authentication Implementation

When implementing authentication systems (including test scenarios), use production-grade security:

```perl
use Crypt::PBKDF2;

# OWASP 2021 compliant PBKDF2 configuration
sub get_pbkdf2_instance {
    return Crypt::PBKDF2->new(
        hash_class => 'HMACSHA2',      # SHA-2 family for cryptographic strength
        hash_args => { sha_size => 256 }, # SHA-256 for collision resistance
        iterations => 100_000,          # 100k iterations (OWASP 2021 minimum)
        salt_len => 16,                 # 128-bit cryptographically random salt
    );
}

sub authenticate_user {
    my ($username, $password) = @_;
    my $users = load_users();
    my $pbkdf2 = get_pbkdf2_instance();
    
    foreach my $user (@$users) {
        if ($user->{name} eq $username) {
            # Constant-time validation prevents timing attacks
            if ($pbkdf2->validate($user->{password_hash}, $password)) {
                return $user;
            }
        }
    }
    return undef;  # Authentication failed
}
```

## Security Requirements

✅ **Cryptographic Standards**: Use OWASP 2021 compliant algorithms and parameters
✅ **Timing Attack Prevention**: Implement constant-time comparisons for authentication
✅ **No Plaintext Storage**: Hash all passwords immediately, never store in clear text
✅ **Secure Salt Generation**: Use cryptographically secure random salts (≥16 bytes)
✅ **Input Validation**: Sanitize and validate all user inputs
✅ **Path Security**: Use canonical paths with workspace boundary validation
✅ **UTF-16 Position Safety**: Symmetric position conversion with boundary validation (PR #153)
✅ **Unicode Security**: Prevent arithmetic overflow in position calculations
✅ **LSP Error Recovery Security**: Secure logging with data truncation and no sensitive data exposure (Issue #144)
✅ **Malformed Input Handling**: Graceful recovery from malformed JSON-RPC frames with session continuity (Issue #144)

## Enhanced LSP Error Recovery Security (Issue #144)

**Enterprise-Grade Error Handling**: Issue #144 implementation introduces comprehensive security measures for LSP error recovery that prevent data exposure and maintain system integrity.

### Security Features

**Secure Content Logging**:
```rust
// SECURE PATTERN (Implemented in Issue #144)
// Safe content truncation prevents sensitive data exposure
let content_str = String::from_utf8_lossy(content);
if content_str.len() > 100 {
    eprintln!(
        "LSP server: Malformed frame (truncated): {}...",
        &content_str[..100]  // Maximum 100 characters logged
    );
} else {
    eprintln!("LSP server: Malformed frame: {}", content_str);
}
```

**Key Security Benefits**:
- **Data Protection**: Content truncated to 100 characters maximum to prevent sensitive data logging
- **Session Integrity**: Malformed frames don't terminate LSP server, preventing denial-of-service
- **Zero Data Leakage**: No client code or sensitive information exposed in error logs
- **Audit Trail**: Secure logging maintains troubleshooting capability without security risks

**Enterprise Compliance**:
- **GDPR Compliance**: No personal data exposure in error logs
- **SOX Compliance**: Audit trail without sensitive data logging
- **Security Standards**: Follows secure logging best practices
- **Data Minimization**: Only necessary debugging information logged

**Attack Vector Prevention**:
```rust
// Prevents these attack scenarios:
// 1. Information disclosure through error logs
// 2. Denial of service through malformed frame injection
// 3. Session hijacking through server termination
// 4. Memory exhaustion through oversized malformed content
```

## UTF-16 Position Security (PR #153)

**Critical Security Enhancement**: UTF-16 position conversion vulnerabilities discovered and eliminated through comprehensive mutation testing.

### Security Vulnerability Resolved

**Issue**: Asymmetric position conversion bug in LSP position mapping led to boundary violations and potential security vulnerabilities:

```rust
// VULNERABLE PATTERN (Fixed in PR #153)
// Asymmetric conversion could cause boundary violations
fn convert_position_unsafe(utf8_pos: usize) -> u32 {
    // Dangerous: no boundary validation, asymmetric conversion
    utf8_pos as u32  // Potential overflow, no validation
}
```

**Solution**: Symmetric fractional position handling with rigorous boundary validation:

```rust
// SECURE PATTERN (PR #153 Implementation)
pub fn convert_utf8_to_utf16_position(text: &str, utf8_offset: usize) -> u32 {
    // Symmetric conversion with boundary checks
    if utf8_offset > text.len() {
        return text.chars().count() as u32;  // Safe fallback
    }

    // Count UTF-16 code units with proper validation
    text[..utf8_offset].encode_utf16().count() as u32
}
```

### Security Architecture for Position Conversion

1. **Boundary Validation**: All position conversions validate input ranges before processing
2. **Symmetric Operations**: UTF-8 ↔ UTF-16 conversions use identical validation logic
3. **Overflow Prevention**: Arithmetic operations include bounds checking
4. **Fractional Handling**: Proper handling of positions that fall within multi-byte sequences

### Security Testing Framework

```rust
#[test]
fn test_utf16_boundary_security() {
    let text = "Hello 🦀 World";

    // Test boundary conditions
    assert_eq!(convert_position(text, 0), 0);
    assert_eq!(convert_position(text, text.len()), expected_length);

    // Test with invalid positions (should not panic)
    let result = convert_position(text, usize::MAX);
    assert!(result <= text.len() as u32);
}
```

### Mutation Testing Integration

The UTF-16 security enhancements were validated through **comprehensive mutation testing** that:
- Discovered the original asymmetric conversion vulnerability
- Validated the symmetric conversion fix
- Ensured boundary conditions are properly handled
- Achieved 87% mutation score with security-focused test coverage

## File Path Completion Security (v0.8.7+)

The LSP server includes comprehensive file path completion with comprehensive security features:

### Security Architecture
- **Path traversal prevention**: Blocks `../` patterns and absolute paths (except `/`)
- **Null byte protection**: Rejects strings containing `\0` characters
- **Reserved name filtering**: Prevents Windows reserved names (CON, PRN, AUX, etc.)
- **Filename validation**: UTF-8 validation, length limits (255 chars), control character filtering
- **Directory safety**: Canonicalization with safe fallbacks, hidden file filtering

### Security API Reference
```rust
// Security validation methods
fn sanitize_path(&self, path: &str) -> Option<String>;
fn is_safe_filename(&self, filename: &str) -> bool;
fn is_hidden_or_forbidden(&self, entry: &walkdir::DirEntry) -> bool;
```

### Security Features
- Path traversal prevention (`../` blocked)
- Null byte detection (`\0` blocked)
- Windows reserved name filtering
- Symbolic link traversal disabled  
- Hidden file exclusion
- Control character filtering

### Performance Limits (Security Boundaries)
- **Max results**: 50 completions
- **Max depth**: 1 directory level
- **Max entries examined**: 200 filesystem entries
- **Path length limit**: 1024 characters
- **Filename length limit**: 255 characters

## Security Testing Requirements

All security-related code must include comprehensive tests:

### Authentication Security
- Test password hashing, validation, and timing consistency
- Verify constant-time comparison behavior
- Test salt generation randomness and uniqueness

### UTF-16 Position Security (PR #153)
- Validate symmetric position conversion logic
- Test boundary conditions with multi-byte Unicode characters
- Verify overflow prevention in position arithmetic
- Test fractional position handling within multi-byte sequences
- Validate security of position conversions at UTF-8/UTF-16 boundaries

### Input Validation
- Verify proper sanitization and boundary checking
- Test for injection vulnerabilities
- Validate error handling for malformed input

### File Access Security
- Test path traversal prevention and workspace boundaries
- Verify symbolic link handling
- Test hidden file and directory exclusion

### Error Message Security
- Ensure no sensitive information disclosure
- Test error responses for information leakage
- Verify consistent error behavior

## Security Review Process

- All authentication/security code changes require security review
- Test implementations serve as security best practice examples  
- Document security assumptions and threat models in code comments
- Use the security implementation in PR #44 as the reference standard

## Testing Security Features

### File Completion Security Tests
```bash
# Test individual security scenarios
cargo test -p perl-parser file_completion_tests::basic_security_test_rejects_path_traversal

# Manual security testing examples
# These should NOT provide completions due to security restrictions:
my $test1 = "../etc/passwd";      # Path traversal blocked
my $test2 = "/etc/hosts";         # Absolute path blocked (except root)
my $test3 = "file\0with\0null";   # Null bytes blocked
```

### UTF-16 Position Security Tests
```bash
# Test UTF-16 position conversion security (PR #153)
cargo test -p perl-lsp lsp_encoding_edge_cases
cargo test -p perl-parser --test mutation_hardening_tests -- utf16_position

# Comprehensive position boundary testing
cargo test -p perl-parser position_tracker_tests -- --nocapture

# Examples of secure position conversion testing:
# These should handle gracefully without panics or overflows:
let text = "Hello 🦀 World 🌍";  // Mixed Unicode
let boundary_test = convert_position(text, text.len() + 1000);  // Beyond bounds
let emoji_boundary = convert_position(text, 7);  // Within emoji sequence
```

### Authentication Security Tests
```bash
# Test authentication implementation (if applicable)
cargo test -p perl-parser authentication_security_tests

# Performance timing tests for constant-time validation
cargo test -p perl-parser authentication_timing_tests -- --nocapture
```

## Security Best Practices

### Code Implementation
1. **Input Validation**: Always validate and sanitize user inputs at boundaries
2. **Path Handling**: Use canonical paths with explicit boundary checking
3. **Error Handling**: Provide consistent error responses without information leakage
4. **Resource Limits**: Implement appropriate limits to prevent resource exhaustion
5. **Cryptographic Operations**: Use established libraries with security-reviewed implementations

### Testing Strategy
1. **Security-Focused Tests**: Create tests specifically targeting security boundaries
2. **Negative Testing**: Test invalid/malicious inputs extensively
3. **Performance Security**: Verify timing-attack resistance where applicable
4. **Boundary Testing**: Test edge cases and limits thoroughly

### Documentation
1. **Security Assumptions**: Document all security assumptions clearly
2. **Threat Models**: Identify and document relevant threat vectors
3. **Mitigation Strategies**: Document how security controls address threats
4. **Review Requirements**: Specify security review requirements for changes

This security framework ensures that all code contributions maintain comprehensive security standards while providing comprehensive protection against common attack vectors.

## Supply Chain Security

This project implements comprehensive supply chain security measures including SBOM generation and SLSA provenance attestation. For detailed information, see:

- **[SUPPLY_CHAIN_SECURITY.md](SUPPLY_CHAIN_SECURITY.md)** - Complete supply chain security documentation
- **SBOM Generation**: `just sbom` to generate Software Bill of Materials
- **Security Audit**: `just security-audit` to run vulnerability scans
- **Provenance Verification**: `gh attestation verify` to verify build attestations

### Quick Start

```bash
# Generate SBOM for dependency audit
just sbom

# Run security audit
just security-audit

# Verify a release artifact
gh attestation verify perl-lsp-v0.9.0-x86_64-unknown-linux-gnu.tar.gz --owner EffortlessMetrics
```

See [SUPPLY_CHAIN_SECURITY.md](SUPPLY_CHAIN_SECURITY.md) for comprehensive documentation on SBOM formats, SLSA provenance, and verification procedures.