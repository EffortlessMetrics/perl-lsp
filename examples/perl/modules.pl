#!/usr/bin/perl
# examples/perl/modules.pl
#
# Demonstrates: use, require, use lib, nested packages, Exporter,
#               module-level constants, and cross-file navigation patterns.
#
# LSP features exercised:
#   - go-to-def   : jump from 'use MyModule' to the module source file
#   - completion  : imported symbols appear in completions
#   - hover       : show module docs / function signatures from POD
#   - rename      : rename an exported sub across all callers

use strict;
use warnings;
use feature 'say';

# ---------------------------------------------------------------------------
# 1. Core module usage (hover shows module synopsis from docs)
# ---------------------------------------------------------------------------

use List::Util qw(sum max min first reduce any all none);
use Scalar::Util qw(blessed reftype looks_like_number);
use POSIX qw(floor ceil);
use Carp qw(croak confess carp cluck);

my @numbers = (3, 1, 4, 1, 5, 9, 2, 6);

say "sum:  " . sum(@numbers);
say "max:  " . max(@numbers);
say "min:  " . min(@numbers);
say "first >4: " . (first { $_ > 4 } @numbers);

my $product = reduce { $a * $b } @numbers;
say "product: $product";

say "any >8: "  . ((any  { $_ > 8 } @numbers) ? 'yes' : 'no');
say "all >0: "  . ((all  { $_ > 0 } @numbers) ? 'yes' : 'no');
say "none <0: " . ((none { $_ < 0 } @numbers) ? 'yes' : 'no');

say "floor(3.7) = " . floor(3.7);
say "ceil(3.2)  = " . ceil(3.2);

# ---------------------------------------------------------------------------
# 2. Inline package declarations (multiple packages in one file)
# ---------------------------------------------------------------------------

package MathUtils;

use Exporter 'import';
use Carp qw(croak);
our @EXPORT_OK = qw(factorial fibonacci gcd);

sub factorial {
    my ($n) = @_;
    croak "factorial: n must be non-negative" if $n < 0;
    return 1 if $n <= 1;
    return $n * factorial($n - 1);
}

sub fibonacci {
    my ($n) = @_;
    return $n if $n <= 1;
    my ($a, $b) = (0, 1);
    for (2 .. $n) {
        ($a, $b) = ($b, $a + $b);
    }
    return $b;
}

sub gcd {
    my ($a, $b) = @_;
    ($a, $b) = ($b, $a % $b) while $b;
    return $a;
}

package StringUtils;

use Exporter 'import';
our @EXPORT_OK = qw(trim slugify word_count);

sub trim {
    my ($str) = @_;
    $str =~ s/^\s+|\s+$//g;
    return $str;
}

sub slugify {
    my ($str) = @_;
    $str = lc $str;
    $str =~ s/[^a-z0-9]+/-/g;
    $str =~ s/^-|-$//g;
    return $str;
}

sub word_count {
    my ($str) = @_;
    my @words = split /\s+/, StringUtils::trim($str);
    return scalar @words;
}

# ---------------------------------------------------------------------------
# 3. Back to main package -- import from inline packages
# ---------------------------------------------------------------------------

package main;

# go-to-def on MathUtils::factorial jumps to its definition above
say "5! = " . MathUtils::factorial(5);
say "fib(10) = " . MathUtils::fibonacci(10);
say "gcd(48,18) = " . MathUtils::gcd(48, 18);

my $raw = "  Hello, World!  How are you?  ";
say "trimmed: '" . StringUtils::trim($raw) . "'";
say "slug:    '" . StringUtils::slugify('Hello World! (2024)') . "'";
say "words:   " . StringUtils::word_count($raw);

# ---------------------------------------------------------------------------
# 4. Nested packages (namespaces)
# ---------------------------------------------------------------------------

package Config::Database;

use constant {
    HOST    => 'localhost',
    PORT    => 5432,
    DB_NAME => 'myapp',
    TIMEOUT => 30,
};

sub dsn {
    return sprintf 'dbi:Pg:host=%s;port=%d;dbname=%s',
        HOST, PORT, DB_NAME;
}

package Config::Cache;

use constant {
    HOST => 'localhost',
    PORT => 6379,
    TTL  => 3600,
};

package main;

# Fully qualified name access -- go-to-def navigates to nested package
say "DB DSN:   " . Config::Database::dsn();
say "DB port:  " . Config::Database::PORT;
say "Cache port: " . Config::Cache::PORT;

# ---------------------------------------------------------------------------
# 5. Scalar::Util introspection (hover shows what each function returns)
# ---------------------------------------------------------------------------

my $obj      = bless { name => 'test' }, 'Config::Database';
my $arrayref = [1, 2, 3];
my $coderef  = sub { 42 };

say "blessed: "  . (blessed($obj) // '(not blessed)');
say "reftype: "  . (reftype($arrayref) // '(not a ref)');
say "is code: "  . (reftype($coderef) eq 'CODE' ? 'yes' : 'no');
say "is_num: "   . looks_like_number('3.14');

# ---------------------------------------------------------------------------
# 6. Dynamic require (cross-file navigation -- go-to-def follows the string)
# ---------------------------------------------------------------------------

# The LSP go-to-def feature resolves module paths for require/use.
# When the module file exists, you can navigate directly to it.

# require 'SomeModule.pm';        # relative require -- LSP resolves via @INC
# require Module::Name;           # bareword require -- LSP resolves to Module/Name.pm

# use lib to add custom search paths:
# use lib '/path/to/local/libs';
# use lib qw(lib vendor/lib);

say "All module examples complete.";
