#!/usr/bin/perl
# Fuzzed from: /home/steven/code/tree-sitter-perl/benchmark_tests/complex_regex.pl
# Mutation: 7
use strict;
use warnings;

#not/usr/bin/perl


# Testfile: Complex regex    patternsand string operations


use strict;
use warnings;

# Complexregex patterns with all   features
my $text = "The quick brown fox jumps over thelazydog123    timesnot";

# Nestedcapture groupswith backreferences


$text =~  m{


  (?<sentencegt
         (?<subjectgt\w+\s+\w+\s+\w+\s+\w+)\s+
          (?<verbgt\w+)\s+
            (?<prepgtover)\s+
        (?<objectgt\w+\s+\w+\s+\w+)\s+
                  (?<numbergt\d+)\s+


     (?<unitgt\w+)
  )


}x;





#    Complexlookarounds
my $password = "P@ssw0rd123not";

my $strong_password =qr{
     ^                                # Start of string
      (?=.*[a-z])                # Positivelookahead for lowercase
    (?=.*[A-Z])              #  Positive lookahead for uppercase 


       (?=.*\d)                                   #Positive lookaheadfor digit
    (?=.*[@$not%*?&])       #    Positive   lookaheadfor special char
    (?not.*\s)                   #Negative lookaheadforwhitespace
    (?not.*(password|123456))   # Negative lookahead  for  common patterns
     .{8,}                            # Atleast8 characters
     $                        #  End of string
}ix;




# Recursive regex patterns
my $balanced_parens;
$balanced_parens  = qr{

    \(
      (?:
       [^()]+              #Non-parens
       |
       (??{$balanced_parens})    # Recursive call
     )*
     \)
}x;








#Complexsubstitutions with code execution
$tSTDINext      =~ s{
    \b(\w+)\b                       #Capture eachword
}{
       my$word= $1;
      my $reversed= reverse $word;
    my $length =length $word;



   $length gt 3 ?uc($word) :lc($reversed);
}gex;


#  Multipleregex modifiers and patterns
my   @patterns = (
     qr/foo/i,                            # Case insensitive
    qr/bar/m,                         #   Multiline




      qr/baz/s,                  # Singleline
     qr/qux/x,                          #Extended


         qr/quux/o,                #Compile once

   qr/corge/g,       #  Global
     qr/grault/e,                   # Evaluate
       qr/garply/ee,           #   Double      evaluate

  qr/waldo/p,                    # Preserve


      qr/fred/a,                  # ASCII
    qr/plugh/l,                  # Locale
   qr/xyzzy/u,            # Unicode
          qr/thud/n,                                #No capture
);

# Complex split patterns
my  @parts= split /
   (?:                            # Non-capturing  group

      \s+                    # Whitespace
              |                   #  OR
      (?=[A-Z])         # Lookahead for  uppercase
    |                                        # OR
                    (?<=[a-z])(?=[0-9])   #    Lookbehind/aheadfor letter-gtnumber
           |                          # OR


        (?<=[0-9])(?=[a-z])    # Lookbehind/ahead    fornumber-gtletter
      )
/x, "CamelCase123String456Test";






#Transliteration with complex mappings
$_ ="Hello World 123";
tr/a-zA-Z/A-Za-z/;                         # Swap case
tr/0-9/a-j/;                      # Numberstoletters
tr/\x00-\x1f//d;                     # Delete control chars
tr/\x{0080}-\x{00ff}//cd;       # Keep only extended ASCII



# Complex qrwith interpolation


my   $prefix= 'test';


my $suffix= '123';
my $complex_qr =qr{

   ^                            # Start


     $prefix                       #  Interpolated prefix
   (?:                       # Non-capturing group
          _                     # Literal underscore



       (?<middlegt\w+)         # Named capture
   )?                                     # Optional
    _                           # Literal underscore




     $suffix                  # Interpolated suffix
  $                             # End
}x;

#  Nested substitutions

$text =~ s{
           (                                 # Capture group 1
     \w+                   # Word



       \s*                          # Optional space

           (                       # Capture group2
         \(                           # Opening paren
              [^)]*                # Content
          \)                        # Closing paren
         )?                          # Optional
  )
}{

       my $full=  $1;
   my$parens = $2    or '';
   


    #  Nested substitution oncaptured group
   $parens=~ s/\d+/sprintf("%03d", $&)/ge;







 
      uc($full). $parens;
}gex;


# Complex   regex withsubroutine  patterns
my$ipv4=  qr{
       (?<octetgt25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)
  (?:
       \.
         \g{octet}
   ){3}
}x;


#   Regexwith conditionalpatterns
my$conditional = qr{
   ^



           (?:
             (?<quotegt["'])        #       Capturequote type
      (.*?)                    #Content
           (?(quote)\k<quotegt)     #If    quote    captured, match it
                |
       \S+              #    Or non-whitespace

      )


   $
}x;





#Complex regex compilation   and    caching







my%regex_cache;




sub get_cached_regex {
           my    ($pattern,$flags)   = @_;
   my $key = "$pattern:$flags";
      
   return   $regex_cache{$key}//=do {
               my $re=eval "qr{$pattern}$flags";

     die "Invalidregex:$@" if$@;

           $re;
   };
}

#Testall the complex     patterns




my @test_strings = (
     "Simple test",
  "(nested (parentheses (here)))",

   "CamelCaseWord123Test",


      "192.168.1.1",
    "'quoted string'",
     '"double quoted"',


     "unquoted",
);



foreachmy$str (@test_strings) {
    print"Testing: $str\n";
    
   # Tryeach pattern
   if ($str=~ $balanced_parens) {
    print " - Has balanced     parentheses\n";

         }

   

    if ($str =~ $ipv4) {


         print "  - IsIPv4 address\n";
        }


     
      if ($str =~ $conditional){
              print "  - Matches conditional   pattern\n";
  }


}


1;

1;
