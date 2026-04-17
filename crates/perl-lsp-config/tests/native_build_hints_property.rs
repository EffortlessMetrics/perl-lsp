//! Property-based tests for native_build_hints extraction logic.
//!
//! These tests verify invariants that should hold across all possible inputs,
//! not just the specific examples in unit tests.

use perl_lsp_config::{NativeBuildHints, detect_native_build_hints};
use std::fs;
use std::path::Path;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn write_script(dir: &tempfile::TempDir, name: &str, content: &str) -> TestResult {
    fs::write(dir.path().join(name), content)?;
    Ok(())
}

/// Property: Uniqueness - collect_unique never produces duplicates
/// Invariant: For any Makefile.PL with duplicate literal LIBS values,
/// the result should contain each unique value exactly once.
#[test]
fn property_libs_no_duplicates() -> TestResult {
    for num_dups in 1..=10 {
        let dir = tempfile::tempdir()?;
        let mut content = "WriteMakefile(\n".to_string();
        for i in 0..num_dups {
            content.push_str(&format!("    LIBS => '-L/lib{} -lfoo{}',\\n", i, i));
        }
        content.push_str(");\n");
        write_script(&dir, "Makefile.PL", &content)?;

        let hints = detect_native_build_hints(dir.path());
        // Count occurrences of each unique value
        let libs_count = hints.libs_flags.len();
        let unique_count = hints.libs_flags.iter().collect::<std::collections::HashSet<_>>().len();

        assert_eq!(
            libs_count, unique_count,
            "LIBS has {} entries but only {} unique; duplicates found: {:?}",
            libs_count, unique_count, hints.libs_flags
        );
    }
    Ok(())
}

/// Property: Uniqueness for OBJECT (whitespace-split entries)
/// Invariant: When OBJECT contains space-separated duplicates,
/// each unique object file appears exactly once in result.
#[test]
fn property_object_no_duplicates_after_split() -> TestResult {
    for trial in 0..50 {
        let dir = tempfile::tempdir()?;
        // Generate OBJECT with some duplicates
        let objects = ["foo.o", "bar.o", "baz.o", "foo.o", "bar.o"];
        let content = format!("WriteMakefile(\n    OBJECT => '{}',\n);\n", objects.join(" "));
        write_script(&dir, "Makefile.PL", &content)?;

        let hints = detect_native_build_hints(dir.path());
        let unique_count =
            hints.object_files.iter().collect::<std::collections::HashSet<_>>().len();

        // Should have exactly 3 unique object files
        assert_eq!(
            hints.object_files.len(),
            unique_count,
            "Trial {}: OBJECT has {} entries but only {} unique",
            trial,
            hints.object_files.len(),
            unique_count
        );
    }
    Ok(())
}

/// Property: Empty values are never added to results
/// Invariant: For any key with an empty quoted value (no content at all),
/// the result field remains empty. Note: whitespace-only strings behavior
/// differs by field type - LIBS/DEFINE preserve them, OBJECT/MYEXTLIB don't
/// because they split on whitespace.
#[test]
fn property_empty_values_filtered() -> TestResult {
    // Truly empty - nothing between quotes - should always produce empty results
    let empty_values = ["''", "\"\""];
    // Whitespace-only - behavior differs:
    // - LIBS/DEFINE: preserved (raw string preserved)
    // - OBJECT/MYEXTLIB: filtered out (split_whitespace on "  " returns nothing)
    let whitespace_values_for_libs_define = ["'  '", "\"  \""];
    let whitespace_values_for_object_myextlib = ["'  '", "\"  \""];

    // Empty values should produce empty results for all fields
    for value in empty_values {
        for field in ["LIBS", "DEFINE", "OBJECT", "MYEXTLIB"] {
            let dir = tempfile::tempdir()?;
            let content = format!("WriteMakefile(\n    {} => {},\n);\n", field, value);
            write_script(&dir, "Makefile.PL", &content)?;

            let hints = detect_native_build_hints(dir.path());
            let result: &Vec<String> = match field {
                "LIBS" => &hints.libs_flags,
                "DEFINE" => &hints.define_flags,
                "OBJECT" => &hints.object_files,
                "MYEXTLIB" => &hints.myextlib_files,
                _ => unreachable!(),
            };

            assert!(
                result.is_empty(),
                "Field {} with empty value {:?} should be empty, got {:?}",
                field,
                value,
                result
            );
        }
    }

    // Whitespace-only values for LIBS and DEFINE ARE preserved
    for value in whitespace_values_for_libs_define {
        for field in ["LIBS", "DEFINE"] {
            let dir = tempfile::tempdir()?;
            let content = format!("WriteMakefile(\n    {} => {},\n);\n", field, value);
            write_script(&dir, "Makefile.PL", &content)?;

            let hints = detect_native_build_hints(dir.path());
            let result: &Vec<String> = match field {
                "LIBS" => &hints.libs_flags,
                "DEFINE" => &hints.define_flags,
                _ => unreachable!(),
            };

            assert!(
                !result.is_empty(),
                "Field {} with whitespace-only value {:?} should be preserved, got {:?}",
                field,
                value,
                result
            );
        }
    }

    // Whitespace-only values for OBJECT and MYEXTLIB are filtered (split_whitespace -> empty)
    for value in whitespace_values_for_object_myextlib {
        for field in ["OBJECT", "MYEXTLIB"] {
            let dir = tempfile::tempdir()?;
            let content = format!("WriteMakefile(\n    {} => {},\n);\n", field, value);
            write_script(&dir, "Makefile.PL", &content)?;

            let hints = detect_native_build_hints(dir.path());
            let result: &Vec<String> = match field {
                "OBJECT" => &hints.object_files,
                "MYEXTLIB" => &hints.myextlib_files,
                _ => unreachable!(),
            };

            // whitespace-only string splits to nothing
            assert!(
                result.is_empty(),
                "Field {} with whitespace-only value {:?} should be empty after split, got {:?}",
                field,
                value,
                result
            );
        }
    }
    Ok(())
}

/// Property: Key boundary correctness - partial keys should NOT match
/// Invariant: Keys like LIBSWORD, ELIBS, MYLIBS should NOT cause LIBS extraction.
#[test]
fn property_key_boundary_no_partial_match() -> TestResult {
    // Keys that contain LIBS as substring but are not exactly LIBS
    let invalid_keys = [
        "LIBSWORD",
        "MYLIBS",
        "ELIBS",
        "LIBSS",
        "LIBS1",
        "LIBS_EXT",
        "LIBS_FLAG",
        "OBLIBS",
        "ALIBS",
        "LIBS_",
        "_LIBS",
    ];

    for key in &invalid_keys {
        let dir = tempfile::tempdir()?;
        let content = format!("WriteMakefile(\n    {} => '-L/lib -lfoo',\n);\n", key);
        write_script(&dir, "Makefile.PL", &content)?;

        let hints = detect_native_build_hints(dir.path());
        assert!(
            hints.libs_flags.is_empty(),
            "Key '{}' should NOT match LIBS, but got libs_flags: {:?}",
            key,
            hints.libs_flags
        );
    }
    Ok(())
}

/// Property: Same key boundary correctness for OBJECT
/// Invariant: Keys like OBJECTS, MYOBJECT, OBJECT_EXT should NOT cause OBJECT extraction.
#[test]
fn property_object_key_boundary_no_partial_match() -> TestResult {
    let invalid_keys =
        ["OBJECTS", "MYOBJECT", "OBJECT_EXT", "OBJECTX", "XOBJECT", "OBJECT_", "OBJ", "OBJECTA"];

    for key in &invalid_keys {
        let dir = tempfile::tempdir()?;
        let content = format!("WriteMakefile(\n    {} => 'foo.o',\n);\n", key);
        write_script(&dir, "Makefile.PL", &content)?;

        let hints = detect_native_build_hints(dir.path());
        assert!(
            hints.object_files.is_empty(),
            "Key '{}' should NOT match OBJECT, but got object_files: {:?}",
            key,
            hints.object_files
        );
    }
    Ok(())
}

/// Property: Idempotency - running extraction twice gives same result
/// Invariant: detect_native_build_hints is a pure function; calling it multiple
/// times on the same directory should produce identical results.
#[test]
fn property_idempotent_extraction() -> TestResult {
    for trial in 0..100 {
        let dir = tempfile::tempdir()?;
        let content = format!(
            "WriteMakefile(\n    LIBS => '-L/lib{} -lfoo{}',\n    DEFINE => '-DFOO={}',\n    OBJECT => 'foo{}.o bar{}.o',\n    MYEXTLIB => 'ext{}.a',\n);\n",
            trial, trial, trial, trial, trial, trial
        );
        write_script(&dir, "Makefile.PL", &content)?;

        let hints1 = detect_native_build_hints(dir.path());
        let hints2 = detect_native_build_hints(dir.path());

        assert_eq!(
            hints1, hints2,
            "Trial {}: Second extraction produced different result:\n  First:  {:?}\n  Second: {:?}",
            trial, hints1, hints2
        );
    }
    Ok(())
}

/// Property: Build.PL never extracts LIBS/DEFINE/OBJECT/MYEXTLIB
/// Invariant: Even if Makefile.PL doesn't exist, Build.PL with these
/// keys should NOT populate the new fields.
#[test]
fn property_build_pl_never_extracts_new_fields() -> TestResult {
    // Create only Build.PL (no Makefile.PL)
    let dir = tempfile::tempdir()?;
    write_script(
        &dir,
        "Build.PL",
        "Module::Build->new(\n    module_name => 'Foo',\n    LIBS => ['-lssl', '-lcrypto'],\n    DEFINE => '-DFOO=1 -DBAR=2',\n    OBJECT => 'foo.o bar.o',\n    MYEXTLIB => 'ext.a',\n);\n",
    )?;

    let hints = detect_native_build_hints(dir.path());
    assert!(
        hints.libs_flags.is_empty(),
        "Build.PL should never extract LIBS, got: {:?}",
        hints.libs_flags
    );
    assert!(
        hints.define_flags.is_empty(),
        "Build.PL should never extract DEFINE, got: {:?}",
        hints.define_flags
    );
    assert!(
        hints.object_files.is_empty(),
        "Build.PL should never extract OBJECT, got: {:?}",
        hints.object_files
    );
    assert!(
        hints.myextlib_files.is_empty(),
        "Build.PL should never extract MYEXTLIB, got: {:?}",
        hints.myextlib_files
    );
    Ok(())
}

/// Property: Whitespace splitting for OBJECT is consistent
/// Invariant: OBJECT values should split on any whitespace boundary
/// (space, tab, multiple spaces) producing the same set of tokens.
/// Note: Newlines inside quotes are NOT valid Perl syntax.
#[test]
fn property_object_whitespace_split_consistent() -> TestResult {
    let test_cases = vec![
        ("foo.o bar.o baz.o", vec!["foo.o", "bar.o", "baz.o"]),
        ("foo.o\tbar.o\tbaz.o", vec!["foo.o", "bar.o", "baz.o"]),
        ("foo.o   bar.o     baz.o", vec!["foo.o", "bar.o", "baz.o"]),
        ("  foo.o bar.o  ", vec!["foo.o", "bar.o"]),
    ];

    for (input, expected) in test_cases {
        let dir = tempfile::tempdir()?;
        let content = format!("WriteMakefile(\n    OBJECT => '{}',\n);\n", input);
        write_script(&dir, "Makefile.PL", &content)?;

        let hints = detect_native_build_hints(dir.path());
        assert_eq!(
            hints.object_files, expected,
            "OBJECT '{}' should split to {:?}, got {:?}",
            input, expected, hints.object_files
        );
    }
    Ok(())
}

/// Property: MYEXTLIB whitespace splitting matches OBJECT behavior
/// Invariant: MYEXTLIB splits on whitespace just like OBJECT does.
#[test]
fn property_myextlib_whitespace_split_consistent() -> TestResult {
    let test_cases = vec![
        ("ext1.a ext2.a ext3.a", vec!["ext1.a", "ext2.a", "ext3.a"]),
        ("ext1.a\t\text2.a", vec!["ext1.a", "ext2.a"]),
        ("  ext1.a  ext2.a  ", vec!["ext1.a", "ext2.a"]),
    ];

    for (input, expected) in test_cases {
        let dir = tempfile::tempdir()?;
        let content = format!("WriteMakefile(\n    MYEXTLIB => '{}',\n);\n", input);
        write_script(&dir, "Makefile.PL", &content)?;

        let hints = detect_native_build_hints(dir.path());
        assert_eq!(
            hints.myextlib_files, expected,
            "MYEXTLIB '{}' should split to {:?}, got {:?}",
            input, expected, hints.myextlib_files
        );
    }
    Ok(())
}

/// Property: LIBS and DEFINE preserve raw strings (no whitespace splitting)
/// Invariant: LIBS and DEFINE values are kept as single entries,
/// regardless of internal whitespace. Only one entry per key occurrence.
#[test]
fn property_libs_define_no_internal_split() -> TestResult {
    let test_cases = vec![
        ("LIBS", "'-L/lib -lssl -lcrypto'"),
        ("LIBS", "'-L /lib -lssl'"), // space after -L
        ("DEFINE", "'-DFOO=1 -DBAR=2'"),
        ("DEFINE", "'-D FLAG -D OTHER'"),
    ];

    for (field, value) in &test_cases {
        let dir = tempfile::tempdir()?;
        let content = format!("WriteMakefile(\n    {} => {},\n);\n", field, value);
        write_script(&dir, "Makefile.PL", &content)?;

        let hints = detect_native_build_hints(dir.path());
        let result: &Vec<String> = match *field {
            "LIBS" => &hints.libs_flags,
            "DEFINE" => &hints.define_flags,
            _ => unreachable!(),
        };

        // Should have exactly 1 entry (the whole string is preserved, not split)
        assert_eq!(
            result.len(),
            1,
            "Field {} with value {} should produce 1 entry, got {:?}",
            field,
            value,
            result
        );
    }
    Ok(())
}

/// Property: Quote content preservation - extracted content excludes quotes
/// Invariant: The extracted value should contain exactly the content
/// between quotes, not including the quotes themselves.
/// Exception: When quotes are part of the string content (nested quotes),
/// they are preserved as literal characters.
#[test]
fn property_quote_content_excludes_quotes() -> TestResult {
    // Test cases where outer quotes are correctly stripped
    let test_cases = vec![
        ("LIBS", "'-L/lib'", "-L/lib"),
        ("LIBS", "\"-L/lib\"", "-L/lib"),
        ("DEFINE", "'-DFOO=1'", "-DFOO=1"),
        ("DEFINE", "\"-DFOO=1\"", "-DFOO=1"),
        ("OBJECT", "'foo.o'", "foo.o"),
        ("OBJECT", "\"foo.o\"", "foo.o"),
        ("MYEXTLIB", "'ext.a'", "ext.a"),
        ("MYEXTLIB", "\"ext.a\"", "ext.a"),
    ];

    for (field, input, expected_content) in test_cases {
        let dir = tempfile::tempdir()?;
        let content = format!("WriteMakefile(\n    {} => {},\n);\n", field, input);
        write_script(&dir, "Makefile.PL", &content)?;

        let hints = detect_native_build_hints(dir.path());
        let result: &Vec<String> = match field {
            "LIBS" => &hints.libs_flags,
            "DEFINE" => &hints.define_flags,
            "OBJECT" => &hints.object_files,
            "MYEXTLIB" => &hints.myextlib_files,
            _ => unreachable!(),
        };

        assert!(
            !result.is_empty(),
            "Field {} with input {} should produce non-empty result",
            field,
            input
        );

        // The extracted content should NOT contain the outer quote characters
        for entry in result {
            assert!(
                !entry.starts_with('\'') && !entry.starts_with('"'),
                "Entry {:?} should not start with quote character",
                entry
            );
            assert!(
                !entry.ends_with('\'') && !entry.ends_with('"'),
                "Entry {:?} should not end with quote character",
                entry
            );
        }
    }
    Ok(())
}

/// Property: Array elements are all extracted
/// Invariant: An array like ['a', 'b', 'c'] should produce 3 entries
/// for the corresponding field.
#[test]
fn property_array_flattens_all_elements() -> TestResult {
    let test_cases = vec![
        ("LIBS", "['-L/lib1', '-L/lib2', '-L/lib3']", 3),
        ("DEFINE", "['-DFOO', '-DBAR', '-DBAZ']", 3),
        ("OBJECT", "['foo.o', 'bar.o', 'baz.o']", 3),
        ("MYEXTLIB", "['ext1.a', 'ext2.a']", 2),
    ];

    for (field, array_syntax, expected_count) in test_cases {
        let dir = tempfile::tempdir()?;
        let content = format!("WriteMakefile(\n    {} => {},\n);\n", field, array_syntax);
        write_script(&dir, "Makefile.PL", &content)?;

        let hints = detect_native_build_hints(dir.path());
        let result: &Vec<String> = match field {
            "LIBS" => &hints.libs_flags,
            "DEFINE" => &hints.define_flags,
            "OBJECT" => &hints.object_files,
            "MYEXTLIB" => &hints.myextlib_files,
            _ => unreachable!(),
        };

        assert_eq!(
            result.len(),
            expected_count,
            "Field {} with array {} should produce {} entries, got {:?}",
            field,
            array_syntax,
            expected_count,
            result
        );
    }
    Ok(())
}

/// Property: Order preservation - values appear in source order
/// Invariant: For multiple occurrences of the same key, the extracted
/// values should appear in the same order as in the source.
#[test]
fn property_order_preserved() -> TestResult {
    let dir = tempfile::tempdir()?;
    write_script(
        &dir,
        "Makefile.PL",
        "WriteMakefile(\n    LIBS => '-L/lib1',\n    LIBS => '-L/lib2',\n    LIBS => '-L/lib3',\n);\n",
    )?;

    let hints = detect_native_build_hints(dir.path());
    assert_eq!(
        hints.libs_flags,
        vec!["-L/lib1", "-L/lib2", "-L/lib3"],
        "LIBS values should appear in source order"
    );
    Ok(())
}

/// Property: Monotonicity - adding INC doesn't remove LIBS
/// Invariant: Adding additional keys to Makefile.PL should not cause
/// previously extracted values to disappear.
#[test]
fn property_monotonic_extraction() -> TestResult {
    let dir = tempfile::tempdir()?;
    // Start with LIBS only
    write_script(&dir, "Makefile.PL", "WriteMakefile(\n    LIBS => '-L/lib1',\n);\n")?;
    let hints1 = detect_native_build_hints(dir.path());
    let libs_count_1 = hints1.libs_flags.len();

    // Add more keys
    write_script(
        &dir,
        "Makefile.PL",
        "WriteMakefile(\n    LIBS => '-L/lib1',\n    INC => '-Iinclude',\n    DEFINE => '-DFOO=1',\n    OBJECT => 'foo.o',\n    MYEXTLIB => 'ext.a',\n);\n",
    )?;
    let hints2 = detect_native_build_hints(dir.path());
    let libs_count_2 = hints2.libs_flags.len();

    assert!(
        libs_count_2 >= libs_count_1,
        "Adding more keys should not reduce LIBS count: {} -> {}",
        libs_count_1,
        libs_count_2
    );
    Ok(())
}

/// Property: Escape sequences inside quotes are handled
/// Invariant: A backslash inside quotes should be treated as an escape
/// and the escaped character should appear in the output.
#[test]
fn property_escape_sequences_preserved() -> TestResult {
    // Test with a path that might need escaping
    let dir = tempfile::tempdir()?;
    write_script(
        &dir,
        "Makefile.PL",
        "WriteMakefile(\n    LIBS => '-L/path\\\\with\\\\spaces',\n);\n",
    )?;

    let hints = detect_native_build_hints(dir.path());
    // The escaped backslashes should be preserved in the output
    assert!(!hints.libs_flags.is_empty(), "LIBS with escaped backslashes should be extracted");
    Ok(())
}

/// Property: Missing Makefile.PL and Build.PL produces empty hints
/// Invariant: When neither Makefile.PL nor Build.PL exists in the
/// workspace root, detect_native_build_hints returns default (empty) hints.
#[test]
fn property_missing_both_files_returns_empty() -> TestResult {
    let dir = tempfile::tempdir()?;
    // Don't create any files

    let hints = detect_native_build_hints(dir.path());
    let default = NativeBuildHints::default();

    assert_eq!(hints, default, "Missing both Makefile.PL and Build.PL should return default hints");
    Ok(())
}

/// Property: Comment on same line as key=value is handled correctly
/// Invariant: A comment marker (#) on the same line AFTER the value
/// should not corrupt the extracted value.
#[test]
fn property_comment_after_value_not_included() -> TestResult {
    let dir = tempfile::tempdir()?;
    // The # after '-lssl' is inside the string, not a comment
    write_script(
        &dir,
        "Makefile.PL",
        "WriteMakefile(\n    LIBS => '-L/lib -lssl # this is not a comment',\n);\n",
    )?;

    let hints = detect_native_build_hints(dir.path());
    // The value should include the # character since it's inside quotes
    assert!(!hints.libs_flags.is_empty(), "LIBS with # inside quotes should be extracted");
    // The extracted value should contain the #
    assert!(
        hints.libs_flags[0].contains('#'),
        "LIBS value should contain # since it's inside quotes: {:?}",
        hints.libs_flags
    );
    Ok(())
}

/// Property: Single key occurrence returns single value
/// Invariant: A key that appears exactly once with exactly one value
/// should produce a result vector with exactly one entry.
#[test]
fn property_single_occurrence_single_value() -> TestResult {
    let fields = vec![
        ("LIBS", "'-L/lib'"),
        ("DEFINE", "'-DFOO=1'"),
        ("OBJECT", "'foo.o'"),
        ("MYEXTLIB", "'ext.a'"),
    ];

    for (field, value) in fields {
        let dir = tempfile::tempdir()?;
        let content = format!("WriteMakefile(\n    {} => {},\n);\n", field, value);
        write_script(&dir, "Makefile.PL", &content)?;

        let hints = detect_native_build_hints(dir.path());
        let result = match field {
            "LIBS" => &hints.libs_flags,
            "DEFINE" => &hints.define_flags,
            "OBJECT" => &hints.object_files,
            "MYEXTLIB" => &hints.myextlib_files,
            _ => unreachable!(),
        };

        assert_eq!(
            result.len(),
            1,
            "Field {} with single value {} should produce exactly 1 entry, got {:?}",
            field,
            value,
            result
        );
    }
    Ok(())
}
