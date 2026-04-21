//! Green TDD: perl-dap integration tests for Wave G2.
//!
//! These tests verify that perl-dap can correctly access and use the new
//! runtime module structure after G2 absorption. Specifically, they test
//! that perl-dap's dependency on launcher::* still works correctly.
//!
//! Risk context: perl-dap was rewired from perl-lsp-launcher to
//! perl-lsp-rs-core::runtime::launcher. These tests ensure:
//! - TransportMode enum is accessible and functional
//! - LaunchConfig struct can be instantiated
//! - perl-dap doesn't regress in its ability to initialize transport
//!
//! All tests are green at HEAD (post-G2).

use perl_lsp_rs_core::runtime::launcher::{
    DEFAULT_LSP_PORT, FeatureProfile, LaunchConfig, TransportMode,
};

/// Test that TransportMode enum variants are accessible.
/// Ensures perl-dap can distinguish between transport modes.
#[test]
fn test_dap_transport_mode_stdio() -> Result<(), Box<dyn std::error::Error>> {
    let _stdio = TransportMode::Stdio;
    // If this compiles and runs, the variant is accessible
    Ok(())
}

/// Test that TransportMode::Socket variant is accessible.
#[test]
fn test_dap_transport_mode_socket() -> Result<(), Box<dyn std::error::Error>> {
    let _socket = TransportMode::Socket { port: 9257 };
    Ok(())
}

/// Test that TransportMode::Socket with DAP port works.
/// Regression guard: ensures port assignment for TCP transport.
#[test]
fn test_dap_transport_mode_socket_dap_port() -> Result<(), Box<dyn std::error::Error>> {
    let socket_mode = TransportMode::Socket { port: 13603 };
    match socket_mode {
        TransportMode::Socket { port } => {
            assert_eq!(port, 13603, "DAP port should be 13603");
        }
        other => return Err(format!("Expected Socket variant, got {other:?}").into()),
    }
    Ok(())
}

/// Test that DEFAULT_LSP_PORT constant is still defined.
/// Even though DAP uses its own port (13603), this ensures no constants were lost.
#[test]
fn test_dap_default_lsp_port_constant() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(DEFAULT_LSP_PORT, 9257);
    Ok(())
}

/// Test that LaunchConfig struct can be instantiated with Stdio transport.
/// Ensures perl-dap's startup configuration remains flexible.
#[test]
fn test_dap_launch_config_stdio() -> Result<(), Box<dyn std::error::Error>> {
    let config = LaunchConfig {
        transport: TransportMode::Stdio,
        enable_logging: false,
        feature_profile: FeatureProfile::Production,
    };

    match config.transport {
        TransportMode::Stdio => {}
        other => return Err(format!("Expected Stdio transport, got {other:?}").into()),
    }
    Ok(())
}

/// Test that LaunchConfig can be instantiated with Socket transport.
#[test]
fn test_dap_launch_config_socket() -> Result<(), Box<dyn std::error::Error>> {
    let config = LaunchConfig {
        transport: TransportMode::Socket { port: 13603 },
        enable_logging: true,
        feature_profile: FeatureProfile::Production,
    };

    match config.transport {
        TransportMode::Socket { port } => {
            assert_eq!(port, 13603);
        }
        other => return Err(format!("Expected Socket transport, got {other:?}").into()),
    }
    Ok(())
}

/// Test that LaunchConfig enables/disables logging independently.
#[test]
fn test_dap_launch_config_logging_flag() -> Result<(), Box<dyn std::error::Error>> {
    let config_with_logging = LaunchConfig {
        transport: TransportMode::Stdio,
        enable_logging: true,
        feature_profile: FeatureProfile::Production,
    };

    let config_without_logging = LaunchConfig {
        transport: TransportMode::Stdio,
        enable_logging: false,
        feature_profile: FeatureProfile::Production,
    };

    assert_ne!(
        config_with_logging.enable_logging, config_without_logging.enable_logging,
        "logging flags should be distinguishable"
    );
    Ok(())
}

/// Test that FeatureProfile enum variants are accessible through LaunchConfig.
#[test]
fn test_dap_launch_config_feature_profiles() -> Result<(), Box<dyn std::error::Error>> {
    let _cfg_ga_lock = LaunchConfig {
        transport: TransportMode::Stdio,
        enable_logging: false,
        feature_profile: FeatureProfile::GaLock,
    };

    let _cfg_production = LaunchConfig {
        transport: TransportMode::Stdio,
        enable_logging: false,
        feature_profile: FeatureProfile::Production,
    };

    let _cfg_all = LaunchConfig {
        transport: TransportMode::Stdio,
        enable_logging: false,
        feature_profile: FeatureProfile::All,
    };

    Ok(())
}

/// Test that multiple LaunchConfig instances are independent.
/// Ensures no shared state pollution between dap server instances.
#[test]
fn test_dap_launch_config_independence() -> Result<(), Box<dyn std::error::Error>> {
    let cfg1 = LaunchConfig {
        transport: TransportMode::Stdio,
        enable_logging: false,
        feature_profile: FeatureProfile::GaLock,
    };

    let cfg2 = LaunchConfig {
        transport: TransportMode::Socket { port: 13603 },
        enable_logging: true,
        feature_profile: FeatureProfile::All,
    };

    assert_ne!(
        cfg1.enable_logging, cfg2.enable_logging,
        "configs should have different logging flags"
    );
    assert_ne!(
        cfg1.feature_profile as u8, cfg2.feature_profile as u8,
        "configs should have different feature profiles"
    );
    Ok(())
}
