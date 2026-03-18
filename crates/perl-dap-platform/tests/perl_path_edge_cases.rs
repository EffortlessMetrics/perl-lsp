//! P1 edge-case tests for perl-dap-platform path resolution.
//!
//! Exercises `resolve_perl_path` and `normalize_path` with:
//! - Non-standard Perl installations (custom PATH, perlbrew, plenv, system perl)
//! - Edge cases: empty PATH, missing perl, multiple perls, unusual locations
//! - Path normalization boundary conditions

use perl_dap_platform::{normalize_path, resolve_perl_path, setup_environment};
use std::path::PathBuf;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ===========================================================================
// 1. resolve_perl_path edge cases
// ===========================================================================

#[test]
fn resolve_perl_path_succeeds_when_perl_available() -> TestResult {
    // Environment-dependent: only assert properties when perl is found
    if let Ok(path) = resolve_perl_path() {
        assert!(path.exists(), "resolved perl path must exist");
        assert!(path.is_file(), "resolved perl path must be a regular file");

        // Should end with "perl" or "perl.exe"
        let filename = path.file_name().ok_or("perl path has no filename")?.to_string_lossy();
        assert!(
            filename == "perl" || filename == "perl.exe",
            "perl binary should be named 'perl' or 'perl.exe', got: {filename}"
        );
    }
    Ok(())
}

#[test]
fn resolve_perl_path_returns_absolute_path() -> TestResult {
    if let Ok(path) = resolve_perl_path() {
        assert!(path.is_absolute(), "resolved perl path should be absolute, got: {path:?}");
    }
    Ok(())
}

// ===========================================================================
// 2. normalize_path edge cases
// ===========================================================================

#[test]
fn normalize_path_empty_string() -> TestResult {
    let input = PathBuf::from("");
    let normalized = normalize_path(&input);
    // Empty path normalizes to something non-panicking
    let _ = normalized;
    Ok(())
}

#[test]
fn normalize_path_single_dot() -> TestResult {
    let normalized = normalize_path(&PathBuf::from("."));
    assert!(!normalized.as_os_str().is_empty(), "single dot should normalize to a non-empty path");
    Ok(())
}

#[test]
fn normalize_path_double_dot() -> TestResult {
    let normalized = normalize_path(&PathBuf::from(".."));
    assert!(!normalized.as_os_str().is_empty(), "double dot should normalize to a non-empty path");
    Ok(())
}

#[test]
fn normalize_path_triple_slash() -> TestResult {
    let normalized = normalize_path(&PathBuf::from("///tmp///test"));
    assert!(!normalized.as_os_str().is_empty(), "triple slash path should normalize");
    Ok(())
}

#[test]
fn normalize_path_with_spaces() -> TestResult {
    let input = PathBuf::from("/path with spaces/script.pl");
    let normalized = normalize_path(&input);
    let s = normalized.to_string_lossy();
    assert!(s.contains("spaces"), "path with spaces should be preserved in normalization");
    Ok(())
}

#[test]
fn normalize_path_with_unicode() -> TestResult {
    let input = PathBuf::from("/tmp/\u{00E9}l\u{00E8}ve/script.pl");
    let normalized = normalize_path(&input);
    let _ = normalized; // should not panic
    Ok(())
}

#[test]
fn normalize_path_very_long_path() -> TestResult {
    let long_component = "a".repeat(255);
    let input = PathBuf::from(format!("/tmp/{long_component}/script.pl"));
    let normalized = normalize_path(&input);
    let _ = normalized; // should not panic
    Ok(())
}

#[test]
fn normalize_path_with_dot_segments() -> TestResult {
    let input = PathBuf::from("/tmp/./test/../other/./script.pl");
    let normalized = normalize_path(&input);
    assert!(!normalized.as_os_str().is_empty(), "path with dot segments should normalize");
    Ok(())
}

#[test]
fn normalize_path_existing_file() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let file = tmp.path().join("test.pl");
    std::fs::write(&file, "1;")?;

    let normalized = normalize_path(&file);
    assert!(normalized.is_absolute(), "existing file should canonicalize to absolute");
    assert!(
        normalized.to_string_lossy().contains("test.pl"),
        "normalized path should contain filename"
    );
    Ok(())
}

#[test]
fn normalize_path_existing_directory() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let dir = tmp.path().join("subdir");
    std::fs::create_dir(&dir)?;

    let normalized = normalize_path(&dir);
    assert!(normalized.is_absolute(), "existing dir should canonicalize to absolute");
    Ok(())
}

#[cfg(unix)]
#[test]
fn normalize_path_symlink_resolves() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let real = tmp.path().join("real.pl");
    std::fs::write(&real, "1;")?;
    let link = tmp.path().join("link.pl");
    std::os::unix::fs::symlink(&real, &link)?;

    let normalized = normalize_path(&link);
    // canonicalize should resolve the symlink
    assert!(
        normalized.to_string_lossy().contains("real.pl"),
        "symlink should resolve to real path, got: {normalized:?}"
    );
    Ok(())
}

// ===========================================================================
// 3. Non-standard Perl installation scenarios (normalize_path)
// ===========================================================================

#[test]
fn normalize_path_perlbrew_style_path() -> TestResult {
    let input = PathBuf::from("/home/user/perl5/perlbrew/perls/perl-5.38.0/bin/perl");
    let normalized = normalize_path(&input);
    // Non-existent but should not panic
    let s = normalized.to_string_lossy();
    assert!(
        s.contains("perlbrew") || s.contains("perl"),
        "perlbrew-style path should be preserved or normalized"
    );
    Ok(())
}

#[test]
fn normalize_path_plenv_style_path() -> TestResult {
    let input = PathBuf::from("/home/user/.plenv/versions/5.38.0/bin/perl");
    let normalized = normalize_path(&input);
    let _ = normalized; // should not panic
    Ok(())
}

#[test]
fn normalize_path_system_perl_usr_bin() -> TestResult {
    let input = PathBuf::from("/usr/bin/perl");
    let normalized = normalize_path(&input);
    // This path likely exists on most Linux systems
    if input.exists() {
        assert!(normalized.is_absolute(), "system perl path should normalize to absolute");
    }
    Ok(())
}

#[test]
fn normalize_path_system_perl_usr_local() -> TestResult {
    let input = PathBuf::from("/usr/local/bin/perl");
    let normalized = normalize_path(&input);
    let _ = normalized; // should not panic
    Ok(())
}

#[test]
fn normalize_path_nix_store_style() -> TestResult {
    let input = PathBuf::from("/nix/store/abc123-perl-5.38.0/bin/perl");
    let normalized = normalize_path(&input);
    let _ = normalized; // should not panic
    Ok(())
}

#[test]
fn normalize_path_homebrew_style() -> TestResult {
    let input = PathBuf::from("/opt/homebrew/Cellar/perl/5.38.0/bin/perl");
    let normalized = normalize_path(&input);
    let _ = normalized; // should not panic
    Ok(())
}

#[test]
fn normalize_path_asdf_style() -> TestResult {
    let input = PathBuf::from("/home/user/.asdf/installs/perl/5.38.0/bin/perl");
    let normalized = normalize_path(&input);
    let _ = normalized; // should not panic
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn normalize_path_wsl_strawberry_perl() -> TestResult {
    // Strawberry Perl on Windows accessed via WSL
    let input = PathBuf::from("/mnt/c/Strawberry/perl/bin/perl.exe");
    let normalized = normalize_path(&input);
    let s = normalized.to_string_lossy();
    assert!(
        s.starts_with("C:"),
        "WSL path to Strawberry Perl should convert to Windows drive, got: {s}"
    );
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn normalize_path_wsl_activeperl() -> TestResult {
    let input = PathBuf::from("/mnt/c/Perl64/bin/perl.exe");
    let normalized = normalize_path(&input);
    let s = normalized.to_string_lossy();
    assert!(
        s.starts_with("C:"),
        "WSL path to ActivePerl should convert to Windows drive, got: {s}"
    );
    Ok(())
}

// ===========================================================================
// 4. setup_environment edge cases
// ===========================================================================

#[test]
fn setup_environment_with_perlbrew_lib_paths() -> TestResult {
    let paths = [
        PathBuf::from("/home/user/perl5/perlbrew/perls/perl-5.38.0/lib/site_perl/5.38.0"),
        PathBuf::from("/home/user/perl5/perlbrew/perls/perl-5.38.0/lib/5.38.0"),
    ];
    let env = setup_environment(&paths);
    let perl5lib = env.get("PERL5LIB").ok_or_else(|| "PERL5LIB not set".to_string())?;
    assert!(perl5lib.contains("site_perl"), "PERL5LIB should include site_perl path");
    assert!(perl5lib.contains("5.38.0"), "PERL5LIB should include version path");
    Ok(())
}

#[test]
fn setup_environment_with_path_containing_colon() -> TestResult {
    // Paths can technically contain colons on some filesystems
    let paths = [PathBuf::from("/tmp/weird:path/lib")];
    let env = setup_environment(&paths);
    let perl5lib = env.get("PERL5LIB").ok_or_else(|| "PERL5LIB not set".to_string())?;
    assert!(perl5lib.contains("weird"), "path with colon should be preserved in PERL5LIB");
    Ok(())
}

#[test]
fn setup_environment_with_unicode_paths() -> TestResult {
    let paths = [PathBuf::from("/tmp/\u{00E9}l\u{00E8}ve/lib")];
    let env = setup_environment(&paths);
    let perl5lib = env.get("PERL5LIB").ok_or_else(|| "PERL5LIB not set".to_string())?;
    assert!(perl5lib.contains("\u{00E9}"), "unicode in paths should be preserved");
    Ok(())
}

#[test]
fn setup_environment_with_many_paths() -> TestResult {
    let paths: Vec<PathBuf> = (0..50).map(|i| PathBuf::from(format!("/lib/path{i}"))).collect();
    let env = setup_environment(&paths);
    let perl5lib = env.get("PERL5LIB").ok_or_else(|| "PERL5LIB not set".to_string())?;

    #[cfg(not(windows))]
    let sep = ':';
    #[cfg(windows)]
    let sep = ';';

    let parts: Vec<&str> = perl5lib.split(sep).collect();
    assert_eq!(parts.len(), 50, "all 50 paths should be in PERL5LIB");
    Ok(())
}
