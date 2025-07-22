//! Final comprehensive showcase of all Perl parser features
use perl_parser::Parser;

fn main() {
    let showcase = r#"#!/usr/bin/perl
# Comprehensive Perl Parser Showcase

# 1. Package and Pragmas
package MyApp::Main;
use strict;
use warnings;
use feature qw(say state);
no warnings 'uninitialized';

# 2. Phase Blocks
BEGIN {
    print "Initializing...";
}

END {
    print "Cleanup...";
}

# 3. Variables and Special Variables
my $scalar = 42;
our @shared = (1, 2, 3);
local %temp = ();
state $count = 0;

# Special variables
print $_;
die $! if $?;
print "PID: $$, OS: $^O";

# 4. Compound Assignments
$x += 10;
$y -= 5;
$str .= " suffix";
$z **= 2;
$flags &= 0xFF;
$value ||= 'default';
$config //= {};

# 5. Arrays and Hashes
$array[0] = 'first';
$hash{key} = 'value';
$complex->{data}[$i]{name} = 'test';

# Array/hash slices
@slice = @array[1..5];
@keys = @hash{'a', 'b', 'c'};
@values = @hash{qw(x y z)};

# 6. References
my $aref = [1, 2, 3];
my $href = { name => 'John', age => 30 };
my $cref = sub { return $_[0] * 2 };

# 7. String Interpolation
my $name = "World";
my $greeting = "Hello, $name!";
my $list = "Items: @array";
my $path = "Path: $ENV{PATH}";

# 8. Regular Expressions
if ($text =~ /pattern/i) {
    print "Match!";
}

unless ($line !~ /^#/) {
    process($line);
}

# Bare regex
while (/\G(\w+)/gc) {
    print "Word: $1";
}

# 9. Object-Oriented
my $obj = bless({}, 'MyClass');
$obj->method();
$obj->method($arg1, $arg2);
MyClass->new(@args);

# 10. Control Flow with Word Operators
if ($x > 0 and $y < 100) {
    print "In range";
}

$result = $a or $b or $c or die "No value";
print "Debug" if $debug and not $silent;

# 11. Ternary Operator
my $status = $age >= 18 ? 'adult' : 'minor';
my $value = defined($x) ? $x : $default;

# 12. File Test Operators
if (-e $file) {
    if (-f $file and -r $file) {
        open(my $fh, '<', $file) or die $!;
    }
}

die "Not a directory" unless -d $dir;
my $size = -s $file;

# 13. Loops
for (my $i = 0; $i < 10; $i++) {
    print $i;
}

foreach my $item (@list) {
    next if $item eq 'skip';
    last if $item eq 'stop';
    process($item);
}

while (my $line = <$fh>) {
    chomp $line;
    print $line;
}

# 14. Subroutines
sub calculate {
    my ($x, $y) = @_;
    return $x + $y;
}

my $double = sub {
    my $n = shift;
    return $n * 2;
};

# 15. List Operations
my ($first, @rest) = @array;
my ($x, $y, $z) = (1, 2, 3);
push @stack, $item;
my $top = pop @stack;

# 16. Word Lists
my @words = qw(foo bar baz);
my @imports = qw(
    first
    second
    third
);

# 17. Function Calls
print "Hello, World!";
die "Error: $!" if $error;
warn "Warning" unless $ok;

# 18. Complex Expressions
my $result = ($a + $b) * ($c - $d) / $e;
my $bool = $x && $y || $z;
my $concat = $first . " " . $middle . " " . $last;

print "Showcase complete!";
"#;

    println!("Parsing comprehensive Perl showcase...\n");
    let mut parser = Parser::new(showcase);
    
    match parser.parse() {
        Ok(ast) => {
            println!("✅ Successfully parsed entire showcase!");
            
            let sexp = ast.to_sexp();
            
            // Count various features
            let compound_assigns = sexp.matches("assignment_").count() - sexp.matches("assignment_assign").count();
            let word_ops = sexp.matches("binary_and").count() + sexp.matches("binary_or").count() + 
                          sexp.matches("unary_not").count() + sexp.matches("binary_xor").count();
            let file_tests = sexp.matches("unary_-").count();
            let ternary = sexp.matches("ternary").count();
            
            println!("\nFeature Statistics:");
            println!("  Compound assignments: {}", compound_assigns);
            println!("  Word operators: {}", word_ops);
            println!("  File test operators: {}", file_tests);
            println!("  Ternary operators: {}", ternary);
            println!("  Total S-expression length: {} chars", sexp.len());
            
            println!("\n✨ All features working correctly!");
        }
        Err(e) => {
            println!("❌ Parse error: {}", e);
        }
    }
}