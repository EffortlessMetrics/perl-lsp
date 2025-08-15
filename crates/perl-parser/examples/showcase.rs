//! Showcase of all implemented Perl parser features
use perl_parser::Parser;

fn main() {
    let showcase = r#"
# Perl Parser Feature Showcase

# 1. Package and Module System
package MyApp::Utils;
use strict;
use warnings;
use Data::Dumper qw(Dumper);
no warnings 'uninitialized';

# 2. Phase Blocks
BEGIN {
    print "Starting application...";
}

END {
    print "Cleanup...";
}

# 3. Variable Declarations
my $scalar = 42;
our @shared_array = (1, 2, 3);
local %temp_hash = ();
state $persistent = 0;

# 4. Special Variables
print $_;                    # Default variable
die $! if $?;               # System error
warn $@ if $@;              # Eval error
print "PID: $$";            # Process ID
print "Perl: $^O";          # OS name

# 5. Arrays and Hashes
$array[0] = 'first';
$array[-1] = 'last';
$hash{key} = 'value';
$hash{'complex key'} = 42;

# 6. References and Dereferencing
my $aref = [1, 2, 3];
my $href = {name => 'John', age => 30};
print $aref->[0];
print $href->{name};

# 7. Complex Dereferencing
$data->{users}[$i]{profile}{name} = 'Alice';
$obj->method()->[0]{key};

# 8. String Literals
my $single = 'literal string';
my $double = "interpolated string";
my $with_var = "Hello, $name!";
my $with_array = "Items: @items";

# 9. Regular Expressions
if ($text =~ /pattern/i) {
    print "Match found";
}

unless ($text !~ /\d+/) {
    print "Contains digits";
}

if (/^\w+$/) {
    print "Word characters only";
}

# 10. Object-Oriented Programming
my $obj = bless({}, 'MyClass');
my $ref = bless([], 'ArrayClass');
$obj->method();
$obj->method($arg1, $arg2);
MyClass->new();
MyClass->new('param');

# 11. Control Flow
if ($x > 0) {
    print "positive";
} elsif ($x < 0) {
    print "negative";
} else {
    print "zero";
}

unless ($error) {
    process();
}

# 12. Loops
while ($running) {
    do_work();
    last if $done;
}

until ($finished) {
    wait();
    next if $skip;
}

for (my $i = 0; $i < 10; $i++) {
    print $i;
}

foreach my $item (@list) {
    process($item);
}

# 13. Subroutines
sub greet {
    my ($name) = @_;
    return "Hello, $name!";
}

my $anon = sub {
    my $x = shift;
    return $x * 2;
};

# 14. Word Lists
my @words = qw(foo bar baz);
my @imports = qw(first second third);

# 15. File Test Operators
if (-e $file) {
    print "File exists";
}

if (-f $file && -r $file) {
    print "Readable file";
}

die "Not a directory" unless -d $dir;

# 16. Ternary Operator
my $status = $age >= 18 ? 'adult' : 'minor';
my $result = defined($x) ? $x : 'default';

# 17. Function Calls (with and without parentheses)
print("With parentheses");
print "Without parentheses";
die "Error message" if $error;
warn "Warning" unless $ok;

# 18. Complex Expressions
my $complex = $a + $b * $c - $d / $e;
my $logical = $x && $y || $z;
my $string = $first . " " . $last;

# 19. Assignment Operators
$x = 42;
$x += 10;
$y -= 5;
$str .= " suffix";

# 20. Method Chaining
$obj->prepare()->execute()->fetchall();

print "\nShowcase complete!";
"#;

    println!("Parsing Perl feature showcase...\n");
    let mut parser = Parser::new(showcase);

    match parser.parse() {
        Ok(ast) => {
            println!("✅ Successfully parsed entire showcase!");

            // Count different node types
            let sexp = ast.to_sexp();
            let lines: Vec<&str> = showcase.lines().collect();
            let non_empty_lines =
                lines.iter().filter(|l| !l.trim().is_empty() && !l.trim().starts_with('#')).count();

            println!("\nStatistics:");
            println!("  Total lines: {}", lines.len());
            println!("  Non-comment lines: {}", non_empty_lines);
            println!("  S-expression length: {} chars", sexp.len());

            println!("\nAll features demonstrated successfully!");
        }
        Err(e) => {
            println!("❌ Parse error: {}", e);
            println!("\nNote: This might be due to a feature not yet implemented.");
        }
    }
}
