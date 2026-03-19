mod cpan_test_helpers;
use cpan_test_helpers::*;

// =============================================================================
// Ternary operator with complex middle expressions
// =============================================================================

#[test]
fn test_ternary_basic() {
    assert_clean_parse(r#"my $x = $a ? "yes" : "no";"#);
}

#[test]
fn test_ternary_with_regex_in_condition() {
    assert_clean_parse(r#"my $x = $str =~ /foo/ ? "yes" : "no";"#);
}

#[test]
fn test_ternary_with_function_call_in_then() {
    assert_clean_parse(r#"my $x = $a ? foo($b, $c) : bar();"#);
}

#[test]
fn test_ternary_with_hash_ref_in_then() {
    assert_clean_parse(r#"my $x = $a ? { key => "value" } : {};"#);
}

#[test]
fn test_ternary_with_array_ref_in_then() {
    assert_clean_parse(r#"my $x = $a ? [1, 2, 3] : [];"#);
}

#[test]
fn test_ternary_nested() {
    assert_clean_parse(r#"my $x = $a ? $b ? 1 : 2 : 3;"#);
}

#[test]
fn test_ternary_chained_else() {
    assert_clean_parse(r#"my $x = $a ? 1 : $b ? 2 : 3;"#);
}

#[test]
fn test_ternary_with_string_concat() {
    assert_clean_parse(r#"my $x = $a ? "hello " . $name : "bye";"#);
}

#[test]
fn test_ternary_with_method_call() {
    assert_clean_parse(r#"my $x = $a ? $obj->method() : $obj->other();"#);
}

#[test]
fn test_ternary_with_deref_in_then() {
    assert_clean_parse(r#"my $x = $a ? $hash->{key} : $hash->{other};"#);
}

// =============================================================================
// Ternary with regex containing colons
// =============================================================================

#[test]
fn test_ternary_with_regex_colon_in_condition() {
    assert_clean_parse(r#"my $x = $str =~ /foo:bar/ ? 1 : 0;"#);
}

#[test]
fn test_ternary_with_regex_in_then_branch() {
    // Regex match inside then branch
    assert_clean_parse(r#"my $x = $a ? $str =~ s/foo/bar/ : $str;"#);
}

// =============================================================================
// Ternary with do blocks
// =============================================================================

#[test]
fn test_ternary_with_do_block() {
    assert_clean_parse(r#"my $x = $a ? do { my $t = 1; $t + 2 } : 0;"#);
}

// =============================================================================
// Ternary with complex arithmetic
// =============================================================================

#[test]
fn test_ternary_with_arithmetic_in_then() {
    assert_clean_parse(r#"my $x = $a ? $b + $c * 2 : $d - 1;"#);
}

// =============================================================================
// Sub attributes (sub foo : lvalue { })
// =============================================================================

#[test]
fn test_sub_with_lvalue_attribute() {
    assert_clean_parse(r#"sub foo : lvalue { return $_[0] }"#);
}

#[test]
fn test_sub_with_method_attribute() {
    assert_clean_parse(r#"sub foo : method { return $_[0] }"#);
}

#[test]
fn test_sub_with_multiple_attributes() {
    assert_clean_parse(r#"sub foo : lvalue : method { return $_[0] }"#);
}

#[test]
fn test_sub_with_attribute_value() {
    assert_clean_parse(r#"sub foo : Attr(value) { return 1 }"#);
}

// =============================================================================
// Labels
// =============================================================================

#[test]
fn test_label_while() {
    assert_clean_parse(r#"OUTER: while (1) { last OUTER; }"#);
}

#[test]
fn test_label_for() {
    assert_clean_parse(r#"LOOP: for my $i (1..10) { next LOOP if $i == 5; }"#);
}

#[test]
fn test_label_foreach() {
    assert_clean_parse(r#"LINE: foreach my $line (@lines) { next LINE if $line =~ /^#/; }"#);
}

// =============================================================================
// Hash slices
// =============================================================================

#[test]
fn test_hash_slice() {
    assert_clean_parse(r#"my @vals = @hash{qw(a b c)};"#);
}

#[test]
fn test_hash_slice_with_variables() {
    assert_clean_parse(r#"my @vals = @hash{@keys};"#);
}

// =============================================================================
// Ternary with comma expressions and lists
// =============================================================================

#[test]
fn test_ternary_in_function_arg() {
    assert_clean_parse(r#"print($a ? "yes" : "no");"#);
}

#[test]
fn test_ternary_in_array_element() {
    assert_clean_parse(r#"my @arr = ($a ? 1 : 0, $b ? 2 : 3);"#);
}

#[test]
fn test_ternary_with_qw() {
    assert_clean_parse(r#"my @x = $a ? qw(foo bar) : qw(baz);"#);
}

// =============================================================================
// Ternary with word operators
// =============================================================================

#[test]
fn test_ternary_with_or_in_condition() {
    assert_clean_parse(r#"my $x = ($a || $b) ? 1 : 0;"#);
}

#[test]
fn test_ternary_result_with_or() {
    assert_clean_parse(r#"my $x = $a ? $b || $c : $d;"#);
}

// =============================================================================
// Complex CPAN-like patterns
// =============================================================================

#[test]
fn test_ternary_with_ref_check() {
    assert_clean_parse(r#"my $x = ref($thing) eq 'HASH' ? $thing->{key} : $thing;"#);
}

#[test]
fn test_ternary_with_defined_check() {
    assert_clean_parse(r#"my $x = defined($val) ? $val : "default";"#);
}

#[test]
fn test_ternary_with_string_comparison_and_method() {
    assert_clean_parse(
        r#"my $x = $type eq 'file' ? $self->read_file($path) : $self->read_dir($path);"#,
    );
}

#[test]
fn test_ternary_multiline() {
    assert_clean_parse(
        r#"my $result = $condition
    ? $then_value
    : $else_value;"#,
    );
}

#[test]
fn test_ternary_with_sprintf() {
    assert_clean_parse(
        r#"my $msg = $count == 1 ? sprintf("1 item") : sprintf("%d items", $count);"#,
    );
}

#[test]
fn test_ternary_with_die_in_else() {
    assert_clean_parse(r#"my $x = $ok ? $val : die "error";"#);
}

#[test]
fn test_ternary_assignment_in_then() {
    // Assignment in then branch (between ? and :) should work
    assert_clean_parse(r#"$a ? $b = 1 : $c = 2;"#);
}

// =============================================================================
// Sub attributes with prototypes
// =============================================================================

#[test]
fn test_sub_prototype_and_attribute() {
    assert_clean_parse(r#"sub foo ($) : lvalue { $_[0] }"#);
}

// =============================================================================
// Ternary with anonymous sub
// =============================================================================

#[test]
fn test_ternary_with_anon_sub_in_then() {
    assert_clean_parse(r#"my $cb = $a ? sub { 1 } : sub { 2 };"#);
}

// =============================================================================
// Ternary with string interpolation containing colons
// =============================================================================

#[test]
fn test_ternary_with_interpolated_string() {
    assert_clean_parse(r#"my $x = $a ? "value: $b" : "none";"#);
}

// =============================================================================
// Ternary with wantarray
// =============================================================================

#[test]
fn test_ternary_with_wantarray() {
    assert_clean_parse(r#"return wantarray ? @list : $scalar;"#);
}

// =============================================================================
// Ternary with complex derefs
// =============================================================================

#[test]
fn test_ternary_with_nested_deref() {
    assert_clean_parse(r#"my $x = $a ? $hash->{$key}->[0] : undef;"#);
}

#[test]
fn test_ternary_with_array_deref_slice() {
    assert_clean_parse(r#"my @x = $a ? @{$ref} : ();"#);
}

// =============================================================================
// Anonymous sub with attributes
// =============================================================================

#[test]
fn test_anon_sub_with_attribute() {
    assert_clean_parse(r#"my $cb = sub : lvalue { $_[0] };"#);
}

#[test]
fn test_anon_sub_with_method_attribute() {
    assert_clean_parse(r#"my $cb = sub : method { $_[0] };"#);
}

// =============================================================================
// Ternary with heredoc
// =============================================================================

#[test]
fn test_ternary_with_heredoc() {
    assert_clean_parse(
        r#"my $x = $a ? <<END : "default";
hello world
END
"#,
    );
}

// =============================================================================
// Complex ternary patterns from CPAN
// =============================================================================

#[test]
fn test_ternary_with_local() {
    assert_clean_parse(r#"local $_ = $a ? $b : $c;"#);
}

#[test]
fn test_ternary_with_join() {
    assert_clean_parse(r#"my $x = @arr ? join(", ", @arr) : "none";"#);
}

#[test]
fn test_ternary_with_map() {
    assert_clean_parse(r#"my @x = $a ? map { $_->{name} } @list : ();"#);
}

#[test]
fn test_ternary_with_grep() {
    assert_clean_parse(r#"my @x = $a ? grep { defined } @list : ();"#);
}

#[test]
fn test_ternary_with_chomp() {
    assert_clean_parse(r#"chomp(my $line = $a ? $b : $c);"#);
}

#[test]
fn test_ternary_with_chained_method() {
    assert_clean_parse(r#"my $x = $a ? $obj->foo->bar->baz : undef;"#);
}

#[test]
fn test_ternary_with_exists() {
    assert_clean_parse(r#"my $x = exists $hash{$key} ? $hash{$key} : "default";"#);
}

#[test]
fn test_ternary_with_delete() {
    assert_clean_parse(r#"my $x = $a ? delete $hash{$key} : undef;"#);
}

// =============================================================================
// Ternary with complex condition using regex
// =============================================================================

#[test]
fn test_ternary_with_negated_regex() {
    assert_clean_parse(r#"my $x = $str !~ /pattern/ ? "no match" : "match";"#);
}

#[test]
fn test_ternary_with_substitution_in_condition() {
    assert_clean_parse(r#"my $x = ($str =~ s/foo/bar/) ? "replaced" : "not found";"#);
}

// =============================================================================
// Ternary where condition/then/else are complex expressions
// =============================================================================

#[test]
fn test_ternary_complex_condition_with_and() {
    assert_clean_parse(r#"my $x = ($a && $b) ? "both" : "nope";"#);
}

#[test]
fn test_ternary_with_list_assignment() {
    assert_clean_parse(r#"my ($x, $y) = $a ? (1, 2) : (3, 4);"#);
}

#[test]
fn test_ternary_with_print() {
    assert_clean_parse(r#"print $a ? "yes\n" : "no\n";"#);
}

#[test]
fn test_ternary_void_context() {
    assert_clean_parse(r#"$a ? foo() : bar();"#);
}

// =============================================================================
// Ternary with string/number ops
// =============================================================================

#[test]
fn test_ternary_with_string_eq() {
    assert_clean_parse(r#"my $x = $a eq "yes" ? 1 : 0;"#);
}

#[test]
fn test_ternary_with_numeric_comparison() {
    assert_clean_parse(r#"my $x = $a >= 10 ? "big" : "small";"#);
}

// =============================================================================
// Ternary inside complex expressions
// =============================================================================

#[test]
fn test_ternary_inside_hash_value() {
    assert_clean_parse(r#"my %h = (key => $a ? "yes" : "no");"#);
}

#[test]
fn test_ternary_inside_array_init() {
    assert_clean_parse(r#"my @a = ($a ? 1 : 0);"#);
}

#[test]
fn test_ternary_as_hash_value_in_constructor() {
    assert_clean_parse(r#"my $obj = Foo->new(bar => $a ? 1 : 0, baz => 3);"#);
}

// =============================================================================
// Ternary with blessed/ref/Scalar::Util patterns
// =============================================================================

#[test]
fn test_ternary_with_blessed() {
    assert_clean_parse(r#"my $x = blessed($obj) ? $obj->name : "not an object";"#);
}

#[test]
fn test_ternary_with_scalar_ref() {
    assert_clean_parse(r#"my $x = ref($thing) ? $$thing : $thing;"#);
}

// =============================================================================
// Ternary in print/say/warn/die context
// =============================================================================

#[test]
fn test_ternary_in_warn() {
    assert_clean_parse(r#"warn $debug ? "debug: $msg\n" : "error: $msg\n";"#);
}

#[test]
fn test_ternary_in_die() {
    assert_clean_parse(r#"die $recoverable ? "warning" : "fatal error";"#);
}

// =============================================================================
// Ternary with postfix deref (->@*, ->%*, etc.)
// =============================================================================

#[test]
fn test_ternary_with_postfix_array_deref() {
    assert_clean_parse(r#"my @x = $a ? $ref->@* : ();"#);
}

#[test]
fn test_ternary_with_postfix_hash_deref() {
    assert_clean_parse(r#"my %h = $a ? $ref->%* : ();"#);
}

// =============================================================================
// Hash slice and array slice with ternary
// =============================================================================

#[test]
fn test_hash_slice_in_ternary() {
    assert_clean_parse(r#"my @vals = $a ? @hash{@keys} : ();"#);
}

#[test]
fn test_array_slice_in_ternary() {
    assert_clean_parse(r#"my @vals = $a ? @arr[0..2] : ();"#);
}

// =============================================================================
// Tricky Perl patterns with colons that might confuse the parser
// =============================================================================

#[test]
fn test_package_with_version() {
    assert_clean_parse(r#"package Foo::Bar 1.00;"#);
}

#[test]
fn test_ternary_with_package_name_in_condition() {
    assert_clean_parse(r#"my $x = Foo::Bar->can("method") ? 1 : 0;"#);
}

#[test]
fn test_ternary_with_package_method_call() {
    assert_clean_parse(r#"my $x = $a ? Foo::Bar->new() : Baz::Qux->new();"#);
}

#[test]
fn test_ternary_with_package_constant() {
    assert_clean_parse(r#"my $x = $a ? Foo::BAR : Baz::QUX;"#);
}

// =============================================================================
// Ternary with hash/array ref constructors that use colons
// =============================================================================

#[test]
fn test_ternary_with_dbi_dsn_string() {
    assert_clean_parse(r#"my $dsn = $use_pg ? "dbi:Pg:dbname=test" : "dbi:SQLite:test.db";"#);
}

// =============================================================================
// Ternary with complex regex
// =============================================================================

#[test]
fn test_ternary_with_character_class_regex() {
    assert_clean_parse(r#"my $x = $str =~ /[a-z:]/ ? 1 : 0;"#);
}

#[test]
fn test_ternary_with_complex_regex_modifiers() {
    assert_clean_parse(r#"my $x = $str =~ /pattern/gi ? "yes" : "no";"#);
}

// =============================================================================
// Ternary with complex hash access patterns
// =============================================================================

#[test]
fn test_ternary_with_nested_hash_access() {
    assert_clean_parse(r#"my $x = $a ? $h->{b}{c}{d} : undef;"#);
}

#[test]
fn test_ternary_with_hash_element_ref() {
    assert_clean_parse(r#"my $x = $a ? \$hash{key} : \$other{key};"#);
}

// =============================================================================
// Ternary inside various statement types
// =============================================================================

#[test]
fn test_ternary_in_for_init() {
    assert_clean_parse(r#"for my $i ($a ? @list1 : @list2) { print $i; }"#);
}

#[test]
fn test_ternary_in_while_condition() {
    assert_clean_parse(r#"while ($x = $a ? shift @list : undef) { print $x; }"#);
}

#[test]
fn test_ternary_in_return() {
    assert_clean_parse(r#"sub foo { return $a ? 1 : 0; }"#);
}

// =============================================================================
// Ternary with eval
// =============================================================================

#[test]
fn test_ternary_with_eval_block() {
    assert_clean_parse(r#"my $x = $a ? eval { $obj->method() } : undef;"#);
}

// =============================================================================
// Ternary with complex string operations
// =============================================================================

#[test]
fn test_ternary_with_substr() {
    assert_clean_parse(r#"my $x = length($s) > 10 ? substr($s, 0, 10) . "..." : $s;"#);
}

#[test]
fn test_ternary_with_sprintf_complex() {
    assert_clean_parse(
        r#"my $msg = $n == 1 ? sprintf("Got %d item", $n) : sprintf("Got %d items", $n);"#,
    );
}

// =============================================================================
// Ternary with complex lvalue
// =============================================================================

#[test]
fn test_ternary_lvalue() {
    assert_clean_parse(r#"($a ? $x : $y) = 42;"#);
}

// =============================================================================
// Ternary with ternary in condition
// =============================================================================

#[test]
fn test_deeply_nested_ternary() {
    assert_clean_parse(r#"my $x = ($a ? $b : $c) ? ($d ? $e : $f) : ($g ? $h : $i);"#);
}

// =============================================================================
// Attributes on my variables (rare but valid Perl)
// =============================================================================

#[test]
fn test_my_with_attribute() {
    // my $x : shared;  -- valid in threaded Perl
    assert_clean_parse(r#"my $x : shared;"#);
}

#[test]
fn test_my_array_with_attribute() {
    assert_clean_parse(r#"my @x : shared;"#);
}

#[test]
fn test_our_with_attribute() {
    assert_clean_parse(r#"our $x : shared = 42;"#);
}

// =============================================================================
// Format with colon-like patterns
// =============================================================================

#[test]
fn test_ternary_in_printf_format() {
    assert_clean_parse(r#"printf "%s: %s\n", $key, $a ? "yes" : "no";"#);
}

// =============================================================================
// Chained ternary with assignment
// =============================================================================

#[test]
fn test_ternary_chain_with_assignments() {
    assert_clean_parse(
        r#"my $x = $type eq 'a' ? 1
       : $type eq 'b' ? 2
       : $type eq 'c' ? 3
       : 0;"#,
    );
}

// =============================================================================
// Ternary with complex boolean expressions
// =============================================================================

#[test]
fn test_ternary_with_complex_boolean() {
    assert_clean_parse(r#"my $x = ($a && $b || $c) ? "yes" : "no";"#);
}

#[test]
fn test_ternary_with_not_operator() {
    assert_clean_parse(r#"my $x = !$a ? "false" : "true";"#);
}

// =============================================================================
// Ternary with local() and complex expressions
// =============================================================================

#[test]
fn test_local_ternary() {
    assert_clean_parse(r#"local $/ = $binary ? undef : "\n";"#);
}

// =============================================================================
// Ternary with array/hash operations
// =============================================================================

#[test]
fn test_ternary_with_push() {
    assert_clean_parse(r#"push @list, $a ? "yes" : "no";"#);
}

#[test]
fn test_ternary_with_unshift() {
    assert_clean_parse(r#"unshift @list, $a ? "first" : "other";"#);
}

#[test]
fn test_ternary_with_splice() {
    assert_clean_parse(r#"splice @list, 0, 0, $a ? @extra : ();"#);
}

// =============================================================================
// Ternary with complex deref chains
// =============================================================================

#[test]
fn test_ternary_with_complex_deref_chain() {
    assert_clean_parse(r#"my $x = $a ? $self->{config}{debug} : 0;"#);
}

#[test]
fn test_ternary_with_arrayref_deref() {
    assert_clean_parse(r#"my $x = $a ? $arr->[0]->{name} : "default";"#);
}

// =============================================================================
// The core failing pattern: user-defined function calls without parens in ternary
// =============================================================================

#[test]
fn test_ternary_with_user_func_no_parens_in_then() {
    // This is the pattern that causes expected_colon:
    // The parser sees `camelize` as a bare identifier, not a function call.
    // It doesn't consume `$name` as the argument, so `:` is not the next token.
    assert_clean_parse(r#"my $suffix = $name =~ /^[a-z]/ ? camelize $name : $name;"#);
}

#[test]
fn test_ternary_with_user_func_no_parens_croak() {
    // croak is from Carp but treated as user function
    assert_clean_parse(r#"my $val = defined $err ? croak $err : $value;"#);
}

#[test]
fn test_ternary_with_user_func_encode() {
    assert_clean_parse(r#"sub _maybe { $_[0] ? encode @_ : $_[1] }"#);
}

#[test]
fn test_ternary_with_user_func_decode() {
    assert_clean_parse(r#"my $msg = $op == 1 ? decode 'UTF-8', $msg : $msg;"#);
}

#[test]
fn test_ternary_with_user_func_quote() {
    assert_clean_parse(r#"my $str = $value =~ /[,;" ]/ ? quote $value : $value;"#);
}

#[test]
fn test_ternary_with_user_func_camelize() {
    assert_clean_parse(r#"my $suffix = $name =~ /^[a-z]/ ? camelize $name : $name;"#);
}

#[test]
fn test_ternary_with_user_func_md5_sum() {
    assert_clean_parse(r#"my $name = defined $inline ? md5_sum encode('UTF-8', $inline) : undef;"#);
}

#[test]
fn test_ternary_with_user_func_b64_decode() {
    assert_clean_parse(r#"my $val = $_[0] && $_[0] =~ /Basic (.+)$/ ? b64_decode $1 : undef;"#);
}

#[test]
fn test_ternary_with_user_func_punycode_decode() {
    assert_clean_parse(r#"my $host = /^xn--(.+)$/ ? punycode_decode $1 : $_;"#);
}

#[test]
fn test_ternary_with_user_func_html_attr_unescape() {
    assert_clean_parse(r#"$attrs{$key} = defined $value ? html_attr_unescape $value : $value;"#);
}

#[test]
fn test_ternary_with_user_func_catfile() {
    assert_clean_parse(r#"my $value = @_ == 1 ? $_[0] : @_ > 1 ? catfile @_ : canonpath getcwd;"#);
}

#[test]
fn test_ternary_with_continue_method() {
    // continue is a keyword but used as method call here
    assert_clean_parse(r#"@{$c->match->stack} ? $self->continue($c) : return undef;"#);
}

#[test]
fn test_ternary_with_core_die() {
    assert_clean_parse(r#"!ref $exception ? CORE::die $exception : Carp::croak $exception;"#);
}

// =============================================================================
// Full function context tests (extracted from CPAN files)
// =============================================================================

#[test]
fn test_mojo_plugins_load_plugin() {
    let source = r#"
sub load_plugin {
  my ($self, $name) = @_;
  my $suffix  = $name =~ /^[a-z]/ ? camelize $name : $name;
  my @classes = map {"${_}::$suffix"} @{$self->namespaces};
  for my $class (@classes, $name) { return $class->new if _load($class) }
  die qq{Plugin "$name" missing, maybe you need to install it?\n};
}
"#;
    assert_clean_parse(source);
}

#[test]
fn test_mojo_json_decode() {
    let source = r#"
sub decode_json {
  my $err = _decode(\my $value, shift);
  return defined $err ? croak $err : $value;
}
"#;
    assert_clean_parse(source);
}

#[test]
fn test_mojo_util_deprecated() {
    let source = r#"
sub deprecated {
  local $Carp::CarpLevel = 1;
  $ENV{MOJO_FATAL_DEPRECATIONS} ? croak @_ : carp @_;
}
"#;
    assert_clean_parse(source);
}

#[test]
fn test_mojo_dom_splice() {
    let source = r#"
sub _splice {
  my ($self, $start, $offset) = @_;
  my $tree = $self->tree;
  $start  = $start  ? ($#$tree + 1) : _start($tree);
  $offset = $offset ? $#$tree       : 0;
  splice @$tree, $start, $offset;
}
"#;
    assert_clean_parse(source);
}

#[test]
fn test_mojo_renderer_maybe() {
    assert_clean_parse(r#"sub _maybe { $_[0] ? encode @_ : $_[1] }"#);
}

#[test]
fn test_mojo_cookie_quote() {
    let source = r#"
sub to_string {
  my $self  = shift;
  my $name  = $self->name // '';
  my $value = $self->value // '';
  return join '=', $name, $value =~ /[,;" ]/ ? quote $value : $value;
}
"#;
    assert_clean_parse(source);
}

#[test]
fn test_mojo_file_new() {
    let source = r#"
sub new {
  my $class = shift;
  croak 'Invalid path' if grep { !defined } @_;
  my $value = @_ == 1 ? $_[0] : @_ > 1 ? catfile @_ : canonpath getcwd;
  return bless \$value, ref $class || $class;
}
"#;
    assert_clean_parse(source);
}

#[test]
fn test_mojo_log_format() {
    let source = r#"
sub _format {
  my ($self, $level) = (shift, pop);
  my @msgs = ref $_[0] eq 'CODE' ? $_[0]() : @_;
  unshift @msgs, @{$self->{context}} if $self->{context};
  return @msgs;
}
"#;
    assert_clean_parse(source);
}

#[test]
fn test_exporter_heavy_ternary() {
    let source = r#"
$type eq '&' ? \&{"${pkg}::$sym"} :
$type eq '$' ? \${"${pkg}::$sym"} :
$type eq '@' ? \@{"${pkg}::$sym"} :
*{"${pkg}::$sym"};
"#;
    assert_clean_parse(source);
}

#[test]
fn test_graph_adjacencymap_map_ternary() {
    let source = r#"
my @result =
    ($arity == 0 && !($f & _UNORD))
        ? map [$_, join '|', map "@$_", @$_], @p
        : map [$_,"@$_"], @p;
"#;
    assert_clean_parse(source);
}

#[test]
fn test_mojo_server_load_app() {
    let source = r#"
sub load_app {
  my ($self, $path, @args) = (shift, shift, ref $_[0] ? %{shift()} : @_);
  return $self;
}
"#;
    assert_clean_parse(source);
}

#[test]
fn test_b64_decode_ternary() {
    assert_clean_parse(r#"sub _basic { $_[0] && $_[0] =~ /Basic (.+)$/ ? b64_decode $1 : undef }"#);
}

#[test]
fn test_mojo_url_punycode() {
    let source = r#"
return $self->host(join '.', map { /^xn--(.+)$/ ? punycode_decode $1 : $_ } split(/\./, shift, -1)) if @_;
"#;
    assert_clean_parse(source);
}

#[test]
fn test_mojo_loader_b64() {
    let source = r#"
$all->{$name} = $name =~ s/\s*\(\s*base64\s*\)$// && ++$BIN{$class}{$name} ? b64_decode $data : $data;
"#;
    assert_clean_parse(source);
}

#[test]
fn test_mojo_websocket_decode() {
    let source = r#"
$self->emit(message => $op == 1 ? decode 'UTF-8', $msg : $msg) if $self->has_subscribers('message');
"#;
    assert_clean_parse(source);
}

#[test]
fn test_future_pp_die() {
    let source = r#"
!ref $exception && $exception =~ m/\n$/ ? CORE::die $exception : Carp::croak $exception;
"#;
    assert_clean_parse(source);
}

#[test]
fn test_chi_driver_slice_grep() {
    let source = r#"
my @opts = ( ref($options) eq 'HASH' )
    ? slice_grep { /(?:expire_if|busy_lock)/ } $options
    : ();
"#;
    assert_clean_parse(source);
}

#[test]
fn test_mojo_routes_continue() {
    let source = r#"
sub dispatch {
  my ($self, $c) = @_;
  $self->match($c);
  @{$c->match->stack} ? $self->continue($c) : return undef;
  return 1;
}
"#;
    assert_clean_parse(source);
}

#[test]
fn test_mojo_parameters_pairs() {
    let source = r#"
sub merge {
  my $self = shift;
  my $old = $self->pairs;
  my @new = @_ == 1 ? @{shift->pairs} : @_;
  return $self;
}
"#;
    assert_clean_parse(source);
}

// =============================================================================
// Full file parse tests (to detect cascading errors)
// =============================================================================

// =============================================================================
// ACTUAL FAILING PATTERNS from CPAN corpus scan
// These patterns cause expected_colon as the FIRST error in their file.
// =============================================================================

#[test]
fn test_cpan_shift_method_deref_in_ternary() {
    // From Mojo/Parameters.pm: @{shift->pairs}
    assert_clean_parse(r#"my @new = @_ == 1 ? @{shift->pairs} : @_;"#);
}

#[test]
fn test_cpan_shift_parens_hash_deref_in_ternary() {
    // From URI/_query.pm: %{shift(@_)}
    assert_clean_parse(r#"$self->query_form(@_ == 1 ? %{shift(@_)} : @_);"#);
}

#[test]
fn test_cpan_shift_parens_array_deref_in_ternary() {
    // From Mouse/Exporter.pm: @{shift(@args)}
    assert_clean_parse(r#"push @traits, ref($args[0]) ? @{shift(@args)} : shift(@args);"#);
}

#[test]
fn test_cpan_shift_parens_hash_deref_in_ternary_2() {
    // From Test/Mojo.pm: %{shift()}
    assert_clean_parse(r#"my @cfg = @_ ? {config => {config_override => 1, %{shift()}}} : ();"#);
}

#[test]
fn test_cpan_ref_shift_hash_deref_in_ternary() {
    // From Pod/Checker.pm: %{shift()}
    assert_clean_parse(r#"my %opts = (ref $_[0]) ? %{shift()} : ();"#);
}

#[test]
fn test_cpan_decode_with_string_arg_in_ternary() {
    // From Mojo/Transaction/WebSocket.pm: decode 'UTF-8', $msg
    assert_clean_parse(r#"$self->emit(message => $op == 1 ? decode 'UTF-8', $msg : $msg);"#);
}

#[test]
fn test_cpan_punycode_decode_in_map_ternary() {
    // From Mojo/URL.pm: punycode_decode $1
    assert_clean_parse(
        r#"return $self->host(join '.', map { /^xn--(.+)$/ ? punycode_decode $1 : $_ } split(/\./, shift, -1));"#,
    );
}

#[test]
fn test_cpan_md5_sum_encode_in_ternary() {
    // From Mojolicious/Plugin/EPLRenderer.pm: md5_sum encode(...)
    assert_clean_parse(r#"my $name = defined $inline ? md5_sum encode('UTF-8', $inline) : undef;"#);
}

#[test]
fn test_cpan_continue_as_method_in_ternary() {
    // From Mojolicious/Routes.pm: $self->continue($c) — continue is a keyword
    assert_clean_parse(r#"@{$c->match->stack} ? $self->continue($c) : return undef;"#);
}

#[test]
fn test_cpan_undef_parens_in_ternary() {
    // From POE — undef() with explicit parens
    assert_clean_parse(r#"StderrEvent => ($conduit eq 'pty' ? undef() : 'stderr');"#);
}

#[test]
fn test_cpan_not_as_method_in_ternary() {
    // From Test/PDL.pm: $mask->not->whichND — not is a keyword
    assert_clean_parse(r#"my $coords = defined $mask ? $mask->not->whichND : undef;"#);
}

#[test]
fn test_cpan_string_x_operator_in_ternary() {
    // From Math/BigInt/Calc.pm: "..." x int(...)
    assert_clean_parse(
        r#"$format .= $] < 5.008 ? "a$BASE_LEN" x int($input_len / $BASE_LEN) : "(a$BASE_LEN)*";"#,
    );
}

#[test]
fn test_cpan_slice_grep_block_in_ternary() {
    // From CHI/Driver.pm: slice_grep { ... } $options
    assert_clean_parse(
        r#"my @opts = (ref($options) eq 'HASH') ? slice_grep { /(?:expire_if|busy_lock)/ } $options : ();"#,
    );
}

#[test]
fn test_cpan_map_complex_in_ternary() {
    // From Graph/AdjacencyMap.pm: map [...], @p
    assert_clean_parse(
        r#"my @r = ($arity == 0 && !($f & _UNORD)) ? map [$_, join '|', map "@$_", @$_], @p : map [$_,"@$_"], @p;"#,
    );
}

#[test]
fn test_cpan_pairvalues_in_ternary() {
    // From Params/ValidationCompiler/Compiler.pm: pairvalues @{...}
    assert_clean_parse(
        r#"my @specs = $p{named_to_list} ? pairvalues @{ $p{params} } : @{ $p{params} };"#,
    );
}

#[test]
fn test_cpan_coderef_call_in_ternary() {
    // From IO/Socket/SSL/Intercept.pm: $self->{serial}($old_cert,$hash)
    assert_clean_parse(
        r#"my $serial = ref($self->{serial}) eq 'CODE' ? $self->{serial}($old_cert,$hash) : ++$self->{serial};"#,
    );
}

#[test]
fn test_cpan_hash_constant_in_ternary() {
    // From Test2/API/Context.pm: $self->{+CHILD_ERROR}
    assert_clean_parse(r#"$self->{+CHILD_ERROR} = $? unless exists $self->{+CHILD_ERROR};"#);
}

#[test]
fn test_cpan_hash_plus_constant_key() {
    // Pattern: $hash->{+CONSTANT} — plus forces constant context
    assert_clean_parse(r#"$self->{+TRACE};"#);
}

#[test]
fn test_cpan_hash_plus_constant_key_unless() {
    assert_clean_parse(r#"confess "error" unless $self->{+TRACE};"#);
}

#[test]
fn test_cpan_hash_plus_constant_assign_special() {
    assert_clean_parse(r#"$self->{+ERRNO} = $! unless exists $self->{+ERRNO};"#);
}

#[test]
fn test_cpan_hash_plus_constant_dollar_question() {
    // $? is a special variable
    assert_clean_parse(r#"$self->{+CHILD_ERROR} = $? unless exists $self->{+CHILD_ERROR};"#);
}

#[test]
fn test_cpan_test2_context_init() {
    // Full function from Test2/API/Context.pm
    let source = r#"
sub init {
    my $self = shift;

    confess "The 'trace' attribute is required"
        unless $self->{+TRACE};

    confess "The 'hub' attribute is required"
        unless $self->{+HUB};

    $self->{+_DEPTH} = 0 unless defined $self->{+_DEPTH};

    $self->{+ERRNO}       = $! unless exists $self->{+ERRNO};
    $self->{+EVAL_ERROR}  = $@ unless exists $self->{+EVAL_ERROR};
    $self->{+CHILD_ERROR} = $? unless exists $self->{+CHILD_ERROR};
}
"#;
    assert_clean_parse(source);
}

#[test]
fn test_cpan_test2_context_snapshot() {
    // bless {%{$_[0]}, ...}, __PACKAGE__
    assert_clean_parse(
        r#"sub snapshot { bless {%{$_[0]}, _is_canon => undef, _is_spawn => undef}, __PACKAGE__ }"#,
    );
}

#[test]
fn test_cpan_test2_context_restore() {
    // ($!, $@, $?) = @$self{+ERRNO, +EVAL_ERROR, +CHILD_ERROR}
    assert_clean_parse(r#"($!, $@, $?) = @$self{+ERRNO, +EVAL_ERROR, +CHILD_ERROR};"#);
}

#[test]
fn test_full_mojo_plugins_file() {
    // Exact content from Mojolicious/Plugins.pm (code portion only)
    let source = r#"package Mojolicious::Plugins;
use Mojo::Base 'Mojo::EventEmitter';

use Mojo::Loader qw(load_class);
use Mojo::Util   qw(camelize);

has namespaces => sub { ['Mojolicious::Plugin'] };

sub emit_chain {
  my ($self, $name, @args) = @_;

  my $wrapper;
  for my $cb (reverse @{$self->subscribers($name)}) {
    my $next = $wrapper;
    $wrapper = sub { $cb->($next, @args) };
  }

  !$wrapper ? return : return $wrapper->();
}

sub emit_hook {
  my $self = shift;
  for my $cb (@{$self->subscribers(shift)}) { $cb->(@_) }
  return $self;
}

sub emit_hook_reverse {
  my $self = shift;
  for my $cb (reverse @{$self->subscribers(shift)}) { $cb->(@_) }
  return $self;
}

sub load_plugin {
  my ($self, $name) = @_;
  my $suffix  = $name =~ /^[a-z]/ ? camelize $name : $name;
  my @classes = map {"${_}::$suffix"} @{$self->namespaces};
  for my $class (@classes, $name) { return $class->new if _load($class) }
  die qq{Plugin "$name" missing, maybe you need to install it?\n};
}

sub register_plugin { shift->load_plugin(shift)->register(shift, ref $_[0] ? $_[0] : {@_}) }

sub _load {
  my $module = shift;
  return $module->isa('Mojolicious::Plugin') unless my $e = load_class $module;
  ref $e ? die $e : return undef;
}

1;
"#;
    assert_clean_parse(source);
}
