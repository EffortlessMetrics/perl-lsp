//! Coverage tests for Perl constructs found to have ZERO test coverage
//! in the edge-case audit. Each section tests a distinct construct family.

mod cpan_test_helpers;
use cpan_test_helpers::*;

// ===========================================================================
// 1. UNIVERSAL methods: isa, can, UNIVERSAL::isa
// ===========================================================================

#[test]
fn universal_isa_method_call() {
    let code = r#"
if ($obj->isa('Foo')) {
    print "is a Foo\n";
}
"#;
    assert_clean_parse(code);
}

#[test]
fn universal_can_method_call() {
    let code = r#"
if ($obj->can('method')) {
    $obj->method();
}
"#;
    assert_clean_parse(code);
}

#[test]
fn universal_isa_function_call() {
    let code = r#"
if (UNIVERSAL::isa($ref, 'Foo')) {
    print "yes\n";
}
"#;
    assert_clean_parse(code);
}

// ===========================================================================
// 2. Yada yada (...)
// ===========================================================================

#[test]
fn yada_yada_in_sub() {
    let code = "sub todo { ... }";
    assert_clean_parse(code);
}

#[test]
fn yada_yada_in_method() {
    let code = r#"
sub not_yet_implemented {
    my ($self) = @_;
    ...
}
"#;
    assert_clean_parse(code);
}

// ===========================================================================
// 3. Typeglob assignment
// ===========================================================================

#[test]
fn typeglob_alias() {
    let code = "*new = \\&old;";
    assert_clean_parse(code);
}

#[test]
fn typeglob_sub_assignment() {
    let code = "*foo = sub { return 42; };";
    assert_clean_parse(code);
}

#[test]
fn typeglob_symbolic_assignment() {
    let code = r#"*{$pkg . '::func'} = sub { return 1; };"#;
    assert_clean_parse(code);
}

// ===========================================================================
// 4. Symbolic references
// ===========================================================================

#[test]
fn symbolic_scalar_deref() {
    let code = r#"
my $name = 'foo';
my $val = $$name;
"#;
    assert_clean_parse(code);
}

#[test]
fn symbolic_array_deref_block() {
    let code = r#"
my $arrayref = [1, 2, 3];
my @items = @{$arrayref};
"#;
    assert_clean_parse(code);
}

#[test]
fn symbolic_scalar_deref_block() {
    let code = r#"
my $scalarref = \42;
my $val = ${$scalarref};
"#;
    assert_clean_parse(code);
}

// ===========================================================================
// 5. V-strings
// ===========================================================================

#[test]
fn use_v_string() {
    let code = "use v5.38.0;";
    assert_clean_parse(code);
}

#[test]
fn v_string_assignment() {
    let code = "my $v = v1.2.3;";
    assert_clean_parse(code);
}

#[test]
fn v_string_comparison() {
    let code = r#"
if ($^V ge v5.10.0) {
    say "modern perl";
}
"#;
    assert_clean_parse(code);
}

// ===========================================================================
// 6. AUTOLOAD
// ===========================================================================

#[test]
fn autoload_basic() {
    let code = r#"
sub AUTOLOAD {
    my $method = our $AUTOLOAD;
    $method =~ s/.*:://;
    return if $method eq 'DESTROY';
    print "Called: $method\n";
}
"#;
    assert_clean_parse(code);
}

// ===========================================================================
// 7. Labeled loops
// ===========================================================================

#[test]
fn labeled_loop_next_last() {
    let code = r#"
OUTER: for my $i (1..10) {
    INNER: for my $j (1..10) {
        next OUTER if $j == 5;
        last INNER if $i == 3;
    }
}
"#;
    assert_clean_parse(code);
}

#[test]
fn labeled_loop_redo() {
    let code = r#"
LINE: while (my $line = <STDIN>) {
    chomp $line;
    redo LINE if $line eq '';
}
"#;
    assert_clean_parse(code);
}

// ===========================================================================
// 8. Chained ternary
// ===========================================================================

#[test]
fn chained_ternary_expression() {
    let code = r#"
my $result = $a ? $b ? 1 : 2 : $c ? 3 : 4;
"#;
    assert_clean_parse(code);
}

#[test]
fn ternary_with_method_calls() {
    let code = r#"
my $val = $obj->is_valid() ? $obj->get_value() : $default;
"#;
    assert_clean_parse(code);
}

// ===========================================================================
// 9. Complex string interpolation
// ===========================================================================

#[test]
fn string_interpolation_block() {
    let code = r#"
my @names = ('Alice', 'Bob');
my $msg = "Hello ${\ join(', ', @names) } world";
"#;
    assert_clean_parse(code);
}

#[test]
fn string_interpolation_array_element() {
    let code = r#"
my $msg = "Value is $hash{key}";
"#;
    assert_clean_parse(code);
}

// ===========================================================================
// 10. do-while
// ===========================================================================

#[test]
fn do_while_loop() {
    let code = r#"
my $line;
do {
    $line = <STDIN>;
    chomp $line;
} while defined $line;
"#;
    assert_clean_parse(code);
}

#[test]
fn do_until_loop() {
    let code = r#"
my $count = 0;
do {
    $count++;
} until ($count >= 10);
"#;
    assert_clean_parse(code);
}

// ===========================================================================
// 11. Postfix for/foreach
// ===========================================================================

#[test]
fn postfix_for_print() {
    let code = r#"
print "$_\n" for @items;
"#;
    assert_clean_parse(code);
}

#[test]
fn postfix_foreach_method() {
    let code = r#"
$obj->process($_) foreach @tasks;
"#;
    assert_clean_parse(code);
}

// ===========================================================================
// 12. Complex dereferencing chains
// ===========================================================================

#[test]
fn complex_deref_chain() {
    let code = r#"
my $result = $hash{key}[0]->method()->{result};
"#;
    assert_clean_parse(code);
}

#[test]
fn complex_deref_nested_arrow() {
    let code = r#"
my $val = $data->{users}->[0]->{name};
"#;
    assert_clean_parse(code);
}

#[test]
fn complex_deref_method_chain() {
    let code = r#"
my $val = $obj->get_list()->[0]->name();
"#;
    assert_clean_parse(code);
}

// ===========================================================================
// 13. BEGIN / CHECK / INIT / UNITCHECK / END blocks
// ===========================================================================

#[test]
fn begin_block() {
    let code = r#"
BEGIN {
    push @INC, '/opt/lib';
}
"#;
    assert_clean_parse(code);
}

#[test]
fn end_block() {
    let code = r#"
END {
    print "Cleaning up\n";
}
"#;
    assert_clean_parse(code);
}

#[test]
fn check_block() {
    let code = r#"
CHECK {
    print "Check phase\n";
}
"#;
    assert_clean_parse(code);
}

#[test]
fn init_block() {
    let code = r#"
INIT {
    print "Init phase\n";
}
"#;
    assert_clean_parse(code);
}

#[test]
fn unitcheck_block() {
    let code = r#"
UNITCHECK {
    print "Unit check phase\n";
}
"#;
    assert_clean_parse(code);
}

#[test]
fn multiple_phase_blocks() {
    let code = r#"
BEGIN { $started = 1; }
CHECK { $checked = 1; }
INIT { $initialized = 1; }
END { $cleaned = 1; }
"#;
    assert_clean_parse(code);
}
