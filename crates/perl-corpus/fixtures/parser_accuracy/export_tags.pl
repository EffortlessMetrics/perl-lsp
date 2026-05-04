package Accuracy::ExportTags;

our %EXPORT_TAGS = (
    all => [qw(foo bar)],
);

sub foo {
    return 1;
}

sub bar {
    return 2;
}

sub use_exports {
    return foo() + bar();
}

1;
