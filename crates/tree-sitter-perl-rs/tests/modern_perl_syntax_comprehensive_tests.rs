//! Modern Perl Syntax Comprehensive Test Scaffolding
//!
//! Tests feature spec: SPEC_144_IGNORED_TESTS_ARCHITECTURAL_BLUEPRINT.md#modern-perl-syntax-support-framework
//!
//! This test suite validates comprehensive modern Perl feature parsing with incremental
//! integration for subroutine signatures, postfix dereferencing, and state variables.
//!
//! AC7: Subroutine Signature Parser
//! AC8: Postfix Dereferencing Parser
//! AC9: State Variable Parser
//! AC11: Unicode Identifier Parser
//!
//! Note: These tests require the `modern-perl-syntax` feature and tree-sitter linkage.
//! Run with: cargo test -p tree-sitter-perl-rs --features modern-perl-syntax

/// Modern Perl syntax support definition
#[derive(Debug, Clone)]
pub struct ModernPerlSyntaxSupport {
    /// Subroutine signature support
    pub subroutine_signatures: SubroutineSignatureSupport,
    /// Postfix dereferencing support
    pub postfix_dereferencing: PostfixDereferencingSupport,
    /// State variable support
    pub state_variables: StateVariableSupport,
    /// Unicode identifier support
    pub unicode_identifiers: UnicodeIdentifierSupport,
    /// Advanced operator support
    pub advanced_operators: AdvancedOperatorSupport,
}

#[derive(Debug, Clone)]
pub struct SubroutineSignatureSupport {
    /// Basic parameter parsing
    pub basic_parameters: bool,
    /// Default value support
    pub default_values: bool,
    /// Slurpy parameters (@, %)
    pub slurpy_parameters: bool,
    /// Named parameters (:$name)
    pub named_parameters: bool,
    /// Type annotations
    pub type_annotations: bool,
    /// Constraint validation
    pub constraint_validation: bool,
}

#[derive(Debug, Clone)]
pub struct PostfixDereferencingSupport {
    /// Array postfix deref (->@*)
    pub array_deref: bool,
    /// Hash postfix deref (->%*)
    pub hash_deref: bool,
    /// Scalar postfix deref (->$*)
    pub scalar_deref: bool,
    /// Code postfix deref (->&*)
    pub code_deref: bool,
    /// Chained dereferencing
    pub chained_deref: bool,
    /// Precedence handling
    pub precedence_support: bool,
}

#[derive(Debug, Clone)]
pub struct StateVariableSupport {
    /// Basic state declaration
    pub basic_state: bool,
    /// State with initialization
    pub state_initialization: bool,
    /// State in subroutine context
    pub subroutine_state: bool,
    /// State scope tracking
    pub scope_tracking: bool,
    /// State persistence validation
    pub persistence_validation: bool,
}

#[derive(Debug, Clone)]
pub struct UnicodeIdentifierSupport {
    /// Basic Unicode identifier parsing
    pub basic_unicode: bool,
    /// Emoji identifier support
    pub emoji_identifiers: bool,
    /// Unicode normalization
    pub normalization: bool,
    /// Security validation
    pub security_validation: bool,
    /// Mixed encoding support
    pub mixed_encoding: bool,
}

#[derive(Debug, Clone)]
pub struct AdvancedOperatorSupport {
    /// Bitwise string operators
    pub bitwise_string_ops: bool,
    /// Defined-or operator
    pub defined_or: bool,
    /// Smart match operator
    pub smart_match: bool,
    /// ISA operator
    pub isa_operator: bool,
}

// ============================================================================
// Modern Perl Syntax Comprehensive Tests - Compile-time Feature Gated
// ============================================================================
// These tests are aspirational and only compile when the feature is enabled.
// Run with: cargo test -p tree-sitter-perl-rs --features modern-perl-syntax
// ============================================================================

#[cfg(feature = "modern-perl-syntax")]
mod modern_perl_syntax {
    use super::*;
    use anyhow::{Context, Result};
    use tree_sitter::{Language, Parser, Tree};

    // Safety: tree_sitter_perl() returns a valid Language pointer from the compiled grammar
    unsafe extern "C" {
        fn tree_sitter_perl() -> Language;
    }

    /// Test helper to create parser
    fn create_parser() -> Result<Parser> {
        let mut parser = Parser::new();
        let language = unsafe { tree_sitter_perl() };
        parser.set_language(&language).context("Failed to set Perl language for parser")?;
        Ok(parser)
    }

    /// Test helper to parse code and validate AST
    fn parse_and_validate(
        parser: &mut Parser,
        code: &str,
        expected_node_types: &[&str],
    ) -> Result<Tree> {
        let tree = parser.parse(code, None).context("Failed to parse Perl code")?;

        let root_node = tree.root_node();
        assert!(!root_node.has_error(), "Parse tree should not have errors for: {}", code);

        // Validate expected node types are present
        for expected_type in expected_node_types {
            let found = find_node_type(&root_node, expected_type);
            assert!(
                found,
                "Expected node type '{}' not found in parse tree for: {}",
                expected_type, code
            );
        }

        Ok(tree)
    }

    /// Helper to recursively find node type in tree
    fn find_node_type(node: &tree_sitter::Node, target_type: &str) -> bool {
        if node.kind() == target_type {
            return true;
        }

        let child_count = node.child_count();
        for i in 0..child_count {
            if let Some(child) = node.child(i) {
                if find_node_type(&child, target_type) {
                    return true;
                }
            }
        }

        false
    }

    fn find_subroutine_node<'a>(node: &'a tree_sitter::Node<'a>) -> Option<tree_sitter::Node<'a>> {
        if node.kind() == "subroutine_declaration" {
            return Some(*node);
        }

        let child_count = node.child_count();
        for i in 0..child_count {
            if let Some(child) = node.child(i) {
                if let Some(found) = find_subroutine_node(&child) {
                    return Some(found);
                }
            }
        }

        None
    }

    fn has_signature(node: &tree_sitter::Node) -> bool {
        find_node_type(node, "signature") || find_node_type(node, "parameter_list")
    }

    fn count_parameters(node: &tree_sitter::Node) -> usize {
        count_nodes_of_type(node, "parameter")
    }

    fn count_parameters_with_defaults(node: &tree_sitter::Node) -> usize {
        count_nodes_of_type(node, "default_value")
    }

    fn count_slurpy_parameters(node: &tree_sitter::Node) -> usize {
        count_nodes_of_type(node, "slurpy_parameter")
    }

    fn count_named_parameters(node: &tree_sitter::Node) -> usize {
        count_nodes_of_type(node, "named_parameter")
    }

    fn count_nodes_of_type(node: &tree_sitter::Node, target_type: &str) -> usize {
        let mut count = 0;

        if node.kind() == target_type {
            count += 1;
        }

        let child_count = node.child_count();
        for i in 0..child_count {
            if let Some(child) = node.child(i) {
                count += count_nodes_of_type(&child, target_type);
            }
        }

        count
    }

    fn validate_parameter_structure(_node: &tree_sitter::Node, _code: &str) -> Result<()> {
        // Validate that parameters have proper structure (name, optional type, optional default)
        // This would be implemented based on the actual tree-sitter grammar
        Ok(())
    }

    fn validate_default_value_expressions(_node: &tree_sitter::Node, _code: &str) -> Result<()> {
        // Validate that default value expressions are properly parsed
        Ok(())
    }

    fn validate_slurpy_parameter_order(_node: &tree_sitter::Node, _code: &str) -> Result<()> {
        // Validate that slurpy parameters come after regular parameters
        Ok(())
    }

    fn validate_named_parameter_syntax(_node: &tree_sitter::Node, _code: &str) -> Result<()> {
        // Validate that named parameters use correct :$name syntax
        Ok(())
    }

    fn validate_postfix_deref_structure(
        _node: &tree_sitter::Node,
        _deref_type: &str,
        _code: &str,
    ) -> Result<()> {
        // Validate postfix dereferencing structure
        Ok(())
    }

    fn validate_chained_deref_structure(_node: &tree_sitter::Node, _code: &str) -> Result<()> {
        // Validate chained dereferencing operations
        Ok(())
    }

    fn validate_state_variable_structure(_node: &tree_sitter::Node, _code: &str) -> Result<()> {
        // Validate state variable declaration structure
        Ok(())
    }

    fn validate_state_variable_scoping(_node: &tree_sitter::Node, _code: &str) -> Result<()> {
        // Validate state variable scoping rules
        Ok(())
    }

    fn validate_unicode_identifier_structure(_node: &tree_sitter::Node, _code: &str) -> Result<()> {
        // Validate Unicode identifier parsing
        Ok(())
    }

    fn validate_unicode_security(_node: &tree_sitter::Node, _code: &str) -> Result<()> {
        // Validate Unicode security (homograph detection, normalization, etc.)
        Ok(())
    }

    fn detect_unicode_security_issues(_node: &tree_sitter::Node, _code: &str) -> Vec<String> {
        // Detect potential Unicode security issues
        // This would implement homograph detection, normalization checks, etc.
        vec![]
    }

    #[test]
    fn test_basic_subroutine_signatures() -> Result<()> {
        // AC7: Subroutine signature parsing
        // Tests feature spec: SPEC_144_IGNORED_TESTS_ARCHITECTURAL_BLUEPRINT.md#subroutine-signature-parser

        let mut parser =
            create_parser().context("Failed to create parser for subroutine signature tests")?;

        let signature_cases = vec![
            // Basic signatures
            (
                "sub foo ($x) { return $x + 1; }",
                &["subroutine_declaration", "signature", "parameter"][..],
            ),
            (
                "sub bar ($x, $y) { return $x + $y; }",
                &["subroutine_declaration", "signature", "parameter"],
            ),
            // Type annotations
            (
                "sub typed (Str $name, Int $age) { return $name; }",
                &["subroutine_declaration", "signature", "type_annotation"],
            ),
            (
                "sub complex_types (ArrayRef[Str] $items, HashRef $opts) { }",
                &["subroutine_declaration", "signature", "type_annotation"],
            ),
            // Method signatures
            (
                "sub method ($self, $param) { return $self->process($param); }",
                &["subroutine_declaration", "signature", "parameter"],
            ),
            (
                "sub class_method ($class, @args) { return $class->new(@args); }",
                &["subroutine_declaration", "signature", "parameter"],
            ),
        ];

        for (code, expected_nodes) in signature_cases {
            let tree = parse_and_validate(&mut parser, code, expected_nodes)
                .context(format!("Failed to parse basic signature: {}", code))?;

            let root = tree.root_node();

            // Validate signature structure
            if let Some(sub_node) = find_subroutine_node(&root) {
                assert!(has_signature(&sub_node), "Subroutine should have signature for: {}", code);

                let param_count = count_parameters(&sub_node);
                assert!(param_count > 0, "Signature should have parameters for: {}", code);

                // Validate parameter structure
                validate_parameter_structure(&sub_node, code)?;
            } else {
                panic!("No subroutine declaration found for: {}", code);
            }
        }

        Ok(())
    }

    #[test]
    fn test_subroutine_signature_default_values() -> Result<()> {
        // AC7: Default parameter values
        // Tests feature spec: SPEC_144_IGNORED_TESTS_ARCHITECTURAL_BLUEPRINT.md#subroutine-signature-parser

        let mut parser =
            create_parser().context("Failed to create parser for default value tests")?;

        let default_value_cases = vec![
            // Simple defaults
            (
                "sub foo ($x, $y = 10) { return $x + $y; }",
                &["parameter", "default_value", "number"][..],
            ),
            (
                "sub bar ($name = 'anonymous') { return $name; }",
                &["parameter", "default_value", "string"],
            ),
            // Complex defaults
            (
                "sub complex ($opts = {}) { return $opts; }",
                &["parameter", "default_value", "hash_constructor"],
            ),
            (
                "sub array_default ($items = []) { return $items; }",
                &["parameter", "default_value", "array_constructor"],
            ),
            // Expression defaults
            (
                "sub computed ($x, $y = $x * 2) { return $y; }",
                &["parameter", "default_value", "expression"],
            ),
            (
                "sub function_default ($cb = \\&default_callback) { return $cb->(); }",
                &["parameter", "default_value", "reference"],
            ),
            // Multiple defaults
            (
                "sub multi ($a, $b = 1, $c = 2) { return $a + $b + $c; }",
                &["parameter", "default_value"],
            ),
        ];

        for (code, expected_nodes) in default_value_cases {
            let tree = parse_and_validate(&mut parser, code, expected_nodes)
                .context(format!("Failed to parse default value signature: {}", code))?;

            let root = tree.root_node();

            // Validate default value parsing
            if let Some(sub_node) = find_subroutine_node(&root) {
                let default_params = count_parameters_with_defaults(&sub_node);
                assert!(
                    default_params > 0,
                    "Should have parameters with default values for: {}",
                    code
                );

                // Validate default value expressions
                validate_default_value_expressions(&sub_node, code)?;
            }
        }

        Ok(())
    }

    #[test]
    fn test_subroutine_signature_slurpy_parameters() -> Result<()> {
        // AC7: Slurpy parameters
        // Tests feature spec: SPEC_144_IGNORED_TESTS_ARCHITECTURAL_BLUEPRINT.md#subroutine-signature-parser

        let mut parser =
            create_parser().context("Failed to create parser for slurpy parameter tests")?;

        let slurpy_cases = vec![
            // Array slurpy
            (
                "sub slurp ($first, @rest) { return @rest; }",
                &["parameter", "slurpy_parameter", "array_sigil"][..],
            ),
            ("sub all_args (@args) { return scalar @args; }", &["slurpy_parameter", "array_sigil"]),
            // Hash slurpy
            (
                "sub with_opts ($x, %opts) { return %opts; }",
                &["parameter", "slurpy_parameter", "hash_sigil"],
            ),
            ("sub hash_args (%all) { return keys %all; }", &["slurpy_parameter", "hash_sigil"]),
            // Combined slurpy
            (
                "sub complex ($required, $optional = 1, @rest) { }",
                &["parameter", "default_value", "slurpy_parameter"],
            ),
            ("sub flexible ($first, @middle, %opts) { }", &["parameter", "slurpy_parameter"]),
            // Type constraints on slurpy
            (
                "sub typed_slurpy (Str $name, ArrayRef @items) { }",
                &["type_annotation", "slurpy_parameter"],
            ),
        ];

        for (code, expected_nodes) in slurpy_cases {
            let tree = parse_and_validate(&mut parser, code, expected_nodes)
                .context(format!("Failed to parse slurpy parameter signature: {}", code))?;

            let root = tree.root_node();

            // Validate slurpy parameter parsing
            if let Some(sub_node) = find_subroutine_node(&root) {
                let slurpy_count = count_slurpy_parameters(&sub_node);
                assert!(slurpy_count > 0, "Should have slurpy parameters for: {}", code);

                // Validate slurpy parameter placement (should be at end)
                validate_slurpy_parameter_order(&sub_node, code)?;
            }
        }

        Ok(())
    }

    #[test]
    fn test_subroutine_signature_named_parameters() -> Result<()> {
        // AC7: Named parameters
        // Tests feature spec: SPEC_144_IGNORED_TESTS_ARCHITECTURAL_BLUEPRINT.md#subroutine-signature-parser

        let mut parser =
            create_parser().context("Failed to create parser for named parameter tests")?;

        let named_parameter_cases = vec![
            // Basic named parameters
            (
                "sub named (:$name, :$age) { return \"$name is $age\"; }",
                &["named_parameter", "parameter_name"][..],
            ),
            (
                "sub with_defaults (:$name = 'John', :$age = 25) { }",
                &["named_parameter", "default_value"],
            ),
            // Mixed positional and named
            ("sub mixed ($required, :$optional) { }", &["parameter", "named_parameter"]),
            (
                "sub complex ($x, $y, :$debug = 0, :$verbose = 0) { }",
                &["parameter", "named_parameter", "default_value"],
            ),
            // Named with types
            (
                "sub typed_named (:Str $name, :Int $age = 18) { }",
                &["named_parameter", "type_annotation", "default_value"],
            ),
            // Named slurpy
            (
                "sub flexible (:$required, :@optional, :%extra) { }",
                &["named_parameter", "slurpy_parameter"],
            ),
        ];

        for (code, expected_nodes) in named_parameter_cases {
            let tree = parse_and_validate(&mut parser, code, expected_nodes)
                .context(format!("Failed to parse named parameter signature: {}", code))?;

            let root = tree.root_node();

            // Validate named parameter parsing
            if let Some(sub_node) = find_subroutine_node(&root) {
                let named_count = count_named_parameters(&sub_node);
                assert!(named_count > 0, "Should have named parameters for: {}", code);

                // Validate named parameter syntax
                validate_named_parameter_syntax(&sub_node, code)?;
            }
        }

        Ok(())
    }

    #[test]
    fn test_postfix_array_dereferencing() -> Result<()> {
        // AC8: Array postfix dereferencing
        // Tests feature spec: SPEC_144_IGNORED_TESTS_ARCHITECTURAL_BLUEPRINT.md#postfix-dereferencing-parser

        let mut parser =
            create_parser().context("Failed to create parser for postfix array deref tests")?;

        let array_deref_cases = vec![
            // Basic array postfix deref
            ("my @array = $ref->@*;", &["postfix_deref", "array_deref", "deref_operator"][..]),
            ("my $count = $ref->@*;", &["postfix_deref", "array_deref"]),
            // Array slice postfix deref
            ("my @slice = $ref->@[0..5];", &["postfix_deref", "array_slice", "range"]),
            ("my @items = $ref->@[0, 2, 4];", &["postfix_deref", "array_slice", "index_list"]),
            // Array operations with postfix deref
            ("push $ref->@*, 1, 2, 3;", &["function_call", "postfix_deref", "array_deref"]),
            ("my $first = ($ref->@*)[0];", &["postfix_deref", "array_access"]),
            // Complex expressions
            ("my @sorted = sort $ref->@*;", &["function_call", "postfix_deref"]),
            (
                "my @processed = map { $_ * 2 } $ref->@*;",
                &["function_call", "postfix_deref", "block"],
            ),
        ];

        for (code, expected_nodes) in array_deref_cases {
            let tree = parse_and_validate(&mut parser, code, expected_nodes)
                .context(format!("Failed to parse array postfix deref: {}", code))?;

            let root = tree.root_node();

            // Validate postfix dereferencing structure
            validate_postfix_deref_structure(&root, "array", code)?;
        }

        Ok(())
    }

    #[test]
    fn test_postfix_hash_dereferencing() -> Result<()> {
        // AC8: Hash postfix dereferencing
        // Tests feature spec: SPEC_144_IGNORED_TESTS_ARCHITECTURAL_BLUEPRINT.md#postfix-dereferencing-parser

        let mut parser =
            create_parser().context("Failed to create parser for postfix hash deref tests")?;

        let hash_deref_cases = vec![
            // Basic hash postfix deref
            ("my %hash = $ref->%*;", &["postfix_deref", "hash_deref", "deref_operator"][..]),
            ("my @keys = keys $ref->%*;", &["function_call", "postfix_deref", "hash_deref"]),
            // Hash slice postfix deref
            ("my @values = $ref->@{qw(a b c)};", &["postfix_deref", "hash_slice", "word_list"]),
            ("my %subset = $ref->%{@keys};", &["postfix_deref", "hash_slice"]),
            // Hash operations with postfix deref
            ("my @all_keys = keys $ref->%*;", &["function_call", "postfix_deref"]),
            ("my @all_values = values $ref->%*;", &["function_call", "postfix_deref"]),
            // Complex hash expressions
            ("my %merged = (%existing, $ref->%*);", &["hash_constructor", "postfix_deref"]),
            ("delete $ref->%{@unwanted_keys};", &["function_call", "postfix_deref", "hash_slice"]),
        ];

        for (code, expected_nodes) in hash_deref_cases {
            let tree = parse_and_validate(&mut parser, code, expected_nodes)
                .context(format!("Failed to parse hash postfix deref: {}", code))?;

            let root = tree.root_node();

            // Validate postfix dereferencing structure
            validate_postfix_deref_structure(&root, "hash", code)?;
        }

        Ok(())
    }

    #[test]
    fn test_chained_postfix_dereferencing() -> Result<()> {
        // AC8: Chained postfix dereferencing
        // Tests feature spec: SPEC_144_IGNORED_TESTS_ARCHITECTURAL_BLUEPRINT.md#postfix-dereferencing-parser

        let mut parser =
            create_parser().context("Failed to create parser for chained postfix deref tests")?;

        let chained_deref_cases = vec![
            // Simple chaining
            ("my $value = $ref->$*->method();", &["postfix_deref", "method_call", "chained_call"][..]),
            ("my @items = $ref->@*->@*;", &["postfix_deref", "array_deref", "chained_deref"]),
            // Complex chaining
            ("my %result = $ref->@*->[0]->%*;", &["postfix_deref", "array_access", "hash_deref"]),
            (
                "my $deep = $ref->%*->{key}->@*->[0];",
                &["postfix_deref", "hash_access", "array_access"],
            ),
            // Chaining with method calls
            (
                "my @processed = $ref->@*->map(sub { $_ * 2 })->@*;",
                &["postfix_deref", "method_call", "chained_call"],
            ),
            (
                "my $count = $ref->%*->keys->@*->scalar;",
                &["postfix_deref", "method_call", "chained_call"],
            ),
            // Mixed dereferencing types
            (
                "my $final = $ref->$*->get_array->@*->[0];",
                &["postfix_deref", "method_call", "array_access"],
            ),
        ];

        for (code, expected_nodes) in chained_deref_cases {
            let tree = parse_and_validate(&mut parser, code, expected_nodes)
                .context(format!("Failed to parse chained postfix deref: {}", code))?;

            let root = tree.root_node();

            // Validate chained dereferencing structure
            validate_chained_deref_structure(&root, code)?;
        }

        Ok(())
    }

    #[test]
    fn test_basic_state_variables() -> Result<()> {
        // AC9: State variable parsing
        // Tests feature spec: SPEC_144_IGNORED_TESTS_ARCHITECTURAL_BLUEPRINT.md#state-variable-parser

        let mut parser =
            create_parser().context("Failed to create parser for state variable tests")?;

        let state_variable_cases = vec![
            // Basic state declarations
            ("state $counter = 0;", &["state_declaration", "scalar_variable", "assignment"][..]),
            (
                "state @items = (1, 2, 3);",
                &["state_declaration", "array_variable", "array_constructor"],
            ),
            ("state %cache = ();", &["state_declaration", "hash_variable", "hash_constructor"]),
            // State in subroutines
            (
                "sub counter { state $x = 0; return ++$x; }",
                &["subroutine_declaration", "state_declaration"],
            ),
            (
                "sub memoize { state %memo; return \\%memo; }",
                &["subroutine_declaration", "state_declaration"],
            ),
            // State with complex initialization
            ("state $config = load_config();", &["state_declaration", "function_call"]),
            ("state @data = read_data_file();", &["state_declaration", "function_call"]),
            // Multiple state declarations
            ("state ($x, $y) = (1, 2);", &["state_declaration", "variable_list", "assignment"]),
            ("state ($counter, @history, %cache);", &["state_declaration", "variable_list"]),
        ];

        for (code, expected_nodes) in state_variable_cases {
            let tree = parse_and_validate(&mut parser, code, expected_nodes)
                .context(format!("Failed to parse state variable: {}", code))?;

            let root = tree.root_node();

            // Validate state variable structure
            validate_state_variable_structure(&root, code)?;
        }

        Ok(())
    }

    #[test]
    fn test_state_variable_scoping() -> Result<()> {
        // AC9: State variable scoping
        // Tests feature spec: SPEC_144_IGNORED_TESTS_ARCHITECTURAL_BLUEPRINT.md#state-variable-parser

        let mut parser =
            create_parser().context("Failed to create parser for state variable scoping tests")?;

        let scoping_cases = vec![
            // State in different scopes
            (
                r#"
            sub outer {
                state $x = 0;
                sub inner {
                    state $y = 0;
                    return $y++;
                }
                return $x++;
            }
            "#,
                &["subroutine_declaration", "state_declaration", "nested_subroutine"][..],
            ),
            // State in loops
            (
                r#"
            for my $i (1..10) {
                state $accumulator = 0;
                $accumulator += $i;
            }
            "#,
                &["for_loop", "state_declaration"],
            ),
            // State in conditionals
            (
                r#"
            if ($condition) {
                state $cache = {};
                return $cache->{$key} //= expensive_computation($key);
            }
            "#,
                &["if_statement", "state_declaration"],
            ),
            // State with closures
            (
                r#"
            sub make_counter {
                return sub {
                    state $count = 0;
                    return ++$count;
                };
            }
            "#,
                &["subroutine_declaration", "anonymous_subroutine", "state_declaration"],
            ),
        ];

        for (code, expected_nodes) in scoping_cases {
            let tree = parse_and_validate(&mut parser, code, expected_nodes)
                .context(format!("Failed to parse state variable scoping: {}", code))?;

            let root = tree.root_node();

            // Validate state variable scoping
            validate_state_variable_scoping(&root, code)?;
        }

        Ok(())
    }

    #[test]
    fn test_unicode_identifiers() -> Result<()> {
        // AC11: Unicode identifier parsing
        // Tests feature spec: SPEC_144_IGNORED_TESTS_ARCHITECTURAL_BLUEPRINT.md#unicode-identifier-parser

        let mut parser =
            create_parser().context("Failed to create parser for Unicode identifier tests")?;

        let unicode_cases = vec![
            // Basic Unicode identifiers
            ("my $测试 = 42;", &["scalar_variable", "unicode_identifier"][..]),
            ("sub функция { return 1; }", &["subroutine_declaration", "unicode_identifier"]),
            ("package ΠάκαγΚλασσ;", &["package_declaration", "unicode_identifier"]),
            // Emoji identifiers
            ("my $🚀 = 'rocket';", &["scalar_variable", "emoji_identifier"]),
            ("sub 🔍search { return @_; }", &["subroutine_declaration", "emoji_identifier"]),
            ("my %📊stats = ();", &["hash_variable", "emoji_identifier"]),
            // Mixed Unicode and ASCII
            ("my $user名前 = 'name';", &["scalar_variable", "mixed_identifier"]),
            ("sub process_データ { }", &["subroutine_declaration", "mixed_identifier"]),
            // Unicode in different contexts
            ("$object->méthode();", &["method_call", "unicode_identifier"]),
            (
                "my $résultat = $obj->データ;",
                &["scalar_variable", "unicode_identifier", "method_call"],
            ),
            // Unicode package names
            ("use モジュール::テスト;", &["use_statement", "unicode_package_name"]),
            ("require Ελληνικά::Κλάση;", &["require_statement", "unicode_package_name"]),
        ];

        for (code, expected_nodes) in unicode_cases {
            let tree = parse_and_validate(&mut parser, code, expected_nodes)
                .context(format!("Failed to parse Unicode identifier: {}", code))?;

            let root = tree.root_node();

            // Validate Unicode identifier structure
            validate_unicode_identifier_structure(&root, code)?;
        }

        Ok(())
    }

    #[test]
    fn test_unicode_security_validation() -> Result<()> {
        // AC11: Unicode security validation
        // Tests feature spec: SPEC_144_IGNORED_TESTS_ARCHITECTURAL_BLUEPRINT.md#unicode-identifier-parser

        let mut parser =
            create_parser().context("Failed to create parser for Unicode security tests")?;

        // Test cases for potential Unicode security issues
        let security_test_cases = vec![
            // Homograph attacks (visually similar characters)
            ("my $variable = 1; my $variabIe = 2;", false), // Latin I vs Greek Iota
            ("sub test { } sub tеst { }", false),           // Latin e vs Cyrillic e
            // Valid Unicode that should be accepted
            ("my $正常な変数 = 'normal';", true),
            ("sub 安全な関数 { return 1; }", true),
            // Potentially confusing but valid
            ("my $αβγ = 'greek';", true),
            ("my $αβγδ = 'extended_greek';", true),
            // Zero-width characters (potential issues)
            ("my $var\u{200B}name = 'hidden';", false), // Zero-width space
            ("my $test\u{FEFF}var = 'bom';", false),    // Byte order mark
        ];

        for (code, should_be_valid) in security_test_cases {
            let parse_result = parser.parse(code, None);

            if let Some(tree) = parse_result {
                let root = tree.root_node();
                let has_errors = root.has_error();

                if should_be_valid {
                    assert!(
                        !has_errors,
                        "Valid Unicode code should parse without errors: {}",
                        code
                    );

                    // Additional security validation would be implemented here
                    // This would check for homograph attacks, normalization issues, etc.
                    validate_unicode_security(&root, code)?;
                } else {
                    // For security issues, the parser might accept the syntax
                    // but additional validation should flag potential issues
                    if !has_errors {
                        println!("Warning: Potentially unsafe Unicode accepted: {}", code);
                        // Security validation should detect issues
                        let security_issues = detect_unicode_security_issues(&root, code);
                        assert!(
                            !security_issues.is_empty(),
                            "Security validation should detect issues in: {}",
                            code
                        );
                    }
                }
            } else if should_be_valid {
                panic!("Valid Unicode code failed to parse: {}", code);
            }
        }

        Ok(())
    }
}
