#!/usr/bin/env perl
# Test: Legacy Perl syntax and features
# Impact: Real-world code often contains legacy patterns

# Legacy package separator (apostrophe)
package Don't;  # Same as Don::t

sub can't {      # Same as can::t
    return "Legacy separator";
}

package I'll::Do'It;  # Mixed separators

# Bareword filehandles
open FILE, '<', 'input.txt';
open(OUTPUT, '>', 'output.txt');
print FILE "content\n";
print OUTPUT "more content\n";
close FILE;
close(OUTPUT);

# Indirect object syntax
my $obj = new Some::Class;
my $result = new Some::Class 'arg1', 'arg2';
$obj = new Some::Class::;  # With trailing ::

# More indirect object
print STDOUT "Hello\n";
print STDERR "Error\n";
printf STDOUT "%s\n", "formatted";

# Indirect method calls
my $method = 'process';
$result = $method $obj @args;

# Filetest operators
if (-e $file && -r $file && -w $file) {
    print "File is readable and writable\n";
}

if (-d $dir && -x $dir) {
    print "Directory is executable\n";
}

my $age = -M $file;     # Modification age in days
my $size = -s $file;    # Size in bytes
my $text = -T $file;    # Text file test
my $binary = -B $file;  # Binary file test

# Stacked filetests (5.10+)
if (-r -w -x $file) {
    print "All permissions\n";
}

# Special literals
my $input = <STDIN>;      # Read from STDIN
my @lines = <FILE>;       # Read all lines
my $glob = <*.pl>;        # Glob pattern
my @files = <{foo,bar}>;  # Brace expansion

# Old-style loops
for $i (0..10) {         # No 'my'
    print "$i\n";
}

foreach $item (@array) {  # No 'my'
    process($item);
}

# $[ variable (array base index)
$[ = 1;  # Arrays start at 1 (DON'T DO THIS!)
my @arr = qw(a b c);
print $arr[1];  # Prints 'a' with $[ = 1

# Reset and study
reset 'X';     # Reset variables starting with X
study $text;   # Optimize regex matching (deprecated)

# Format declarations
format STDOUT_TOP =
                  Page @<<<
                  $%
.

format STDOUT =
@<<<<<<<< @||||||| @>>>>>>>>
$name,    $score,   $date
.

write STDOUT;

# Typeglobs and symbol table manipulation
*alias = *original;
*My::Package::func = \&other_func;
local *FH;

# Glob assignment
*PI = \3.14159;
print "PI = $PI\n";

# Auto-vivification of filehandles
my $fh = \*GLOB;
open($fh, '<', 'file.txt');

# Tied variables
tie my %hash, 'Tie::IxHash';
tie my @array, 'Tie::File', 'filename';
tie my $scalar, 'Tie::Scalar';

# Magic goto
sub recurse {
    @_ = ("new", "args");
    goto &other_sub;  # Tail call
}

# Labels and goto
LABEL: {
    print "Start\n";
    goto END_LABEL if $condition;
    print "Middle\n";
    END_LABEL:
    print "End\n";
}

# Here-doc with backticks
my $output = <<`END`;
echo "Shell command"
date
END

# Comma operator in scalar context
my $last = (1, 2, 3);  # $last gets 3

# Flip-flop operator
while (<>) {
    print if /START/ .. /END/;  # Range operator
    print if 3 .. 5;             # Line numbers
}

# Defined-or (older alternative)
my $value = $input || $default;     # Problem with 0 and ""
my $better = $input // $default;    # Defined-or (5.10+)

# Prototype with special markers
sub mysub(_) { }           # Default to $_
sub mygrep(&@) { }        # Block and list
sub mymap(&@) { }         # Like map
sub myfunc($;$) { }       # One required, one optional
sub myref(\@) { }         # Array reference
sub myhash(\%) { }        # Hash reference
sub myglob(*) { }         # Typeglob
sub mycode(&) { }         # Code reference

# Attributes
my $shared : shared;
my $const : Readonly = 42;
sub lvalue : lvalue { $x }

# BEGIN and friends in unusual places
my $x = do {
    BEGIN { print "compile time\n" }
    42
};

sub with_begin {
    BEGIN { print "still compile time\n" }
    return 1;
}

# AUTOLOAD
our $AUTOLOAD;
sub AUTOLOAD {
    my $method = $AUTOLOAD;
    $method =~ s/.*:://;
    print "Called $method\n";
}

# Special variables
$_ = "default";
@_ = qw(args);
$" = ', ';           # List separator
$, = ', ';           # Output field separator
$\ = "\n";           # Output record separator
$/ = "\n";           # Input record separator
$| = 1;              # Autoflush
$. = 42;             # Line number
$$ = $$;             # Process ID
$? = 0;              # Child error
$! = 0;              # System error

# Obsolete variables
$* = 1;              # Multi-line matching (removed)
$# = "%.2f";         # Output format (deprecated)

# Old-style subs without parens
sub noparens { @_ }
my $r = noparens 1, 2, 3;

# Use of $#array
my @list = (1,2,3);
print "Last index: $#list\n";
$#list = 10;  # Extend array

# Each/keys/values on arrays
while (my ($i, $v) = each @array) {
    print "$i => $v\n";
}

__END__
Parser assertions:
1. Legacy ' separator recognized as package separator
2. Bareword filehandles don't cause errors
3. Indirect object syntax parsed correctly
4. Filetest operators recognized
5. Format blocks handled properly
6. Special variables recognized
7. AUTOLOAD and other special subs identified
8. Attributes parsed without errors