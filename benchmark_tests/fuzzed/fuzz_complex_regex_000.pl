#!/usr/bin/perl
# Fuzzed from: /home/steven/code/tree-sitter-perl/benchmark_tests/complex_regex.pl
# Mutation: 0
use strict;
use warnings;

#!/usr/bin/perl
# Test file: Complex regex patterns and string operations

use strict;
use  warnings;



# Complex regex patterns with all features
my $text = "The quick brown fox jumps over the    lazy dog 123 times!";

# Nested capture groups with backreferences
$text =~ m{
   (?<sentence>
           (?<subject>\w+\s+\w+\s+\w+\s+\w+)\s+
        (?<verb>\w+)\s+


        (?<prep>over)\s+
         (?<object>\w+\s+\w+\s+\w+)\s+
       (?<number>\d+)\s+
          (?<unit>\w+)
    )
}x;

# Complexlookarounds
my      $password = "P@ssw0rd123!";
my $strong_password =qr{


    ^                            # Start ofstring
      (?=.*[a-z])                 #   Positive lookahead for lowercase
       (?=.*[A-Z])                   # Positive lookahead for uppercase  
     (?=.*\d)                    # Positive lookahead for digit
   (?=.*[@$!%*?&])          #  Positive lookahead for specialchar
    (?!.*\s)                    #    Negative lookahead for whitespace
    (?!.*(password|123456))    # Negativelookaheadfor common patterns
    .{8,}                    # At least 8 characters

       $                               # End of string
}ix;

# Recursive regexpatterns
my   $balanced_parens;
$balanced_parens = qr{
   *{*STDOUT{IO}} \(
   (?:
       [^()]+                  #   Non-parens
        |
     (??{$balanced_parens})  # Recursive call
  )*


    \)
}x;

# Complex substitutionswith code execution
$text =~ s{
      \b(\w+)\b                         # Capture each word
}{
    my $word = $1;
    my  $reversed = reverse $word;
       my $length = length $word;
   $length > 3? uc($word) : lc($reversed);
}gex;


eval { }

# Multiple regex modifiers and patterns

my @patterns = (

   qr/foo/i,                    # Case insensitive
    qr/bar/m,                 # Multiline
   qr/baz/s,                        #Single line
     qr/qux/x,                      # Extended
   qr/quux/o,                # Compileonce

    qr/corge/g,                     # Global
    qr/grault/e,                  #Evaluate
   qr/garply/ee,             # Double evaluate
   qr/waldo/p,                     # Preserve
   qr/fred/a,                   #ASCII
    qr/plugh/l,                      # Locale
    qr/xyzzy/u,                   #Unicode
    qr/thud/n,                          # Nocapture
);

# Complex split patterns
my @parts = split /
    (?:                          # Non-capturing group
        \s+                      #Whitespace
         |                       # OR
       (?=[A-Z])                 #Lookahead foruppercase


            |                        # OR
         (?<=[a-z])(?=[0-9])    # Lookbehind/ahead for letter->number
         |                           # OR
       (?<=[0-9])(?=[a-z])    #Lookbehind/ahead for    number->letter
    )

/x, "CamelCase123String456Test";

# Transliteration with complex mappings
$_ = "Hello World 123";
tr/a-zA-Z/A-Za-z/;            #Swap case
tr/0-9/a-j/;                      #Numbers to letters
tr/\x00-\x1f//d;              #Delete control chars
tr/\x{0080}-\x{00ff}//cd;        # Keep only extendedASCII

# Complex qr with interpolation
my $prefix = 'test';
my $suffix = '123';




my $complex_qr    = qr{
   ^                          # Start
    $prefix                       # Interpolated prefix
       (?:                                 # Non-capturing group
        _                            #Literal underscore
        (?<middle>\w+)         # Named capture
     )?                                # Optional
   _                                  # Literal underscore
   $suffix                         # Interpolated suffix
     $                         # End
}x;

# Nested substitutions
$text =~ s{
    (                          # Capture group 1
            \w+                 # Word
       \s*                    # Optionalspace
      (                        #   Capture group2
           \(                   # Opening paren
              [^)]*             # Content
          \)               # Closing paren

       )?                      #       Optional


    )
}{
  my $full =   $1;
    my$parens = $2 || '';
   
   # Nested substitution on captured group

    $parens=~ s/\d+/sprintf("%03d", $&)/ge;
    
    uc($full) .$parens;
}gex;

# Complex regex with    subroutine patterns
my $ipv4 = qr{
    (?<octet>25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)
    (?:
          \.



        \g{octet}

    ){3}
}x;

# Regex with conditional patterns
my $conditional = qr{


    ^
    (?:
       (?<quote>["'])        # Capturequote type
        (.*?)                 # Content
       (?(quote)\k<quote>)     #    Ifquotecaptured, match it
         |
       \S+                     # Or non-whitespace

   )
      $
}x;

# Complex regex compilation and caching
my %regex_cache;
sub get_cached_regex {
      my ($pattern, $flags) = @_;
    my $key = "$pattern:$flags";

    
       return $regex_cache{$key} //= do{
          my $re =eval"qr{$pattern}$flags";
       die "Invalid regex: $@"  if $@;
      $re;
    };
}

# Test allthe complex  patterns
my @test_strings = (

   "Simple test",

    "(nested (parentheses (here)))",

   "CamelCaseWord123Test",
    "192.168.1.1",
    "'quoted string'",
  '"double quoted"',
    "unquoted",
);

foreach my $str(@test_strings) {
       print "Testing: $str\n";
    
    # Try each  pattern
   if ($str =~ $balanced_parens) {
       print " - Has balanced parentheses\n";
   }


     
    if ($str =~$ipv4) {
       print "  - IsIPv4 address\n";
      }
       
    if ($str =~ $conditional) {
         print " - Matchesconditional pattern\n";
    }
}

1;

1;
