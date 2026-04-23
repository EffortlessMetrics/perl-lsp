//! Regression guards for docs/reference/STABILITY.md.
//!
//! These tests keep the written stability contract aligned with the workspace
//! truth sources (Cargo.toml metadata).

use std::fs;
use std::path::PathBuf;

fn project_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop();
    dir
}

#[test]
fn stability_doc_mentions_current_release_line_and_publish_count()
-> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let cargo_toml = fs::read_to_string(root.join("Cargo.toml"))?;
    let cargo_value: toml::Value = toml::from_str(&cargo_toml)?;

    let version = cargo_value
        .get("workspace")
        .and_then(|v| v.get("package"))
        .and_then(|v| v.get("version"))
        .and_then(toml::Value::as_str)
        .ok_or("workspace.package.version missing from Cargo.toml")?;

    let mut parts = version.split('.');
    let major = parts.next().ok_or("missing major version component")?;
    let minor = parts.next().ok_or("missing minor version component")?;
    let release_line = format!("v{major}.{minor}.x");

    let publish_allow = cargo_value
        .get("workspace")
        .and_then(|v| v.get("metadata"))
        .and_then(|v| v.get("publish"))
        .and_then(|v| v.get("allow"))
        .and_then(toml::Value::as_array)
        .ok_or("workspace.metadata.publish.allow missing from Cargo.toml")?;
    let publish_count = publish_allow.len();

    let stability_doc = fs::read_to_string(root.join("docs/reference/STABILITY.md"))?;

    assert!(
        stability_doc.contains(&release_line),
        "STABILITY.md must mention the current release line `{release_line}` (derived from workspace version `{version}`)"
    );

    let publish_count_phrase = format!("Published crate set:** {publish_count} crates");
    assert!(
        stability_doc.contains(&publish_count_phrase),
        "STABILITY.md must mention `{publish_count_phrase}` to keep the contract aligned with Cargo.toml"
    );

    Ok(())
}

#[test]
fn stability_doc_lists_facade_ratchet_crates() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let stability_doc = fs::read_to_string(root.join("docs/reference/STABILITY.md"))?;

    let facade_crates = ["perl-lsp-rs", "perl-parser", "perl-uri", "perl-dap", "perllsp"];

    for crate_name in facade_crates {
        assert!(
            stability_doc.contains(crate_name),
            "STABILITY.md must mention facade ratchet crate `{crate_name}`"
        );
    }

    Ok(())
}
