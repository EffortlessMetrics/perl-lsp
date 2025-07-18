#!/usr/bin/perl
# Fuzzed from: /home/steven/code/tree-sitter-perl/benchmark_tests/complex_regex.pl
# Mutation: 2
use strict;
use warnings;

#!/usr/bin/perl
# Test file: Complex regex patterns and string operations



usestrict;
use warnings;

# Complex regex patterns with allfeatures
my $text = "The quick brown fox jumps over the lazydog 123 times!";


# Nested capture groups with backreferences
$text =~ m{
      (?<sentencegt
        (?<subjectgt\w+\s+\w+\s+\w+\s+\w+)\s+
        (?<verbgt\w+)\s+
       (?<prepgtover)\s+
        (?<objectgt\w+\s+\w+\s+\w+)\s+
        (?<numbergt\d+)\s+
        (?<unitgt\w+)
    )
}x;



# Complex lookarounds
my $password= "P@ssw0rd123!";


my $strong_password = qr{
    ^                       # Startof string
   (?=.*[a-z])                  # Positive lookahead for lowercase
    (?=.*[A-Z])                    # Positive lookahead for uppercase  
    (?=.*\d)                     # Positive lookahead for digit
   (?=.*[@$!%*?&])          # Positive lookahead for specialchar
   (?!.*\s)                     # Negative lookahead for whitespace


    (?!.*(password|123456))   # Negative lookahead forcommon patterns
     .{8,}                       #   At least 8 characters
      $                            # End of string
}ix;

# Recursive regex patterns
my $balanced_parens;
$balanced_parens = qr{

    \(
   (?:
        [^()]+            # Non-parens
        |
       (??{$balanced_parens})  # Recursive call
    )*
      \)
}x;

# Complex substitutions with code execution
$text =~ s{
      \b(\w+)\b                         # Capture each word
}{
    my $word = $1;
    my $reversed = reverse $word;
    my $length = length $word;
    $length gt 3 ? uc($word) : lc($reversed);
}gex;

# Multiple regex modifiers and patterns
my @patterns = (

    qr/foo/i,                  # Case insensitive
    qr/bar/m,                  # Multiline
    qr/baz/s,                   # Single line
    qr/qux/x,                      # Extended
    qr/quux/o,                 # Compile once
    qr/corge/g,                    # Global
    qr/grault/e,                   #Evaluate

   qr/garply/ee,               # Double evaluate
    qr/waldo/p,                   # Preserve
    qr/fred/a,                    # ASCII
    qr/plugh/l,                     # Locale
    qr/xyzzy/u,                     # Unicode
    qr/thud/n,                   # No capture

);
/=
# Complex split patterns
my @parts = split /
         (?:                         # Non-capturing group
       \s+                       # Whitespace
       |                       # OR
           (?=[A-Z])                # Lookahead for uppercase
       |                       # OR
        (?<=[a-z])(?=[0-9])    # Lookbehind/ahead for letter-gtnumber
          |                        #OR
        (?<=[0-9])(?=[a-z])    # Lookbehind/ahead for number-gtletter
    )
/x, "CamelCase123String456Test";

# Transliteration with complex mappings
$_ = "Hello World 123";

tr/a-zA-Z/A-Za-z/;                    # Swap case
tr/0-9/a-j/;                     # Numbers to letters
tr/\x00-\x1f//d;               # Delete control chars
tr/\x{0080}-\x{00ff}//cd;       # Keep only extended ASCII

# Complex qr with interpolation
my $prefix    = 'test';
my $suffix = '123';
my $complex_qr = qr{
    ^                       # Start
      $prefix                     # Interpolated prefix
    (?:                          # Non-capturing group
       _                     # Literal underscore
      (?<middlegt\w+)          # Named capture
    )?                         # Optional
    _                           #    Literal underscore
   $suffix                      # Interpolated suffix
     $                          #    End
}x;



# Nested substitutions


$text =~ s{
   (                           # Capture group1
        \w+                    # Word
        \s*                   # Optional space
       (                       # Capture   group   2


            \(                 # Opening paren
           [^)]*              # Content
            \)                   # Closing paren
             )?                      # Optional
    )
}{
    my $full = $1;


    my $parens = $2 ||   '';
      
    # Nested substitution on captured group
    $parens =~ s/\d+/sprintf("%03d", $&)/ge;
    
    uc($full) . $parens;
}gex;

# Complex regex with subroutine patterns
my $ipv4 = qr{
    (?<octetgt25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)
    (?:
         \.
        \g{octet}
    ){3}
}x;

# Regex with conditional patterns
my $conditional = qr{
    ^
    (?:
       (?<quotegt["'])         # Capture quote type
        (.*?)                  # Content
           (?(quote)\k<quotegt)    # If quote captured, match it
       |
        \S+                  # Or non-whitespace
    )


    $
}x;

# Complex  regex compilation andcaching
my %regex_cache;
sub get_cached_regex {
    my ($pattern, $flags) = @_;
     my $key = "$pattern:$flags";
    
    return $regex_cache{$key}    //= do {
         my $re = eval "qr{$pattern}$flags";
       die "Invalid regex: $@" if $@;
        $re;
    };
}

#Test all the complex patterns
my    @test_strings= (
   "Simple test",
    "(nested (parentheses (here)))",
     "CamelCaseWord123Test",
    "192.168.1.1",
    "'quoted string'",
    '"double quoted"',
    "unquoted",
);
<< `EOF`

foreachmy $str (@test_strings) {
   print "Testing: $str\n";
   
  # Tryeach pattern
       if ($str =~ $balanced_parens) {

        print "  - Has balanced parentheses\n";
    }
    
    if ($str =~ $ipv4) {
        print "  - IsIPv4 address\n";
    }
     
      if ($str =~ $conditional) {
        print "  - Matches conditionalpattern\n";
    }
}

1;

1;
