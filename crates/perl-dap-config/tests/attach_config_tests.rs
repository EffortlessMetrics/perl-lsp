use perl_dap_config::{AttachConfiguration, create_attach_json_snippet};

#[test]
fn test_attach_configuration_default() {
    let config = AttachConfiguration::default();
    assert_eq!(config.host, "localhost");
    assert_eq!(config.port, 13603);
    assert_eq!(config.timeout_ms, Some(5000));
}

#[test]
fn test_attach_json_snippet() {
    let snippet = create_attach_json_snippet();
    assert!(snippet.contains("\"type\""));
    assert!(snippet.contains("perl"));
    assert!(snippet.contains("\"request\""));
    assert!(snippet.contains("attach"));
    assert!(snippet.contains("13603"));
}

#[test]
fn test_attach_config_custom_port() -> Result<(), Box<dyn std::error::Error>> {
    // Test: custom port handling
    let config = AttachConfiguration {
        host: "192.168.1.100".to_string(),
        port: 9000,
        timeout_ms: Some(10000),
        stop_on_entry: None,
    };

    let json = serde_json::to_string(&config)?;
    assert!(json.contains("192.168.1.100"), "Should contain custom host");
    assert!(json.contains("9000"), "Should contain custom port");
    Ok(())
}

#[test]
fn test_attach_config_validation_valid() {
    // Test: valid attach configuration
    let config = AttachConfiguration {
        host: "localhost".to_string(),
        port: 13603,
        timeout_ms: Some(5000),
        stop_on_entry: None,
    };

    assert!(config.validate().is_ok(), "Valid config should pass validation");
}

#[test]
fn test_attach_config_validation_empty_host() -> Result<(), Box<dyn std::error::Error>> {
    // Test: empty host fails validation
    let config = AttachConfiguration {
        host: "".to_string(),
        port: 13603,
        timeout_ms: Some(5000),
        stop_on_entry: None,
    };

    let result = config.validate();
    assert!(result.is_err(), "Empty host should fail validation");
    assert!(result.err().ok_or("Expected an error")?.to_string().contains("Host"));
    Ok(())
}

#[test]
fn test_attach_config_validation_whitespace_host() {
    // Test: whitespace-only host fails validation
    let config = AttachConfiguration {
        host: "   ".to_string(),
        port: 13603,
        timeout_ms: Some(5000),
        stop_on_entry: None,
    };

    let result = config.validate();
    assert!(result.is_err(), "Whitespace host should fail validation");
}

#[test]
fn test_attach_config_validation_zero_port() -> Result<(), Box<dyn std::error::Error>> {
    // Test: port 0 is invalid
    let config = AttachConfiguration {
        host: "localhost".to_string(),
        port: 0,
        timeout_ms: Some(5000),
        stop_on_entry: None,
    };

    let result = config.validate();
    assert!(result.is_err(), "Port 0 should fail validation");
    assert!(result.err().ok_or("Expected an error")?.to_string().contains("Port"));
    Ok(())
}

#[test]
fn test_attach_config_validation_zero_timeout() -> Result<(), Box<dyn std::error::Error>> {
    // Test: zero timeout fails validation
    let config = AttachConfiguration {
        host: "localhost".to_string(),
        port: 13603,
        timeout_ms: Some(0),
        stop_on_entry: None,
    };

    let result = config.validate();
    assert!(result.is_err(), "Zero timeout should fail validation");
    assert!(result.err().ok_or("Expected an error")?.to_string().contains("Timeout"));
    Ok(())
}

#[test]
fn test_attach_config_validation_excessive_timeout() {
    // Test: timeout > 5 minutes fails validation
    let config = AttachConfiguration {
        host: "localhost".to_string(),
        port: 13603,
        timeout_ms: Some(400_000), // 400 seconds
        stop_on_entry: None,
    };

    let result = config.validate();
    assert!(result.is_err(), "Excessive timeout should fail validation");
}

#[test]
fn test_attach_config_validation_no_timeout() {
    // Test: no timeout specified is valid
    let config = AttachConfiguration {
        host: "localhost".to_string(),
        port: 13603,
        timeout_ms: None,
        stop_on_entry: None,
    };

    assert!(config.validate().is_ok(), "Config without timeout should be valid");
}

#[test]
fn test_attach_json_snippet_valid_json() -> Result<(), Box<dyn std::error::Error>> {
    // Test: attach JSON snippet is valid and complete
    let snippet = create_attach_json_snippet();
    let parsed: serde_json::Value = serde_json::from_str(&snippet)?;

    assert_eq!(parsed["type"], "perl");
    assert_eq!(parsed["request"], "attach");
    assert_eq!(parsed["host"], "localhost");
    assert_eq!(parsed["port"], 13603);
    assert!(parsed["timeout"].is_number());
    Ok(())
}
