//! Tests for issue #1704: unclosed_paren errors from complex arg expressions.
//! Patterns extracted from actual corpus files that fail with this error.

mod cpan_test_helpers;
use cpan_test_helpers::*;

// === Pattern 1: *$self->{key} (glob deref without ${}) ===
// From IO::Compress::Gzip, IO::Compress::Zip, IO::Uncompress::Base

#[test]
fn glob_deref_arrow_hash() {
    // *$self->{Compress}->crc32()
    assert_clean_parse(r#"my $crc = *$self->{Compress}->crc32();"#);
}

#[test]
fn glob_deref_arrow_hash_in_pack() {
    // pack("V V", *$self->{Compress}->crc32(), *$self->{UnCompSize}->get32bit())
    assert_clean_parse(
        r#"return pack("V V", *$self->{Compress}->crc32(), *$self->{UnCompSize}->get32bit());"#,
    );
}

#[test]
fn glob_deref_arrow_hash_in_if() {
    // if (*$self->{ZipData}{Sparse})
    assert_clean_parse(r#"if (*$self->{ZipData}{Sparse}) { print "sparse" }"#);
}

#[test]
fn glob_deref_arrow_hash_defined() {
    // if (defined *$self->{InputLength})
    assert_clean_parse(r#"if (defined *$self->{InputLength}) { return 0 }"#);
}

#[test]
fn glob_deref_hash_access_no_braces() {
    // *$oldglob{$slot}
    assert_clean_parse(r#"next unless defined(*$oldglob{$slot});"#);
}

#[test]
fn glob_deref_hash_assign() {
    // *alias = *$oldglob{$slot};
    assert_clean_parse(r#"*alias = *$oldglob{$slot};"#);
}

// === Pattern 2: Symbolic dereference with string expressions ===
// From Digest.pm, Test::Builder, autodie::hints

#[test]
fn symbolic_deref_hash_version() {
    // exists ${"$class\::"}{"VERSION"}
    assert_clean_parse(r#"unless (exists ${"$class\::"}{"VERSION"}) { print "no version" }"#);
}

#[test]
fn symbolic_deref_hash_todo() {
    // ${"$cpkg\::TODO"} if $cpkg
    assert_clean_parse(r#"$todo = ${"$cpkg\::TODO"} if $cpkg;"#);
}

#[test]
fn symbolic_deref_hash_does() {
    // ${"${package}::DOES"}{HINTS_PROVIDER.""}
    assert_clean_parse(r#"if (${"${package}::DOES"}{HINTS_PROVIDER.""}) { print "ok" }"#);
}

#[test]
fn symbolic_deref_encoding() {
    // return ${"ENCODING_" . uc(shift)};
    assert_clean_parse(r#"return ${"ENCODING_" . uc(shift)};"#);
}

// === Pattern 3: (caller N)[index] ===
// From charnames.pm, warnings.pm

#[test]
fn caller_subscript() {
    // (caller 0)[8]
    assert_clean_parse(r#"my $bits = (caller 0)[8];"#);
}

#[test]
fn caller_subscript_in_condition() {
    // return chr ... || ! ((caller 0)[8] & $bytes::hint_bits);
    assert_clean_parse(
        r#"return chr $ord if $ord <= 255 || !((caller 0)[8] & $bytes::hint_bits);"#,
    );
}

#[test]
fn caller_1_subscript() {
    // my $sub = (caller 1)[3];
    assert_clean_parse(r#"my $sub = (caller 1)[3];"#);
}

// === Pattern 4: ref eq without parens ===
// From base.pm

#[test]
fn ref_eq_string() {
    // ref eq 'CODE'
    assert_clean_parse(r#"my @filtered = grep !(ref eq 'CODE' && $_ == $_[0]), @INC;"#);
}

#[test]
fn ref_eq_simple() {
    assert_clean_parse(r#"print "ok" if ref eq 'HASH';"#);
}

// === Pattern 5: Complex nested calls in args ===
// From Test2::API

#[test]
fn blessed_new_with_dclone() {
    // blessed($_[0])->new(@{dclone($_[0])})
    assert_clean_parse(r#"sub clone { blessed($_[0])->new(@{dclone($_[0])}) }"#);
}

#[test]
fn facet_class_new_with_dclone() {
    // $self->_facet_class($name)->new(%{dclone($data)})
    assert_clean_parse(
        r#"return $self->_facet_class($name)->new(%{dclone($data)}) if $type eq 'HASH';"#,
    );
}

// === Pattern 6: Exponentiation in complex math ===
// From Math::Complex

#[test]
fn complex_math_with_exponent() {
    // 1/(2 * $z) - 1/(8 * $z**3)
    assert_clean_parse(r#"$t = 1/(2 * $z) - 1/(8 * $z**3) + 1/(16 * $z**5);"#);
}

#[test]
fn exponent_chain() {
    assert_clean_parse(r#"my $x = $z**3 + $z**5 - $z**7;"#);
}

// === Pattern 7: Complex foreach with ref ternary ===
// From Net::SMTP

#[test]
fn foreach_ref_ternary() {
    // foreach my $h (@{ref($hosts) ? $hosts : [$hosts]})
    assert_clean_parse(r#"foreach my $h (@{ref($hosts) ? $hosts : [$hosts]}) { print $h }"#);
}

// === Pattern 8: utf8::downgrade with "or croak" ===
// From Compress::Zlib

#[test]
fn utf8_downgrade_or_croak() {
    // $] >= 5.008 and (utf8::downgrade($_[0], 1) or croak "Wide character in gzwrite");
    assert_clean_parse(
        r#"$] >= 5.008 and (utf8::downgrade($_[0], 1) or croak "Wide character in gzwrite");"#,
    );
}

// === Pattern 9: _estr helper in function call ===
// From ExtUtils::Install

#[test]
fn estr_in_croak() {
    // _croak( _estr "ERROR: Cannot copy '$from' to '$to': $!" );
    assert_clean_parse(r#"_croak(_estr "ERROR: Cannot copy '$from' to '$to': $!");"#);
}

// === Pattern 10: reftype in condition ===
// From Test2::API::InterceptResult::Event

#[test]
fn reftype_or_confess() {
    assert_clean_parse(r#"my $type = reftype($data) or confess "Facet has a bad value";"#);
}

// === Already-passing patterns (from issue description, kept as regression tests) ===

#[test]
fn use_constant_hashref() {
    assert_clean_parse(
        r#"
use constant {
    UNICODE_VERSION => "15.0.0",
    CLDR_VERSION    => "42",
};
"#,
    );
}

#[test]
fn return_list_with_or_undef() {
    assert_clean_parse(r#"return ($addr || undef, $port || undef, $proto || undef);"#);
}

#[test]
fn xsloader_load_with_package() {
    assert_clean_parse(r#"XSLoader::load(__PACKAGE__, $XS_VERSION);"#);
}
