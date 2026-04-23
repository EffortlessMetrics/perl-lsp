//! Test patterns in their actual file context to find the real failures.

mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn io_compress_gzip_mktrailer() {
    // This is the exact context from IO::Compress::Gzip
    assert_clean_parse(
        r#"
sub mkTrailer
{
    my $self = shift ;
    return pack("V V", *$self->{Compress}->crc32(),
                       *$self->{UnCompSize}->get32bit());
}
"#,
    );
}

#[test]
fn io_compress_zip_sparse_check() {
    assert_clean_parse(
        r#"
sub something {
    my $self = shift ;
    if (*$self->{ZipData}{Sparse} ) {
        my $inc = 1024 * 100 ;
        my $NULLS = ("\x00" x $inc) ;
    }
}
"#,
    );
}

#[test]
fn io_uncompress_base_input_length() {
    assert_clean_parse(
        r#"
sub something {
    if (defined *$self->{InputLength}) {
        return 0
            if *$self->{InputLengthRemaining} <= 0 ;
    }
}
"#,
    );
}

#[test]
fn digest_version_check() {
    assert_clean_parse(
        r#"
sub new {
    my $class = shift;
    ( $class, @args ) = @$class if ref($class);
    no strict 'refs';
    unless ( exists ${"$class\::"}{"VERSION"} ) {
        my $pm_file = $class . ".pm";
        $pm_file =~ s{::}{/}g;
    }
}
"#,
    );
}

#[test]
fn test_builder_todo() {
    assert_clean_parse(
        r#"
sub something {
    no warnings 'once';
    my $todo;
    $todo = ${"$cpkg\::TODO"} if $cpkg;
    $todo = ${"$epkg\::TODO"} if $epkg && !$todo;
}
"#,
    );
}

#[test]
fn base_pm_unhook() {
    // From base.pm - ref eq 'CODE' without parens
    assert_clean_parse(
        r#"sub base::__inc::unhook { @INC = grep !(ref eq 'CODE' && $_ == $_[0]), @INC }"#,
    );
}

#[test]
fn charnames_caller_subscript() {
    assert_clean_parse(
        r#"
sub something {
    my $ord = CORE::hex $1;
    return chr utf8::unicode_to_native($ord) if $ord <= 255
                                         || ! ((caller 0)[8] & $bytes::hint_bits);
}
"#,
    );
}

#[test]
fn warnings_caller_subscript() {
    assert_clean_parse(
        r#"
sub something {
    if ($has_level) {
        if (@_ != ($has_message ? 3 : 2)) {
            my $sub = (caller 1)[3];
        }
    }
}
"#,
    );
}

#[test]
fn net_smtp_foreach_ref() {
    assert_clean_parse(
        r#"
sub new {
    my $self = shift;
    my $type = ref($self) || $self;
    my ($host, %arg) = @_;
    $arg{Timeout} = 120 if ! defined $arg{Timeout};

    foreach my $h (@{ref($hosts) ? $hosts : [$hosts]}) {
        $obj = $type->SUPER::new(
            PeerAddr => ($host = $h),
        );
    }
}
"#,
    );
}

#[test]
fn compress_zlib_downgrade() {
    assert_clean_parse(
        r#"
sub gzwrite {
    $] >= 5.008 and (utf8::downgrade($_[0], 1)
        or croak "Wide character in gzwrite");

    my $status = $gz->write($_[0]) ;
}
"#,
    );
}

#[test]
fn extutils_install_estr() {
    assert_clean_parse(
        r#"
sub something {
    if (!$dry_run) {
        File::Copy::copy($from,$to)
            or _croak( _estr "ERROR: Cannot copy '$from' to '$to': $!" );
    }
}
"#,
    );
}

#[test]
fn extutils_locale_encoding() {
    assert_clean_parse(
        r#"
sub something {
    no warnings 'once';
    return ${"ENCODING_" . uc(shift)};
}
"#,
    );
}

#[test]
fn test2_clone_dclone() {
    assert_clean_parse(r#"sub clone { blessed($_[0])->new(@{dclone($_[0])}) }"#);
}

#[test]
fn test2_facet_class_new() {
    assert_clean_parse(
        r#"
sub something {
    my $type = reftype($data) or confess "Facet has a bad value";

    return $self->_facet_class($name)->new(%{dclone($data)})
        if $type eq 'HASH';
}
"#,
    );
}

#[test]
fn math_complex_exponent() {
    assert_clean_parse(
        r#"
sub something {
    $t = 1/(2 * $z) - 1/(8 * $z**3) + 1/(16 * $z**5) - 5/(128 * $z**7)
        if $t == 0;
    my $u = &log($t);
}
"#,
    );
}

#[test]
fn autodie_util_glob_slot() {
    assert_clean_parse(
        r#"
sub something {
    foreach my $slot (qw( SCALAR ARRAY HASH IO ) ) {
        next unless defined(*$oldglob{$slot});
        *alias = *$oldglob{$slot};
    }
}
"#,
    );
}

#[test]
fn autodie_hints_does() {
    assert_clean_parse(
        r#"
sub something {
    if ($hints_available) {
        print "ok";
    }
    elsif ( ${"${package}::DOES"}{HINTS_PROVIDER.""} ) {
        $hints_available = 1;
    }
}
"#,
    );
}

#[test]
fn carp_return_glob_scalar() {
    assert_clean_parse(
        r#"
sub something {
    return unless ref \$_ eq 'GLOB' && *$_{HASH} && exists $$_{$sub};
    return ${*$_{SCALAR}};
}
"#,
    );
}

#[test]
fn file_fetch_return_error() {
    assert_clean_parse(
        r#"
sub something {
    for my $i (1..5) {
        return 1 if $success;
    }
    return $self->_error("Fetch failed! Gave up after 5 tries");
}
"#,
    );
}

// Test closure over sub argument in different context
#[test]
fn extutils_locale_sub_closure() {
    assert_clean_parse(
        r#"
Memoize::memoize('_langinfo', NORMALIZER => sub {
    no warnings 'once';
    return ${"ENCODING_" . uc(shift)};
}, "locale");
"#,
    );
}
