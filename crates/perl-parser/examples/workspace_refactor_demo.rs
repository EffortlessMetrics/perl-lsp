//! Demo of workspace-wide refactoring capabilities
//!
//! Run with: cargo run -p perl-parser --example workspace_refactor_demo

use perl_parser::{
    workspace_index::{WorkspaceIndex, SymbolKind},
    workspace_refactor::{WorkspaceRefactor, RefactorResult},
    import_optimizer::{ImportOptimizer, ImportAnalysis},
    dead_code_detector::{DeadCodeDetector, generate_report},
};
use std::path::Path;
use std::fs;
use std::time::Instant;

fn main() {
    println!("=== Workspace-wide Refactoring Demo ===\n");

    // Create workspace index
    let mut index = WorkspaceIndex::new();
    
    // Create test files
    create_test_workspace();
    
    // Index the workspace
    println!("1. Indexing workspace files...");
    let start = Instant::now();
    
    let files = vec![
        "demo_workspace/main.pl",
        "demo_workspace/lib/Utils.pm",
        "demo_workspace/lib/Database.pm",
    ];
    
    for file in &files {
        let path = Path::new(file);
        if let Ok(content) = fs::read_to_string(path) {
            if let Err(e) = index.index_file(path, &content) {
                eprintln!("  Failed to index {}: {}", file, e);
            } else {
                println!("  ✓ Indexed {}", file);
            }
        }
    }
    
    println!("  Indexing completed in {:.2}ms\n", start.elapsed().as_secs_f64() * 1000.0);

    // Demo 1: Find symbols
    println!("2. Symbol Search Demo:");
    let symbols = index.find_symbols("process_data");
    for symbol in &symbols {
        println!("  Found: {} ({:?}) at {}:{}",
            symbol.name,
            symbol.kind,
            symbol.file_path.display(),
            symbol.line + 1
        );
    }
    
    // Demo 2: Multi-file rename
    println!("\n3. Multi-file Rename Demo:");
    let refactor = WorkspaceRefactor::new(index.clone());
    
    match refactor.rename_symbol(
        "process_data",
        "transform_data",
        Path::new("demo_workspace/lib/Utils.pm"),
        (3, 4) // Line 4, column 4
    ) {
        Ok(result) => {
            println!("  Rename 'process_data' -> 'transform_data':");
            println!("  {}", result.description);
            for edit in &result.file_edits {
                println!("    ✓ {} ({} edits)", 
                    edit.file_path.display(), 
                    edit.edits.len()
                );
            }
            if !result.warnings.is_empty() {
                println!("  Warnings:");
                for warning in &result.warnings {
                    println!("    ⚠ {}", warning);
                }
            }
        }
        Err(e) => eprintln!("  Error: {}", e),
    }

    // Demo 3: Extract module
    println!("\n4. Extract Module Demo:");
    match refactor.extract_module(
        Path::new("demo_workspace/lib/Utils.pm"),
        10, // Start line
        15, // End line
        "Utils::Helpers"
    ) {
        Ok(result) => {
            println!("  Extract lines 10-15 to 'Utils::Helpers':");
            println!("  {}", result.description);
            for edit in &result.file_edits {
                println!("    ✓ {}", edit.file_path.display());
            }
        }
        Err(e) => eprintln!("  Error: {}", e),
    }

    // Demo 4: Import optimization
    println!("\n5. Import Optimization Demo:");
    let optimizer = ImportOptimizer::new();
    
    for file in &files {
        let path = Path::new(file);
        match optimizer.analyze_file(path) {
            Ok(analysis) => {
                println!("  {}:", file);
                if !analysis.unused_imports.is_empty() {
                    println!("    Unused imports:");
                    for unused in &analysis.unused_imports {
                        println!("      - {} (line {}): {}", 
                            unused.module, 
                            unused.line + 1,
                            unused.reason
                        );
                    }
                }
                if !analysis.missing_imports.is_empty() {
                    println!("    Missing imports:");
                    for missing in &analysis.missing_imports {
                        println!("      + {} (confidence: {:.0}%)", 
                            missing.module,
                            missing.confidence * 100.0
                        );
                    }
                }
                if !analysis.duplicate_imports.is_empty() {
                    println!("    Duplicate imports:");
                    for dup in &analysis.duplicate_imports {
                        println!("      ! {} on lines {:?}", 
                            dup.module,
                            dup.lines.iter().map(|l| l + 1).collect::<Vec<_>>()
                        );
                    }
                }
            }
            Err(e) => eprintln!("    Error: {}", e),
        }
    }

    // Demo 5: Dead code detection
    println!("\n6. Dead Code Detection Demo:");
    let mut detector = DeadCodeDetector::new(index.clone());
    detector.add_entry_point("demo_workspace/main.pl".into());
    
    let analysis = detector.analyze_workspace();
    println!("  Found {} dead code items:", analysis.dead_code.len());
    println!("    Unused subroutines: {}", analysis.stats.unused_subroutines);
    println!("    Unused variables: {}", analysis.stats.unused_variables);
    println!("    Unused packages: {}", analysis.stats.unused_packages);
    
    // Show first few items
    for item in analysis.dead_code.iter().take(3) {
        if let Some(name) = &item.name {
            println!("    - {:?} '{}' at {}:{} (confidence: {:.0}%)",
                item.code_type,
                name,
                item.file_path.display(),
                item.start_line + 1,
                item.confidence * 100.0
            );
        }
    }

    // Demo 6: Optimize imports across workspace
    println!("\n7. Workspace Import Optimization:");
    match refactor.optimize_imports() {
        Ok(result) => {
            println!("  {}", result.description);
            for edit in &result.file_edits {
                println!("    ✓ {} ({} changes)", 
                    edit.file_path.display(),
                    edit.edits.len()
                );
            }
        }
        Err(e) => eprintln!("  Error: {}", e),
    }

    println!("\n=== Demo Complete ===");
    println!("\nCapabilities demonstrated:");
    println!("  ✓ Workspace-wide symbol indexing");
    println!("  ✓ Multi-file rename refactoring");
    println!("  ✓ Extract module refactoring");
    println!("  ✓ Import optimization");
    println!("  ✓ Dead code detection");
    println!("  ✓ Cross-file dependency tracking");
}

fn create_test_workspace() {
    // Create demo workspace structure
    fs::create_dir_all("demo_workspace/lib").ok();
    
    // Main script
    let main_content = r#"#!/usr/bin/perl
use strict;
use warnings;
use lib 'lib';
use Utils;
use Database;

my $data = load_data();
my $processed = Utils::process_data($data);
Database::save($processed);

print "Done\n";
"#;
    fs::write("demo_workspace/main.pl", main_content).ok();

    // Utils module
    let utils_content = r#"package Utils;
use strict;
use warnings;
use List::Util qw(max min sum);
use Data::Dumper;

sub process_data {
    my ($data) = @_;
    
    # Some processing logic
    my $max_val = max(@$data);
    my $min_val = min(@$data);
    my $sum_val = sum(@$data);
    
    return {
        max => $max_val,
        min => $min_val,
        sum => $sum_val,
    };
}

sub load_data {
    return [1, 2, 3, 4, 5];
}

sub unused_helper {
    # This function is never called
    return 42;
}

1;
"#;
    fs::write("demo_workspace/lib/Utils.pm", utils_content).ok();

    // Database module
    let db_content = r#"package Database;
use strict;
use warnings;
use DBI;

sub connect {
    # Database connection logic
    return 1;
}

sub save {
    my ($data) = @_;
    # Save data to database
    print "Saving data...\n";
    return 1;
}

sub unused_query {
    # This is dead code
    return "SELECT * FROM table";
}

1;
"#;
    fs::write("demo_workspace/lib/Database.pm", db_content).ok();
}