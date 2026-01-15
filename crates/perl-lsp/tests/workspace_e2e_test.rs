mod support;

#[cfg(all(feature = "workspace", feature = "expose_lsp_test_api"))]
#[test]
#[serial_test::serial]
fn test_goto_definition_across_files() {
    use lsp_types::Position;
    use perl_lsp::LspServer;
    use serde_json::json;
    use std::fs;
    use support::env_guard::EnvGuard;
    use tempfile::tempdir;

    // Enable workspace indexing
    // SAFETY: Test runs single-threaded with #[serial_test::serial]
    let _guard = unsafe { EnvGuard::set("PERL_LSP_WORKSPACE", "1") };

    // Create temporary directory structure
    let dir = tempdir().unwrap();
    let root = dir.path();

    // Create lib/Foo/Bar.pm
    let bar_dir = root.join("lib").join("Foo");
    std::fs::create_dir_all(&bar_dir).unwrap();
    let bar_file = bar_dir.join("Bar.pm");
    fs::write(&bar_file, "package Foo::Bar;\n\nsub baz {\n    return 42;\n}\n\n1;\n").unwrap();

    // Create script/main.pl
    let script_dir = root.join("script");
    std::fs::create_dir_all(&script_dir).unwrap();
    let main_file = script_dir.join("main.pl");
    fs::write(
        &main_file,
        "use lib 'lib';\nuse Foo::Bar;\n\nmy $result = Foo::Bar::baz();\nprint $result;\n",
    )
    .unwrap();

    // Create URIs with proper encoding
    let bar_uri = url::Url::from_file_path(&bar_file).unwrap().to_string();
    let main_uri = url::Url::from_file_path(&main_file).unwrap().to_string();

    // Create LSP server with test output
    use std::io::Cursor;
    use std::sync::{Arc, Mutex};
    let output =
        Arc::new(Mutex::new(Box::new(Cursor::new(Vec::new())) as Box<dyn std::io::Write + Send>));
    let srv = LspServer::with_output(output.clone());

    // Open both files to index them
    srv.test_handle_did_open(Some(json!({
        "textDocument": {
            "uri": bar_uri.clone(),
            "text": fs::read_to_string(&bar_file).unwrap(),
            "version": 1
        }
    })))
    .unwrap();

    srv.test_handle_did_open(Some(json!({
        "textDocument": {
            "uri": main_uri.clone(),
            "text": fs::read_to_string(&main_file).unwrap(),
            "version": 1
        }
    })))
    .unwrap();

    // Test: Go to definition on "baz" in "Foo::Bar::baz()"
    // Position is line 3, character 24 (the 'b' in 'baz')
    let pos = Position { line: 3, character: 24 };
    let result = srv
        .test_handle_definition(Some(json!({
            "textDocument": {"uri": main_uri.clone()},
            "position": pos
        })))
        .unwrap();

    // Check result
    if let Some(defs) = result {
        let defs_array = defs.as_array().expect("Expected array of definitions");
        assert!(!defs_array.is_empty(), "Should find definition");

        // The definition should point to Bar.pm
        let first_def = &defs_array[0];
        let def_uri = first_def["uri"].as_str().unwrap();
        assert!(def_uri.contains("Bar.pm"), "Definition should be in Bar.pm, got: {}", def_uri);
    } else {
        panic!("No definitions found");
    }
}

#[cfg(all(feature = "workspace", feature = "expose_lsp_test_api"))]
#[test]
#[serial_test::serial]
fn test_find_references_across_files() {
    use lsp_types::Position;
    use perl_lsp::LspServer;
    use serde_json::json;
    use std::fs;
    use support::env_guard::EnvGuard;
    use tempfile::tempdir;

    // Enable workspace indexing
    // SAFETY: Test runs single-threaded with #[serial_test::serial]
    let _guard = unsafe { EnvGuard::set("PERL_LSP_WORKSPACE", "1") };

    // Create temporary directory structure
    let dir = tempdir().unwrap();
    let root = dir.path();

    // Create lib/Utils.pm
    let lib_dir = root.join("lib");
    std::fs::create_dir_all(&lib_dir).unwrap();
    let utils_file = lib_dir.join("Utils.pm");
    fs::write(&utils_file, "package Utils;\n\nsub process_data {\n    my ($data) = @_;\n    return $data * 2;\n}\n\n1;\n").unwrap();

    // Create script1.pl
    let script1_file = root.join("script1.pl");
    fs::write(
        &script1_file,
        "use lib 'lib';\nuse Utils;\n\nmy $result = Utils::process_data(21);\nprint $result;\n",
    )
    .unwrap();

    // Create script2.pl
    let script2_file = root.join("script2.pl");
    fs::write(&script2_file, "use lib 'lib';\nuse Utils;\n\nmy $value = Utils::process_data(100);\nmy $doubled = Utils::process_data($value);\nprint $doubled;\n").unwrap();

    // Create URIs with proper encoding
    let utils_uri = url::Url::from_file_path(&utils_file).unwrap().to_string();
    let script1_uri = url::Url::from_file_path(&script1_file).unwrap().to_string();
    let script2_uri = url::Url::from_file_path(&script2_file).unwrap().to_string();

    // Create LSP server
    use std::io::Cursor;
    use std::sync::{Arc, Mutex};
    let output =
        Arc::new(Mutex::new(Box::new(Cursor::new(Vec::new())) as Box<dyn std::io::Write + Send>));
    let srv = LspServer::with_output(output.clone());

    // Open all files to index them
    srv.test_handle_did_open(Some(json!({
        "textDocument": {
            "uri": utils_uri.clone(),
            "text": fs::read_to_string(&utils_file).unwrap(),
            "version": 1
        }
    })))
    .unwrap();

    srv.test_handle_did_open(Some(json!({
        "textDocument": {
            "uri": script1_uri.clone(),
            "text": fs::read_to_string(&script1_file).unwrap(),
            "version": 1
        }
    })))
    .unwrap();

    srv.test_handle_did_open(Some(json!({
        "textDocument": {
            "uri": script2_uri.clone(),
            "text": fs::read_to_string(&script2_file).unwrap(),
            "version": 1
        }
    })))
    .unwrap();

    // Test: Find all references to "process_data" from Utils.pm
    // Position is line 2, character 5 (inside 'process_data' in Utils.pm)
    let pos = Position { line: 2, character: 5 };
    let result = srv
        .test_handle_references(Some(json!({
            "textDocument": {"uri": utils_uri.clone()},
            "position": pos,
            "context": {"includeDeclaration": true}
        })))
        .unwrap();

    // Check result
    if let Some(refs) = result {
        let refs_array = refs.as_array().expect("Expected array of references");

        // Should find at least 3 references:
        // 1. Definition in Utils.pm
        // 2. Usage in script1.pl
        // 3. Two usages in script2.pl
        assert!(
            refs_array.len() >= 3,
            "Should find at least 3 references, found: {}",
            refs_array.len()
        );

        // Check that references are in different files
        let uris: Vec<String> =
            refs_array.iter().filter_map(|r| r["uri"].as_str()).map(|s| s.to_string()).collect();

        let unique_files: std::collections::HashSet<_> = uris.iter().collect();
        assert!(unique_files.len() >= 2, "References should be in at least 2 different files");
    } else {
        panic!("No references found");
    }
}

#[cfg(all(feature = "workspace", feature = "expose_lsp_test_api"))]
#[test]
#[serial_test::serial]
fn test_workspace_symbol_completion() {
    use lsp_types::Position;
    use perl_lsp::LspServer;
    use serde_json::json;
    use std::fs;
    use support::env_guard::EnvGuard;
    use tempfile::tempdir;

    // Enable workspace indexing
    // SAFETY: Test runs single-threaded with #[serial_test::serial]
    let _guard = unsafe { EnvGuard::set("PERL_LSP_WORKSPACE", "1") };

    // Create temporary directory structure
    let dir = tempdir().unwrap();
    let root = dir.path();

    // Create lib/Math/Advanced.pm
    let math_dir = root.join("lib").join("Math");
    std::fs::create_dir_all(&math_dir).unwrap();
    let advanced_file = math_dir.join("Advanced.pm");
    fs::write(&advanced_file,
        "package Math::Advanced;\n\nsub calculate_factorial {\n    my ($n) = @_;\n    return 1 if $n <= 1;\n    return $n * calculate_factorial($n - 1);\n}\n\nsub calculate_fibonacci {\n    my ($n) = @_;\n    return $n if $n <= 1;\n    return calculate_fibonacci($n-1) + calculate_fibonacci($n-2);\n}\n\n1;\n"
    ).unwrap();

    // Create main.pl with partial typing
    let main_file = root.join("main.pl");
    fs::write(
        &main_file,
        "use lib 'lib';\nuse Math::Advanced;\n\nmy $result = Math::Advanced::calc",
    )
    .unwrap();

    // Create URIs with proper encoding
    let advanced_uri = url::Url::from_file_path(&advanced_file).unwrap().to_string();
    let main_uri = url::Url::from_file_path(&main_file).unwrap().to_string();

    // Create LSP server
    use std::io::Cursor;
    use std::sync::{Arc, Mutex};
    let output =
        Arc::new(Mutex::new(Box::new(Cursor::new(Vec::new())) as Box<dyn std::io::Write + Send>));
    let srv = LspServer::with_output(output.clone());

    // Open both files to index them
    srv.test_handle_did_open(Some(json!({
        "textDocument": {
            "uri": advanced_uri.clone(),
            "text": fs::read_to_string(&advanced_file).unwrap(),
            "version": 1
        }
    })))
    .unwrap();

    srv.test_handle_did_open(Some(json!({
        "textDocument": {
            "uri": main_uri.clone(),
            "text": fs::read_to_string(&main_file).unwrap(),
            "version": 1
        }
    })))
    .unwrap();

    // Test: Get completions after "Math::Advanced::calc"
    // Position is at the end of line 3 (after 'calc')
    let pos = Position { line: 3, character: 33 };
    let result = srv
        .test_handle_completion(Some(json!({
            "textDocument": {"uri": main_uri.clone()},
            "position": pos
        })))
        .unwrap();

    // Check result
    if let Some(completions) = result {
        let items = completions["items"].as_array().expect("Expected items array");

        // Should find calculate_factorial and calculate_fibonacci
        let labels: Vec<String> =
            items.iter().filter_map(|item| item["label"].as_str()).map(|s| s.to_string()).collect();

        assert!(
            labels.iter().any(|l| l.contains("calculate_factorial")),
            "Should suggest calculate_factorial, got: {:?}",
            labels
        );
        assert!(
            labels.iter().any(|l| l.contains("calculate_fibonacci")),
            "Should suggest calculate_fibonacci, got: {:?}",
            labels
        );
    } else {
        panic!("No completions found");
    }
}
