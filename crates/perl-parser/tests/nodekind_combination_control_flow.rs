//! Comprehensive tests for control flow combinations
//! 
//! These tests validate complex interactions between control flow constructs
//! including labeled statements, nested loops, statement modifiers, return statements,
//! and loop control operations.

use perl_parser::{Parser, ast::{Node, NodeKind}};

/// Test labeled statements with nested loops and conditionals
#[test]
fn test_labeled_nested_loops_conditionals() {
    let code = r#"
OUTER: for my $i (1..10) {
    INNER: for my $j (1..10) {
        if ($i * $j > 50) {
            last OUTER;
        }
        
        if ($i == $j) {
            next INNER;
        }
        
        if ($j % 2 == 0) {
            redo INNER if $i < 3;
        }
        
        print "$i x $j = " . ($i * $j) . "\n";
    }
}

SEARCH: while (my $line = <DATA>) {
    chomp $line;
    
    LINE: for my $word (split /\s+/, $line) {
        if ($word eq 'QUIT') {
            last SEARCH;
        }
        
        if ($word =~ /^\d+$/) {
            next LINE;
        }
        
        if (length $word > 10) {
            redo LINE if $word !~ /[aeiou]/;
        }
        
        print "Word: $word\n";
    }
}

PROCESS: foreach my $file (@ARGV) {
    open my $fh, '<', $file or do {
        warn "Cannot open $file: $!";
        next PROCESS;
    };
    
    while (my $line = <$fh>) {
        if ($line =~ /^__END__$/) {
            last PROCESS;
        }
        
        if ($line =~ /^\s*#/) {
            next;
        }
        
        print $line;
    }
    
    close $fh;
}

__DATA__
hello world
test 123
QUIT
longwordwithoutvowels
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse().expect("Should parse successfully");
    
    // Verify labeled statements
    let labeled_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::LabeledStatement { .. }));
    assert!(!labeled_nodes.is_empty(), "Should have labeled statements");
    
    // Verify different loop types
    let for_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::For { .. }));
    let foreach_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Foreach { .. }));
    let while_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::While { .. }));
    
    assert!(!for_nodes.is_empty(), "Should have for loops");
    assert!(!foreach_nodes.is_empty(), "Should have foreach loops");
    assert!(!while_nodes.is_empty(), "Should have while loops");
    
    // Verify loop control statements
    let loop_control_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::LoopControl { .. }));
    assert!(!loop_control_nodes.is_empty(), "Should have loop control statements");
    
    // Verify conditional statements
    let if_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::If { .. }));
    assert!(!if_nodes.is_empty(), "Should have if statements");
}

/// Test statement modifiers with complex expressions and subroutines
#[test]
fn test_statement_modifiers_complex_expressions() {
    let code = r#"
my $count = 0;
my @results;
my %processed;

# Simple modifiers
print "Starting process\n" if $count == 0;
$count++ unless defined $ENV{SKIP_COUNT};

# Complex expression modifiers
push @results, process_item($_) for @input_data;
$processed{$_} = 1 foreach grep { defined $_ } @filtered_data;

# Nested modifiers with subroutine calls
warn "Processing complete" if $count > 100 and check_status();
$ENV{DEBUG} = 1 unless validate_config() or $ENV{FORCE_DEBUG};

# Modifiers with method calls
$obj->reset() if $obj->can('reset');
$self->cleanup() unless $self->is_valid();

# Complex conditional logic
$cache{$_} = expensive_calculation($_) for grep { !exists $cache{$_} } @uncached_keys;
@sorted = sort { compare_items($a, $b) } grep { is_valid($_) } @unsorted_items;

# Modifiers with regex and string operations
s/^\s+|\s+$//g for @lines_to_clean;
tr/a-z/A-Z/ for @uppercase_strings;

# Error handling with modifiers
log_error($@) if $@;
rollback_transaction() unless commit_successful();

# File operations with modifiers
close $fh if $fh and $fh->opened();
unlink $temp_file if -e $temp_file and $temp_file =~ /\.tmp$/;

sub process_item {
    my ($item) = @_;
    return uc $item;
}

sub check_status {
    return $count > 50;
}

sub validate_config {
    return -f 'config.json';
}

sub compare_items {
    my ($a, $b) = @_;
    return $a cmp $b;
}

sub is_valid {
    my ($item) = @_;
    return defined $item && length $item > 0;
}

sub expensive_calculation {
    my ($key) = @_;
    return $key * 2; # Simulate expensive operation
}

sub log_error {
    my ($error) = @_;
    print STDERR "Error: $error\n";
}

sub rollback_transaction {
    print "Rolling back transaction\n";
}

sub commit_successful {
    return 1; # Simulate successful commit
}
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse().expect("Should parse successfully");
    
    // Verify statement modifiers
    let modifier_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::StatementModifier { .. }));
    assert!(!modifier_nodes.is_empty(), "Should have statement modifiers");
    
    // Verify different types of expressions in modifiers
    let binary_ops = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Binary { .. }));
    let function_calls = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::FunctionCall { .. }));
    let method_calls = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::MethodCall { .. }));
    
    assert!(!binary_ops.is_empty(), "Should have binary operations");
    assert!(!function_calls.is_empty(), "Should have function calls");
    assert!(!method_calls.is_empty(), "Should have method calls");
    
    // Verify regex operations
    let substitution_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Substitution { .. }));
    let transliteration_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Transliteration { .. }));
    
    assert!(!substitution_nodes.is_empty(), "Should have substitution operations");
    assert!(!transliteration_nodes.is_empty(), "Should have transliteration operations");
}

/// Test return statements in various contexts
#[test]
fn test_return_statements_various_contexts() {
    let code = r#"
sub simple_return {
    return;
}

sub return_with_value {
    return "success";
}

sub conditional_return {
    my ($condition) = @_;
    
    if ($condition) {
        return "true branch";
    } else {
        return "false branch";
    }
}

sub loop_return {
    my ($max) = @_;
    my $sum = 0;
    
    for my $i (1..$max) {
        if ($i > 100) {
            return "overflow";
        }
        
        $sum += $i;
        
        if ($sum > 1000) {
            return $sum;
        }
    }
    
    return $sum;
}

sub nested_return {
    my ($data) = @_;
    
    foreach my $item (@$data) {
        if (ref $item eq 'HASH') {
            while (my ($key, $value) = each %$item) {
                if ($key eq 'stop') {
                    return "found stop key";
                }
                
                if (ref $value eq 'ARRAY') {
                    for my $subitem (@$value) {
                        if ($subitem eq 'abort') {
                            return "found abort value";
                        }
                    }
                }
            }
        }
    }
    
    return "completed";
}

sub eval_return {
    my ($code) = @_;
    
    my $result = eval {
        return eval_sub($code);
    };
    
    if ($@) {
        return "error: $@";
    }
    
    return $result;
}

sub eval_sub {
    my ($code) = @_;
    return eval $code;
}

sub try_catch_return {
    my ($value) = @_;
    
    try {
        if ($value < 0) {
            return "negative";
        }
        
        if ($value > 100) {
            return "too large";
        }
        
        return "acceptable: $value";
    } catch ($e) {
        return "exception: $e";
    }
}

sub subroutine_reference_return {
    my ($sub_ref) = @_;
    
    return $sub_ref->(@_) if ref $sub_ref eq 'CODE';
    return "not a subroutine reference";
}

# Test calls
my $result1 = simple_return();
my $result2 = return_with_value();
my $result3 = conditional_return(1);
my $result4 = loop_return(50);
my $result5 = nested_return([{a => 1, b => ['x', 'abort', 'z']}]);
my $result6 = eval_return('return 42');
my $result7 = try_catch_return(25);
my $result8 = subroutine_reference_return(sub { return shift() * 2 });
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse().expect("Should parse successfully");
    
    // Verify return statements
    let return_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Return { .. }));
    assert!(!return_nodes.is_empty(), "Should have return statements");
    
    // Verify returns in different contexts
    let if_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::If { .. }));
    let for_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::For { .. }));
    let foreach_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Foreach { .. }));
    let while_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::While { .. }));
    let eval_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Eval { .. }));
    let try_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Try { .. }));
    
    assert!(!if_nodes.is_empty(), "Should have if statements with returns");
    assert!(!for_nodes.is_empty(), "Should have for loops with returns");
    assert!(!foreach_nodes.is_empty(), "Should have foreach loops with returns");
    assert!(!while_nodes.is_empty(), "Should have while loops with returns");
    assert!(!eval_nodes.is_empty(), "Should have eval blocks with returns");
    assert!(!try_nodes.is_empty(), "Should have try blocks with returns");
}

/// Test loop control (next/last/redo/continue) with complex nesting
#[test]
fn test_loop_control_complex_nesting() {
    let code = r#"
my @data = (1..20);
my @results;

# Basic next/last/redo
OUTER: for my $i (@data) {
    if ($i % 2 == 0) {
        next OUTER;
    }
    
    if ($i > 15) {
        last OUTER;
    }
    
    if ($i == 7) {
        $i--; # Modify to test redo
        redo OUTER;
    }
    
    push @results, $i;
}

# Nested loop control with labels
MATRIX: for my $row (1..5) {
    ROW: for my $col (1..5) {
        if ($row == 3 && $col == 3) {
            last MATRIX; # Exit outer loop
        }
        
        if ($col == 2) {
            next ROW; # Skip to next column
        }
        
        if ($row * $col > 10) {
            redo ROW; # Retry current row
        }
        
        print "Processing [$row,$col]\n";
    }
}

# Continue blocks
for my $i (1..10) {
    print "Iteration $i\n";
    
    if ($i % 3 == 0) {
        next;
    }
    
    print "  Processed $i\n";
} continue {
    print "  Continue block for $i\n";
}

while (my $line = <DATA>) {
    chomp $line;
    
    if ($line eq 'QUIT') {
        last;
    }
    
    if ($line =~ /^\s*#/) {
        next;
    }
    
    if ($line eq 'RETRY') {
        redo;
    }
    
    print "Line: $line\n";
} continue {
    print "  Processed line\n";
}

# Foreach with continue
foreach my $item (@data) {
    if ($item == 13) {
        next;
    }
    
    print "Item: $item\n";
} continue {
    print "  Finished item\n";
}

# Complex nested with multiple control types
SEARCH: foreach my $file (@files) {
    open my $fh, '<', $file or next SEARCH;
    
    LINE: while (my $line = <$fh>) {
        if ($line =~ /^__END__$/) {
            last LINE;
        }
        
        if ($line =~ /^\s*$/) {
            next LINE;
        }
        
        if ($line =~ /ERROR/i) {
            redo LINE if $line =~ /retry/i;
            last SEARCH;
        }
        
        if ($line =~ /SKIP/i) {
            next SEARCH;
        }
        
        process_line($line);
    }
    
    close $fh;
} continue {
    print "Finished processing $file\n";
}

__DATA__
line 1
# comment
line 2
RETRY
ERROR: retry this line
SKIP this file
QUIT
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse().expect("Should parse successfully");
    
    // Verify loop control statements
    let loop_control_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::LoopControl { .. }));
    assert!(!loop_control_nodes.is_empty(), "Should have loop control statements");
    
    // Check for different control types
    let mut next_count = 0;
    let mut last_count = 0;
    let mut redo_count = 0;
    
    for node in &loop_control_nodes {
        if let NodeKind::LoopControl { op, .. } = &node.kind {
            match op.as_str() {
                "next" => next_count += 1,
                "last" => last_count += 1,
                "redo" => redo_count += 1,
                _ => {}
            }
        }
    }
    
    assert!(next_count > 0, "Should have next statements");
    assert!(last_count > 0, "Should have last statements");
    assert!(redo_count > 0, "Should have redo statements");
    
    // Verify continue blocks
    let for_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::For { .. }));
    let foreach_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Foreach { .. }));
    let while_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::While { .. }));
    
    // Check for continue blocks in loops
    let mut continue_count = 0;
    
    for node in &for_nodes {
        if let NodeKind::For { continue_block, .. } = &node.kind {
            if continue_block.is_some() {
                continue_count += 1;
            }
        }
    }
    
    for node in &foreach_nodes {
        if let NodeKind::Foreach { continue_block, .. } = &node.kind {
            if continue_block.is_some() {
                continue_count += 1;
            }
        }
    }
    
    for node in &while_nodes {
        if let NodeKind::While { continue_block, .. } = &node.kind {
            if continue_block.is_some() {
                continue_count += 1;
            }
        }
    }
    
    assert!(continue_count > 0, "Should have continue blocks");
    
    // Verify labeled statements with loop control
    let labeled_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::LabeledStatement { .. }));
    assert!(!labeled_nodes.is_empty(), "Should have labeled statements");
}

/// Test complex conditional structures
#[test]
fn test_complex_conditional_structures() {
    let code = r#"
# Nested if-elsif-else
sub complex_conditional {
    my ($value) = @_;
    
    if (!defined $value) {
        return "undefined";
    } elsif (ref $value) {
        if (ref $value eq 'ARRAY') {
            return "array with " . scalar(@$value) . " elements";
        } elsif (ref $value eq 'HASH') {
            return "hash with " . scalar(keys %$value) . " keys";
        } elsif (ref $value eq 'CODE') {
            return "code reference";
        } else {
            return "other reference: " . ref $value;
        }
    } else {
        if ($value =~ /^\d+$/) {
            return "integer: $value";
        } elsif ($value =~ /^\d*\.\d+$/) {
            return "float: $value";
        } elsif ($value =~ /^[a-zA-Z]+$/) {
            return "string: $value";
        } else {
            return "mixed: $value";
        }
    }
}

# Ternary operators
sub ternary_chain {
    my ($x, $y, $z) = @_;
    
    my $result = $x ? $y : $z;
    my $nested = $x ? ($y ? $z : $x) : ($z ? $x : $y);
    my $complex = $x ? ($y ? $z : ($x ? $y : $z)) : ($z ? ($x ? $y : $z) : $x);
    
    return $complex;
}

# Logical expressions
sub logical_conditions {
    my ($flags) = @_;
    
    if ($flags->{debug} && $flags->{verbose} && $flags->{log}) {
        print "All debug flags on\n";
    }
    
    if ($flags->{error} or $flags->{warning} or $flags->{critical}) {
        handle_error($flags);
    }
    
    if (!$flags->{dry_run} && !$flags->{test_mode}) {
        execute_operation($flags);
    }
    
    if ($flags->{auto_save} xor $flags->{manual_save}) {
        save_data($flags);
    }
    
    return $flags;
}

# Pattern matching with given/when
sub pattern_matching {
    my ($input) = @_;
    
    given ($input) {
        when (undef) {
            return "undefined input";
        }
        when (/^\d+$/) {
            return "positive integer";
        }
        when (/^-\d+$/) {
            return "negative integer";
        }
        when (/^\d*\.\d+$/) {
            return "floating point";
        }
        when (['ARRAY']) {
            return "array reference";
        }
        when (['HASH']) {
            return "hash reference";
        }
        when (['CODE']) {
            return "code reference";
        }
        when ($input > 0 && $input < 100) {
            return "small positive number";
        }
        when ($input < 0 && $input > -100) {
            return "small negative number";
        }
        default {
            return "unknown type: " . ref($input) || "scalar";
        }
    }
}

# Complex boolean logic
sub boolean_logic {
    my ($a, $b, $c) = @_;
    
    if (($a && $b) || $c) {
        return "condition 1";
    }
    
    if ($a && ($b || $c)) {
        return "condition 2";
    }
    
    if (($a || $b) && ($b || $c)) {
        return "condition 3";
    }
    
    if (!($a && $b) && !($a || $b)) {
        return "condition 4";
    }
    
    if (($a xor $b) && ($b xor $c)) {
        return "condition 5";
    }
    
    return "no condition matched";
}

sub handle_error {
    my ($flags) = @_;
    print "Error handling\n";
}

sub execute_operation {
    my ($flags) = @_;
    print "Executing operation\n";
}

sub save_data {
    my ($flags) = @_;
    print "Saving data\n";
}
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse().expect("Should parse successfully");
    
    // Verify conditional structures
    let if_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::If { .. }));
    let ternary_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Ternary { .. }));
    let given_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Given { .. }));
    let when_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::When { .. }));
    let default_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Default { .. }));
    
    assert!(!if_nodes.is_empty(), "Should have if statements");
    assert!(!ternary_nodes.is_empty(), "Should have ternary operators");
    assert!(!given_nodes.is_empty(), "Should have given statements");
    assert!(!when_nodes.is_empty(), "Should have when clauses");
    assert!(!default_nodes.is_empty(), "Should have default clauses");
    
    // Verify logical operators
    let binary_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Binary { .. }));
    assert!(!binary_nodes.is_empty(), "Should have binary operations");
    
    // Verify regex patterns
    let regex_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Regex { .. }));
    assert!(!regex_nodes.is_empty(), "Should have regex patterns");
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
fn find_nodes_recursive<F>(node: &Node, predicate: &F, results: &mut Vec<&Node>)
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
        NodeKind::Class { body } => {
            find_nodes_recursive(body, predicate, results);
        }
        NodeKind::FunctionCall { args } => {
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