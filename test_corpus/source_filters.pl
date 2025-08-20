#!/usr/bin/env perl
# Test: Source filters (parse-time code rewriting)
# Impact: Real CPAN modules use Filter::Simple, Filter::Util::Call
# Parser must handle filter declarations even if it can't execute them

# Filter::Simple example - declarative filter
use Filter::Simple;
FILTER {
    s/BANG!/return "excited"/g;
    s/MAGIC/42/g;
};

sub get_mood {
    BANG!;  # becomes: return "excited"
}

my $answer = MAGIC;  # becomes: 42

# Filter::Util::Call example - procedural filter
package MyFilter;
use Filter::Util::Call;

sub import {
    my ($type) = @_;
    my ($ref) = [];
    filter_add(bless $ref);
}

sub filter {
    my ($self) = @_;
    my $status;
    
    # Read and transform source
    $status = filter_read();
    if ($status > 0) {
        s/\bDEBUG\b/print STDERR/g;
        s/\bASSERT\b/die unless/g;
    }
    return $status;
}

package main;
use MyFilter;

DEBUG "Starting program\n";  # becomes: print STDERR "Starting program\n"
ASSERT $x > 0;               # becomes: die unless $x > 0

# Acme::Bleach style (all whitespace)
use Acme::Bleach;
                             # invisible code here

# Source filter with regex
use Filter::cpp;
#ifdef DEBUG
print "Debug mode\n";
#endif

# Multiple filters
use Filter::Simple;
use Filter::exec qw(sh);
FILTER { s/TODO/die "Not implemented"/g };

sub unfinished {
    TODO;  # becomes: die "Not implemented"
}

# Echo filter
`echo "Shell command in filter"`;

__END__
Parser assertions:
1. Should not hang or crash on filter declarations
2. Should produce stable AST even if transformations aren't applied
3. LSP diagnostics should always return (no timeouts)
4. Document symbols should show subroutines despite filters