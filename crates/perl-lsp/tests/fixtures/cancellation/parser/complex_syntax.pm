package ComplexSyntax;
# Complex Perl syntax patterns for comprehensive cancellation testing
# Tests edge cases and boundary conditions during parsing cancellation

use strict;
use warnings;
use feature qw(say state switch);

# Quote operators and delimiter variations for cancellation testing
sub quote_operator_scenarios {
    my ($input) = @_;

    # Test cancellation during q{} parsing
    my $single_quoted = q{This is a 'quoted' string with {nested} braces};

    # Test cancellation during qq[] parsing
    my $double_quoted = qq[This is a "quoted" string with [nested] brackets and $input interpolation];

    # Test cancellation during qr<> parsing
    my $regex = qr<pattern_\w+_with_<nested>_angles>;

    # Test cancellation during qw() parsing
    my @word_list = qw(one two three four five six seven eight nine ten);

    # Test cancellation during qx// parsing
    my $command_output = qx/echo "Command execution for testing"/;

    return {
        single => $single_quoted,
        double => $double_quoted,
        regex => $regex,
        words => \@word_list,
        command => $command_output,
    };
}

# Heredoc scenarios for cancellation during multi-line parsing
sub heredoc_parsing_scenarios {
    my ($variable) = @_;

    # Test cancellation during heredoc parsing
    my $simple_heredoc = <<'END_SIMPLE';
This is a simple heredoc
with multiple lines
for cancellation testing
END_SIMPLE

    # Test cancellation during interpolated heredoc parsing
    my $interpolated_heredoc = <<"END_INTERPOLATED";
This heredoc contains $variable interpolation
and can be cancelled during parsing
Line count: multiple
END_INTERPOLATED

    # Test cancellation during indented heredoc parsing
    my $indented_heredoc = <<~'END_INDENTED';
        This is an indented heredoc
        with consistent indentation
        for cancellation testing scenarios
        END_INDENTED

    return {
        simple => $simple_heredoc,
        interpolated => $interpolated_heredoc,
        indented => $indented_heredoc,
    };
}

# Complex regex patterns for cancellation testing
sub regex_parsing_scenarios {
    my ($text) = @_;

    # Test cancellation during complex regex parsing
    my $email_pattern = qr/
        (?<username>[a-zA-Z0-9._%+-]+)
        @
        (?<domain>[a-zA-Z0-9.-]+\.[a-zA-Z]{2,})
    /x;

    # Test cancellation during substitution with lookahead/lookbehind
    $text =~ s/(?<=\b)(\w+)(?=\s+important)/HIGHLIGHTED_$1/g;

    # Test cancellation during complex character classes
    $text =~ s/[^\p{L}\p{N}\s\-_.,!?;:()[\]{}'"]/SPECIAL_CHAR/g;

    # Test cancellation during named captures
    if ($text =~ /(?<year>\d{4})-(?<month>\d{2})-(?<day>\d{2})/) {
        my $date_parts = {
            year => $+{year},
            month => $+{month},
            day => $+{day},
        };
        return $date_parts;
    }

    return { processed => $text };
}

# Anonymous subroutines and closures for cancellation testing
sub closure_scenarios {
    my ($data) = @_;

    # Test cancellation during anonymous subroutine parsing
    my $processor = sub {
        my ($item) = @_;
        return "processed_$item";
    };

    # Test cancellation during closure with captured variables
    my $multiplier = 3;
    my $multiply_closure = sub {
        my ($value) = @_;
        return $value * $multiplier;
    };

    # Test cancellation during map with anonymous subroutine
    my @results = map {
        my $item = $_;
        my $processed = $processor->($item);
        $multiply_closure->($processed);
    } @$data;

    return \@results;
}

# Reference and dereferencing scenarios for cancellation testing
sub reference_scenarios {
    my ($input) = @_;

    # Test cancellation during complex reference operations
    my $scalar_ref = \$input;
    my $array_ref = [$input, "second", "third"];
    my $hash_ref = { key1 => $input, key2 => "value" };

    # Test cancellation during reference-to-reference parsing
    my $ref_to_ref = \$scalar_ref;
    my $array_of_refs = [\$scalar_ref, \$array_ref, \$hash_ref];

    # Test cancellation during complex dereferencing
    my $dereferenced = ${$ref_to_ref};
    my @array_copy = @{$array_ref};
    my %hash_copy = %{$hash_ref};

    # Test cancellation during typeglob operations
    my $glob_ref = \*ComplexSyntax::reference_scenarios;

    return {
        scalar_ref => $scalar_ref,
        array_ref => $array_ref,
        hash_ref => $hash_ref,
        ref_to_ref => $ref_to_ref,
        dereferenced => $dereferenced,
        glob_ref => $glob_ref,
    };
}

# State variables and advanced features for cancellation testing
sub advanced_features {
    state $call_count = 0;
    $call_count++;

    # Test cancellation during given/when parsing
    my $input = shift;
    my $result;

    given ($input) {
        when (/^\d+$/) {
            $result = "numeric: $input";
        }
        when (/^[a-zA-Z]+$/) {
            $result = "alphabetic: $input";
        }
        default {
            $result = "mixed: $input";
        }
    }

    # Test cancellation during postfix conditionals
    say "Processing input" if $input;
    my $processed = process_advanced($input) unless !defined($input);

    return {
        call_count => $call_count,
        result => $result,
        processed => $processed,
    };
}

# Package variables and our declarations for cancellation testing
our $GLOBAL_CONFIG = {
    timeout => 30,
    retries => 3,
    debug => 1,
};

our @EXPORT_LIST = qw(
    quote_operator_scenarios
    heredoc_parsing_scenarios
    regex_parsing_scenarios
    closure_scenarios
    reference_scenarios
    advanced_features
);

our %FUNCTION_MAP = (
    'quotes' => \&quote_operator_scenarios,
    'heredoc' => \&heredoc_parsing_scenarios,
    'regex' => \&regex_parsing_scenarios,
    'closures' => \&closure_scenarios,
    'refs' => \&reference_scenarios,
    'advanced' => \&advanced_features,
);

# Helper functions for realistic parsing scenarios
sub process_advanced {
    my ($input) = @_;
    return "advanced_$input";
}

# Format strings and printf scenarios for cancellation testing
sub format_string_scenarios {
    my ($data) = @_;

    # Test cancellation during format string parsing
    my $formatted = sprintf("Value: %d, String: %s, Float: %.2f",
                           $data->{number},
                           $data->{text},
                           $data->{float});

    # Test cancellation during printf parsing
    printf("Debug: Processing %s with %d items\n",
           $data->{name},
           scalar(@{$data->{items}}));

    return $formatted;
}

1;