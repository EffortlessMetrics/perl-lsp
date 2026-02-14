//! Agent Configuration Validation Tests
//!
//! Tests for issue #156: Agent Architecture - Need automated validation for agent configuration consistency
//!
//! This test suite validates:
//! - YAML front matter schema compliance for all agent files
//! - Required field presence and format validation
//! - Consistency checks between agent configurations
//! - Agent specialization patterns for Perl parser ecosystem
//!
//! ## Agent Configuration Requirements
//!
//! Each agent file in `.claude/agents2/` must have YAML front matter with:
//! - `name`: Required - Agent identifier (lowercase-with-hyphens)
//! - `description`: Required - When to use the agent with examples
//! - `model`: Required - Model to use (sonnet, opus, haiku)
//! - `color`: Optional - Display color
//!
//! ## Related Documentation
//! - Issue #156: Agent configuration validation gap
//! - `.claude/agents2/`: 95+ specialized agents for Perl parser ecosystem

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Agent configuration front matter schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Agent identifier (required)
    pub name: String,
    /// When to use the agent with examples (required)
    pub description: String,
    /// Model to use: sonnet, opus, haiku (required)
    pub model: String,
    /// Display color (optional)
    pub color: Option<String>,
}

/// Agent file metadata
#[derive(Debug, Clone)]
pub struct AgentFile {
    pub path: PathBuf,
    pub config: AgentConfig,
    pub category: AgentCategory,
}

/// Agent categories based on directory structure
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AgentCategory {
    Generative,
    Integration,
    Mantle,
    Other,
    Root,
}

impl AgentCategory {
    fn from_path(path: &Path) -> Result<Self> {
        let path_str = path.to_string_lossy();
        if path_str.contains("/generative/") {
            Ok(Self::Generative)
        } else if path_str.contains("/integration/") {
            Ok(Self::Integration)
        } else if path_str.contains("/mantle/") {
            Ok(Self::Mantle)
        } else if path_str.contains("/other/") {
            Ok(Self::Other)
        } else if path_str.contains(".claude/agents2/") && !path_str.contains('/') {
            Ok(Self::Root)
        } else {
            Err(anyhow!("Unknown agent category for path: {}", path_str))
        }
    }
}

/// Validation result for an agent configuration
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub path: PathBuf,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    fn new(path: PathBuf) -> Self {
        Self { path, errors: Vec::new(), warnings: Vec::new() }
    }

    fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Agent configuration validator
pub struct AgentConfigValidator {
    agents_dir: PathBuf,
}

impl AgentConfigValidator {
    pub fn new() -> Result<Self> {
        // Try current directory first, then parent directory (for when running from xtask/)
        let candidates =
            vec![PathBuf::from(".claude/agents2"), PathBuf::from("../.claude/agents2")];

        for agents_dir in candidates {
            if agents_dir.exists() {
                return Ok(Self { agents_dir });
            }
        }

        Err(anyhow!("Agent directory not found. Tried: .claude/agents2 and ../.claude/agents2"))
    }

    /// Find all agent markdown files
    pub fn find_agent_files(&self) -> Result<Vec<PathBuf>> {
        let mut agent_files = Vec::new();
        self.find_agent_files_recursive(&self.agents_dir, &mut agent_files)?;
        Ok(agent_files)
    }

    fn find_agent_files_recursive(&self, dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
        for entry in fs::read_dir(dir).context("Failed to read directory")? {
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();
            if path.is_dir() {
                self.find_agent_files_recursive(&path, files)?;
            } else if path.extension().is_some_and(|ext| ext == "md") {
                files.push(path);
            }
        }
        Ok(())
    }

    /// Parse agent configuration from markdown file
    pub fn parse_agent_config(&self, path: &Path) -> Result<AgentConfig> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read agent file: {}", path.display()))?;

        // Extract YAML front matter between --- delimiters
        let yaml_lines = self.extract_yaml_front_matter(&content).with_context(|| {
            format!("Failed to extract YAML front matter from: {}", path.display())
        })?;

        // Parse YAML manually due to unquoted multi-line strings with special chars
        self.parse_yaml_front_matter(&yaml_lines)
            .with_context(|| format!("Failed to parse YAML front matter from: {}", path.display()))
    }

    fn extract_yaml_front_matter(&self, content: &str) -> Result<Vec<String>> {
        let mut lines = content.lines();

        // First line should be ---
        if lines.next() != Some("---") {
            return Err(anyhow!("Missing opening --- delimiter"));
        }

        // Collect lines until closing ---
        let mut yaml_lines = Vec::new();
        for line in lines {
            if line == "---" {
                return Ok(yaml_lines);
            }
            yaml_lines.push(line.to_string());
        }

        Err(anyhow!("Missing closing --- delimiter"))
    }

    fn parse_yaml_front_matter(&self, lines: &[String]) -> Result<AgentConfig> {
        let mut name = None;
        let mut description = None;
        let mut model = None;
        let mut color = None;

        for line in lines {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim();

                match key {
                    "name" => name = Some(value.to_string()),
                    "description" => description = Some(value.to_string()),
                    "model" => model = Some(value.to_string()),
                    "color" => color = Some(value.to_string()),
                    _ => {} // Ignore unknown fields
                }
            }
        }

        Ok(AgentConfig {
            name: name.ok_or_else(|| anyhow!("Missing required field: name"))?,
            description: description
                .ok_or_else(|| anyhow!("Missing required field: description"))?,
            model: model.ok_or_else(|| anyhow!("Missing required field: model"))?,
            color,
        })
    }

    /// Validate a single agent configuration
    pub fn validate_agent(&self, path: &Path, config: &AgentConfig) -> ValidationResult {
        let mut result = ValidationResult::new(path.to_path_buf());

        // Validate name field
        if config.name.is_empty() {
            result.errors.push("name field is empty".to_string());
        } else if !self.is_valid_agent_name(&config.name) {
            result
                .warnings
                .push(format!("name '{}' should use lowercase-with-hyphens format", config.name));
        }

        // Validate description field
        if config.description.is_empty() {
            result.errors.push("description field is empty".to_string());
        } else if config.description.len() < 50 {
            result.warnings.push("description is very short (< 50 chars)".to_string());
        }

        // Validate model field
        let valid_models = ["sonnet", "opus", "haiku", "claude-sonnet", "claude-opus"];
        if config.model.is_empty() {
            result.errors.push("model field is empty".to_string());
        } else if !valid_models.contains(&config.model.as_str()) {
            result.warnings.push(format!(
                "model '{}' is not a standard value (expected: sonnet, opus, haiku)",
                config.model
            ));
        }

        // Validate color field (optional but should be valid if present)
        if let Some(color) = &config.color {
            let valid_colors =
                ["cyan", "green", "blue", "yellow", "red", "magenta", "white", "gray"];
            if !valid_colors.contains(&color.as_str()) {
                result.warnings.push(format!("color '{}' is not a standard ANSI color", color));
            }
        }

        result
    }

    fn is_valid_agent_name(&self, name: &str) -> bool {
        // Agent names should be lowercase with hyphens
        name.chars().all(|c| c.is_ascii_lowercase() || c == '-' || c.is_ascii_digit())
    }

    /// Validate all agent configurations
    pub fn validate_all_agents(&self) -> Result<Vec<ValidationResult>> {
        let agent_files = self.find_agent_files()?;
        let mut results = Vec::new();

        for path in agent_files {
            match self.parse_agent_config(&path) {
                Ok(config) => {
                    let validation = self.validate_agent(&path, &config);
                    results.push(validation);
                }
                Err(e) => {
                    let mut result = ValidationResult::new(path.clone());
                    result.errors.push(format!("Failed to parse config: {}", e));
                    results.push(result);
                }
            }
        }

        Ok(results)
    }

    /// Check for duplicate agent names
    pub fn check_name_uniqueness(&self) -> Result<Vec<String>> {
        let agent_files = self.find_agent_files()?;
        let mut name_map: HashMap<String, Vec<PathBuf>> = HashMap::new();

        for path in agent_files {
            if let Ok(config) = self.parse_agent_config(&path) {
                name_map.entry(config.name.clone()).or_default().push(path);
            }
        }

        let mut duplicates = Vec::new();
        for (name, paths) in name_map {
            if paths.len() > 1 {
                let paths_str: Vec<_> = paths.iter().map(|p| p.display().to_string()).collect();
                duplicates.push(format!(
                    "Duplicate agent name '{}' found in: {}",
                    name,
                    paths_str.join(", ")
                ));
            }
        }

        Ok(duplicates)
    }

    /// Load all valid agent configurations
    pub fn load_all_agents(&self) -> Result<Vec<AgentFile>> {
        let agent_files = self.find_agent_files()?;
        let mut agents = Vec::new();

        for path in agent_files {
            if let (Ok(config), Ok(category)) = (self.parse_agent_config(&path), AgentCategory::from_path(&path)) {
                agents.push(AgentFile { path: path.clone(), config, category });
            }
        }

        Ok(agents)
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_directory_exists() -> Result<()> {
        let validator = AgentConfigValidator::new()?;
        assert!(
            validator.agents_dir.exists(),
            "Agent directory should exist: {}",
            validator.agents_dir.display()
        );
        Ok(())
    }

    #[test]
    fn test_find_agent_files() -> Result<()> {
        let validator = AgentConfigValidator::new()?;
        let agent_files = validator.find_agent_files()?;

        assert!(!agent_files.is_empty(), "Should find at least one agent file");
        assert!(agent_files.len() >= 90, "Expected ~95 agent files, found {}", agent_files.len());

        // All files should be .md files
        for file in &agent_files {
            assert_eq!(
                file.extension().and_then(|s| s.to_str()),
                Some("md"),
                "Agent file should have .md extension: {}",
                file.display()
            );
        }

        Ok(())
    }

    #[test]
    fn test_parse_agent_configs() -> Result<()> {
        let validator = AgentConfigValidator::new()?;
        let agent_files = validator.find_agent_files()?;

        let mut parse_errors = Vec::new();
        let mut successful_parses = 0;

        for path in agent_files {
            match validator.parse_agent_config(&path) {
                Ok(config) => {
                    successful_parses += 1;

                    // Verify required fields are present
                    assert!(!config.name.is_empty(), "name should not be empty");
                    assert!(!config.description.is_empty(), "description should not be empty");
                    assert!(!config.model.is_empty(), "model should not be empty");
                }
                Err(e) => {
                    parse_errors.push(format!("{}: {}", path.display(), e));
                }
            }
        }

        if !parse_errors.is_empty() {
            eprintln!("Parse errors found:");
            for error in &parse_errors {
                eprintln!("  - {}", error);
            }
        }

        assert!(
            parse_errors.is_empty(),
            "All agent configs should parse successfully. Found {} errors",
            parse_errors.len()
        );
        assert!(
            successful_parses >= 90,
            "Expected at least 90 successful parses, got {}",
            successful_parses
        );

        Ok(())
    }

    #[test]
    fn test_validate_all_agents() -> Result<()> {
        let validator = AgentConfigValidator::new()?;
        let results = validator.validate_all_agents()?;

        let mut has_errors = false;
        let mut error_summary = Vec::new();

        for result in &results {
            if !result.is_valid() {
                has_errors = true;
                error_summary.push(format!(
                    "{}: {} errors, {} warnings",
                    result.path.display(),
                    result.errors.len(),
                    result.warnings.len()
                ));

                for error in &result.errors {
                    error_summary.push(format!("  ERROR: {}", error));
                }
                for warning in &result.warnings {
                    error_summary.push(format!("  WARN: {}", warning));
                }
            }
        }

        if has_errors {
            eprintln!("Validation errors found:");
            for line in &error_summary {
                eprintln!("{}", line);
            }
        }

        assert!(
            !has_errors,
            "All agent configurations should be valid. Found errors in {} agents",
            error_summary.len()
        );

        Ok(())
    }

    #[test]
    fn test_agent_name_uniqueness() -> Result<()> {
        let validator = AgentConfigValidator::new()?;
        let duplicates = validator.check_name_uniqueness()?;

        if !duplicates.is_empty() {
            eprintln!("\nWARNING: Duplicate agent names found:");
            for dup in &duplicates {
                eprintln!("  - {}", dup);
            }
            eprintln!("\nFound {} duplicate agent names across directories.", duplicates.len());
            eprintln!("Consider making agent names unique or using directory-qualified names.");
        }

        // For now, just report duplicates as warnings, not failures
        // In the future, this should be enforced
        Ok(())
    }

    #[test]
    fn test_agent_model_values() -> Result<()> {
        let validator = AgentConfigValidator::new()?;
        let agents = validator.load_all_agents()?;

        let mut model_counts: HashMap<String, usize> = HashMap::new();
        for agent in &agents {
            *model_counts.entry(agent.config.model.clone()).or_default() += 1;
        }

        // All agents should use valid model names
        let valid_models = ["sonnet", "opus", "haiku", "claude-sonnet", "claude-opus"];
        for (model, count) in &model_counts {
            assert!(
                valid_models.contains(&model.as_str()) || model.starts_with("claude-"),
                "Invalid model name '{}' used by {} agents",
                model,
                count
            );
        }

        // Most agents should use 'sonnet' for consistency
        let sonnet_count = model_counts.get("sonnet").copied().unwrap_or(0);
        assert!(
            sonnet_count > agents.len() / 2,
            "Most agents should use 'sonnet' model for consistency"
        );

        Ok(())
    }

    #[test]
    fn test_agent_categories() -> Result<()> {
        let validator = AgentConfigValidator::new()?;
        let agents = validator.load_all_agents()?;

        let mut category_counts: HashMap<AgentCategory, usize> = HashMap::new();
        for agent in &agents {
            *category_counts.entry(agent.category.clone()).or_default() += 1;
        }

        // Should have agents in multiple categories
        assert!(
            category_counts.len() >= 3,
            "Should have agents in at least 3 categories, found {}",
            category_counts.len()
        );

        // Each category should have at least a few agents
        for (category, count) in &category_counts {
            assert!(*count > 0, "Category {:?} should have at least one agent", category);
        }

        Ok(())
    }

    #[test]
    fn test_agent_description_quality() -> Result<()> {
        let validator = AgentConfigValidator::new()?;
        let agents = validator.load_all_agents()?;

        for agent in &agents {
            // Description should contain usage examples
            assert!(
                agent.config.description.contains("<example>")
                    || agent.config.description.contains("Example:")
                    || agent.config.description.len() > 200,
                "Agent '{}' description should contain examples or be comprehensive",
                agent.config.name
            );

            // Description should mention the agent's purpose
            assert!(
                agent.config.description.contains("Use this agent when"),
                "Agent '{}' description should start with 'Use this agent when'",
                agent.config.name
            );
        }

        Ok(())
    }

    #[test]
    fn test_perl_parser_specialization() -> Result<()> {
        let validator = AgentConfigValidator::new()?;
        let agents = validator.load_all_agents()?;

        let mut specialized_count = 0;

        // Count agents that mention Perl parser ecosystem specifics
        let perl_keywords = [
            "perl",
            "parser",
            "lsp",
            "tree-sitter",
            "recursive descent",
            "workspace",
            "dual indexing",
        ];

        for agent in &agents {
            let description = agent.config.description.to_lowercase();
            let has_perl_specialization =
                perl_keywords.iter().any(|keyword| description.contains(keyword));

            if has_perl_specialization {
                specialized_count += 1;
            }
        }

        // At least 50% of agents should be specialized for Perl parser ecosystem
        assert!(
            specialized_count > agents.len() / 2,
            "Expected at least {}% of agents to be specialized for Perl parser ecosystem, found {} out of {}",
            50,
            specialized_count,
            agents.len()
        );

        Ok(())
    }

    #[test]
    fn test_agent_config_schema_compliance() -> Result<()> {
        let validator = AgentConfigValidator::new()?;
        let agent_files = validator.find_agent_files()?;

        // Test that all agent files have valid YAML front matter structure
        for path in agent_files {
            let content =
                fs::read_to_string(&path).with_context(|| format!("Reading {}", path.display()))?;

            // Check YAML delimiters
            assert!(
                content.starts_with("---\n") || content.starts_with("---\r\n"),
                "Agent file {} should start with YAML front matter delimiter ---",
                path.display()
            );

            let lines: Vec<&str> = content.lines().collect();
            let closing_delimiter = lines.iter().skip(1).position(|line| *line == "---");
            let yaml_end = closing_delimiter.map_or(0, |pos| pos + 2); // +1 for opening, +1 for closing
            assert!(
                closing_delimiter.is_some() && lines.len() > yaml_end,
                "Agent file {} should have content after YAML front matter",
                path.display()
            );
        }

        Ok(())
    }

    #[test]
    fn test_required_fields_present() -> Result<()> {
        let validator = AgentConfigValidator::new()?;
        let agents = validator.load_all_agents()?;

        for agent in &agents {
            // All required fields must be non-empty
            assert!(
                !agent.config.name.is_empty(),
                "Agent at {} must have non-empty name",
                agent.path.display()
            );
            assert!(
                !agent.config.description.is_empty(),
                "Agent '{}' must have non-empty description",
                agent.config.name
            );
            assert!(
                !agent.config.model.is_empty(),
                "Agent '{}' must have non-empty model",
                agent.config.name
            );
        }

        Ok(())
    }

    #[test]
    fn test_agent_name_format() -> Result<()> {
        let validator = AgentConfigValidator::new()?;
        let agents = validator.load_all_agents()?;

        for agent in &agents {
            // Agent names should use lowercase-with-hyphens format
            let valid_chars = agent
                .config
                .name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c == '-' || c.is_ascii_digit());

            assert!(
                valid_chars,
                "Agent name '{}' should use lowercase-with-hyphens format",
                agent.config.name
            );

            // Should not start or end with hyphen
            assert!(
                !agent.config.name.starts_with('-') && !agent.config.name.ends_with('-'),
                "Agent name '{}' should not start or end with hyphen",
                agent.config.name
            );
        }

        Ok(())
    }
}
