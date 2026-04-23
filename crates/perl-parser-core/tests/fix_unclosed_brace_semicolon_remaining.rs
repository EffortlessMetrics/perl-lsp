mod cpan_test_helpers;
use cpan_test_helpers::*;

// Issue #2754: unclosed_brace_semicolon bucket — 'new' as hash subscript key
//
// When 'new' is used as a bareword hash subscript key (e.g. $h{new} or $ref->{new}),
// the parser was treating it as an indirect constructor call and consuming the '}' as
// part of the class name expression, causing "expected '}', found ';'" errors.
//
// Fix: primary.rs 'new' arm now checks if next token is '}' or '=>' and returns
// a bareword Identifier node instead of attempting to parse a constructor call.

// --- BUG CORE: direct hash subscript with 'new' as key ---

#[test]
fn test_hash_subscript_new_key() {
    // $h{new} was broken: 'new' consumed '}' as class name, causing parse error
    assert_clean_parse(r#"$h{new} = 1;"#);
}

#[test]
fn test_arrow_hash_subscript_new_key() {
    // $ref->{new} was broken: same root cause via parse_primary's "new" arm
    assert_clean_parse(r#"$ref->{new};"#);
}

#[test]
fn test_delete_arrow_hash_subscript_new() {
    // Real-world pattern from Moo.pm and Constructor.pm
    assert_clean_parse(r#"delete _getstash($target)->{new};"#);
}

// --- REAL-WORLD PATTERNS from the failing corpus files ---

// From Moo.pm line 156: delete _getstash($target)->{new}
#[test]
fn test_moo_delete_stash_new() {
    assert_clean_parse(
        r#"
    if (my $old = delete $Moo::MAKERS{$target}{constructor}) {
        $old->assert_constructor;
        delete _getstash($target)->{new};
        Moo->_constructor_maker_for($target)
           ->register_attribute_specs(%{$old->all_attribute_specs});
    }
    elsif (!$target->isa('Moo::Object')) {
        Moo->_constructor_maker_for($target);
    }
"#,
    );
}

// From Constructor.pm: sub new { delete _getstash(__PACKAGE__)->{new}; }
#[test]
fn test_constructor_pm_bootstrap_new() {
    assert_clean_parse(
        r#"
sub new {
    my $class = shift;
    delete _getstash(__PACKAGE__)->{new};
    bless $class->BUILDARGS(@_), $class;
}
"#,
    );
}

// From IO::Socket::SSL: $sess_cb{new}($ctx, sub { ... })
#[test]
fn test_ssl_hash_subscript_new_call() {
    // $sess_cb{new} as a coderef being invoked
    assert_clean_parse(
        r#"
    $sess_cb{new}($ctx, sub {
        my ($ctx, $session, $key) = @_;
        $cache->add_session($key, $session);
    });
"#,
    );
}

// --- FAT-ARROW AUTOQUOTING with 'new' ---

#[test]
fn test_fat_arrow_new_key() {
    // (new => 1) -- fat-arrow autoquotes 'new' to a string
    assert_clean_parse(r#"my %h = (new => 1);"#);
}

#[test]
fn test_hash_literal_new_key() {
    assert_clean_parse(r#"my %h = (new => 'MyClass');"#);
}

// --- REGRESSION: 'new' as constructor call must still work ---

#[test]
fn test_new_as_constructor_call() {
    // new ClassName(...) -- indirect method syntax, must still work
    assert_clean_parse(r#"my $obj = new Foo(1, 2);"#);
}

#[test]
fn test_new_with_left_paren() {
    // new(...) -- function-call form, must still work
    assert_clean_parse(r#"my $obj = new(1, 2);"#);
}

#[test]
fn test_class_new_method_call() {
    // ClassName->new(...) -- method call, must still work
    assert_clean_parse(r#"my $obj = Foo->new(1, 2);"#);
}

// --- RELATED: other bareword hash keys that were already working ---

#[test]
fn test_hash_key_m_still_works() {
    assert_clean_parse(r#"$h{m} = 1;"#);
}

#[test]
fn test_hash_key_s_still_works() {
    assert_clean_parse(r#"$h{s} = 1;"#);
}

#[test]
fn test_hash_key_do_still_works() {
    assert_clean_parse(r#"$h{do} = 1;"#);
}

#[test]
fn test_hash_key_eval_still_works() {
    assert_clean_parse(r#"$h{eval} = 1;"#);
}

// --- HASH SLICE with 'new' as one of the keys ---

#[test]
fn test_hash_slice_new_as_first_key() {
    // @h{new, other} — 'new' followed by comma inside a hash subscript
    // Without the comma guard this would try to parse 'new' as a constructor call.
    assert_clean_parse(r#"my @vals = @h{new, other};"#);
}

#[test]
fn test_delete_hash_slice_new_key() {
    // delete @h{new} — delete a single element whose key is 'new'
    assert_clean_parse(r#"delete $h{new};"#);
}
