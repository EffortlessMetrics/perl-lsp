#!/usr/bin/env perl
# Test: Context-sensitive builtins and dual-form operators
# NodeKinds exercised: FunctionCall, Unary, Assignment, Variable, Block, If, Return
# Coverage gap: builtins whose parse depends on context (list vs scalar, 1-arg vs 2-arg)

use strict;
use warnings;

# --- caller() - scalar vs list context ---
sub show_caller_scalar {
    my $pkg = caller;          # scalar: package name only
    return $pkg;
}

sub show_caller_list {
    my @info = caller(0);      # list: (package, filename, line)
    my ($pkg, $file, $line) = caller(1);  # explicit frame
    return ($pkg, $file, $line, @info);
}

# caller in conditional
sub caller_guard {
    if (caller) {
        return "called from somewhere";
    }
    return "top-level";
}

# --- wantarray() ---
sub context_detector {
    if (wantarray()) {
        return (1, 2, 3);        # list context
    } elsif (defined wantarray()) {
        return "scalar";         # scalar context
    }
    return;                      # void context
}

my @list_result = context_detector();
my $scalar_result = context_detector();
context_detector();  # void

# wantarray as ternary condition
sub ternary_context {
    return wantarray ? (1, 2) : 42;
}

# --- pos() - lvalue and rvalue ---
my $text = "hello world hello";
$text =~ /hello/g;
my $p = pos($text);          # rvalue
pos($text) = 0;              # lvalue assignment
while ($text =~ /hello/g) {
    print "Found at pos: " . pos($text) . "\n";
}

# pos on $_
$_ = "aaa bbb ccc";
/bbb/g;
my $default_pos = pos;       # pos on $_
pos = 0;                     # lvalue on $_

# --- lock() ---
# lock is context-sensitive: builtin vs method
# use threads::shared;
# my $shared_var :shared = 0;
# lock($shared_var);          # builtin lock

# lock as method call (different parse)
my $mutex = bless {}, "Mutex";
# $mutex->lock();

# --- select() - two completely different forms ---
# 4-arg select (I/O multiplexing)
my $rin = '';
vec($rin, fileno(STDIN), 1) = 1;
# select($rin, undef, undef, 0.5);  # timeout select

# 1-arg select (filehandle selection)
my $old_fh = select(STDOUT);      # get/set default output
select($old_fh);                   # restore

# select with format variables
select(STDOUT);
$| = 1;                           # autoflush via select side-effect

# --- local() scoping ---
our $global = "original";
our @global_array = (1, 2, 3);
our %global_hash = (a => 1);

sub with_local_scalar {
    local $global = "temporary";
    return inner_reader();
}

sub with_local_array {
    local @global_array = (10, 20);
    return inner_reader();
}

sub with_local_hash {
    local %global_hash = (z => 99);
    return inner_reader();
}

sub inner_reader {
    return ($global, @global_array, %global_hash);
}

# local on special variables
sub local_special_vars {
    local $/ = undef;             # slurp mode
    local $\ = "\n";             # output record separator
    local $, = ", ";             # output field separator
    local $" = "-";              # list separator
    local $; = "::";             # subscript separator
    local $! ;                   # errno
    print "hello", "world";      # uses $, and $\
}

# local on glob
sub local_glob {
    local *STDERR;
    open STDERR, ">", "/dev/null";
    warn "suppressed";
}

# local on hash slice
sub local_hash_slice {
    local @ENV{qw(PATH HOME)} = ("/usr/bin", "/tmp");
    return $ENV{PATH};
}

# --- chdir() ---
# chdir with no args uses $ENV{HOME}
# chdir("/tmp");
# chdir;

# --- chomp/chop - return value differences ---
my $str1 = "hello\n";
my $removed_count = chomp $str1;     # returns count of chars removed
my $str2 = "hello";
my $removed_char = chop $str2;       # returns the removed character

# chomp/chop on arrays
my @lines = ("a\n", "b\n", "c\n");
my $total_chomped = chomp @lines;    # returns total count

# chomp on hash values
my %data = (x => "foo\n", y => "bar\n");
chomp %data;

# --- defined() vs exists() ---
my %test_hash = (a => 1, b => undef);
my $def = defined $test_hash{b};     # false - value is undef
my $ex  = exists $test_hash{b};      # true - key exists
my $neither = exists $test_hash{c};  # false - key missing

# defined on function return
sub maybe_undef { return undef }
if (defined(maybe_undef())) {
    print "defined\n";
}

# --- delete() - returns removed value ---
my %del_hash = (a => 1, b => 2, c => 3);
my $removed = delete $del_hash{a};           # scalar context
my @removed_list = delete @del_hash{qw(b c)};  # list context (hash slice)

# delete on array
my @del_array = (10, 20, 30, 40);
my $del_elem = delete $del_array[2];

print "Context-sensitive builtins test complete\n";
