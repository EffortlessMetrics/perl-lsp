//! Security validation fixtures for LSP security testing
//!
//! Provides test data for validating security aspects of the Perl LSP:
//! - UTF-16/UTF-8 boundary testing and symmetric position conversion
//! - Path traversal prevention in file operations
//! - Input validation and sanitization testing
//! - Resource exhaustion prevention scenarios
//! - Injection attack prevention validation

use serde_json::{json, Value};
use std::collections::HashMap;

#[cfg(test)]
pub struct SecurityFixture {
    pub name: &'static str,
    pub description: &'static str,
    pub attack_vector: AttackVector,
    pub test_input: SecurityTestInput,
    pub expected_behavior: SecurityExpectedBehavior,
    pub security_category: SecurityCategory,
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub struct SecurityTestInput {
    pub malicious_content: String,
    pub context: SecurityTestContext,
    pub payload_size_bytes: usize,
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub struct SecurityTestContext {
    pub uri: String,
    pub position: Option<Position>,
    pub additional_params: Value,
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub enum AttackVector {
    PathTraversal,
    UtfBoundaryExploit,
    ResourceExhaustion,
    CodeInjection,
    BufferOverflow,
    SymmetricPositionAttack,
    FileSystemAccess,
    MemoryExhaustion,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub enum SecurityExpectedBehavior {
    ShouldReject,
    ShouldSanitize,
    ShouldLimit,
    ShouldValidate,
    ShouldIsolate,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub enum SecurityCategory {
    InputValidation,
    FileSystem,
    Memory,
    Position,
    Protocol,
    Injection,
}

/// Comprehensive security validation fixtures
#[cfg(test)]
pub fn load_security_fixtures() -> Vec<SecurityFixture> {
    vec![
        // Path traversal prevention
        SecurityFixture {
            name: "path_traversal_dot_dot_slash",
            description: "Path traversal attempt using ../../../ sequences",
            attack_vector: AttackVector::PathTraversal,
            test_input: SecurityTestInput {
                malicious_content: "file:///../../etc/passwd".to_string(),
                context: SecurityTestContext {
                    uri: "file:///test/../../../etc/passwd".to_string(),
                    position: None,
                    additional_params: json!({
                        "method": "textDocument/didOpen",
                        "textDocument": {
                            "uri": "file:///test/../../../etc/passwd",
                            "languageId": "perl",
                            "version": 1,
                            "text": "#!/usr/bin/perl\nprint 'pwned';"
                        }
                    }),
                },
                payload_size_bytes: 256,
            },
            expected_behavior: SecurityExpectedBehavior::ShouldReject,
            security_category: SecurityCategory::FileSystem,
        },

        SecurityFixture {
            name: "path_traversal_encoded_sequences",
            description: "Path traversal using URL-encoded sequences",
            attack_vector: AttackVector::PathTraversal,
            test_input: SecurityTestInput {
                malicious_content: "file:///test%2F%2E%2E%2F%2E%2E%2Fetc%2Fpasswd".to_string(),
                context: SecurityTestContext {
                    uri: "file:///test%2F%2E%2E%2F%2E%2E%2Fetc%2Fpasswd".to_string(),
                    position: None,
                    additional_params: json!({
                        "method": "textDocument/definition",
                        "params": {
                            "textDocument": {
                                "uri": "file:///test%2F%2E%2E%2F%2E%2E%2Fetc%2Fpasswd"
                            },
                            "position": { "line": 0, "character": 0 }
                        }
                    }),
                },
                payload_size_bytes: 512,
            },
            expected_behavior: SecurityExpectedBehavior::ShouldReject,
            security_category: SecurityCategory::FileSystem,
        },

        SecurityFixture {
            name: "path_traversal_windows_backslash",
            description: "Windows-style path traversal with backslashes",
            attack_vector: AttackVector::PathTraversal,
            test_input: SecurityTestInput {
                malicious_content: r"file:///test\..\..\..\windows\system32\config\sam".to_string(),
                context: SecurityTestContext {
                    uri: r"file:///test\..\..\..\windows\system32\config\sam".to_string(),
                    position: None,
                    additional_params: json!({
                        "method": "textDocument/hover",
                        "params": {
                            "textDocument": {
                                "uri": r"file:///test\..\..\..\windows\system32\config\sam"
                            },
                            "position": { "line": 5, "character": 10 }
                        }
                    }),
                },
                payload_size_bytes: 384,
            },
            expected_behavior: SecurityExpectedBehavior::ShouldReject,
            security_category: SecurityCategory::FileSystem,
        },

        // UTF-16/UTF-8 boundary exploits
        SecurityFixture {
            name: "utf16_boundary_position_manipulation",
            description: "Position manipulation using UTF-16 surrogate pairs",
            attack_vector: AttackVector::UtfBoundaryExploit,
            test_input: SecurityTestInput {
                malicious_content: "my $emoji = \"🚀\"; # U+1F680 (surrogate pair)\nmy $var = \"test\";".to_string(),
                context: SecurityTestContext {
                    uri: "file:///test/utf16_boundary.pl".to_string(),
                    position: Some(Position { line: 0, character: 12 }), // Within emoji
                    additional_params: json!({
                        "method": "textDocument/completion",
                        "params": {
                            "textDocument": {
                                "uri": "file:///test/utf16_boundary.pl"
                            },
                            "position": { "line": 0, "character": 12 }
                        }
                    }),
                },
                payload_size_bytes: 128,
            },
            expected_behavior: SecurityExpectedBehavior::ShouldValidate,
            security_category: SecurityCategory::Position,
        },

        SecurityFixture {
            name: "utf8_normalization_attack",
            description: "Unicode normalization confusion attack",
            attack_vector: AttackVector::UtfBoundaryExploit,
            test_input: SecurityTestInput {
                malicious_content: "my $café = 'cafe\\u0301'; # É vs é + combining accent".to_string(),
                context: SecurityTestContext {
                    uri: "file:///test/normalization.pl".to_string(),
                    position: Some(Position { line: 0, character: 8 }),
                    additional_params: json!({
                        "method": "textDocument/references",
                        "params": {
                            "textDocument": {
                                "uri": "file:///test/normalization.pl"
                            },
                            "position": { "line": 0, "character": 8 },
                            "context": { "includeDeclaration": true }
                        }
                    }),
                },
                payload_size_bytes: 256,
            },
            expected_behavior: SecurityExpectedBehavior::ShouldSanitize,
            security_category: SecurityCategory::InputValidation,
        },

        SecurityFixture {
            name: "symmetric_position_conversion_attack",
            description: "Exploit symmetric position conversion between UTF-8 and UTF-16",
            attack_vector: AttackVector::SymmetricPositionAttack,
            test_input: SecurityTestInput {
                malicious_content: "# UTF-8: 世界\n# UTF-16: same chars, different byte positions\nmy $test = \"世界hello\";\n".to_string(),
                context: SecurityTestContext {
                    uri: "file:///test/symmetric_position.pl".to_string(),
                    position: Some(Position { line: 2, character: 15 }), // Position within mixed content
                    additional_params: json!({
                        "method": "textDocument/documentSymbol",
                        "params": {
                            "textDocument": {
                                "uri": "file:///test/symmetric_position.pl"
                            }
                        }
                    }),
                },
                payload_size_bytes: 512,
            },
            expected_behavior: SecurityExpectedBehavior::ShouldValidate,
            security_category: SecurityCategory::Position,
        },

        // Resource exhaustion attacks
        SecurityFixture {
            name: "memory_exhaustion_large_file",
            description: "Memory exhaustion via extremely large file",
            attack_vector: AttackVector::MemoryExhaustion,
            test_input: SecurityTestInput {
                malicious_content: generate_large_malicious_content(50_000_000), // 50MB
                context: SecurityTestContext {
                    uri: "file:///test/memory_bomb.pl".to_string(),
                    position: None,
                    additional_params: json!({
                        "method": "textDocument/didOpen",
                        "params": {
                            "textDocument": {
                                "uri": "file:///test/memory_bomb.pl",
                                "languageId": "perl",
                                "version": 1,
                                "text": "[LARGE_CONTENT]"
                            }
                        }
                    }),
                },
                payload_size_bytes: 50_000_000,
            },
            expected_behavior: SecurityExpectedBehavior::ShouldLimit,
            security_category: SecurityCategory::Memory,
        },

        SecurityFixture {
            name: "recursive_symbol_attack",
            description: "Resource exhaustion via recursive symbol definitions",
            attack_vector: AttackVector::ResourceExhaustion,
            test_input: SecurityTestInput {
                malicious_content: generate_recursive_symbols(10000),
                context: SecurityTestContext {
                    uri: "file:///test/recursive_symbols.pl".to_string(),
                    position: None,
                    additional_params: json!({
                        "method": "textDocument/documentSymbol",
                        "params": {
                            "textDocument": {
                                "uri": "file:///test/recursive_symbols.pl"
                            }
                        }
                    }),
                },
                payload_size_bytes: 2_000_000,
            },
            expected_behavior: SecurityExpectedBehavior::ShouldLimit,
            security_category: SecurityCategory::Memory,
        },

        SecurityFixture {
            name: "deep_nesting_stack_overflow",
            description: "Stack overflow via deeply nested structures",
            attack_vector: AttackVector::BufferOverflow,
            test_input: SecurityTestInput {
                malicious_content: generate_deeply_nested_structure(5000),
                context: SecurityTestContext {
                    uri: "file:///test/deep_nesting.pl".to_string(),
                    position: Some(Position { line: 1, character: 50 }),
                    additional_params: json!({
                        "method": "textDocument/hover",
                        "params": {
                            "textDocument": {
                                "uri": "file:///test/deep_nesting.pl"
                            },
                            "position": { "line": 1, "character": 50 }
                        }
                    }),
                },
                payload_size_bytes: 1_000_000,
            },
            expected_behavior: SecurityExpectedBehavior::ShouldLimit,
            security_category: SecurityCategory::Memory,
        },

        // Code injection attacks
        SecurityFixture {
            name: "perl_code_injection_eval",
            description: "Code injection via malicious eval statements",
            attack_vector: AttackVector::CodeInjection,
            test_input: SecurityTestInput {
                malicious_content: r#"
my $user_input = q{system("rm -rf /")};
eval $user_input;  # Malicious eval
my $safe_code = "print 'hello'";
"#.to_string(),
                context: SecurityTestContext {
                    uri: "file:///test/code_injection.pl".to_string(),
                    position: Some(Position { line: 2, character: 5 }),
                    additional_params: json!({
                        "method": "workspace/executeCommand",
                        "params": {
                            "command": "perl.runFile",
                            "arguments": ["file:///test/code_injection.pl"]
                        }
                    }),
                },
                payload_size_bytes: 512,
            },
            expected_behavior: SecurityExpectedBehavior::ShouldIsolate,
            security_category: SecurityCategory::Injection,
        },

        SecurityFixture {
            name: "regex_injection_catastrophic_backtracking",
            description: "ReDoS attack via catastrophic backtracking",
            attack_vector: AttackVector::ResourceExhaustion,
            test_input: SecurityTestInput {
                malicious_content: r#"
my $text = "a" x 1000 . "b";
$text =~ /(a+)+b/;  # Catastrophic backtracking pattern
"#.to_string(),
                context: SecurityTestContext {
                    uri: "file:///test/regex_dos.pl".to_string(),
                    position: Some(Position { line: 2, character: 10 }),
                    additional_params: json!({
                        "method": "textDocument/completion",
                        "params": {
                            "textDocument": {
                                "uri": "file:///test/regex_dos.pl"
                            },
                            "position": { "line": 2, "character": 10 }
                        }
                    }),
                },
                payload_size_bytes: 256,
            },
            expected_behavior: SecurityExpectedBehavior::ShouldLimit,
            security_category: SecurityCategory::InputValidation,
        },

        // Protocol-level attacks
        SecurityFixture {
            name: "json_rpc_injection",
            description: "JSON-RPC injection via malformed requests",
            attack_vector: AttackVector::CodeInjection,
            test_input: SecurityTestInput {
                malicious_content: r#"{"jsonrpc":"2.0","method":"textDocument/didOpen\","params":{"textDocument":{"uri":"file:///etc/passwd","text":"pwned"}}}"#.to_string(),
                context: SecurityTestContext {
                    uri: "malformed_request".to_string(),
                    position: None,
                    additional_params: json!({
                        "raw_request": r#"{"jsonrpc":"2.0","method":"textDocument/didOpen\","params":{"textDocument":{"uri":"file:///etc/passwd","text":"pwned"}}}"#
                    }),
                },
                payload_size_bytes: 256,
            },
            expected_behavior: SecurityExpectedBehavior::ShouldReject,
            security_category: SecurityCategory::Protocol,
        },

        SecurityFixture {
            name: "null_byte_injection",
            description: "Null byte injection in file paths",
            attack_vector: AttackVector::FileSystemAccess,
            test_input: SecurityTestInput {
                malicious_content: "file:///test/safe.pl\0../../../etc/passwd".to_string(),
                context: SecurityTestContext {
                    uri: "file:///test/safe.pl\0../../../etc/passwd".to_string(),
                    position: None,
                    additional_params: json!({
                        "method": "textDocument/didOpen",
                        "params": {
                            "textDocument": {
                                "uri": "file:///test/safe.pl\0../../../etc/passwd",
                                "languageId": "perl",
                                "version": 1,
                                "text": "#!/usr/bin/perl"
                            }
                        }
                    }),
                },
                payload_size_bytes: 128,
            },
            expected_behavior: SecurityExpectedBehavior::ShouldReject,
            security_category: SecurityCategory::FileSystem,
        },
    ]
}

/// Generate malicious content patterns
#[cfg(test)]
fn generate_large_malicious_content(size_bytes: usize) -> String {
    // Generate a large string that could cause memory exhaustion
    let chunk = "# This is a comment designed to consume memory\n";
    let chunks_needed = (size_bytes / chunk.len()) + 1;
    chunk.repeat(chunks_needed)
}

#[cfg(test)]
fn generate_recursive_symbols(count: usize) -> String {
    let mut content = String::from("#!/usr/bin/perl\nuse strict;\nuse warnings;\n\n");

    for i in 0..count {
        content.push_str(&format!(
            "sub recursive_function_{} {{ return recursive_function_{}(); }}\n",
            i,
            (i + 1) % count
        ));
    }

    content
}

#[cfg(test)]
fn generate_deeply_nested_structure(depth: usize) -> String {
    let mut content = String::from("#!/usr/bin/perl\nmy $deep = ");

    // Create deeply nested hash references
    for _ in 0..depth {
        content.push_str("{ nested => ");
    }

    content.push_str("'value'");

    for _ in 0..depth {
        content.push_str(" }");
    }

    content.push_str(";\n");
    content
}

/// UTF-16/UTF-8 position conversion test utilities
#[cfg(test)]
pub struct PositionConversionTestCase {
    pub name: &'static str,
    pub text: String,
    pub utf8_positions: Vec<usize>,
    pub utf16_positions: Vec<usize>,
    pub expected_conversions: Vec<(usize, usize)>, // (utf8_pos, utf16_pos)
}

#[cfg(test)]
pub fn load_position_conversion_test_cases() -> Vec<PositionConversionTestCase> {
    vec![
        PositionConversionTestCase {
            name: "emoji_surrogate_pairs",
            text: "Hello 🚀 World".to_string(),
            utf8_positions: vec![0, 6, 10, 11], // H, 🚀 start, space after emoji, W
            utf16_positions: vec![0, 6, 8, 9],  // Same but emoji takes 2 UTF-16 code units
            expected_conversions: vec![
                (0, 0),   // H -> H
                (6, 6),   // 🚀 start (same)
                (10, 8),  // Space after emoji (different due to surrogate pair)
                (11, 9),  // W (offset by 1)
            ],
        },
        PositionConversionTestCase {
            name: "mixed_unicode_ascii",
            text: "ASCII café 世界 end".to_string(),
            utf8_positions: vec![0, 6, 11, 12, 16, 17], // A, c, é, space, 世, 界
            utf16_positions: vec![0, 6, 11, 12, 16, 17], // Same (no surrogates)
            expected_conversions: vec![
                (0, 0),   // A -> A
                (6, 6),   // c -> c
                (11, 11), // é -> é (single code point)
                (12, 12), // space -> space
                (16, 16), // 世 -> 世
                (17, 17), // 界 -> 界
            ],
        },
        PositionConversionTestCase {
            name: "complex_emoji_sequence",
            text: "Family: 👨‍👩‍👧‍👦 End".to_string(), // Complex emoji with ZWJ sequences
            utf8_positions: vec![0, 8, 33, 37], // F, space, space after emoji, E
            utf16_positions: vec![0, 8, 19, 23], // Different due to complex emoji encoding
            expected_conversions: vec![
                (0, 0),   // F -> F
                (8, 8),   // space -> space
                (33, 19), // Space after emoji (big difference)
                (37, 23), // E (offset maintained)
            ],
        },
    ]
}

/// Security test validation utilities
#[cfg(test)]
pub struct SecurityTestValidator {
    pub max_memory_mb: u64,
    pub max_processing_time_ms: u64,
    pub allowed_file_patterns: Vec<String>,
    pub blocked_patterns: Vec<String>,
}

#[cfg(test)]
impl SecurityTestValidator {
    pub fn new() -> Self {
        Self {
            max_memory_mb: 100,
            max_processing_time_ms: 5000,
            allowed_file_patterns: vec![
                r"^file:///test/.*\.pl$".to_string(),
                r"^file:///test/.*\.pm$".to_string(),
                r"^file:///test/.*\.t$".to_string(),
            ],
            blocked_patterns: vec![
                r"\.\.".to_string(),                    // Path traversal
                r"/etc/".to_string(),                   // System files
                r"/proc/".to_string(),                  // Process info
                r"C:\\Windows\\".to_string(),           // Windows system
                r"\0".to_string(),                      // Null bytes
                r"%2E%2E".to_string(),                  // URL encoded ..
                r"\\x00".to_string(),                   // Hex encoded null
            ],
        }
    }

    pub fn validate_uri(&self, uri: &str) -> Result<(), String> {
        // Check for blocked patterns
        for pattern in &self.blocked_patterns {
            if uri.contains(pattern) {
                return Err(format!("URI contains blocked pattern: {}", pattern));
            }
        }

        // Check against allowed patterns
        let is_allowed = self.allowed_file_patterns.iter().any(|pattern| {
            regex::Regex::new(pattern)
                .map(|re| re.is_match(uri))
                .unwrap_or(false)
        });

        if !is_allowed {
            return Err(format!("URI not in allowed pattern list: {}", uri));
        }

        Ok(())
    }

    pub fn validate_content_size(&self, content: &str) -> Result<(), String> {
        let size_bytes = content.len();
        let max_bytes = (self.max_memory_mb * 1024 * 1024) as usize;

        if size_bytes > max_bytes {
            return Err(format!(
                "Content size {} bytes exceeds limit {} bytes",
                size_bytes, max_bytes
            ));
        }

        Ok(())
    }

    pub fn validate_position_bounds(&self, text: &str, line: u32, character: u32) -> Result<(), String> {
        let lines: Vec<&str> = text.lines().collect();

        if line as usize >= lines.len() {
            return Err(format!("Line {} exceeds text bounds (max: {})", line, lines.len() - 1));
        }

        let line_text = lines[line as usize];
        let char_count = line_text.chars().count();

        if character as usize > char_count {
            return Err(format!(
                "Character {} exceeds line bounds (max: {})",
                character, char_count
            ));
        }

        Ok(())
    }
}

/// Generate security test reports
#[cfg(test)]
pub struct SecurityTestReport {
    pub test_name: String,
    pub attack_vector: AttackVector,
    pub test_passed: bool,
    pub actual_behavior: String,
    pub security_violation: Option<String>,
    pub performance_impact: Option<PerformanceImpact>,
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub struct PerformanceImpact {
    pub memory_used_mb: f64,
    pub processing_time_ms: u64,
    pub cpu_usage_percent: f32,
}

use std::sync::LazyLock;

/// Lazy-loaded security fixture registry
#[cfg(test)]
pub static SECURITY_FIXTURE_REGISTRY: LazyLock<HashMap<&'static str, SecurityFixture>> =
    LazyLock::new(|| {
        let mut registry = HashMap::new();

        for fixture in load_security_fixtures() {
            registry.insert(fixture.name, fixture);
        }

        registry
    });

/// Get security fixture by name
#[cfg(test)]
pub fn get_security_fixture_by_name(name: &str) -> Option<&'static SecurityFixture> {
    SECURITY_FIXTURE_REGISTRY.get(name)
}

/// Get fixtures by attack vector
#[cfg(test)]
pub fn get_fixtures_by_attack_vector(attack_vector: AttackVector) -> Vec<&'static SecurityFixture> {
    SECURITY_FIXTURE_REGISTRY
        .values()
        .filter(|fixture| fixture.attack_vector == attack_vector)
        .collect()
}

/// Get fixtures by security category
#[cfg(test)]
pub fn get_fixtures_by_security_category(category: SecurityCategory) -> Vec<&'static SecurityFixture> {
    SECURITY_FIXTURE_REGISTRY
        .values()
        .filter(|fixture| fixture.security_category == category)
        .collect()
}

/// Get high-risk security fixtures
#[cfg(test)]
pub fn get_high_risk_fixtures() -> Vec<&'static SecurityFixture> {
    SECURITY_FIXTURE_REGISTRY
        .values()
        .filter(|fixture| {
            matches!(
                fixture.attack_vector,
                AttackVector::PathTraversal
                    | AttackVector::CodeInjection
                    | AttackVector::MemoryExhaustion
                    | AttackVector::FileSystemAccess
            )
        })
        .collect()
}

// Mock regex module for validation
#[cfg(test)]
mod regex {
    pub struct Regex;

    impl Regex {
        pub fn new(_pattern: &str) -> Result<Self, String> {
            Ok(Regex)
        }

        pub fn is_match(&self, _text: &str) -> bool {
            // Mock implementation - in real code, use actual regex
            true
        }
    }
}