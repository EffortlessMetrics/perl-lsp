//! CPAN Pattern Tests: Module / Import Patterns

mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn use_strict_warnings() {
    let code = "use strict;\nuse warnings;";
    assert_clean_parse(code);
}

#[test]
fn use_with_qw_import() {
    let code = "use List::Util qw(reduce first uniq);";
    assert_clean_parse(code);
}

#[test]
fn use_with_version() {
    let code = "use v5.36;";
    assert_clean_parse(code);
}

#[test]
fn use_parent() {
    let code = "use parent qw(Base::Class);";
    assert_clean_parse(code);
}

#[test]
fn use_base() {
    let code = "use base 'Exporter';";
    assert_clean_parse(code);
}

#[test]
fn require_module() {
    let code = "require Foo::Bar;";
    assert_clean_parse(code);
}

#[test]
fn do_file() {
    let code = "do 'config.pl';";
    assert_clean_parse(code);
}

#[test]
fn exporter_our() {
    let code = r#"
use Exporter 'import';
our @EXPORT_OK = qw(foo bar baz);
our %EXPORT_TAGS = (all => [qw(foo bar baz)]);
"#;
    assert_clean_parse(code);
}

#[test]
fn begin_block() {
    let code = r#"
BEGIN {
    push @INC, 'lib';
}
"#;
    assert_clean_parse(code);
    let ast = parse(code);
    let kinds = top_level_kinds(&ast);
    assert!(kinds.contains(&"PhaseBlock"), "expected PhaseBlock for BEGIN");
}

#[test]
fn end_block() {
    let code = r#"
END {
    cleanup();
}
"#;
    assert_clean_parse(code);
}

#[test]
fn package_with_version() {
    let code = "package My::Module 1.23;";
    assert_clean_parse(code);
}

// ===========================================================================
// use if pragma patterns
// ===========================================================================

/// Basic `use if` with string equality condition (Win32 compatibility).
#[test]
fn use_if_os_check() {
    let code = r#"use if $^O eq "MSWin32", "Win32";"#;
    assert_clean_parse(code);
}

/// `use if` with version comparison and fat arrow.
#[test]
fn use_if_version_fat_arrow() {
    let code = r"use if $] < 5.008 => 'IO::Scalar';";
    assert_clean_parse(code);
}

/// `use if` with a constant condition.
#[test]
fn use_if_constant_condition() {
    let code = "use if DEBUG, 'Data::Dumper';";
    assert_clean_parse(code);
}

/// Multiple `use if` statements in the same file.
#[test]
fn use_if_multiple_in_file() {
    let code = r#"
package Test;
use strict;
use warnings;
use if $^O eq "MSWin32", "Win32";
use if $^O eq "MSWin32", "Win32::Console";
use Carp;
1;
"#;
    assert_clean_parse(code);
}

/// `use if` doesn't interfere with regular `if` statements.
#[test]
fn use_if_doesnt_break_if_statements() {
    let code = r#"
use if $^O eq "MSWin32", "Win32";
sub foo {
    if ($x > 0) {
        return 1;
    }
    return 0;
}
"#;
    assert_clean_parse(code);
}

/// Regular `use parent` still works (keyword 'parent' is not affected).
#[test]
fn use_parent_regression() {
    let code = "use parent qw(Base::Class Other::Base);";
    assert_clean_parse(code);
}

mod use_constant_angle_bracket {
    use super::*;

    /// Data::Dumper pattern — the original failing file.
    #[test]
    fn data_dumper_is_pre_516_perl() {
        let code = "use constant IS_PRE_516_PERL => $] < 5.016;";
        assert_clean_parse(code);
    }

    #[test]
    fn use_constant_less_equal() {
        let code = "use constant IS_OLD => $] <= 5.020;";
        assert_clean_parse(code);
    }

    #[test]
    fn use_constant_greater_than() {
        let code = "use constant IS_MODERN => $] > 5.020;";
        assert_clean_parse(code);
    }

    #[test]
    fn use_constant_greater_equal() {
        let code = "use constant SUPPORTS_BOOLS => $] >= 5.036;";
        assert_clean_parse(code);
    }

    /// Simple scalar constant still works.
    #[test]
    fn use_constant_simple_value() {
        let code = "use constant MAX => 42;";
        assert_clean_parse(code);
    }

    /// String constant still works.
    #[test]
    fn use_constant_string_value() {
        let code = r#"use constant NAME => "Perl";"#;
        assert_clean_parse(code);
    }

    /// Angle bracket readline must still work.
    #[test]
    fn angle_bracket_readline_still_works() {
        let code = "my $line = <STDIN>;";
        assert_clean_parse(code);
    }

    /// Full Data::Dumper preamble pattern.
    #[test]
    fn data_dumper_preamble() {
        let code = r#"
use constant IS_PRE_516_PERL => $] < 5.016;
use constant SUPPORTS_CORE_BOOLS => defined &builtin::is_bool;
"#;
        assert_clean_parse(code);
    }
}
