use perl_lsp_import_management::{
    collect_imports, find_imports_range, guess_module_for_function, sort_imports,
};

#[test]
fn guesses_common_module_names() {
    assert_eq!(
        guess_module_for_function("encode"),
        Some("Encode".to_string())
    );
    assert_eq!(guess_module_for_function("missing"), None);
}

#[test]
fn collects_use_and_require_lines() {
    let lines = vec![
        "#!/usr/bin/perl".to_string(),
        "use strict;".to_string(),
        "my $x = 1;".to_string(),
        "require Foo::Bar;".to_string(),
    ];

    let imports = collect_imports(&lines);
    assert_eq!(imports, vec!["use strict;", "require Foo::Bar;"]);
}

#[test]
fn sorts_imports_by_expected_bucket_order() {
    let sorted = sort_imports(vec![
        "use Foo::Bar;".to_string(),
        "use strict;".to_string(),
        "use lib './lib';".to_string(),
        "use warnings;".to_string(),
        "use integer;".to_string(),
    ]);

    assert_eq!(
        sorted,
        vec![
            "use strict;",
            "use warnings;",
            "use integer;",
            "use Foo::Bar;",
            "use lib './lib';",
        ]
    );
}

#[test]
fn finds_import_block_range() {
    let source = "#!/usr/bin/perl\nuse strict;\nuse warnings;\nprint 'ok';\n";
    let lines = source.lines().map(str::to_string).collect::<Vec<_>>();

    let range = find_imports_range(source, &lines);
    assert!(range.is_some());
    if let Some((start, end)) = range {
        assert_eq!(&source[start..end], "use strict;\nuse warnings;");
    }
}

#[test]
fn finds_import_block_range_when_import_line_repeats_later() {
    let source = "#!/usr/bin/perl\nuse strict;\nuse warnings;\nprint 'ok';\nuse warnings;\n";
    let lines = source.lines().map(str::to_string).collect::<Vec<_>>();

    let range = find_imports_range(source, &lines);
    assert!(range.is_some());
    if let Some((start, end)) = range {
        assert_eq!(
            &source[start..end],
            "use strict;\nuse warnings;\nprint 'ok';\nuse warnings;"
        );
    }
}

#[test]
fn returns_none_for_sources_without_imports() {
    let source = "#!/usr/bin/perl\nmy $x = 1;\nprint $x;\n";
    let lines = source.lines().map(str::to_string).collect::<Vec<_>>();

    assert_eq!(find_imports_range(source, &lines), None);
}

#[test]
fn sort_imports_removes_duplicates() {
    let sorted = sort_imports(vec![
        "use Data::Dumper;".to_string(),
        "use strict;".to_string(),
        "use Data::Dumper;".to_string(),
        "use warnings;".to_string(),
    ]);

    assert_eq!(
        sorted,
        vec!["use strict;", "use warnings;", "use Data::Dumper;"]
    );
}

#[test]
fn sort_imports_mixed_pragma_and_cpan() {
    let sorted = sort_imports(vec![
        "use JSON;".to_string(),
        "use File::Path qw(mkpath);".to_string(),
        "use warnings;".to_string(),
        "use strict;".to_string(),
        "use Data::Dumper;".to_string(),
        "use lib './lib';".to_string(),
    ]);

    // pragmas first (sorted), then core, then CPAN (sorted), then local
    assert_eq!(
        sorted,
        vec![
            "use strict;",
            "use warnings;",
            "use JSON;",
            "use Data::Dumper;",
            "use File::Path qw(mkpath);",
            "use lib './lib';",
        ]
    );
}

#[test]
fn sort_imports_trims_whitespace_before_dedup() {
    let sorted = sort_imports(vec![
        "  use strict;".to_string(),
        "use strict;".to_string(),
        "use warnings;".to_string(),
    ]);

    assert_eq!(sorted, vec!["use strict;", "use warnings;"]);
}
