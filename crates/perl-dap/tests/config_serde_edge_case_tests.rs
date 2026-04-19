//! Serde round-trip and deserialization edge-case tests for perl-dap-config.
//!
//! These tests verify that LaunchConfiguration and AttachConfiguration
//! serialize/deserialize correctly and handle edge cases.

use perl_dap::config::{AttachConfiguration, LaunchConfiguration};
use std::collections::HashMap;
use std::path::PathBuf;

// ── LaunchConfiguration serde ──────────────────────────────────────

#[test]
fn launch_config_round_trip_all_fields() -> Result<(), Box<dyn std::error::Error>> {
    let mut env = HashMap::new();
    env.insert("PERL5LIB".to_string(), "lib".to_string());
    env.insert("DEBUG".to_string(), "1".to_string());

    let config = LaunchConfiguration {
        program: PathBuf::from("/path/to/script.pl"),
        args: vec!["--verbose".to_string(), "-n".to_string()],
        cwd: Some(PathBuf::from("/workspace")),
        env,
        perl_path: Some(PathBuf::from("/usr/local/bin/perl")),
        include_paths: vec![PathBuf::from("lib"), PathBuf::from("local/lib/perl5")],
    };

    let json = serde_json::to_string(&config)?;
    let back: LaunchConfiguration = serde_json::from_str(&json)?;

    assert_eq!(back.program, config.program);
    assert_eq!(back.args, config.args);
    assert_eq!(back.cwd, config.cwd);
    assert_eq!(back.perl_path, config.perl_path);
    assert_eq!(back.include_paths, config.include_paths);
    assert_eq!(back.env.len(), 2);
    Ok(())
}

#[test]
fn launch_config_round_trip_minimal() -> Result<(), Box<dyn std::error::Error>> {
    let config = LaunchConfiguration {
        program: PathBuf::from("test.pl"),
        args: vec![],
        cwd: None,
        env: HashMap::new(),
        perl_path: None,
        include_paths: vec![],
    };

    let json = serde_json::to_string(&config)?;
    let back: LaunchConfiguration = serde_json::from_str(&json)?;

    assert_eq!(back.program, PathBuf::from("test.pl"));
    assert!(back.cwd.is_none());
    assert!(back.perl_path.is_none());
    assert!(back.args.is_empty());
    assert!(back.include_paths.is_empty());
    Ok(())
}

#[test]
fn launch_config_deserialize_camel_case() -> Result<(), Box<dyn std::error::Error>> {
    let json = r#"{
        "program": "script.pl",
        "args": ["--debug"],
        "cwd": "/work",
        "env": {},
        "perlPath": "/usr/bin/perl",
        "includePaths": ["/lib"]
    }"#;

    let config: LaunchConfiguration = serde_json::from_str(json)?;
    assert_eq!(config.program, PathBuf::from("script.pl"));
    assert_eq!(config.args, vec!["--debug".to_string()]);
    assert_eq!(config.cwd, Some(PathBuf::from("/work")));
    assert_eq!(config.perl_path, Some(PathBuf::from("/usr/bin/perl")));
    assert_eq!(config.include_paths, vec![PathBuf::from("/lib")]);
    Ok(())
}

#[test]
fn launch_config_deserialize_optional_fields_missing() -> Result<(), Box<dyn std::error::Error>> {
    // Only required field is program; other fields have defaults via serde
    let json = r#"{"program": "minimal.pl"}"#;
    let config: LaunchConfiguration = serde_json::from_str(json)?;
    assert_eq!(config.program, PathBuf::from("minimal.pl"));
    assert!(config.args.is_empty());
    assert!(config.cwd.is_none());
    assert!(config.env.is_empty());
    assert!(config.perl_path.is_none());
    assert!(config.include_paths.is_empty());
    Ok(())
}

#[test]
fn launch_config_serializes_camel_case_field_names() -> Result<(), Box<dyn std::error::Error>> {
    let config = LaunchConfiguration {
        program: PathBuf::from("x.pl"),
        args: vec![],
        cwd: None,
        env: HashMap::new(),
        perl_path: Some(PathBuf::from("/usr/bin/perl")),
        include_paths: vec![PathBuf::from("lib")],
    };

    let json = serde_json::to_string(&config)?;
    assert!(json.contains("perlPath"), "Should use camelCase: {json}");
    assert!(
        json.contains("includePaths"),
        "Should use camelCase: {json}"
    );
    assert!(
        !json.contains("perl_path"),
        "Should not use snake_case: {json}"
    );
    assert!(
        !json.contains("include_paths"),
        "Should not use snake_case: {json}"
    );
    Ok(())
}

#[test]
fn launch_config_omits_none_fields() -> Result<(), Box<dyn std::error::Error>> {
    let config = LaunchConfiguration {
        program: PathBuf::from("x.pl"),
        args: vec![],
        cwd: None,
        env: HashMap::new(),
        perl_path: None,
        include_paths: vec![],
    };

    let json = serde_json::to_string(&config)?;
    assert!(
        !json.contains("perlPath"),
        "None perlPath should be omitted: {json}"
    );
    assert!(!json.contains("cwd"), "None cwd should be omitted: {json}");
    Ok(())
}

#[test]
fn launch_config_resolve_paths_no_cwd() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = LaunchConfiguration {
        program: PathBuf::from("script.pl"),
        args: vec![],
        cwd: None,
        env: HashMap::new(),
        perl_path: None,
        include_paths: vec![PathBuf::from("lib")],
    };

    let workspace = PathBuf::from("/workspace");
    config.resolve_paths(&workspace)?;

    assert_eq!(config.program, workspace.join("script.pl"));
    assert!(
        config.cwd.is_none(),
        "cwd should remain None after resolve_paths"
    );
    assert_eq!(config.include_paths[0], workspace.join("lib"));
    Ok(())
}

#[test]
fn launch_config_resolve_paths_multiple_include_paths() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = LaunchConfiguration {
        program: PathBuf::from("app.pl"),
        args: vec![],
        cwd: None,
        env: HashMap::new(),
        perl_path: None,
        include_paths: vec![
            PathBuf::from("lib"),
            PathBuf::from("/absolute/lib"),
            PathBuf::from("local/lib"),
        ],
    };

    let workspace = PathBuf::from("/ws");
    config.resolve_paths(&workspace)?;

    assert_eq!(config.include_paths[0], PathBuf::from("/ws/lib"));
    assert_eq!(
        config.include_paths[1],
        PathBuf::from("/absolute/lib"),
        "Absolute include path should be preserved"
    );
    assert_eq!(config.include_paths[2], PathBuf::from("/ws/local/lib"));
    Ok(())
}

// ── AttachConfiguration serde ──────────────────────────────────────

#[test]
fn attach_config_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let config = AttachConfiguration {
        host: "192.168.1.100".to_string(),
        port: 9229,
        timeout_ms: Some(15000),
        stop_on_entry: None,
    };

    let json = serde_json::to_string(&config)?;
    let back: AttachConfiguration = serde_json::from_str(&json)?;

    assert_eq!(back.host, "192.168.1.100");
    assert_eq!(back.port, 9229);
    assert_eq!(back.timeout_ms, Some(15000));
    Ok(())
}

#[test]
fn attach_config_round_trip_no_timeout() -> Result<(), Box<dyn std::error::Error>> {
    let config = AttachConfiguration {
        host: "localhost".to_string(),
        port: 13603,
        timeout_ms: None,
        stop_on_entry: None,
    };

    let json = serde_json::to_string(&config)?;
    assert!(
        !json.contains("timeoutMs"),
        "None timeoutMs should be omitted: {json}"
    );

    let back: AttachConfiguration = serde_json::from_str(&json)?;
    assert!(back.timeout_ms.is_none());
    Ok(())
}

#[test]
fn attach_config_deserialize_camel_case() -> Result<(), Box<dyn std::error::Error>> {
    let json = r#"{"host": "myhost", "port": 5555, "timeoutMs": 3000}"#;
    let config: AttachConfiguration = serde_json::from_str(json)?;

    assert_eq!(config.host, "myhost");
    assert_eq!(config.port, 5555);
    assert_eq!(config.timeout_ms, Some(3000));
    Ok(())
}

#[test]
fn attach_config_validation_boundary_timeout() {
    // Exactly at maximum (300_000) should succeed
    let config = AttachConfiguration {
        host: "localhost".to_string(),
        port: 13603,
        timeout_ms: Some(300_000),
        stop_on_entry: None,
    };
    assert!(config.validate().is_ok(), "Max timeout should be valid");

    // One over maximum should fail
    let config = AttachConfiguration {
        host: "localhost".to_string(),
        port: 13603,
        timeout_ms: Some(300_001),
        stop_on_entry: None,
    };
    assert!(config.validate().is_err(), "Over-max timeout should fail");
}

#[test]
fn attach_config_validation_port_boundaries() {
    // Port 1 is valid
    let config = AttachConfiguration {
        host: "localhost".to_string(),
        port: 1,
        timeout_ms: None,
        stop_on_entry: None,
    };
    assert!(config.validate().is_ok(), "Port 1 should be valid");

    // Port 65535 is valid (max u16)
    let config = AttachConfiguration {
        host: "localhost".to_string(),
        port: 65535,
        timeout_ms: None,
        stop_on_entry: None,
    };
    assert!(config.validate().is_ok(), "Port 65535 should be valid");
}

#[test]
fn attach_config_validation_timeout_1ms() {
    // Timeout of 1ms is the minimum valid timeout
    let config = AttachConfiguration {
        host: "localhost".to_string(),
        port: 13603,
        timeout_ms: Some(1),
        stop_on_entry: None,
    };
    assert!(config.validate().is_ok(), "1ms timeout should be valid");
}

// ── launch.json / attach.json snippet cross-checks ─────────────────

#[test]
fn launch_snippet_deserialization_produces_valid_config() -> Result<(), Box<dyn std::error::Error>>
{
    let snippet = perl_dap::config::create_launch_json_snippet();
    let parsed: serde_json::Value = serde_json::from_str(&snippet)?;

    // Verify it has all the required fields for VS Code DAP
    assert!(parsed.get("type").is_some(), "Must have 'type' field");
    assert!(parsed.get("request").is_some(), "Must have 'request' field");
    assert!(parsed.get("name").is_some(), "Must have 'name' field");
    assert!(parsed.get("program").is_some(), "Must have 'program' field");
    Ok(())
}

#[test]
fn attach_snippet_deserialization_produces_valid_config() -> Result<(), Box<dyn std::error::Error>>
{
    let snippet = perl_dap::config::create_attach_json_snippet();
    let parsed: serde_json::Value = serde_json::from_str(&snippet)?;

    assert!(parsed.get("type").is_some(), "Must have 'type' field");
    assert!(parsed.get("request").is_some(), "Must have 'request' field");
    assert!(parsed.get("name").is_some(), "Must have 'name' field");
    assert!(parsed.get("host").is_some(), "Must have 'host' field");
    assert!(parsed.get("port").is_some(), "Must have 'port' field");
    Ok(())
}
