use std::error::Error;

#[test]
fn perl_token_has_no_runtime_dependencies_unless_explicitly_allowed() -> Result<(), Box<dyn Error>>
{
    let cargo_toml = include_str!("../Cargo.toml");

    let mut in_runtime_dependencies = false;
    let mut in_allowlist = false;
    let mut runtime_deps = Vec::new();
    let mut allowed_runtime_deps = Vec::new();

    for raw_line in cargo_toml.lines() {
        let line = raw_line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            let section = &line[1..line.len() - 1];
            in_runtime_dependencies =
                section == "dependencies" || section.ends_with(".dependencies");
            in_allowlist = section == "package.metadata.perl-token-contract";
            continue;
        }

        if in_allowlist && line.starts_with("allowed-runtime-dependencies") {
            if let Some(start) = line.find('[') {
                if let Some(end) = line.rfind(']') {
                    let entries = &line[start + 1..end];
                    for item in entries.split(',').map(str::trim).filter(|item| !item.is_empty()) {
                        allowed_runtime_deps.push(item.trim_matches('"').to_string());
                    }
                }
            }
        }

        if in_runtime_dependencies {
            if let Some((name, _rest)) = line.split_once('=') {
                let dep_name = name.trim().to_string();
                if !dep_name.is_empty() {
                    runtime_deps.push(dep_name);
                }
            }
        }
    }

    let violations: Vec<String> =
        runtime_deps.into_iter().filter(|dep| !allowed_runtime_deps.contains(dep)).collect();

    assert!(
        violations.is_empty(),
        "perl-token must remain a leaf crate with std-only runtime dependencies. Unapproved dependencies: {:?}",
        violations,
    );

    Ok(())
}
