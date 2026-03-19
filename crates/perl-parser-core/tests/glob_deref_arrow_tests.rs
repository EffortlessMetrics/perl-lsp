//! Tests for *$self->{key} glob dereference patterns.
//! These are the primary driver of unclosed_paren errors in the corpus.

mod cpan_test_helpers;
use cpan_test_helpers::*;

// The core issue: *$self->{key} at various positions

#[test]
fn glob_deref_hash_in_if_condition() {
    // IO::Uncompress::Base L39 - first error in file
    assert_clean_parse(r#"if (defined *$self->{InputLength}) { return 0 }"#);
}

#[test]
fn glob_deref_hash_simple_stmt() {
    assert_clean_parse(r#"*$self->{key};"#);
}

#[test]
fn glob_deref_hash_in_assignment() {
    assert_clean_parse(r#"my $x = *$self->{key};"#);
}

#[test]
fn glob_deref_hash_in_return() {
    assert_clean_parse(r#"return *$self->{key};"#);
}

#[test]
fn glob_deref_hash_method_call() {
    // IO::Compress::Gzip L164
    assert_clean_parse(r#"return pack("V V", *$self->{Compress}->crc32());"#);
}

#[test]
fn glob_deref_hash_double_key() {
    // IO::Compress::Zip L114
    assert_clean_parse(r#"if (*$self->{ZipData}{Sparse}) { print "ok" }"#);
}

#[test]
fn glob_deref_hash_method_chain_in_pack() {
    assert_clean_parse(
        r#"return pack("V V", *$self->{Compress}->crc32(), *$self->{UnCompSize}->get32bit());"#,
    );
}

// *$glob{SLOT} patterns (from Carp.pm, autodie)

#[test]
fn glob_hash_slot() {
    // *$_{HASH}
    assert_clean_parse(r#"if (*$_{HASH}) { print "ok" }"#);
}

#[test]
fn glob_code_slot() {
    // *$_{CODE}
    assert_clean_parse(r#"return *$_{CODE};"#);
}

#[test]
fn glob_scalar_slot() {
    // ${*$_{SCALAR}}
    assert_clean_parse(r#"return ${*$_{SCALAR}};"#);
}

#[test]
fn glob_hash_slot_defined() {
    // defined(*$oldglob{$slot})
    assert_clean_parse(r#"next unless defined(*$oldglob{$slot});"#);
}

#[test]
fn glob_hash_slot_assign() {
    // *alias = *$oldglob{$slot};
    assert_clean_parse(r#"*alias = *$oldglob{$slot};"#);
}

// Full context patterns from real files

#[test]
fn io_uncompress_base_full_sub() {
    assert_clean_parse(
        r#"
sub smartRead {
    my $self = $_[0];
    my $out = $_[1];
    my $size = $_[2];
    $$out = "";
    my $offset = 0;
    my $status = 1;
    if (defined *$self->{InputLength}) {
        return 0 if *$self->{InputLengthRemaining} <= 0;
        $size = $size;
    }
}
"#,
    );
}

#[test]
fn carp_fetch_sub_full() {
    assert_clean_parse(
        r#"
sub _fetch_sub {
    my($pack, $sub) = @_;
    $pack .= '::';
    return unless exists($::{$pack});
    for ($::{$pack}) {
        return unless ref \$_ eq 'GLOB' && *$_{HASH} && exists $$_{$sub};
        for ($$_{$sub}) {
            return ref \$_ eq 'GLOB' ? *$_{CODE} : undef
        }
    }
}
"#,
    );
}
