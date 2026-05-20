//! Validate the committed Claude swarm agent roster contract.
//!
//! This is the Rust-native implementation of the former
//! `scripts/validate_swarm_agent_roster.py` policy check. Keeping it in xtask
//! makes the roster validator part of the repository automation surface instead
//! of an ad-hoc Python dependency.

use color_eyre::eyre::{Result, bail, eyre};
use serde_json::Value as JsonValue;
use serde_yaml_ng::Value as YamlValue;
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

fn valid_agent_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

const ALLOWED_ROOT_KEYS: &[&str] = &["agents", "last_updated", "schema_version"];
const ALLOWED_AGENT_KEYS: &[&str] = &[
    "category",
    "class",
    "description",
    "file",
    "first_entrypoints",
    "handoff_to",
    "name",
    "owns",
    "spawned_by",
];
const ALLOWED_CLASSES: &[&str] = &["coordinator", "reusable_worker", "specialist_worker"];
const ALLOWED_CATEGORIES: &[&str] =
    &["docs_devex", "explore", "implementation", "quality", "quality_ops", "review", "scout"];

/// Validate `.claude/agents/agent-roster.json` against agent frontmatter.
pub fn run(root: Option<PathBuf>) -> Result<()> {
    let root = match root {
        Some(root) => root,
        None => std::env::current_dir()
            .map_err(|error| eyre!("failed to get current directory: {error}"))?,
    };
    let agents_dir = root.join(".claude/agents");
    let roster_path = agents_dir.join("agent-roster.json");
    let commands_dir = root.join(".claude/commands");
    let skills_dir = root.join(".claude/skills");

    if !roster_path.exists() {
        return validate_agent_frontmatter_only(&agents_dir);
    }

    let raw = fs::read_to_string(&roster_path)
        .map_err(|error| eyre!("failed to read {}: {error}", roster_path.display()))?;
    let data: JsonValue = serde_json::from_str(&raw)
        .map_err(|error| eyre!("failed to parse {}: {error}", roster_path.display()))?;
    let root_obj = data
        .as_object()
        .ok_or_else(|| eyre!("{} must parse to a JSON object", roster_path.display()))?;

    ensure_keys(root_obj.keys().map(String::as_str), ALLOWED_ROOT_KEYS, "root")?;
    if data.get("schema_version").and_then(JsonValue::as_i64) != Some(1) {
        bail!("schema_version must be 1");
    }
    let last_updated = json_string(data.get("last_updated"), "last_updated")?;
    chrono::NaiveDate::parse_from_str(last_updated, "%Y-%m-%d")
        .map_err(|error| eyre!("last_updated must be ISO date YYYY-MM-DD: {error}"))?;

    let agents = data
        .get("agents")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| eyre!("agents must be a non-empty array"))?;
    if agents.is_empty() {
        bail!("agents must be a non-empty array");
    }

    let agent_files = discover_agent_files(&agents_dir)?;
    if agent_files.is_empty() {
        bail!("no agent definition files found in {}", agents_dir.display());
    }

    let mut seen_names = BTreeSet::new();
    let mut seen_files = BTreeSet::new();
    let mut roster_files = BTreeSet::new();

    for (index, agent) in agents.iter().enumerate() {
        let agent_obj =
            agent.as_object().ok_or_else(|| eyre!("agent #{} must be an object", index + 1))?;
        ensure_keys(
            agent_obj.keys().map(String::as_str),
            ALLOWED_AGENT_KEYS,
            &format!("agent #{}", index + 1),
        )?;

        let field_prefix = format!("agent #{}.name", index + 1);
        let name = json_string(agent.get("name"), &field_prefix)?;
        if !valid_agent_name(name) {
            bail!("{name} must match [a-z0-9-]+");
        }
        if !seen_names.insert(name.to_string()) {
            bail!("duplicate agent name: {name}");
        }

        let agent_class = json_string(agent.get("class"), &format!("{name}.class"))?;
        if !ALLOWED_CLASSES.contains(&agent_class) {
            bail!("{name}.class must be one of {}", format_allowed(ALLOWED_CLASSES));
        }

        match (agent_class, agent.get("category")) {
            ("specialist_worker", category) => {
                let category = json_string(category, &format!("{name}.category"))?;
                if !ALLOWED_CATEGORIES.contains(&category) {
                    bail!("{name}.category must be one of {}", format_allowed(ALLOWED_CATEGORIES));
                }
            }
            (_, Some(_)) => bail!("{name}.category is only allowed for specialist_worker entries"),
            _ => {}
        }

        let agent_file = json_string(agent.get("file"), &format!("{name}.file"))?;
        if !seen_files.insert(agent_file.to_string()) {
            bail!("duplicate agent file: {agent_file}");
        }
        roster_files.insert(agent_file.to_string());

        let agent_path = agents_dir.join(agent_file);
        if !agent_path.exists() {
            bail!("{name}.file does not exist: {agent_file}");
        }

        json_string_list(agent.get("spawned_by"), &format!("{name}.spawned_by"))?;
        json_string_list(agent.get("handoff_to"), &format!("{name}.handoff_to"))?;
        let first_entrypoints =
            json_string_list(agent.get("first_entrypoints"), &format!("{name}.first_entrypoints"))?;
        let description = json_string(agent.get("description"), &format!("{name}.description"))?;

        match (agent_class, agent.get("owns")) {
            ("coordinator", owns) => {
                json_string(owns, &format!("{name}.owns"))?;
            }
            (_, Some(_)) => bail!("{name}.owns is only allowed for coordinator entries"),
            _ => {}
        }

        let frontmatter = load_frontmatter(&agent_path)?;
        let frontmatter_name =
            yaml_string(frontmatter.get("name"), &format!("{agent_file} frontmatter.name"))?;
        if frontmatter_name != name {
            bail!(
                "{agent_file} frontmatter.name ({frontmatter_name}) does not match roster name ({name})"
            );
        }

        let frontmatter_description = yaml_string(
            frontmatter.get("description"),
            &format!("{agent_file} frontmatter.description"),
        )?;
        if frontmatter_description != description {
            bail!("{agent_file} description does not match agent-roster.json");
        }

        if let Some(skills) = frontmatter.get("skills") {
            for skill in
                yaml_string_list(Some(skills), &format!("{agent_file} frontmatter.skills"))?
            {
                let skill_path = skills_dir.join(&skill).join("SKILL.md");
                if !skill_path.exists() {
                    bail!("{agent_file} references missing skill: {skill}");
                }
            }
        }

        for entrypoint in first_entrypoints {
            if !entrypoint.starts_with('/') {
                bail!("{name}.first_entrypoints entries must start with '/': {entrypoint}");
            }
            if !entrypoint_exists(&commands_dir, &skills_dir, &entrypoint) {
                bail!("{name}.first_entrypoints references missing command/skill: {entrypoint}");
            }
        }
    }

    if roster_files != agent_files {
        bail!(
            "agent-roster.json file set does not match .claude/agents surface: roster={} agents={}",
            format_set(&roster_files),
            format_set(&agent_files)
        );
    }

    println!("Validated {} agents in {}", agents.len(), roster_path.display());
    Ok(())
}

fn validate_agent_frontmatter_only(agents_dir: &Path) -> Result<()> {
    let agent_files = discover_agent_files(agents_dir)?;
    if agent_files.is_empty() {
        bail!("no agent definition files found in {}", agents_dir.display());
    }

    let mut seen_names = BTreeSet::new();
    for agent_file in &agent_files {
        let agent_path = agents_dir.join(agent_file);
        let frontmatter = load_frontmatter(&agent_path)?;
        let name = yaml_string(frontmatter.get("name"), &format!("{agent_file} frontmatter.name"))?;
        if !valid_agent_name(name) {
            bail!("{name} must match [a-z0-9-]+");
        }
        if !seen_names.insert(name.to_string()) {
            bail!("duplicate agent name: {name}");
        }
        let expected_file = format!("{name}.md");
        if expected_file != *agent_file {
            bail!("{agent_file} frontmatter.name ({name}) does not match file stem");
        }
        yaml_string(
            frontmatter.get("description"),
            &format!("{agent_file} frontmatter.description"),
        )?;
    }

    println!("Validated {} agent frontmatter files in {}", agent_files.len(), agents_dir.display());
    Ok(())
}

fn discover_agent_files(agents_dir: &Path) -> Result<BTreeSet<String>> {
    let mut files = BTreeSet::new();
    for entry in fs::read_dir(agents_dir)
        .map_err(|error| eyre!("failed to read {}: {error}", agents_dir.display()))?
    {
        let entry = entry
            .map_err(|error| eyre!("failed to read {} entry: {error}", agents_dir.display()))?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !matches!(name, "README.md" | "AGENT_CATALOG.md") {
            files.insert(name.to_string());
        }
    }
    Ok(files)
}

fn load_frontmatter(path: &Path) -> Result<serde_yaml_ng::Mapping> {
    let text = fs::read_to_string(path)
        .map_err(|error| eyre!("failed to read {}: {error}", path.display()))?;
    if !text.starts_with("---\n") {
        bail!("{} must start with YAML frontmatter", path.display());
    }
    let Some(rest) = text.strip_prefix("---\n") else {
        bail!("{} must start with YAML frontmatter", path.display());
    };
    let Some((frontmatter, _body)) = rest.split_once("---") else {
        bail!("{} must contain opening and closing frontmatter markers", path.display());
    };
    let data: YamlValue = serde_yaml_ng::from_str(frontmatter)
        .map_err(|error| eyre!("{} has invalid YAML frontmatter: {error}", path.display()))?;
    data.as_mapping()
        .cloned()
        .ok_or_else(|| eyre!("{} frontmatter must parse to a mapping", path.display()))
}

fn entrypoint_exists(commands_dir: &Path, skills_dir: &Path, entrypoint: &str) -> bool {
    let name = entrypoint.trim_start_matches('/');
    commands_dir.join(format!("{name}.md")).exists()
        || skills_dir.join(name).join("SKILL.md").exists()
}

fn ensure_keys<'a>(
    keys: impl Iterator<Item = &'a str>,
    allowed: &[&str],
    label: &str,
) -> Result<()> {
    let allowed: BTreeSet<&str> = allowed.iter().copied().collect();
    let extra: Vec<&str> = keys.filter(|key| !allowed.contains(key)).collect();
    if !extra.is_empty() {
        bail!("{label} has unexpected keys: {extra:?}");
    }
    Ok(())
}

fn json_string<'a>(value: Option<&'a JsonValue>, field: &str) -> Result<&'a str> {
    let value = value
        .and_then(JsonValue::as_str)
        .ok_or_else(|| eyre!("{field} must be a non-empty string"))?;
    if value.trim().is_empty() {
        bail!("{field} must be a non-empty string");
    }
    Ok(value)
}

fn json_string_list(value: Option<&JsonValue>, field: &str) -> Result<Vec<String>> {
    let values = value
        .and_then(JsonValue::as_array)
        .ok_or_else(|| eyre!("{field} must be a non-empty array"))?;
    if values.is_empty() {
        bail!("{field} must be a non-empty array");
    }
    values
        .iter()
        .enumerate()
        .map(|(index, item)| {
            json_string(Some(item), &format!("{field}[{index}]")).map(str::to_string)
        })
        .collect()
}

fn yaml_string<'a>(value: Option<&'a YamlValue>, field: &str) -> Result<&'a str> {
    let value = value
        .and_then(YamlValue::as_str)
        .ok_or_else(|| eyre!("{field} must be a non-empty string"))?;
    if value.trim().is_empty() {
        bail!("{field} must be a non-empty string");
    }
    Ok(value)
}

fn yaml_string_list(value: Option<&YamlValue>, field: &str) -> Result<Vec<String>> {
    let values = value
        .and_then(YamlValue::as_sequence)
        .ok_or_else(|| eyre!("{field} must be a non-empty array"))?;
    if values.is_empty() {
        bail!("{field} must be a non-empty array");
    }
    values
        .iter()
        .enumerate()
        .map(|(index, item)| {
            yaml_string(Some(item), &format!("{field}[{index}]")).map(str::to_string)
        })
        .collect()
}

fn format_allowed(values: &[&str]) -> String {
    format!("{:?}", values)
}

fn format_set(values: &BTreeSet<String>) -> String {
    format!("{:?}", values)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_file(path: &Path, content: &str) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| eyre!("failed to create {}: {error}", parent.display()))?;
        }
        fs::write(path, content)
            .map_err(|error| eyre!("failed to write {}: {error}", path.display()))
    }

    fn write_agent(root: &Path, file: &str, name: &str, description: &str) -> Result<()> {
        write_file(
            &root.join(".claude/agents").join(file),
            &format!("---\nname: {name}\ndescription: {description}\n---\n\nagent body\n"),
        )
    }

    #[test]
    fn frontmatter_only_validation_accepts_agent_files_without_roster() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        write_agent(tmp.path(), "builder.md", "builder", "Build focused patches")?;

        run(Some(tmp.path().to_path_buf()))
    }

    #[test]
    fn roster_validation_accepts_matching_agent_contract() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        write_agent(tmp.path(), "builder.md", "builder", "Build focused patches")?;
        write_file(&tmp.path().join(".claude/commands/builder-read.md"), "command body\n")?;
        write_file(&tmp.path().join(".claude/skills/swarm/SKILL.md"), "skill body\n")?;
        write_file(
            &tmp.path().join(".claude/agents/agent-roster.json"),
            r#"{
  "schema_version": 1,
  "last_updated": "2026-05-20",
  "agents": [
    {
      "name": "builder",
      "class": "specialist_worker",
      "category": "implementation",
      "file": "builder.md",
      "spawned_by": ["lead-build"],
      "handoff_to": ["reviewer"],
      "first_entrypoints": ["/builder-read", "/swarm"],
      "description": "Build focused patches"
    }
  ]
}
"#,
        )?;

        run(Some(tmp.path().to_path_buf()))
    }

    #[test]
    fn roster_validation_rejects_agent_file_without_roster_entry() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        write_agent(tmp.path(), "builder.md", "builder", "Build focused patches")?;
        write_agent(tmp.path(), "reviewer.md", "reviewer", "Review focused patches")?;
        write_file(&tmp.path().join(".claude/commands/builder-read.md"), "command body\n")?;
        write_file(
            &tmp.path().join(".claude/agents/agent-roster.json"),
            r#"{
  "schema_version": 1,
  "last_updated": "2026-05-20",
  "agents": [
    {
      "name": "builder",
      "class": "reusable_worker",
      "file": "builder.md",
      "spawned_by": ["lead-build"],
      "handoff_to": ["reviewer"],
      "first_entrypoints": ["/builder-read"],
      "description": "Build focused patches"
    }
  ]
}
"#,
        )?;

        match run(Some(tmp.path().to_path_buf())) {
            Ok(()) => bail!("missing roster entry should reject the file set"),
            Err(error) => assert!(error.to_string().contains("file set does not match")),
        }
        Ok(())
    }
}
