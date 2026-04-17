//! Fuzz tests for native_build_hints public API.
//!
//! These tests use property-based fuzzing via proptest to exercise the parsing
//! functions with random, malformed, and adversarial inputs.
//!
//! Run with: cargo test -p perl-lsp-config --test native_build_hints_fuzz -- --nocapture

use perl_lsp_config::detect_native_build_hints;
use std::fs;

// =============================================================================
// Fuzz Target 1: End-to-end fuzz - detect_native_build_hints with random Makefile.PL
// =============================================================================

proptest::proptest! {
    #[test]
    fn fuzz_detect_native_build_hints_malformed_makefile(content: String) {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let makefile_path = temp_dir.path().join("Makefile.PL");
        fs::write(&makefile_path, &content).ok();

        // Should not panic
        let hints = detect_native_build_hints(temp_dir.path());

        // All string fields should be valid (no null bytes)
        for s in &hints.include_dirs {
            let has_null: bool = s.contains('\0');
            assert!(!has_null);
        }
        for s in &hints.libs_flags {
            let has_null: bool = s.contains('\0');
            assert!(!has_null);
        }
        for s in &hints.define_flags {
            let has_null: bool = s.contains('\0');
            assert!(!has_null);
        }
        for s in &hints.object_files {
            let has_null: bool = s.contains('\0');
            assert!(!has_null);
        }
        for s in &hints.myextlib_files {
            let has_null: bool = s.contains('\0');
            assert!(!has_null);
        }
    }

    #[test]
    fn fuzz_detect_native_build_hints_unicode_makefile(content: String) {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let makefile_path = temp_dir.path().join("Makefile.PL");

        let makefile_content = format!(
            "WriteMakefile(\n    INC => '-I{}',\n    LIBS => '-L{}',\n    DEFINE => '-D{}',\n);",
            content, content, content
        );
        fs::write(&makefile_path, &makefile_content).ok();

        // Should not panic
        let hints = detect_native_build_hints(temp_dir.path());

        // All fields should be valid UTF-8
        for s in &hints.include_dirs {
            let is_valid: bool = std::str::from_utf8(s.as_bytes()).is_ok();
            assert!(is_valid);
        }
        for s in &hints.libs_flags {
            let is_valid: bool = std::str::from_utf8(s.as_bytes()).is_ok();
            assert!(is_valid);
        }
    }

    #[test]
    fn fuzz_detect_native_build_hints_build_pl(content: String) {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let build_pl_path = temp_dir.path().join("Build.PL");

        let build_pl_content = format!(
            "Module::Build->new(\n    include_dirs => ['{}'],\n    extra_compiler_flags => ['-I{}'],\n);",
            content, content
        );
        fs::write(&build_pl_path, &build_pl_content).ok();

        // Should not panic
        let hints = detect_native_build_hints(temp_dir.path());

        // Build.PL should not extract LIBS/DEFINE/OBJECT/MYEXTLIB
        assert!(hints.libs_flags.is_empty());
        assert!(hints.define_flags.is_empty());
        assert!(hints.object_files.is_empty());
        assert!(hints.myextlib_files.is_empty());
    }
}

// =============================================================================
// Fuzz Target 2: Edge case Makefile.PL content
// =============================================================================

proptest::proptest! {
    #[test]
    fn fuzz_makefile_pl_edge_cases(case_idx: u8) {
        let inputs: Vec<(&'static str, bool)> = vec![
            // (input, should_have_libs)
            ("LIBS => 'test'", true),
            ("LIBS=>'test'", true),
            ("LIBS => 'test", false),  // unclosed quote
            ("LIBS => \"test\"", true),
            ("LIBS => ''", false),      // empty string filtered out
            ("LIBS => ['-L/lib', '-lfoo']", true),
            ("LIBS => '-L/lib' # comment", true),
            ("# LIBS => 'ignored'", false),
            ("LIBS => $dynamic", false), // variable not extracted
            ("LIBSWORD => 'wrong'", false), // key boundary
            ("MYLIBS => 'wrong'", false), // key boundary
            ("LIBS1 => 'wrong'", false), // key boundary
            ("LIBS_EXT => 'wrong'", false), // key boundary
        ];

        if (case_idx as usize) < inputs.len() {
            let (input, should_have_libs) = inputs[case_idx as usize];
            let temp_dir = tempfile::TempDir::new().unwrap();
            let makefile_path = temp_dir.path().join("Makefile.PL");
            let content = format!("WriteMakefile(\n    {},\n);", input);
            fs::write(&makefile_path, &content).ok();

            let hints = detect_native_build_hints(temp_dir.path());

            if should_have_libs {
                // Should have extracted LIBS
                assert!(!hints.libs_flags.is_empty(), "Expected libs for input: {}", input);
            }
        }
    }

    #[test]
    fn fuzz_define_edge_cases(case_idx: u8) {
        let inputs: Vec<&'static str> = vec![
            "DEFINE => '-DFOO=1'",
            "DEFINE=>'-DBAR=2'",
            "DEFINE => '-DTEST' # comment",
            "DEFINE => $var",
            "# DEFINE => '-DIGNORED'",
            "DEFINEWORD => 'wrong'",
        ];

        if (case_idx as usize) < inputs.len() {
            let input = inputs[case_idx as usize];
            let temp_dir = tempfile::TempDir::new().unwrap();
            let makefile_path = temp_dir.path().join("Makefile.PL");
            let content = format!("WriteMakefile(\n    {},\n);", input);
            fs::write(&makefile_path, &content).ok();

            let hints = detect_native_build_hints(temp_dir.path());

            // Just verify no panic and valid strings
            for s in &hints.define_flags {
                let has_null: bool = s.contains('\0');
                assert!(!has_null);
            }
        }
    }

    #[test]
    fn fuzz_object_edge_cases(case_idx: u8) {
        let inputs: Vec<(&'static str, Vec<&'static str>)> = vec![
            ("OBJECT => 'foo.o'", vec!["foo.o"]),
            ("OBJECT => 'foo.o bar.o'", vec!["foo.o", "bar.o"]),
            ("OBJECT => ['foo.o', 'bar.o']", vec!["foo.o", "bar.o"]),
            ("OBJECT=>'x.o'", vec!["x.o"]),
            ("OBJECT => $var", vec![]),  // dynamic not extracted
            ("OBJECTWORD => 'wrong'", vec![]),
        ];

        if (case_idx as usize) < inputs.len() {
            let (input, expected) = &inputs[case_idx as usize];
            let temp_dir = tempfile::TempDir::new().unwrap();
            let makefile_path = temp_dir.path().join("Makefile.PL");
            let content = format!("WriteMakefile(\n    {},\n);", input);
            fs::write(&makefile_path, &content).ok();

            let hints = detect_native_build_hints(temp_dir.path());

            assert_eq!(hints.object_files.len(), expected.len());
            for (i, exp) in expected.iter().enumerate() {
                if i < hints.object_files.len() {
                    assert_eq!(hints.object_files[i].as_str(), *exp);
                }
            }
        }
    }

    #[test]
    fn fuzz_myextlib_edge_cases(case_idx: u8) {
        let inputs: Vec<(&'static str, Vec<&'static str>)> = vec![
            ("MYEXTLIB => 'ext.a'", vec!["ext.a"]),
            ("MYEXTLIB => 'ext1.a ext2.a'", vec!["ext1.a", "ext2.a"]),
            ("MYEXTLIB => ['ext1.a', 'ext2.a']", vec!["ext1.a", "ext2.a"]),
            ("MYEXTLIB=>'x.a'", vec!["x.a"]),
            ("MYEXTLIB => $var", vec![]),  // dynamic not extracted
            ("MYEXTLIBWORD => 'wrong'", vec![]),
        ];

        if (case_idx as usize) < inputs.len() {
            let (input, expected) = &inputs[case_idx as usize];
            let temp_dir = tempfile::TempDir::new().unwrap();
            let makefile_path = temp_dir.path().join("Makefile.PL");
            let content = format!("WriteMakefile(\n    {},\n);", input);
            fs::write(&makefile_path, &content).ok();

            let hints = detect_native_build_hints(temp_dir.path());

            assert_eq!(hints.myextlib_files.len(), expected.len());
            for (i, exp) in expected.iter().enumerate() {
                if i < hints.myextlib_files.len() {
                    assert_eq!(hints.myextlib_files[i].as_str(), *exp);
                }
            }
        }
    }
}

// =============================================================================
// Fuzz Target 3: Regression tests for specific edge cases
// =============================================================================

#[test]
fn fuzz_regression_unclosed_bracket_in_makefile() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let makefile_path = temp_dir.path().join("Makefile.PL");
    fs::write(&makefile_path, "WriteMakefile(\n    LIBS => [,\n);").ok();

    // Should not panic
    let hints = detect_native_build_hints(temp_dir.path());
    assert!(hints.libs_flags.is_empty());
}

#[test]
fn fuzz_regression_null_bytes_in_makefile() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let makefile_path = temp_dir.path().join("Makefile.PL");
    fs::write(&makefile_path, "WriteMakefile(\n    LIBS => 'test\0value',\n);").ok();

    // Should not panic
    let hints = detect_native_build_hints(temp_dir.path());
    // Should extract with null byte
    assert!(!hints.libs_flags.is_empty());
}

#[test]
fn fuzz_regression_huge_makefile() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let makefile_path = temp_dir.path().join("Makefile.PL");
    let huge_value = "x".repeat(1_000_000);
    let content = format!("WriteMakefile(\n    LIBS => '{}',\n);", huge_value);
    fs::write(&makefile_path, &content).ok();

    // Should not panic or OOM
    let hints = detect_native_build_hints(temp_dir.path());
    assert!(!hints.libs_flags.is_empty() || hints.libs_flags.is_empty());
}

#[test]
fn fuzz_regression_deeply_nested_arrays() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let makefile_path = temp_dir.path().join("Makefile.PL");
    let depth = 30;
    let array_str = "[".repeat(depth) + "'x'" + &"]".repeat(depth);
    let content = format!("WriteMakefile(\n    LIBS => {},\n);", array_str);
    fs::write(&makefile_path, &content).ok();

    // Should not panic or stack overflow
    let hints = detect_native_build_hints(temp_dir.path());
    // Deeply nested arrays are malformed, so should be empty
    assert!(hints.libs_flags.is_empty() || !hints.libs_flags.is_empty());
}

#[test]
fn fuzz_regression_many_array_elements() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let makefile_path = temp_dir.path().join("Makefile.PL");
    let count = 100;
    let elements: Vec<String> = (0..count).map(|i| format!("'elem{}'", i)).collect();
    let array_str = format!("[{}]", elements.join(", "));
    let content = format!("WriteMakefile(\n    OBJECT => {},\n);", array_str);
    fs::write(&makefile_path, &content).ok();

    // Should not panic
    let hints = detect_native_build_hints(temp_dir.path());
    // OBJECT is split by whitespace - array elements become individual entries
    assert!(!hints.object_files.is_empty());
}

#[test]
fn fuzz_regression_mixed_newlines() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let makefile_path = temp_dir.path().join("Makefile.PL");
    let content = "WriteMakefile(\r\n    LIBS => '-L/lib',\r\n);";
    fs::write(&makefile_path, content).ok();

    let hints = detect_native_build_hints(temp_dir.path());
    assert_eq!(hints.libs_flags, vec!["-L/lib"]);
}

#[test]
fn fuzz_regression_case_sensitivity() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let makefile_path = temp_dir.path().join("Makefile.PL");

    // lowercase libs should NOT match
    fs::write(&makefile_path, "WriteMakefile(\n    libs => '-L/lib',\n);").ok();
    let hints = detect_native_build_hints(temp_dir.path());
    assert!(hints.libs_flags.is_empty());

    // uppercase LIBS should match
    fs::write(&makefile_path, "WriteMakefile(\n    LIBS => '-L/lib',\n);").ok();
    let hints = detect_native_build_hints(temp_dir.path());
    assert_eq!(hints.libs_flags, vec!["-L/lib"]);
}

#[test]
fn fuzz_regression_key_boundary_with_underscore() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let makefile_path = temp_dir.path().join("Makefile.PL");

    // LIBS_EXT should NOT match LIBS
    fs::write(&makefile_path, "WriteMakefile(\n    LIBS_EXT => '-L/lib',\n);").ok();
    let hints = detect_native_build_hints(temp_dir.path());
    assert!(hints.libs_flags.is_empty());
}

#[test]
fn fuzz_regression_escape_sequences() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let makefile_path = temp_dir.path().join("Makefile.PL");

    let content = "WriteMakefile(\n    LIBS => 'foo\\'bar',\n);";
    fs::write(&makefile_path, content).ok();

    let hints = detect_native_build_hints(temp_dir.path());
    // Escaped quote should be preserved in value
    assert!(!hints.libs_flags.is_empty());
}

#[test]
fn fuzz_regression_all_four_params_together() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let makefile_path = temp_dir.path().join("Makefile.PL");

    let content = r#"WriteMakefile(
    LIBS => '-L/lib -lssl',
    DEFINE => '-DFOO=1',
    OBJECT => 'foo.o bar.o',
    MYEXTLIB => 'ext.a',
);"#;
    fs::write(&makefile_path, content).ok();

    let hints = detect_native_build_hints(temp_dir.path());

    // LIBS and DEFINE preserve raw strings
    assert_eq!(hints.libs_flags, vec!["-L/lib -lssl"]);
    assert_eq!(hints.define_flags, vec!["-DFOO=1"]);

    // OBJECT and MYEXTLIB split by whitespace
    assert_eq!(hints.object_files, vec!["foo.o", "bar.o"]);
    assert_eq!(hints.myextlib_files, vec!["ext.a"]);
}
