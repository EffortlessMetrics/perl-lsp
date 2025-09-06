# Security Development Guidelines

This project demonstrates **enterprise-grade security practices** in its test infrastructure. All contributors should follow these security development standards.

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

## File Path Completion Security (v0.8.7+)

The LSP server includes comprehensive file path completion with enterprise-grade security features:

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

This security framework ensures that all code contributions maintain enterprise-grade security standards while providing comprehensive protection against common attack vectors.