//! Test that newly connected features are properly advertised

// Integration tests print diagnostic output for CI troubleshooting; this is
// not the LSP server's stdio transport, so print_stdout/print_stderr don't
// apply the way they do to production code.
#![allow(clippy::print_stdout, clippy::print_stderr)]

mod support;
use support::lsp_harness::LspHarness;

#[test]
fn test_new_features_advertised() -> Result<(), Box<dyn std::error::Error>> {
    let mut harness = LspHarness::new();
    let init_response = harness.initialize(None)?;

    // Check that our newly connected features are advertised
    let caps = init_response
        .get("capabilities")
        .ok_or("Missing capabilities in initialization response")?;

    #[allow(clippy::single_component_path_imports)]
    {
        use serde_json;
        let caps_json = serde_json::to_string_pretty(&caps)?;
        println!("Capabilities: {}", caps_json);
    }

    assert!(
        caps.get("typeDefinitionProvider").is_some(),
        "typeDefinitionProvider should be advertised"
    );

    assert!(
        caps.get("implementationProvider").is_some(),
        "implementationProvider should be advertised"
    );

    assert!(
        caps.get("executeCommandProvider").is_some(),
        "executeCommandProvider should be advertised"
    );

    println!("✓ All new features are properly advertised!");
    Ok(())
}
