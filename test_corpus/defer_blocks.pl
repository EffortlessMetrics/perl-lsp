#!/usr/bin/env perl
# Test: defer blocks (Perl 5.36+ experimental)
# Coverage: NodeKind::Defer in multiple parent contexts

use v5.36;
use feature 'defer';
no warnings 'experimental::defer';

# Defer in a subroutine body (parent: subroutine)
sub with_cleanup {
    my $resource = 1;
    defer { warn "cleanup\n" }
    return $resource;
}

# Another subroutine using defer (second corpus file context)
sub nested_defer {
    defer { warn "outer\n" }
    defer { warn "inner\n" }
    return 42;
}
