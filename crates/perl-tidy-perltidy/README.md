# perl-tidy-perltidy

`perl-tidy-perltidy` is the explicit compatibility adapter for projects
that require the external `perltidy` executable or exact `Perl::Tidy`
behavior. It owns command-line configuration, subprocess invocation,
memoization, and legacy direct-consumer helpers.

The default native formatter is `perl-tidy`; this adapter is never selected
merely because `perltidy` happens to be installed.
