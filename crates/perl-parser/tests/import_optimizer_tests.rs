use std::io::Write;

use tempfile::NamedTempFile;

use perl_parser::import_optimizer::ImportOptimizer;

#[test]
fn detects_unused_and_duplicate_imports() {
    let code = r#"use Foo qw(a b);
use Foo qw(c);
use Bar qw(x y);

a();
x();
"#;

    let mut file = NamedTempFile::new().unwrap();
    write!(file, "{}", code).unwrap();

    let optimizer = ImportOptimizer::new();
    let analysis = optimizer.analyze_file(file.path()).unwrap();

    // One duplicate module (Foo)
    assert_eq!(analysis.duplicate_imports.len(), 1);
    assert_eq!(analysis.duplicate_imports[0].module, "Foo");

    // Collect unused symbols per module
    let mut unused: std::collections::BTreeMap<String, Vec<String>> = std::collections::BTreeMap::new();
    for u in &analysis.unused_imports {
        unused.entry(u.module.clone()).or_default().extend(u.symbols.clone());
    }
    assert_eq!(unused.get("Foo").unwrap().iter().cloned().collect::<std::collections::BTreeSet<_>>(),
               ["b".to_string(), "c".to_string()].into_iter().collect());
    assert_eq!(unused.get("Bar").unwrap().iter().cloned().collect::<std::collections::BTreeSet<_>>(),
               ["y".to_string()].into_iter().collect());

    // Generate optimized imports
    let optimized = optimizer.generate_optimized_imports(&analysis);
    assert!(optimized.contains("use Foo qw(a);"));
    assert!(optimized.contains("use Bar qw(x);"));
    assert!(!optimized.contains("b"));
    assert!(!optimized.contains("c"));
    assert!(!optimized.contains("y"));
}
