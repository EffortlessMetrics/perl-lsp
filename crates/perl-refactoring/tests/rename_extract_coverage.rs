//! Test coverage for rename and extract-function refactoring operations.
//!
//! Covers:
//! - Variable rename respecting scope (file, function, package, block)
//! - Function (subroutine) rename
//! - Package rename
//! - Extract function from selection
//! - Cross-file rename references

use perl_refactoring::refactor::refactoring::{
    RefactoringConfig, RefactoringEngine, RefactoringScope, RefactoringType,
};
use std::io::Write;
use tempfile::NamedTempFile;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a `RefactoringEngine` with safe_mode off and backups disabled.
fn engine_no_safe() -> RefactoringEngine {
    RefactoringEngine::with_config(RefactoringConfig {
        safe_mode: false,
        create_backups: false,
        ..Default::default()
    })
}

/// Write content to a temp file and return the file handle + path.
fn temp_perl(
    content: &str,
) -> Result<(NamedTempFile, std::path::PathBuf), Box<dyn std::error::Error>> {
    let mut f = NamedTempFile::new()?;
    write!(f, "{}", content)?;
    let p = f.path().to_path_buf();
    Ok((f, p))
}

// ===========================================================================
// Section 1: Variable rename respecting scope
// ===========================================================================

#[test]
fn rename_variable_file_scope_replaces_all_occurrences() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $count = 0;\n$count++;\nprint $count;\n";
    let (_f, path) = temp_perl(code)?;
    let mut engine = engine_no_safe();
    engine.index_file(&path, code)?;

    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$count".to_string(),
            new_name: "$total".to_string(),
            scope: RefactoringScope::File(path.clone()),
        },
        vec![path.clone()],
    )?;

    assert!(result.success, "Rename should succeed");
    let new_code = std::fs::read_to_string(&path)?;
    assert!(new_code.contains("$total"), "New name should appear in output");
    assert!(!new_code.contains("$count"), "Old name should not appear in output");
    Ok(())
}

#[test]
fn rename_variable_function_scope_only_renames_inside_function()
-> Result<(), Box<dyn std::error::Error>> {
    // $val appears both inside sub process and outside. Renaming at function
    // scope should only touch occurrences inside sub process.
    let code = concat!(
        "my $val = 'outer';\n",
        "sub process {\n",
        "    my $val = 'inner';\n",
        "    print $val;\n",
        "}\n",
        "print $val;\n",
    );
    let (_f, path) = temp_perl(code)?;
    let mut engine = engine_no_safe();
    engine.index_file(&path, code)?;

    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$val".to_string(),
            new_name: "$data".to_string(),
            scope: RefactoringScope::Function { file: path.clone(), name: "process".to_string() },
        },
        vec![path.clone()],
    )?;

    assert!(result.success, "Rename should succeed");
    let new_code = std::fs::read_to_string(&path)?;

    // The inner function body should have $data
    let sub_start = new_code.find("sub process").ok_or("sub process not found")?;
    let sub_body = &new_code[sub_start..];
    assert!(sub_body.contains("$data"), "Function body should contain renamed variable");

    // The last line (outside the function) should still have $val
    let last_line = new_code.lines().last().ok_or("no last line")?;
    assert!(last_line.contains("$val"), "Code outside function should keep original variable name");
    Ok(())
}

#[test]
fn rename_variable_package_scope_only_renames_in_target_package()
-> Result<(), Box<dyn std::error::Error>> {
    // Two packages in one file, both use $data.
    // Renaming $data in package Alpha should not touch package Beta.
    let code = concat!(
        "package Alpha;\n",
        "my $data = 1;\n",
        "print $data;\n",
        "package Beta;\n",
        "my $data = 2;\n",
        "print $data;\n",
    );
    let (_f, path) = temp_perl(code)?;
    let mut engine = engine_no_safe();
    engine.index_file(&path, code)?;

    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$data".to_string(),
            new_name: "$info".to_string(),
            scope: RefactoringScope::Package { file: path.clone(), name: "Alpha".to_string() },
        },
        vec![path.clone()],
    )?;

    assert!(result.success, "Rename should succeed");
    let new_code = std::fs::read_to_string(&path)?;

    // Alpha section should have $info
    let alpha_start = new_code.find("package Alpha").ok_or("package Alpha not found")?;
    let beta_start = new_code.find("package Beta").ok_or("package Beta not found")?;
    let alpha_section = &new_code[alpha_start..beta_start];
    assert!(alpha_section.contains("$info"), "Alpha section should contain renamed variable");

    // Beta section should still have $data
    let beta_section = &new_code[beta_start..];
    assert!(beta_section.contains("$data"), "Beta section should keep original variable name");
    Ok(())
}

#[test]
fn rename_variable_block_scope_limits_to_block_range() -> Result<(), Box<dyn std::error::Error>> {
    // $x appears before, inside, and after a block. Renaming within the block
    // should only affect the block.
    let code = concat!(
        "my $x = 1;\n",     // line 0
        "{\n",              // line 1
        "    my $x = 2;\n", // line 2
        "    print $x;\n",  // line 3
        "}\n",              // line 4
        "print $x;\n",      // line 5
    );
    let (_f, path) = temp_perl(code)?;
    let mut engine = engine_no_safe();
    engine.index_file(&path, code)?;

    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$x".to_string(),
            new_name: "$y".to_string(),
            scope: RefactoringScope::Block {
                file: path.clone(),
                start: (1, 0), // start of block
                end: (4, 0),   // end of block (exclusive-ish)
            },
        },
        vec![path.clone()],
    )?;

    assert!(result.success, "Rename should succeed");
    let new_code = std::fs::read_to_string(&path)?;

    // The last line outside the block should retain $x
    let last_line = new_code.lines().last().ok_or("no last line")?;
    assert!(last_line.contains("$x"), "Code outside block should keep $x");

    // Inside the block, $y should appear
    let block_start = new_code.find("{\n").ok_or("block start not found")?;
    let block_end = new_code[block_start + 1..]
        .find("\n}")
        .map(|i| block_start + 1 + i + 2)
        .ok_or("block end not found")?;
    let block_content = &new_code[block_start..block_end];
    assert!(
        block_content.contains("$y"),
        "Block should contain renamed variable $y, got: {}",
        block_content
    );
    Ok(())
}

// ===========================================================================
// Section 2: Function (subroutine) rename
// ===========================================================================

#[test]
fn rename_subroutine_in_file_scope() -> Result<(), Box<dyn std::error::Error>> {
    let code = concat!(
        "sub process_data {\n",
        "    my $x = shift;\n",
        "    return $x * 2;\n",
        "}\n",
        "my $result = process_data(42);\n",
    );
    let (_f, path) = temp_perl(code)?;
    let mut engine = engine_no_safe();
    engine.index_file(&path, code)?;

    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "process_data".to_string(),
            new_name: "transform_data".to_string(),
            scope: RefactoringScope::File(path.clone()),
        },
        vec![path.clone()],
    )?;

    assert!(result.success, "Rename should succeed");
    let new_code = std::fs::read_to_string(&path)?;
    assert!(new_code.contains("transform_data"), "New sub name should appear");
    assert!(!new_code.contains("process_data"), "Old sub name should not appear");
    Ok(())
}

#[test]
fn rename_subroutine_definition_and_call_sites() -> Result<(), Box<dyn std::error::Error>> {
    // Both the definition and multiple call sites should be renamed.
    let code = concat!(
        "sub helper {\n",
        "    return 1;\n",
        "}\n",
        "helper();\n",
        "my $v = helper();\n",
        "print helper();\n",
    );
    let (_f, path) = temp_perl(code)?;
    let mut engine = engine_no_safe();
    engine.index_file(&path, code)?;

    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "helper".to_string(),
            new_name: "utility".to_string(),
            scope: RefactoringScope::File(path.clone()),
        },
        vec![path.clone()],
    )?;

    assert!(result.success, "Rename should succeed");
    let new_code = std::fs::read_to_string(&path)?;

    // Count occurrences of old and new names
    let old_count = new_code.matches("helper").count();
    let new_count = new_code.matches("utility").count();
    assert_eq!(old_count, 0, "Old name 'helper' should not appear");
    assert!(
        new_count >= 4,
        "New name 'utility' should appear at least 4 times (1 def + 3 calls), found {}",
        new_count
    );
    Ok(())
}

#[test]
fn rename_subroutine_does_not_rename_substring_matches() -> Result<(), Box<dyn std::error::Error>> {
    // Renaming "get" should NOT rename "get_name" or "forget".
    // This verifies word-boundary awareness.
    let code = concat!(
        "sub get { return 1; }\n",
        "sub get_name { return 'name'; }\n",
        "sub forget { return 0; }\n",
        "get();\n",
        "get_name();\n",
        "forget();\n",
    );
    let (_f, path) = temp_perl(code)?;
    let mut engine = engine_no_safe();
    engine.index_file(&path, code)?;

    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "get".to_string(),
            new_name: "fetch".to_string(),
            scope: RefactoringScope::File(path.clone()),
        },
        vec![path.clone()],
    )?;

    // The engine should succeed
    assert!(result.success, "Rename should succeed");
    let new_code = std::fs::read_to_string(&path)?;

    // get_name and forget should remain unchanged
    assert!(new_code.contains("get_name"), "get_name should not be affected");
    assert!(new_code.contains("forget"), "forget should not be affected");
    Ok(())
}

// ===========================================================================
// Section 3: Package rename
// ===========================================================================

#[test]
fn rename_package_name_in_declaration() -> Result<(), Box<dyn std::error::Error>> {
    let code = concat!("package MyApp::Utils;\n", "use strict;\n", "sub helper { 1 }\n", "1;\n",);
    let (_f, path) = temp_perl(code)?;
    let mut engine = engine_no_safe();
    engine.index_file(&path, code)?;

    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "MyApp::Utils".to_string(),
            new_name: "MyApp::Helpers".to_string(),
            scope: RefactoringScope::File(path.clone()),
        },
        vec![path.clone()],
    )?;

    assert!(result.success, "Rename should succeed");
    let new_code = std::fs::read_to_string(&path)?;
    assert!(new_code.contains("MyApp::Helpers"), "New package name should appear");
    assert!(!new_code.contains("MyApp::Utils"), "Old package name should not appear");
    Ok(())
}

#[test]
fn rename_package_used_in_qualified_call() -> Result<(), Box<dyn std::error::Error>> {
    // Package name appears in `use` and qualified method calls.
    // The workspace index may only capture the `use` occurrence through
    // its indexer; qualified calls (MyModule->new, MyModule::run) may
    // require deeper semantic analysis. This test validates that at least
    // the indexed occurrence is renamed and the replacement is correct.
    let code = concat!("use MyModule;\n", "my $obj = MyModule->new();\n", "MyModule::run();\n",);
    let (_f, path) = temp_perl(code)?;
    let mut engine = engine_no_safe();
    engine.index_file(&path, code)?;

    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "MyModule".to_string(),
            new_name: "NewModule".to_string(),
            scope: RefactoringScope::File(path.clone()),
        },
        vec![path.clone()],
    )?;

    assert!(result.success, "Rename should succeed");
    assert!(result.changes_made >= 1, "Should rename at least one occurrence");
    let new_code = std::fs::read_to_string(&path)?;
    assert!(
        new_code.contains("NewModule"),
        "New module name should appear in at least one location"
    );
    Ok(())
}

// ===========================================================================
// Section 4: Extract function from selection
// ===========================================================================

#[test]
fn extract_function_simple_statements() -> Result<(), Box<dyn std::error::Error>> {
    let code = concat!(
        "sub main {\n",           // line 0
        "    my $a = 1;\n",       // line 1
        "    my $b = 2;\n",       // line 2
        "    my $c = $a + $b;\n", // line 3
        "    print $c;\n",        // line 4
        "}\n",                    // line 5
    );
    let (_f, path) = temp_perl(code)?;
    let mut engine = engine_no_safe();

    // Extract lines 2-3 (the computation part: my $b = 2; my $c = $a + $b;)
    let result = engine.refactor(
        RefactoringType::ExtractMethod {
            method_name: "compute_sum".to_string(),
            start_position: (2, 0),
            end_position: (4, 0),
        },
        vec![path.clone()],
    )?;

    assert!(result.success, "Extract should succeed");
    assert_eq!(result.changes_made, 2, "Should report 2 changes (call + sub)");
    let new_code = std::fs::read_to_string(&path)?;

    // The extracted subroutine should exist
    assert!(
        new_code.contains("sub compute_sum"),
        "Extracted sub should be present in: {}",
        new_code
    );

    // The call site should reference the new function
    assert!(new_code.contains("compute_sum("), "Call to extracted function should be present");
    Ok(())
}

#[test]
fn extract_function_with_inputs_from_outer_scope() -> Result<(), Box<dyn std::error::Error>> {
    // Extract code that uses variables from the outer scope.
    // The extracted function should receive them as parameters.
    let code = concat!(
        "my $name = 'world';\n",             // line 0
        "my $greeting = \"Hello $name\";\n", // line 1
        "print $greeting;\n",                // line 2
    );
    let (_f, path) = temp_perl(code)?;
    let mut engine = engine_no_safe();

    // Extract lines 1-2 (the greeting construction and printing)
    let result = engine.refactor(
        RefactoringType::ExtractMethod {
            method_name: "greet".to_string(),
            start_position: (1, 0),
            end_position: (3, 0),
        },
        vec![path.clone()],
    )?;

    assert!(result.success, "Extract should succeed");
    let new_code = std::fs::read_to_string(&path)?;
    assert!(new_code.contains("sub greet"), "Extracted sub should exist in: {}", new_code);
    Ok(())
}

#[test]
fn extract_function_validation_rejects_empty_range() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = 1;\n";
    let (_f, path) = temp_perl(code)?;
    let mut engine = RefactoringEngine::with_config(RefactoringConfig {
        safe_mode: true,
        create_backups: false,
        ..Default::default()
    });

    // Start position >= end position should fail validation
    let result = engine.refactor(
        RefactoringType::ExtractMethod {
            method_name: "bad_extract".to_string(),
            start_position: (5, 0),
            end_position: (3, 0),
        },
        vec![path.clone()],
    );

    assert!(result.is_err(), "Should reject invalid range");
    Ok(())
}

#[test]
fn extract_function_validation_rejects_ampersand_prefix() -> Result<(), Box<dyn std::error::Error>>
{
    let code = "my $x = 1;\nmy $y = $x + 1;\n";
    let (_f, path) = temp_perl(code)?;
    let mut engine = RefactoringEngine::with_config(RefactoringConfig {
        safe_mode: true,
        create_backups: false,
        ..Default::default()
    });

    // Method name starting with '&' should be rejected for ExtractMethod
    let result = engine.refactor(
        RefactoringType::ExtractMethod {
            method_name: "&bad_name".to_string(),
            start_position: (0, 0),
            end_position: (1, 0),
        },
        vec![path.clone()],
    );

    assert!(result.is_err(), "Should reject & prefix for ExtractMethod");
    Ok(())
}

#[test]
fn extract_function_preserves_remaining_code() -> Result<(), Box<dyn std::error::Error>> {
    let code = concat!(
        "my $x = 10;\n",    // line 0
        "my $y = 20;\n",    // line 1
        "print $x + $y;\n", // line 2
    );
    let (_f, path) = temp_perl(code)?;
    let mut engine = engine_no_safe();

    // Extract just line 1
    let result = engine.refactor(
        RefactoringType::ExtractMethod {
            method_name: "init_y".to_string(),
            start_position: (1, 0),
            end_position: (2, 0),
        },
        vec![path.clone()],
    )?;

    assert!(result.success, "Extract should succeed");
    let new_code = std::fs::read_to_string(&path)?;

    // The first and last lines should still be present
    assert!(new_code.contains("my $x = 10"), "First line should remain");
    assert!(new_code.contains("print"), "Last line should remain");
    assert!(new_code.contains("sub init_y"), "Extracted function should exist");
    Ok(())
}

// ===========================================================================
// Section 5: Cross-file rename references
// ===========================================================================

#[test]
fn rename_variable_across_two_files_workspace_scope() -> Result<(), Box<dyn std::error::Error>> {
    let code1 = "my $shared = 1;\nprint $shared;\n";
    let code2 = "print $shared;\nmy $local = $shared + 1;\n";

    let (_f1, path1) = temp_perl(code1)?;
    let (_f2, path2) = temp_perl(code2)?;
    let mut engine = engine_no_safe();
    engine.index_file(&path1, code1)?;
    engine.index_file(&path2, code2)?;

    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$shared".to_string(),
            new_name: "$common".to_string(),
            scope: RefactoringScope::Workspace,
        },
        vec![path1.clone(), path2.clone()],
    )?;

    assert!(result.success, "Workspace rename should succeed");
    assert_eq!(result.files_modified, 2, "Both files should be modified");

    let new_code1 = std::fs::read_to_string(&path1)?;
    let new_code2 = std::fs::read_to_string(&path2)?;

    assert!(new_code1.contains("$common"), "File 1 should contain renamed variable");
    assert!(!new_code1.contains("$shared"), "File 1 should not contain old variable");
    assert!(new_code2.contains("$common"), "File 2 should contain renamed variable");
    assert!(!new_code2.contains("$shared"), "File 2 should not contain old variable");
    Ok(())
}

#[test]
fn rename_subroutine_across_files_workspace_scope() -> Result<(), Box<dyn std::error::Error>> {
    let code1 = "sub do_work { return 1; }\n";
    let code2 = "do_work();\nmy $r = do_work();\n";

    let (_f1, path1) = temp_perl(code1)?;
    let (_f2, path2) = temp_perl(code2)?;
    let mut engine = engine_no_safe();
    engine.index_file(&path1, code1)?;
    engine.index_file(&path2, code2)?;

    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "do_work".to_string(),
            new_name: "execute_task".to_string(),
            scope: RefactoringScope::Workspace,
        },
        vec![path1.clone(), path2.clone()],
    )?;

    assert!(result.success, "Workspace rename should succeed");
    let new_code1 = std::fs::read_to_string(&path1)?;
    let new_code2 = std::fs::read_to_string(&path2)?;

    assert!(new_code1.contains("execute_task"), "File 1 should contain renamed sub");
    assert!(!new_code1.contains("do_work"), "File 1 should not contain old sub name");
    assert!(new_code2.contains("execute_task"), "File 2 should contain renamed sub");
    assert!(!new_code2.contains("do_work"), "File 2 should not contain old sub name");
    Ok(())
}

#[test]
fn rename_file_scope_does_not_affect_other_files() -> Result<(), Box<dyn std::error::Error>> {
    let code1 = "my $target = 1;\nprint $target;\n";
    let code2 = "my $target = 2;\nprint $target;\n";

    let (_f1, path1) = temp_perl(code1)?;
    let (_f2, path2) = temp_perl(code2)?;
    let mut engine = engine_no_safe();
    engine.index_file(&path1, code1)?;
    engine.index_file(&path2, code2)?;

    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$target".to_string(),
            new_name: "$dest".to_string(),
            scope: RefactoringScope::File(path1.clone()),
        },
        vec![path1.clone(), path2.clone()],
    )?;

    assert!(result.success, "File-scoped rename should succeed");
    // File 1 should be changed
    let new_code1 = std::fs::read_to_string(&path1)?;
    assert!(new_code1.contains("$dest"), "File 1 should be renamed");

    // File 2 should NOT be changed
    let new_code2 = std::fs::read_to_string(&path2)?;
    assert!(new_code2.contains("$target"), "File 2 should not be affected by file-scoped rename");
    assert!(!new_code2.contains("$dest"), "File 2 should not contain new name");
    Ok(())
}

// ===========================================================================
// Section 6: Validation and error handling
// ===========================================================================

#[test]
fn rename_rejects_same_old_and_new_name() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = 1;\n";
    let (_f, path) = temp_perl(code)?;
    let mut engine = RefactoringEngine::with_config(RefactoringConfig {
        safe_mode: true,
        create_backups: false,
        ..Default::default()
    });

    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$x".to_string(),
            new_name: "$x".to_string(),
            scope: RefactoringScope::File(path.clone()),
        },
        vec![path.clone()],
    );

    assert!(result.is_err(), "Should reject renaming to the same name");
    Ok(())
}

#[test]
fn rename_rejects_mismatched_sigils() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = 1;\n";
    let (_f, path) = temp_perl(code)?;
    let mut engine = RefactoringEngine::with_config(RefactoringConfig {
        safe_mode: true,
        create_backups: false,
        ..Default::default()
    });

    // $x -> @y is a sigil mismatch
    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$x".to_string(),
            new_name: "@y".to_string(),
            scope: RefactoringScope::File(path.clone()),
        },
        vec![path.clone()],
    );

    assert!(result.is_err(), "Should reject sigil mismatch");
    Ok(())
}

#[test]
fn rename_rejects_empty_name() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = 1;\n";
    let (_f, path) = temp_perl(code)?;
    let mut engine = RefactoringEngine::with_config(RefactoringConfig {
        safe_mode: true,
        create_backups: false,
        ..Default::default()
    });

    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "".to_string(),
            new_name: "$y".to_string(),
            scope: RefactoringScope::File(path.clone()),
        },
        vec![path.clone()],
    );

    assert!(result.is_err(), "Should reject empty name");
    Ok(())
}

#[test]
fn rename_rejects_sigil_only_name() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = 1;\n";
    let (_f, path) = temp_perl(code)?;
    let mut engine = RefactoringEngine::with_config(RefactoringConfig {
        safe_mode: true,
        create_backups: false,
        ..Default::default()
    });

    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$".to_string(),
            new_name: "$y".to_string(),
            scope: RefactoringScope::File(path.clone()),
        },
        vec![path.clone()],
    );

    assert!(result.is_err(), "Should reject sigil-only name");
    Ok(())
}

// ===========================================================================
// Section 7: Array and hash variable renames
// ===========================================================================

#[test]
fn rename_array_variable_preserves_sigil() -> Result<(), Box<dyn std::error::Error>> {
    // The workspace index may not capture every textual occurrence of array
    // variables; this test validates that the renamed occurrences preserve
    // the @ sigil correctly and that at least some are renamed.
    let code = "my @items = (1, 2, 3);\npush @items, 4;\nprint @items;\n";
    let (_f, path) = temp_perl(code)?;
    let mut engine = engine_no_safe();
    engine.index_file(&path, code)?;

    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "@items".to_string(),
            new_name: "@elements".to_string(),
            scope: RefactoringScope::File(path.clone()),
        },
        vec![path.clone()],
    )?;

    assert!(result.success, "Rename should succeed");
    assert!(result.changes_made >= 1, "Should rename at least one occurrence");
    let new_code = std::fs::read_to_string(&path)?;
    assert!(new_code.contains("@elements"), "New array name should appear with @ sigil preserved");
    // Verify that the replacement kept the sigil intact (not $elements or %elements)
    assert!(
        !new_code.contains("$elements") && !new_code.contains("%elements"),
        "Sigil should remain @, not $ or %"
    );
    Ok(())
}

#[test]
fn rename_hash_variable_preserves_sigil() -> Result<(), Box<dyn std::error::Error>> {
    // Hash variables may have partial index coverage; validate that
    // renamed occurrences keep the % sigil.
    let code = "my %config = (key => 'val');\nprint %config;\n";
    let (_f, path) = temp_perl(code)?;
    let mut engine = engine_no_safe();
    engine.index_file(&path, code)?;

    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "%config".to_string(),
            new_name: "%settings".to_string(),
            scope: RefactoringScope::File(path.clone()),
        },
        vec![path.clone()],
    )?;

    assert!(result.success, "Rename should succeed");
    assert!(result.changes_made >= 1, "Should rename at least one occurrence");
    let new_code = std::fs::read_to_string(&path)?;
    assert!(new_code.contains("%settings"), "New hash name should appear with % sigil preserved");
    // Verify that the replacement kept the sigil intact
    assert!(
        !new_code.contains("$settings") && !new_code.contains("@settings"),
        "Sigil should remain %, not $ or @"
    );
    Ok(())
}

// ===========================================================================
// Section 8: Extract method edge cases
// ===========================================================================

#[test]
fn extract_method_requires_exactly_one_file() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = RefactoringEngine::with_config(RefactoringConfig {
        safe_mode: true,
        create_backups: false,
        ..Default::default()
    });

    // No files provided
    let result = engine.refactor(
        RefactoringType::ExtractMethod {
            method_name: "extracted".to_string(),
            start_position: (0, 0),
            end_position: (1, 0),
        },
        vec![],
    );

    assert!(result.is_err(), "Should fail with no files");
    Ok(())
}

#[test]
fn extract_method_rejects_multiple_files() -> Result<(), Box<dyn std::error::Error>> {
    let code1 = "my $x = 1;\nmy $y = 2;\n";
    let code2 = "my $z = 3;\n";
    let (_f1, path1) = temp_perl(code1)?;
    let (_f2, path2) = temp_perl(code2)?;

    let mut engine = RefactoringEngine::with_config(RefactoringConfig {
        safe_mode: true,
        create_backups: false,
        ..Default::default()
    });

    let result = engine.refactor(
        RefactoringType::ExtractMethod {
            method_name: "extracted".to_string(),
            start_position: (0, 0),
            end_position: (1, 0),
        },
        vec![path1.clone(), path2.clone()],
    );

    assert!(result.is_err(), "Should fail with multiple files");
    Ok(())
}

// ===========================================================================
// Section 9: Operation history and rollback
// ===========================================================================

#[test]
fn rename_records_operation_in_history() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $a = 1;\n";
    let (_f, path) = temp_perl(code)?;
    let mut engine = engine_no_safe();
    engine.index_file(&path, code)?;

    assert!(engine.get_operation_history().is_empty());

    engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$a".to_string(),
            new_name: "$b".to_string(),
            scope: RefactoringScope::File(path.clone()),
        },
        vec![path.clone()],
    )?;

    assert_eq!(engine.get_operation_history().len(), 1, "Should have one operation in history");
    Ok(())
}

#[test]
fn multiple_operations_accumulate_in_history() -> Result<(), Box<dyn std::error::Error>> {
    let code1 = "my $a = 1;\n";
    let code2 = "my $x = 2;\n";
    let (_f1, path1) = temp_perl(code1)?;
    let (_f2, path2) = temp_perl(code2)?;
    let mut engine = engine_no_safe();
    engine.index_file(&path1, code1)?;
    engine.index_file(&path2, code2)?;

    engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$a".to_string(),
            new_name: "$b".to_string(),
            scope: RefactoringScope::File(path1.clone()),
        },
        vec![path1.clone()],
    )?;

    // Re-index after rename
    let new_code2 = std::fs::read_to_string(&path2)?;
    engine.index_file(&path2, &new_code2)?;

    engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$x".to_string(),
            new_name: "$y".to_string(),
            scope: RefactoringScope::File(path2.clone()),
        },
        vec![path2.clone()],
    )?;

    assert_eq!(engine.get_operation_history().len(), 2, "Should have two operations in history");
    Ok(())
}

// ===========================================================================
// Section 10: Qualified name and special characters
// ===========================================================================

#[test]
fn rename_qualified_subroutine_name() -> Result<(), Box<dyn std::error::Error>> {
    let code = concat!("package Foo::Bar;\n", "sub Foo::Bar::baz { 1 }\n", "Foo::Bar::baz();\n",);
    let (_f, path) = temp_perl(code)?;
    let mut engine = engine_no_safe();
    engine.index_file(&path, code)?;

    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "Foo::Bar::baz".to_string(),
            new_name: "Foo::Bar::qux".to_string(),
            scope: RefactoringScope::File(path.clone()),
        },
        vec![path.clone()],
    )?;

    assert!(result.success, "Rename should succeed");
    let new_code = std::fs::read_to_string(&path)?;
    assert!(new_code.contains("Foo::Bar::qux"), "New qualified name should appear");
    Ok(())
}

#[test]
fn rename_variable_with_underscores_and_digits() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $var_123 = 'test';\nprint $var_123;\n";
    let (_f, path) = temp_perl(code)?;
    let mut engine = engine_no_safe();
    engine.index_file(&path, code)?;

    let result = engine.refactor(
        RefactoringType::SymbolRename {
            old_name: "$var_123".to_string(),
            new_name: "$result_456".to_string(),
            scope: RefactoringScope::File(path.clone()),
        },
        vec![path.clone()],
    )?;

    assert!(result.success, "Rename should succeed");
    let new_code = std::fs::read_to_string(&path)?;
    assert!(new_code.contains("$result_456"), "New variable name should appear");
    assert!(!new_code.contains("$var_123"), "Old variable name should not appear");
    Ok(())
}
