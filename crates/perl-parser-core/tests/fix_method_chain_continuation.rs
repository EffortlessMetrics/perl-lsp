mod cpan_test_helpers;
use cpan_test_helpers::*;

/// Stricter clean-parse check that also catches (ERROR ...) nodes
/// which `assert_clean_parse` misses due to case sensitivity.
fn assert_no_errors(source: &str) {
    let ast = parse(source);
    let sexp = ast.to_sexp();
    let sexp_lower = sexp.to_lowercase();
    let markers = [
        "(error ",
        "(missing_expression",
        "(missing_statement",
        "(missing_identifier",
        "(missing_block",
    ];
    for marker in &markers {
        assert!(
            !sexp_lower.contains(marker),
            "Parse error found for source:\n{}\n\nsexp:\n{}",
            source,
            sexp,
        );
    }
}

// --- Basic multi-line method chains ---

#[test]
fn test_method_chain_multiline() {
    let source = r#"$obj->method1()
    ->method2()
    ->method3();"#;
    assert_no_errors(source);
}

#[test]
fn test_method_chain_multiline_no_parens() {
    let source = r#"$obj->method1
    ->method2
    ->method3;"#;
    assert_no_errors(source);
}

#[test]
fn test_method_chain_same_line() {
    let source = r#"$obj->method1()->method2()->method3();"#;
    assert_no_errors(source);
}

#[test]
fn test_method_chain_with_args() {
    let source = r#"$obj->method1("arg1")
    ->method2($arg2)
    ->method3();"#;
    assert_no_errors(source);
}

// --- Arrow dereference across lines ---

#[test]
fn test_method_chain_arrow_deref_multiline() {
    let source = r#"$ref->{key}
    ->{nested_key};"#;
    assert_no_errors(source);
}

#[test]
fn test_method_chain_arrow_array_deref_multiline() {
    let source = r#"$ref->[0]
    ->[1];"#;
    assert_no_errors(source);
}

// --- Method calls with complex arguments ---

#[test]
fn test_scalar_method_call() {
    let source = r#"scalar $dh->read;"#;
    assert_no_errors(source);
}

#[test]
fn test_return_method_chain() {
    let source = r#"return $sock->SUPER::connect(@_);"#;
    assert_no_errors(source);
}

#[test]
fn test_return_ternary_with_method() {
    let source = r#"return $sock->SUPER::connect(@_ == 1 ? shift : pack_sockaddr_in(@_));"#;
    assert_no_errors(source);
}

#[test]
fn test_chained_method_on_constructor() {
    let source = r#"My::Class->new()
    ->init()
    ->run();"#;
    assert_no_errors(source);
}

// --- Patterns from corpus: indirect method syntax with -> continuation ---

#[test]
fn test_method_on_do_block() {
    let source = r#"do { $class }->new();"#;
    assert_no_errors(source);
}

#[test]
fn test_method_chain_after_shift() {
    let source = r#"shift->method();"#;
    assert_no_errors(source);
}

#[test]
fn test_method_chain_after_paren_expr() {
    let source = r#"($obj || $default)->method();"#;
    assert_no_errors(source);
}

#[test]
fn test_super_method_with_ternary_arg() {
    let source = r#"sub connect {
    my $sock = shift;
    return $sock->SUPER::connect(@_ == 1 ? shift : pack_sockaddr_in(@_));
}"#;
    assert_no_errors(source);
}

#[test]
fn test_complex_chain_with_hash_deref() {
    let source = r#"$self->{config}
    ->{database}
    ->{host};"#;
    assert_no_errors(source);
}

#[test]
fn test_method_chain_mixed_deref() {
    let source = r#"$self->get_config()
    ->{items}
    ->[0]
    ->process();"#;
    assert_no_errors(source);
}

// --- Arrow in complex expression contexts ---

#[test]
fn test_arrow_in_ternary_branches() {
    let source = r#"$cond ? $a->method : $b->method;"#;
    assert_no_errors(source);
}

#[test]
fn test_arrow_after_array_ref_constructor() {
    let source = r#"[1, 2, 3]->[0];"#;
    assert_no_errors(source);
}

#[test]
fn test_arrow_after_hash_ref_constructor() {
    let source = r#"{key => "value"}->{key};"#;
    assert_no_errors(source);
}

#[test]
fn test_arrow_after_sub_ref() {
    let source = r#"sub { 42 }->();"#;
    assert_no_errors(source);
}

#[test]
fn test_method_chain_in_print() {
    let source = r#"print $obj->method1()->method2();"#;
    assert_no_errors(source);
}

#[test]
fn test_chained_method_with_list_arg() {
    let source = r#"$obj->push(@items)
    ->sort()
    ->first();"#;
    assert_no_errors(source);
}

#[test]
fn test_io_socket_inet_configure() {
    let source = r#"sub configure {
    my($sock, $arg) = @_;
    my($lport, $rport, $laddr, $raddr, $proto, $type);

    $arg->{LocalAddr} = $arg->{LocalHost}
        if exists $arg->{LocalHost} && !exists $arg->{LocalAddr};

    ($laddr, $lport, $proto) = _sock_info(
        $arg->{LocalAddr}, $arg->{LocalPort}, $arg->{Proto}
    );

    $sock->socket(AF_INET, $type, $proto) or
        return _error($sock, $!, "socket");

    return $sock;
}"#;
    assert_no_errors(source);
}

// --- Actual corpus patterns that trigger unexpected_arrow_expr ---

// Pattern 1: Glob dereference *$self followed by arrow hash access
// From IO::String, IO::Scalar, IO::Compress::Base::Common
#[test]
fn test_glob_deref_arrow_hash() {
    let source = r#"*$self->{buf} = $bufref;"#;
    assert_no_errors(source);
}

#[test]
fn test_glob_deref_arrow_hash_multiple() {
    let source = r#"*$self->{buf} = $bufref;
*$self->{pos} = 0;
*$self->{lno} = 0;"#;
    assert_no_errors(source);
}

#[test]
fn test_glob_deref_arrow_assign() {
    let source = r#"*$obj->{Closed} = 1;
*$obj->{Error} = $error_ref;
*$obj->{ErrorNo} = \$errno;"#;
    assert_no_errors(source);
}

// Pattern 2: print builtin consuming class name before ->
// From ExtUtils::Installed
#[test]
fn test_print_class_method_call() {
    let source = r#"print Data::Dumper->new([$self])->Sortkeys(1)->Indent(1)->Dump();"#;
    assert_no_errors(source);
}

// Pattern 3: Unary + on hashref constructor followed by arrow
// From Log::Dispatch::Output
#[test]
fn test_unary_plus_hashref_arrow() {
    let source = r#"+{@_}->{message} . "\n";"#;
    assert_no_errors(source);
}

#[test]
fn test_arrow_after_wantarray() {
    let source = r#"wantarray ? @list : $list[0];"#;
    assert_no_errors(source);
}

#[test]
fn test_method_chain_after_conditional_assignment() {
    let source = r#"my $obj = $factory->create();
$obj->configure()
    ->start();"#;
    assert_no_errors(source);
}
