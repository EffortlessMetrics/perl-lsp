//! Convert existing test files to unified test format

use crate::TestCase;
use std::fs;
use std::path::Path;
use anyhow::Result;

/// Convert perl-parser test files
pub fn convert_perl_parser_tests() -> Result<Vec<TestCase>> {
    let mut tests = vec![];
    
    // These are based on the test files we've seen in perl-parser
    tests.push(TestCase {
        name: "perl_parser::variables".to_string(),
        input: r#"
my $scalar = 42;
my @array = (1, 2, 3);
my %hash = (key => 'value');
our $global = 'global';
local $package::var = 'local';
state $persistent = 0;
"#.trim().to_string(),
        description: Some("Variable declarations".to_string()),
        should_parse: true,
        expected_sexp: None,
    });
    
    tests.push(TestCase {
        name: "perl_parser::operators".to_string(),
        input: r#"
$a + $b;
$x - $y;
$p * $q;
$m / $n;
$i % $j;
$base ** $exp;
$str . $str2;
$str x 3;
"#.trim().to_string(),
        description: Some("Basic operators".to_string()),
        should_parse: true,
        expected_sexp: None,
    });
    
    tests.push(TestCase {
        name: "perl_parser::control_flow".to_string(),
        input: r#"
if ($x) {
    print "true\n";
} elsif ($y) {
    print "y is true\n";
} else {
    print "false\n";
}

unless ($error) {
    process();
}

while ($i < 10) {
    $i++;
}

for (my $i = 0; $i < 10; $i++) {
    print "$i\n";
}

foreach my $item (@list) {
    process($item);
}
"#.trim().to_string(),
        description: Some("Control flow structures".to_string()),
        should_parse: true,
        expected_sexp: None,
    });
    
    tests.push(TestCase {
        name: "perl_parser::regex".to_string(),
        input: r#"
$text =~ /pattern/;
$text !~ /pattern/;
$text =~ s/old/new/g;
$text =~ tr/a-z/A-Z/;
$text =~ m{pattern}x;
$text =~ s{old}{new}gi;
"#.trim().to_string(),
        description: Some("Regular expressions".to_string()),
        should_parse: true,
        expected_sexp: None,
    });
    
    tests.push(TestCase {
        name: "perl_parser::subroutines".to_string(),
        input: r#"
sub simple {
    return 42;
}

sub with_params {
    my ($x, $y) = @_;
    return $x + $y;
}

sub with_signature ($x, $y) {
    return $x + $y;
}

my $anon = sub {
    return "anonymous";
};
"#.trim().to_string(),
        description: Some("Subroutine definitions".to_string()),
        should_parse: true,
        expected_sexp: None,
    });
    
    tests.push(TestCase {
        name: "perl_parser::modern_perl".to_string(),
        input: r#"
use v5.36;
use strict;
use warnings;

try {
    risky_operation();
} catch ($e) {
    warn "Error: $e";
}

defer {
    cleanup();
}

class Point {
    field $x;
    field $y;
    
    method new($x, $y) {
        $self->{x} = $x;
        $self->{y} = $y;
        return $self;
    }
}
"#.trim().to_string(),
        description: Some("Modern Perl features".to_string()),
        should_parse: true,
        expected_sexp: None,
    });
    
    tests.push(TestCase {
        name: "perl_parser::references".to_string(),
        input: r#"
my $scalar_ref = \$scalar;
my $array_ref = \@array;
my $hash_ref = \%hash;
my $sub_ref = \&subroutine;

$$scalar_ref = 42;
$array_ref->[0] = 1;
$hash_ref->{key} = 'value';
$sub_ref->();

my $nested = $hash_ref->{data}->[0]->{name};
"#.trim().to_string(),
        description: Some("References and dereferencing".to_string()),
        should_parse: true,
        expected_sexp: None,
    });
    
    tests.push(TestCase {
        name: "perl_parser::strings".to_string(),
        input: r#"
my $single = 'single quoted';
my $double = "double quoted with $interpolation";
my $q_string = q{custom delimiter};
my $qq_string = qq{interpolated $var};
my $qw_list = qw(word1 word2 word3);
my $qr_regex = qr/pattern/i;
my $heredoc = <<'EOF';
Heredoc content
Multiple lines
EOF
"#.trim().to_string(),
        description: Some("String types and quoting".to_string()),
        should_parse: true,
        expected_sexp: None,
    });
    
    tests.push(TestCase {
        name: "perl_parser::special_variables".to_string(),
        input: r#"
$_;
@_;
$@;
$!;
$$;
$0;
$1;
@ARGV;
%ENV;
$^O;
$#array;
"#.trim().to_string(),
        description: Some("Special Perl variables".to_string()),
        should_parse: true,
        expected_sexp: None,
    });
    
    tests.push(TestCase {
        name: "perl_parser::packages".to_string(),
        input: r#"
package Foo;

sub new {
    my $class = shift;
    return bless {}, $class;
}

package Bar {
    use parent 'Foo';
    
    sub method {
        my $self = shift;
        $self->SUPER::method();
    }
}

Foo->new();
Bar->new()->method();
"#.trim().to_string(),
        description: Some("Package and OO features".to_string()),
        should_parse: true,
        expected_sexp: None,
    });
    
    Ok(tests)
}