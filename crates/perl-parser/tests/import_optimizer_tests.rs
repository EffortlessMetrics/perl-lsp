use std::io::Write;

use tempfile::NamedTempFile;

use perl_parser::import_optimizer::ImportOptimizer;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn detects_unused_and_duplicate_imports() -> TestResult {
    let code = r#"use Foo qw(a b);
use Foo qw(c);
use Bar qw(x y);

a();
x();
"#;

    let mut file = NamedTempFile::new()?;
    write!(file, "{}", code)?;

    let optimizer = ImportOptimizer::new();
    let analysis = optimizer.analyze_file(file.path())?;

    // One duplicate module (Foo)
    assert_eq!(analysis.duplicate_imports.len(), 1);
    assert_eq!(analysis.duplicate_imports[0].module, "Foo");

    // Collect unused symbols per module
    let mut unused: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for u in &analysis.unused_imports {
        unused.entry(u.module.clone()).or_default().extend(u.symbols.clone());
    }
    assert_eq!(
        unused.get("Foo").ok_or("Missing Foo in unused")?.iter().cloned().collect::<std::collections::BTreeSet<_>>(),
        ["b".to_string(), "c".to_string()].into_iter().collect()
    );
    assert_eq!(
        unused.get("Bar").ok_or("Missing Bar in unused")?.iter().cloned().collect::<std::collections::BTreeSet<_>>(),
        ["y".to_string()].into_iter().collect()
    );

    // Generate optimized imports
    let optimized = optimizer.generate_optimized_imports(&analysis);
    assert!(optimized.contains("use Foo qw(a);"));
    assert!(optimized.contains("use Bar qw(x);"));
    assert!(!optimized.contains("b"));
    assert!(!optimized.contains("c"));
    assert!(!optimized.contains("y"));
    Ok(())
}

#[test]
fn handles_bare_imports_without_symbols() -> TestResult {
    let code = r#"use strict;
use warnings;
use Data::Dumper;

print "Hello\n";
"#;

    let mut file = NamedTempFile::new()?;
    write!(file, "{}", code)?;

    let optimizer = ImportOptimizer::new();
    let analysis = optimizer.analyze_file(file.path())?;

    // Should find 3 imports
    assert_eq!(analysis.imports.len(), 3);

    // All imports have empty symbols list
    for import in &analysis.imports {
        assert!(import.symbols.is_empty());
    }

    // No duplicates or unused since they don't have symbols
    assert!(analysis.duplicate_imports.is_empty());
    assert!(analysis.unused_imports.is_empty());

    // Generate optimized imports - should include all bare imports
    let optimized = optimizer.generate_optimized_imports(&analysis);
    assert!(optimized.contains("use strict;"));
    assert!(optimized.contains("use warnings;"));
    assert!(optimized.contains("use Data::Dumper;"));
    Ok(())
}

#[test]
fn handles_mixed_imports_and_usage() -> TestResult {
    let code = r#"use List::Util qw(first max min);
use Scalar::Util qw(blessed);
use Data::Dumper qw(Dumper);

my $val = first { $_ > 10 } (1, 2, 15, 3);
my $max_val = max(1, 2, 3);
print Dumper($val);
"#;

    let mut file = NamedTempFile::new()?;
    write!(file, "{}", code)?;

    let optimizer = ImportOptimizer::new();
    let analysis = optimizer.analyze_file(file.path())?;

    assert_eq!(analysis.imports.len(), 3);

    // Check that 'min' is unused from List::Util and 'blessed' is unused from Scalar::Util
    let mut unused_symbols = std::collections::HashMap::new();
    for unused in &analysis.unused_imports {
        unused_symbols.insert(unused.module.clone(), unused.symbols.clone());
    }

    assert!(unused_symbols.get("List::Util").ok_or("Missing List::Util")?.contains(&"min".to_string()));
    assert!(unused_symbols.get("Scalar::Util").ok_or("Missing Scalar::Util")?.contains(&"blessed".to_string()));

    // Generate optimized imports
    let optimized = optimizer.generate_optimized_imports(&analysis);
    assert!(optimized.contains("use List::Util qw(first max);"));
    assert!(optimized.contains("use Data::Dumper qw(Dumper);"));
    assert!(!optimized.contains("min"));
    assert!(!optimized.contains("blessed"));
    Ok(())
}

#[test]
fn handles_entirely_unused_imports() -> TestResult {
    let code = r#"use UnusedModule qw(unused_func);
use AnotherUnused qw(another_func);

print "No functions used\n";
"#;

    let mut file = NamedTempFile::new()?;
    write!(file, "{}", code)?;

    let optimizer = ImportOptimizer::new();
    let analysis = optimizer.analyze_file(file.path())?;

    // Should detect 2 unused imports
    assert_eq!(analysis.unused_imports.len(), 2);

    // Both imports are entirely unused
    for unused in &analysis.unused_imports {
        assert_eq!(unused.symbols.len(), 1);
    }

    // Generate optimized imports - should be empty since all are unused
    let optimized = optimizer.generate_optimized_imports(&analysis);
    assert!(optimized.trim().is_empty());
    Ok(())
}

#[test]
fn handles_complex_symbol_names_and_delimiters() -> TestResult {
    let code = r#"use Test::More qw( ok is like );
use File::Spec qw( catfile  catdir );

ok(1, "test passes");
my $file = catfile("a", "b");
"#;

    let mut file = NamedTempFile::new()?;
    write!(file, "{}", code)?;

    let optimizer = ImportOptimizer::new();
    let analysis = optimizer.analyze_file(file.path())?;

    assert_eq!(analysis.imports.len(), 2);

    // Should detect unused symbols: 'is', 'like', 'catdir'
    let mut all_unused = Vec::new();
    for unused in &analysis.unused_imports {
        all_unused.extend(unused.symbols.clone());
    }

    assert!(all_unused.contains(&"is".to_string()));
    assert!(all_unused.contains(&"like".to_string()));
    assert!(all_unused.contains(&"catdir".to_string()));

    let optimized = optimizer.generate_optimized_imports(&analysis);
    assert!(optimized.contains("use Test::More qw(ok);"));
    assert!(optimized.contains("use File::Spec qw(catfile);"));
    Ok(())
}

#[test]
fn handles_empty_file() -> TestResult {
    let code = "";

    let mut file = NamedTempFile::new()?;
    write!(file, "{}", code)?;

    let optimizer = ImportOptimizer::new();
    let analysis = optimizer.analyze_file(file.path())?;

    assert!(analysis.imports.is_empty());
    assert!(analysis.unused_imports.is_empty());
    assert!(analysis.duplicate_imports.is_empty());

    let optimized = optimizer.generate_optimized_imports(&analysis);
    assert!(optimized.trim().is_empty());
    Ok(())
}

#[test]
fn handles_symbols_used_in_comments() -> TestResult {
    let code = r#"use Test qw(func);

# func is mentioned in comment but not actually used
print "Hello\n";
"#;

    let mut file = NamedTempFile::new()?;
    write!(file, "{}", code)?;

    let optimizer = ImportOptimizer::new();
    let analysis = optimizer.analyze_file(file.path())?;

    // Should detect 'func' as unused even though it's in comment
    assert_eq!(analysis.unused_imports.len(), 1);
    assert!(analysis.unused_imports[0].symbols.contains(&"func".to_string()));
    Ok(())
}

#[test]
fn preserves_order_in_optimized_output() -> TestResult {
    let code = r#"use Zebra qw(z);
use Alpha qw(a);
use Beta qw(b);

a();
b();
z();
"#;

    let mut file = NamedTempFile::new()?;
    write!(file, "{}", code)?;

    let optimizer = ImportOptimizer::new();
    let analysis = optimizer.analyze_file(file.path())?;

    let optimized = optimizer.generate_optimized_imports(&analysis);

    // Should be sorted alphabetically by module name
    let lines: Vec<&str> = optimized.trim().split('\n').collect();
    assert!(lines[0].contains("Alpha"));
    assert!(lines[1].contains("Beta"));
    assert!(lines[2].contains("Zebra"));
    Ok(())
}
