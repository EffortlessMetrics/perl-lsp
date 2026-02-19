//! Comprehensive tests for regex and pattern matching
//!
//! These tests validate complex interactions between regex operations
//! including lookarounds, backreferences, modifiers, substitution,
//! given/when with regex, and pattern matching with data structures.

use perl_parser::Parser;
use perl_tdd_support::must;

mod nodekind_helpers;
use nodekind_helpers::has_node_kind;

/// Test complex regex with lookarounds, backreferences, and modifiers
#[test]
fn test_complex_regex_lookarounds_backreferences() {
    let code = r#"
# Complex regex with lookarounds
my $text = "The quick brown fox jumps over the lazy dog";

# Positive lookahead
if ($text =~ /quick(?=\s+brown)/) {
    print "Found 'quick' followed by 'brown'\n";
}

# Negative lookahead
if ($text =~ /fox(?!\s+cat)/) {
    print "Found 'fox' not followed by 'cat'\n";
}

# Positive lookbehind
if ($text =~ /(?<=\s+brown)\s+fox/) {
    print "Found 'fox' preceded by 'brown'\n";
}

# Negative lookbehind
if ($text =~ /(?<!quick)\s+brown/) {
    print "Found 'brown' not preceded by 'quick'\n";
}

# Complex nested lookarounds
if ($text =~ /(?<=The\s+)(?P<word>\w+)(?=\s+\w+)/) {
    print "Found word with lookahead and lookbehind: $+{word}\n";
}

# Backreferences
my $duplicate = "hello hello world";
if ($duplicate =~ /(?P<word>\w+)\s+\g{word}/) {
    print "Found duplicate word: $+{word}\n";
}

# Complex backreference patterns
my $pattern = "abcabcabc";
if ($pattern =~ /(?P<group>\w+).*\g{group}.*\g{group}/) {
    print "Found repeating group: $+{group}\n";
}

# Conditional regex patterns
my $data = "123-456-789";
if ($data =~ /(?:(?P<digits>\d{3})(?-)|(?P<letters>\w{3}))/) {
    if ($+{digits}) {
        print "Found digits: $+{digits}\n";
    } elsif ($+{letters}) {
        print "Found letters: $+{letters}\n";
    }
}

# Regex with multiple modifiers
my $case_insensitive = "Hello World";
if ($case_insensitive =~ /hello world/i) {
    print "Case insensitive match\n";
}

my $multiline = "Line 1\nLine 2\nLine 3";
if ($multiline =~ /^Line \d+$/m) {
    print "Multiline match\n";
}

my $extended_regex = "Test123";
if ($extended_regex =~ /
    test      # Match 'test'
    \d+        # Followed by one or more digits
    /x) {
    print "Extended regex match\n";
}

# Complex regex with atomic grouping
my $atomic_text = "abcabcabc";
if ($atomic_text =~ /(?>a+b+)/) {
    print "Atomic grouping match\n";
}

# Regex with possessive quantifiers
my $possessive_text = "aaaaab";
if ($possessive_text =~ /a++b/) {
    print "Possessive quantifier match\n";
}

# Regex with recursive patterns (PCRE style)
my $recursive_text = "a(b(c(d)e)f)g";
if ($recursive_text =~ /a\((?:[^()]*|(?R))*\)/) {
    print "Recursive pattern match\n";
}

# Complex regex with Unicode properties
my $unicode_text = "Café naïve résumé";
if ($unicode_text =~ /\p{Letter}+/) {
    print "Unicode letter match\n";
}

if ($unicode_text =~ /\p{L}+\s+\p{N}+/) {
    print "Unicode letter and number match\n";
}

# Regex with character class intersections
my $class_text = "abc123XYZ";
if ($class_text =~ /[\p{L}&&[a-z]]+/) {
    print "Character class intersection\n";
}

# Regex with conditional subpatterns
my $conditional_text = "abc123";
if ($conditional_text =~ /^(?(?=\d)\d+|[a-z]+)$/) {
    print "Conditional subpattern match\n";
}
"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    // Verify regex operations
    assert!(has_node_kind(&ast, "Regex"), "Should have regex nodes");

    // Verify match operations
    assert!(has_node_kind(&ast, "Match"), "Should have match operations");

    // Verify conditional statements
    assert!(has_node_kind(&ast, "If"), "Should have conditional statements");

    // Verify string literals with regex content
    assert!(has_node_kind(&ast, "String"), "Should have string literals");
}

/// Test substitution with complex patterns and delimiters
#[test]
fn test_substitution_complex_patterns_delimiters() {
    let code = r#"
# Basic substitution with different delimiters
my $text = "Hello World";
$text =~ s/World/Perl/;
$text =~ s#World#Perl#;
$text =~ s|World|Perl|;

# Substitution with modifiers
$text = "hello world";
$text =~ s/hello/hi/g;  # Global
$text =~ s/hello/hi/i;  # Case insensitive
$text =~ s/hello/hi/gi; # Both global and case insensitive
$text =~ s/\s+/ /g;   # Extended whitespace
$text =~ s/\b\w+\b/uc($&)/ge; # Evaluation with global

# Complex substitution patterns
$text = "The quick brown fox jumps over the lazy dog";
$text =~ s/(?<=\s)(\w+)(?=\s)/uc($1)/ge;
$text =~ s/(\w+)(\s+)(\w+)/$3$2$1/g;

# Substitution with backreferences
$text = "abc abc abc";
$text =~ s/(abc)/$1$1/g;
$text =~ s/(\w+)(\s+)/\U$1\E$2/g;

# Substitution with lookarounds
$text = "test123test456";
$text =~ s/(?<=test)(\d+)(?=test)/NUMBER/g;

# Substitution with conditional replacement
$text = "Item 1: apple, Item 2: banana, Item 3: cherry";
$text =~ s/Item (\d+): (\w+)/$2 (item $1)/g;

# Substitution with Unicode and character classes
$text = "Café naïve résumé";
$text =~ s/\p{Letter}+/\U$&/g;
$text =~ s/[\p{L}\p{N}]+/[$&]/g;

# Substitution with evaluation
my $count = 0;
$text = "one two three four";
$text =~ s/(\w+)/ ++$count; uc($1) /ge;

# Substitution with code execution (dangerous but testable)
$text = "func1() func2() func3()";
$text =~ s/(\w+)\(\)/$1()/gee;

# Substitution with complex delimiters and escaping
$text = "path/to/file.txt";
$text =~ s{/}{/}g;  # Replace forward slashes
$text =~ s[\/][\/]---g;  # Using brackets as delimiters

# Substitution with transliteration-like behavior
$text = "hello world";
$text =~ tr/hlo/HLO/;

# Complex substitution with multiple operations
$text = "The price is $100.00 and tax is $10.00";
$text =~ s/\$(\d+\.\d+)/$1/g;
$text =~ s/price/amount/g;
$text =~ s/tax/fee/g;

# Substitution in different contexts
sub process_text {
    my ($input) = @_;
    
    $input =~ s/^\s+|\s+$//g;  # Trim whitespace
    $input =~ s/\s+/ /g;         # Normalize whitespace
    $input =~ s/(\w+)/\u\L$1/g;   # Title case words
    
    return $input;
}

# Substitution with error handling
eval {
    my $risky_text = "test";
    $risky_text =~ s/(\w+)/$1/ee;  # Double evaluation
};

if ($@) {
    warn "Substitution failed: $@";
}

# Substitution with file processing
sub process_file_substitutions {
    my ($filename) = @_;
    
    open my $fh, '<', $filename or die "Cannot open $filename: $!";
    my @lines = <$fh>;
    close $fh;
    
    for my $line (@lines) {
        $line =~ s/old/new/g;
        $line =~ s/deprecated/obsolete/gi;
        $line =~ s/TODO/FIXME/g;
        
        print $line;
    }
}

# Test substitution operations
my $processed = process_text("  hello   world  ");
process_file_substitutions('config.txt');
"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    // Verify substitution operations
    assert!(has_node_kind(&ast, "Substitution"), "Should have substitution operations");

    // Verify transliteration operations
    assert!(has_node_kind(&ast, "Transliteration"), "Should have transliteration operations");

    // Verify match operations
    assert!(has_node_kind(&ast, "Match"), "Should have match operations");

    // Verify eval blocks for error handling
    assert!(has_node_kind(&ast, "Eval"), "Should have eval blocks");

    // Verify function calls
    assert!(has_node_kind(&ast, "FunctionCall"), "Should have function calls");
}

/// Test given/when with regex conditions and smart matching
#[test]
fn test_given_when_regex_smart_matching() {
    let code = r#"
use feature 'switch';

# Basic given/when with regex
my $value = "hello world";

given ($value) {
    when (/^hello/) {
        print "Starts with 'hello'\n";
    }
    when (/world$/) {
        print "Ends with 'world'\n";
    }
    when (/\s+/) {
        print "Contains whitespace\n";
    }
    default {
        print "No match found\n";
    }
}

# Given/when with complex regex patterns
my $email = "user@example.com";
given ($email) {
    when (/^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$/) {
        print "Valid email format\n";
    }
    when (/^[a-zA-Z0-9._%+-]+@localhost$/) {
        print "Local email address\n";
    }
    default {
        print "Invalid email format\n";
    }
}

# Given/when with smart matching
my $pattern = qr/^\d+$/;
my $number = 42;
my $string = "42";
my $array_ref = [42];
my $hash_ref = {value => 42};

given ($number) {
    when ($pattern) {
        print "Number matches pattern\n";
    }
    when (42) {
        print "Number equals 42\n";
    }
}

given ($string) {
    when ($pattern) {
        print "String matches pattern\n";
    }
    when (42) {
        print "String equals '42'\n";
    }
}

given ($array_ref) {
    when (@$array_ref) {
        print "Array reference has elements\n";
    }
    when (42) {
        print "Array contains 42\n";
    }
}

given ($hash_ref) {
    when (%$hash_ref) {
        print "Hash reference has keys\n";
    }
    when ({value => 42}) {
        print "Hash has value 42\n";
    }
}

# Complex given/when with multiple conditions
my $data = {
    type => 'user',
    status => 'active',
    level => 5
};

given ($data) {
    when (ref $_ eq 'HASH') {
        given ($_->{type}) {
            when ('user') {
                given ($_->{status}) {
                    when ('active') {
                        given ($_->{level}) {
                            when ($_ > 3) {
                                print "Active user with level > 3\n";
                            }
                            when ($_ <= 3) {
                                print "Active user with level <= 3\n";
                            }
                        }
                    }
                    when ('inactive') {
                        print "Inactive user\n";
                    }
                }
            }
            when ('admin') {
                print "Admin user\n";
            }
        }
    }
    when (ref $_ eq 'ARRAY') {
        print "Array data\n";
    }
    default {
        print "Unknown data type\n";
    }
}

# Given/when with regex capture groups
my $text = "Version 2.3.4";
given ($text) {
    when (/^Version (\d+)\.(\d+)\.(\d+)$/) {
        print "Major: $1, Minor: $2, Patch: $3\n";
    }
    when (/^(\d{4})-(\d{2})-(\d{2})$/) {
        print "Date: $1/$2/$3\n";
    }
    default {
        print "Unrecognized format\n";
    }
}

# Given/when with combined conditions
my $input = "test123";
given ($input) {
    when (/^[a-zA-Z]+$/ && length $_ > 5) {
        print "Long word\n";
    }
    when (/^\d+$/ && $_ > 100) {
        print "Large number\n";
    }
    when (/^[a-zA-Z]+\d+$/ && length $_ == 7) {
        print "7-character alphanumeric\n";
    }
    default {
        print "No special pattern\n";
    }
}

# Given/when with undef and defined checks
my $maybe_undef = undef;
my $definitely_defined = "defined";

given ($maybe_undef) {
    when (undef) {
        print "Value is undefined\n";
    }
    default {
        print "Value is defined\n";
    }
}

given ($definitely_defined) {
    when (undef) {
        print "This shouldn't print\n";
    }
    default {
        print "Value is defined\n";
    }
}

# Given/when with subroutine references
my $code_ref = sub { return $_[0] * 2 };
my $value_to_test = 21;

given ($value_to_test) {
    when ($code_ref) {
        my $result = $code_ref->($_);
        print "Code reference result: $result\n";
    }
    when (42) {
        print "Value equals 42\n";
    }
    default {
        print "No match\n";
    }
}
"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    // Verify given statements
    assert!(has_node_kind(&ast, "Given"), "Should have given statements");

    // Verify when clauses
    assert!(has_node_kind(&ast, "When"), "Should have when clauses");

    // Verify default clauses
    assert!(has_node_kind(&ast, "Default"), "Should have default clauses");

    // Verify regex operations
    assert!(has_node_kind(&ast, "Regex"), "Should have regex nodes");

    // Verify match operations
    assert!(has_node_kind(&ast, "Match"), "Should have match operations");

    // Verify hash literals
    assert!(has_node_kind(&ast, "HashLiteral"), "Should have hash literals");

    // Verify array literals
    assert!(has_node_kind(&ast, "ArrayLiteral"), "Should have array literals");
}

/// Test pattern matching with complex data structures
#[test]
fn test_pattern_matching_complex_data_structures() {
    let code = r#"
# Pattern matching with arrays
my @numbers = (1, 2, 3, 4, 5);
my @strings = ("apple", "banana", "cherry");
my @mixed = (1, "two", 3, "four");

# Array pattern matching
given (\@numbers) {
    when (1, 2, 3, 4, 5) {
        print "Exact array match\n";
    }
    when (1, 2, 3, @rest) {
        print "Prefix match: @rest\n";
    }
    when (@first, @second) {
        print "Split array: @first | @second\n";
    }
}

# Hash pattern matching
my %person = (
    name => "John",
    age => 30,
    city => "New York"
);

my %config = (
    database => {
        host => "localhost",
        port => 5432,
        name => "mydb"
    },
    cache => {
        enabled => 1,
        ttl => 3600
    }
);

given (%person) {
    when (name => "John", age => 30) {
        print "John, age 30\n";
    }
    when (name => $name, age => $age) {
        print "Person: $name, age $age\n";
    }
    when (%all) {
        print "All person data: " . join(", ", map { "$_=>$all{$_}" } keys %all) . "\n";
    }
}

# Complex nested structure matching
my $complex_data = {
    users => [
        {
            id => 1,
            name => "Alice",
            roles => ["admin", "user"],
            profile => {
                email => "alice@example.com",
                settings => {
                    theme => "dark",
                    notifications => 1
                }
            }
        },
        {
            id => 2,
            name => "Bob",
            roles => ["user"],
            profile => {
                email => "bob@example.com",
                settings => {
                    theme => "light",
                    notifications => 0
                }
            }
        }
    ],
    metadata => {
        total => 2,
        last_updated => time()
    }
};

# Pattern matching with nested structures
sub match_user_structure {
    my ($data) = @_;
    
    given ($data) {
        when ({
            users => [
                {
                    id => $id,
                    name => $name,
                    profile => {
                        email => $email,
                        settings => {
                            theme => $theme
                        }
                    }
                }
            ]
        }) {
            return {
                id => $id,
                name => $name,
                email => $email,
                theme => $theme
            };
        }
        default {
            return undef;
        }
    }
}

# Pattern matching with conditionals
sub match_with_conditions {
    my ($data) = @_;
    
    given ($data) {
        when ([$x, $y]) {
            when ($x > 0 && $y > 0) {
                return "Both positive";
            }
            when ($x < 0 && $y < 0) {
                return "Both negative";
            }
            when ($x * $y > 0) {
                return "Product positive";
            }
            default {
                return "Mixed or zero";
            }
        }
        when ({type => $type, value => $value}) {
            when ($type eq 'number' && $value > 100) {
                return "Large number";
            }
            when ($type eq 'string' && length $value > 10) {
                return "Long string";
            }
            default {
                return "Other data";
            }
        }
        default {
            return "Unknown pattern";
        }
    }
}

# Pattern matching with regex in structures
sub extract_data_with_regex {
    my ($text) = @_;
    
    given ($text) {
        when (/^(\w+):\s*(\d+)$/) {
            return {name => $1, value => $2};
        }
        when (/^(https?):\/\/([^\/]+)(.*)$/) {
            return {protocol => $1, host => $2, path => $3};
        }
        when (/^(\d{4})-(\d{2})-(\d{2})\s+(\d{2}):(\d{2}):(\d{2})$/) {
            return {
                date => "$1-$2-$3",
                time => "$4:$5:$6"
            };
        }
        default {
            return {error => "No match"};
        }
    }
}

# Pattern matching with type checking
sub type_based_matching {
    my ($value) = @_;
    
    given ($value) {
        when (undef) {
            return "undefined";
        }
        when (/^\d+$/) {
            return "integer string";
        }
        when (/^\d*\.\d+$/) {
            return "float string";
        }
        when (ref $_ eq 'ARRAY') {
            return "array reference";
        }
        when (ref $_ eq 'HASH') {
            return "hash reference";
        }
        when (ref $_ eq 'CODE') {
            return "code reference";
        }
        when (ref $_) {
            return "other reference: " . ref($value);
        }
        default {
            return "simple scalar";
        }
    }
}

# Test pattern matching functions
my $user_match = match_user_structure($complex_data);
my $conditional_match = match_with_conditions([1, -2]);
my $regex_match = extract_data_with_regex("https://example.com/path/to/resource");
my $type_match = type_based_matching("123.45");

# Pattern matching with guards
sub match_with_guards {
    my ($data) = @_;
    
    given ($data) {
        when ([$x, $y]) where { $x + $y == 10 } {
            return "Sum equals 10";
        }
        when ([$x, $y]) where { $x > $y } {
            return "First greater than second";
        }
        when ([$x, $y]) where { $x < $y } {
            return "First less than second";
        }
        when ({min => $min, max => $max}) where { $max - $min <= 100 } {
            return "Range within 100";
        }
        default {
            return "No guard match";
        }
    }
}

my $guard_match = match_with_guards([3, 7]);
"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    // Verify given statements
    assert!(has_node_kind(&ast, "Given"), "Should have given statements");

    // Verify when clauses
    assert!(has_node_kind(&ast, "When"), "Should have when clauses");

    // Verify default clauses
    assert!(has_node_kind(&ast, "Default"), "Should have default clauses");

    // Verify regex operations
    assert!(has_node_kind(&ast, "Regex"), "Should have regex nodes");

    // Verify match operations
    assert!(has_node_kind(&ast, "Match"), "Should have match operations");

    // Verify complex data structures
    assert!(has_node_kind(&ast, "HashLiteral"), "Should have hash literals");
    assert!(has_node_kind(&ast, "ArrayLiteral"), "Should have array literals");

    // Verify subroutine declarations
    assert!(has_node_kind(&ast, "Subroutine"), "Should have subroutine declarations");
}

/// Test advanced regex features and edge cases
#[test]
fn test_advanced_regex_features_edge_cases() {
    let code = r#"
# Recursive regex patterns
my $balanced_parens = "(a(b(c)d)e)f";
if ($balanced_parens =~ /^(?:[^()]|\((?:[^()]|\([^()]*\))*\))*$/) {
    print "Balanced parentheses\n";
}

# Regex with callouts and embedded code
my $text_with_code = "test123";
if ($text_with_code =~ /(?{print "Found match at pos $-[0]\n"; ''})\d+/) {
    print "Embedded code executed\n";
}

# Regex with conditional groups
my $conditional_text = "abc123";
if ($conditional_text =~ /^(?(?=\d)\d+|[a-z]+)$/) {
    print "Conditional pattern matched\n";
}

# Regex with branch reset
my $branch_text = "abc123def456";
if ($branch_text =~ /^(a|b|c)\d+(?|(d|e|f)\d+)*$/) {
    print "Branch reset pattern\n";
}

# Regex with possessive quantifiers and atomic groups
my $possessive_text = "aaaaab";
if ($possessive_text =~ /^(?>a+)b$/) {
    print "Possessive quantifier matched\n";
}

# Regex with lookaround and backreferences
my $complex_text = "abba";
if ($complex_text =~ /^(?=(.))\1\1$/) {
    print "Palindrome with lookahead\n";
}

# Regex with Unicode properties and scripts
my $unicode_mixed = "Café Москва 東京";
if ($unicode_mixed =~ /^\p{Latin}+\s+\p{Cyrillic}+\s+\p{Han}+$/) {
    print "Unicode script match\n";
}

# Regex with character class operations
my $class_text = "abcXYZ123";
if ($class_text =~ /^[\p{L}&&[a-z]]+\p{N}+$/) {
    print "Character class intersection\n";
}

# Regex with relative backreferences
my $relative_text = "abcabcabc";
if ($relative_text =~ /(abc)(?1)/) {
    print "Relative backreference matched\n";
}

# Regex with named capture groups and recursion
my $recursive_text = "a(b(c(d)e)f)g";
if ($recursive_text =~ /^(?P<outer>a(?P<inner>b(?P<deep>c(?P<deepest>d)e)f)g)$/) {
    print "Named recursive groups: outer=$+{outer}, inner=$+{inner}, deep=$+{deep}, deepest=$+{deepest}\n";
}

# Regex with conditional backreferences
my $cond_backref_text = "abc123abc";
if ($cond_backref_text =~ /^(a)(b)(c)(?(1)\1|(?2)\2|(?3)\3)$/) {
    print "Conditional backreference matched\n";
}

# Regex with parallel patterns
my $parallel_text = "abc123";
if ($parallel_text =~ /^(?(?=abc)[a-z]+|(?=\d)\d+)$/) {
    print "Parallel patterns matched\n";
}

# Regex with delayed execution
my $delayed_text = "test123";
if ($delayed_text =~ /(?{print "Delay executed\n"; ''})test/) {
    print "Delayed execution\n";
}

# Regex with verb patterns
my $verb_text = "running jumping";
if ($verb_text =~ /\b(run|jump|walk)(?:ing|ed)\b/) {
    print "Verb pattern matched\n";
}

# Regex with quantifier patterns
my $quantifier_text = "aaaabbbbccccc";
if ($quantifier_text =~ /^(a{4,})(b{4,})(c{5,})$/) {
    print "Quantifier pattern matched\n";
}

# Regex with lazy quantifiers
my $lazy_text = "tag1 content tag2 content tag3";
if ($lazy_text =~ /<.*?>.*?<\/.*?>/g) {
    print "Lazy quantifier matched\n";
}

# Regex with atomic and possessive combinations
my $atomic_possessive = "abcabcabc";
if ($atomic_possessive =~ /^(?>a+)(?>b+)(?>c+)$/) {
    print "Atomic and possessive combined\n";
}

# Regex with multiple conditions
my $multi_cond_text = "test123example";
if ($multi_cond_text =~ /^(?(?=test)test\d+|(?=example)example\w+)$/) {
    print "Multiple conditions matched\n";
}

# Regex with embedded comments and verbose mode
my $verbose_text = "test123";
if ($verbose_text =~ /
    test        # Match 'test'
    \d+         # Followed by digits
    (?{         # Embedded code
        print "Verbose mode match\n";
        '';     # Return empty string
    })
/x) {
    print "Verbose regex with comments\n";
}
"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    // Verify regex operations
    assert!(has_node_kind(&ast, "Regex"), "Should have regex nodes");

    // Verify match operations
    assert!(has_node_kind(&ast, "Match"), "Should have match operations");

    // Verify conditional statements
    assert!(has_node_kind(&ast, "If"), "Should have conditional statements");

    // Verify string literals with regex content
    assert!(has_node_kind(&ast, "String"), "Should have string literals");

    // Verify function calls
    assert!(has_node_kind(&ast, "FunctionCall"), "Should have function calls");
}
