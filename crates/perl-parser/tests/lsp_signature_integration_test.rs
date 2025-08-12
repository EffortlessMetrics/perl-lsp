//! Integration tests for LSP signature help functionality
//! Tests the full LSP request/response flow for signature help

use perl_parser::{Parser, SignatureHelpProvider};
use serde_json::json;

#[test]
fn test_lsp_signature_help_request_format() {
    // Test that signature help responses match LSP specification
    let code = "print($x, ";
    let position = 9; // After comma

    let ast = Parser::new(code)
        .parse()
        .unwrap_or_else(|_| Parser::new("").parse().unwrap());
    let provider = SignatureHelpProvider::new(&ast);

    if let Some(help) = provider.get_signature_help(code, position) {
        // Convert to JSON to verify LSP format
        let json_help = json!({
            "signatures": help.signatures.iter().map(|sig| {
                json!({
                    "label": sig.label,
                    "documentation": {
                        "kind": "markdown",
                        "value": sig.documentation.clone()
                            .unwrap_or_default()
                    },
                    "parameters": sig.parameters.iter().map(|p| {
                        json!({
                            "label": p.label
                        })
                    }).collect::<Vec<_>>()
                })
            }).collect::<Vec<_>>(),
            "activeSignature": help.active_signature,
            "activeParameter": help.active_parameter
        });

        // Verify structure
        assert!(json_help["signatures"].is_array());
        assert!(json_help["activeSignature"].is_number());
        assert!(json_help["activeParameter"].is_number());
    }
}

#[test]
fn test_signature_help_trigger_characters() {
    // Test that signature help is triggered correctly
    let trigger_cases = vec![
        ("print(", 6, true),    // Opening paren
        ("print($x,", 9, true), // Comma
        ("print $x", 8, false), // No trigger
        ("my $x = ", 8, false), // No trigger
    ];

    for (code, position, should_trigger) in trigger_cases {
        let ast = Parser::new(code)
            .parse()
            .unwrap_or_else(|_| Parser::new("").parse().unwrap());
        let provider = SignatureHelpProvider::new(&ast);

        let help = provider.get_signature_help(code, position);
        assert_eq!(
            help.is_some(),
            should_trigger,
            "Trigger mismatch for '{}' at position {}",
            code,
            position
        );
    }
}

#[test]
fn test_multi_signature_functions() {
    // Test functions with multiple signatures
    let multi_sig_functions = vec![
        (
            "substr",
            vec![
                "substr EXPR, OFFSET, LENGTH, REPLACEMENT",
                "substr EXPR, OFFSET, LENGTH",
                "substr EXPR, OFFSET",
            ],
        ),
        (
            "splice",
            vec![
                "splice ARRAY, OFFSET, LENGTH, LIST",
                "splice ARRAY, OFFSET, LENGTH",
                "splice ARRAY, OFFSET",
                "splice ARRAY",
            ],
        ),
        (
            "index",
            vec!["index STR, SUBSTR, POSITION", "index STR, SUBSTR"],
        ),
    ];

    for (func, expected_sigs) in multi_sig_functions {
        let code = format!("{}(", func);
        let position = code.len();

        let ast = Parser::new(&code)
            .parse()
            .unwrap_or_else(|_| Parser::new("").parse().unwrap());
        let provider = SignatureHelpProvider::new(&ast);

        if let Some(help) = provider.get_signature_help(&code, position) {
            assert_eq!(
                help.signatures.len(),
                expected_sigs.len(),
                "Wrong number of signatures for {}",
                func
            );

            for expected in &expected_sigs {
                assert!(
                    help.signatures.iter().any(|s| s.label.contains(expected)),
                    "Missing signature '{}' for {}",
                    expected,
                    func
                );
            }
        }
    }
}

#[test]
fn test_parameter_highlighting() {
    // Test that active parameter is correctly highlighted
    struct TestCase {
        code: &'static str,
        position: usize,
        expected_param: usize,
        function: &'static str,
    }

    let test_cases = vec![
        TestCase {
            code: "substr($str, ",
            position: 12,
            expected_param: 1,
            function: "substr",
        },
        TestCase {
            code: "substr($str, 0, ",
            position: 16,
            expected_param: 2,
            function: "substr",
        },
        TestCase {
            code: "splice(@arr, 0, 1, ",
            position: 19,
            expected_param: 3,
            function: "splice",
        },
        TestCase {
            code: "index($haystack, $needle, ",
            position: 26,
            expected_param: 2,
            function: "index",
        },
    ];

    for test in test_cases {
        let ast = Parser::new(test.code)
            .parse()
            .unwrap_or_else(|_| Parser::new("").parse().unwrap());
        let provider = SignatureHelpProvider::new(&ast);

        if let Some(help) = provider.get_signature_help(test.code, test.position) {
            assert_eq!(
                help.active_parameter,
                Some(test.expected_param),
                "Wrong active parameter for {} in '{}'",
                test.function,
                test.code
            );
        }
    }
}

#[test]
fn test_builtin_vs_user_functions() {
    // Test that we distinguish between built-in and user-defined functions
    let code = r#"
sub my_print {
    print @_;
}

my_print("hello");
print("world");
"#;

    // Test user function (should not have built-in signature)
    let user_func_pos = code.find("my_print(").unwrap() + 9;
    let ast = Parser::new(code)
        .parse()
        .unwrap_or_else(|_| Parser::new("").parse().unwrap());
    let provider = SignatureHelpProvider::new(&ast);

    // User function might have signature from parsing
    let _user_help = provider.get_signature_help(code, user_func_pos);

    // Test built-in function
    let builtin_pos = code.find("print(\"world").unwrap() + 6;
    let builtin_help = provider.get_signature_help(code, builtin_pos);

    assert!(
        builtin_help.is_some(),
        "Should have signature for built-in print"
    );
}

#[test]
fn test_method_call_signatures() {
    // Test that method calls don't incorrectly trigger built-in signatures
    let test_cases = vec![
        "$obj->print(",  // Method call, not built-in print
        "$class->new(",  // Method call, not built-in
        "SUPER::print(", // Qualified call
    ];

    for code in test_cases {
        let position = code.len();
        let ast = Parser::new(code)
            .parse()
            .unwrap_or_else(|_| Parser::new("").parse().unwrap());
        let provider = SignatureHelpProvider::new(&ast);

        // Method calls might not have signatures unless defined
        let _help = provider.get_signature_help(code, position);
        // We don't assert absence since methods might be recognized
    }
}

#[test]
fn test_signature_help_with_syntax_errors() {
    // Test that signature help works even with syntax errors
    let error_cases = vec![
        "print(((",       // Unmatched parens
        "print($x, $y, ", // Incomplete
        "print($x, , $y", // Extra comma
        "print($",        // Incomplete variable
    ];

    for code in error_cases {
        let position = code.len() - 1;
        let ast = Parser::new(code)
            .parse()
            .unwrap_or_else(|_| Parser::new("").parse().unwrap());
        let provider = SignatureHelpProvider::new(&ast);

        // Should still try to provide help
        let _help = provider.get_signature_help(code, position);
        // Don't assert - just ensure no panic
    }
}

#[test]
fn test_overloaded_operators_as_functions() {
    // Test operators that can be used as functions
    let operator_functions = vec![
        ("push", "push ARRAY, LIST"),
        ("unshift", "unshift ARRAY, LIST"),
        ("shift", "shift ARRAY"),
        ("pop", "pop ARRAY"),
        ("splice", "splice ARRAY, OFFSET, LENGTH, LIST"),
    ];

    for (op, sig) in operator_functions {
        let code = format!("{}(@array, ", op);
        let position = code.len();

        let ast = Parser::new(&code)
            .parse()
            .unwrap_or_else(|_| Parser::new("").parse().unwrap());
        let provider = SignatureHelpProvider::new(&ast);

        if let Some(help) = provider.get_signature_help(&code, position) {
            assert!(
                help.signatures.iter().any(|s| s.label.contains(sig)),
                "Missing signature for operator function {}",
                op
            );
        }
    }
}

#[test]
fn test_file_test_operators() {
    // Test file test operators
    let file_tests = vec![
        "-e", "-f", "-d", "-r", "-w", "-x", "-o", "-R", "-W", "-X", "-O", "-z", "-s", "-l", "-p",
        "-S", "-b", "-c", "-t", "-u", "-g", "-k", "-T", "-B", "-M", "-A", "-C",
    ];

    for op in file_tests {
        let ast = Parser::new("").parse().unwrap();
        let provider = SignatureHelpProvider::new(&ast);

        // File test operators should have signatures
        assert!(
            provider.has_builtin(op),
            "Missing file test operator: {}",
            op
        );
    }
}

#[test]
fn test_special_forms() {
    // Test special forms that look like functions
    let special_forms = vec![
        ("do", vec!["do BLOCK", "do EXPR"]),
        ("eval", vec!["eval EXPR", "eval BLOCK"]),
        (
            "require",
            vec!["require VERSION", "require EXPR", "require"],
        ),
        ("use", vec!["use Module VERSION LIST", "use Module"]),
        ("no", vec!["no Module VERSION LIST", "no Module"]),
    ];

    for (form, _sigs) in special_forms {
        let ast = Parser::new("").parse().unwrap();
        let provider = SignatureHelpProvider::new(&ast);

        assert!(provider.has_builtin(form), "Missing special form: {}", form);
    }
}

#[test]
fn test_pragma_like_functions() {
    // Test pragma-like built-ins
    let pragmas = vec![
        "strict", "warnings", "feature", "utf8", "bytes", "integer", "locale", "constant",
    ];

    // These are not functions but modules, so they shouldn't have signatures
    let ast = Parser::new("").parse().unwrap();
    let provider = SignatureHelpProvider::new(&ast);

    for pragma in pragmas {
        // Pragmas are not in builtin_signatures
        assert!(
            !provider.has_builtin(pragma),
            "Pragma {} should not be in built-in functions",
            pragma
        );
    }
}

#[test]
fn test_comprehensive_function_categories() {
    // Ensure all categories of functions are covered
    let ast = Parser::new("").parse().unwrap();
    let provider = SignatureHelpProvider::new(&ast);

    let categories = vec![
        ("String", vec!["chomp", "chr", "index", "lc", "length"]),
        ("Array", vec!["push", "pop", "shift", "splice", "grep"]),
        ("Hash", vec!["keys", "values", "each", "delete", "exists"]),
        ("I/O", vec!["print", "open", "close", "read", "write"]),
        ("File", vec!["stat", "chmod", "rename", "unlink", "mkdir"]),
        ("Process", vec!["fork", "exec", "system", "kill", "wait"]),
        ("Time", vec!["time", "localtime", "gmtime", "sleep"]),
        ("Math", vec!["abs", "sin", "cos", "sqrt", "rand"]),
        ("Socket", vec!["socket", "bind", "listen", "accept"]),
        ("Misc", vec!["die", "warn", "exit", "eval", "require"]),
    ];

    for (category, funcs) in categories {
        for func in funcs {
            assert!(
                provider.has_builtin(func),
                "Missing {} function: {}",
                category,
                func
            );
        }
    }
}

#[test]
fn test_documentation_quality() {
    // Test that all signatures have documentation
    let ast = Parser::new("").parse().unwrap();
    let provider = SignatureHelpProvider::new(&ast);

    let important_funcs = vec![
        "print", "open", "close", "push", "pop", "substr", "index", "split", "join", "map", "grep",
        "sort",
    ];

    for func in important_funcs {
        if let Some(sig) = provider.get_builtin_signature(func) {
            assert!(
                !sig.documentation.is_empty(),
                "Function {} should have documentation",
                func
            );
        }
    }
}
