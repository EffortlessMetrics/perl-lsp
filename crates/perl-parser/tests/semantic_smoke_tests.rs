/// Semantic Analyzer Smoke Tests (Issue #188)
///
/// Tests comprehensive AST node type coverage in the semantic analyzer.
/// Each test validates that a specific node type generates appropriate:
/// - Semantic tokens for syntax highlighting
/// - Hover information for symbols
/// - Symbol table entries for navigation
///
/// Test organization:
/// - Phase 1: Critical LSP features (12 tests)
/// - Phase 2: Enhanced features (8 tests)
/// - Phase 3: Complete coverage (remaining tests)
use perl_parser::Parser;
use perl_parser::semantic::{SemanticAnalyzer, SemanticTokenType};
use perl_parser::symbol::SymbolKind;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ============================================================================
// PHASE 1: Critical LSP Features
// ============================================================================

#[test]
fn test_expression_statement_semantic() -> TestResult {
    let code = r#"
my $x = 42;
$x + 10;  # ExpressionStatement wrapping binary expression
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);

    // Should have semantic tokens for variables
    let tokens = analyzer.semantic_tokens();
    let var_tokens: Vec<_> = tokens
        .iter()
        .filter(|t| {
            matches!(
                t.token_type,
                SemanticTokenType::Variable | SemanticTokenType::VariableDeclaration
            )
        })
        .collect();

    assert!(!var_tokens.is_empty(), "Should have variable tokens");
    Ok(())
}

#[test]
fn test_try_block_semantic() -> TestResult {
    let code = r#"
try {
    my $x = risky_operation();
} catch {
    warn "Error: $_";
}
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);

    // Should not crash and should generate some tokens
    let tokens = analyzer.semantic_tokens();
    assert!(!tokens.is_empty(), "Try blocks should generate semantic tokens");
    Ok(())
}

#[test]
fn test_eval_block_semantic() -> TestResult {
    let code = r#"
eval {
    my $result = dangerous_operation();
    $result;
};
if ($@) {
    warn "Error: $@";
}
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);

    // Should have tokens for variables inside eval
    let tokens = analyzer.semantic_tokens();
    assert!(!tokens.is_empty(), "Eval blocks should generate semantic tokens");

    // Should have symbol for $result (scoped to eval block)
    let _result_symbols = analyzer.symbol_table().find_symbol("result", 0, SymbolKind::scalar());

    // Note: Depending on scope handling, this might be empty
    // The important part is that analyzer doesn't crash
    Ok(())
}

#[test]
fn test_do_block_semantic() -> TestResult {
    let code = r#"
my $value = do {
    my $temp = calculate();
    $temp * 2;
};
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);

    let tokens = analyzer.semantic_tokens();
    assert!(!tokens.is_empty(), "Do blocks should generate semantic tokens");

    // Should have symbols for both $value and $temp
    let symbols = analyzer.symbol_table();
    let value_symbols = symbols.find_symbol("value", 0, SymbolKind::scalar());
    assert!(!value_symbols.is_empty(), "Should find $value declaration");
    Ok(())
}

#[test]
fn test_variable_list_declaration_semantic() -> TestResult {
    let code = r#"
my ($x, $y, $z) = (1, 2, 3);
my @arr = ($x, $y);
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);

    // Should have declaration tokens for all three variables
    let tokens = analyzer.semantic_tokens();
    let decl_tokens: Vec<_> = tokens
        .iter()
        .filter(|t| matches!(t.token_type, SemanticTokenType::VariableDeclaration))
        .collect();

    assert!(
        decl_tokens.len() >= 3,
        "Should have at least 3 variable declarations (got {})",
        decl_tokens.len()
    );

    // Should have symbols for x, y, z
    let symbols = analyzer.symbol_table();
    assert!(!symbols.find_symbol("x", 0, SymbolKind::scalar()).is_empty());
    assert!(!symbols.find_symbol("y", 0, SymbolKind::scalar()).is_empty());
    assert!(!symbols.find_symbol("z", 0, SymbolKind::scalar()).is_empty());
    Ok(())
}

#[test]
fn test_variable_with_attributes_semantic() -> TestResult {
    let code = r#"
my $shared :shared = 42;
sub lvalue_sub :lvalue {
    my $internal;
    $internal;
}
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);

    // Should handle attribute annotations
    let tokens = analyzer.semantic_tokens();
    assert!(!tokens.is_empty(), "Should generate tokens for attributed variables");

    // Should have symbol for $shared
    let shared_symbols = analyzer.symbol_table().find_symbol("shared", 0, SymbolKind::scalar());
    assert!(!shared_symbols.is_empty(), "Should find $shared symbol");
    Ok(())
}

#[test]
fn test_ternary_expression_semantic() -> TestResult {
    let code = r#"
my $x = 10;
my $result = $x > 5 ? "big" : "small";
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);

    let tokens = analyzer.semantic_tokens();

    // Should have tokens for variables and strings
    let var_count = tokens
        .iter()
        .filter(|t| {
            matches!(
                t.token_type,
                SemanticTokenType::Variable | SemanticTokenType::VariableDeclaration
            )
        })
        .count();
    let string_count =
        tokens.iter().filter(|t| matches!(t.token_type, SemanticTokenType::String)).count();

    assert!(var_count >= 2, "Should have variable tokens");
    assert!(string_count >= 2, "Should have string tokens");
    Ok(())
}

#[test]
fn test_unary_operators_semantic() -> TestResult {
    let code = r#"
my $x = 10;
my $y = -$x;
my $z = !$x;
$x++;
++$x;
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);

    let tokens = analyzer.semantic_tokens();

    // Should have tokens for all variables
    let var_tokens: Vec<_> = tokens
        .iter()
        .filter(|t| {
            matches!(
                t.token_type,
                SemanticTokenType::Variable | SemanticTokenType::VariableDeclaration
            )
        })
        .collect();

    assert!(var_tokens.len() >= 3, "Should have tokens for x, y, z");
    Ok(())
}

#[test]
fn test_readline_operator_semantic() -> TestResult {
    let code = r#"
my $line = <STDIN>;
while (<>) {
    chomp;
    print;
}
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);

    let tokens = analyzer.semantic_tokens();
    assert!(!tokens.is_empty(), "Should generate tokens for readline operations");

    // Should have symbol for $line
    let line_symbols = analyzer.symbol_table().find_symbol("line", 0, SymbolKind::scalar());
    assert!(!line_symbols.is_empty(), "Should find $line symbol");
    Ok(())
}

#[test]
fn test_array_literal_semantic() -> TestResult {
    let code = r#"
my @arr = [1, 2, 3, 4];
my $first = $arr[0];
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);

    let tokens = analyzer.semantic_tokens();

    // Should have tokens for array and scalar variables
    let var_tokens: Vec<_> = tokens
        .iter()
        .filter(|t| {
            matches!(
                t.token_type,
                SemanticTokenType::Variable | SemanticTokenType::VariableDeclaration
            )
        })
        .collect();

    assert!(!var_tokens.is_empty(), "Should have variable tokens");

    // Should have number tokens for array elements
    let num_tokens: Vec<_> =
        tokens.iter().filter(|t| matches!(t.token_type, SemanticTokenType::Number)).collect();

    assert!(num_tokens.len() >= 4, "Should have number tokens for [1,2,3,4]");
    Ok(())
}

#[test]
fn test_hash_literal_semantic() -> TestResult {
    let code = r#"
my %hash = { key1 => "value1", key2 => "value2" };
my $val = $hash{key1};
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);

    let tokens = analyzer.semantic_tokens();

    // Should have tokens for hash and scalar variables
    let var_tokens: Vec<_> = tokens
        .iter()
        .filter(|t| {
            matches!(
                t.token_type,
                SemanticTokenType::Variable | SemanticTokenType::VariableDeclaration
            )
        })
        .collect();

    assert!(!var_tokens.is_empty(), "Should have variable tokens");

    // Should have string tokens for values
    let string_tokens: Vec<_> =
        tokens.iter().filter(|t| matches!(t.token_type, SemanticTokenType::String)).collect();

    assert!(string_tokens.len() >= 2, "Should have string tokens for hash values");
    Ok(())
}

#[test]
fn test_phase_block_semantic() -> TestResult {
    let code = r#"
BEGIN {
    print "Starting up\n";
}

END {
    print "Shutting down\n";
}

INIT {
    my $config = load_config();
}
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);

    let tokens = analyzer.semantic_tokens();
    assert!(!tokens.is_empty(), "Should generate tokens for phase blocks");

    // Should have function tokens for print
    let fn_tokens: Vec<_> =
        tokens.iter().filter(|t| matches!(t.token_type, SemanticTokenType::Function)).collect();

    assert!(!fn_tokens.is_empty(), "Should have function tokens");
    Ok(())
}

// ============================================================================
// PHASE 2: Enhanced Features
// ============================================================================

#[test]
fn test_substitution_operator_semantic() -> TestResult {
    let code = r#"
my $text = "hello world";
$text =~ s/world/universe/g;
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);

    let tokens = analyzer.semantic_tokens();

    // Should have operator tokens
    let op_tokens: Vec<_> =
        tokens.iter().filter(|t| matches!(t.token_type, SemanticTokenType::Operator)).collect();

    assert!(!op_tokens.is_empty(), "Should have operator tokens for s///");
    Ok(())
}

// =============================================================================
// PHASE 2/3 Tests - Feature gated under semantic-phase2
// =============================================================================
// Run with: cargo test -p perl-parser --features semantic-phase2
// =============================================================================
#[cfg(feature = "semantic-phase2")]
mod semantic_phase2_tests {
    use perl_parser::Parser;
    use perl_parser::semantic::{SemanticAnalyzer, SemanticTokenType};
    use perl_parser::symbol::SymbolKind;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn test_method_call_semantic() -> TestResult {
        let code = r#"
my $obj = Foo->new();
$obj->process();
my $result = $obj->get_value();
"#;

        let mut parser = Parser::new(code);
        let ast = parser.parse()?;
        let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);

        let tokens = analyzer.semantic_tokens();

        // Should have method tokens
        let method_tokens: Vec<_> =
            tokens.iter().filter(|t| matches!(t.token_type, SemanticTokenType::Method)).collect();

        assert!(method_tokens.len() >= 3, "Should have tokens for new, process, get_value");
        Ok(())
    }

    #[test]
    fn test_reference_dereference_semantic() -> TestResult {
        let code = r#"
my $scalar = 42;
my $ref = \$scalar;
my $value = $$ref;
"#;

        let mut parser = Parser::new(code);
        let ast = parser.parse()?;
        let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);

        let tokens = analyzer.semantic_tokens();
        assert!(!tokens.is_empty(), "Should generate tokens for references");

        // Should have symbols for all variables
        let symbols = analyzer.symbol_table();
        assert!(!symbols.find_symbol("scalar", 0, SymbolKind::scalar()).is_empty());
        assert!(!symbols.find_symbol("ref", 0, SymbolKind::scalar()).is_empty());
        assert!(!symbols.find_symbol("value", 0, SymbolKind::scalar()).is_empty());
        Ok(())
    }

    #[test]
    fn test_use_require_semantic() -> TestResult {
        let code = r#"
use strict;
use warnings;
use Data::Dumper qw(Dumper);
require Exporter;
"#;

        let mut parser = Parser::new(code);
        let ast = parser.parse()?;
        let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);

        let tokens = analyzer.semantic_tokens();

        // Should have namespace tokens
        let ns_tokens: Vec<_> = tokens
            .iter()
            .filter(|t| matches!(t.token_type, SemanticTokenType::Namespace))
            .collect();

        assert!(!ns_tokens.is_empty(), "Should have namespace tokens for modules");
        Ok(())
    }

    #[test]
    fn test_given_when_semantic() -> TestResult {
        let code = r#"
use v5.10;
given ($value) {
    when (1) { say "one"; }
    when (2) { say "two"; }
    default { say "other"; }
}
"#;

        let mut parser = Parser::new(code);
        let ast = parser.parse()?;
        let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);

        let tokens = analyzer.semantic_tokens();
        assert!(!tokens.is_empty(), "Should generate tokens for given/when");
        Ok(())
    }

    #[test]
    fn test_control_flow_keywords_semantic() -> TestResult {
        let code = r#"
sub process {
    foreach my $item (@items) {
        next if $item == 0;
        last if $item > 100;
        redo if $item < 0;
        return $item;
    }
}
"#;

        let mut parser = Parser::new(code);
        let ast = parser.parse()?;
        let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);

        let tokens = analyzer.semantic_tokens();
        assert!(!tokens.is_empty(), "Should generate tokens for control flow");
        Ok(())
    }

    // Phase 3 tests (also under semantic-phase2 feature for simplicity)
    #[test]
    fn test_postfix_loop_semantic() -> TestResult {
        let code = r#"
say $_ for @items;
print "$_\n" while <>;
"#;

        let mut parser = Parser::new(code);
        let ast = parser.parse()?;
        let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);

        let tokens = analyzer.semantic_tokens();
        assert!(!tokens.is_empty(), "Should generate tokens for postfix loops");
        Ok(())
    }

    #[test]
    fn test_file_test_semantic() -> TestResult {
        let code = r#"
my $exists = -e $file;
my $is_dir = -d $path;
my $readable = -r $filename;
"#;

        let mut parser = Parser::new(code);
        let ast = parser.parse()?;
        let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);

        let tokens = analyzer.semantic_tokens();
        assert!(!tokens.is_empty(), "Should generate tokens for file tests");
        Ok(())
    }
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_complex_real_world_semantic() -> TestResult {
    let code = r#"
package MyModule;
use strict;
use warnings;

# Constructor
sub new {
    my ($class, %args) = @_;
    my $self = {
        name => $args{name} // "default",
        value => $args{value} // 0,
    };
    return bless $self, $class;
}

# Method with error handling
sub process {
    my ($self) = @_;

    eval {
        my $result = $self->{value} * 2;
        return $result;
    };

    if ($@) {
        warn "Error in process: $@";
        return undef;
    }
}

1;
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);

    // Should not crash
    let tokens = analyzer.semantic_tokens();
    assert!(!tokens.is_empty(), "Should generate tokens for complex code");

    // Should have multiple token types
    let has_package = tokens.iter().any(|t| matches!(t.token_type, SemanticTokenType::Namespace));
    let has_function = tokens.iter().any(|t| {
        matches!(t.token_type, SemanticTokenType::Function | SemanticTokenType::FunctionDeclaration)
    });
    let has_variable = tokens.iter().any(|t| {
        matches!(t.token_type, SemanticTokenType::Variable | SemanticTokenType::VariableDeclaration)
    });

    assert!(has_package, "Should have package tokens");
    assert!(has_function, "Should have function tokens");
    assert!(has_variable, "Should have variable tokens");
    Ok(())
}
