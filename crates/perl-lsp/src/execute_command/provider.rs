use crate::perl_critic::{BuiltInAnalyzer, CriticAnalyzer, CriticConfig};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Execute command provider implementing the LSP executeCommand method.
pub struct ExecuteCommandProvider {
    workspace_roots: Vec<PathBuf>,
}

impl Default for ExecuteCommandProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecuteCommandProvider {
    /// Create a new execute command provider.
    pub fn new() -> Self {
        Self { workspace_roots: Vec::new() }
    }

    /// Create a provider with workspace root enforcement.
    pub fn with_workspace_roots(workspace_roots: Vec<PathBuf>) -> Self {
        Self { workspace_roots }
    }

    /// Execute a supported command with validated JSON arguments.
    pub fn execute_command(&self, command: &str, arguments: Vec<Value>) -> Result<Value, String> {
        match command {
            "perl.runTests" => {
                let file_path = self.resolve_path_from_args(&arguments)?;
                self.run_tests(&file_path)
            }
            "perl.runFile" => {
                let file_path = self.resolve_path_from_args(&arguments)?;
                self.run_file(&file_path)
            }
            "perl.runTestSub" => {
                let file_path = self.resolve_path_from_args(&arguments)?;
                let sub_name = arguments
                    .get(1)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing subroutine name argument".to_string())?;
                self.run_test_sub(&file_path, sub_name)
            }
            "perl.debugTests" => {
                let file_path = self.resolve_path_from_args(&arguments)?;
                self.debug_tests(&file_path)
            }
            "perl.runCritic" => self.run_critic_secure(&arguments),
            _ => Err(format!("Unknown command: {}", command)),
        }
    }

    pub(crate) fn run_tests(&self, file_path: &Path) -> Result<Value, String> {
        let file_path_str = file_path.to_string_lossy();
        let is_test_file = self.is_test_file(&file_path_str);
        let (command_name, mut cmd) = if is_test_file && self.command_exists("prove") {
            ("prove", {
                let mut cmd = Command::new("prove");
                cmd.arg("-v").arg("--").arg(file_path.as_os_str());
                cmd
            })
        } else {
            ("perl", {
                let mut cmd = Command::new("perl");
                cmd.arg("--").arg(file_path.as_os_str());
                cmd
            })
        };

        let result = cmd.output().map_err(|e| format!("Failed to run {}: {}", command_name, e))?;
        Ok(self.format_command_result(result, Some(("command", command_name.into()))))
    }

    pub(crate) fn run_test_sub(&self, file_path: &Path, sub_name: &str) -> Result<Value, String> {
        let perl_code = r#"
            my ($file, $sub) = @ARGV;
            do $file;
            if (defined &$sub) {
                no strict 'refs';
                &$sub();
            } else {
                die "Subroutine $sub not found";
            }
        "#;

        let result = Command::new("perl")
            .arg("-e")
            .arg(perl_code)
            .arg("--")
            .arg(file_path.as_os_str())
            .arg(sub_name)
            .output()
            .map_err(|e| format!("Failed to run test subroutine: {}", e))?;

        Ok(self.format_command_result(result, Some(("subroutine", sub_name.into()))))
    }

    pub(crate) fn run_file(&self, file_path: &Path) -> Result<Value, String> {
        let result = Command::new("perl")
            .arg("--")
            .arg(file_path.as_os_str())
            .output()
            .map_err(|e| format!("Failed to run file: {}", e))?;

        Ok(self.format_command_result(result, None))
    }

    fn debug_tests(&self, file_path: &Path) -> Result<Value, String> {
        let file_path_str = file_path.to_string_lossy();
        Ok(json!({
            "success": false,
            "output": format!("Debug mode not yet implemented for {}", file_path_str),
            "error": Some("Debugging support coming soon".to_string())
        }))
    }

    fn run_critic_secure(&self, arguments: &[Value]) -> Result<Value, String> {
        let canonical_path = match self.resolve_path_from_args(arguments) {
            Ok(path) => path,
            Err(e) => {
                if e.contains("Missing file path argument") {
                    return Err(e);
                }

                if e.contains("File not found")
                    || e.contains("does not exist")
                    || e.contains("No such file or directory")
                    || e.contains("Failed to canonicalize")
                {
                    let error_message = if e.contains("Failed to canonicalize") {
                        if let Some(start) = e.find('\'') {
                            if let Some(end) = e[start + 1..].find('\'') {
                                let path = &e[start + 1..start + 1 + end];
                                format!("File not found: {}", path)
                            } else {
                                "File not found".to_string()
                            }
                        } else {
                            "File not found".to_string()
                        }
                    } else {
                        e.clone()
                    };
                    return Ok(self.format_critic_error(error_message, "none"));
                }

                if e.contains("Path traversal")
                    || e.contains("outside workspace")
                    || e.contains("Argument too long")
                {
                    return Err(format!("Path resolution failed: {}", e));
                }

                return Ok(self.format_critic_error(e, "none"));
            }
        };

        if command_exists("perlcritic") {
            if let Ok(result) = self.run_external_critic(&canonical_path) {
                return Ok(result);
            }
        }

        self.run_builtin_critic(&canonical_path)
    }

    #[deprecated(since = "0.8.9", note = "Use run_critic_secure for secure path resolution")]
    #[allow(dead_code)]
    #[allow(deprecated)]
    pub(crate) fn run_critic(&self, file_path: &str) -> Result<Value, String> {
        let normalized_path = self.normalize_file_path(file_path);
        let path = Path::new(normalized_path);

        if !path.exists() {
            return Ok(
                self.format_critic_error(format!("File not found: {}", normalized_path), "none")
            );
        }

        if command_exists("perlcritic") {
            if let Ok(result) = self.run_external_critic(path) {
                return Ok(result);
            }
        }

        self.run_builtin_critic(path)
    }

    fn run_external_critic(&self, file_path: &Path) -> Result<Value, String> {
        let config = CriticConfig { severity: 3, verbose: true, ..Default::default() };
        let mut analyzer = CriticAnalyzer::with_os_runtime(config);

        match analyzer.analyze_file(file_path) {
            Ok(violations) => {
                let formatted_violations: Vec<_> = violations
                    .iter()
                    .map(|v| {
                        self.format_violation(
                            &v.policy,
                            &v.description,
                            &v.explanation,
                            v.severity as u8,
                            (v.range.start.line + 1) as usize,
                            (v.range.start.column + 1) as usize,
                            &v.file,
                        )
                    })
                    .collect();

                Ok(json!({
                    "status": "success",
                    "violations": formatted_violations,
                    "violationCount": formatted_violations.len(),
                    "analyzerUsed": "external"
                }))
            }
            Err(e) => Err(format!("External perlcritic failed: {}", e)),
        }
    }

    pub(crate) fn run_builtin_critic(&self, file_path: &Path) -> Result<Value, String> {
        use crate::Parser;

        let content = std::fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        let code_text = perl_parser::util::code_slice(&content);
        let mut parser = Parser::new(code_text);

        let (ast, parse_error) = match parser.parse() {
            Ok(ast) => (ast, None),
            Err(error) => {
                let message = error.to_string();
                (
                    crate::ast::Node::new(
                        crate::ast::NodeKind::Error {
                            message,
                            expected: vec![],
                            found: None,
                            partial: None,
                        },
                        crate::ast::SourceLocation { start: 0, end: code_text.len() },
                    ),
                    Some(error),
                )
            }
        };

        let analyzer = BuiltInAnalyzer::new();
        let mut all_violations = analyzer.analyze(&ast, code_text);
        if let Some(error) = parse_error {
            all_violations.push(self.create_syntax_error_violation(&error, code_text, file_path));
        }

        let formatted_violations: Vec<_> = all_violations
            .iter()
            .map(|v| {
                self.format_violation(
                    &v.policy,
                    &v.description,
                    &v.explanation,
                    v.severity as u8,
                    (v.range.start.line + 1) as usize,
                    (v.range.start.column + 1) as usize,
                    &file_path.to_string_lossy(),
                )
            })
            .collect();

        Ok(json!({
            "status": "success",
            "violations": formatted_violations,
            "violationCount": formatted_violations.len(),
            "analyzerUsed": "builtin"
        }))
    }

    pub(crate) fn is_test_file(&self, file_path: &str) -> bool {
        file_path.ends_with(".t") || file_path.contains("/t/") || file_path.contains("test")
    }

    pub(crate) fn format_command_result(
        &self,
        result: std::process::Output,
        extra_field: Option<(&str, Value)>,
    ) -> Value {
        let output = String::from_utf8_lossy(&result.stdout);
        let error = if !result.status.success() {
            Some(String::from_utf8_lossy(&result.stderr).to_string())
        } else {
            None
        };

        let mut response = json!({
            "success": result.status.success(),
            "output": output.to_string(),
            "error": error
        });

        if let Some((key, value)) = extra_field {
            response[key] = value;
        }

        response
    }

    fn resolve_path_from_args(&self, arguments: &[Value]) -> Result<PathBuf, String> {
        let raw_path = arguments
            .first()
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing file path argument".to_string())?;

        const MAX_ARG_LENGTH: usize = 4096;
        if raw_path.len() > MAX_ARG_LENGTH {
            return Err(format!(
                "Argument too long ({} bytes, max {})",
                raw_path.len(),
                MAX_ARG_LENGTH
            ));
        }

        let normalized_path = raw_path.strip_prefix("file://").unwrap_or(raw_path);
        if normalized_path.contains("..") {
            return Err("Path traversal attempt detected: path contains '..' component".to_string());
        }

        let path = Path::new(normalized_path);
        let canonical_path = path
            .canonicalize()
            .map_err(|e| format!("Failed to canonicalize path '{}': {}", normalized_path, e))?;

        let effective_roots: Vec<PathBuf> = if self.workspace_roots.is_empty() {
            match std::env::current_dir() {
                Ok(cwd) => vec![cwd],
                Err(_) => {
                    return Err(
                        "No workspace roots configured and cannot determine working directory"
                            .to_string(),
                    );
                }
            }
        } else {
            self.workspace_roots.clone()
        };

        let allowed = effective_roots.iter().any(|workspace_root| {
            workspace_root
                .canonicalize()
                .map(|canonical_root| canonical_path.starts_with(&canonical_root))
                .unwrap_or(false)
        });

        if !allowed {
            return Err(format!(
                "Path traversal detected: {} is outside workspace boundaries",
                canonical_path.display()
            ));
        }

        if !canonical_path.exists() {
            return Err(format!("File not found: {}", canonical_path.display()));
        }

        if !canonical_path.is_file() {
            return Err(format!("Path is not a file: {}", canonical_path.display()));
        }

        std::fs::metadata(&canonical_path).map_err(|e| {
            format!("Cannot read file metadata '{}': {}", canonical_path.display(), e)
        })?;

        Ok(canonical_path)
    }

    /// Resolve a debug file path using the same workspace security checks.
    pub fn resolve_debug_file_path(&self, file_path: &str) -> Result<PathBuf, String> {
        self.resolve_path_from_args(&[Value::String(file_path.to_string())])
    }

    #[deprecated(since = "0.8.9", note = "Use resolve_path_from_args for secure path resolution")]
    #[allow(dead_code)]
    pub(crate) fn normalize_file_path<'a>(&self, file_path: &'a str) -> &'a str {
        file_path.strip_prefix("file://").unwrap_or(file_path)
    }

    pub(crate) fn format_violation(
        &self,
        policy: &str,
        description: &str,
        explanation: &str,
        severity: u8,
        line: usize,
        column: usize,
        file: &str,
    ) -> Value {
        json!({
            "policy": policy,
            "description": description,
            "explanation": explanation,
            "severity": severity,
            "line": line,
            "column": column,
            "file": file
        })
    }

    pub(crate) fn format_critic_error(&self, error_message: String, analyzer_used: &str) -> Value {
        json!({
            "status": "error",
            "error": error_message,
            "violations": [],
            "violationCount": 0,
            "analyzerUsed": analyzer_used
        })
    }

    fn create_syntax_error_violation(
        &self,
        error: &perl_parser::ParseError,
        _content: &str,
        file_path: &Path,
    ) -> crate::perl_critic::Violation {
        let error_msg = format!("{}", error);
        let (line, column) = (0, 0);

        crate::perl_critic::Violation {
            policy: "Syntax::ParseError".to_string(),
            description: format!("Syntax error: {}", error_msg),
            explanation: "This code contains a syntax error that prevents parsing. Fix the syntax error before running additional checks.".to_string(),
            severity: crate::perl_critic::Severity::Brutal,
            range: crate::position::Range {
                start: crate::position::Position { byte: 0, line: line as u32, column: column as u32 },
                end: crate::position::Position { byte: 1, line: line as u32, column: (column + 1) as u32 },
            },
            file: file_path.to_string_lossy().to_string(),
        }
    }

    pub(crate) fn command_exists(&self, command: &str) -> bool {
        Command::new("which")
            .arg(command)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}

/// Check whether a command exists in the current PATH.
pub fn command_exists(command: &str) -> bool {
    std::process::Command::new(command)
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Return the supported executeCommand identifiers.
pub fn get_supported_commands() -> Vec<String> {
    crate::protocol::capabilities::get_supported_commands()
}
