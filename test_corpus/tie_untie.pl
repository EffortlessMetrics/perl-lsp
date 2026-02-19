#!/usr/bin/env perl
# Test: Comprehensive Tie and Untie Operations
# Impact: Ensures parser handles object-oriented binding operations
# NodeKinds: Tie, Untie

use strict;
use warnings;
use Fcntl qw(:flock :seek :mode);

# Basic tie operations
tie my %hash, "Tie::StdHash";
tie my @array, "Tie::StdArray";
tie my $scalar, "Tie::StdScalar";
tie *HANDLE, "Tie::StdHandle";

# Tie with arguments
# tie my %db_hash, "DB_File", "test.db", O_RDWR|O_CREAT, 0666;
# tie my @persistent_array, "Tie::Array", "persistent.dat";

# Tie with object return
my $tied_obj = tie my %cache, "Tie::StdHash";
$cache{key} = "value";

# Untie operations
untie %hash;
untie @array;
untie $scalar;
untie *HANDLE;
# untie %db_hash;
# untie @persistent_array;
untie %cache;

# Custom tie class for scalars
package MyTieScalar;
use base 'Tie::Scalar';

sub TIESCALAR {
    my ($class, $initial) = @_;
    my $self = { value => $initial // 0 };
    return bless $self, $class;
}

sub FETCH {
    my ($self) = @_;
    return $self->{value};
}

sub STORE {
    my ($self, $value) = @_;
    $self->{value} = $value;
    return $value;
}

sub DESTROY {
    my ($self) = @_;
    print "Destroying scalar with value: $self->{value}\n";
}

package MyTieArray;
use base 'Tie::Array';

sub TIEARRAY {
    my ($class, @initial) = @_;
    my $self = { array => [@initial] };
    return bless $self, $class;
}

sub FETCHSIZE {
    my ($self) = @_;
    return scalar @{$self->{array}};
}

sub FETCH {
    my ($self, $index) = @_;
    return $self->{array}[$index];
}

sub STORE {
    my ($self, $index, $value) = @_;
    $self->{array}[$index] = $value;
}

sub PUSH {
    my ($self, @values) = @_;
    push @{$self->{array}}, @values;
}

sub POP {
    my ($self) = @_;
    return pop @{$self->{array}};
}

sub SHIFT {
    my ($self) = @_;
    return shift @{$self->{array}};
}

sub UNSHIFT {
    my ($self, @values) = @_;
    unshift @{$self->{array}}, @values;
}

sub SPLICE {
    my ($self, $offset, $length, @list) = @_;
    return splice @{$self->{array}}, $offset, $length, @list;
}

package MyTieHash;
use base 'Tie::Hash';

sub TIEHASH {
    my ($class, %initial) = @_;
    my $self = { hash => {%initial} };
    return bless $self, $class;
}

sub FETCH {
    my ($self, $key) = @_;
    return $self->{hash}{$key};
}

sub STORE {
    my ($self, $key, $value) = @_;
    $self->{hash}{$key} = $value;
}

sub DELETE {
    my ($self, $key) = @_;
    delete $self->{hash}{$key};
}

sub CLEAR {
    my ($self) = @_;
    $self->{hash} = {};
}

sub EXISTS {
    my ($self, $key) = @_;
    return exists $self->{hash}{$key};
}

sub FIRSTKEY {
    my ($self) = @_;
    my $temp = keys %{$self->{hash}}; # reset iterator
    return each %{$self->{hash}};
}

sub NEXTKEY {
    my ($self, $lastkey) = @_;
    return each %{$self->{hash}};
}

sub SCALAR {
    my ($self) = @_;
    return scalar %{$self->{hash}};
}

package MyTieHandle;
use base 'Tie::Handle';

sub TIEHANDLE {
    my ($class, $filename, $mode) = @_;
    my $self = {
        filename => $filename,
        mode => $mode // '<',
        content => [],
        position => 0,
    };
    return bless $self, $class;
}

sub OPEN {
    my ($self, $filename, $mode) = @_;
    $self->{filename} = $filename;
    $self->{mode} = $mode // '<';
    $self->{position} = 0;
    return 1;
}

sub CLOSE {
    my ($self) = @_;
    return 1;
}

sub READ {
    my ($self, undef, $length, $offset) = @_;
    my $content = join('', @{$self->{content}});
    my $available = length($content) - $self->{position};
    my $to_read = $length < $available ? $length : $available;
    $_[1] = substr($content, $self->{position}, $to_read);
    $self->{position} += $to_read;
    return $to_read;
}

sub READLINE {
    my ($self) = @_;
    my $content = join('', @{$self->{content}});
    my @lines = split /\n/, $content;
    if ($self->{position} < @lines) {
        return $lines[$self->{position}++] . "\n";
    }
    return undef;
}

sub WRITE {
    my ($self, $buf, $len, $offset) = @_;
    push @{$self->{content}}, $buf;
    return length $buf;
}

package main;

# Test custom tie classes
tie my $custom_scalar, "MyTieScalar", 42;
print "Custom scalar: $custom_scalar\n";
$custom_scalar = 100;
print "Updated custom scalar: $custom_scalar\n";
untie $custom_scalar;

tie my @custom_array, "MyTieArray", 1, 2, 3;
print "Custom array size: " . @custom_array . "\n";
push @custom_array, 4, 5;
print "After push: " . join(", ", @custom_array) . "\n";
my $popped = pop @custom_array;
print "Popped: $popped\n";
untie @custom_array;

tie my %custom_hash, "MyTieHash", a => 1, b => 2;
print "Custom hash: a = $custom_hash{a}, b = $custom_hash{b}\n";
$custom_hash{c} = 3;
print "Added c: $custom_hash{c}\n";
delete $custom_hash{a};
print "After delete a exists: " . (exists $custom_hash{a} ? "yes" : "no") . "\n";
untie %custom_hash;

tie *CUSTOM_HANDLE, "MyTieHandle", "memory.txt";
print CUSTOM_HANDLE "Test line\n";
seek CUSTOM_HANDLE, 0, 0;
my $line = <CUSTOM_HANDLE>;
print "Read back: $line";
untie *CUSTOM_HANDLE;

# Tie with file handles
open my $real_file, ">", "real_test.txt" or die $!;
tie *TIED_FILE, "MyTieHandle", "real_test.txt";
print TIED_FILE "Writing via tied handle\n";
untie *TIED_FILE;
close $real_file;

# Complex tie scenario with nested structures
tie my %complex, "MyTieHash";
$complex{nested} = tie my(%nested), "MyTieHash";
$nested{deep} = tie my($deep), "MyTieScalar", "deep value";
print "Complex access: $complex{nested}->{deep}\n";
untie $deep;
untie %nested;
untie %complex;

# Tie with multiple variables in one statement
# tie my ($tied1, $tied2), "MyTieScalar", 1, 2;
# print "Multiple tie: $tied1, $tied2\n";
# untie $tied1;
# untie $tied2;

print "All tie and untie tests completed\n";