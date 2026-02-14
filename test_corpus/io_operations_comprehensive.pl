#!/usr/bin/env perl
# Test: Comprehensive I/O Operations
# Impact: Ensures parser handles file input/output operations
# NodeKinds: Readline, Diamond

use strict;
use warnings;

# Basic diamond operator (reading from ARGV)
while (my $line = <>) {
    chomp $line;
    print "Read from ARGV: $line\n";
    last if $. > 5;  # Limit to 5 lines for testing
}

# Diamond operator in list context
my @all_lines = <>;
print "Total lines read: " . scalar @all_lines . "\n";

# Diamond operator with explicit file handle
@ARGV = ('test_corpus/basic_constructs.pl');  # Read from existing file
while (my $line = <>) {
    print "File line: $line";
    last if $. > 3;
}

# Readline operator with STDIN
print "Enter some text (Ctrl+D to end):\n";
while (my $input = <STDIN>) {
    chomp $input;
    last if $input eq 'quit';
    print "You entered: $input\n";
}

# Readline with file handles
open my $fh, '<', 'test_corpus/basic_constructs.pl' or die $!;
while (my $line = <$fh>) {
    print "File handle line: $line";
    last if $. > 3;
}
close $fh;

# Readline in scalar context
open $fh, '<', 'test_corpus/basic_constructs.pl' or die $!;
my $first_line = <$fh>;
print "First line: $first_line";
close $fh;

# Readline with different file handles
open my $fh1, '<', 'test_corpus/basic_constructs.pl' or die $!;
open my $fh2, '<', 'test_corpus/given_when_default.pl' or die $!;

my $line1 = <$fh1>;
my $line2 = <$fh2>;

print "From file 1: $line1";
print "From file 2: $line2";

close $fh1;
close $fh2;

# Diamond operator with glob patterns
@ARGV = ('test_corpus/*.pl');
while (my $line = <>) {
    print "Glob pattern line: $line";
    last if $. > 3;
}

# Readline with indirect file handle
package IndirectHandle;
sub new {
    my ($class, $filename) = @_;
    open my $fh, '<', $filename or die $!;
    return bless { fh => $fh }, $class;
}

sub readline {
    my ($self) = @_;
    return readline $self->{fh};
}

sub close {
    my ($self) = @_;
    close $self->{fh};
}

package main;

my $indirect = IndirectHandle->new('test_corpus/basic_constructs.pl');
my $indirect_line = $indirect->readline();
print "Indirect handle line: $indirect_line";
$indirect->close();

# Diamond operator with command-line arguments
# Simulate command line arguments
local @ARGV = ('test_corpus/basic_constructs.pl');
my @diamond_lines = <>;
print "Diamond read " . scalar @diamond_lines . " lines\n";

# Readline with special variables
open $fh, '<', 'test_corpus/basic_constructs.pl' or die $!;
$/ = "\n";  # Input record separator
$\ = "";    # Output record separator

while (my $line = <$fh>) {
    print "Line $.: $line";
    last if $. > 3;
}

close $fh;

# Diamond operator with context sensitivity
{
    local $/;
    @ARGV = ('test_corpus/basic_constructs.pl');
    my $whole_file = <>;
    print "Whole file length: " . length($whole_file) . " characters\n";
}

# Readline with array slicing
open $fh, '<', 'test_corpus/basic_constructs.pl' or die $!;
my @first_five;
for (1..5) {
    my $line = <$fh>;
    last unless defined $line;
    push @first_five, $line;
}
print "First 5 lines: " . scalar @first_five . "\n";
close $fh;

# Diamond operator with multiple files
@ARGV = ('test_corpus/basic_constructs.pl', 'test_corpus/given_when_default.pl');
my $file_count = 0;
while (my $line = <>) {
    if (eof) {
        $file_count++;
        print "End of file $file_count reached\n";
    }
    last if $file_count >= 2;  # Process just 2 files
}

# Readline with error handling
eval {
    open my $bad_fh, '<', 'nonexistent_file.pl' or die $!;
    my $line = <$bad_fh>;
    close $bad_fh;
};
if ($@) {
    print "Expected error: $@";
}

# Diamond operator with pipe input (simulated)
# Note: This would work with actual pipe input: script.pl < somefile.txt
# For testing, we'll simulate with a file
@ARGV = ('test_corpus/basic_constructs.pl');
my @pipe_input = <>;
print "Simulated pipe input: " . scalar @pipe_input . " lines\n";

# Readline with different line endings
open $fh, '<', 'test_corpus/basic_constructs.pl' or die $!;
$/ = "\r\n";  # Windows line endings
while (my $line = <$fh>) {
    print "Windows-style line: $line";
    last if $. > 2;
}
close $fh;

# Diamond operator with file testing
@ARGV = ('test_corpus/basic_constructs.pl');
while (my $line = <>) {
    if (-f $ARGV) {
        print "Reading from regular file: $ARGV\n";
    }
    last if $. > 2;
}

# Complex readline with processing
open $fh, '<', 'test_corpus/basic_constructs.pl' or die $!;
my $processed_lines = 0;
while (my $line = <$fh>) {
    chomp $line;
    next if $line =~ /^\s*$/;  # Skip empty lines
    next if $line =~ /^\s*#/;  # Skip comments
    $processed_lines++;
    print "Processed line $processed_lines: $line\n";
    last if $processed_lines >= 5;
}
close $fh;

print "All I/O operations tests completed\n";