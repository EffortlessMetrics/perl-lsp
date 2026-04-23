mod cpan_test_helpers;
use cpan_test_helpers::*;

// Issue #4167: postfix deref chain on method call result
// Perl: $obj->method()->[0]->name() is valid and should parse cleanly.

#[test]
fn test_method_result_array_deref() {
    // $obj->method()->[0]
    assert_clean_parse("my $item = $obj->get_items()->[0];");
}

#[test]
fn test_method_result_array_deref_then_method() {
    // $obj->method()->[0]->name()
    assert_clean_parse("my $name = $obj->get_items()->[0]->name();");
}

#[test]
fn test_method_result_hash_deref() {
    // $obj->method()->{key}
    assert_clean_parse("my $val = $obj->get_data()->{key};");
}

#[test]
fn test_method_result_hash_deref_then_method() {
    // $obj->method()->{key}->value()
    assert_clean_parse("my $v = $obj->get_data()->{key}->value();");
}

#[test]
fn test_method_result_chained_array_hash_deref() {
    // $obj->method()->[0]->{key}
    assert_clean_parse("my $v = $obj->get_items()->[0]->{name};");
}

#[test]
fn test_full_deref_chain() {
    // Full chain: method()->[0]->{key}->method()
    assert_clean_parse("my $x = $obj->method()->[0]->{key}->name();");
}
