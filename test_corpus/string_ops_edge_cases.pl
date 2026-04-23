#!/usr/bin/env perl
# Test: String operators, repetition, slicing, and list assignment edge cases
# NodeKinds exercised: Binary, Unary, FunctionCall, Assignment, Variable, ArrayLiteral, HashLiteral
# Coverage gap: x operator, chomp/chop, sprintf, complex slices, list assignment

use strict;
use warnings;

# --- String repetition operator (x) ---
my $dashes = "-" x 40;               # string repetition
my $stars = "*" x 0;                  # zero repetitions -> empty string
my $multi = "abc" x 3;               # "abcabcabc"
my $expr_rep = ("ha" x 3) . "!";     # with expression context

# x in list context: list repetition
my @zeros = (0) x 10;                # (0, 0, 0, ...) ten zeros
my @pattern = (1, 2, 3) x 4;         # repeats the list
my @single = ("x") x 5;

# x as operator vs x as part of hex literal
my $hex = 0x1F;                       # hex literal, not x operator
my $also_hex = 0xFF;
my $x_var = 10;
my $result = $x_var x 3;             # "101010" - x operator on variable

# Nested repetition
my @matrix = map { [(0) x 5] } 1..3; # 3x5 zero matrix

# --- chomp/chop ---
my $line = "hello\n";
my $n = chomp $line;                  # $n = 1, $line = "hello"

my $crlf = "hello\r\n";
{
    local $/ = "\r\n";
    chomp $crlf;                      # removes \r\n with custom $/
}

my $no_newline = "hello";
my $n2 = chomp $no_newline;          # $n2 = 0, unchanged

# chomp on list
my @lines = ("a\n", "b\n", "c\n");
my $total = chomp @lines;            # $total = 3

# chomp on hash values
my %h = (x => "foo\n", y => "bar\n");
chomp %h;

# chop returns the removed character
my $str = "hello!";
my $ch = chop $str;                  # $ch = "!", $str = "hello"

# chop on list
my @strs = ("ab", "cd", "ef");
chop @strs;                          # removes last char from each

# --- sprintf ---
my $formatted = sprintf "%d items at \$%.2f each", 5, 3.14;
my $padded = sprintf "%-20s: %s", "Name", "Value";
my $hex_str = sprintf "%#x", 255;         # "0xff"
my $oct_str = sprintf "%#o", 255;         # "0377"
my $bin_str = sprintf "%#b", 255;         # "0b11111111"
my $sci = sprintf "%e", 12345.678;        # scientific notation
my $pct = sprintf "%.1f%%", 99.5;         # escaped percent

# sprintf with vector flag
my $ip = sprintf "%vd", "127.0.0.1";     # version string
my $ver = sprintf "%vd", chr(1).chr(2).chr(3);

# sprintf with argument reordering
my $reordered = sprintf '%2$s is %1$d', 42, "answer";

# --- Array/hash slices ---
my @array = (10, 20, 30, 40, 50);

# Array slice
my @slice = @array[1, 3];                # (20, 40)
my @range_slice = @array[1..3];           # (20, 30, 40)
my @negative = @array[-2, -1];            # (40, 50)

# Hash slice
my %hash = (a => 1, b => 2, c => 3, d => 4);
my @hash_slice = @hash{qw(a c)};         # (1, 3)
my @hash_range = @hash{'a', 'b'};        # (1, 2)

# Slice assignment
@array[0, 2] = (100, 300);
@hash{qw(a b)} = (10, 20);

# Key-value slice (5.20+)
# my %kv_slice = %hash{qw(a c)};          # (a => 1, c => 3)
# my %arr_kv = %array[1, 3];              # (1 => 20, 3 => 40)

# --- List assignment edge cases ---
# Swap without temp
my ($x, $y) = (1, 2);
($x, $y) = ($y, $x);                     # swap

# List assignment with different counts
my ($a, $b, $c) = (1, 2);                # $c = undef
my ($d, $e) = (1, 2, 3);                 # 3 is discarded

# List assignment in boolean context
my $count = () = (1, 2, 3);              # $count = 3 (goatse/Saturn operator)

# Nested list assignment
my ($first, @rest) = (10, 20, 30, 40);   # $first=10, @rest=(20,30,40)
my (@head, $last);                        # @head slurps all - $last stays undef

# Hash from list
my %from_list = (a => 1, b => 2, c => 3);
my %from_flat = ('x', 10, 'y', 20);      # fat comma not required

# --- String comparison and operations ---
my $cmp_result = "abc" cmp "def";         # string comparison
my $eq_result = "abc" eq "abc";           # string equality
my $ne_result = "abc" ne "def";           # string inequality
my $lt_result = "abc" lt "def";           # string less-than
my $gt_result = "def" gt "abc";           # string greater-than

# String concatenation edge cases
my $concat = "a" . "b" . "c";
my $num_concat = "num:" . 42;            # auto-stringification
my $undef_concat;
$undef_concat .= "appended";             # .= on undef

# String multiplication / repetition in assignment
my $growing = "";
$growing .= "x" x 3;                     # "xxx"
$growing x= 2;                           # "xxxxxx"

# --- reverse, sort, join, split ---
my @sorted = sort @array;
my @rsorted = sort { $b <=> $a } @array;  # numeric reverse sort
my @custom = sort { lc($a) cmp lc($b) } ("Banana", "apple", "Cherry");

my @reversed = reverse @array;
my $rev_str = reverse "hello";            # "olleh"

my $joined = join ", ", @array;
my $joined2 = join(":", "a", "b", "c");

my @split1 = split /,/, "a,b,c";
my @split2 = split /\s+/, "  hello   world  ";
my @split3 = split //, "abc";            # character split
my @split4 = split /,/, "a,b,c,", -1;   # preserve trailing empty

# --- map and grep ---
my @doubled = map { $_ * 2 } @array;
my @evens = grep { $_ % 2 == 0 } @array;
my @transformed = map { uc } ("hello", "world");
my $count_match = grep { /^\d+$/ } ("1", "a", "2", "b");  # scalar: count

# Chained map/grep
my @result = sort grep { defined } map { $_ > 20 ? $_ : undef } @array;

print "String ops edge cases test complete\n";
