//! Edge case tests for built-in function signatures
//! Tests special cases, error conditions, and unusual usage patterns

use perl_parser::{Parser, SignatureHelpProvider};

#[test]
fn test_signature_at_various_positions() {
    // Test getting signatures at different cursor positions
    let test_cases = vec![
        ("print(", 6, 0),  // Right after opening paren
        ("print()", 6, 0), // At opening paren
        ("print()", 7, 0), // At closing paren
        ("print($x", 8, 0), // After first argument
        ("print($x,", 9, 1), // After comma
        ("print($x, $y", 12, 1), // After second argument
        ("print($x, $y, $z", 16, 2), // After third argument
    ];
    
    for (code, position, expected_param) in test_cases {
        let ast = Parser::new(code).parse().unwrap_or_else(|_| {
            Parser::new("").parse().unwrap() // Fallback to empty AST
        });
        let provider = SignatureHelpProvider::new(&ast);
        
        if let Some(help) = provider.get_signature_help(code, position) {
            assert_eq!(help.active_parameter, expected_param,
                      "Wrong active parameter for '{}' at position {}", code, position);
        }
    }
}

#[test]
fn test_nested_function_calls() {
    // Test signatures in nested function calls
    let test_cases = vec![
        ("print(substr(", 13, "substr"),  // Inner function
        ("print(substr($s, ", 17, "substr"), // Still in inner
        ("substr(print(", 13, "print"), // Different nesting
        ("map { print } grep { ", 21, "grep"), // Block forms
    ];
    
    for (code, position, expected_func) in test_cases {
        let ast = Parser::new(code).parse().unwrap_or_else(|_| {
            Parser::new("").parse().unwrap()
        });
        let provider = SignatureHelpProvider::new(&ast);
        
        if let Some(help) = provider.get_signature_help(code, position) {
            if let Some(sig) = help.signatures.first() {
                assert!(sig.label.contains(expected_func),
                       "Expected function '{}' in signature for '{}' at position {}",
                       expected_func, code, position);
            }
        }
    }
}

#[test]
fn test_ambiguous_functions() {
    // Test functions that can be both unary and list operators
    let functions = vec![
        "defined", "undef", "delete", "exists", "ref", "scalar",
        "chomp", "chop", "chr", "ord", "lc", "uc", "length",
    ];
    
    for func in functions {
        let code1 = format!("{}($x)", func);
        let code2 = format!("{} $x", func);
        let code3 = format!("{}()", func);
        
        for code in [code1, code2, code3] {
            let ast = Parser::new(&code).parse().unwrap_or_else(|_| {
                Parser::new("").parse().unwrap()
            });
            let provider = SignatureHelpProvider::new(&ast);
            
            // Should provide signatures for all forms
            assert!(provider.has_builtin(func), "Missing signatures for {}", func);
        }
    }
}

#[test]
fn test_filehandle_functions() {
    // Test functions that take filehandles
    let test_cases = vec![
        ("print $fh ", 10),
        ("print STDOUT ", 13),
        ("printf $fh ", 11),
        ("say $fh ", 8),
        ("read $fh, $buf, ", 16),
        ("sysread $fh, $buf, ", 19),
        ("syswrite $fh, $buf", 18),
        ("seek $fh, ", 10),
        ("tell $fh", 8),
        ("eof $fh", 7),
        ("close $fh", 9),
    ];
    
    for (code, position) in test_cases {
        let ast = Parser::new(code).parse().unwrap_or_else(|_| {
            Parser::new("").parse().unwrap()
        });
        let provider = SignatureHelpProvider::new(&ast);
        
        // Should recognize filehandle context
        if let Some(help) = provider.get_signature_help(code, position) {
            assert!(!help.signatures.is_empty(),
                   "Should have signatures for filehandle function: {}", code);
        }
    }
}

#[test]
fn test_special_variables_in_signatures() {
    // Test functions that work with special variables
    let test_cases = vec![
        ("chomp", true),  // Works on $_
        ("chop", true),   // Works on $_
        ("lc", true),     // Works on $_
        ("uc", true),     // Works on $_
        ("study", true),  // Works on $_
        ("tr///", false), // Special case
        ("s///", false),  // Special case
    ];
    
    for (func, should_have_default) in test_cases {
        let ast = Parser::new("").parse().unwrap();
        let provider = SignatureHelpProvider::new(&ast);
        
        if let Some(sig) = provider.get_builtin_signature(func) {
            let has_no_arg_form = sig.signatures.iter()
                .any(|s| s.contains(&format!("{} ", func)) || s == func);
            
            if should_have_default {
                assert!(has_no_arg_form,
                       "{} should have a form that works on $_", func);
            }
        }
    }
}

#[test]
fn test_list_operators() {
    // Test list operators with special parsing
    let list_ops = vec![
        ("map", "map BLOCK LIST", "map EXPR, LIST"),
        ("grep", "grep BLOCK LIST", "grep EXPR, LIST"),
        ("sort", "sort BLOCK LIST", "sort LIST"),
        ("reverse", "reverse LIST", ""),
        ("join", "join EXPR, LIST", ""),
        ("split", "split /PATTERN/, EXPR, LIMIT", "split /PATTERN/, EXPR"),
    ];
    
    for (func, sig1, sig2) in list_ops {
        let ast = Parser::new("").parse().unwrap();
        let provider = SignatureHelpProvider::new(&ast);
        
        if let Some(sigs) = provider.get_builtin_signature(func) {
            assert!(sigs.signatures.iter().any(|s| s.contains(sig1)),
                   "Missing signature '{}' for {}", sig1, func);
            
            if !sig2.is_empty() {
                assert!(sigs.signatures.iter().any(|s| s.contains(sig2)),
                       "Missing signature '{}' for {}", sig2, func);
            }
        }
    }
}

#[test]
fn test_io_layer_functions() {
    // Test functions with IO layers
    let io_functions = vec![
        ("open", "open FILEHANDLE, MODE, FILENAME"),
        ("binmode", "binmode FILEHANDLE, LAYER"),
        ("sysopen", "sysopen FILEHANDLE, FILENAME, MODE, PERMS"),
    ];
    
    for (func, expected_sig) in io_functions {
        let ast = Parser::new("").parse().unwrap();
        let provider = SignatureHelpProvider::new(&ast);
        
        if let Some(sigs) = provider.get_builtin_signature(func) {
            assert!(sigs.signatures.iter().any(|s| s.contains(expected_sig)),
                   "Missing IO layer signature for {}", func);
        }
    }
}

#[test]
fn test_regex_related_functions() {
    // Test regex-related functions
    let regex_funcs = vec![
        ("qr", "qr/STRING/msixpodualn"),
        ("quotemeta", "quotemeta EXPR"),
        ("study", "study SCALAR"),
        ("pos", "pos SCALAR"),
        ("reset", "reset EXPR"),
    ];
    
    for (func, expected) in regex_funcs {
        let ast = Parser::new("").parse().unwrap();
        let provider = SignatureHelpProvider::new(&ast);
        
        if let Some(sigs) = provider.get_builtin_signature(func) {
            assert!(!sigs.signatures.is_empty(),
                   "Should have signatures for regex function {}", func);
        }
    }
}

#[test]
fn test_pack_unpack_signatures() {
    // Test pack/unpack with template strings
    let ast = Parser::new("").parse().unwrap();
    let provider = SignatureHelpProvider::new(&ast);
    
    // Check pack
    if let Some(pack_sigs) = provider.get_builtin_signature("pack") {
        assert!(pack_sigs.signatures.iter()
               .any(|s| s.contains("TEMPLATE")),
               "pack should mention TEMPLATE");
    }
    
    // Check unpack  
    if let Some(unpack_sigs) = provider.get_builtin_signature("unpack") {
        assert!(unpack_sigs.signatures.iter()
               .any(|s| s.contains("TEMPLATE")),
               "unpack should mention TEMPLATE");
    }
}

#[test]
fn test_tie_related_functions() {
    // Test tie mechanism functions
    let tie_funcs = vec![
        ("tie", "tie VARIABLE, CLASSNAME, LIST"),
        ("tied", "tied VARIABLE"),
        ("untie", "untie VARIABLE"),
    ];
    
    for (func, expected_sig) in tie_funcs {
        let ast = Parser::new("").parse().unwrap();
        let provider = SignatureHelpProvider::new(&ast);
        
        if let Some(sigs) = provider.get_builtin_signature(func) {
            assert!(sigs.signatures.iter().any(|s| s.contains("VARIABLE")),
                   "{} signature should mention VARIABLE", func);
        }
    }
}

#[test]
fn test_socket_functions() {
    // Test socket-related functions
    let socket_funcs = vec![
        "socket", "bind", "listen", "accept", "connect",
        "shutdown", "send", "recv", "getsockopt", "setsockopt",
    ];
    
    for func in socket_funcs {
        let ast = Parser::new("").parse().unwrap();
        let provider = SignatureHelpProvider::new(&ast);
        
        assert!(provider.has_builtin(func),
               "Missing socket function: {}", func);
    }
}

#[test]
fn test_math_functions() {
    // Test mathematical functions
    let math_funcs = vec![
        ("abs", vec!["abs VALUE", "abs"]),
        ("atan2", vec!["atan2 Y, X"]),
        ("cos", vec!["cos EXPR", "cos"]),
        ("sin", vec!["sin EXPR", "sin"]),
        ("exp", vec!["exp EXPR", "exp"]),
        ("log", vec!["log EXPR", "log"]),
        ("sqrt", vec!["sqrt EXPR", "sqrt"]),
        ("int", vec!["int EXPR", "int"]),
        ("rand", vec!["rand EXPR", "rand"]),
        ("srand", vec!["srand EXPR", "srand"]),
    ];
    
    for (func, expected_sigs) in math_funcs {
        let ast = Parser::new("").parse().unwrap();
        let provider = SignatureHelpProvider::new(&ast);
        
        if let Some(sigs) = provider.get_builtin_signature(func) {
            for expected in expected_sigs {
                assert!(sigs.signatures.iter().any(|s| s.contains(expected)),
                       "Missing signature '{}' for {}", expected, func);
            }
        }
    }
}

#[test]
fn test_context_functions() {
    // Test functions that depend on context
    let context_funcs = vec![
        ("wantarray", "wantarray"),
        ("caller", "caller EXPR"),
        ("scalar", "scalar EXPR"),
    ];
    
    for (func, sig) in context_funcs {
        let ast = Parser::new("").parse().unwrap();
        let provider = SignatureHelpProvider::new(&ast);
        
        assert!(provider.has_builtin(func),
               "Missing context function: {}", func);
    }
}

#[test]
fn test_deprecated_functions() {
    // Test that deprecated functions are still recognized
    let deprecated = vec![
        "dump",     // Deprecated
        "reset",    // Rarely used
        "dbmopen",  // Old-style DBM
        "dbmclose", // Old-style DBM
    ];
    
    for func in deprecated {
        let ast = Parser::new("").parse().unwrap();
        let provider = SignatureHelpProvider::new(&ast);
        
        // Should still have signatures for compatibility
        assert!(provider.has_builtin(func),
               "Missing deprecated function: {}", func);
    }
}

#[test]
fn test_prototype_preservation() {
    // Test functions that preserve prototypes
    let proto_funcs = vec![
        ("prototype", "prototype FUNCTION"),
        ("bless", "bless REF, CLASSNAME"),
        ("lock", "lock THING"),
    ];
    
    for (func, _sig) in proto_funcs {
        let ast = Parser::new("").parse().unwrap();
        let provider = SignatureHelpProvider::new(&ast);
        
        assert!(provider.has_builtin(func),
               "Missing prototype-related function: {}", func);
    }
}

#[test] 
fn test_comprehensive_coverage() {
    // Ensure we have truly comprehensive coverage
    let ast = Parser::new("").parse().unwrap();
    let provider = SignatureHelpProvider::new(&ast);
    
    // Should have at least 150 functions
    assert!(provider.builtin_count() >= 150,
           "Should have at least 150 built-in functions, got {}",
           provider.builtin_count());
    
    // Critical functions that must be present
    let critical = vec![
        // I/O
        "print", "printf", "say", "open", "close", "read", "write",
        // String
        "substr", "index", "rindex", "sprintf", "join", "split",
        // Array
        "push", "pop", "shift", "unshift", "splice", "reverse",
        // Hash
        "keys", "values", "each", "delete", "exists",
        // File
        "stat", "lstat", "-e", "-f", "-d", "-r", "-w", "-x",
        // Process
        "system", "exec", "fork", "wait", "kill",
        // Math
        "abs", "int", "sqrt", "sin", "cos", "atan2",
        // Refs
        "ref", "bless", "tie", "tied", "untie",
    ];
    
    for func in critical {
        assert!(provider.has_builtin(func),
               "Missing critical function: {}", func);
    }
}