mod cpan_test_helpers;
use cpan_test_helpers::*;

// ── print/say regression: they must still work in paren context ──────────────

#[test]
fn edge_print_stderr_in_parens() {
    // print STDERR "msg" inside parens — the original filehandle builtins must
    // continue to work after adding send to the indirect-object list.
    assert_clean_parse(r#"my $r = (print STDERR "msg");"#);
}

#[test]
fn edge_say_stderr_in_parens() {
    assert_clean_parse(r#"my $r = (say STDERR "msg");"#);
}

#[test]
fn edge_print_scalar_fh_in_parens() {
    assert_clean_parse(r#"my $r = (print $fh "msg");"#);
}

// ── send: comma-separated form must still work (not broken by indirect fix) ──

#[test]
fn edge_send_comma_args_stmt() {
    // Classic send with explicit commas — standard function call form.
    // Exercises the calls.rs RightParen/RightBracket terminator path.
    assert_clean_parse(r#"send $socket, $data, 0;"#);
}

#[test]
fn edge_send_comma_args_in_parens() {
    // send with commas inside parens — comma-separated args must not be eaten
    // by the indirect-object logic (which fires only when no comma follows
    // the first arg).
    assert_clean_parse(r#"my $n = (send $socket, $data, 0);"#);
}

// ── send: must not be mistaken for indirect call in hash/fat-arrow context ───

#[test]
fn edge_send_as_hash_key() {
    // send as fat-arrow key — autoquoted, NOT an indirect call
    assert_clean_parse(r#"my %h = (send => "value");"#);
}

// ── send: method call form must not be affected ──────────────────────────────

#[test]
fn edge_send_method_call() {
    // $obj->send("msg") is a method call, NOT an indirect-object builtin call
    assert_clean_parse(r#"$obj->send("msg");"#);
}

// ── send: explicit-paren form must continue to work ─────────────────────────

#[test]
fn edge_send_explicit_parens() {
    assert_clean_parse(r#"send($socket, $data, 0);"#);
}

// ── send: in block context (RightBrace terminator, fixed in calls.rs) ────────

#[test]
fn edge_send_last_in_block() {
    // RightBrace terminator was fixed in the ancestor PR (#2236).
    // This ensures the send addition to calls.rs doesn't break that.
    assert_clean_parse(r#"sub test { send $socket "msg" }"#);
}

// ── send: block-form socket (send { $sock } "msg") ───────────────────────────

#[test]
fn edge_send_block_form_socket() {
    // send with block-form socket — exercises the is_fh_builtin path (line 429)
    // and the block-form LeftBrace dispatch (line 723) added by this PR.
    assert_clean_parse(r#"send { $socket } "msg";"#);
}

#[test]
fn edge_send_block_form_in_parens() {
    // Block-form socket inside parens — exercises both the block-form path
    // and the RightParen terminator from calls.rs.
    assert_clean_parse(r#"my $n = (send { $socket } "msg");"#);
}

// ── print block form in parens: must not regress ─────────────────────────────

#[test]
fn edge_print_block_form_in_parens() {
    assert_clean_parse(r#"my $r = (print { $fh } "msg");"#);
}

// ── send nested inside a function call ───────────────────────────────────────

#[test]
fn edge_send_nested_in_call() {
    // send result used as argument to another function
    assert_clean_parse(r#"my $r = foo(send($socket, $data, 0));"#);
}

// ── send in arrayref (RightBracket terminator) ───────────────────────────────

#[test]
fn edge_send_in_arrayref_with_flags() {
    // send with flags argument inside arrayref — verifies that the trailing
    // comma-separated arg is parsed and the RightBracket terminates correctly.
    assert_clean_parse(r#"my $a = [send $sock "msg", 0];"#);
}
