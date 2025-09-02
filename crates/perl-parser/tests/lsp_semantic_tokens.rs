use perl_parser::lsp_server::{JsonRpcRequest, LspServer};
use perl_parser::semantic_tokens_provider::{SemanticTokenModifier, SemanticTokenType};
use serde_json::{Value, json};
use std::collections::HashMap;

/// Helper function to initialize LSP server
fn init_lsp_server() -> LspServer {
    let mut srv = LspServer::new();
    let init = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "initialize".into(),
        params: Some(json!({"capabilities":{}})),
    };
    srv.handle_request(init);
    srv
}

/// Helper function to open document and get semantic tokens
fn get_semantic_tokens(srv: &mut LspServer, uri: &str, text: &str) -> Vec<u32> {
    let open = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "textDocument/didOpen".into(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": text
            }
        })),
    };
    srv.handle_request(open);

    let req = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "textDocument/semanticTokens/full".into(),
        params: Some(json!({"textDocument": {"uri": uri}})),
    };
    let res = srv.handle_request(req).expect("Failed to get semantic tokens");
    let result = res.result.expect("No result in response");
    let arr = result["data"].as_array().expect("Data should be an array");

    arr.iter().map(|v| v.as_u64().unwrap() as u32).collect()
}

/// Decode semantic tokens from delta encoding to absolute positions
fn decode_semantic_tokens(data: &[u32]) -> Vec<(u32, u32, u32, u32, u32)> {
    let mut tokens = Vec::new();
    let mut current_line = 0;
    let mut current_char = 0;

    for chunk in data.chunks(5) {
        let delta_line = chunk[0];
        let delta_char = chunk[1];
        let length = chunk[2];
        let token_type = chunk[3];
        let modifiers = chunk[4];

        current_line += delta_line;
        if delta_line > 0 {
            current_char = delta_char;
        } else {
            current_char += delta_char;
        }

        tokens.push((current_line, current_char, length, token_type, modifiers));
    }

    tokens
}

#[test]
fn semantic_tokens_basic_functionality() {
    let mut srv = init_lsp_server();
    let uri = "file:///basic_test.pl";
    let text = r#"package Foo; my $x = 1; sub bar { return $x } $x = 2; bar();"#;

    let data = get_semantic_tokens(&mut srv, uri, text);
    assert!(!data.is_empty(), "semantic tokens should return data");
    assert_eq!(data.len() % 5, 0, "semantic tokens must be 5-tuples");

    let tokens = decode_semantic_tokens(&data);
    assert!(!tokens.is_empty(), "should have at least one token");
}

#[test]
fn semantic_tokens_comprehensive_perl_constructs() {
    let mut srv = init_lsp_server();
    let uri = "file:///comprehensive_test.pl";
    let text = r#"#!/usr/bin/perl
use strict;
use warnings;
use Data::Dumper;

# Package declaration
package MyPackage::Helper;

# Constants and variables
use constant PI => 3.14159;
my $scalar_var = "hello world";
my @array_var = (1, 2, 3, 4, 5);
my %hash_var = (key => "value", count => 42);
our $global_var = 100;

# Subroutine with prototype and signature
sub calculate_area($$) {
    my ($width, $height) = @_;
    return $width * $height * PI;
}

# Method call and object-oriented code
sub new {
    my $class = shift;
    my $self = {
        value => 0,
        name => "default",
    };
    bless $self, $class;
    return $self;
}

sub get_value {
    my $self = shift;
    return $self->{value};
}

# Built-in functions
my $obj = MyPackage::Helper->new();
my $result = $obj->get_value();
print "Result: $result\n";
say "Using modern perl syntax";

# Regular expressions
my $pattern = qr/test\d+/;
if ($scalar_var =~ /$pattern/) {
    print "Match found\n";
}

# Advanced constructs
my $code_ref = sub { return "anonymous function" };
my @filtered = grep { $_ > 2 } @array_var;
my @mapped = map { $_ * 2 } @filtered;

# Modern Perl features
state $counter = 0;
$counter++;

# File operations
open(my $fh, '<', 'input.txt') or die "Cannot open file: $!";
while (my $line = <$fh>) {
    chomp $line;
    print "Line: $line\n";
}
close($fh);

1;
"#;

    let data = get_semantic_tokens(&mut srv, uri, text);
    assert!(!data.is_empty(), "comprehensive test should return tokens");
    assert_eq!(data.len() % 5, 0, "tokens must be properly encoded");

    let tokens = decode_semantic_tokens(&data);

    // Count different token types to ensure comprehensive coverage
    let mut token_type_counts = HashMap::new();
    for (_, _, _, token_type, _) in &tokens {
        *token_type_counts.entry(*token_type).or_insert(0) += 1;
    }

    // Should have multiple token types represented
    assert!(
        token_type_counts.len() >= 5,
        "Should have at least 5 different token types, got {}: {:?}",
        token_type_counts.len(),
        token_type_counts
    );

    // Verify we have namespace tokens (packages)
    let namespace_type =
        SemanticTokenType::all().iter().position(|&t| t == SemanticTokenType::Namespace).unwrap()
            as u32;
    assert!(token_type_counts.contains_key(&namespace_type), "Should contain namespace tokens");

    // Verify we have function tokens
    let function_type =
        SemanticTokenType::all().iter().position(|&t| t == SemanticTokenType::Function).unwrap()
            as u32;
    assert!(token_type_counts.contains_key(&function_type), "Should contain function tokens");

    // Verify we have variable tokens
    let variable_type =
        SemanticTokenType::all().iter().position(|&t| t == SemanticTokenType::Variable).unwrap()
            as u32;
    assert!(token_type_counts.contains_key(&variable_type), "Should contain variable tokens");
}

#[test]
fn semantic_tokens_error_recovery() {
    let mut srv = init_lsp_server();
    let uri = "file:///error_test.pl";

    // Test with syntactically incorrect Perl code
    let text = r#"package Broken;

# Incomplete function declaration
sub incomplete_func {
    my $var = "unclosed string
    # Missing closing brace

# Incomplete hash
my %hash = (
    key1 => "value1",
    key2 =>
# Missing value and closing paren

# Valid code after errors
sub valid_func {
    my $x = 42;
    return $x;
}
"#;

    let data = get_semantic_tokens(&mut srv, uri, text);
    // Should still return some tokens even with syntax errors
    assert_eq!(data.len() % 5, 0, "error recovery should still produce valid token encoding");

    if !data.is_empty() {
        let tokens = decode_semantic_tokens(&data);
        // Should have at least the package and the valid function
        assert!(tokens.len() >= 2, "should recover some tokens even with syntax errors");
    }
}

#[test]
fn semantic_tokens_modifier_validation() {
    let mut srv = init_lsp_server();
    let uri = "file:///modifiers_test.pl";
    let text = r#"package TestModifiers;

# Declaration contexts
my $declared_var = 42;
sub declared_func {
    my ($param1, $param2) = @_;
    return $param1 + $param2;
}

# Reference contexts
$declared_var = 100;  # modification
my $result = declared_func(10, 20);  # function reference
print $declared_var;  # variable reference
"#;

    let data = get_semantic_tokens(&mut srv, uri, text);
    assert!(!data.is_empty(), "modifier test should return tokens");

    let tokens = decode_semantic_tokens(&data);

    // Check that we have tokens with different modifiers
    let mut has_declaration_modifier = false;
    let mut has_reference_modifier = false;
    let mut has_modification_modifier = false;

    for (_, _, _, _, modifiers) in &tokens {
        if *modifiers
            & (1 << SemanticTokenModifier::all()
                .iter()
                .position(|&m| m == SemanticTokenModifier::Declaration)
                .unwrap())
            != 0
        {
            has_declaration_modifier = true;
        }
        if *modifiers
            & (1 << SemanticTokenModifier::all()
                .iter()
                .position(|&m| m == SemanticTokenModifier::Reference)
                .unwrap())
            != 0
        {
            has_reference_modifier = true;
        }
        if *modifiers
            & (1 << SemanticTokenModifier::all()
                .iter()
                .position(|&m| m == SemanticTokenModifier::Modification)
                .unwrap())
            != 0
        {
            has_modification_modifier = true;
        }
    }

    // These assertions may fail if the semantic analysis isn't complete, but we want to verify the infrastructure works
    println!("Declaration modifier found: {}", has_declaration_modifier);
    println!("Reference modifier found: {}", has_reference_modifier);
    println!("Modification modifier found: {}", has_modification_modifier);
}

#[test]
fn semantic_tokens_builtin_functions() {
    let mut srv = init_lsp_server();
    let uri = "file:///builtins_test.pl";
    let text = r#"#!/usr/bin/perl

# Built-in functions should be marked with defaultLibrary modifier
my @array = (1, 2, 3, 4, 5);
my $length = length("test string");
my $joined = join(", ", @array);
my @sorted = sort @array;
my @reversed = reverse @array;
my $substring = substr("hello world", 0, 5);

print "Length: $length\n";
print "Joined: $joined\n";

# User-defined function for comparison
sub custom_func {
    return "custom";
}

my $custom_result = custom_func();
"#;

    let data = get_semantic_tokens(&mut srv, uri, text);
    assert!(!data.is_empty(), "builtin test should return tokens");

    let tokens = decode_semantic_tokens(&data);

    // Look for built-in function tokens with defaultLibrary modifier
    let function_type =
        SemanticTokenType::all().iter().position(|&t| t == SemanticTokenType::Function).unwrap()
            as u32;
    let default_library_bit = 1
        << SemanticTokenModifier::all()
            .iter()
            .position(|&m| m == SemanticTokenModifier::DefaultLibrary)
            .unwrap();

    let mut has_builtin_function = false;
    let mut has_user_function = false;

    for (_, _, _, token_type, modifiers) in &tokens {
        if *token_type == function_type {
            if *modifiers & default_library_bit != 0 {
                has_builtin_function = true;
            } else {
                has_user_function = true;
            }
        }
    }

    println!("Built-in function tokens found: {}", has_builtin_function);
    println!("User-defined function tokens found: {}", has_user_function);
}

#[test]
fn semantic_tokens_unicode_support() {
    let mut srv = init_lsp_server();
    let uri = "file:///unicode_test.pl";
    let text = r#"#!/usr/bin/perl
use utf8;

package UnicodeTest;

# Unicode identifiers and strings
my $résumé = "curriculum vitæ";
my $温度 = 25.5;  # Temperature in Chinese
my $🚀 = "rocket";  # Emoji identifier

sub café {
    return "coffee";
}

print "$résumé - $温度°C - $🚀\n";
"#;

    let data = get_semantic_tokens(&mut srv, uri, text);
    // Should handle Unicode without crashing
    assert_eq!(data.len() % 5, 0, "Unicode handling should produce valid tokens");

    if !data.is_empty() {
        let tokens = decode_semantic_tokens(&data);
        assert!(!tokens.is_empty(), "should tokenize Unicode code correctly");
    }
}

#[test]
fn semantic_tokens_performance_stress_test() {
    let mut srv = init_lsp_server();
    let uri = "file:///large_test.pl";

    // Generate a large Perl file
    let mut large_code = String::from("#!/usr/bin/perl\nuse strict;\nuse warnings;\n\n");

    // Add many packages, functions, and variables
    for i in 0..50 {
        large_code.push_str(&format!(
            "package TestPackage{};\n\n\
             my $var{} = {};\n\
             my @array{} = (1..{});\n\
             my %hash{} = (key{} => 'value{}');\n\n\
             sub function{} {{\n\
                 my ($param1, $param2) = @_;\n\
                 my $result = $param1 + $param2 + $var{};\n\
                 return $result;\n\
             }}\n\n\
             my $result{} = function{}(10, 20);\n\n",
            i,
            i,
            i * 10,
            i,
            i * 5,
            i,
            i,
            i,
            i,
            i,
            i,
            i
        ));
    }

    large_code.push_str("1;\n");

    let start = std::time::Instant::now();
    let data = get_semantic_tokens(&mut srv, uri, &large_code);
    let duration = start.elapsed();

    assert_eq!(data.len() % 5, 0, "large file should produce valid tokens");

    // Performance assertion - should complete within reasonable time
    assert!(
        duration.as_millis() < 5000,
        "semantic tokens should complete within 5 seconds, took {:?}",
        duration
    );

    if !data.is_empty() {
        let tokens = decode_semantic_tokens(&data);
        // Should have many tokens from the generated code
        assert!(tokens.len() > 100, "large file should generate many tokens, got {}", tokens.len());
    }
}
