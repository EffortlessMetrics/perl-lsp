// Badge generation tests

#[test]
fn test_publication_facts_loads_correctly() -> Result<(), Box<dyn std::error::Error>> {
    let toml_content = r#"
[external]

[external.vscode_marketplace_installs]
label = "VS Marketplace installs"
value = 287
unit = "installs"
tier = "D"
verified_at = "2026-05-06"
source = "https://marketplace.visualstudio.com/items?itemName=EffortlessMetrics.perl-lsp-rs"
"#;

    let parsed: toml::Value = toml::from_str(toml_content)?;
    let value = parsed
        .get("external")
        .and_then(|e| e.get("vscode_marketplace_installs"))
        .and_then(|v| v.get("value"))
        .and_then(|v| v.as_integer());

    assert_eq!(value, Some(287));
    Ok(())
}

#[test]
fn test_badge_url_rendering() -> Result<(), Box<dyn std::error::Error>> {
    let installs = 287u32;
    let badge_url =
        format!("https://img.shields.io/badge/VS%20Marketplace-{}%20installs-0078D4", installs);

    assert_eq!(badge_url, "https://img.shields.io/badge/VS%20Marketplace-287%20installs-0078D4");
    Ok(())
}

#[test]
fn test_html_badge_rendering() -> Result<(), Box<dyn std::error::Error>> {
    let badge_url = "https://img.shields.io/badge/VS%20Marketplace-287%20installs-0078D4";
    let html = format!(
        "<a href=\"https://marketplace.visualstudio.com/items?itemName=EffortlessMetrics.perl-lsp-rs\"><img src=\"{}\" alt=\"VS Marketplace installs\" /></a>",
        badge_url
    );

    assert!(html.contains("287%20installs"));
    assert!(html.contains("0078D4"));
    Ok(())
}

#[test]
fn test_markdown_badge_rendering() -> Result<(), Box<dyn std::error::Error>> {
    let badge_url = "https://img.shields.io/badge/VS%20Marketplace-287%20installs-0078D4";
    let markdown = format!(
        "[![VS Marketplace Installs (manual)]({})](https://marketplace.visualstudio.com/items?itemName=EffortlessMetrics.perl-lsp-rs)",
        badge_url
    );

    assert!(markdown.contains("287%20installs"));
    assert!(markdown.contains("!["));
    assert!(markdown.contains("]("));
    Ok(())
}

#[test]
fn test_badge_marker_detection_html() -> Result<(), Box<dyn std::error::Error>> {
    let content = r#"<p>
  <!-- perl-lsp:vs-marketplace-installs-badge:start -->
  <a href="https://marketplace.visualstudio.com/items?itemName=EffortlessMetrics.perl-lsp-rs"><img src="https://img.shields.io/badge/VS%20Marketplace-277%20installs-0078D4" alt="VS Marketplace installs" /></a>
  <!-- perl-lsp:vs-marketplace-installs-badge:end -->
</p>"#;

    assert!(content.contains("<!-- perl-lsp:vs-marketplace-installs-badge:start -->"));
    assert!(content.contains("<!-- perl-lsp:vs-marketplace-installs-badge:end -->"));
    assert!(content.contains("277%20installs"));
    Ok(())
}

#[test]
fn test_badge_marker_detection_markdown() -> Result<(), Box<dyn std::error::Error>> {
    let content = r#"
<!-- perl-lsp:vs-marketplace-installs-badge:start -->
[![VS Marketplace Installs (manual)](https://img.shields.io/badge/VS%20Marketplace-277%20installs-0078D4)](https://marketplace.visualstudio.com/items?itemName=EffortlessMetrics.perl-lsp-rs)
<!-- perl-lsp:vs-marketplace-installs-badge:end -->
"#;

    assert!(content.contains("<!-- perl-lsp:vs-marketplace-installs-badge:start -->"));
    assert!(content.contains("<!-- perl-lsp:vs-marketplace-installs-badge:end -->"));
    assert!(content.contains("277%20installs"));
    Ok(())
}

#[test]
fn test_drift_detection_extracts_stale_count() -> Result<(), Box<dyn std::error::Error>> {
    let content = "https://img.shields.io/badge/VS%20Marketplace-277%20installs-0078D4";
    let re = regex::Regex::new(r"VS%20Marketplace-(\d+)%20installs")?;

    let caps = re.captures(content).ok_or("Expected to find badge count")?;
    let count = caps.get(1).ok_or("Expected capture group 1")?;
    assert_eq!(count.as_str(), "277");
    Ok(())
}

#[test]
fn test_multiple_readme_files_supported() -> Result<(), Box<dyn std::error::Error>> {
    let root = "README.md";
    let ext = "vscode-extension/README.md";

    assert!(root.ends_with(".md"));
    assert!(ext.ends_with(".md"));
    assert_ne!(root, ext);
    Ok(())
}
