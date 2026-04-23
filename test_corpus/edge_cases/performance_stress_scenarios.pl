#!/usr/bin/env perl
# Test: Performance stress scenarios
# Impact: Test parser performance with large inputs and pathological cases

use strict;
use warnings;

# Test 1: Very large strings (100KB+)
my $large_string = "x" x 100000;  # 100KB string
my $very_large_string = "y" x 1000000;  # 1MB string

# Large string with complex content
my $complex_large = "";
for my $i (1..10000) {
    $complex_large .= "Line $i: This is a test line with some content and numbers: $i\n";
}

# Test 2: Massive arrays and data structures
my @large_array = (1..100000);  # Array with 100K elements
my %large_hash = map { $_ => $_ * 2 } (1..50000);  # Hash with 50K elements

# Deep nested structure with many levels
my $massive_structure = {};
my $current = $massive_structure;
for my $i (1..1000) {
    $current->{level} = $i;
    $current->{next} = {};
    $current = $current->{next};
}

# Test 3: Pathological regex patterns that could cause backtracking
# Catastrophic backtracking patterns
my $backtrack1 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
my $pattern1 = /^(a+)+b$/;  # Classic catastrophic backtracking

my $backtrack2 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
my $pattern2 = /^(a+)*a$/;  # Another backtracking case

my $backtrack3 = "abcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabc";
my $pattern3 = /^(a.*)+b$/;  # Nested quantifiers

# Test 4: Massive files (simulated with large string)
# Simulate a 10K line Perl file
my $large_file_content = "";
for my $line_num (1..10000) {
    $large_file_content .= "my \$var$line_num = $line_num;\n";
    $large_file_content .= "print \"Line $line_num: \$var$line_num\\n\";\n";
    $large_file_content .= "if (\$var$line_num % 2 == 0) {\n";
    $large_file_content .= "    print \"Even number\\n\";\n";
    $large_file_content .= "} else {\n";
    $large_file_content .= "    print \"Odd number\\n\";\n";
    $large_file_content .= "}\n\n";
}

# Test 5: Memory-intensive operations
# Create many references
my @many_refs;
for my $i (1..100000) {
    push @many_refs, {
        id => $i,
        data => "x" x 100,
        nested => {
            level1 => $i * 2,
            level2 => $i * 3,
            level3 => $i * 4,
        }
    };
}

# Create circular references (memory leak test)
my $circular1 = { id => 1 };
my $circular2 = { id => 2 };
$circular1->{next} = $circular2;
$circular2->{next} = $circular1;

# Test 6: Complex nested operations
# Deeply nested map/grep/sort operations
my @source_data = (1..10000);

my $nested_ops = 
    map { $_ * 10 } 
    grep { $_ % 2 == 0 } 
    sort { $b <=> $a } 
    map { $_ + 5 } 
    grep { $_ > 100 } 
    map { $_ * 2 } 
    grep { $_ < 5000 } 
    @source_data;

# Test 7: Large heredocs
# Create a heredoc with 50K lines
my $large_heredoc = <<'LARGE_HEREDOC';
This is a very large heredoc with many lines.
Each line contains some text content.
Line 1: Initial content
LARGE_HEREDOC

for my $i (2..50000) {
    $large_heredoc .= "Line $i: More content with number $i\n";
}

$large_heredoc .= "End of large heredoc\n";

# Test 8: Complex regex with many alternatives
my $regex_alternatives = qr/
    (apple|banana|cherry|date|elderberry|fig|grape|honeydew|kiwi|lemon|mango|nectarine|
    orange|papaya|quince|raspberry|strawberry|tangerine|ugli|vanilla|watermelon|xigua|yuzu|
    zucchini|apricot|blueberry|cantaloupe|durian|eggfruit|feijoa|grapefruit|huckleberry|
    imbe|jackfruit|kumquat|lime|mulberry|olive|peach|persimmon|rambutan|starfruit|tomato|
    ugli|voavanga|watermelon|xigua|yellow|zucchini|almond|brazil|cashew|date|elderberry|
    fig|grape|hazelnut|imbe|jujube|kiwi|lime|macadamia|nutmeg|olive|pecan|quandong|
    raspberry|strawberry|tamarind|ugli|vanilla|walnut|xigua|yew|zucchini)
/x;

# Test 9: Massive symbol tables
# Create many variables with different names
my $var_declarations = "";
for my $i (1..10000) {
    $var_declarations .= "my \$var$i = $i;\n";
    $var_declarations .= "my \@arr$i = (1..\$i);\n";
    $var_declarations .= "my \%hash$i = (key$i => 'value$i');\n";
}

# Test 10: Complex nested conditionals
my $nested_conditionals = "";
my $indent = "";
for my $i (1..100) {
    $nested_conditionals .= $indent . "if (\$var$i > $i) {\n";
    $indent .= "    ";
    $nested_conditionals .= $indent . "print \"Level $i\\n\";\n";
}

for my $i (1..100) {
    $indent = substr($indent, 4);
    $nested_conditionals .= $indent . "}\n";
}

# Test 11: Large string concatenations
my $concat_result = "";
for my $i (1..10000) {
    $concat_result .= "Part $i: " . ("x" x 100) . "\n";
}

# Test 12: Complex data structure traversal
my $deep_structure = {
    level1 => {
        level2 => {
            level3 => {
                level4 => {
                    level5 => {
                        data => []
                    }
                }
            }
        }
    }
};

# Fill the deep structure
my $current_ref = $deep_structure;
for my $i (1..5) {
    my $key = "level" . ($i + 1);
    if ($i < 5) {
        $current_ref = $current_ref->{$key};
    } else {
        for my $j (1..1000) {
            push @{$current_ref->{data}}, {
                id => $j,
                content => "Item $j: " . ("y" x 50)
            };
        }
    }
}

# Test 13: Performance with Unicode stress
my $unicode_stress = "";
for my $i (1..10000) {
    $unicode_stress .= "Unicode test $i: 中文 العربية Русский עברית 日本語\n";
}

# Test 14: Complex eval statements
my $eval_code = "";
for my $i (1..1000) {
    $eval_code .= "my \$eval_var$i = $i;\n";
    $eval_code .= "\$result += \$eval_var$i;\n";
}

my $eval_result = 0;
eval $eval_code;

# Test 15: Large symbol table with complex names
my $complex_symbols = "";
for my $i (1..5000) {
    my $var_name = "var_" . ("a" x ($i % 50)) . "_$i";
    my $sub_name = "sub_" . ("b" x ($i % 30)) . "_$i";
    
    $complex_symbols .= "my \$$var_name = $i;\n";
    $complex_symbols .= "sub $sub_name { return $i; }\n";
}

# Test 16: Memory stress with many file handles (simulated)
my @file_handle_names = ();
for my $i (1..1000) {
    push @file_handle_names, "FILE$i";
}

# Test 17: Complex string operations
my $string_ops = "x" x 50000;
$string_ops =~ s/x/y/g;  # Global substitution
$string_ops =~ tr/y/z/;  # Transliteration
my @split_result = split //, $string_ops;  # Split into characters
my $joined = join "", @split_result;  # Join back

# Test 18: Large sort operations
my @unsorted = ();
for my $i (1..50000) {
    push @unsorted, int(rand(100000));
}

my @sorted = sort { $a <=> $b } @unsorted;

# Test 19: Complex hash operations
my %large_complex_hash;
for my $i (1..25000) {
    my $key = "key_" . ("a" x ($i % 100)) . "_$i";
    my $value = {
        number => $i,
        string => "value_" . ("b" x ($i % 50)) . "_$i",
        array => [1..($i % 100 + 1)]
    };
    $large_complex_hash{$key} = $value;
}

# Test 20: Stress with recursive functions
sub recursive_stress {
    my ($depth) = @_;
    return 1 if $depth <= 0;
    return $depth + recursive_stress($depth - 1);
}

my $recursive_result = recursive_stress(1000);

print "Performance stress scenarios test completed\n";
print "Large string length: " . length($large_string) . "\n";
print "Array size: " . scalar(@large_array) . "\n";
print "Hash size: " . scalar(keys %large_hash) . "\n";
print "Unicode stress length: " . length($unicode_stress) . "\n";
print "Recursive result: $recursive_result\n";