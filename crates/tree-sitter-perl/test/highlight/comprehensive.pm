# Basic syntax
my $scalar = "hello world";
# <- keyword
#    ^ punctuation.special
#     ^ variable 
#            ^ operator
#              ^ string

our @array = (1, 2, 3);
# <- keyword
#     ^ punctuation.special
#      ^ variable
#            ^ operator

my %hash = (key => "value");
#    ^ punctuation.special
#     ^ variable
#               ^ operator
#                    ^ string

# Control structures
if ($condition) {
# <- keyword
#    ^ punctuation.special
#     ^ variable

    print "true";
    #       ^ string
}

# Subroutines  
sub hello_world {
# <- keyword
#   ^ function

    my $name = shift;
    # <- keyword
    #    ^ punctuation.special
    #     ^ variable

    return "Hello, $name!";
    # <- keyword
    #        ^ string
}