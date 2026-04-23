//! Tests for issue #2399: expected_left_brace in sub forward declarations,
//! string eval, and do $file forms.
//!
//! Three valid Perl forms that previously triggered the expected_left_brace bucket:
//! 1. `sub foo;` — forward declaration without body
//! 2. `eval $expr;` — string/expression eval
//! 3. `do $file;` — file do (not block)

mod cpan_test_helpers;
use cpan_test_helpers::*;

// ---- Sub forward declarations (sub NAME;) ----

#[test]
fn test_sub_forward_decl_simple() {
    assert_clean_parse("sub foo;");
}

#[test]
fn test_sub_forward_decl_with_prototype() {
    assert_clean_parse("sub foo ($);");
}

#[test]
fn test_sub_forward_decl_with_multiarg_prototype() {
    assert_clean_parse("sub foo ($$);");
}

#[test]
fn test_sub_forward_decl_with_attribute() {
    assert_clean_parse("sub foo :method;");
}

#[test]
fn test_sub_forward_decl_with_lvalue_attr() {
    assert_clean_parse("sub foo :lvalue;");
}

#[test]
fn test_sub_forward_decl_multiple_in_class() {
    // From Moose::Meta::Class pattern: multiple forward declarations in a package
    assert_clean_parse(
        r#"
package Moose::Meta::Class;
use Moose;

sub add_method;
sub remove_method;
sub get_method;
sub get_all_methods;
sub find_method_by_name;
sub new_object;
sub clone_object;
sub rebless_instance;

1;
"#,
    );
}

#[test]
fn test_sub_forward_decl_after_use() {
    assert_clean_parse(
        r#"
package MyModule;
use strict;
use warnings;

sub BUILD;
sub DEMOLISH;
sub BUILDARGS;

1;
"#,
    );
}

#[test]
fn test_sub_forward_decl_mixed_with_definitions() {
    // Mix of forward declarations and full definitions
    assert_clean_parse(
        r#"
sub foo;
sub bar { 1 }
sub baz;
sub qux { return "hello" }
"#,
    );
}

// ---- String eval (eval $expr, eval $var) ----

#[test]
fn test_eval_scalar_var() {
    // eval $variable — string eval of a scalar
    assert_clean_parse("eval $code;");
}

#[test]
fn test_eval_scalar_var_assignment() {
    // $result = eval $expr — common pattern
    assert_clean_parse("my $result = eval $code;");
}

#[test]
fn test_eval_scalar_or_die() {
    // eval $code or die $@ — common error handling
    assert_clean_parse("eval $code or die $@;");
}

#[test]
fn test_eval_scalar_with_die_check() {
    assert_clean_parse(
        r#"
eval $code;
die $@ if $@;
"#,
    );
}

#[test]
fn test_eval_scalar_in_conditional() {
    assert_clean_parse(
        r#"
if (eval $code) {
    print "ok\n";
}
"#,
    );
}

#[test]
fn test_eval_string_expr() {
    // eval "string" — the original string eval form
    assert_clean_parse(r#"eval "require $module";"#);
}

#[test]
fn test_eval_with_method_result() {
    // eval $obj->get_code() — eval of method call result
    assert_clean_parse("eval $obj->get_code();");
}

#[test]
fn test_eval_mixed_block_and_string() {
    // Both block and string eval forms in same file
    assert_clean_parse(
        r#"
eval { require Foo; 1 } or die $@;
eval $runtime_code;
my $ok = eval { 1 };
eval $plugin_code or die $@;
"#,
    );
}

#[test]
fn test_eval_nested_string_in_block() {
    // String eval inside block eval
    assert_clean_parse(
        r#"
eval {
    eval $inner_code;
};
"#,
    );
}

// ---- do $file (file do, not block do) ----

#[test]
fn test_do_scalar_file() {
    // do $file — execute a file
    assert_clean_parse("do $file;");
}

#[test]
fn test_do_file_with_assignment() {
    // my %config = do $file — common config loading pattern
    assert_clean_parse("my %config = do $config_file;");
}

#[test]
fn test_do_file_or_die() {
    // do $file or die — error handling
    assert_clean_parse(r#"do $file or die "cannot load: $!";"#);
}

#[test]
fn test_do_file_conditional() {
    // do $file if condition
    assert_clean_parse("do $local_config if -f $local_config;");
}

#[test]
fn test_do_file_hash_deref() {
    // my %h = %{ do $file } — hash config
    assert_clean_parse("my %config = %{ do $config_file };");
}

#[test]
fn test_do_file_in_sub() {
    // do $file inside a sub
    assert_clean_parse(
        r#"
sub load_config {
    my $file = shift;
    do $file;
}
"#,
    );
}

#[test]
fn test_do_mixed_file_and_block() {
    // Both do $file and do { } in same file
    assert_clean_parse(
        r#"
do $config_file;
my $result = do { my $x = 1; $x * 2 };
do $extra_config if -f $extra_config;
do { cleanup() } while $running;
"#,
    );
}

// ---- Catalyst-style eval $string_expr patterns ----

#[test]
fn test_catalyst_eval_plugin_load() {
    // Pattern from Catalyst.pm: eval "require $plugin"
    assert_clean_parse(
        r#"
my $plugin = "Catalyst::Plugin::$name";
eval "require $plugin";
"#,
    );
}

#[test]
fn test_catalyst_eval_code_var() {
    assert_clean_parse(
        r#"
my $code = 'use strict; use warnings;';
eval $code;
"#,
    );
}

// ---- DBIx::Class::Schema-style do $file patterns ----

#[test]
fn test_dbix_do_schema_file() {
    // Pattern from DBIx-Class/Schema.pm
    assert_clean_parse(
        r#"
my $schema_file = $self->_schema_file;
do $schema_file;
"#,
    );
}

#[test]
fn test_dbix_do_config_with_error_check() {
    assert_clean_parse(
        r#"
my $config = do $config_file;
die "Failed to load config: $!" unless defined $config;
"#,
    );
}
