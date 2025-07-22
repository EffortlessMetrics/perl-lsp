//! Comprehensive test of implemented Perl features
use perl_parser::Parser;

fn main() {
    let test = r#"
# Package and pragmas
package MyModule;
use strict;
use warnings;
no warnings 'uninitialized';

# Phase blocks
BEGIN {
    print "Starting up";
}

END {
    print "Cleaning up";
}

# Variable declarations
my $scalar = 42;
our @array = (1, 2, 3);
local %hash = ();
state $persistent = 0;

# Special variables
print $_;
die $! if $?;
warn $@ if $@;
print "PID: $$";
print "Perl: $^O";

# Arrays and hashes
$array[0] = 'first';
$hash{key} = 'value';
my @slice = @array[1..3];

# References and dereferencing
my $aref = [1, 2, 3];
my $href = {};
print $aref->[0];
print $href->{key};

# Complex dereferencing
$data->{users}[$i]{name} = 'John';

# String interpolation
my $name = "World";
print "Hello, $name!";
print "Array: @array";
print "Path: $ENV{PATH}";

# Regular expressions
if ($text =~ /pattern/i) {
    print "Match found";
}

if ($text !~ /error/) {
    print "No errors";
}

# Object-oriented programming
my $obj = bless({}, 'MyClass');
$obj->method();
$obj->method($arg1, $arg2);
MyClass->new();

# Control flow
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

# Loops
while ($running) {
    do_work();
}

until ($done) {
    wait();
}

for (my $i = 0; $i < 10; $i++) {
    print $i;
}

foreach my $item (@list) {
    process($item);
}

# Subroutines
sub hello {
    my ($name) = @_;
    return "Hello, $name!";
}

my $anon = sub {
    return 42;
};

# Word lists
my @words = qw(foo bar baz);

# Function calls
print("Hello");
die("Error") if $error;
return $value;
"#;

    println!("Parsing comprehensive Perl code...\n");
    let mut parser = Parser::new(test);
    match parser.parse() {
        Ok(ast) => {
            println!("✅ Successfully parsed!");
            println!("\nS-expression output:");
            println!("{}", ast.to_sexp());
            
            // Count statements
            if let perl_parser::ast::NodeKind::Program { statements } = &ast.kind {
                println!("\nStatistics:");
                println!("  Total statements: {}", statements.len());
            }
        }
        Err(e) => {
            println!("❌ Parse error: {}", e);
        }
    }
}