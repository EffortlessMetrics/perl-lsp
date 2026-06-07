use perl_parser::Parser;

/// Helper to assert code parses successfully
fn assert_parses(code: &str) {
    use perl_tdd_support::must;
    let mut parser = Parser::new(code);
    must(parser.parse());
}

fn assert_parses_without_recovery_errors(code: &str) {
    use perl_tdd_support::must;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let sexp = ast.to_sexp();
    assert!(
        !sexp.contains("ERROR"),
        "Expected clean parse without ERROR nodes for `{code}`, got: {sexp}"
    );
}

/// Helper to assert code fails to parse
#[allow(dead_code)]
fn assert_parse_fails(code: &str) {
    let mut parser = Parser::new(code);
    let result = parser.parse();
    assert!(result.is_err(), "Expected parse to fail but got AST:\n{:?}", result.ok());
}

#[test]
fn print_scalar_in_simple_context() {
    // Basic print $var should work
    assert_parses("print $x;");
    assert_parses("print $x");
    assert_parses("{ print $x; }");
    assert_parses("if (1) { print $x; }");
}

#[test]
fn print_scalar_after_my_inside_if() {
    let code = r#"
my $y = 10;
if (1) {
    print $y;
}
"#;
    assert_parses(code);
}

#[test]
fn print_scalar_with_arithmetic() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"print $x + 1;"#;
    let mut parser = Parser::new(code);
    let ast = parser.parse();
    assert!(ast.is_ok(), "Failed to parse: print $x + 1");

    // Should parse as print($x + 1), NOT as indirect object
    let ast = ast?;
    let sexp = ast.to_sexp();
    assert!(
        !sexp.contains("indirect_call"),
        "Should not parse arithmetic as indirect object: {}",
        sexp
    );
    Ok(())
}

#[test]
fn print_scalar_with_string_concat() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"print $x . "s";"#;
    let mut parser = Parser::new(code);
    let ast = parser.parse();
    assert!(ast.is_ok(), "Failed to parse: print $x . \"s\"");

    // Should parse as print($x . "s"), NOT as indirect object
    let ast = ast?;
    let sexp = ast.to_sexp();
    assert!(
        !sexp.contains("indirect_call"),
        "Should not parse string concat as indirect object: {}",
        sexp
    );
    Ok(())
}

#[test]
fn print_indirect_object_still_works() {
    // These should parse as indirect object syntax
    assert_parses(r#"open($fh, '<', 'x.txt'); print $fh "hi\n";"#);
    assert_parses(r#"print STDOUT "hello";"#);
    assert_parses(r#"print STDERR "error", "\n";"#);
    assert_parses(r#"say $fh "message";"#);
}

#[test]
fn print_filehandle_then_variable_is_indirect() -> Result<(), Box<dyn std::error::Error>> {
    // Ensure: print $fh $x; is treated as indirect object form
    let code = r#"print $fh $x;"#;
    let mut parser = Parser::new(code);
    let ast = parser.parse();
    assert!(ast.is_ok(), "Failed to parse: print $fh $x");

    let ast = ast?;
    let sexp = ast.to_sexp();
    assert!(
        sexp.contains("indirect_call"),
        "print $fh $x should be treated as indirect object: {}",
        sexp
    );
    Ok(())
}

#[test]
fn print_scalar_vs_indirect_object() {
    // print $var; should NOT be treated as indirect object
    assert_parses("print $x;");
    assert_parses("print $x, $y;");
    assert_parses("print $array[0];");
    assert_parses("print $hash{key};");

    // print $fh ... with more args should be indirect object
    assert_parses(r#"print $fh "text";"#);
    assert_parses(r#"print $fh "text", "more";"#);
}

/// Regression #974: print $hash{key} / print $array[i] must parse as argument
/// expressions, not as indirect-object (filehandle + arg) syntax.
#[test]
fn print_hash_subscript_not_indirect_object() -> Result<(), Box<dyn std::error::Error>> {
    // Hash subscript: $hash{key} must never be treated as a filehandle
    let mut parser = Parser::new("print $config{host};");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(
        !sexp.contains("indirect_call"),
        "print $config{{host}} must NOT be indirect_call, got: {sexp}"
    );

    // Array subscript: $array[i] must never be treated as a filehandle
    let mut parser2 = Parser::new("print $array[0];");
    let ast2 = parser2.parse()?;
    let sexp2 = ast2.to_sexp();
    assert!(
        !sexp2.contains("indirect_call"),
        "print $array[0] must NOT be indirect_call, got: {sexp2}"
    );

    // Named-capture variable $+{name} must never be treated as a filehandle
    let mut parser3 = Parser::new("say $+{year};");
    let ast3 = parser3.parse()?;
    let sexp3 = ast3.to_sexp();
    assert!(
        !sexp3.contains("indirect_call"),
        "say $+{{year}} must NOT be indirect_call, got: {sexp3}"
    );

    // say with hash subscript — same code path as print
    let mut parser4 = Parser::new("say $hash{key};");
    let ast4 = parser4.parse()?;
    let sexp4 = ast4.to_sexp();
    assert!(
        !sexp4.contains("indirect_call"),
        "say $hash{{key}} must NOT be indirect_call, got: {sexp4}"
    );

    // printf with hash subscript
    let mut parser5 = Parser::new(r#"printf $hash{fmt}, $val;"#);
    let ast5 = parser5.parse()?;
    let sexp5 = ast5.to_sexp();
    assert!(
        !sexp5.contains("indirect_call"),
        "printf $hash{{fmt}}, $val must NOT be indirect_call, got: {sexp5}"
    );

    // Regression: legitimate indirect-object (filehandle + string) must still work
    let mut parser6 = Parser::new(r#"print $fh "text";"#);
    let ast6 = parser6.parse()?;
    let sexp6 = ast6.to_sexp();
    assert!(
        sexp6.contains("indirect_call"),
        "print $fh \"text\" MUST be indirect_call, got: {sexp6}"
    );

    // Regression: print $fh $x (variable) must still be indirect-object
    let mut parser7 = Parser::new("print $fh $x;");
    let ast7 = parser7.parse()?;
    let sexp7 = ast7.to_sexp();
    assert!(sexp7.contains("indirect_call"), "print $fh $x MUST be indirect_call, got: {sexp7}");

    Ok(())
}

/// Regression #974: arrow-chained subscripts must also parse as argument
/// expressions, not indirect-object syntax.
#[test]
fn print_arrow_chain_not_indirect_object() -> Result<(), Box<dyn std::error::Error>> {
    // $obj->{key} is an arrow dereference chain, not a filehandle
    let mut parser = Parser::new("print $self->{output};");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(
        !sexp.contains("indirect_call"),
        "print $self->{{output}} must NOT be indirect_call, got: {sexp}"
    );

    // $obj->[0] arrow-indexed array dereference, not a filehandle
    let mut parser2 = Parser::new("print $self->[0];");
    let ast2 = parser2.parse()?;
    let sexp2 = ast2.to_sexp();
    assert!(
        !sexp2.contains("indirect_call"),
        "print $self->[0] must NOT be indirect_call, got: {sexp2}"
    );

    Ok(())
}

#[test]
fn new_constructor_pattern() {
    assert_parses("new Class");
    assert_parses("new Class()");
    assert_parses("new Class('arg')");
    assert_parses("$obj = new Class;");
}

#[test]
fn new_qualified_constructor_indirect_call() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"my $obj = new IO::Handle $fh;"#;
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();

    assert!(
        sexp.contains("indirect_call"),
        "qualified constructor should parse as indirect call: {sexp}"
    );

    Ok(())
}

#[test]
fn statement_modifier_inside_block_if() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
    {
        my @array = (1, 2, 3);
        foreach my $item (@array) {
            print "$item\n" if $item > 1;
        }
    }
    "#;
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    // We accept the statement modifier node in the output
    assert!(
        sexp.contains("statement_modifier") || sexp.contains("(if "),
        "expected statement_modifier or if in output; got: {sexp}"
    );
    Ok(())
}

#[test]
fn statement_modifier_inside_block_for() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
    {
        my @arr = (1,2,3);
        print $_ for @arr;
    }
    "#;
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(
        sexp.contains("statement_modifier") || sexp.contains("(for ") || sexp.contains("(foreach "),
        "expected statement_modifier or for/foreach in output"
    );
    Ok(())
}

// Regression tests for declaration + control flow issues
#[test]
fn decl_then_if_allows_assignment() {
    let code = r#"my $x; if (1) { $x = 5; }"#;
    assert_parses(code);
}

#[test]
fn decl_then_if_allows_call() {
    let code = r#"my $x; if (1) { foo("bar"); }"#;
    assert_parses(code);
}

#[test]
fn decl_then_if_allows_print() {
    let code = r#"my $x; if (1) { print "hi"; }"#;
    assert_parses(code);
}

#[test]
fn decl_then_if_allows_postfix_if() {
    let code = r#"my @a=(1,2,3); if (1) { print "$_" if 1; }"#;
    assert_parses(code);
}

#[test]
fn decl_then_foreach_allows_postfix_if() {
    let code = r#"my @a=(1,2,3); foreach my $x (@a) { print "$x\n" if $x > 1; }"#;
    assert_parses(code);
}

#[test]
fn package_then_if_allows_assignment() {
    let code = r#"package Foo; if (1) { $x = 5; }"#;
    assert_parses(code);
}

#[test]
fn our_then_if_allows_assignment() {
    let code = r#"our $x; if (1) { $x = 5; }"#;
    assert_parses(code);
}

#[test]
fn decl_then_while_allows_assignment() {
    let code = r#"my $x; while (1) { $x = 5; last; }"#;
    assert_parses(code);
}

#[test]
fn decl_then_foreach_allows_print() {
    let code = r#"my $x; foreach my $y (@a) { print "hi"; }"#;
    assert_parses(code);
}

#[test]
fn multiple_semicolons_in_block() {
    let code = r#"{ print "hi";; print "bye";;; }"#;
    assert_parses(code);
}

#[test]
fn empty_statements_allowed() {
    let code = r#";;; print "hi"; ;;;"#;
    assert_parses(code);
}

#[test]
fn statement_modifier_in_foreach_with_prior_decl() {
    let code = r#"
    my @array = (1, 2, 3);
    foreach my $item (@array) {
        print "$item\n" if $item > 1;
    }
    "#;
    assert_parses(code);
}

#[test]
fn complex_foreach_with_modifiers() {
    let code = r#"
    sub test {
        for my $i (1..10) {
            print "$i\n" if $i % 2;
        }
    }
    "#;
    assert_parses(code);
}

#[test]
fn do_while_block_condition() {
    // Perl supports do { ... } while/until CONDITION;
    assert_parses_without_recovery_errors("do { $x++ } while $x < 10;");
    assert_parses_without_recovery_errors("do { $x-- } until $x == 0;");
}

#[test]
fn state_variable_declaration() {
    assert_parses("use feature 'state'; sub counter { state $x = 0; return ++$x; }");
}

#[test]
fn our_variable_list_declaration() {
    assert_parses_without_recovery_errors("our ($foo, @bar, %baz);");
}

#[test]
fn version_string_literal_expression() {
    assert_parses("my $v = v5.38.0;");
}

#[test]
fn probe_valid_constructs_for_clean_parse() {
    let cases = [
        "format STDOUT =\n@<<<\n$x\n.\n",
        "my $x = ${^GLOBAL_PHASE};",
        "my $x = do { 1 };",
        "my $x = eval { 1 };",
        "my $x = qx{echo hi};",
        "my $x = qr/foo/i;",
        "my $x = m{foo}i;",
        "my $x = s{foo}{bar}r;",
        "my $x = y/abc/xyz/;",
        "my $x = tr/abc/xyz/;",
        "my $x = <<'EOF';\nhello\nEOF\n",
        "sub f ($x, $y = 1) { return $x + $y; }",
        "my $x = bless {}, 'Pkg';",
        "given ($x) { when (1) { say 'one'; } default { say 'other'; } }",
        "my @x = map { $_ + 1 } @y;",
        "my @x = grep { $_ % 2 } @y;",
        "my $x :shared = 1;",
        "our $x :shared;",
        "local $\" = ',';",
        "my $guard = local $SIG{__WARN__} = sub { 1; };",
        "open my $fh, '<', $file or die $!;",
        "my $n = scalar @{ $arr_ref };",
        "my $n = $hash_ref->{k}->{nested};",
        "my $v = $obj->${\\(\"foo\" . \"method\")};",
        "use v5.36;",
        "no feature 'indirect';",
        "package Foo 1.23;",
        "if ($x) { } elsif ($y) { } else { }",
        "for (my $i = 0; $i < 10; $i++) { }",
        "while (my $line = <STDIN>) { chomp $line; }",
        "UNITCHECK { say 'unit'; }",
        "CHECK { say 'check'; }",
        "INIT { say 'init'; }",
        "END { say 'end'; }",
        "my $x = do FILE;",
        "my $x = $#{ $array_ref };",
        "my $x = $#array;",
        "my $x = ${^WARNING_BITS};",
        "my $x = prototype 'CORE::open';",
        "my $x = __PACKAGE__;",
        "my $x = __SUB__;",
        "my $x = __FILE__ . __LINE__;",
    ];
    for case in cases {
        assert_parses_without_recovery_errors(case);
    }
}

#[test]
fn statement_modifier_unless_and_while() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
    {
        my $x = 0;
        print "ok\n" unless $x;
        print "loop\n" while $x < 0;
    }
    "#;
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(sexp.contains("statement_modifier"), "expected statement_modifier nodes in output");
    Ok(())
}

#[test]
fn statement_modifier_nested_blocks() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
    sub test {
        {
            my $count = 10;
            print "Count: $count\n" if $count > 5;
            last if $count == 10;
        }
    }
    "#;
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(
        sexp.contains("statement_modifier"),
        "expected statement_modifier nodes in nested blocks"
    );
    Ok(())
}

#[test]
fn statement_modifier_with_complex_expression() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
    {
        my $x = 5;
        print $x * 2, "\n" if $x > 0 && $x < 10;
    }
    "#;
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(
        sexp.contains("statement_modifier"),
        "expected statement_modifier with complex expression"
    );
    Ok(())
}

#[test]
fn method_call_fat_arrow_with_builtin_bareword_key() {
    let code = r#"
my $emitter = Mojo::EventEmitter->new();
$emitter->on(accept => sub { 1 });
"#;
    assert_parses(code);
}

#[test]
fn common_module_bootstrap_patterns_parse_cleanly() {
    let cases = [
        "package My::Exporter; use parent 'Exporter'; our @EXPORT_OK = qw(foo bar);",
        "BEGIN { require Exporter; our @ISA = qw(Exporter); }",
        "use lib 'lib'; require My::Plugin; My::Plugin->import(qw(run));",
        "no strict 'refs'; *{\"My::Package::generated\"} = sub { return 1; };",
    ];
    for case in cases {
        assert_parses_without_recovery_errors(case);
    }
}

#[test]
fn real_world_argument_unpacking_patterns_parse_cleanly() {
    let cases = [
        "sub sum { my ($first, @rest) = @_; return $first + scalar @rest; }",
        "sub configure { my ($self, %opts) = @_; $self->{timeout} = $opts{timeout} // 30; }",
        "sub callback { my ($code, @args) = @_; return $code->(@args); }",
        "sub named { my ($class, $name, $value) = @_; return bless { name => $name, value => $value }, $class; }",
    ];
    for case in cases {
        assert_parses_without_recovery_errors(case);
    }
}

#[test]
fn nested_reference_and_slice_patterns_parse_cleanly() {
    let cases = [
        "my $value = $config->{database}{hosts}[0]{name};",
        "my @subset = @{$rows}[0, 2, 4];",
        "my %copy = %{$object->{metadata} || {}};",
        "my ($first, $last) = @{$names}[0, -1];",
    ];
    for case in cases {
        assert_parses_without_recovery_errors(case);
    }
}

#[test]
fn loop_control_with_labels_parse_cleanly() {
    let code = r#"
OUTER: for my $row (@rows) {
    INNER: for my $cell (@$row) {
        next INNER if !defined $cell;
        redo INNER if $cell eq 'retry';
        last OUTER if $cell eq 'stop';
    }
}
"#;
    assert_parses_without_recovery_errors(code);
}
