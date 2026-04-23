#!/usr/bin/env perl
# Test: Modern Perl 5.34-5.38 features
# Impact: Shows parser handles current stable Perl features

use v5.38;
use feature qw(signatures try isa builtin class defer);
no warnings qw(experimental::signatures experimental::try experimental::isa 
               experimental::builtin experimental::class experimental::defer);

# Signatures (stable in 5.36+)
sub add($x, $y) {
    return $x + $y;
}

sub greet($name = 'World') {
    say "Hello, $name!";
}

sub process($first, $second, @rest) {
    return $first + $second + @rest;
}

sub optional($req, $opt = undef, @) {
    return $req;
}

# Slurpy hash
sub config($name, %options) {
    return { name => $name, %options };
}

# Type constraints in signatures (experimental)
sub typed_add(Num $x, Num $y) {
    return $x + $y;
}

# Try/catch/finally (experimental, core since 5.34)
sub safe_divide($a, $b) {
    try {
        die "Division by zero" if $b == 0;
        return $a / $b;
    }
    catch ($e) {
        warn "Error: $e";
        return undef;
    }
    finally {
        # Cleanup code
        say "Division attempted";
    }
}

# Nested try/catch
try {
    try {
        die "Inner error";
    }
    catch ($inner) {
        die "Wrapped: $inner";
    }
}
catch ($outer) {
    say "Caught: $outer";
}

# Defer blocks (experimental, 5.36+)
sub with_defer {
    defer { say "This runs on scope exit" }
    defer { say "This runs first (LIFO)" }
    
    say "Normal code";
    return 42;
}

# Builtin functions (experimental, 5.36+)
use builtin qw(true false is_bool weaken blessed refaddr reftype
               created_as_string created_as_number
               ceil floor indexed trim);

my $bool = true;
my $not_bool = !false;

if (is_bool($bool)) {
    say "It's a boolean";
}

my @indexed_array = indexed('a', 'b', 'c');  # (0, 'a', 1, 'b', 2, 'c')

my $trimmed = trim("  spaces  ");

my $ceiling = ceil(3.14);
my $floored = floor(3.14);

# ISA operator (experimental, 5.32+)
my $array_ref = [];
my $hash_ref = {};

if ($array_ref isa ARRAY) {
    say "It's an array reference";
}

if ($hash_ref isa HASH) {
    say "It's a hash reference";
}

my $object = bless {}, 'MyClass';
if ($object isa MyClass) {
    say "It's a MyClass object";
}

# Class feature (Corinna) - experimental, 5.38+
use feature 'class';
no warnings 'experimental::class';

class Point {
    field $x :param = 0;
    field $y :param = 0;
    
    method new($class: $x = 0, $y = 0) {
        return $class->SUPER::new(x => $x, y => $y);
    }
    
    method move($dx, $dy) {
        $x += $dx;
        $y += $dy;
    }
    
    method distance($other) {
        return sqrt(($other->x - $x)**2 + ($other->y - $y)**2);
    }
}

class Point3D :isa(Point) {
    field $z :param = 0;
    
    method altitude() {
        return $z;
    }
}

# Postfix dereferencing (stable since 5.24)
my $aref = [1, 2, 3];
my $href = {a => 1, b => 2};
my $cref = sub { return "code" };

my @array = $aref->@*;
my %hash = $href->%*;
my $result = $cref->&*;

# Slice with postfix
my @slice = $aref->@[0, 2];
my @keys = $href->@{qw(a b)};

# Indented heredocs (stable since 5.26)
my $indented = <<~'EOF';
    This is indented
    but the indentation
    will be stripped
    EOF

# Unicode delimiters
my $unicode = qw「one two three」;

# Chained comparisons (5.32+)
my $x = 5;
if (1 < $x < 10) {
    say "x is between 1 and 10";
}

# Subroutine signatures in anonymous subs
my $anon = sub ($x, $y = 0) {
    return $x + $y;
};

# Method signatures
package MyPackage {
    use feature 'signatures';
    
    sub method :method ($self, $arg) {
        return $self->{value} + $arg;
    }
}

# State variables with initialization
sub counter($increment = 1) {
    state $count = 0;
    return $count += $increment;
}

# Match operator with unicode properties
my $text = "Hello 世界";
if ($text =~ /\p{Han}/) {
    say "Contains Chinese characters";
}

# Non-capturing groups in list context
my ($first, $second) = "test123" =~ /([a-z]+)(?:[0-9]+)/;

# Multiple package declarations with versions
package Foo 1.23 {
    sub foo { "foo" }
}

package Bar 4.56 {
    sub bar { "bar" }
}

__END__
Parser assertions:
1. All modern syntax recognized without errors
2. Signatures parsed correctly in hover/completion
3. Try/catch/finally blocks have proper folding ranges
4. Class/field/method keywords recognized as symbols
5. ISA operator doesn't confuse type checking
6. Builtin functions have hover documentation
7. Postfix dereference operators tokenized correctly