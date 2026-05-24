my $sql = qq{select * from users where name = '{admin}'};
my $path = q{/tmp/{staging}/artifact};
my @items = qw(one {two} three);
