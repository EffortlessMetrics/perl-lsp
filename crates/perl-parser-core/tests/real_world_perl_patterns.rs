//! Edge-case tests for real-world Perl patterns commonly found in CPAN modules.
//!
//! Covers chained method calls with complex arguments, regex in various
//! contexts, here-doc variations, nested data structures, postfix
//! dereferencing, chained ternaries, slice expressions, qw() in various
//! contexts, prototyped subroutines, and multiline string concatenation.

mod cpan_test_helpers;
use cpan_test_helpers::*;

// ===========================================================================
// 1. Chained method calls with complex arguments
// ===========================================================================

#[test]
fn chained_methods_with_hashref_arg() -> Result<(), String> {
    let code = r#"
$schema->resultset('User')
    ->search({ active => 1, role => 'admin' })
    ->order_by('created_at')
    ->all;
"#;
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn chained_methods_with_nested_sub_arg() -> Result<(), String> {
    let code = r#"
$app->route('/api')
    ->under(sub { my $c = shift; $c->stash->{user} = authenticate($c) })
    ->get('/data' => sub { shift->render(json => { ok => 1 }) });
"#;
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn chained_methods_with_ternary_arg() -> Result<(), String> {
    let code = r#"
$query->select('*')
    ->from('users')
    ->where($active ? { status => 'active' } : {})
    ->limit($max_results // 100);
"#;
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn method_chain_across_arrow_deref() -> Result<(), String> {
    let code = "$self->{schema}->resultset('Item')->find({ id => $id })->update({ seen => 1 });";
    assert_clean_parse(code);
    Ok(())
}

// ===========================================================================
// 2. Regex in various contexts
// ===========================================================================

#[test]
fn regex_in_map_block() -> Result<(), String> {
    let code = r#"my @matches = map { /^(\w+)=(.*)$/ ? ($1 => $2) : () } @lines;"#;
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn regex_in_grep_with_variable_interpolation() -> Result<(), String> {
    let code = r#"my @found = grep { /$pattern/i } @candidates;"#;
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn regex_in_while_with_captures() -> Result<(), String> {
    let code = r#"
while ($text =~ m/\b(\w+)\s+\1\b/g) {
    push @dupes, $1;
}
"#;
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn regex_substitution_in_map() -> Result<(), String> {
    let code = r#"my @cleaned = map { (my $s = $_) =~ s/^\s+|\s+$//gr } @strings;"#;
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn regex_in_list_assignment() -> Result<(), String> {
    let code = r#"my ($proto, $host, $port) = $url =~ m{^(\w+)://([^/:]+)(?::(\d+))?};"#;
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn regex_alternation_in_condition() -> Result<(), String> {
    let code = r#"
if ($line =~ /^(?:BEGIN|END|INIT|CHECK|UNITCHECK)\s*\{/) {
    $in_special_block = 1;
}
"#;
    assert_clean_parse(code);
    Ok(())
}

// ===========================================================================
// 3. Here-doc variations
// ===========================================================================

#[test]
fn heredoc_indented_tilde() -> Result<(), String> {
    let code = r#"
my $text = <<~END;
    This is indented text.
    Leading whitespace is stripped.
    END
"#;
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn heredoc_single_quoted_no_interpolation() -> Result<(), String> {
    let code = r#"
my $raw = <<'LITERAL';
No $interpolation here.
Not even \n escapes.
LITERAL
"#;
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn heredoc_in_function_arg_position() -> Result<(), String> {
    let code = r#"
my $result = process(<<END, $extra_arg);
Here-doc content as first arg
END
"#;
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn heredoc_backtick_command() -> Result<(), String> {
    let code = r#"
my $output = <<`CMD`;
echo "Hello from $user"
CMD
"#;
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn heredoc_in_hash_value() -> Result<(), String> {
    let code = r#"
my %templates = (
    header => <<'END_HEADER',
<html><head><title>Page</title></head>
END_HEADER
    footer => <<'END_FOOTER',
</body></html>
END_FOOTER
);
"#;
    assert_clean_parse(code);
    Ok(())
}

// ===========================================================================
// 4. Complex data structure literals
// ===========================================================================

#[test]
fn deeply_nested_hashref_with_arrayrefs() -> Result<(), String> {
    let code = r#"
my $config = {
    routes => [
        { path => '/api/users', methods => ['GET', 'POST'], handler => \&user_handler },
        { path => '/api/items', methods => ['GET'], handler => sub { shift->render(json => []) } },
    ],
    middleware => [
        { name => 'auth', config => { realm => 'api', exclude => ['/health'] } },
    ],
};
"#;
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn hashref_with_computed_keys() -> Result<(), String> {
    let code = r#"
my $table = {
    "prefix_$name" => $value,
    join('_', @parts) => $computed,
    lc($key) => $normalized,
};
"#;
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn arrayref_of_arrayrefs_matrix() -> Result<(), String> {
    let code = r#"
my $matrix = [
    [1, 0, 0, 0],
    [0, 1, 0, 0],
    [0, 0, 1, 0],
    [0, 0, 0, 1],
];
"#;
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn dispatch_table_with_complex_subs() -> Result<(), String> {
    let code = r#"
my %handlers = (
    create => sub {
        my ($self, %args) = @_;
        my $obj = $self->schema->resultset($args{type})->create(\%args);
        return $obj->id;
    },
    delete => sub {
        my ($self, %args) = @_;
        $self->schema->resultset($args{type})->find($args{id})->delete;
    },
);
"#;
    assert_clean_parse(code);
    Ok(())
}

// ===========================================================================
// 5. Postfix dereferencing ($hashref->%*, $arrayref->@*)
// ===========================================================================

#[test]
fn postfix_hash_deref() -> Result<(), String> {
    let code = "my %copy = $hashref->%*;";
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn postfix_array_deref_in_for() -> Result<(), String> {
    let code = "for my $item ($arrayref->@*) { process($item); }";
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn postfix_hash_slice() -> Result<(), String> {
    let code = "my @vals = $hashref->@{qw(foo bar baz)};";
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn postfix_deref_in_method_chain() -> Result<(), String> {
    let code = "my @items = $self->get_data->{results}->@*;";
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn postfix_deref_in_grep() -> Result<(), String> {
    let code = "my @active = grep { $_->{active} } $list->@*;";
    assert_clean_parse(code);
    Ok(())
}

// ===========================================================================
// 6. Chained ternary operators (edge cases beyond basic)
// ===========================================================================

#[test]
fn chained_ternary_with_method_calls() -> Result<(), String> {
    let code = r#"
my $label = $obj->is_admin   ? 'Admin'
          : $obj->is_mod     ? 'Moderator'
          : $obj->is_premium ? 'Premium'
          :                    'User';
"#;
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn ternary_in_string_interpolation_context() -> Result<(), String> {
    let code = r#"my $msg = "Status: " . ($ok ? "pass" : "fail") . "\n";"#;
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn ternary_in_hash_constructor() -> Result<(), String> {
    let code = r#"
my %opts = (
    verbose => $debug ? 1 : 0,
    output  => $file  ? $file : '-',
    format  => $json  ? 'json' : $xml ? 'xml' : 'text',
);
"#;
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn ternary_as_array_index() -> Result<(), String> {
    let code = "my $val = $array[$reverse ? -1 : 0];";
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn nested_ternary_with_parens() -> Result<(), String> {
    let code = "my $x = ($a > $b) ? ($c > $d ? $c : $d) : ($e > $f ? $e : $f);";
    assert_clean_parse(code);
    Ok(())
}

// ===========================================================================
// 7. Complex slice expressions
// ===========================================================================

#[test]
fn array_slice_with_range_and_negative() -> Result<(), String> {
    let code = "my @tail = @array[-3 .. -1];";
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn hash_slice_from_arrayref_keys() -> Result<(), String> {
    let code = "my @selected = @config{@{$wanted_keys}};";
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn array_slice_in_assignment() -> Result<(), String> {
    let code = "@array[0, 2, 4] = ('a', 'b', 'c');";
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn hash_slice_with_fat_comma_keys() -> Result<(), String> {
    let code = "my @vals = @hash{'key1', 'key2', 'key3'};";
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn arrayref_slice_via_arrow() -> Result<(), String> {
    let code = "my @items = $aref->@[0, 3, 5];";
    assert_clean_parse(code);
    Ok(())
}

// ===========================================================================
// 8. qw() in various contexts
// ===========================================================================

#[test]
fn qw_in_use_parent() -> Result<(), String> {
    let code = "use parent qw(Exporter Class::Accessor);";
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn qw_in_for_loop() -> Result<(), String> {
    let code = r#"
for my $method (qw(get post put delete patch)) {
    install_route($method);
}
"#;
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn qw_in_hash_slice() -> Result<(), String> {
    let code = "my @vals = @ENV{qw(HOME PATH USER SHELL)};";
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn qw_as_function_args() -> Result<(), String> {
    let code = "push @ISA, qw(Foo::Bar Baz::Qux);";
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn qw_with_alternate_delimiters() -> Result<(), String> {
    let code = "my @items = qw{alpha beta gamma};";
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn qw_in_list_assignment() -> Result<(), String> {
    let code = "my ($foo, $bar, $baz) = qw(one two three);";
    assert_clean_parse(code);
    Ok(())
}

// ===========================================================================
// 9. Prototyped subroutines
// ===========================================================================

#[test]
fn sub_with_scalar_prototype() -> Result<(), String> {
    let code = "sub myfunc ($) { return $_[0] * 2 }";
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn sub_with_block_prototype() -> Result<(), String> {
    let code = r#"
sub try (&) {
    my $code = shift;
    eval { $code->() };
    return $@;
}
"#;
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn sub_with_mixed_prototype() -> Result<(), String> {
    let code = r#"
sub reduce (&@) {
    my $code = shift;
    my $acc = shift;
    $acc = $code->($acc, $_) for @_;
    return $acc;
}
"#;
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn sub_with_optional_prototype() -> Result<(), String> {
    let code = "sub myopen (*;$) { open(my $fh, $_[1] // '<', $_[0]) }";
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn sub_with_empty_prototype() -> Result<(), String> {
    let code = "sub PI () { 3.14159265358979 }";
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn sub_with_backslash_prototype() -> Result<(), String> {
    let code = r#"
sub push_to (\@@) {
    my $aref = shift;
    push @$aref, @_;
}
"#;
    assert_clean_parse(code);
    Ok(())
}

// ===========================================================================
// 10. Multiline string concatenation (edge cases)
// ===========================================================================

#[test]
fn multiline_concat_with_method_calls() -> Result<(), String> {
    let code = r#"
my $html = '<div class="' . $self->css_class . '">'
         . '<h1>' . encode_entities($title) . '</h1>'
         . '<p>' . $self->render_body . '</p>'
         . '</div>';
"#;
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn multiline_concat_with_ternary() -> Result<(), String> {
    let code = r#"
my $sql = "SELECT * FROM users"
        . ($where ? " WHERE $where" : "")
        . ($order ? " ORDER BY $order" : "")
        . ($limit ? " LIMIT $limit" : "");
"#;
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn multiline_concat_with_join() -> Result<(), String> {
    let code = r#"
my $csv = join(",",
    $record->{name},
    $record->{email},
    $record->{phone} // '',
    $record->{address} // '',
) . "\n";
"#;
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn string_repetition_in_concat() -> Result<(), String> {
    let code = r#"
my $box = '+' . '-' x 78 . '+' . "\n"
        . '|' . ' ' x 78 . '|' . "\n"
        . '+' . '-' x 78 . '+' . "\n";
"#;
    assert_clean_parse(code);
    Ok(())
}

#[test]
fn heredoc_after_concat_operator() -> Result<(), String> {
    let code = r#"
my $msg = "Error at line $line:\n" . <<END;
    Details of the error go here.
    File: $file
END
"#;
    assert_clean_parse(code);
    Ok(())
}
