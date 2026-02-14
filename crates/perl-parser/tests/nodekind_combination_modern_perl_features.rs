//! Comprehensive tests for modern Perl feature combinations
//! 
//! These tests validate complex interactions between modern Perl features
//! including try/catch, given/when, class/method, signatures, and more.

use perl_parser::{Parser, ast::{Node, NodeKind}};

/// Test try/catch with signatures, class methods, and variable declarations
#[test]
fn test_try_catch_with_signatures_and_classes() {
    let code = r#"
use feature 'try';
use feature 'class';

class MyClass {
    field $name :param;
    field $value :param;

    method process_data($input, $options = {}) {
        try {
            my $result = $self->validate_input($input);
            return $self->transform($result, $options);
        } catch ($e) {
            warn "Processing failed: $e";
            return undef;
        }
    }

    method validate_input($data) {
        die "Invalid input" unless defined $data && length $data > 0;
        return $data;
    }

    method transform($data, $options) {
        my $transformed = $data;
        $transformed =~ s/^\s+|\s+$//g if $options->{trim};
        $transformed = uc $transformed if $options->{uppercase};
        return $transformed;
    }
}

my $obj = MyClass->new(name => "test", value => 42);
my $result = $obj->process_data("  hello world  ", {trim => 1, uppercase => 1});
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse().expect("Should parse successfully");
    
    // Verify we have a Program with multiple statements
    assert!(matches!(ast.kind, NodeKind::Program { .. }));
    
    if let NodeKind::Program { statements } = &ast.kind {
        // Should have use statements, class declaration, and variable declarations
        assert!(statements.len() >= 4);
        
        // Find the class declaration
        let class_nodes: Vec<_> = statements.iter()
            .filter(|s| matches!(s.kind, NodeKind::Class { .. }))
            .collect();
        assert_eq!(class_nodes.len(), 1, "Should have exactly one class");
        
        // Verify class has methods
        if let NodeKind::Class { body, .. } = &class_nodes[0].kind {
            if let NodeKind::Block { statements: class_statements } = &body.kind {
                let method_nodes: Vec<_> = class_statements.iter()
                    .filter(|s| matches!(s.kind, NodeKind::Method { .. }))
                    .collect();
                assert_eq!(method_nodes.len(), 3, "Should have three methods");
                
                // Verify methods have signatures
                for method in &method_nodes {
                    if let NodeKind::Method { signature, .. } = &method.kind {
                        assert!(signature.is_some(), "Each method should have a signature");
                    }
                }
            }
        }
        
        // Find try-catch blocks
        let try_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Try { .. }));
        assert!(!try_nodes.is_empty(), "Should have try blocks");
        
        // Verify variable declarations with signatures
        let var_decls = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::VariableDeclaration { .. }));
        assert!(!var_decls.is_empty(), "Should have variable declarations");
        
        // Check for method calls
        let method_calls = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::MethodCall { .. }));
        assert!(!method_calls.is_empty(), "Should have method calls");
    }
}

/// Test given/when with complex data structures and method calls
#[test]
fn test_given_when_with_complex_structures() {
    let code = r#"
use feature 'switch';

my $data = {
    type => 'complex',
    value => 42,
    nested => {
        items => [1, 2, 3],
        flags => { active => 1, verified => 0 }
    }
};

given ($data) {
    when (ref) {
        die "Got reference";
    }
    when (undef) {
        warn "Data is undefined";
    }
    when (['hash']) {
        my $type = $data->{type} // 'unknown';
        my $value = $data->{value} // 0;
        
        given ($type) {
            when ('complex') {
                process_complex($data);
            }
            when ('simple') {
                process_simple($data);
            }
            default {
                warn "Unknown data type: $type";
            }
        }
    }
    when (['array']) {
        for my $item (@$data) {
            process_item($item);
        }
    }
    default {
        warn "Unsupported data structure";
    }
}

sub process_complex {
    my ($hash) = @_;
    my $nested = $hash->{nested};
    my $items = $nested->{items} || [];
    my $flags = $nested->{flags} || {};
    
    if ($flags->{active}) {
        print "Processing active items: @$items\n";
    }
}

sub process_simple {
    my ($hash) = @_;
    print "Processing simple data: $hash->{value}\n";
}

sub process_item {
    my ($item) = @_;
    print "Processing item: $item\n";
}
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse().expect("Should parse successfully");
    
    // Verify given/when structure
    let given_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Given { .. }));
    assert!(!given_nodes.is_empty(), "Should have given statements");
    
    let when_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::When { .. }));
    assert!(!when_nodes.is_empty(), "Should have when clauses");
    
    let default_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Default { .. }));
    assert!(!default_nodes.is_empty(), "Should have default clauses");
    
    // Verify hash and array literals
    let hash_literals = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::HashLiteral { .. }));
    assert!(!hash_literals.is_empty(), "Should have hash literals");
    
    let array_literals = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::ArrayLiteral { .. }));
    assert!(!array_literals.is_empty(), "Should have array literals");
    
    // Verify dereferencing operations
    let binary_ops = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Binary { .. }));
    assert!(!binary_ops.is_empty(), "Should have binary operations for dereferencing");
}

/// Test class/method with inheritance, roles, and attributes
#[test]
fn test_class_method_inheritance_roles_attributes() {
    let code = r#"
use feature 'class';

role Drawable {
    method draw() {
        print "Drawing\n";
    }
}

role Serializable {
    method serialize() {
        return { class => ref($self), data => $self->{_data} };
    }
}

class Shape {
    field $x :param = 0;
    field $y :param = 0;
    
    method move($dx, $dy) {
        $x += $dx;
        $y += $dy;
    }
    
    method position() {
        return { x => $x, y => $y };
    }
}

class Rectangle :isa(Shape) :does(Drawable, Serializable) {
    field $width :param;
    field $height :param;
    field $color = 'black';
    
    method area() {
        return $width * $height;
    }
    
    method perimeter() {
        return 2 * ($width + $height);
    }
    
    method draw() override {
        print "Drawing rectangle at ($x, $y) with size ${width}x${height}\n";
    }
    
    method serialize() override {
        my $base_data = $self->Shape::serialize();
        return {
            %$base_data,
            width => $width,
            height => $height,
            color => $color
        };
    }
}

my $rect = Rectangle->new(x => 10, y => 20, width => 100, height => 50);
$rect->move(5, -5);
$rect->draw();
my $data = $rect->serialize();
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse().expect("Should parse successfully");
    
    // Verify role declarations
    let role_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Class { .. }));
    let roles: Vec<_> = role_nodes.iter()
        .filter(|n| {
            if let NodeKind::Class { name, .. } = &n.kind {
                name.contains("Drawable") || name.contains("Serializable")
            } else {
                false
            }
        })
        .collect();
    assert!(!roles.is_empty(), "Should have role declarations");
    
    // Verify class with inheritance
    let class_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Class { .. }));
    let shape_class: Vec<_> = class_nodes.iter()
        .filter(|n| {
            if let NodeKind::Class { name, .. } = &n.kind {
                name == "Shape"
            } else {
                false
            }
        })
        .collect();
    assert_eq!(shape_class.len(), 1, "Should have Shape class");
    
    // Verify class with roles
    let rect_class: Vec<_> = class_nodes.iter()
        .filter(|n| {
            if let NodeKind::Class { name, .. } = &n.kind {
                name == "Rectangle"
            } else {
                false
            }
        })
        .collect();
    assert_eq!(rect_class.len(), 1, "Should have Rectangle class");
    
    // Verify field declarations with attributes
    let field_decls = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::VariableDeclaration { .. }));
    assert!(!field_decls.is_empty(), "Should have field declarations");
    
    // Verify method with override
    let method_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Method { .. }));
    assert!(!method_nodes.is_empty(), "Should have methods");
}

/// Test signatures with complex parameter types and default values
#[test]
fn test_signatures_complex_parameters_defaults() {
    let code = r#"
use feature 'signatures';

sub process_data(
    $required_param,
    $optional_param = 'default_value',
    $array_ref = [],
    $hash_ref = {},
    $code_ref = sub { return 'default' },
    $type_glob = *STDOUT,
) {
    my $result = {
        required => $required_param,
        optional => $optional_param,
        array_size => scalar @$array_ref,
        hash_keys => [sort keys %$hash_ref],
        code_result => $code_ref->(),
        glob_type => ref($type_glob),
    };
    
    return $result;
}

sub complex_signature(
    $scalar,
    @array,
    %hash,
    &$code,
    *glob,
    $optional = undef,
    $typed :readonly,
    $slurpy@
) {
    # Complex parameter processing
    my $processed = {
        scalar => $scalar,
        array_count => scalar @array,
        hash_count => scalar %hash,
        code_ref => $code,
        glob_ref => $glob,
        optional => $optional // 'not_provided',
        typed_readonly => $typed,
        slurpy_array => \@slurpy,
    };
    
    return $processed;
}

# Test calls with various argument patterns
my $result1 = process_data('test');
my $result2 = process_data('test', 'custom', [1, 2, 3], {a => 1, b => 2});
my $result3 = process_data('test', 'custom', [], {}, sub { return 'custom code' });

my $complex1 = complex_signature('scalar');
my $complex2 = complex_signature('scalar', 1, 2, 3, 4, 5, 'optional_val');
my $complex3 = complex_signature('scalar', (1, 2, 3), (a => 1, b => 2), sub { }, *STDERR);
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse().expect("Should parse successfully");
    
    // Verify subroutine signatures
    let sub_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Subroutine { .. }));
    assert!(!sub_nodes.is_empty(), "Should have subroutines");
    
    // Check for signatures in subroutines
    for sub in &sub_nodes {
        if let NodeKind::Subroutine { signature, .. } = &sub.kind {
            assert!(signature.is_some(), "Each subroutine should have a signature");
        }
    }
    
    // Verify signature parameters
    let sig_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Signature { .. }));
    assert!(!sig_nodes.is_empty(), "Should have signature nodes");
    
    // Verify different parameter types
    let mandatory_params = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::MandatoryParameter { .. }));
    let optional_params = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::OptionalParameter { .. }));
    let slurpy_params = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::SlurpyParameter { .. }));
    
    assert!(!mandatory_params.is_empty(), "Should have mandatory parameters");
    assert!(!optional_params.is_empty(), "Should have optional parameters");
    assert!(!slurpy_params.is_empty(), "Should have slurpy parameters");
    
    // Verify function calls with various argument patterns
    let func_calls = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::FunctionCall { .. }));
    assert!(!func_calls.is_empty(), "Should have function calls");
    
    // Verify array and hash literals in calls
    let array_literals = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::ArrayLiteral { .. }));
    let hash_literals = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::HashLiteral { .. }));
    assert!(!array_literals.is_empty(), "Should have array literals");
    assert!(!hash_literals.is_empty(), "Should have hash literals");
}

/// Helper function to find nodes of specific kinds
fn find_nodes_of_kind<F>(node: &Node, predicate: F) -> Vec<&Node>
where
    F: Fn(&NodeKind) -> bool,
{
    let mut results = Vec::new();
    find_nodes_recursive(node, &predicate, &mut results);
    results
}

/// Recursive helper to find nodes matching predicate
fn find_nodes_recursive<'a, F>(node: &'a Node, predicate: &F, results: &mut Vec<&'a Node>)
where
    F: Fn(&NodeKind) -> bool,
{
    if predicate(&node.kind) {
        results.push(node);
    }
    
    // Recurse into child nodes based on node type
    match &node.kind {
        NodeKind::Program { statements } => {
            for stmt in statements {
                find_nodes_recursive(stmt, predicate, results);
            }
        }
        NodeKind::Block { statements } => {
            for stmt in statements {
                find_nodes_recursive(stmt, predicate, results);
            }
        }
        NodeKind::ExpressionStatement { expression } => {
            find_nodes_recursive(expression, predicate, results);
        }
        NodeKind::VariableDeclaration { initializer, .. } => {
            if let Some(init) = initializer {
                find_nodes_recursive(init, predicate, results);
            }
        }
        NodeKind::VariableListDeclaration { initializer, .. } => {
            if let Some(init) = initializer {
                find_nodes_recursive(init, predicate, results);
            }
        }
        NodeKind::Assignment { lhs, rhs, .. } => {
            find_nodes_recursive(lhs, predicate, results);
            find_nodes_recursive(rhs, predicate, results);
        }
        NodeKind::Binary { left, right, .. } => {
            find_nodes_recursive(left, predicate, results);
            find_nodes_recursive(right, predicate, results);
        }
        NodeKind::Unary { operand, .. } => {
            find_nodes_recursive(operand, predicate, results);
        }
        NodeKind::Ternary { condition, then_expr, else_expr } => {
            find_nodes_recursive(condition, predicate, results);
            find_nodes_recursive(then_expr, predicate, results);
            find_nodes_recursive(else_expr, predicate, results);
        }
        NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
            find_nodes_recursive(condition, predicate, results);
            find_nodes_recursive(then_branch, predicate, results);
            for (_, branch) in elsif_branches {
                find_nodes_recursive(branch, predicate, results);
            }
            if let Some(else_branch) = else_branch {
                find_nodes_recursive(else_branch, predicate, results);
            }
        }
        NodeKind::While { condition, body, continue_block } => {
            find_nodes_recursive(condition, predicate, results);
            find_nodes_recursive(body, predicate, results);
            if let Some(cont) = continue_block {
                find_nodes_recursive(cont, predicate, results);
            }
        }
        NodeKind::For { init, condition, update, body, continue_block } => {
            if let Some(init) = init {
                find_nodes_recursive(init, predicate, results);
            }
            if let Some(cond) = condition {
                find_nodes_recursive(cond, predicate, results);
            }
            if let Some(upd) = update {
                find_nodes_recursive(upd, predicate, results);
            }
            find_nodes_recursive(body, predicate, results);
            if let Some(cont) = continue_block {
                find_nodes_recursive(cont, predicate, results);
            }
        }
        NodeKind::Foreach { variable, list, body, continue_block } => {
            find_nodes_recursive(variable, predicate, results);
            find_nodes_recursive(list, predicate, results);
            find_nodes_recursive(body, predicate, results);
            if let Some(cont) = continue_block {
                find_nodes_recursive(cont, predicate, results);
            }
        }
        NodeKind::Try { body, catch_blocks, finally_block } => {
            find_nodes_recursive(body, predicate, results);
            for (_, catch_body) in catch_blocks {
                find_nodes_recursive(catch_body, predicate, results);
            }
            if let Some(final_body) = finally_block {
                find_nodes_recursive(final_body, predicate, results);
            }
        }
        NodeKind::Given { expr, body } => {
            find_nodes_recursive(expr, predicate, results);
            find_nodes_recursive(body, predicate, results);
        }
        NodeKind::When { condition, body } => {
            find_nodes_recursive(condition, predicate, results);
            find_nodes_recursive(body, predicate, results);
        }
        NodeKind::Default { body } => {
            find_nodes_recursive(body, predicate, results);
        }
        NodeKind::Subroutine { body, .. } => {
            find_nodes_recursive(body, predicate, results);
        }
        NodeKind::Method { body, .. } => {
            find_nodes_recursive(body, predicate, results);
        }
        NodeKind::Class { body, .. } => {
            find_nodes_recursive(body, predicate, results);
        }
        NodeKind::FunctionCall { args, .. } => {
            for arg in args {
                find_nodes_recursive(arg, predicate, results);
            }
        }
        NodeKind::MethodCall { object, args, .. } => {
            find_nodes_recursive(object, predicate, results);
            for arg in args {
                find_nodes_recursive(arg, predicate, results);
            }
        }
        NodeKind::ArrayLiteral { elements } => {
            for element in elements {
                find_nodes_recursive(element, predicate, results);
            }
        }
        NodeKind::HashLiteral { pairs } => {
            for (key, value) in pairs {
                find_nodes_recursive(key, predicate, results);
                find_nodes_recursive(value, predicate, results);
            }
        }
        NodeKind::StatementModifier { statement, condition, .. } => {
            find_nodes_recursive(statement, predicate, results);
            find_nodes_recursive(condition, predicate, results);
        }
        NodeKind::LabeledStatement { statement, .. } => {
            find_nodes_recursive(statement, predicate, results);
        }
        NodeKind::Eval { block } => {
            find_nodes_recursive(block, predicate, results);
        }
        NodeKind::Do { block } => {
            find_nodes_recursive(block, predicate, results);
        }
        NodeKind::Return { value } => {
            if let Some(val) = value {
                find_nodes_recursive(val, predicate, results);
            }
        }
        NodeKind::LoopControl { .. } => {} // No children
        NodeKind::Tie { variable, package, args } => {
            find_nodes_recursive(variable, predicate, results);
            find_nodes_recursive(package, predicate, results);
            for arg in args {
                find_nodes_recursive(arg, predicate, results);
            }
        }
        NodeKind::Untie { variable } => {
            find_nodes_recursive(variable, predicate, results);
        }
        NodeKind::Readline { .. } => {} // No complex children
        NodeKind::Diamond => {} // No children
        NodeKind::Glob { .. } => {} // No children
        NodeKind::Typeglob { .. } => {} // No children
        NodeKind::Number { .. } => {} // No children
        NodeKind::String { .. } => {} // No children
        NodeKind::Heredoc { .. } => {} // No children
        NodeKind::Undef => {} // No children
        NodeKind::Ellipsis => {} // No children
        NodeKind::Regex { .. } => {} // No children
        NodeKind::Match { expr, .. } => {
            find_nodes_recursive(expr, predicate, results);
        }
        NodeKind::Substitution { expr, .. } => {
            find_nodes_recursive(expr, predicate, results);
        }
        NodeKind::Transliteration { expr, .. } => {
            find_nodes_recursive(expr, predicate, results);
        }
        NodeKind::Package { block, .. } => {
            if let Some(b) = block {
                find_nodes_recursive(b, predicate, results);
            }
        }
        NodeKind::Use { .. } => {} // No complex children
        NodeKind::No { .. } => {} // No complex children
        NodeKind::PhaseBlock { block, .. } => {
            find_nodes_recursive(block, predicate, results);
        }
        NodeKind::DataSection { .. } => {} // No children
        NodeKind::Format { .. } => {} // No children
        NodeKind::Identifier { .. } => {} // No children
        NodeKind::Variable { .. } => {} // No children
        NodeKind::VariableWithAttributes { variable, .. } => {
            find_nodes_recursive(variable, predicate, results);
        }
        NodeKind::Prototype { .. } => {} // No children
        NodeKind::Signature { parameters } => {
            for param in parameters {
                find_nodes_recursive(param, predicate, results);
            }
        }
        NodeKind::MandatoryParameter { variable } => {
            find_nodes_recursive(variable, predicate, results);
        }
        NodeKind::OptionalParameter { variable, default_value } => {
            find_nodes_recursive(variable, predicate, results);
            find_nodes_recursive(default_value, predicate, results);
        }
        NodeKind::SlurpyParameter { variable } => {
            find_nodes_recursive(variable, predicate, results);
        }
        NodeKind::NamedParameter { variable } => {
            find_nodes_recursive(variable, predicate, results);
        }
        NodeKind::IndirectCall { object, args, .. } => {
            find_nodes_recursive(object, predicate, results);
            for arg in args {
                find_nodes_recursive(arg, predicate, results);
            }
        }
        NodeKind::Error { partial, .. } => {
            if let Some(p) = partial {
                find_nodes_recursive(p, predicate, results);
            }
        }
        NodeKind::MissingExpression | NodeKind::MissingStatement | 
        NodeKind::MissingIdentifier | NodeKind::MissingBlock => {} // No children
        NodeKind::UnknownRest => {} // No children
    }
}