// Integration test for improved completion with thread safety
use perl_parser::{CompletionProvider, Parser};

#[test]
fn test_completion_array_variables_thread_safe() {
    let code = r#"
my @array_items = (1, 2, 3);
my @array_data = ("a", "b", "c");
my $scalar_var = 42;

@ar
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();
    
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions_with_path_cancellable(
        code, 
        code.len() - 1, 
        None, 
        &|| false // Not cancelled
    );
    
    // Should find both array variables
    let array_completions: Vec<_> = completions.iter()
        .filter(|c| c.label.starts_with("@array_"))
        .collect();
    
    assert!(!array_completions.is_empty(), "Should find array completions");
    println!("Found array completions: {:?}", array_completions.iter().map(|c| &c.label).collect::<Vec<_>>());
}

#[test] 
fn test_completion_cancellation_handling() {
    let code = r#"
my @array_items = (1, 2, 3);
my @array_data = ("a", "b", "c");
my $scalar_var = 42;

@ar
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();
    
    let provider = CompletionProvider::new(&ast);
    
    // Test with cancellation immediately
    let completions_cancelled = provider.get_completions_with_path_cancellable(
        code, 
        code.len() - 1, 
        None, 
        &|| true // Always cancelled
    );
    
    // Should return empty due to cancellation
    assert!(completions_cancelled.is_empty(), "Should return empty when cancelled");
    
    // Test without cancellation
    let completions_normal = provider.get_completions_with_path_cancellable(
        code, 
        code.len() - 1, 
        None, 
        &|| false // Never cancelled
    );
    
    assert!(!completions_normal.is_empty(), "Should return completions when not cancelled");
}

#[test]
fn test_completion_builtin_functions_thread_safe() {
    let code = "pr";
    
    let mut parser = Parser::new("");
    let ast = parser.parse().unwrap();
    
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions_with_path_cancellable(
        code, 
        code.len(), 
        None, 
        &|| false // Not cancelled
    );
    
    // Should find print and printf
    let print_completions: Vec<_> = completions.iter()
        .filter(|c| c.label.starts_with("print"))
        .collect();
    
    assert!(!print_completions.is_empty(), "Should find print-related builtin completions");
    println!("Found print completions: {:?}", print_completions.iter().map(|c| &c.label).collect::<Vec<_>>());
}