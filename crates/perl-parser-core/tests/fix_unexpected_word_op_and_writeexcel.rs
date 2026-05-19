mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn test_writeexcel_filter_expression_after_quote_replacement_substitution() {
    let source = r#"
sub _extract_filter_tokens {
    my $expression = $_[0];
    my @tokens = ($expression =~ /"(?:[^"]|"")*"|\S+/g); #"

    for (@tokens) {
        s/^"//;     #"
        s/"$//;     #"
        s/""/"/g;   #"
    }

    return @tokens;
}

sub _parse_filter_expression {
    my $conditional = $tokens[3];

    if ($conditional =~ /^(and|&&)$/) {
        $conditional = 0;
    }
    elsif ($conditional =~ /^(or|\|\|)$/) {
        $conditional = 1;
    }
}
"#;
    assert_clean_parse(source);
}
