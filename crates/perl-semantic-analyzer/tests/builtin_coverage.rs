use perl_parser_core::Parser;
use perl_semantic_analyzer::analysis::semantic::{SemanticAnalyzer, SemanticTokenModifier, SemanticTokenType};

#[test]
fn test_builtin_coverage_expansion() {
    // A mix of built-ins that were previously covered (print), previously missed (mkdir, chmod),
    // and the special case (blessed).
    let code = r#"
print "hello";
mkdir("dir");
chmod 0755, "file";
socket(S, 1, 1, 1);
blessed($obj);
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse().expect("Failed to parse");
    let analyzer = SemanticAnalyzer::analyze(&ast);
    let tokens = analyzer.semantic_tokens();

    // Helper to find token for a given function name
    let find_token = |name: &str| {
        tokens.iter().find(|t| {
            // Find tokens that correspond to the function name in the source
            let start = t.location.start;
            let end = t.location.end;
            if end > code.len() { return false; }
            let token_text = &code[start..end];
            // If the token text is exactly the name, good.
            // If the token text starts with the name and follows with space or parens, maybe that's how it works?
            // But let's assume strict match first.
            token_text == name
        })
    };

    // 1. Verify 'print' (was already covered)
    let print_token = find_token("print").expect("print token not found");
    assert_eq!(print_token.token_type, SemanticTokenType::Function);
    assert!(print_token.modifiers.contains(&SemanticTokenModifier::DefaultLibrary),
            "print should have DefaultLibrary modifier");

    // 2. Verify 'mkdir' (was missing)
    let mkdir_token = find_token("mkdir").expect("mkdir token not found");
    assert_eq!(mkdir_token.token_type, SemanticTokenType::Function);
    // This assertion is expected to fail before the fix
    assert!(mkdir_token.modifiers.contains(&SemanticTokenModifier::DefaultLibrary),
            "mkdir should have DefaultLibrary modifier");

    // 3. Verify 'chmod' (was missing)
    let chmod_token = find_token("chmod").expect("chmod token not found");
    assert_eq!(chmod_token.token_type, SemanticTokenType::Function);
    // This assertion is expected to fail before the fix
    assert!(chmod_token.modifiers.contains(&SemanticTokenModifier::DefaultLibrary),
            "chmod should have DefaultLibrary modifier");

    // 4. Verify 'socket' (was missing)
    let socket_token = find_token("socket").expect("socket token not found");
    assert_eq!(socket_token.token_type, SemanticTokenType::Function);
    // This assertion is expected to fail before the fix
    assert!(socket_token.modifiers.contains(&SemanticTokenModifier::DefaultLibrary),
            "socket should have DefaultLibrary modifier");

    // 5. Verify 'blessed' (special case, not in PHF but handled manually)
    let blessed_token = find_token("blessed").expect("blessed token not found");
    assert_eq!(blessed_token.token_type, SemanticTokenType::Function);
    assert!(blessed_token.modifiers.contains(&SemanticTokenModifier::DefaultLibrary),
            "blessed should have DefaultLibrary modifier");
}
