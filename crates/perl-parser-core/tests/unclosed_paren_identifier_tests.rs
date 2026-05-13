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

#[test]
fn unicode_collate_map_expr_in_for_list() {
    // From Unicode::Collate: map EXPR, LIST inside a for-list must not leave
    // the parenthesized list waiting for a `)` before the identifier body.
    assert_clean_parse(
        r#"for my $vwt (map $self->getWt($_), @$subE) { my($var, @wt) = unpack(VCE_TEMPLATE, $vwt); }"#,
    );
}

#[test]
fn unicode_collate_pack_u_coderef_map_expr() {
    // From Unicode::Collate: pack arguments may include map EXPR, LIST where
    // the expression is a lexical coderef invocation.
    assert_clean_parse(r#"return pack('U*', map $unicode_to_native->($_), @_);"#);
}

#[test]
fn unicode_collate_unpack_u_coderef_map_expr() {
    // From Unicode::Collate: return may use map EXPR, LIST where the
    // expression is a lexical coderef call and the list is an unpack call.
    assert_clean_parse(
        r#"return map $native_to_unicode->($_), unpack('U*', shift(@_).pack('U*'));"#,
    );
}

#[test]
fn unicode_collate_override_hangul_map_expr() {
    // From Unicode::Collate: map EXPR, LIST may assign the result of a helper
    // call over a coderef-produced source list.
    assert_clean_parse(r#"@ce = map _pack_override($_, $u, $der), $hang->($u);"#);
}

#[test]
fn unicode_collate_decomposition_map_block() {
    // From Unicode::Collate: parenthesized map BLOCK LIST may contain nested
    // ternaries without leaving the caller waiting for a `)` before @decH.
    assert_clean_parse(
        r#"@ce = map({
            exists $map->{$_} ? @{ $map->{$_} } :
            $uXS && _exists_simple($_) ? _fetch_simple($_) :
            $der->($_);
        } @decH);"#,
    );
}

#[test]
fn unicode_collate_varce_return_map_expr() {
    // From Unicode::Collate: return may use map EXPR, LIST where the
    // expression is a method call and the list is a plain array.
    assert_clean_parse(r#"return map $self->varCE($_), @ce;"#);
}

#[test]
fn unicode_collate_gmatch_substr_return_map_expr() {
    // From Unicode::Collate: return may use map EXPR, LIST where the mapped
    // expression is a builtin call and the source list is a method call.
    assert_clean_parse(
        r#"return map substr($str, $_->[0], $_->[1]), $self->index($str, $sub, 0, 'g');"#,
    );
}

#[test]
fn unicode_collate_sort_map_arrayref_pipeline() {
    // From Unicode::Collate: a return may pipeline map BLOCK, sort BLOCK, and
    // map EXPR arrayref construction without leaving the statement open.
    assert_clean_parse(
        r#"return map { $_->[1] } sort { $a->[0] cmp $b->[0] } map [ $obj->getSortKey($_), $_ ], @_;"#,
    );
}

#[test]
fn unicode_collate_hst_join_map_split_expr() {
    // From Unicode::Collate: join may take map EXPR, LIST where the source
    // list is a split expression after the mapped function call.
    assert_clean_parse(r#"my $curHST = join '', map getHST($_, $vers), split /;/, $jcps;"#);
}

#[test]
fn dbi_registry_map_block_over_grep_block() {
    // From DBI: map BLOCK LIST may take a grep BLOCK expression as the source
    // list before a keys expression without leaving the parent assignment open.
    assert_clean_parse(
        r#"my %dbd_class_registry = map { $dbd_prefix_registry->{$_}->{class} => { prefix => $_ } } grep { exists $dbd_prefix_registry->{$_}->{class} } keys %{$dbd_prefix_registry};"#,
    );
}

#[test]
fn extutils_mm_unix_bootstrap_join_interpolated_map() {
    // From ExtUtils::MM_Unix: a return join list may contain an interpolated
    // map block in one string and a separate adjacent map block item.
    assert_clean_parse(
        r#"return join "\n",
        "BOOTSTRAP = @{[map { qq{$_.bs} } @exts]}\n",
        map { $self->_xs_make_bs($_) } @exts;"#,
    );
}

#[test]
fn capture_tiny_return_if_grep_comparison() {
    // From Capture::Tiny: a postfix-if return condition may compare @_ with a
    // grep BLOCK expression without treating the block as an unclosed list.
    assert_clean_parse(r#"return 1 if @_ == grep { -f } @_;"#);
}

#[test]
fn capture_tiny_stash_map_list_assignment() {
    // From Capture::Tiny: lexical list assignment may consume a map BLOCK over
    // a qw list without leaving the parenthesized declaration open.
    assert_clean_parse(r#"my ($fh, $pos) = map { $stash->{$_}{$name} } qw/capture pos/;"#);
}

#[test]
fn regexp_common_comment_combine_parenthesized_map_args() {
    // From Regexp::Common::comment: unary plus before a parenthesized map block
    // in a comma-separated argument list must not leave `combine` waiting for a `)`.
    assert_clean_parse(
        r#"my $pattern = combine +(map {to_eol $_} @{$group -> {to_eol}}),
                       (map {from_to @$_} @{$group -> {from_to}}),
                       (map {id       $_} @{$group -> {id}});"#,
    );
}

#[test]
fn extutils_mm_unix_for_list_with_parenthesized_map() {
    // From ExtUtils::MM_Unix: a for-list may mix qw groups and a parenthesized
    // map block without treating the next identifier as a missing `)`.
    assert_clean_parse(
        r#"for my $macro (qw(PERL_LIB PERL_ARCHLIB), (map { ("INSTALL".$_, "DESTINSTALL".$_) } $self->installvars), qw(PERL_SRC)) { $self->{$macro} ||= ""; }"#,
    );
}

#[test]
fn extutils_mm_unix_grep_parens_over_map_arrayref_default() {
    // From ExtUtils::MM_Unix: grep() may take a map BLOCK source over an
    // array dereference whose container falls back through word `or`.
    assert_clean_parse(
        r#"my @dirs = grep( -d $_, map { $self->catdir($_, 'auto') } @{$searchdirs || []} );"#,
    );
}

#[test]
fn main_package_variable_in_paren_expr() {
    // From Unicode::Normalize: `$::IS_ASCII` must parse as a main-package
    // variable, not as `$::` followed by a stray bare identifier.
    assert_clean_parse(r#"my $x = ($::IS_ASCII || $] < 5.008);"#);
}

#[test]
fn unicode_normalize_typeglob_ternary_native_subs() {
    // From Unicode::Normalize: a typeglob assignment may use a parenthesized
    // main-package condition followed by ternary anonymous subs.
    assert_clean_parse(
        r#"*to_native = ($::IS_ASCII || $] < 5.008)
             ? sub { return shift }
             : sub { utf8::unicode_to_native(shift) };"#,
    );
}

#[test]
fn unicode_normalize_pack_u_map_block() {
    // From Unicode::Normalize: pack arguments may include a map block before
    // the remaining argument list.
    assert_clean_parse(r#"return pack('U*', map { to_native($_) } @_);"#);
}

#[test]
fn unicode_normalize_unpack_u_map_block() {
    // From Unicode::Normalize: map may take an unpack call whose list contains
    // a shifted argument concatenated with a pack call.
    assert_clean_parse(r#"return map { from_native($_) } unpack('U*', shift(@_).pack('U*'));"#);
}

#[test]
fn unicode_normalize_printable_map_sprintf_split() {
    // From Unicode::Normalize: join may take a map block whose expression calls
    // sprintf, followed by a split expression as the map source list.
    assert_clean_parse(r#"return join " ", map { sprintf "\\x%02x", ord $_ } split "", $s;"#);
}

#[test]
fn x_repetition_prefix_decrement_in_parens() {
    // From Pod::Simple::XHTML: repetition RHS may be a prefix decrement.
    assert_clean_parse(r#"push @out, ('  ' x --$indent) . '</li>';"#);
}

#[test]
fn pod_simple_xhtml_entity_regex_map_assignment() {
    // From Pod::Simple::XHTML: a parenthesized lexical assignment may use
    // map EXPR, LIST followed by a join expression with another map chain.
    assert_clean_parse(
        r#"my ($entity_re) = map qr{$_}, join '|', map quotemeta, sort keys %entity_to_char;"#,
    );
}

#[test]
fn local_carp_not_scalar_ternary_caller() {
    // From Carp: localized package arrays may be assigned from a scalar()
    // parenthesized ternary without treating caller() as a missing `)`.
    assert_clean_parse(r#"local @CARP_NOT = scalar( $cgc ? $cgc->() : caller() );"#);
}

// === Sigil-peek heuristic: imported unary functions without parens (#1943) ===
// These all fail with "expected ')', found identifier" before the fix because
// `blessed`, `reftype`, etc. are not in the builtin table. The fix adds a
// sigil-peek heuristic in postfix.rs: if an unknown identifier is immediately
// followed by a sigil-starting token, treat it as a unary function call.

#[test]
fn blessed_self_in_if() {
    // From Moose::Util::TypeConstraints and many CPAN modules
    assert_clean_parse(r#"if (blessed $self) { $self->foo() }"#);
}

#[test]
fn blessed_in_unless() {
    // unless (blessed $obj)
    assert_clean_parse(r#"unless (blessed $obj) { die "not an object" }"#);
}

#[test]
fn blessed_with_and_chain() {
    // if (blessed $err and $err->isa("Foo"))
    assert_clean_parse(r#"if (blessed $err and $err->isa("Foo")) { 1 }"#);
}

#[test]
fn reftype_scalar_comparison() {
    // if (reftype $x eq 'ARRAY')
    assert_clean_parse(r#"if (reftype $x eq 'ARRAY') { 1 }"#);
}

#[test]
fn looks_like_number_sigil() {
    // looks_like_number $val — common in Type::Tiny and Params::Util
    assert_clean_parse(r#"return 0 unless looks_like_number $val;"#);
}

// === caller N edge cases ===

#[test]
fn caller_zero() {
    // caller 0 — most common stack-level query
    assert_clean_parse(r#"my @c = caller 0;"#);
}

#[test]
fn caller_one() {
    // caller 1 — one level up
    assert_clean_parse(r#"my @c = caller 1;"#);
}

#[test]
fn caller_with_parens() {
    // caller(0) — explicit parens, should still work
    assert_clean_parse(r#"my @c = caller(0);"#);
}

#[test]
fn caller_empty_parens() {
    // caller() — nullary with explicit empty parens
    assert_clean_parse(r#"my @c = caller();"#);
}

#[test]
fn caller_in_condition() {
    // Common defensive OO idiom: if (caller ne 'main') { ... }
    assert_clean_parse(r#"if (caller ne 'main') { run_tests() }"#);
}

// === ref + string comparison operators (is_str_op_terminated) ===

#[test]
fn ref_eq_string() {
    // ref $x eq 'ARRAY' — original motivation
    assert_clean_parse(r#"if (ref $x eq 'ARRAY') { 1 }"#);
}

#[test]
fn ref_ne_string() {
    assert_clean_parse(r#"if (ref $x ne 'CODE') { 1 }"#);
}

#[test]
fn ref_cmp_string() {
    // ref cmp 'value' — cmp is also a string comparison operator
    assert_clean_parse(r#"my $ord = ref $x cmp 'ARRAY';"#);
}

#[test]
fn defined_eq_string() {
    // Other builtins also need is_str_op_terminated: defined eq check
    assert_clean_parse(r#"if (lc $str eq 'hello') { 1 }"#);
}

// === ** precedence edge cases ===

#[test]
fn power_in_product() {
    // 8 * $z**3 must parse as 8 * ($z**3), not (8 * $z)**3
    assert_clean_parse(r#"my $x = 8 * $z**3;"#);
}

#[test]
fn power_both_sides_product() {
    // $a**2 * $b**2 — power on both sides of multiply
    assert_clean_parse(r#"my $x = $a**2 * $b**2;"#);
}

#[test]
fn power_in_division() {
    // 1 / $z**2 — power on RHS of division
    assert_clean_parse(r#"my $x = 1 / $z**2;"#);
}

#[test]
fn power_in_complex_formula() {
    // Multi-term formula from Legendre polynomial approximation
    assert_clean_parse(r#"$t = 1/(2 * $z) - 1/(8 * $z**3) + 1/(16 * $z**5);"#);
}

// === String literal as bare-call argument (TokenKind::String => true) ===

#[test]
fn croak_bare_string() {
    // croak "message" — Carp import without parens
    assert_clean_parse(r#"croak "Invalid argument";"#);
}

#[test]
fn confess_bare_string() {
    // confess "message" — Carp import without parens
    assert_clean_parse(r#"confess "Something went wrong";"#);
}

#[test]
fn carp_bare_string() {
    assert_clean_parse(r#"carp "Warning: deprecated";"#);
}

#[test]
fn hash_literal_not_confused_as_call() {
    // Hash construction must NOT be confused with bare call
    // 'key' is followed by =>, not a string argument
    assert_clean_parse(r#"my %h = (name => "Alice", age => 30);"#);
}

#[test]
fn list_with_bareword_and_string() {
    // (key, "value") — bareword in list context followed by comma, then string
    // The comma prevents TokenKind::String from firing for the bareword
    assert_clean_parse(r#"my @a = (foo, "bar", baz, "qux");"#);
}

// === Moo/Moose DSL now parses as FunctionCall — bare string args ===

#[test]
fn moo_has_bare_string_arg() {
    // has 'attr' => (is => 'ro') — string literal as first arg
    assert_clean_parse(r#"has 'name' => (is => 'ro', isa => 'Str');"#);
}

#[test]
fn moose_extends_bare_string() {
    // extends 'Base' — string literal as bare call arg
    assert_clean_parse(r#"extends 'Moose::Object';"#);
}

#[test]
fn moo_with_bare_string() {
    // with 'Role' — string literal as bare call arg
    assert_clean_parse(r#"with 'MooseX::Singleton';"#);
}

#[test]
fn moo_before_bare_string() {
    // before 'method' => sub { } — string literal as bare call arg
    assert_clean_parse(r#"before 'BUILD' => sub { my $self = shift; $self->_init };"#);
}

#[test]
fn moo_after_bare_string() {
    assert_clean_parse(r#"after 'save' => sub { my $self = shift; $self->_notify };"#);
}

#[test]
fn moo_around_bare_string() {
    assert_clean_parse(r#"around 'format' => sub { my ($orig, $self) = @_; $orig->($self) };"#);
}

#[test]
fn moo_requires_bare_string() {
    assert_clean_parse(r#"requires 'serialize';"#);
}

// === Dancer2 / Mojolicious web route DSL ===

#[test]
fn dancer_get_route() {
    assert_clean_parse(r#"get '/users' => sub { return 'ok' };"#);
}

#[test]
fn dancer_post_route() {
    assert_clean_parse(r#"post '/users' => sub { my $body = request->body; };"#);
}

#[test]
fn dancer_any_route() {
    assert_clean_parse(r#"any '/ping' => sub { return 'pong' };"#);
}

// === undef EXPR in expression context (#2834) ===
// undef is a keyword token (TokenKind::Undef), not Identifier.
// When used as `undef $var` in an expression (not at statement start),
// the postfix chain must recognise it and parse the argument.

#[test]
fn undef_expr_in_paren_or() {
    // From Storable.pm: close(FILE) or undef $ret
    assert_clean_parse(r#"if ($x or undef $ret) { 1 }"#);
}

#[test]
fn undef_expr_negated_or() {
    // From Storable.pm: if (!(close(FILE) or undef $ret) || $@)
    assert_clean_parse(r#"if (!(close($f) or undef $ret)) { die; }"#);
}

#[test]
fn undef_expr_nested_parens() {
    // undef inside nested parens with or
    assert_clean_parse(r#"my $ok = ($x || undef $y);"#);
}

// === x repetition operator with non-sigil identifier as RHS (#2834) ===
// In `'-' x width $title`, the RHS of `x` is an unqualified identifier
// (imported function) applied to a sigil argument. The parser must accept
// a plain identifier as the start of the x-operator RHS.

#[test]
fn x_rep_with_identifier_func() {
    // From Debconf: ('-' x width $title)
    assert_clean_parse(r#"my $s = ('-' x width $title);"#);
}

#[test]
fn x_rep_identifier_in_list() {
    // As it appears in the original: unshift @lines, $t, ('-' x width $t), '';
    assert_clean_parse(r#"unshift @lines, $title, ('-' x width $title), '';"#);
}

// === print(FILEHANDLE LIST) with explicit parens (#2834) ===
// `print( $fh EXPR )` — filehandle inside explicit parens.
// The parser must detect the indirect-object pattern even when
// print is called with explicit parentheses.

#[test]
fn print_parens_filehandle_join() {
    // From IPC::Run3::ProfLogger: print( $fh join(...) )
    assert_clean_parse(r#"print( $fh join(" ", @items) );"#);
}

#[test]
fn print_parens_filehandle_string() {
    // print( $fh "message" ) — string after filehandle var
    assert_clean_parse(r#"print( $fh "hello\n" );"#);
}

#[test]
fn print_parens_filehandle_var() {
    // print( $fh $msg ) — variable after filehandle
    assert_clean_parse(r#"print( $fh $msg );"#);
}

// === Additional edge case coverage (#2834 deep review) ===

#[test]
fn undef_no_arg_in_expr() {
    // Plain `undef` with no argument in expression context — must not consume next token
    assert_clean_parse(r#"my $x = $y || undef;"#);
}

#[test]
fn undef_no_arg_in_ternary() {
    // undef as rhs of ternary — no argument
    assert_clean_parse(r#"my $x = $cond ? 1 : undef;"#);
}

#[test]
fn undef_array_arg_in_expr() {
    // undef @arr in expression context (% sigil also supported)
    assert_clean_parse(r#"$x or undef @arr;"#);
}

#[test]
fn print_parens_empty() {
    // print() with empty parens — early exit path
    assert_clean_parse(r#"print();"#);
}

#[test]
fn print_parens_single_scalar_no_fh() {
    // print($msg) — single scalar, is the message not the filehandle
    // second token is ), so second_is_not_separator=false => regular parse
    assert_clean_parse(r#"print($msg);"#);
}

#[test]
fn print_parens_with_explicit_comma() {
    // print($fh, $msg) — with comma: second is Comma, regular parse
    assert_clean_parse(r#"print($fh, "hello\n");"#);
}

#[test]
fn say_parens_filehandle() {
    // say with explicit parens and filehandle
    assert_clean_parse(r#"say($fh "line\n");"#);
}

#[test]
fn printf_parens_filehandle_format() {
    // printf($fh "%s\n", $val) — printf with filehandle and format string
    assert_clean_parse(r#"printf($fh "%s\n", $val);"#);
}

#[test]
fn x_rep_with_builtin_func() {
    // "str" x length($s) — length is Identifier, not keyword in this context
    assert_clean_parse(r#"my $s = "-" x length($title);"#);
}

#[test]
fn x_rep_with_constant() {
    // "str" x CONSTANT — bareword constant as RHS
    assert_clean_parse(r#"my $s = "*" x COLS;"#);
}

#[test]
fn x_rep_with_expr_rhs() {
    // "str" x (func()) — parenthesized expression as RHS (was already working)
    assert_clean_parse(r#"my $s = "-" x (5 + 3);"#);
}

#[test]
fn print_parens_filehandle_list() {
    // print($fh @arr) — array arg after filehandle (second token is @arr, not a separator)
    assert_clean_parse(r#"print($fh @lines);"#);
}

#[test]
fn print_parens_block_filehandle_typeglob_deref() {
    // From Dpkg::Source::Archive: print({ *$self->{tar_input} } "$file\0")
    assert_clean_parse(r#"print({ *$self->{tar_input} } "$file\0") or die "write failed";"#);
}

#[test]
fn path_tiny_print_braced_filehandle_map_block() {
    // From Path::Tiny: print with explicit parens may combine a braced
    // filehandle and a map BLOCK source list.
    assert_clean_parse(
        r#"print( {$fh} map { ref eq 'ARRAY' ? @$_ : $_ } @data ) or $self->_throw('print');"#,
    );
}

#[test]
fn undef_in_return_expr() {
    // return undef — undef at statement boundary, no sigil follows
    assert_clean_parse(r#"sub f { return undef; }"#);
}

#[test]
fn undef_hash_arg_in_expr() {
    // undef %hash in expression context
    assert_clean_parse(r#"$ok or undef %cache;"#);
}

#[test]
fn send_parens_filehandle_stmt() {
    // send($sock "msg") — send with explicit parens and socket as filehandle, at statement level
    assert_clean_parse(r#"send($sock "data\n");"#);
}
