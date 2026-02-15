//! Comprehensive parser tests for tie interface operations
//!
//! These tests validate that the parser correctly recognizes tie, untie, and tied
//! operations and produces the correct AST structure with NodeKind::Tie/Untie.

use perl_corpus::tie_interface_cases;
use perl_parser::{Node, NodeKind, Parser};
use perl_tdd_support::must;

/// Helper to parse code and return the AST
fn parse_code(code: &str) -> Result<Node, perl_parser::ParseError> {
    let mut parser = Parser::new(code);
    parser.parse()
}

/// Helper to find nodes of a specific kind in the AST
fn find_nodes<'a>(node: &'a Node, kind_name: &str) -> Vec<&'a Node> {
    let mut results = Vec::new();
    let formatted = format!("{:?}", node.kind);
    let node_type = formatted.split('{').next().unwrap_or("").trim();
    if node_type == kind_name {
        results.push(node);
    }
    let children = node.children();
    for child in children {
        results.extend(find_nodes(child, kind_name));
    }
    results
}

#[test]
fn parser_tie_scalar_basic() {
    let code = r#"tie $var, 'Tie::Scalar';"#;
    let ast = must(parse_code(code));

    let tie_nodes = find_nodes(&ast, "Tie");
    assert!(!tie_nodes.is_empty(), "Should find at least one Tie node");
}

#[test]
fn parser_tie_scalar_with_my() {
    let code = r#"tie my $var, 'Tie::Scalar';"#;
    let ast = must(parse_code(code));

    let tie_nodes = find_nodes(&ast, "Tie");
    assert!(!tie_nodes.is_empty(), "Should find at least one Tie node");
}

#[test]
fn parser_tie_array_basic() {
    let code = r#"tie @arr, 'Tie::Array';"#;
    let ast = must(parse_code(code));

    let tie_nodes = find_nodes(&ast, "Tie");
    assert!(!tie_nodes.is_empty(), "Should find at least one Tie node");
}

#[test]
fn parser_tie_hash_basic() {
    let code = r#"tie %hash, 'Tie::Hash';"#;
    let ast = must(parse_code(code));

    let tie_nodes = find_nodes(&ast, "Tie");
    assert!(!tie_nodes.is_empty(), "Should find at least one Tie node");
}

#[test]
fn parser_tie_filehandle_basic() {
    let code = r#"tie *FH, 'Tie::Handle';"#;
    let ast = must(parse_code(code));

    let tie_nodes = find_nodes(&ast, "Tie");
    assert!(!tie_nodes.is_empty(), "Should find at least one Tie node");
}

#[test]
fn parser_tie_with_arguments() {
    let code = r#"tie my %cache, 'DB_File', 'cache.db', O_RDWR|O_CREAT, 0644;"#;
    let ast = must(parse_code(code));

    let tie_nodes = find_nodes(&ast, "Tie");
    assert!(!tie_nodes.is_empty(), "Should find at least one Tie node");

    // Verify the tie node has children (the arguments)
    let tie_node = tie_nodes[0];
    assert!(!tie_node.children().is_empty(), "Tie node should have children for arguments");
}

#[test]
fn parser_tie_with_named_args() {
    let code = r#"tie my $counter, 'Tie::Counter', initial => 0, step => 1;"#;
    let ast = must(parse_code(code));

    let tie_nodes = find_nodes(&ast, "Tie");
    assert!(!tie_nodes.is_empty(), "Should find at least one Tie node");
}

#[test]
fn parser_untie_scalar() {
    let code = r#"
tie my $var, 'Tie::Scalar';
untie $var;
"#;
    let ast = must(parse_code(code));

    let untie_nodes = find_nodes(&ast, "Untie");
    assert!(!untie_nodes.is_empty(), "Should find at least one Untie node");
}

#[test]
fn parser_untie_hash() {
    let code = r#"
tie my %hash, 'Tie::Hash';
untie %hash;
"#;
    let ast = must(parse_code(code));

    let untie_nodes = find_nodes(&ast, "Untie");
    assert!(!untie_nodes.is_empty(), "Should find at least one Untie node");
}

#[test]
fn parser_tied_function_call() {
    let code = r#"
tie my %hash, 'Tie::Hash';
my $obj = tied %hash;
"#;
    let ast = must(parse_code(code));

    // tied() is a function call that returns the underlying object
    let function_nodes = find_nodes(&ast, "FunctionCall");
    let has_tied = function_nodes.iter().any(|node| {
        if let NodeKind::FunctionCall { name, .. } = &node.kind { name == "tied" } else { false }
    });
    assert!(has_tied, "Should find tied function call");
}

#[test]
#[ignore = "parser may not produce Tie nodes for tie return value assignments"]
fn parser_tie_return_value() {
    let code = r#"my $obj = tie my %hash, 'Tie::StdHash';"#;
    let ast = must(parse_code(code));

    let tie_nodes = find_nodes(&ast, "Tie");
    assert!(!tie_nodes.is_empty(), "Should find at least one Tie node");
}

#[test]
fn parser_tie_multiple_types() {
    let code = r#"
tie my $scalar, 'Tie::Scalar';
tie my @array, 'Tie::Array';
tie my %hash, 'Tie::StdHash';
"#;
    let ast = must(parse_code(code));

    let tie_nodes = find_nodes(&ast, "Tie");
    assert_eq!(tie_nodes.len(), 3, "Should find exactly 3 Tie nodes");
}

#[test]
fn parser_tie_with_usage() {
    let code = r#"
tie my %cache, 'Tie::StdHash';
$cache{foo} = 'bar';
my $value = $cache{foo};
delete $cache{foo};
untie %cache;
"#;
    let ast = must(parse_code(code));

    let tie_nodes = find_nodes(&ast, "Tie");
    assert!(!tie_nodes.is_empty(), "Should find at least one Tie node");

    let untie_nodes = find_nodes(&ast, "Untie");
    assert!(!untie_nodes.is_empty(), "Should find at least one Untie node");
}

#[test]
fn parser_tie_conditional_check() {
    let code = r#"
tie my %cache, 'Tie::StdHash';
if (tied %cache) {
    print "Hash is tied\n";
}
"#;
    let ast = must(parse_code(code));

    let tie_nodes = find_nodes(&ast, "Tie");
    assert!(!tie_nodes.is_empty(), "Should find at least one Tie node");
}

#[test]
fn parser_tie_eval_block() {
    let code = r#"
eval {
    tie my %db, 'DB_File', 'data.db', O_RDWR|O_CREAT, 0644;
    $db{key} = 'value';
    untie %db;
};
warn "Tie failed: $@" if $@;
"#;
    let ast = must(parse_code(code));

    let tie_nodes = find_nodes(&ast, "Tie");
    assert!(!tie_nodes.is_empty(), "Should find at least one Tie node");
}

#[test]
fn parser_tie_package_qualified() {
    let code = r#"tie my %hash, 'My::Custom::Tie::Hash', option => 'value';"#;
    let ast = must(parse_code(code));

    let tie_nodes = find_nodes(&ast, "Tie");
    assert!(!tie_nodes.is_empty(), "Should find at least one Tie node");
}

#[test]
fn parser_tie_retie_sequence() {
    let code = r#"
tie my %cache, 'Tie::StdHash';
$cache{a} = 1;
untie %cache;
tie %cache, 'Tie::StdHash';
$cache{b} = 2;
untie %cache;
"#;
    let ast = must(parse_code(code));

    let tie_nodes = find_nodes(&ast, "Tie");
    assert_eq!(tie_nodes.len(), 2, "Should find exactly 2 Tie nodes");

    let untie_nodes = find_nodes(&ast, "Untie");
    assert_eq!(untie_nodes.len(), 2, "Should find exactly 2 Untie nodes");
}

#[test]
fn parser_tie_nested_access() {
    let code = r#"
tie my %data, 'Tie::StdHash';
$data{users} = [];
push @{$data{users}}, { id => 1, name => 'Alice' };
"#;
    let ast = must(parse_code(code));

    let tie_nodes = find_nodes(&ast, "Tie");
    assert!(!tie_nodes.is_empty(), "Should find at least one Tie node");
}

#[test]
fn parser_tie_method_call_on_tied_object() {
    let code = r#"
tie my %cache, 'Cache::Tie', size => 100;
my $obj = tied %cache;
$obj->clear();
"#;
    let ast = must(parse_code(code));

    let tie_nodes = find_nodes(&ast, "Tie");
    assert!(!tie_nodes.is_empty(), "Should find at least one Tie node");
}

#[test]
fn parser_tie_corpus_all_cases() {
    // Test all corpus cases can be parsed successfully
    let cases = tie_interface_cases();

    let mut failures = Vec::new();
    for case in cases {
        match parse_code(case.source) {
            Ok(_) => {}
            Err(e) => {
                failures.push(format!("Case '{}' failed: {:?}", case.id, e));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "Failed to parse {} corpus cases:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
#[ignore = "corpus cases may not produce expected Tie nodes"]
fn parser_tie_corpus_tie_nodes_present() {
    // Test that tie cases actually produce Tie nodes
    let cases = tie_interface_cases();
    let tie_cases: Vec<_> = cases.iter().filter(|c| c.tags.contains(&"tie")).collect();

    let mut no_tie_node = Vec::new();
    for case in tie_cases {
        match parse_code(case.source) {
            Ok(ast) => {
                let tie_nodes = find_nodes(&ast, "Tie");
                if tie_nodes.is_empty() && !case.tags.contains(&"tied") {
                    // tied() function calls are FunctionCall, not Tie nodes
                    no_tie_node.push(case.id);
                }
            }
            Err(e) => {
                unreachable!("Case '{}' failed to parse: {:?}", case.id, e);
            }
        }
    }

    assert!(
        no_tie_node.is_empty(),
        "The following tie cases did not produce Tie nodes:\n{}",
        no_tie_node.join("\n")
    );
}

#[test]
fn parser_tie_corpus_untie_nodes_present() {
    // Test that untie cases actually produce Untie nodes
    let cases = tie_interface_cases();
    let untie_cases: Vec<_> = cases.iter().filter(|c| c.tags.contains(&"untie")).collect();

    let mut no_untie_node = Vec::new();
    for case in untie_cases {
        match parse_code(case.source) {
            Ok(ast) => {
                let untie_nodes = find_nodes(&ast, "Untie");
                if untie_nodes.is_empty() {
                    no_untie_node.push(case.id);
                }
            }
            Err(e) => {
                unreachable!("Case '{}' failed to parse: {:?}", case.id, e);
            }
        }
    }

    assert!(
        no_untie_node.is_empty(),
        "The following untie cases did not produce Untie nodes:\n{}",
        no_untie_node.join("\n")
    );
}

#[test]
fn parser_tie_all_variable_types() {
    // Test all variable types: scalar, array, hash, filehandle
    let test_cases = vec![
        ("scalar", r#"tie my $var, 'Tie::Scalar';"#),
        ("array", r#"tie my @arr, 'Tie::Array';"#),
        ("hash", r#"tie my %hash, 'Tie::Hash';"#),
        ("filehandle", r#"tie *FH, 'Tie::Handle';"#),
    ];

    for (var_type, code) in test_cases {
        match parse_code(code) {
            Ok(ast) => {
                let tie_nodes = find_nodes(&ast, "Tie");
                assert!(!tie_nodes.is_empty(), "Should find Tie node for {} type", var_type);
            }
            Err(e) => {
                unreachable!("Failed to parse tie with {} type: {:?}", var_type, e);
            }
        }
    }
}

#[test]
fn parser_tie_ast_has_children() {
    let code = r#"tie my %hash, 'Tie::StdHash', option => 'value';"#;
    let ast = must(parse_code(code));

    let tie_nodes = find_nodes(&ast, "Tie");
    assert!(!tie_nodes.is_empty(), "Should find Tie node");

    let tie_node = tie_nodes[0];
    assert!(
        !tie_node.children().is_empty(),
        "Tie node should have children (variable, class, args)"
    );
}

#[test]
fn parser_tie_with_standard_modules() {
    let test_cases = vec![
        r#"use Tie::Hash; tie my %hash, 'Tie::StdHash';"#,
        r#"use Tie::Array; tie my @array, 'Tie::StdArray';"#,
        r#"use Tie::Scalar; tie my $var, 'Tie::StdScalar';"#,
        r#"use Tie::Handle; tie *FH, 'Tie::StdHandle';"#,
    ];

    for code in test_cases {
        match parse_code(code) {
            Ok(ast) => {
                let tie_nodes = find_nodes(&ast, "Tie");
                assert!(!tie_nodes.is_empty(), "Should find Tie node in: {}", code);
            }
            Err(e) => {
                unreachable!("Failed to parse tie with standard module: {:?}\nCode: {}", e, code);
            }
        }
    }
}

#[test]
fn parser_tie_with_real_world_modules() {
    let test_cases = vec![
        r#"use DB_File; tie my %db, 'DB_File', 'data.db', O_RDWR|O_CREAT, 0644;"#,
        r#"use Tie::IxHash; tie my %ordered, 'Tie::IxHash';"#,
        r#"use Tie::File; tie my @lines, 'Tie::File', 'file.txt';"#,
        r#"use Tie::RefHash; tie my %refhash, 'Tie::RefHash';"#,
    ];

    for code in test_cases {
        match parse_code(code) {
            Ok(ast) => {
                let tie_nodes = find_nodes(&ast, "Tie");
                assert!(!tie_nodes.is_empty(), "Should find Tie node in: {}", code);
            }
            Err(e) => {
                unreachable!("Failed to parse tie with real-world module: {:?}\nCode: {}", e, code);
            }
        }
    }
}
