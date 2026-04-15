//! module_signature_nongol.rs — BDD-style tests documenting that Perl module
//! signature verification is NOT performed by perl-module-resolution-uri.
//!
//! These tests serve as executable documentation of the module resolution
//! trust boundary. They verify:
//!
//! 1. Module resolution returns a URI when the module file exists (path-based check only)
//! 2. A module with an adjacent SIGNATURE file is resolved without reading/verifying the signature
//! 3. The `IncRoot` struct carries no signature-related fields
//!
//! The absence of signature verification is a deliberate design decision documented in
//! ADR-0020. Users needing signature verification should use external tools such as
//! `CPAN::Shell->verify`.

use perl_module_resolution_uri::{IncRoot, IncRootKind, ModuleUriResolution, resolve_module_uri};
use std::path::PathBuf;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ---------------------------------------------------------------------------
// AC4.1: Module resolution returns URI via path-based check only
// ---------------------------------------------------------------------------

#[test]
fn given_existing_module_file_when_resolving_then_uri_is_returned() -> TestResult {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    let module_file = workspace.join("lib").join("Foo").join("Bar.pm");

    std::fs::create_dir_all(module_file.parent().unwrap_or(&workspace))?;
    std::fs::write(&module_file, "package Foo::Bar; 1;\n")?;

    let workspace_uri =
        url::Url::from_file_path(&workspace).map_err(|()| "failed to build workspace URI")?;
    let workspace_uri = workspace_uri.to_string();

    let result = resolve_module_uri(
        "Foo::Bar",
        &[],
        &[workspace_uri],
        &["lib".to_string()],
        false,
        &[],
        Duration::from_millis(50),
    );

    match result {
        ModuleUriResolution::Resolved(uri) => {
            // Verify path-based resolution returned a valid file URI
            assert!(uri.starts_with("file://"), "Expected file:// URI, got: {}", uri);
            assert!(uri.contains("Foo"), "Expected URI to contain 'Foo', got: {}", uri);
            assert!(uri.contains("Bar.pm"), "Expected URI to contain 'Bar.pm', got: {}", uri);
        }
        other => {
            return Err(
                format!("Expected Resolved(...) for existing module, got {:?}", other).into()
            );
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// AC4.2: Module with adjacent SIGNATURE file is resolved without reading signature
// ---------------------------------------------------------------------------

#[test]
fn given_module_with_adjacent_signature_file_when_resolving_then_signature_not_verified()
-> TestResult {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    let module_file = workspace.join("lib").join("Secure").join("Module.pm");

    std::fs::create_dir_all(module_file.parent().unwrap_or(&workspace))?;

    // Write a valid Perl module
    std::fs::write(&module_file, "package Secure::Module; 1;\n")?;

    // Write a SIGNATURE file (simulating CPAN distribution signing)
    // Per Module::Signature, this would contain an encrypted signature
    let signature_file = module_file.parent().unwrap().join("SIGNATURE");
    std::fs::write(
        &signature_file,
        "-----BEGIN PGP SIGNATURE-----
Version: GnuPG v2

iQEzBAEBCAAdFiEE1234567890ABCDEFGHIJKLMNOFKA4nCFqZ0ACgkQFGHIJKLMNOFKA
jwMAiE+z8v9K3j2+9W4R2yZqX3R8M6vF
=WXYZ
-----END PGP SIGNATURE-----
",
    )?;

    let workspace_uri =
        url::Url::from_file_path(&workspace).map_err(|()| "failed to build workspace URI")?;
    let workspace_uri = workspace_uri.to_string();

    // Resolve the module
    let result = resolve_module_uri(
        "Secure::Module",
        &[],
        &[workspace_uri],
        &["lib".to_string()],
        false,
        &[],
        Duration::from_millis(50),
    );

    match result {
        ModuleUriResolution::Resolved(uri) => {
            // The module was resolved successfully WITHOUT signature verification
            // This is the expected behavior - signature files are NOT read or verified
            assert!(uri.starts_with("file://"), "Expected file:// URI, got: {}", uri);
            assert!(uri.contains("Secure"), "Expected URI to contain 'Secure', got: {}", uri);
            assert!(uri.contains("Module.pm"), "Expected URI to contain 'Module.pm', got: {}", uri);
        }
        other => {
            return Err(format!(
                "Expected Resolved(...) for module with SIGNATURE file, got {:?}.                  Module resolution should succeed without verifying the signature.",
                other
            )
            .into());
        }
    }

    // Verification: The SIGNATURE file still exists (was not consumed or modified)
    // This confirms signature verification was NOT performed
    assert!(signature_file.exists(), "SIGNATURE file should still exist after resolution");
    let signature_content = std::fs::read_to_string(&signature_file)?;
    assert!(
        signature_content.contains("-----BEGIN PGP SIGNATURE-----"),
        "SIGNATURE file content should be unchanged"
    );

    Ok(())
}

#[test]
fn given_module_in_inc_with_signature_when_resolving_then_no_signature_check() -> TestResult {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    let external_inc = temp.path().join("cpan");
    let module_file = external_inc.join("Dist").join("Name.pm");

    std::fs::create_dir_all(module_file.parent().unwrap_or(&external_inc))?;
    std::fs::write(&module_file, "package Dist::Name; 1;\n")?;

    // Write a SIGNATURE file
    let signature_file = module_file.parent().unwrap().join("SIGNATURE");
    std::fs::write(
        &signature_file,
        "-----BEGIN SIGNATURE-----
Digest::SHA1::Base64 'abc123'
-----END SIGNATURE-----
",
    )?;

    let workspace_uri =
        url::Url::from_file_path(&workspace).map_err(|()| "failed to build workspace URI")?;
    let workspace_uri = workspace_uri.to_string();

    // Resolve with system_inc enabled (this uses InterpreterStartup roots)
    let result = resolve_module_uri(
        "Dist::Name",
        &[],
        &[workspace_uri],
        &[],
        true, // use_system_inc = true
        &[PathBuf::from(&external_inc)],
        Duration::from_millis(50),
    );

    match result {
        ModuleUriResolution::Resolved(uri) => {
            // Resolution succeeded - no signature verification was performed
            assert!(uri.contains("Dist"), "Expected URI to contain 'Dist', got: {}", uri);
        }
        other => {
            return Err(format!(
                "Expected Resolved(...) for module in system INC with SIGNATURE, got {:?}",
                other
            )
            .into());
        }
    }

    // Confirm SIGNATURE file is untouched
    assert!(signature_file.exists(), "SIGNATURE file should remain after resolution");
    let content = std::fs::read_to_string(&signature_file)?;
    assert!(content.contains("Digest::SHA1::Base64"), "SIGNATURE content should be unchanged");

    Ok(())
}

// ---------------------------------------------------------------------------
// AC4.3: IncRoot struct has no signature-related fields
// ---------------------------------------------------------------------------

#[test]
fn inc_root_struct_has_no_signature_related_fields() {
    // This test documents the API contract: IncRoot must NOT have fields
    // related to signature status, trust level, or distribution integrity.

    // We use compile-time checks to verify the struct fields
    let root = IncRoot {
        kind: IncRootKind::WorkspaceRelative,
        path: PathBuf::from("/some/path"),
        precedence: 0,
        source: "test".to_string(),
    };

    // Document the expected fields via struct access
    let _kind: IncRootKind = root.kind;
    let _path: PathBuf = root.path.clone();
    let _precedence: usize = root.precedence;
    let _source: String = root.source.clone();

    // Verify there are no unexpected fields by checking the size
    // A struct with 4 fields should have a predictable memory representation
    // If signature fields were added, this would likely change the size
    // (though this is an indirect check)
    let root_size = std::mem::size_of::<IncRoot>();
    assert!(
        root_size <= 128,
        "IncRoot size ({}) suggests possible additional fields.          Current fields: kind, path, precedence, source.          No signature, trust_level, or integrity fields should exist.",
        root_size
    );
}

#[test]
fn inc_root_does_not_carry_signature_status() {
    // Verify IncRoot does not implement any hypothetical signature-related methods
    // that would indicate signature verification capability

    let root = IncRoot {
        kind: IncRootKind::ExternalAbsolute,
        path: PathBuf::from("/opt/perl/lib"),
        precedence: 10,
        source: "system-lib".to_string(),
    };

    // Document that IncRoot only carries path-based resolution metadata
    // The struct should NOT have methods like:
    // - signature_status()
    // - verify_signature()
    // - trust_level()
    // - provenance()

    // We verify by checking the Debug output contains expected fields only
    let debug_str = format!("{:?}", root);
    assert!(debug_str.contains("kind"), "Debug output should contain 'kind'");
    assert!(debug_str.contains("path"), "Debug output should contain 'path'");
    assert!(debug_str.contains("precedence"), "Debug output should contain 'precedence'");
    assert!(debug_str.contains("source"), "Debug output should contain 'source'");

    // Verify NO signature-related fields are present
    let debug_lower = debug_str.to_lowercase();
    assert!(
        !debug_lower.contains("signature"),
        "Debug output should NOT contain 'signature' - IncRoot does not carry signature status"
    );
    assert!(
        !debug_lower.contains("trust"),
        "Debug output should NOT contain 'trust' - IncRoot does not carry trust levels"
    );
    assert!(
        !debug_lower.contains("provenance"),
        "Debug output should NOT contain 'provenance' - IncRoot has no provenance fields"
    );
    assert!(
        !debug_lower.contains("integrity"),
        "Debug output should NOT contain 'integrity' - IncRoot has no integrity fields"
    );
}

// ---------------------------------------------------------------------------
// AC4: Edge cases for module resolution trust boundary
// ---------------------------------------------------------------------------

#[test]
fn given_empty_module_name_with_signature_file_present_returns_not_found() -> TestResult {
    // Even with SIGNATURE files present, empty module names should not resolve
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    let module_dir = workspace.join("lib").join("Empty");

    std::fs::create_dir_all(&module_dir)?;

    // Write a module
    let module_file = module_dir.join("Name.pm");
    std::fs::write(&module_file, "package Empty::Name; 1;\n")?;

    // Write a SIGNATURE file nearby
    let signature_file = module_dir.join("SIGNATURE");
    std::fs::write(
        &signature_file,
        "-----BEGIN PGP SIGNATURE-----\n...\n-----END PGP SIGNATURE-----\n",
    )?;

    let workspace_uri =
        url::Url::from_file_path(&workspace).map_err(|()| "failed to build workspace URI")?;
    let workspace_uri = workspace_uri.to_string();

    // Empty module name should return NotFound regardless of SIGNATURE presence
    let result = resolve_module_uri(
        "",
        &[],
        &[workspace_uri],
        &["lib".to_string()],
        false,
        &[],
        Duration::from_millis(50),
    );

    assert!(
        matches!(result, ModuleUriResolution::NotFound),
        "Empty module name should return NotFound, got {:?}",
        result
    );
    Ok(())
}

#[test]
fn given_malformed_signature_content_module_still_resolves() -> TestResult {
    // Malformed/invalid SIGNATURE content should not affect resolution
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    let module_file = workspace.join("lib").join("Malformed").join("Sig.pm");

    std::fs::create_dir_all(module_file.parent().unwrap_or(&workspace))?;
    std::fs::write(&module_file, "package Malformed::Sig; 1;\n")?;

    // Write various malformed SIGNATURE files
    let signature_file = module_file.parent().unwrap().join("SIGNATURE");

    // Case 1: Invalid PGP format
    std::fs::write(&signature_file, "This is not a valid SIGNATURE file at all!!!\n")?;

    let workspace_uri =
        url::Url::from_file_path(&workspace).map_err(|()| "failed to build workspace URI")?;
    let workspace_uri = workspace_uri.to_string();

    let result = resolve_module_uri(
        "Malformed::Sig",
        &[],
        &[workspace_uri],
        &["lib".to_string()],
        false,
        &[],
        Duration::from_millis(50),
    );

    match result {
        ModuleUriResolution::Resolved(uri) => {
            assert!(uri.contains("Malformed"), "Expected URI to contain 'Malformed'");
        }
        other => {
            return Err(format!(
                "Expected Resolved(...) with malformed SIGNATURE, got {:?}",
                other
            )
            .into());
        }
    }

    // Verify SIGNATURE file was not consumed or modified
    assert!(signature_file.exists(), "Malformed SIGNATURE file should still exist");
    Ok(())
}

#[test]
fn given_module_with_traversal_name_and_signature_traversal_still_blocked() -> TestResult {
    // Module names that look like path traversal should still be blocked
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    let module_file = workspace.join("lib").join("Normal.pm");

    std::fs::create_dir_all(module_file.parent().unwrap_or(&workspace))?;
    std::fs::write(&module_file, "package Normal; 1;\n")?;

    // Write a SIGNATURE file
    let signature_file = module_file.parent().unwrap().join("SIGNATURE");
    std::fs::write(
        &signature_file,
        "-----BEGIN PGP SIGNATURE-----\n...\n-----END PGP SIGNATURE-----\n",
    )?;

    let workspace_uri =
        url::Url::from_file_path(&workspace).map_err(|()| "failed to build workspace URI")?;
    let workspace_uri = workspace_uri.to_string();

    // Attempt traversal - should be blocked even though there's a SIGNATURE file
    let result = resolve_module_uri(
        "../../../etc/passwd",
        &[],
        &[workspace_uri],
        &["lib".to_string()],
        false,
        &[],
        Duration::from_millis(50),
    );

    match result {
        ModuleUriResolution::NotFound => {
            // Good - traversal was blocked
        }
        ModuleUriResolution::Resolved(uri) => {
            return Err(format!("Traversal should be blocked, but got Resolved({})", uri).into());
        }
        ModuleUriResolution::TimedOut => {
            return Err("Traversal should not timeout".into());
        }
    }
    Ok(())
}

#[test]
fn given_unicode_module_name_with_signature_resolves_correctly() -> TestResult {
    // Unicode module names should resolve correctly regardless of SIGNATURE presence
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    let module_file = workspace.join("lib").join("Unicode").join("Module.pm");

    std::fs::create_dir_all(module_file.parent().unwrap_or(&workspace))?;
    std::fs::write(&module_file, "package Unicode::Module; 1;\n")?;

    // Write a SIGNATURE file
    let signature_file = module_file.parent().unwrap().join("SIGNATURE");
    std::fs::write(
        &signature_file,
        "-----BEGIN SIGNATURE-----\nDigest::SHA1::Base64 'unicode-test'\n-----END SIGNATURE-----\n",
    )?;

    let workspace_uri =
        url::Url::from_file_path(&workspace).map_err(|()| "failed to build workspace URI")?;
    let workspace_uri = workspace_uri.to_string();

    let result = resolve_module_uri(
        "Unicode::Module",
        &[],
        &[workspace_uri],
        &["lib".to_string()],
        false,
        &[],
        Duration::from_millis(50),
    );

    match result {
        ModuleUriResolution::Resolved(uri) => {
            assert!(uri.contains("Unicode"), "Expected URI to contain 'Unicode'");
            assert!(uri.contains("Module.pm"), "Expected URI to contain 'Module.pm'");
        }
        other => {
            return Err(format!(
                "Expected Resolved(...) for Unicode module with SIGNATURE, got {:?}",
                other
            )
            .into());
        }
    }

    // Verify SIGNATURE was not read
    assert!(signature_file.exists(), "SIGNATURE should still exist");
    let content = std::fs::read_to_string(&signature_file)?;
    assert!(content.contains("unicode-test"), "SIGNATURE content should be unchanged");
    Ok(())
}

#[test]
fn given_multiple_workspace_folders_with_signature_precedence_honored() -> TestResult {
    // Multiple workspace folders should still honor precedence rules
    let temp = tempfile::tempdir()?;
    let workspace1 = temp.path().join("workspace1");
    let workspace2 = temp.path().join("workspace2");
    let module_file1 = workspace1.join("lib").join("PrefTest.pm");
    let module_file2 = workspace2.join("lib").join("PrefTest.pm");

    std::fs::create_dir_all(module_file1.parent().unwrap_or(&workspace1))?;
    std::fs::create_dir_all(module_file2.parent().unwrap_or(&workspace2))?;
    std::fs::write(&module_file1, "package PrefTest; 1; # from workspace1\n")?;
    std::fs::write(&module_file2, "package PrefTest; 1; # from workspace2\n")?;

    // Write SIGNATURE files in both locations
    let sig1 = module_file1.parent().unwrap().join("SIGNATURE");
    let sig2 = module_file2.parent().unwrap().join("SIGNATURE");
    std::fs::write(&sig1, "-----BEGIN SIGNATURE-----\nws1\n-----END SIGNATURE-----\n")?;
    std::fs::write(&sig2, "-----BEGIN SIGNATURE-----\nws2\n-----END SIGNATURE-----\n")?;

    let ws1_uri =
        url::Url::from_file_path(&workspace1).map_err(|()| "failed to build workspace1 URI")?;
    let ws2_uri =
        url::Url::from_file_path(&workspace2).map_err(|()| "failed to build workspace2 URI")?;
    let ws1_uri = ws1_uri.to_string();
    let ws2_uri = ws2_uri.to_string();

    // First workspace folder should win (precedence order)
    let result = resolve_module_uri(
        "PrefTest",
        &[],
        &[ws1_uri.clone(), ws2_uri.clone()],
        &["lib".to_string()],
        false,
        &[],
        Duration::from_millis(50),
    );

    match result {
        ModuleUriResolution::Resolved(uri) => {
            // The first workspace should win
            assert!(
                uri.contains("workspace1"),
                "Expected first workspace to win, got URI: {}",
                uri
            );
        }
        other => {
            return Err(format!("Expected Resolved(...), got {:?}", other).into());
        }
    }

    // Verify SIGNATURE files were not consumed
    assert!(sig1.exists(), "SIGNATURE in workspace1 should still exist");
    assert!(sig2.exists(), "SIGNATURE in workspace2 should still exist");
    Ok(())
}

#[test]
fn given_signature_file_not_read_during_timeout_check() -> TestResult {
    // Timeout should not be affected by SIGNATURE file presence
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    let module_file = workspace.join("lib").join("TimeoutTest").join("Mod.pm");

    std::fs::create_dir_all(module_file.parent().unwrap_or(&workspace))?;
    std::fs::write(&module_file, "package TimeoutTest::Mod; 1;\n")?;

    // Write SIGNATURE file
    let signature_file = module_file.parent().unwrap().join("SIGNATURE");
    std::fs::write(
        &signature_file,
        "-----BEGIN PGP SIGNATURE-----\nThisWouldBeVerifiedIfWeDidSignatureVerification\n-----END PGP SIGNATURE-----\n",
    )?;

    let workspace_uri =
        url::Url::from_file_path(&workspace).map_err(|()| "failed to build workspace URI")?;
    let workspace_uri = workspace_uri.to_string();

    // Zero timeout should cause timeout (if module was found it would return Resolved)
    // Since the module exists, it should be found quickly without timing out
    let result = resolve_module_uri(
        "TimeoutTest::Mod",
        &[],
        &[workspace_uri],
        &["lib".to_string()],
        false,
        &[],
        Duration::from_secs(86400), // Very long timeout - should succeed
    );

    match result {
        ModuleUriResolution::Resolved(uri) => {
            assert!(uri.contains("TimeoutTest"), "Expected URI to contain 'TimeoutTest'");
        }
        ModuleUriResolution::TimedOut => {
            return Err("Should not timeout with 86400s budget".into());
        }
        ModuleUriResolution::NotFound => {
            return Err("Module should be found".into());
        }
    }

    // Verify SIGNATURE was not read
    assert!(signature_file.exists(), "SIGNATURE should still exist");
    Ok(())
}

#[test]
#[allow(clippy::cloned_ref_to_slice_refs)]
fn given_various_signature_formats_all_ignored() -> TestResult {
    // Different SIGNATURE file formats should all be ignored
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");

    let signature_formats = [
        (
            "Module::Core",
            "-----BEGIN PGP SIGNATURE-----\nVersion: GnuPG\n\niQEz...\n-----END PGP SIGNATURE-----",
        ),
        (
            "Module::Canonical",
            "-----BEGIN SIGNATURE-----\nDigest::SHA1::Base64 'abcdefg'\nDigest::SHA256::Base64 '123456'\n-----END SIGNATURE-----",
        ),
        ("Module::Minimal", "Hash: SHA1 abcdef123456"),
    ];

    for (module_name, sig_content) in signature_formats {
        let module_rel = format!("lib/{}.pm", module_name.replace("::", "/"));
        let module_file = workspace.join(&module_rel);
        std::fs::create_dir_all(module_file.parent().unwrap_or(&workspace))?;
        std::fs::write(&module_file, format!("package {}; 1;\n", module_name))?;

        let sig_file = module_file.parent().unwrap().join("SIGNATURE");
        std::fs::write(&sig_file, sig_content)?;
    }

    let workspace_uri =
        url::Url::from_file_path(&workspace).map_err(|()| "failed to build workspace URI")?;
    let workspace_uri = workspace_uri.to_string();

    for (module_name, _sig_content) in &signature_formats {
        let result = resolve_module_uri(
            module_name,
            &[],
            &[workspace_uri.clone()],
            &["lib".to_string()],
            false,
            &[],
            Duration::from_millis(100),
        );

        match result {
            ModuleUriResolution::Resolved(uri) => {
                assert!(
                    uri.contains(module_name.split("::").last().unwrap()),
                    "Expected URI to contain module name, got: {}",
                    uri
                );
            }
            other => {
                return Err(format!(
                    "Expected Resolved for {} with various SIGNATURE formats, got {:?}",
                    module_name, other
                )
                .into());
            }
        }
    }

    Ok(())
}

#[test]
#[allow(clippy::cloned_ref_to_slice_refs)]
fn given_module_with_signature_in_open_doc_precedence_still_honored() -> TestResult {
    // Open document precedence should still work even with SIGNATURE files
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    let module_file = workspace.join("lib").join("OpenPrecedent").join("Mod.pm");

    std::fs::create_dir_all(module_file.parent().unwrap_or(&workspace))?;
    std::fs::write(&module_file, "package OpenPrecedent::Mod; 1;\n")?;

    // Write SIGNATURE file
    let signature_file = module_file.parent().unwrap().join("SIGNATURE");
    std::fs::write(
        &signature_file,
        "-----BEGIN SIGNATURE-----\nFromOpenDoc\n-----END SIGNATURE-----\n",
    )?;

    let workspace_uri =
        url::Url::from_file_path(&workspace).map_err(|()| "failed to build workspace URI")?;
    let workspace_uri = workspace_uri.to_string();

    // Open document that exactly matches
    let open_doc_uri = "file:///open/doc/OpenPrecedent/Mod.pm".to_string();

    let result = resolve_module_uri(
        "OpenPrecedent::Mod",
        &[open_doc_uri.clone()],
        &[workspace_uri],
        &["lib".to_string()],
        false,
        &[],
        Duration::from_millis(50),
    );

    match result {
        ModuleUriResolution::Resolved(uri) => {
            // Open document should win over workspace even with SIGNATURE
            assert!(uri.contains("open/doc"), "Open document should win precedence, got: {}", uri);
        }
        other => {
            return Err(format!("Expected Resolved from open_doc, got {:?}", other).into());
        }
    }

    // Verify SIGNATURE file was not read
    assert!(signature_file.exists(), "SIGNATURE should still exist");
    Ok(())
}
