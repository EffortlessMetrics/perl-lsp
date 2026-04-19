use perl_dap::config::{LaunchConfiguration, create_launch_json_snippet};
use std::collections::HashMap;
use std::path::PathBuf;

#[test]
fn test_launch_configuration_serialization() -> Result<(), Box<dyn std::error::Error>> {
    let config = LaunchConfiguration {
        program: PathBuf::from("/path/to/script.pl"),
        args: vec!["--verbose".to_string()],
        cwd: Some(PathBuf::from("/workspace")),
        env: HashMap::new(),
        perl_path: Some(PathBuf::from("/usr/bin/perl")),
        include_paths: vec![PathBuf::from("/workspace/lib")],
    };

    let json = serde_json::to_string(&config)?;
    assert!(json.contains("perlPath"));
    assert!(json.contains("includePaths"));
    Ok(())
}

#[test]
fn test_launch_json_snippet() {
    let snippet = create_launch_json_snippet();
    assert!(snippet.contains("\"type\""));
    assert!(snippet.contains("perl"));
    assert!(snippet.contains("\"request\""));
    assert!(snippet.contains("launch"));
    assert!(snippet.contains("program"));
}

#[test]
fn test_launch_config_validation_missing_program() {
    // Test: program file doesn't exist
    let config = LaunchConfiguration {
        program: PathBuf::from("/nonexistent/script.pl"),
        args: vec![],
        cwd: None,
        env: HashMap::new(),
        perl_path: None,
        include_paths: vec![],
    };

    let result = config.validate();
    assert!(
        result.is_err(),
        "Should fail validation for missing program file"
    );
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("does not exist"),
        "Error should mention file doesn't exist"
    );
}

#[test]
fn test_launch_config_validation_program_is_directory() -> Result<(), Box<dyn std::error::Error>> {
    // Test: program path is a directory, not a file
    use std::env;
    let config = LaunchConfiguration {
        program: env::current_dir()?,
        args: vec![],
        cwd: None,
        env: HashMap::new(),
        perl_path: None,
        include_paths: vec![],
    };

    let result = config.validate();
    assert!(
        result.is_err(),
        "Should fail validation when program is a directory"
    );
    let err = result.err().ok_or("Expected an error")?;
    assert!(
        err.to_string().contains("not a file"),
        "Error should mention path is not a file"
    );
    Ok(())
}

#[test]
fn test_launch_config_validation_invalid_cwd() -> Result<(), Box<dyn std::error::Error>> {
    // Test: cwd is not a directory
    use std::env;

    // Create a config with a file as cwd (should fail)
    let temp_file = env::temp_dir().join("test_file.txt");
    std::fs::write(&temp_file, "test")?;

    let config = LaunchConfiguration {
        program: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
        args: vec![],
        cwd: Some(temp_file.clone()),
        env: HashMap::new(),
        perl_path: None,
        include_paths: vec![],
    };

    let result = config.validate();
    // Clean up temp file
    let _ = std::fs::remove_file(&temp_file);

    assert!(
        result.is_err(),
        "Should fail validation when cwd is not a directory"
    );
    Ok(())
}

#[test]
fn test_launch_config_validation_missing_cwd() {
    // Test: cwd directory doesn't exist
    let config = LaunchConfiguration {
        program: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
        args: vec![],
        cwd: Some(PathBuf::from("/nonexistent/directory")),
        env: HashMap::new(),
        perl_path: None,
        include_paths: vec![],
    };

    let result = config.validate();
    assert!(result.is_err(), "Should fail validation for missing cwd");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("does not exist"),
        "Error should mention directory doesn't exist"
    );
}

#[test]
fn test_launch_config_validation_invalid_perl_path() {
    // Test: perl_path doesn't exist
    let config = LaunchConfiguration {
        program: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
        args: vec![],
        cwd: None,
        env: HashMap::new(),
        perl_path: Some(PathBuf::from("/nonexistent/perl")),
        include_paths: vec![],
    };

    let result = config.validate();
    assert!(
        result.is_err(),
        "Should fail validation for missing perl binary"
    );
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("does not exist"),
        "Error should mention perl binary doesn't exist"
    );
}

#[test]
fn test_launch_config_path_resolution_absolute() -> Result<(), Box<dyn std::error::Error>> {
    // Test: absolute paths don't get modified
    let mut config = LaunchConfiguration {
        program: PathBuf::from("/absolute/path/script.pl"),
        args: vec![],
        cwd: Some(PathBuf::from("/absolute/cwd")),
        env: HashMap::new(),
        perl_path: None,
        include_paths: vec![PathBuf::from("/absolute/lib")],
    };

    config.resolve_paths(&PathBuf::from("/workspace"))?;

    assert_eq!(
        config.program,
        PathBuf::from("/absolute/path/script.pl"),
        "Absolute program path should be preserved"
    );
    assert_eq!(
        config.cwd.as_ref().ok_or("cwd should be Some")?,
        &PathBuf::from("/absolute/cwd"),
        "Absolute cwd should be preserved"
    );
    assert_eq!(
        config.include_paths[0],
        PathBuf::from("/absolute/lib"),
        "Absolute include path should be preserved"
    );
    Ok(())
}

#[test]
fn test_launch_config_path_resolution_relative() -> Result<(), Box<dyn std::error::Error>> {
    // Test: relative paths get resolved against workspace root
    let mut config = LaunchConfiguration {
        program: PathBuf::from("script.pl"),
        args: vec![],
        cwd: Some(PathBuf::from("build")),
        env: HashMap::new(),
        perl_path: None,
        include_paths: vec![PathBuf::from("lib")],
    };

    let workspace = PathBuf::from("/workspace");
    config.resolve_paths(&workspace)?;

    assert_eq!(
        config.program,
        workspace.join("script.pl"),
        "Relative program path should be resolved"
    );
    assert_eq!(
        config.cwd.as_ref().ok_or("cwd should be Some")?,
        &workspace.join("build"),
        "Relative cwd should be resolved"
    );
    assert_eq!(
        config.include_paths[0],
        workspace.join("lib"),
        "Relative include path should be resolved"
    );
    Ok(())
}

#[test]
fn test_launch_json_snippet_valid_json() -> Result<(), Box<dyn std::error::Error>> {
    // Test: generated JSON snippets parse correctly
    let snippet = create_launch_json_snippet();
    let parsed: serde_json::Value = serde_json::from_str(&snippet)?;

    assert_eq!(parsed["type"], "perl");
    assert_eq!(parsed["request"], "launch");
    assert!(parsed["program"].is_string());
    assert!(parsed["args"].is_array());
    assert!(parsed["perlPath"].is_string());
    assert!(parsed["includePaths"].is_array());
    Ok(())
}

#[test]
fn test_launch_config_empty_args() -> Result<(), Box<dyn std::error::Error>> {
    // Test: empty args array is valid
    let config = LaunchConfiguration {
        program: PathBuf::from("script.pl"),
        args: vec![],
        cwd: None,
        env: HashMap::new(),
        perl_path: None,
        include_paths: vec![],
    };

    let json = serde_json::to_string(&config)?;
    assert!(
        json.contains("\"args\":[]"),
        "Empty args should serialize correctly"
    );
    Ok(())
}

#[test]
fn test_launch_config_empty_include_paths() -> Result<(), Box<dyn std::error::Error>> {
    // Test: empty include_paths is valid
    let config = LaunchConfiguration {
        program: PathBuf::from("script.pl"),
        args: vec![],
        cwd: None,
        env: HashMap::new(),
        perl_path: None,
        include_paths: vec![],
    };

    let json = serde_json::to_string(&config)?;
    assert!(
        json.contains("\"includePaths\":[]"),
        "Empty include_paths should serialize correctly"
    );
    Ok(())
}
