use Test2::V0;

# `use Test2::V0;` turns strict + warnings on for us, so this file must stay
# clean under native critic even without explicit `use strict; use warnings;`.

subtest 'arithmetic' => sub {
    my $sum = 1 + 1;
    is($sum, 2, 'addition works');
    ok($sum > 0, 'sum is positive');
};

subtest 'strings' => sub {
    my $greeting = 'hello world';
    like($greeting, qr/world/, 'greeting contains world');
};

done_testing;
