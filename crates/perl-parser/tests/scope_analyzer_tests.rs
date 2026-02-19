use perl_parser::{
    Parser,
    pragma_tracker::PragmaTracker,
    scope_analyzer::{IssueKind, ScopeAnalyzer, ScopeIssue},
};

fn analyze_code(code: &str) -> Vec<ScopeIssue> {
    use perl_tdd_support::must;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let analyzer = ScopeAnalyzer::new();
    let pragma_map = PragmaTracker::build(&ast);
    analyzer.analyze(&ast, code, &pragma_map)
}

#[test]
fn test_undefined_variable_detection() {
    let code = r#"
use strict;
print $undefined_var;
"#;

    let issues = analyze_code(code);
    assert!(issues.iter().any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)));
}

#[test]
fn test_unused_variable_detection() {
    let code = r#"
use warnings;
my $unused = 42;
print "Hello";
"#;

    let issues = analyze_code(code);
    assert!(issues.iter().any(|i| matches!(i.kind, IssueKind::UnusedVariable)));
}

#[test]
fn test_variable_shadowing() {
    let code = r#"
my $x = 1;
{
    my $x = 2;  # shadows outer $x
    print $x;
}
"#;

    let issues = analyze_code(code);
    assert!(issues.iter().any(|i| matches!(i.kind, IssueKind::VariableShadowing)));
}

#[test]
fn test_our_variable_not_undefined() {
    let code = r#"
use strict;
our $global_var = 42;
print $global_var;
"#;

    let issues = analyze_code(code);
    assert!(!issues.iter().any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)));
}

#[test]
fn test_local_variable_not_undefined() {
    let code = r#"
use strict;
local $custom_var = '/usr/bin';
print $custom_var;
print %ENV;  # Built-in global should not trigger undefined
"#;

    let issues = analyze_code(code);
    // %ENV is a built-in global
    assert!(!issues
        .iter()
        .any(|i| matches!(i.kind, IssueKind::UndeclaredVariable) && (i.variable_name == "%ENV")));
    // local $custom_var should not trigger undefined either
    assert!(
        !issues.iter().any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)
            && (i.variable_name == "$custom_var"))
    );
}

#[test]
fn test_package_variable_tracking() {
    let code = r#"
package Foo;
use strict;
our $package_var = 42;

package Bar;
use strict;
print $Foo::package_var;  # Should be ok
"#;

    let issues = analyze_code(code);
    assert!(!issues.iter().any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)
        && (i.variable_name == "$Foo::package_var")));
}

#[test]
fn test_subroutine_parameters() {
    let code = r#"
use strict;
sub foo {
    my ($x, $y) = @_;
    return $x + $y;
}
"#;

    let issues = analyze_code(code);
    assert!(!issues.iter().any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)));
}

#[test]
fn test_foreach_loop_variable() {
    let code = r#"
use strict;
my @arr = (1, 2, 3);
foreach my $item (@arr) {
    print $item;
}
"#;

    let issues = analyze_code(code);
    assert!(!issues.iter().any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)));
}

#[test]
fn test_while_loop_variable() {
    let code = r#"
use strict;
while (my $line = <STDIN>) {
    print $line;
}
"#;

    let issues = analyze_code(code);
    assert!(!issues.iter().any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)));
}

#[test]
fn test_if_condition_variable() {
    let code = r#"
use strict;
if (my $result = some_func()) {
    print $result;
}
"#;

    let issues = analyze_code(code);
    // some_func is undefined but $result should be ok
    assert!(
        !issues
            .iter()
            .any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)
                && (i.variable_name == "$result"))
    );
}

#[test]
fn test_nested_scopes() {
    let code = r#"
use strict;
my $outer = 1;
{
    my $inner = 2;
    print $outer;  # ok
    print $inner;  # ok
}
print $outer;  # ok
print $inner;  # undefined
"#;

    let issues = analyze_code(code);
    assert!(
        issues
            .iter()
            .any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)
                && (i.variable_name == "$inner"))
    );
}

#[test]
fn test_builtin_variables() {
    let code = r#"
use strict;
print $_;
print @ARGV;
print %ENV;
print $0;
print $!;
print $@;
print $$;
"#;

    let issues = analyze_code(code);
    // None of these should be undefined
    assert!(!issues.iter().any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)));
}

#[test]
fn test_unused_variable_basic() {
    let code = r#"
my $unused = 42;
"#;

    let issues = analyze_code(code);

    // Check for unused variables
    let unused_issues: Vec<_> =
        issues.iter().filter(|i| matches!(i.kind, IssueKind::UnusedVariable)).collect();
    assert!(!unused_issues.is_empty());
}

#[test]
fn test_my_declaration_in_list() {
    let code = r#"
use strict;
my ($x, $y, $z) = (1, 2, 3);
print "$x $y $z";
"#;

    let issues = analyze_code(code);
    assert!(!issues.iter().any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)));
}

#[test]
fn test_hash_slice_not_undefined() {
    let code = r#"
use strict;
my %hash = (a => 1, b => 2);
my @values = @hash{'a', 'b'};
print @values;
"#;

    let issues = analyze_code(code);
    assert!(!issues.iter().any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)));
}

#[test]
fn test_array_slice_not_undefined() {
    let code = r#"
use strict;
my @array = (1, 2, 3, 4);
my @slice = @array[0..2];
print @slice;
"#;

    let issues = analyze_code(code);
    assert!(!issues.iter().any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)));
}

#[test]
fn test_special_blocks() {
    let code = r#"
use strict;
BEGIN {
    my $begin_var = 1;
    print $begin_var;  # ok
}
END {
    my $end_var = 2;
    print $end_var;  # ok
}
"#;

    let issues = analyze_code(code);
    assert!(!issues.iter().any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)));
}

#[test]
fn test_use_vars_pragma() {
    let code = r#"
use strict;
our $global_var;
our @global_array;
$global_var = 42;
@global_array = (1, 2, 3);
"#;

    let issues = analyze_code(code);
    // Variables declared with 'our' should not be undefined
    assert!(!issues.iter().any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)));
}

#[test]
fn test_glob_assignment() {
    let code = r#"
use strict;
*alias = *main::original;
"#;

    let issues = analyze_code(code);
    // Glob assignments are special and shouldn't trigger undefined warnings
    // Note: This might need special handling in the analyzer
    assert!(issues.len() <= 2); // Only pragma warnings expected
}

#[test]
fn test_state_variable() {
    let code = r#"
use strict;
use feature 'state';
sub counter {
    state $count = 0;
    return ++$count;
}
"#;

    let issues = analyze_code(code);
    assert!(!issues.iter().any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)));
}

#[test]
fn test_regex_capture_variables() {
    let code = r#"
use strict;
my $text = "hello world";
if ($text =~ /(\w+)\s+(\w+)/) {
    print "$1 $2";  # Capture variables should be recognized
}
"#;

    let issues = analyze_code(code);
    // $1, $2 etc. are special regex capture variables
    assert!(!issues.iter().any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)
        && (i.variable_name == "$1" || i.variable_name == "$2")));
}

#[test]
fn test_eval_block() {
    let code = r#"
use strict;
eval {
    my $eval_var = 42;
    print $eval_var;  # ok
};
"#;

    let issues = analyze_code(code);
    assert!(!issues.iter().any(
        |i| matches!(i.kind, IssueKind::UndeclaredVariable) && (i.variable_name == "$eval_var")
    ));
}

#[test]
fn test_do_block() {
    let code = r#"
use strict;
my $result = do {
    my $temp = 42;
    $temp * 2
};
print $result;
"#;

    let issues = analyze_code(code);
    assert!(!issues.iter().any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)));
}

#[test]
fn test_given_when() {
    let code = r#"
use strict;
use feature 'switch';
my $value = 5;
given ($value) {
    when (1) { print "one" }
    when (2) { print "two" }
    default { print "other" }
}
"#;

    let issues = analyze_code(code);
    assert!(!issues.iter().any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)));
}

// ================================
// Enhanced Variable Resolution Tests
// Testing the new try_resolve_variable_reference functionality
// ================================

#[test]
fn test_hash_access_variable_resolution() {
    let code = r#"
use strict;
my %config = (path => '/usr/bin', debug => 1);
my $v = $config{path};  # Should resolve $config{path} -> %config
"#;

    let issues = analyze_code(code);
    // $config{path} should be resolved to %config and not trigger undefined
    assert!(
        !issues.iter().any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)
            && i.variable_name.contains("config"))
    );
}

#[test]
fn test_array_access_variable_resolution() {
    let code = r#"
use strict;
my @items = (1, 2, 3, 4);
my $v1 = $items[0];  # Should resolve $items[0] -> @items
my $v2 = $items[1];  # Should resolve $items[1] -> @items
"#;

    let issues = analyze_code(code);
    // $items[0] and $items[1] should be resolved to @items and not trigger undefined
    assert!(
        !issues.iter().any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)
            && i.variable_name.contains("items"))
    );
}

#[test]
fn test_nested_hash_access_resolution() {
    let code = r#"
use strict;
my %data = (user => { name => 'John', age => 30 });
my $v1 = $data{user};      # Should resolve to %data
my $v2 = $data{settings};  # Should resolve to %data
"#;

    let issues = analyze_code(code);
    // Both hash accesses should resolve to %data
    assert!(!issues.iter().any(
        |i| matches!(i.kind, IssueKind::UndeclaredVariable) && i.variable_name.contains("data")
    ));
}

#[test]
fn test_mixed_array_hash_access() {
    let code = r#"
use strict;
my @users = ({name => 'Alice'}, {name => 'Bob'});
my %lookup = (alice => 0, bob => 1);
my $v1 = $users[0];        # Should resolve to @users
my $v2 = $lookup{alice};   # Should resolve to %lookup
"#;

    let issues = analyze_code(code);
    // No undefined variable errors should occur
    assert!(!issues.iter().any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)));
}

#[test]
fn test_complex_variable_patterns() {
    let code = r#"
use strict;
my %config = (db => {host => 'localhost'});
my @servers = ('web1', 'web2', 'db1');
my $index = 0;

# Complex patterns that should resolve
my $v1 = $config{db};      # %config access
my $v2 = $servers[0];      # @servers access
my $v3 = $servers[$index]; # @servers access (dynamic index)
"#;

    let issues = analyze_code(code);
    // All variables should be properly resolved
    assert!(!issues.iter().any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)));
}

#[test]
fn test_variable_resolution_with_undeclared_base() {
    let code = r#"
use strict;
my $v1 = $undeclared_hash{key};    # Should trigger undefined for hash access
my $v2 = $undeclared_array[0];     # Should trigger undefined for array access
"#;

    let issues = analyze_code(code);
    // Should find undefined variables for the hash and array accesses
    assert!(issues.iter().any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)
        && i.variable_name.starts_with("$undeclared_hash")));
    assert!(issues.iter().any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)
        && i.variable_name.starts_with("$undeclared_array")));
}

#[test]
fn test_hash_key_context_detection() {
    let code = r#"
use strict;
my %hash = (key1 => 'value1');
print $hash{bareword_key};  # bareword_key should be treated as hash key, not undefined identifier
"#;

    let issues = analyze_code(code);
    // bareword_key in hash context should not trigger bareword warning
    assert!(!issues.iter().any(
        |i| matches!(i.kind, IssueKind::UnquotedBareword) && i.variable_name == "bareword_key"
    ));
}

#[test]
fn test_array_slice_variable_resolution() {
    let code = r#"
use strict;
my @colors = ('red', 'green', 'blue');
my @subset = @colors[0, 2];  # Should resolve @colors[0,2] -> @colors
my @v = @subset;
"#;

    let issues = analyze_code(code);
    // Array slice should not trigger undefined
    assert!(
        !issues.iter().any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)
            && i.variable_name.contains("colors"))
    );
}

#[test]
fn test_hash_slice_variable_resolution() {
    let code = r#"
use strict;
my %settings = (debug => 1, verbose => 0, level => 2);
my @values = @settings{qw(debug verbose)};  # Should resolve to %settings
my @v = @values;
"#;

    let issues = analyze_code(code);
    // Hash slice should not trigger undefined
    assert!(
        !issues.iter().any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)
            && i.variable_name.contains("settings"))
    );
}

#[test]
fn test_enhanced_resolution_edge_cases() {
    let code = r#"
use strict;
my %data = ();
my @list = ();

# Edge cases for enhanced resolution
my $v1 = $list[0];         # Zero index
my $v2 = $data{key};       # Simple hash key
my $v3 = $list[-1];        # Negative array index (if supported)
"#;

    let issues = analyze_code(code);
    // All accesses should resolve to declared variables
    assert!(!issues.iter().any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)
        && (i.variable_name.contains("data") || i.variable_name.contains("list"))));
}

#[test]
fn test_sigil_conversion_accuracy() {
    let code = r#"
use strict;
my %hash_var = (a => 1);
my @array_var = (1, 2, 3);

# Test sigil conversion: $hash{key} should resolve to %hash, not $hash
my $v1 = $hash_var{key};
# Test sigil conversion: $array[idx] should resolve to @array, not $array  
my $v2 = $array_var[0];
"#;

    let issues = analyze_code(code);
    // Enhanced resolution should properly convert sigils
    assert!(!issues.iter().any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)));
}

#[test]
fn test_method_call_variable_patterns() {
    let code = r#"
use strict;
my $obj = bless {}, 'MyClass';
my @methods = ('get', 'set');

# Variable access patterns in method context (should not affect resolution)
my $v1 = $obj;             # Simple variable access
my @v2 = @methods;         # Simple array access
"#;

    let issues = analyze_code(code);
    // Should not flag the base variables as undefined
    assert!(!issues.iter().any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)
        && (i.variable_name == "$obj" || i.variable_name == "@methods")));
}

#[test]
fn test_enhanced_resolution_recursion() {
    let code = r#"
use strict;
my %outer = (inner => {deep => 'value'});

# Test that enhanced resolution handles recursive patterns
my $v = $outer{inner};    # Should resolve to %outer
"#;

    let issues = analyze_code(code);
    // Recursive resolution should work
    assert!(
        !issues.iter().any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)
            && i.variable_name.contains("outer"))
    );
}

#[test]
fn test_enhanced_resolution_fallback() {
    let code = r#"
use strict;
my $simple_var = 42;

# Simple variable access should still work (fallback case)
my $v = $simple_var;
"#;

    let issues = analyze_code(code);
    // Simple variables should still work with enhanced resolution
    assert!(!issues.iter().any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)));
}

#[test]
fn test_hash_key_not_flagged_as_bareword() {
    let code = r#"
use strict;
my %h = ();
my $x = $h{key};
print FOO;
"#;

    let issues = analyze_code(code);
    let bareword_issues: Vec<_> =
        issues.iter().filter(|i| matches!(i.kind, IssueKind::UnquotedBareword)).collect();
    assert_eq!(bareword_issues.len(), 1);
    assert_eq!(bareword_issues[0].variable_name, "FOO");
}

#[test]
fn test_hash_slice_bareword_keys() {
    let code = r#"
use strict;
my %h = ();
my @values = @h{key1, key2};
print STDERR;
"#;

    let issues = analyze_code(code);
    let bareword_issues: Vec<_> =
        issues.iter().filter(|i| matches!(i.kind, IssueKind::UnquotedBareword)).collect();
    assert_eq!(bareword_issues.len(), 1);
    assert_eq!(bareword_issues[0].variable_name, "STDERR");
}

#[test]
fn test_comprehensive_hash_key_context() {
    let code = r#"
use strict;
my %hash = (key1 => 'value1', key2 => 'value2');
my $value = $hash{bareword_key};
my @values = @hash{key1, key2, another_key};
my %hash2 = ( another_key => 'value' );
print INVALID_BAREWORD;
"#;

    let issues = analyze_code(code);
    let bareword_issues: Vec<_> =
        issues.iter().filter(|i| matches!(i.kind, IssueKind::UnquotedBareword)).collect();

    // Only INVALID_BAREWORD should be flagged - hash keys should be ignored
    assert_eq!(bareword_issues.len(), 1);
    assert_eq!(bareword_issues[0].variable_name, "INVALID_BAREWORD");
}

#[test]
fn test_scalar_usage_with_only_hash_declared() {
    let code = r#"
use strict;
my %h = ();
print $h; # Should NOT resolve to %h, because it is a scalar usage
"#;
    let issues = analyze_code(code);
    assert!(
        issues
            .iter()
            .any(|i| matches!(i.kind, IssueKind::UndeclaredVariable) && i.variable_name == "$h"),
        "Expected UndeclaredVariable for $h, but got: {:?}",
        issues
    );
}

#[test]
fn test_scalar_usage_with_only_array_declared() {
    let code = r#"
use strict;
my @arr = ();
print $arr; # Should NOT resolve to @arr
"#;
    let issues = analyze_code(code);
    assert!(
        issues
            .iter()
            .any(|i| matches!(i.kind, IssueKind::UndeclaredVariable) && i.variable_name == "$arr"),
        "Expected UndeclaredVariable for $arr, but got: {:?}",
        issues
    );
}

#[test]
fn test_hash_element_access_correctness() {
    let code = r#"
use strict;
my %h = ();
my $v = $h{k}; # Should resolve to %h
print($h{k}); # Should also resolve (FunctionCall -> Binary)
"#;
    let issues = analyze_code(code);
    assert!(
        !issues.iter().any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)),
        "Unexpected undeclared variable error for $h{{k}}: {:?}",
        issues
    );
}

#[test]
fn test_array_element_access_correctness() {
    let code = r#"
use strict;
my @a = ();
print($a[0]); # Should resolve to @a
"#;
    let issues = analyze_code(code);
    assert!(
        !issues.iter().any(|i| matches!(i.kind, IssueKind::UndeclaredVariable)),
        "Unexpected undeclared variable error for $a[0]: {:?}",
        issues
    );
}
