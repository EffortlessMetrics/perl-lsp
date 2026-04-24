mod cpan_test_helpers;
use cpan_test_helpers::*;

// ===== Fat arrow in various expression contexts =====

// Moose/Moo `has` with fat arrow: has name => (is => 'ro')
#[test]
fn has_attribute_fat_arrow() {
    let source = r#"has name => (is => 'ro', isa => 'Str');"#;
    assert_clean_parse(source);
}

// Multiple `has` attributes
#[test]
fn has_multiple_attributes() {
    let source = r#"has [qw(foo bar)] => (is => 'ro');"#;
    assert_clean_parse(source);
}

// Anonymous hash ref with fat arrow
#[test]
fn anon_hashref_fat_arrow() {
    let source = r#"my $h = { key => "value", foo => "bar" };"#;
    assert_clean_parse(source);
}

// Function call with fat arrow named args
#[test]
fn function_call_fat_arrow_args() {
    let source = r#"foo(bar => 1, baz => 2);"#;
    assert_clean_parse(source);
}

// Method call with fat arrow named args
#[test]
fn method_call_fat_arrow_args() {
    let source = r#"$obj->method(key => "value");"#;
    assert_clean_parse(source);
}

// push with fat arrow (Perl allows => anywhere comma is valid)
#[test]
fn push_fat_arrow_separator() {
    let source = r#"push @array => $value;"#;
    assert_clean_parse(source);
}

// Hash assignment with fat arrow
#[test]
fn hash_assignment_fat_arrow() {
    let source = r#"my %hash = (key1 => "val1", key2 => "val2");"#;
    assert_clean_parse(source);
}

// Nested hash refs
#[test]
fn nested_hashref_fat_arrow() {
    let source = r#"my $config = { db => { host => "localhost", port => 5432 } };"#;
    assert_clean_parse(source);
}

// Fat arrow in array ref context
#[test]
fn fat_arrow_in_arrayref() {
    let source = r#"my $a = [key => "value"];"#;
    assert_clean_parse(source);
}

// Fat arrow after string literal
#[test]
fn string_fat_arrow() {
    let source = r#"my %h = ("key" => "value");"#;
    assert_clean_parse(source);
}

// Fat arrow after number
#[test]
fn number_fat_arrow() {
    let source = r#"my %h = (1 => "one", 2 => "two");"#;
    assert_clean_parse(source);
}

// Fat arrow in return context
#[test]
fn return_hash_fat_arrow() {
    let source = r#"sub foo { return (key => "value") }"#;
    assert_clean_parse(source);
}

// Chained method calls with fat arrow args
#[test]
fn chained_method_fat_arrow() {
    let source = r#"$obj->foo(bar => 1)->baz(qux => 2);"#;
    assert_clean_parse(source);
}

// Fat arrow in ternary
#[test]
fn fat_arrow_in_ternary_value() {
    let source = r#"my %h = $cond ? (a => 1) : (b => 2);"#;
    assert_clean_parse(source);
}

// Fat arrow with complex RHS expressions
#[test]
fn fat_arrow_complex_rhs() {
    let source = r#"my %h = (key => $a + $b, other => $c || $d);"#;
    assert_clean_parse(source);
}

// Fat arrow with sub ref as value
#[test]
fn fat_arrow_sub_ref_value() {
    let source = r#"my %dispatch = (add => sub { $_[0] + $_[1] }, mul => sub { $_[0] * $_[1] });"#;
    assert_clean_parse(source);
}

// Fat arrow in method call without parens (common Moose pattern)
#[test]
fn has_fat_arrow_no_parens() {
    let source = r#"has 'name' => (is => 'ro');"#;
    assert_clean_parse(source);
}

// Comma and fat arrow mixed
#[test]
fn mixed_comma_fat_arrow() {
    let source = r#"my @list = (a => 1, "b", "c", d => 2);"#;
    assert_clean_parse(source);
}

// Fat arrow with negative number value
#[test]
fn fat_arrow_negative_number() {
    let source = r#"my %h = (offset => -1, limit => 100);"#;
    assert_clean_parse(source);
}

// Fat arrow with array ref value
#[test]
fn fat_arrow_arrayref_value() {
    let source = r#"my %h = (items => [1, 2, 3], names => ["a", "b"]);"#;
    assert_clean_parse(source);
}

// Fat arrow with hashref value
#[test]
fn fat_arrow_hashref_value() {
    let source = r#"my %h = (config => { debug => 1 });"#;
    assert_clean_parse(source);
}

// die with hashref (common pattern)
#[test]
fn die_hashref_fat_arrow() {
    let source = r#"die { error => "bad input", code => 42 };"#;
    assert_clean_parse(source);
}

// Moose-style `extends`, `with`, `before`, `after`, `around`
#[test]
fn moose_extends_with() {
    let source = r#"extends 'Some::Class';
with 'Some::Role';
before 'method' => sub { print "before\n" };
after 'method' => sub { print "after\n" };
around 'method' => sub { my $orig = shift; $orig->(@_) };"#;
    assert_clean_parse(source);
}

// Perl `=>` used as separator in print
#[test]
fn print_fat_arrow() {
    let source = r#"print STDERR => "error message\n";"#;
    assert_clean_parse(source);
}

// Complex: hash slice with fat arrow
#[test]
fn hash_constructor_in_call() {
    let source = r#"$self->configure(name => $name, verbose => 1);"#;
    assert_clean_parse(source);
}

// Trailing comma after fat arrow pair
#[test]
fn trailing_comma_after_fat_arrow() {
    let source = r#"my %h = (key => "value",);"#;
    assert_clean_parse(source);
}

// Empty hash
#[test]
fn empty_hash_ref() {
    let source = r#"my $h = {};"#;
    assert_clean_parse(source);
}

// `use parent` / `use base` with fat arrow
#[test]
fn use_parent_fat_arrow() {
    let source = r#"use parent -norequire => 'Some::Class';"#;
    assert_clean_parse(source);
}

// ===== Patterns likely seen in CPAN that might trigger unexpected_fat_arrow_expr =====

// Fat arrow after a variable (not a bareword)
#[test]
fn variable_fat_arrow() {
    let source = r#"my %h = ($key => $value);"#;
    assert_clean_parse(source);
}

// Fat arrow in list assignment
#[test]
fn list_assignment_fat_arrow() {
    let source = r#"my @pairs = (one => 1, two => 2, three => 3);"#;
    assert_clean_parse(source);
}

// Fat arrow in for loop
#[test]
fn fat_arrow_in_for_hash() {
    let source = r#"for my $k (keys %h) { print $k => $h{$k} }"#;
    assert_clean_parse(source);
}

// Bare `has` as user-defined function
#[test]
fn user_defined_has_fat_arrow() {
    let source = r#"has name => (is => 'ro', isa => 'Str', default => sub { 'unnamed' });"#;
    assert_clean_parse(source);
}

// Fat arrow in map
#[test]
fn map_fat_arrow_result() {
    let source = r#"my %h = map { $_ => 1 } @list;"#;
    assert_clean_parse(source);
}

// Fat arrow inside grep block
#[test]
fn grep_with_fat_arrow() {
    let source = r#"my @r = grep { $_->{key} => 1 } @items;"#;
    assert_clean_parse(source);
}

// Chained fat arrow pairs in assignment
#[test]
fn chained_fat_arrow_pairs_stmt() {
    let source = r#"%opts = (verbose => 1, debug => 0, output => '/tmp/log');"#;
    assert_clean_parse(source);
}

// Passing hash to function without parens
#[test]
fn function_hash_no_parens() {
    let source = r#"configure name => "test", verbose => 1;"#;
    assert_clean_parse(source);
}

// Dispatch table
#[test]
fn dispatch_table() {
    let source = r#"my %dispatch = (
    add  => \&do_add,
    del  => \&do_del,
    list => \&do_list,
);"#;
    assert_clean_parse(source);
}

// Moose `has` with `+` prefix (attribute override)
#[test]
fn moose_has_plus_override() {
    let source = r#"has '+name' => (is => 'rw');"#;
    assert_clean_parse(source);
}

// Type::Tiny / Type::Utils `declare`, `coerce` with fat arrow
#[test]
fn type_tiny_declare_coerce() {
    let source = r#"declare "PositiveInt", as Int, where { $_ > 0 };"#;
    assert_clean_parse(source);
}

// Hash ref in return
#[test]
fn return_hashref() {
    let source = r#"sub foo { return { status => 'ok', code => 200 } }"#;
    assert_clean_parse(source);
}

// Constructor pattern
#[test]
fn constructor_new_hash() {
    let source = r#"my $obj = Class->new(name => "test", id => 42);"#;
    assert_clean_parse(source);
}

// Nested method calls with hash args
#[test]
fn nested_method_hash_args() {
    let source = r#"$self->log->info(message => "starting", level => 1);"#;
    assert_clean_parse(source);
}

// Exception object construction (common pattern)
#[test]
fn exception_construction() {
    let source = r#"Exception->throw(error => "bad", trace => Carp::longmess());"#;
    assert_clean_parse(source);
}

// Catalyst / Dancer route with fat arrow
#[test]
fn catalyst_action() {
    let source = r#"__PACKAGE__->config(namespace => 'api');"#;
    assert_clean_parse(source);
}

#[test]
fn map_block_with_builtin_call_key_expr() {
    let source = r#"our %ENCODE_NAME_OF = map { uc $MIME_NAME_OF{$_} => $_ } keys %MIME_NAME_OF;"#;
    assert_clean_parse(source);
}

#[test]
fn map_block_with_ternary_key_expr_in_arrayref() {
    let source = r#"my @pattern = map { [ ( ref $_ ? $_ : qr/$_/ ) => 0 ] } @values;"#;
    assert_clean_parse(source);
}

#[test]
fn readonly_style_scalar_decl_before_fat_arrow() {
    let source = r#"if (ref eq 'SCALAR') { Scalar my $v => $$_; $_ = \$v }"#;
    assert_clean_parse(source);
}

#[test]
fn readonly_style_array_decl_before_fat_arrow() {
    let source = r#"if (ref eq 'ARRAY') { Array my @v => @$_; $_ = \@v }"#;
    assert_clean_parse(source);
}

#[test]
fn readonly_style_hash_decl_before_fat_arrow() {
    let source = r#"if (ref eq 'HASH') { Hash my %v => $_; $_ = \%v }"#;
    assert_clean_parse(source);
}

#[test]
fn word_or_rhs_hash_pair() {
    let source = r#"$a or foo => 1;"#;
    assert_clean_parse(source);
}

#[test]
fn word_and_rhs_hash_pair() {
    let source = r#"$a and foo => 1;"#;
    assert_clean_parse(source);
}

#[test]
fn trailing_fat_arrow_in_parenthesized_call_args() {
    let source = r#"foo(on_closed =>);"#;
    assert_clean_parse(source);
}

#[test]
fn keyword_hash_key_after_block_in_bare_call_args() {
    let source = r#"repeat { 1 } foreach => $addrlist;"#;
    assert_clean_parse(source);
}

#[test]
fn chained_keyword_fat_arrow_args_in_method_call() {
    let source = r#"$future->new->fail("connect: $err", connect => connect => $err);"#;
    assert_clean_parse(source);
}

#[test]
fn variable_then_dash_key_fat_arrow_args() {
    let source = r#"field $class_id => -init => "$value";"#;
    assert_clean_parse(source);
}

#[test]
fn dash_prefixed_named_args_in_method_call() {
    let source = r#"$w->configure(-background => 'black');"#;
    assert_clean_parse(source);
}

#[test]
fn my_declaration_followed_by_fat_arrow_separator() {
    let source = r#"my $filename => basename $url->path;"#;
    assert_clean_parse(source);
}

#[test]
fn bare_block_call_with_multiple_named_keyword_args() {
    let source =
        r#"try_repeat { $self->bind($ai) } foreach => \@addrs, until => sub { shift->is_done };"#;
    assert_clean_parse(source);
}

#[test]
fn filetest_chain_as_fat_arrow_table() {
    let source = r#"my %x = (-r => readable => -R => r_readable =>);"#;
    assert_clean_parse(source);
}

#[test]
fn dash_prefixed_key_in_array_constructor() {
    let source = r#"my @args = (-G => '%{cmake_generator}');"#;
    assert_clean_parse(source);
}

#[test]
fn map_prefix_plus_hash_pairs() {
    let source = r#"my %h = ((map +($_ => __PACKAGE__->make_binop_expander('_expand_between')), qw(between not_between)),);"#;
    assert_clean_parse(source);
}

#[test]
fn double_fat_arrow_before_subref_value() {
    let source = r#"my %noquote = (bit => => sub { $_[0] =~ /^[01]\z/ },);"#;
    assert_clean_parse(source);
}

#[test]
fn chained_subref_pairs_with_double_fat_arrow_middle_entry() {
    let source = r#"my %noquote = (
        int => sub { $_[0] =~ /^ [-+]? \d+ \z/x },
        bit => => sub { $_[0] =~ /^[01]\z/ },
        money => sub { $_[0] =~ /^\$ \d+ (?:\.\d*)? \z/x },
    );"#;
    assert_clean_parse(source);
}

#[test]
fn word_operator_tokens_used_as_hash_keys() {
    let source = r#"my %render = (not => '_render_unop_paren', and => '_render_op_andor', or => '_render_op_andor');"#;
    assert_clean_parse(source);
}

#[test]
fn overload_qw_name_followed_by_subref_value() {
    let source = r#"use overload qw[bool] => sub { 0 };"#;
    assert_clean_parse(source);
}

#[test]
fn aligned_filetest_chain_with_empty_middle_key() {
    let source = r#"my %x = (-e => exists => -f => file => => -p => fifo =>);"#;
    assert_clean_parse(source);
}

#[test]
fn bare_hash_assignment_with_aligned_filetest_chain() {
    let source = r#"%X_tests = (
    -r  =>  readable           =>  -R  =>  r_readable      =>
    -w  =>  writeable          =>  -W  =>  r_writeable     =>
    -w  =>  writable           =>  -W  =>  r_writable      =>
    -x  =>  executable         =>  -X  =>  r_executable    =>
    -o  =>  owned              =>  -O  =>  r_owned         =>

    -e  =>  exists             =>  -f  =>  file            =>
    -z  =>  empty              =>  -d  =>  directory       =>
    -s  =>  nonempty           =>  -l  =>  symlink         =>
                               =>  -p  =>  fifo            =>
    -u  =>  setuid             =>  -S  =>  socket          =>
    -g  =>  setgid             =>  -b  =>  block           =>
    -k  =>  sticky             =>  -c  =>  character       =>
                               =>  -t  =>  tty             =>
    -M  =>  modified                                       =>
    -A  =>  accessed           =>  -T  =>  ascii           =>
    -C  =>  changed            =>  -B  =>  binary          =>
   );"#;
    assert_clean_parse(source);
}

#[test]
fn use_constant_with_block_and_named_callback() {
    let source =
        r#"use constant ON_APPLICATION => do { after apply_roles_to_package => sub { 1 }; 1; };"#;
    assert_clean_parse(source);
}

// DBI connect with hash
#[test]
fn dbi_connect_hash() {
    let source =
        r#"my $dbh = DBI->connect($dsn, $user, $pass, { RaiseError => 1, AutoCommit => 0 });"#;
    assert_clean_parse(source);
}

// Multiple fat arrow in complex expression
#[test]
fn complex_nested_fat_arrow() {
    let source = r#"my %config = (
    database => {
        host => 'localhost',
        port => 5432,
        options => { timeout => 30, retry => 3 },
    },
    logging => {
        level => 'info',
        file  => '/var/log/app.log',
    },
);"#;
    assert_clean_parse(source);
}

// Ternary with hash construction
#[test]
fn ternary_hash_construction() {
    let source = r#"my $opts = $debug ? { verbose => 1, trace => 1 } : { verbose => 0 };"#;
    assert_clean_parse(source);
}

// eval with hash
#[test]
fn eval_hashref() {
    let source = r#"eval { $obj->method(timeout => 10) };"#;
    assert_clean_parse(source);
}

// wantarray with fat arrow
#[test]
fn wantarray_fat_arrow_context() {
    let source = r#"return wantarray ? (status => 'ok') : { status => 'ok' };"#;
    assert_clean_parse(source);
}

// Fat arrow after expression (e.g., in list context)
#[test]
fn expression_result_fat_arrow() {
    let source = r#"my @x = ($a + $b => $c);"#;
    assert_clean_parse(source);
}

// Unshift with fat arrow
#[test]
fn unshift_fat_arrow_sep() {
    let source = r#"unshift @arr => $val;"#;
    assert_clean_parse(source);
}

// splice with fat arrow
#[test]
fn splice_fat_arrow_sep() {
    let source = r#"splice @arr, 0, 1 => @new;"#;
    assert_clean_parse(source);
}

// Hash in anonymous sub
#[test]
fn hash_in_anon_sub() {
    let source = r#"my $cb = sub { return { ok => 1 } };"#;
    assert_clean_parse(source);
}

// do-while with hashref
#[test]
fn do_block_hashref() {
    let source = r#"my $r = do { { key => "val" } };"#;
    assert_clean_parse(source);
}

// Local hash assignment
#[test]
fn local_hash_assignment() {
    let source = r#"local %ENV = (%ENV, PATH => '/usr/bin');"#;
    assert_clean_parse(source);
}

// Fat arrow with qw
#[test]
fn fat_arrow_with_qw() {
    let source = r#"my %h = (names => [qw(foo bar baz)]);"#;
    assert_clean_parse(source);
}

// Multiple return values with fat arrow
#[test]
fn multi_return_fat_arrow() {
    let source = r#"sub info { return (name => $self->{name}, age => $self->{age}) }"#;
    assert_clean_parse(source);
}

// Complex: hash of arrays
#[test]
fn hash_of_arrays() {
    let source = r#"my %data = (fruits => ['apple', 'banana'], vegs => ['carrot', 'pea']);"#;
    assert_clean_parse(source);
}

// OO: Moo/Moose `with` and `extends` combined
#[test]
fn moose_with_extends() {
    let source = r#"
extends 'Base::Class';
with 'Role::One', 'Role::Two';
has foo => (is => 'ro', default => sub { [] });
has bar => (is => 'rw', isa => 'Str', required => 1);
"#;
    assert_clean_parse(source);
}

// Test::More with fat arrow
#[test]
fn test_more_fat_arrow() {
    let source = r#"is($got, $expected, "test name");"#;
    assert_clean_parse(source);
}

// Carp with fat arrow in hashref
#[test]
fn croak_hashref() {
    let source = r#"croak { message => "bad input", code => 42 };"#;
    assert_clean_parse(source);
}

// ===== Patterns found in actual CPAN/system Perl files =====

// bless EXPR => CLASS (CPAN::Distroprefs, Encode.pm)
// In Perl, `bless $ref => $class` is valid — => acts as comma
#[test]
fn bless_fat_arrow_class() {
    let source = r#"bless $_[1] => $_[0];"#;
    assert_clean_parse(source);
}

// bless {} => CLASS
#[test]
fn bless_hashref_fat_arrow_class() {
    let source = r#"sub new { bless $_[1] || {} => $_[0] }"#;
    assert_clean_parse(source);
}

// bless hash element => package (Encode.pm)
#[test]
fn bless_hash_elem_fat_arrow_package() {
    let source = r#"bless $obj{$_} => __PACKAGE__;"#;
    assert_clean_parse(source);
}

// Comma followed by fat arrow: `key, => value` (B::Deparse)
// Perl treats `, =>` as just `,` then `=>` — the comma is redundant
#[test]
fn comma_then_fat_arrow() {
    let source = r#"my %h = (OPpLVREF_SV, => '$', OPpLVREF_AV, => '@');"#;
    assert_clean_parse(source);
}

// Array ref with multiple fat arrows: [key => KEY => [value]]
// (ExtUtils::Installed)
#[test]
fn arrayref_chained_fat_arrows() {
    let source = r#"my @tuples = ([inc_override => INC => [ @INC ]]);"#;
    assert_clean_parse(source);
}

// Full ExtUtils::Installed pattern
#[test]
fn for_tuple_chained_fat_arrows() {
    let source = r#"for my $tuple ([inc_override => INC => [ @INC ] ],
                   [ extra_libs => EXTRA => [] ])
{
    my ($arg,$key,$val)=@$tuple;
}"#;
    assert_clean_parse(source);
}

// Regex capture var with comma-fat-arrow (Pod::Simple::HTML)
#[test]
fn regex_capture_comma_fat_arrow() {
    let source = r#"my @r = ( $1, => "<$2>", "/$1", => "</$2>" );"#;
    assert_clean_parse(source);
}

// Full Pod::Simple::HTML pattern
#[test]
fn map_ternary_comma_fat_arrow() {
    let source = r#"return map {; m/^([-_:0-9a-zA-Z]+)=([-_:0-9a-zA-Z]+)$/s
     ? ( $1, => "\n<$2>", "/$1", => "</$2>\n" ) : die "Funky $_"
  } @_;"#;
    assert_clean_parse(source);
}

// bless with define_encoding pattern (Encode.pm)
#[test]
fn define_encoding_fat_arrow() {
    let source = r#"Encode::define_encoding( $obj{$_} => $_ );"#;
    assert_clean_parse(source);
}

// ===== Additional regression tests for complex fat arrow value expressions =====

// Fat arrow with do block value (issue #1651 acceptance criteria)
#[test]
fn fat_arrow_do_block_value() {
    let source = r#"my %h = (data => do { my $x = 1; $x + 1 });"#;
    assert_clean_parse(source);
}

// Fat arrow with method call chain as value
#[test]
fn fat_arrow_method_chain_value() {
    let source = r#"my %h = (result => $obj->foo->bar->baz);"#;
    assert_clean_parse(source);
}

// Fat arrow with regex as value
#[test]
fn fat_arrow_qr_regex_value() {
    let source = r#"my %h = (pattern => qr/^\d+$/, alt => qr{foo|bar}i);"#;
    assert_clean_parse(source);
}

// Fat arrow with string concatenation value
#[test]
fn fat_arrow_concat_value() {
    let source = r#"my %h = (path => $dir . "/" . $file);"#;
    assert_clean_parse(source);
}

// Fat arrow with ternary value
#[test]
fn fat_arrow_ternary_value() {
    let source = r#"my %h = (mode => $debug ? "verbose" : "quiet");"#;
    assert_clean_parse(source);
}

// Fat arrow with logical or value (common default pattern)
#[test]
fn fat_arrow_logical_or_value() {
    let source = r#"my %h = (name => $opts{name} || "default");"#;
    assert_clean_parse(source);
}

// Fat arrow with defined-or value
#[test]
fn fat_arrow_defined_or_value() {
    let source = r#"my %h = (port => $ENV{PORT} // 8080);"#;
    assert_clean_parse(source);
}

// Fat arrow with anonymous sub that has prototype
#[test]
fn fat_arrow_sub_with_args() {
    let source = r#"my %h = (handler => sub { my ($self, $req) = @_; return $req });"#;
    assert_clean_parse(source);
}

// Fat arrow with backslash ref values
#[test]
fn fat_arrow_ref_values() {
    let source = r#"my %h = (code => \&handler, list => \@items, map => \%config);"#;
    assert_clean_parse(source);
}

// Deeply nested hash with mixed structures
#[test]
fn fat_arrow_deeply_nested() {
    let source = r#"my $cfg = {
    server => {
        listen => [":8080", ":8443"],
        tls => { cert => "/etc/ssl/cert.pem", key => "/etc/ssl/key.pem" },
    },
    routes => [
        { path => "/api", handler => \&api_handler },
        { path => "/", handler => sub { return { status => 200 } } },
    ],
};"#;
    assert_clean_parse(source);
}

// Fat arrow in complex Moose has with builder/default
#[test]
fn moose_has_complex_attributes() {
    let source = r#"has config => (
    is      => 'ro',
    isa     => 'HashRef',
    lazy    => 1,
    builder => '_build_config',
    default => sub { {} },
);"#;
    assert_clean_parse(source);
}

// Fat arrow in hash slice assignment
#[test]
fn hash_slice_assignment_fat_arrow() {
    let source = r#"@hash{qw(a b c)} = (1 => 2, 3);"#;
    assert_clean_parse(source);
}

// Fat arrow in complex map expression
#[test]
fn fat_arrow_in_map_to_hash() {
    let source = r#"my %index = map { $_->name => $_ } @objects;"#;
    assert_clean_parse(source);
}

// Fat arrow with sprintf value
#[test]
fn fat_arrow_sprintf_value() {
    let source = r#"my %h = (msg => sprintf("Hello %s, you have %d items", $name, $count));"#;
    assert_clean_parse(source);
}

// Fat arrow in die with complex expression
#[test]
fn fat_arrow_die_complex() {
    let source = r#"die {
    error   => "Request failed",
    code    => $resp->code,
    message => $resp->message // "unknown",
    trace   => Carp::longmess(),
};"#;
    assert_clean_parse(source);
}

// Fat arrow after expression with postfix deref
#[test]
fn fat_arrow_postfix_deref_value() {
    let source =
        r#"my %h = (items => $data->{results}->@*, count => scalar $data->{results}->@*);"#;
    assert_clean_parse(source);
}

// Multiple fat arrows in dispatch table
#[test]
fn fat_arrow_complex_dispatch_table() {
    let source = r#"my %dispatch = (
    GET    => sub { $self->handle_get(@_) },
    POST   => sub { $self->handle_post(@_) },
    DELETE => sub { $self->handle_delete(@_) },
    PUT    => sub { $self->handle_put(@_) },
);"#;
    assert_clean_parse(source);
}

// Fat arrow in eval block with error hash
#[test]
fn fat_arrow_eval_error_handling() {
    let source = r#"my $result = eval {
    my $data = $self->fetch(timeout => 30, retry => 3);
    return { ok => 1, data => $data };
} or do {
    return { ok => 0, error => $@ };
};"#;
    assert_clean_parse(source);
}
