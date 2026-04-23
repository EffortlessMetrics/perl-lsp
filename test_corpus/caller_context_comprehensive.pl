#!/usr/bin/env perl
use strict;
use warnings;

# caller()/wantarray coverage with wrapper delegation and goto &sub patterns.

sub log_context (&) {
    my ($code) = @_;
    my $ctx = wantarray;
    my $ctx_name = !defined $ctx ? 'void' : $ctx ? 'list' : 'scalar';
    my @caller = caller(0);

    return {
        context => $ctx_name,
        package => $caller[0],
        line    => $caller[2],
        result  => $code->(),
    };
}

sub delegated_sum {
    my ($left, $right) = @_;
    return $left + $right;
}

sub tailcall_sum {
    # Ensure parser handles goto &sub tail-call syntax.
    goto &delegated_sum;
}

sub collect_pairs {
    my (%pairs) = @_;
    while (my ($k, $v) = each %pairs) {
        next if !defined $k || !defined $v;
        $pairs{$k} = $v // '<undef>';
    }
    return \%pairs;
}

sub evaluate_contexts {
    my $scalar = log_context { scalar tailcall_sum(20, 22) };
    my @list = (
        log_context { [tailcall_sum(1, 2), tailcall_sum(3, 4)] },
        log_context { collect_pairs(alpha => 1, beta => undef) },
    );

    log_context {
        my $sum = 0;
        $sum += $_ for map { $_ * 2 } grep { $_ > 1 } (1 .. 4);
        return $sum;
    };

    return ($scalar, @list);
}

my ($scalar_ctx, $list_ctx_a, $list_ctx_b) = evaluate_contexts();
print join q{ | }, $scalar_ctx->{context}, $list_ctx_a->{context}, ref $list_ctx_b->{result};
