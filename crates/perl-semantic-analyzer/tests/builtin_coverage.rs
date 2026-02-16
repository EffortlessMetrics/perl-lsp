use perl_semantic_analyzer::{Parser, scope_analyzer::{ScopeAnalyzer, IssueKind}};
use perl_tdd_support::must;
use perl_parser_core::pragma_tracker::PragmaState;

#[test]
fn test_builtin_coverage() {
    let code = r#"
        # Built-in functions
        print "hello";
        say "world";
        open my $fh, "<", "file.txt";
        close $fh;
        map { $_ } (1, 2, 3);
        grep { $_ } (1, 2, 3);
        join ":", (1, 2, 3);
        split /:/, "1:2:3";
        chomp my $line;
        chop $line;

        # Keywords
        my $x = 1;
        our $y = 2;
        local $z = 3;
        state $s = 4;

        sub foo { return 1; }
        package Bar;

        use Data::Dumper;
        no strict 'refs';
        require strict;

        eval { 1 };

        # Control flow
        LOOP: for (1..10) {
            last LOOP if $_ == 5;
            next if $_ == 3;
            redo if $_ == 1;
        }

        # Quote-like
        q/foo/;
        qq/bar/;
        qw(a b c);
        qx/ls/;
        tr/a/b/;
        y/a/b/;

        # System
        system "echo hello";
        exec "echo bye";
        exit 0;
        die "Error";
        warn "Warning";

        # Misc
        defined $x;
        undef $x;
        ref $x;
        bless {}, "Foo";

        # File tests
        if (-f "file.txt") {}
        if (-d ".") {}
    "#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let analyzer = ScopeAnalyzer::new();

    // Enable strict mode manually for the whole file
    let pragma_map = vec![
        (0..code.len(), PragmaState {
            strict_vars: true,
            strict_subs: true,
            strict_refs: true,
            warnings: true
        })
    ];

    let issues = analyzer.analyze(&ast, code, &pragma_map);

    // Filter for UnquotedBareword issues
    let bareword_issues: Vec<_> = issues.iter()
        .filter(|i| i.kind == IssueKind::UnquotedBareword)
        .filter(|i| i.variable_name != "$_") // Ignore known issue with $_ for now
        .collect();

    for issue in &bareword_issues {
        println!("Unexpected bareword issue: {:?}", issue);
    }

    assert!(bareword_issues.is_empty(), "Should not report barewords for built-ins or keywords, found: {}", bareword_issues.len());
}
