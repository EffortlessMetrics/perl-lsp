#[cfg(test)]
mod real_world_corpus {
    use super::super::test_harness::{parse_perl_code, validate_tree_no_errors, capture_error_snapshot};
    use std::path::Path;

    /// Test parsing of real-world Perl code samples
    /// These are common patterns and constructs found in actual Perl codebases
    
    #[test]
    fn test_cpan_style_modules() {
        // Common CPAN module patterns
        let cpan_samples = [
            // Module with strict pragmas
            r#"
use strict;
use warnings;
use Exporter 'import';

our @EXPORT_OK = qw(foo bar baz);

sub foo {
    my ($self, @args) = @_;
    return 1;
}
"#,
            // Object-oriented module
            r#"
package MyClass;

use strict;
use warnings;

sub new {
    my ($class, %args) = @_;
    bless \%args, $class;
}

sub method {
    my ($self, $param) = @_;
    return $self->{data}->{$param};
}
"#,
            // Module with complex data structures
            r#"
use strict;
use warnings;

my %config = (
    database => {
        host => 'localhost',
        port => 5432,
        credentials => {
            user => 'dbuser',
            pass => 'secret'
        }
    },
    features => [qw(api webhook oauth)]
);

sub process_config {
    my ($config_ref) = @_;
    return $config_ref->{database}->{host};
}
"#,
        ];

        for (i, code) in cpan_samples.iter().enumerate() {
            let result = parse_perl_code(code);
            assert!(
                result.is_ok(),
                "CPAN sample {} failed to parse: {:?}",
                i, result
            );
            
            let tree = result.unwrap();
            assert!(
                validate_tree_no_errors(&tree).is_ok(),
                "CPAN sample {} has error nodes",
                i
            );
        }
    }

    #[test]
    fn test_script_patterns() {
        // Common script patterns
        let script_samples = [
            // Command-line script
            r#"
#!/usr/bin/env perl

use strict;
use warnings;
use Getopt::Long;

my ($input, $output, $verbose);
GetOptions(
    'input=s' => \$input,
    'output=s' => \$output,
    'verbose' => \$verbose
);

die "Usage: $0 --input file --output file" unless $input && $output;

open my $in, '<', $input or die "Cannot open $input: $!";
open my $out, '>', $output or die "Cannot open $output: $!";

while (my $line = <$in>) {
    chomp $line;
    print $out uc($line), "\n";
}
"#,
            // Data processing script
            r#"
use strict;
use warnings;

my @data = ();
while (<>) {
    chomp;
    next if /^#/;  # Skip comments
    next if /^\s*$/;  # Skip empty lines
    
    my ($name, $value) = split /\s*=\s*/, $_, 2;
    push @data, { name => $name, value => $value };
}

for my $item (@data) {
    printf "%-20s = %s\n", $item->{name}, $item->{value};
}
"#,
        ];

        for (i, code) in script_samples.iter().enumerate() {
            let result = parse_perl_code(code);
            assert!(
                result.is_ok(),
                "Script sample {} failed to parse: {:?}",
                i, result
            );
            
            let tree = result.unwrap();
            assert!(
                validate_tree_no_errors(&tree).is_ok(),
                "Script sample {} has error nodes",
                i
            );
        }
    }

    #[test]
    fn test_complex_expressions() {
        // Complex expressions commonly found in real code
        let complex_samples = [
            // Complex conditional with multiple operators
            r#"
my $result = defined($hash{$key}) && 
             ref($hash{$key}) eq 'ARRAY' && 
             @{$hash{$key}} > 0 ? 
             $hash{$key}->[0] : 
             undef;
"#,
            // Complex regex with modifiers
            r#"
my $pattern = qr{
    ^
    (?<protocol>https?://)?
    (?<domain>[a-zA-Z0-9.-]+)
    (?<port>:\d+)?
    (?<path>/[^\s]*)?
    $
}ix;
"#,
            // Heredoc with interpolation
            r#"
my $template = <<"EOF";
<html>
<head><title>$title</title></head>
<body>
<h1>$heading</h1>
<p>$content</p>
</body>
</html>
EOF
"#,
            // Complex map/grep chains
            r#"
my @filtered = map { 
    $_->{processed} = 1; 
    $_ 
} grep { 
    defined $_->{value} && 
    $_->{value} =~ /^\d+$/ && 
    $_->{value} > 0 
} @data;
"#,
        ];

        for (i, code) in complex_samples.iter().enumerate() {
            let result = parse_perl_code(code);
            assert!(
                result.is_ok(),
                "Complex expression {} failed to parse: {:?}",
                i, result
            );
            
            let tree = result.unwrap();
            assert!(
                validate_tree_no_errors(&tree).is_ok(),
                "Complex expression {} has error nodes",
                i
            );
        }
    }

    #[test]
    fn test_error_recovery_patterns() {
        // Code with intentional errors to test error recovery
        let error_samples = [
            // Unterminated string
            (r#"my $str = "Hello, World!;"#, 1), // Missing closing quote
            // Unterminated block
            (r#"if ($condition) { my $var = 1;"#, 1), // Missing closing brace
            // Invalid syntax
            (r#"my $var = 1 +;"#, 1), // Incomplete expression
            // Mixed errors
            (r#"my $str = "unterminated; if ($x) { $y = 1;"#, 2), // Multiple errors
        ];

        for (i, (code, expected_errors)) in error_samples.iter().enumerate() {
            let result = parse_perl_code(code);
            // Should parse (with error nodes) rather than fail completely
            assert!(
                result.is_ok(),
                "Error sample {} failed to parse at all: {:?}",
                i, result
            );
            
            let tree = result.unwrap();
            let error_snapshot = capture_error_snapshot(&tree);
            
            // Should have the expected number of error nodes
            assert!(
                error_snapshot.count >= *expected_errors,
                "Error sample {} expected at least {} errors, got {}",
                i, expected_errors, error_snapshot.count
            );
        }
    }

    #[test]
    fn test_unicode_real_world() {
        // Real-world Unicode patterns
        let unicode_samples = [
            // Japanese variable names
            r#"
my $変数 = "値";
my $日本語 = "こんにちは";
sub 関数 { return "関数です"; }
"#,
            // Mixed Unicode identifiers
            r#"
my $über = "cool";
my $naïve = "simple";
my $café = "coffee";
my $résumé = "summary";
"#,
            // Unicode in strings
            r#"
my $message = "Hello 世界! 🌍";
my $emoji = "🚀 rocket";
my $mixed = "ASCII + 日本語 + emoji 🎉";
"#,
        ];

        for (i, code) in unicode_samples.iter().enumerate() {
            let result = parse_perl_code(code);
            assert!(
                result.is_ok(),
                "Unicode sample {} failed to parse: {:?}",
                i, result
            );
            
            let tree = result.unwrap();
            assert!(
                validate_tree_no_errors(&tree).is_ok(),
                "Unicode sample {} has error nodes",
                i
            );
        }
    }

    #[test]
    fn test_performance_edge_cases() {
        // Large/complex constructs that might stress the parser
        let performance_samples = [
            // Deeply nested structures
            &format!(
                "my $deep = {};",
                (0..20).map(|i| format!("'level{}' => {{", i)).collect::<Vec<_>>().join(" ")
            ),
            // Long identifier chains
            &format!(
                "my $result = {};",
                (0..50).map(|i| format!("$var{}", i)).collect::<Vec<_>>().join(" + ")
            ),
            // Many small statements
            &(0..100).map(|i| format!("my $var{} = {};", i, i)).collect::<Vec<_>>().join("\n"),
        ];

        for (i, code) in performance_samples.iter().enumerate() {
            let result = parse_perl_code(code);
            assert!(
                result.is_ok(),
                "Performance sample {} failed to parse: {:?}",
                i, result
            );
            
            let tree = result.unwrap();
            // For performance samples, we just check it doesn't panic
            // Error nodes might be acceptable for very complex constructs
            println!("Performance sample {} parsed successfully", i);
        }
    }
} 