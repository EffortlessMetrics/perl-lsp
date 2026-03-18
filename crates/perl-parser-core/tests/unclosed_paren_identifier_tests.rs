//! Tests for the unclosed_paren_identifier error bucket.
//! These test patterns that trigger "expected ')', found identifier" errors
//! commonly seen in CPAN modules.

mod cpan_test_helpers;
use cpan_test_helpers::*;

// === Moose/Moo `has` with parenthesized arguments ===

#[test]
fn has_parens_bare_name_fat_arrow() {
    // has(name => ...) - very common Moose/Moo pattern
    assert_clean_parse(r#"has(name => (is => 'ro'));"#);
}

#[test]
fn has_parens_multiple_attrs() {
    assert_clean_parse(r#"has(name => "test", is => "ro", isa => "Str", required => 1);"#);
}

#[test]
fn has_parens_plus_name() {
    // Moose attribute with + prefix (attribute override)
    assert_clean_parse(r#"has("+name" => (is => "ro"));"#);
}

#[test]
fn has_parens_arrayref_attrs() {
    // has([qw(foo bar)] => (is => 'ro'))
    assert_clean_parse(r#"has([qw(foo bar)] => (is => 'ro'));"#);
}

// === local() declarations ===

#[test]
fn local_paren_list_assign() {
    // local($a, $b) = @_;
    assert_clean_parse(r#"local($a, $b) = @_;"#);
}

#[test]
fn local_paren_single() {
    assert_clean_parse(r#"local($x) = @_;"#);
}

#[test]
fn local_glob_paren() {
    // local(*FH) - localizing a glob
    assert_clean_parse(r#"local(*FH);"#);
}

#[test]
fn local_hash_element() {
    // local($hash{key}) = $val;
    assert_clean_parse(r#"local($hash{key}) = $val;"#);
}

// === Function calls with complex arguments ===

#[test]
fn func_call_fat_arrow_pairs() {
    assert_clean_parse(r#"foo(bar => 1, baz => 2);"#);
}

#[test]
fn func_call_mixed_args() {
    // Mix of positional and fat arrow args
    assert_clean_parse(r#"foo($x, bar => 1, baz => 2);"#);
}

#[test]
fn func_call_nested_parens() {
    assert_clean_parse(r#"foo(bar(1, 2), baz(3));"#);
}

#[test]
fn method_call_fat_arrow_args() {
    assert_clean_parse(r#"$obj->method(foo => 1, bar => 2);"#);
}

#[test]
fn constructor_with_parens() {
    assert_clean_parse(r#"Foo->new(bar => 1, baz => 2);"#);
}

// === Nested calls and complex expressions in parens ===

#[test]
fn nested_function_in_parens() {
    assert_clean_parse(r#"my $x = (foo(1) + bar(2));"#);
}

#[test]
fn ternary_in_parens() {
    assert_clean_parse(r#"my $x = ($a ? $b : $c);"#);
}

#[test]
fn hash_slice_in_parens() {
    assert_clean_parse(r#"my @vals = @hash{qw(foo bar baz)};"#);
}

// === Moose/Moo patterns from CPAN ===

#[test]
fn moose_has_with_lazy_builder() {
    assert_clean_parse(r#"has(cache => (is => 'ro', lazy => 1, builder => '_build_cache'));"#);
}

#[test]
fn moose_has_with_trigger() {
    assert_clean_parse(
        r#"has(name => (is => 'rw', trigger => sub { my ($self, $new) = @_; $self->_validate($new) }));"#,
    );
}

#[test]
fn moose_has_with_default_sub() {
    assert_clean_parse(r#"has(items => (is => 'ro', default => sub { [] }));"#);
}

#[test]
fn moo_has_coerce() {
    assert_clean_parse(r#"has(count => (is => 'ro', coerce => sub { int($_[0]) }));"#);
}

// === Other common CPAN patterns ===

#[test]
fn class_accessor_style() {
    // Class::Accessor / Moo::Role-style
    assert_clean_parse(r#"__PACKAGE__->mk_accessors(qw(name age color));"#);
}

#[test]
fn test_more_subtest() {
    assert_clean_parse(r#"subtest("widget tests" => sub { ok(1); });"#);
}

#[test]
fn exception_class_declare() {
    // Exception::Class style
    assert_clean_parse(r#"use Exception::Class ('MyException' => { fields => ['message'] });"#);
}

#[test]
fn dbi_connect_with_attrs() {
    assert_clean_parse(
        r#"my $dbh = DBI->connect($dsn, $user, $pass, { RaiseError => 1, AutoCommit => 0 });"#,
    );
}

#[test]
fn cgi_param_pairs() {
    assert_clean_parse(r#"$q->param(-name => 'foo', -value => 'bar');"#);
}

// === Edge cases that stress parenthesized argument parsing ===

#[test]
fn trailing_comma_in_parens() {
    assert_clean_parse(r#"foo(1, 2, 3,);"#);
}

#[test]
fn empty_parens() {
    assert_clean_parse(r#"foo();"#);
}

#[test]
fn nested_hash_in_call() {
    assert_clean_parse(r#"foo({ bar => 1, baz => 2 });"#);
}

#[test]
fn array_ref_in_call() {
    assert_clean_parse(r#"foo([1, 2, 3]);"#);
}

#[test]
fn complex_moose_has_statement() {
    // Real-world Moose has with many options
    assert_clean_parse(
        r#"
has(config => (
    is      => 'ro',
    isa     => 'HashRef',
    lazy    => 1,
    builder => '_build_config',
    handles => {
        get_setting => 'get',
        set_setting => 'set',
    },
));
"#,
    );
}

// === Patterns from CPAN corpus: map/grep inside for ===

#[test]
fn for_with_map_block() {
    // for my $x (map { $_->name } @items) { ... }
    assert_clean_parse(r#"for my $x (map { $_->name } @items) { print $x }"#);
}

#[test]
fn for_with_grep_block() {
    assert_clean_parse(r#"for my $x (grep { defined $_ } @items) { print $x }"#);
}

#[test]
fn for_with_sort_block() {
    assert_clean_parse(r#"for my $x (sort { $a cmp $b } @items) { print $x }"#);
}

#[test]
fn foreach_with_map_block() {
    assert_clean_parse(r#"foreach my $item (map { lc $_ } @list) { print $item }"#);
}

#[test]
fn for_with_nested_map_grep() {
    assert_clean_parse(
        r#"for my $x (map { $_->{name} } grep { $_->{active} } @items) { print $x }"#,
    );
}

// === Bare word function calls in paren context ===

#[test]
fn bare_func_in_parens() {
    // split inside parens with regex
    assert_clean_parse(r#"my @parts = (split /,/, $str);"#);
}

#[test]
fn join_with_args_in_parens() {
    assert_clean_parse(r#"my $str = join(",", @items);"#);
}

#[test]
fn sprintf_in_parens() {
    assert_clean_parse(r#"my $s = sprintf("%s: %d", $name, $count);"#);
}

// === Complex paren expressions from CPAN ===

#[test]
fn chained_method_in_for() {
    assert_clean_parse(r#"for my $row ($sth->fetchrow_hashref) { print $row->{name} }"#);
}

#[test]
fn keys_in_for() {
    assert_clean_parse(r#"for my $key (keys %hash) { print $hash{$key} }"#);
}

#[test]
fn values_in_for() {
    assert_clean_parse(r#"for my $val (values %hash) { print $val }"#);
}

#[test]
fn reverse_in_for() {
    assert_clean_parse(r#"for my $item (reverse @list) { print $item }"#);
}

#[test]
fn grep_regex_in_for() {
    assert_clean_parse(r#"for my $file (grep /\.pm$/, @files) { print $file }"#);
}

#[test]
fn map_builtin_in_for() {
    assert_clean_parse(r#"for my $x (map lc, @items) { print $x }"#);
}

// === Common CPAN calling conventions ===

#[test]
fn test_builder_pattern() {
    assert_clean_parse(
        r#"
Test::More::subtest('my test' => sub {
    my $obj = Foo->new(
        name => 'test',
        value => 42,
    );
    ok($obj->name eq 'test');
});
"#,
    );
}

#[test]
fn dispatch_table_in_hash() {
    assert_clean_parse(
        r#"
my %dispatch = (
    add    => sub { $_[0] + $_[1] },
    sub    => sub { $_[0] - $_[1] },
    mul    => sub { $_[0] * $_[1] },
);
"#,
    );
}

#[test]
fn complex_constructor_args() {
    assert_clean_parse(
        r#"
my $obj = Some::Class->new(
    name    => $config->{name},
    verbose => ($ENV{DEBUG} ? 1 : 0),
    handler => sub { my $self = shift; $self->process(@_) },
);
"#,
    );
}

// === Tricky parse patterns ===

#[test]
fn paren_list_with_word_operators() {
    assert_clean_parse(r#"my @result = ($x or $y, $z and $w);"#);
}

#[test]
fn do_block_in_parens() {
    assert_clean_parse(r#"my $val = (do { my $x = 1; $x + 2 });"#);
}

#[test]
fn eval_in_parens() {
    assert_clean_parse(r#"my $val = (eval { $obj->method() });"#);
}

#[test]
fn wantarray_in_parens() {
    assert_clean_parse(r#"return (wantarray ? @results : $results[0]);"#);
}

// === Indirect object syntax in parens ===

#[test]
fn new_in_parens() {
    assert_clean_parse(r#"my @objs = (Foo->new, Bar->new(1, 2));"#);
}

// === Multiline parenthesized expressions ===

#[test]
fn multiline_func_args() {
    assert_clean_parse(
        r#"
my $result = some_function(
    $first_arg,
    $second_arg,
    key => 'value',
    other_key => $var,
);
"#,
    );
}

#[test]
fn multiline_list_assignment() {
    assert_clean_parse(
        r#"
my ($self, %args) = @_;
"#,
    );
}

#[test]
fn multiline_hash_in_parens() {
    assert_clean_parse(
        r#"
my %opts = (
    verbose => 1,
    debug   => 0,
    output  => '/dev/null',
);
"#,
    );
}

// === Patterns found in CPAN corpus that trigger unclosed_paren_identifier ===

#[test]
fn for_range_deref_last_index() {
    // for my $i (0 .. $#$nums) — very common in Math::BigInt etc.
    assert_clean_parse(r#"for my $i (0 .. $#$nums) { print $nums->[$i] }"#);
}

#[test]
fn for_range_deref_last_index_2() {
    // for my $i (1 .. $#$in)
    assert_clean_parse(r#"for my $i (1 .. $#$in) { $x = $in->[$i] }"#);
}

#[test]
fn while_deref_last_index() {
    // while ($#$x > $#$y)
    assert_clean_parse(r#"while ($#$x > $#$y) { pop @$x }"#);
}

#[test]
fn sort_custom_comparator_in_parens() {
    // (sort _released_order @perls)[0]
    assert_clean_parse(r#"my $first = (sort _released_order @perls)[0];"#);
}

#[test]
fn sort_custom_cmp_chain() {
    // sort cmp_events map { ... } readdir($dh)
    assert_clean_parse(r#"for my $info (sort cmp_events map { $_ } readdir($dh)) { print $info }"#);
}

#[test]
fn uniq_in_for() {
    // foreach my $name (uniq @names)
    assert_clean_parse(r#"foreach my $name (uniq @names) { print $name }"#);
}

#[test]
fn uniq_map_in_for() {
    // foreach my $f (uniq map { ... } @items)
    assert_clean_parse(r#"foreach my $f (uniq map { $_->name } @items) { print $f }"#);
}

#[test]
fn blessed_in_condition() {
    // if (blessed $element && $element->isa(__PACKAGE__))
    assert_clean_parse(r#"if (blessed $element) { print "blessed" }"#);
}

#[test]
fn blessed_and_isa_in_condition() {
    assert_clean_parse(r#"if (blessed $element && $element->isa("Foo")) { print "ok" }"#);
}

#[test]
fn print_filehandle_in_unless() {
    // unless( print $handle $header )
    assert_clean_parse(r#"unless (print $handle $header) { die "write failed" }"#);
}

#[test]
fn print_block_filehandle_in_if() {
    // if (print { $self->{gui} } $mode)
    assert_clean_parse(r#"if (print { $self->{gui} } $mode) { return 1 }"#);
}

#[test]
fn exec_with_block_arg() {
    // exec({ $prog[0] } @prog)
    assert_clean_parse(r#"exec({ $prog[0] } @prog) or die "exec failed";"#);
}

#[test]
fn stat_list_subscript() {
    // (stat($file))[9]
    assert_clean_parse(r#"my $mtime = (stat($file))[9];"#);
}

#[test]
fn map_with_parens_and_keys() {
    // map({$_cache_id{$_} => $_} keys %_cache_id)
    assert_clean_parse(r#"my %rev = map({$cache{$_} => $_} keys %cache);"#);
}

#[test]
fn bare_function_call_in_args() {
    // inet_aton $host (imported function used as list op)
    assert_clean_parse(r#"connect $sock, sockaddr_in(6000, inet_aton $host);"#);
}

#[test]
fn bare_function_in_condition() {
    // defined(my $sub = _fetch_sub utf8 => 'is_utf8')
    assert_clean_parse(r#"if (defined(my $sub = _fetch_sub utf8 => 'is_utf8')) { print "ok" }"#);
}

#[test]
fn c_style_for_in_foreach() {
    // foreach (my $i = $n-1; $i >= 0; $i--)
    assert_clean_parse(r#"for (my $i = 0; $i < 10; $i++) { print $i }"#);
}

#[test]
fn if_last_index_comparison() {
    // if ($#$arg == 5)
    assert_clean_parse(r#"if ($#$arg == 5) { print "five elements" }"#);
}

#[test]
fn condition_with_deref_last_index() {
    // if ($i == $#$sibs)
    assert_clean_parse(r#"if ($i == $#$sibs) { print "last sibling" }"#);
}

#[test]
fn open_my_script_in_and_chain() {
    // open my $script, '<', $0
    assert_clean_parse(r#"if (-f $0 and open my $script, '<', $0) { print "ok" }"#);
}

#[test]
fn max_values_postfix_deref() {
    // (max values $CONFIG->{state}{keyorder}{$section}->%*)
    assert_clean_parse(r#"return (max values $hash->%*) || 0;"#);
}

#[test]
fn unless_null_in_paren() {
    // unless (null $root)
    assert_clean_parse(r#"unless (null $root) { print "not null" }"#);
}
