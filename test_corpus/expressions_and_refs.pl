#!/usr/bin/env perl
# Test: Expressions as values, references, dereferencing, and literal constructs
# NodeKinds exercised: ArrayLiteral, HashLiteral, Undef, Block (as expression),
#     Return, Unary, Binary, Ternary, FunctionCall, Variable, Assignment
# Coverage gap: undef literal, anon constructors, ref/deref, expression blocks

use strict;
use warnings;

# --- Undef literal in various contexts ---
my $x = undef;                            # explicit undef assignment
my @a = (1, undef, 3);                    # undef in list
my %h = (a => 1, b => undef);            # undef hash value
my @undefs = (undef) x 5;                # list of undefs

# undef as function argument
sub takes_optional {
    my ($required, $optional) = @_;
    $optional //= "default";
    return "$required:$optional";
}
takes_optional("key", undef);

# undef in comparisons
my $maybe;
if (!defined $maybe) {
    $maybe = "now defined";
}

# undef to undefine things
my $to_clear = 42;
undef $to_clear;                          # undefine scalar

my @to_clear_arr = (1, 2, 3);
undef @to_clear_arr;                      # undefine array

my %to_clear_hash = (a => 1);
undef %to_clear_hash;                     # undefine hash

# --- Anonymous array/hash constructors ---
my $aref = [1, 2, 3];                    # anonymous array ref
my $href = {a => 1, b => 2};             # anonymous hash ref
my $nested = {
    list => [1, [2, 3], [4, [5, 6]]],
    map  => {x => {y => {z => 1}}},
};

# Empty constructors
my $empty_aref = [];
my $empty_href = {};

# Constructor in expression context
my $length = scalar @{[1, 2, 3, 4]};     # array ref deref in scalar context
my $first = [10, 20, 30]->[0];           # immediate deref

# Complex nested construction
my @records = map {
    { id => $_, name => "item_$_", tags => [qw(a b c)] }
} 1..5;

# --- References and dereferencing ---
my $scalar_val = 42;
my $sref = \$scalar_val;                  # scalar reference
my $aref2 = \@a;                          # array reference
my $href2 = \%h;                          # hash reference
my $cref = \&takes_optional;              # code reference
my $gref = \*STDOUT;                      # glob reference

# Dereferencing
my $deref_scalar = $$sref;               # scalar deref
my @deref_array = @$aref2;               # array deref
my %deref_hash = %$href2;                # hash deref
my $deref_call = $cref->("arg");         # code deref and call
# print $gref "output\n";               # glob deref

# Arrow notation
my $elem = $aref->[1];                   # array element via ref
my $val = $href->{a};                    # hash value via ref
my $deep = $nested->{list}[1][0];        # chained deref

# ref() to check reference type
my $type_s = ref $sref;                  # "SCALAR"
my $type_a = ref $aref;                  # "ARRAY"
my $type_h = ref $href;                  # "HASH"
my $type_c = ref $cref;                  # "CODE"
my $type_n = ref 42;                     # "" (not a ref)

# --- Block as expression ---
my $block_val = do { my $tmp = 10; $tmp * 2 };

# Blocks in list context
my @block_list = (
    do { "first" },
    do { "second" },
    do { "third" },
);

# Block with early return-like behavior
my $computed = do {
    my $input = 42;
    if ($input > 100) {
        "large";
    } elsif ($input > 10) {
        "medium";
    } else {
        "small";
    }
};

# --- Return in various contexts ---
sub multi_return {
    my ($mode) = @_;

    return if $mode eq 'void';                      # bare return
    return undef if $mode eq 'undef';               # explicit undef return
    return 0 if $mode eq 'false';                   # false return
    return (1, 2, 3) if $mode eq 'list';            # list return
    return {status => 'ok'} if $mode eq 'ref';      # ref return
    return wantarray ? (1, 2) : "scalar";           # context-dependent return
}

# Return from nested block
sub nested_return {
    for my $i (1..10) {
        for my $j (1..10) {
            return ($i, $j) if $i * $j == 42;
        }
    }
    return ();  # not found
}

# Return value of eval
sub eval_return {
    my $result = eval {
        return 42;  # returns from eval block, not sub
    };
    return $result;
}

# --- Complex expression combinations ---
# Chained method calls as expression
my $obj = bless { data => [1, 2, 3] }, "Chainable";

# Ternary in assignment
my $sign = ($x // 0) > 0 ? "positive" : ($x // 0) < 0 ? "negative" : "zero";

# Logical operators as control flow
my $default = $maybe || "fallback";
my $defined_or = $maybe // "defined_fallback";
my $and_result = $maybe && process($maybe);

sub process { return $_[0] }

# Complex boolean expressions
my $complex = (defined $x && ref $x eq 'ARRAY')
    || (defined $x && ref $x eq 'HASH')
    || (!defined $x);

# Numeric vs string context
my $num_context = 0 + "42abc";            # 42
my $str_context = "" . 42;               # "42"
my $bool_context = !!"non-empty";        # 1 (double negation for boolean)

# List operations as expressions
my $max = (sort { $b <=> $a } @a)[0];
my $joined = join("-", map { uc } qw(hello world));
my $count = () = ("a" =~ /a/g);          # count matches

# Qw and other quoting
my @words = qw(alpha beta gamma delta);
my @interpolated = ("item_$_") for 1..3;

# Wantarray-based return
sub flexible {
    return wantarray ? (1, 2, 3) : [1, 2, 3];
}
my @list_ctx = flexible();
my $scalar_ctx = flexible();

# --- Postfix dereference (5.20+) ---
my $aref3 = [10, 20, 30];
my @postfix_arr = $aref3->@*;            # postfix array deref
my $href3 = {a => 1, b => 2};
my %postfix_hash = $href3->%*;           # postfix hash deref
my @postfix_slice = $aref3->@[0, 2];     # postfix array slice
my @postfix_hslice = $href3->@{qw(a b)}; # postfix hash slice

print "Expressions and refs test complete\n";
