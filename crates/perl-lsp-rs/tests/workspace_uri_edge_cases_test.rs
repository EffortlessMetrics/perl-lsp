mod support;

#[cfg(all(feature = "workspace", feature = "expose_lsp_test_api"))]
#[cfg(test)]
mod tests {
    use crate::support::env_guard::EnvGuard;
    use lsp_types::Position;
    use parking_lot::Mutex;
    use perl_lsp::LspServer;
    use serde_json::json;
    use serial_test::serial;
    use std::sync::Arc;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    #[serial]
    fn test_cross_file_with_spaces_in_directory() -> TestResult {
        use std::fs;
        use tempfile::tempdir;

        // Create a workspace with spaces in directory names
        let temp = tempdir()?;
        let workspace_dir = temp.path().join("My Perl Project");
        fs::create_dir(&workspace_dir)?;

        let lib_dir = workspace_dir.join("Lib Modules");
        fs::create_dir(&lib_dir)?;

        // Create a module in a directory with spaces
        let module_file = lib_dir.join("Math Utils.pm");
        let module_content = r#"package Math::Utils;

sub calculate_🚀 {
    my ($x, $y) = @_;
    return $x + $y;
}

1;
"#;
        fs::write(&module_file, module_content)?;

        // Create main script that uses the module
        let main_file = workspace_dir.join("main script.pl");
        let main_content = r#"#!/usr/bin/perl
use lib 'Lib Modules';
use Math::Utils;

my $result = Math::Utils::calculate_🚀(5, 10);
print "Result: $result\n";
"#;
        fs::write(&main_file, main_content)?;

        // Set up LSP server with workspace indexing
        // SAFETY: Test runs single-threaded with #[serial_test::serial]
        let _guard = unsafe { EnvGuard::set("PERL_LSP_WORKSPACE", "1") };
        let output: Arc<Mutex<Box<dyn std::io::Write + Send>>> =
            Arc::new(Mutex::new(Box::new(Vec::new())));
        let srv = LspServer::with_output(output.clone());

        // Convert paths to URIs (with proper percent-encoding for spaces)
        let module_uri = url::Url::from_file_path(&module_file)
            .map_err(|_| "Failed to create URL from module file path")?
            .to_string();
        let main_uri = url::Url::from_file_path(&main_file)
            .map_err(|_| "Failed to create URL from main file path")?
            .to_string();

        // Open both files to index them
        srv.test_handle_did_open(Some(json!({
            "textDocument": {
                "uri": module_uri.clone(),
                "text": module_content,
                "languageId": "perl"
            }
        })))
        .map_err(|e| format!("Failed to open module file: {e:?}"))?;

        srv.test_handle_did_open(Some(json!({
            "textDocument": {
                "uri": main_uri.clone(),
                "text": main_content,
                "languageId": "perl"
            }
        })))
        .map_err(|e| format!("Failed to open main file: {e:?}"))?;

        // Test: Go to definition on "calculate_🚀" with emoji
        // Position is line 4, character 30 (inside 'calculate_🚀')
        let pos = Position { line: 4, character: 30 };
        let result = srv
            .test_handle_definition(Some(json!({
                "textDocument": {"uri": main_uri.clone()},
                "position": pos
            })))
            .map_err(|e| format!("Failed to handle definition: {e:?}"))?;

        // Should find the definition in Math Utils.pm
        let result = result.ok_or("Expected definition result")?;
        let locations = result.as_array().ok_or("Expected array of locations")?;
        assert!(!locations.is_empty(), "Should find definition");

        let location = &locations[0];
        let def_uri = location["uri"].as_str().ok_or("Expected uri as string")?;

        // Verify it points to the module file (with proper encoding)
        assert!(
            def_uri.contains("Math%20Utils.pm"),
            "Definition should be in 'Math Utils.pm' with encoded space"
        );
        assert!(
            def_uri.contains("Lib%20Modules"),
            "Path should contain 'Lib Modules' with encoded space"
        );

        Ok(())
    }

    #[test]
    #[serial]
    fn test_references_with_emoji_on_line() -> TestResult {
        use std::fs;
        use tempfile::tempdir;

        // Create workspace
        let temp = tempdir()?;

        // Create a module with emoji identifiers
        let emoji_file = temp.path().join("emoji.pm");
        let emoji_content = r#"package Emoji;

my $♥ = 'love';  # Heart emoji variable

sub process_♥ {  # Line 4 - emoji in function name
    my $data = shift;
    return "♥ $data ♥";
}

sub use_emoji {
    my $result = process_♥("test");  # Line 10 - reference to emoji function
    print "Got: $result\n";
}

1;
"#;
        fs::write(&emoji_file, emoji_content)?;

        // Set up LSP server
        // SAFETY: Test runs single-threaded with #[serial_test::serial]
        let _guard = unsafe { EnvGuard::set("PERL_LSP_WORKSPACE", "1") };
        let output: Arc<Mutex<Box<dyn std::io::Write + Send>>> =
            Arc::new(Mutex::new(Box::new(Vec::new())));
        let srv = LspServer::with_output(output.clone());

        let emoji_uri = url::Url::from_file_path(&emoji_file)
            .map_err(|_| "Failed to create URL from emoji file path")?
            .to_string();

        // Open file to index it
        srv.test_handle_did_open(Some(json!({
            "textDocument": {
                "uri": emoji_uri.clone(),
                "text": emoji_content,
                "languageId": "perl"
            }
        })))
        .map_err(|e| format!("Failed to open emoji file: {e:?}"))?;

        // Test: Find references to "process_♥"
        // Position is line 4, character 5 (inside 'process_♥' definition)
        let pos = Position { line: 4, character: 5 };
        let result = srv
            .test_handle_references(Some(json!({
                "textDocument": {"uri": emoji_uri.clone()},
                "position": pos,
                "context": {"includeDeclaration": true}
            })))
            .map_err(|e| format!("Failed to handle references: {e:?}"))?;

        // Should find both definition and usage
        let result = result.ok_or("Expected references result")?;
        let references = result.as_array().ok_or("Expected array of references")?;
        assert_eq!(references.len(), 2, "Should find definition and usage");

        // Verify the line numbers
        let lines: Vec<u32> = references
            .iter()
            .map(|r| {
                r["range"]["start"]["line"].as_u64().ok_or("Expected line number").map(|v| v as u32)
            })
            .collect::<Result<Vec<u32>, _>>()?;

        assert!(lines.contains(&4), "Should find definition on line 4");
        assert!(lines.contains(&10), "Should find usage on line 10");

        Ok(())
    }

    #[test]
    #[serial]
    fn test_completion_with_utf16_columns() -> TestResult {
        use std::fs;
        use tempfile::tempdir;

        let temp = tempdir()?;

        // Create a module with mixed-width characters
        let unicode_file = temp.path().join("unicode.pm");
        let unicode_content = r#"package Unicode;

sub 日本語_function { }  # Japanese characters
sub café_function { }    # Accented characters
sub 𝕦𝕟𝕚𝕔𝕠𝕕𝕖_function { }  # Mathematical unicode
sub emoji_🎉_function { }  # Emoji in name

1;
"#;
        fs::write(&unicode_file, unicode_content)?;

        // Create main file
        let main_file = temp.path().join("main.pl");
        let main_content = r#"use Unicode;

# Type after Unicode:: to get completions
Unicode::
"#;
        fs::write(&main_file, main_content)?;

        // Set up LSP server
        // SAFETY: Test runs single-threaded with #[serial_test::serial]
        let _guard = unsafe { EnvGuard::set("PERL_LSP_WORKSPACE", "1") };
        let output: Arc<Mutex<Box<dyn std::io::Write + Send>>> =
            Arc::new(Mutex::new(Box::new(Vec::new())));
        let srv = LspServer::with_output(output.clone());

        let unicode_uri = url::Url::from_file_path(&unicode_file)
            .map_err(|_| "Failed to create URL from unicode file path")?
            .to_string();
        let main_uri = url::Url::from_file_path(&main_file)
            .map_err(|_| "Failed to create URL from main file path")?
            .to_string();

        // Open both files
        srv.test_handle_did_open(Some(json!({
            "textDocument": {
                "uri": unicode_uri.clone(),
                "text": unicode_content,
                "languageId": "perl"
            }
        })))
        .map_err(|e| format!("Failed to open unicode file: {e:?}"))?;

        srv.test_handle_did_open(Some(json!({
            "textDocument": {
                "uri": main_uri.clone(),
                "text": main_content,
                "languageId": "perl"
            }
        })))
        .map_err(|e| format!("Failed to open main file: {e:?}"))?;

        // Test: Get completions after "Unicode::"
        // Position is line 3, character 9 (after '::')
        let pos = Position { line: 3, character: 9 };
        let result = srv
            .test_handle_completion(Some(json!({
                "textDocument": {"uri": main_uri.clone()},
                "position": pos
            })))
            .map_err(|e| format!("Failed to handle completion: {e:?}"))?;

        // Should get all the unicode function completions
        let result = result.ok_or("Expected completion result")?;
        let items = result["items"].as_array().ok_or("Expected completion items")?;
        assert!(items.len() >= 4, "Should have at least 4 unicode functions");

        let labels: Vec<String> = items
            .iter()
            .map(|item| {
                item["label"].as_str().ok_or("Expected label as string").map(|s| s.to_string())
            })
            .collect::<Result<Vec<String>, _>>()?;

        assert!(labels.iter().any(|l| l.contains("日本語")), "Should have Japanese function");
        assert!(labels.iter().any(|l| l.contains("café")), "Should have accented function");
        assert!(
            labels.iter().any(|l| l.contains("𝕦𝕟𝕚𝕔𝕠𝕕𝕖")),
            "Should have mathematical unicode function"
        );
        assert!(labels.iter().any(|l| l.contains("🎉")), "Should have emoji function");

        Ok(())
    }
}
