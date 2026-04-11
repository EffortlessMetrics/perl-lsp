use crate::perl_critic::{BuiltInAnalyzer, CriticAnalyzer, CriticConfig};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Strip the Windows extended-length path prefix (`\\?\`) before passing a path
/// to an external command such as `perl`, `prove`, or `yath`.
///
/// On Windows, `Path::canonicalize` returns paths prefixed with `\\?\`, which is
/// understood by Win32 APIs but not by external programs (e.g. `perl.exe`).  This
/// helper strips that prefix so the resulting path is usable as a command-line
/// argument.  On non-Windows platforms the function is a no-op identity.
///
/// Two prefix forms are handled:
/// - `\\?\C:\...`         (local drive) → `C:\...`
/// - `\\?\UNC\server\...` (network UNC) → `\\server\...`
///
/// The UNC form requires special treatment: stripping `\\?\` alone would leave
/// `UNC\server\...` which is not a valid path.  Instead we replace `\\?\UNC\`
/// with `\\` so the result is a conventional UNC path (`\\server\share\...`).
pub(crate) fn normalize_path_for_external_command(path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        let s = path.to_string_lossy();
        // Network UNC extended-length paths: \\?\UNC\server\share\...
        // Must become \\server\share\... (not UNC\server\share\...)
        if let Some(unc_rest) = s.strip_prefix(r"\\?\UNC\") {
            return PathBuf::from(format!(r"\\{}", unc_rest));
        }
        // Local drive extended-length paths: \\?\C:\... → C:\...
        if let Some(stripped) = s.strip_prefix(r"\\?\") {
            return PathBuf::from(stripped.to_string());
        }
    }
    path.to_path_buf()
}

/// Execute command provider implementing the LSP executeCommand method.
pub struct ExecuteCommandProvider {
    workspace_roots: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TestRunner {
    Yath,
    Prove,
    Perl,
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
            "perl.debugTests" | "perl.debugFile" | "perl.debugTest" => {
                let file_path = self.resolve_path_from_args(&arguments)?;
                self.debug_tests(&file_path)
            }
            "perl.runTest" | "perl.runTestFile" => {
                let file_path = self.resolve_path_from_args(&arguments)?;
                self.run_tests(&file_path)
            }
            "perl.runSubtest" => {
                let file_path = self.resolve_path_from_args(&arguments)?;
                let sub_name = arguments
                    .get(1)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing subroutine name argument".to_string())?;
                self.run_test_sub(&file_path, sub_name)
            }
            "perl.runCritic" => self.run_critic_secure(&arguments),
            "perl.goToTest" => {
                let file_path = self.resolve_path_from_args(&arguments)?;
                Ok(self.go_to_test(&file_path))
            }
            "perl.goToImplementation" => {
                let file_path = self.resolve_path_from_args(&arguments)?;
                Ok(self.go_to_implementation(&file_path))
            }
            _ => Err(format!("Unknown command: {}", command)),
        }
    }

    pub(crate) fn run_tests(&self, file_path: &Path) -> Result<Value, String> {
        let file_path_str = file_path.to_string_lossy();
        let is_test_file = self.is_test_file(&file_path_str);
        let runner = select_test_runner(
            is_test_file,
            self.command_exists("yath"),
            self.command_exists("prove"),
        );
        let ext_path = normalize_path_for_external_command(file_path);

        match runner {
            TestRunner::Yath => {
                let mut yath_cmd = Command::new("yath");
                yath_cmd.arg("-v").arg("--").arg(ext_path.as_os_str());
                match crate::util::run_command_with_timeout(yath_cmd, 30) {
                    Ok(result) => {
                        Ok(self.format_command_result(result, Some(("command", "yath".into()))))
                    }
                    Err(error) => {
                        if self.command_exists("prove") {
                            let mut prove_cmd = Command::new("prove");
                            prove_cmd.arg("-v").arg("--").arg(ext_path.as_os_str());
                            match crate::util::run_command_with_timeout(prove_cmd, 30) {
                                Ok(result) => Ok(self.format_command_result(
                                    result,
                                    Some(("command", "prove".into())),
                                )),
                                Err(fallback_error) => {
                                    let mut perl_cmd = Command::new("perl");
                                    perl_cmd.arg("--").arg(ext_path.as_os_str());
                                    match crate::util::run_command_with_timeout(perl_cmd, 30) {
                                        Ok(result) => Ok(self.format_command_result(
                                            result,
                                            Some(("command", "perl".into())),
                                        )),
                                        Err(perl_error) => Ok(self.format_command_launch_failure(
                                            "yath",
                                            format!(
                                                "Failed to run yath: {error}; prove fallback also failed: {fallback_error}; perl fallback also failed: {perl_error}"
                                            ),
                                        )),
                                    }
                                }
                            }
                        } else {
                            let mut perl_cmd = Command::new("perl");
                            perl_cmd.arg("--").arg(ext_path.as_os_str());
                            match crate::util::run_command_with_timeout(perl_cmd, 30) {
                                Ok(result) => Ok(self
                                    .format_command_result(result, Some(("command", "perl".into())))),
                                Err(perl_error) => Ok(self.format_command_launch_failure(
                                    "yath",
                                    format!(
                                        "Failed to run yath: {error}; perl fallback also failed: {perl_error}"
                                    ),
                                )),
                            }
                        }
                    }
                }
            }
            TestRunner::Prove => {
                let mut prove_cmd = Command::new("prove");
                prove_cmd.arg("-v").arg("--").arg(ext_path.as_os_str());
                match crate::util::run_command_with_timeout(prove_cmd, 30) {
                    Ok(result) => {
                        Ok(self.format_command_result(result, Some(("command", "prove".into()))))
                    }
                    Err(error) => {
                        let mut perl_cmd = Command::new("perl");
                        perl_cmd.arg("--").arg(ext_path.as_os_str());
                        match crate::util::run_command_with_timeout(perl_cmd, 30) {
                            Ok(result) => Ok(self
                                .format_command_result(result, Some(("command", "perl".into())))),
                            Err(fallback_error) => Ok(self.format_command_launch_failure(
                                "prove",
                                format!(
                                    "Failed to run prove: {error}; perl fallback also failed: {fallback_error}"
                                ),
                            )),
                        }
                    }
                }
            }
            TestRunner::Perl => {
                let mut perl_cmd = Command::new("perl");
                perl_cmd.arg("--").arg(ext_path.as_os_str());
                match crate::util::run_command_with_timeout(perl_cmd, 30) {
                    Ok(result) => {
                        Ok(self.format_command_result(result, Some(("command", "perl".into()))))
                    }
                    Err(error) => Ok(self.format_command_launch_failure(
                        "perl",
                        format!("Failed to run perl: {error}"),
                    )),
                }
            }
        }
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
        let ext_path = normalize_path_for_external_command(file_path);

        let mut perl_cmd = Command::new("perl");
        perl_cmd.arg("-e").arg(perl_code).arg("--").arg(ext_path.as_os_str()).arg(sub_name);
        match crate::util::run_command_with_timeout(perl_cmd, 30) {
            Ok(result) => {
                Ok(self.format_command_result(result, Some(("subroutine", sub_name.into()))))
            }
            Err(error) => Ok(self.format_command_launch_failure(
                "perl",
                format!("Failed to run test subroutine: {error}"),
            )),
        }
    }

    pub(crate) fn run_file(&self, file_path: &Path) -> Result<Value, String> {
        let ext_path = normalize_path_for_external_command(file_path);
        let mut perl_cmd = Command::new("perl");
        perl_cmd.arg("--").arg(ext_path.as_os_str());
        match crate::util::run_command_with_timeout(perl_cmd, 30) {
            Ok(result) => Ok(self.format_command_result(result, None)),
            Err(error) => {
                Ok(self
                    .format_command_launch_failure("perl", format!("Failed to run file: {error}")))
            }
        }
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

    /// Convert a Perl module name to a test file stem.
    ///
    /// `Foo::Bar` -> `foo-bar` (canonical hyphen form used by many CPAN distributions)
    pub fn module_to_test_stem(&self, module_name: &str) -> String {
        module_name.replace("::", "-").to_lowercase()
    }

    /// Infer a module name from a `lib/` path component.
    ///
    /// `/path/to/lib/Foo/Bar.pm` -> `Foo::Bar`
    fn pm_path_to_module(&self, pm_path: &std::path::Path) -> Option<String> {
        // Walk up from the file to find the `lib` directory anchor.
        let components: Vec<_> = pm_path.components().collect();
        let lib_pos = components.iter().rposition(|c| c.as_os_str() == "lib")?;
        let after_lib: Vec<_> = components[lib_pos + 1..].to_vec();
        if after_lib.is_empty() {
            return None;
        }
        let mut parts = Vec::new();
        for c in &after_lib {
            let s = c.as_os_str().to_string_lossy();
            let part = if s.ends_with(".pm") {
                s.trim_end_matches(".pm").to_string()
            } else {
                s.to_string()
            };
            parts.push(part);
        }
        Some(parts.join("::"))
    }

    /// Navigate from a `.pm` implementation file to its companion test file.
    ///
    /// Probes (in order):
    ///   1. `t/<stem>.t`  where stem is the hyphen-lowercased module name
    ///   2. `t/<stem>.t`  where stem uses underscores instead of hyphens
    ///   3. `t/<leaf>.t`  where leaf is just the last module component lowercased
    ///   4. `t/lib/<Foo/Bar>.t`  where the relative path mirrors the module hierarchy
    pub(crate) fn go_to_test(&self, pm_path: &std::path::Path) -> Value {
        let module_name = match self.pm_path_to_module(pm_path) {
            Some(m) => m,
            None => {
                return json!({ "found": false, "candidates": [] });
            }
        };

        // Find workspace root: walk up until we find a `lib` or `t` sibling.
        let workspace_root = match self.find_workspace_root(pm_path) {
            Some(r) => r,
            None => {
                return json!({ "found": false, "candidates": [] });
            }
        };

        let t_dir = workspace_root.join("t");
        let stem_hyphen = self.module_to_test_stem(&module_name);
        let stem_underscore = stem_hyphen.replace('-', "_");
        // Leaf component without unwrap: split always produces at least one element.
        let leaf = match module_name.rsplit_once("::") {
            Some((_, last)) => last.to_lowercase(),
            None => module_name.to_lowercase(),
        };
        // Mirror path under t/lib/ (e.g. Foo::Bar::Baz -> t/lib/Foo/Bar/Baz.t)
        let mirror_rel = module_name.replace("::", std::path::MAIN_SEPARATOR_STR) + ".t";

        let candidates = [
            t_dir.join(format!("{stem_hyphen}.t")),
            t_dir.join(format!("{stem_underscore}.t")),
            t_dir.join(format!("{leaf}.t")),
            t_dir.join("lib").join(&mirror_rel),
        ];

        for candidate in &candidates {
            if candidate.exists() {
                return json!({
                    "found": true,
                    "path": candidate.to_string_lossy(),
                    "module": module_name,
                });
            }
        }

        let candidate_strings: Vec<_> =
            candidates.iter().map(|p| p.to_string_lossy().to_string()).collect();
        json!({ "found": false, "candidates": candidate_strings })
    }

    /// Navigate from a test file to the first local module it uses.
    ///
    /// Scans the test file for `use Foo::Bar;` statements (skipping well-known
    /// CPAN pragmas and test modules), then maps the first match to
    /// `lib/Foo/Bar.pm` relative to the workspace root.
    pub(crate) fn go_to_implementation(&self, test_path: &std::path::Path) -> Value {
        let content = match std::fs::read_to_string(test_path) {
            Ok(c) => c,
            Err(_) => return json!({ "found": false }),
        };

        let workspace_root = match self.find_workspace_root(test_path) {
            Some(r) => r,
            None => return json!({ "found": false }),
        };

        // Well-known modules that are NOT local implementations (exact matches).
        const SKIP_MODULES: &[&str] = &[
            // Core pragmas
            "strict",
            "warnings",
            "utf8",
            "feature",
            "parent",
            "base",
            "vars",
            "constant",
            "overload",
            "ok",
            // Core modules
            "Carp",
            "Exporter",
            "Scalar::Util",
            "List::Util",
            "Hash::Util",
            "POSIX",
            "Data::Dumper",
            "Storable",
            "Encode",
            "Cwd",
            "FindBin",
            "File::Basename",
            "File::Path",
            "File::Spec",
            "File::Find",
            "File::Temp",
            "File::Copy",
            "IO::File",
            "IO::Handle",
            "IO::Select",
            "Getopt::Long",
            "Getopt::Std",
            // OO frameworks
            "Moo",
            "Moose",
            "Mouse",
            // Test modules (exact)
            "Test::More",
            "Test::Simple",
            "Test::Builder",
            "Test::Deep",
            "Test::Exception",
            "Test::Warn",
            "Test::Fatal",
            "Test::MockObject",
            "Test::MockModule",
            "Test::Output",
            "Test::Differences",
            "Test::Class",
            "Test::Pod",
            "Test::Pod::Coverage",
            "Try::Tiny",
        ];

        // Module name-space prefixes whose entire hierarchy is non-local.
        // Any `use` statement whose module starts with one of these prefixes
        // will be skipped without needing every sub-module listed.
        const SKIP_PREFIXES: &[&str] = &[
            "Test2::",  // Test2::V0, Test2::Bundle::*, Test2::Tools::*
            "MooseX::", // MooseX::Types, MooseX::Declare, etc.
            "MouseX::",
            "Moo::Role",
            "Moose::Role",
            "Types::",     // Types::Standard, Types::Path::Tiny, etc.
            "namespace::", // namespace::autoclean, namespace::clean
            "Sub::",       // Sub::Exporter, Sub::Quote, etc.
            "Class::MOP",
            "DBIx::", // DBIx::Class, DBIx::Connector
            "DBI",
            "LWP::",
            "HTTP::",
            "URI::",
            "JSON::",
            "YAML::",
            "XML::",
            "DateTime::",
            "Path::Tiny",
            "Path::Class",
        ];

        for line in content.lines() {
            let trimmed = line.trim();
            // Match `use Module::Name;` or `use Module::Name qw(...);`
            if !trimmed.starts_with("use ") {
                continue;
            }
            let after_use = trimmed.trim_start_matches("use ").trim();
            // Extract the module name (stop at first whitespace or semicolon)
            let module_name: String =
                after_use.chars().take_while(|c| c.is_alphanumeric() || *c == ':').collect();

            if module_name.is_empty() {
                continue;
            }
            // Skip version-only pragmas like `use 5.010;` or `use v5.10;`
            if module_name.chars().next().is_some_and(|c| c.is_ascii_digit() || c == 'v') {
                continue;
            }
            if SKIP_MODULES.contains(&module_name.as_str()) {
                continue;
            }
            if SKIP_PREFIXES
                .iter()
                .any(|p| module_name.starts_with(p) || module_name == p.trim_end_matches("::"))
            {
                continue;
            }

            // Map Foo::Bar -> lib/Foo/Bar.pm
            let rel_path = module_name.replace("::", std::path::MAIN_SEPARATOR_STR) + ".pm";
            let candidate = workspace_root.join("lib").join(&rel_path);
            if candidate.exists() {
                return json!({
                    "found": true,
                    "path": candidate.to_string_lossy(),
                    "module": module_name,
                });
            }
        }

        json!({ "found": false })
    }

    /// Find the workspace root by walking up from `path`.
    ///
    /// Preference order:
    ///   1. Explicit workspace roots registered with the provider (normal LSP runtime path).
    ///   2. Nearest ancestor that contains a Perl project marker (`Makefile.PL`,
    ///      `Build.PL`, `cpanfile`, `dist.ini`, `META.json`, `META.yml`, `.git`).
    ///   3. Nearest ancestor that contains either a `lib/` or `t/` child directory.
    ///
    /// This multi-tier strategy avoids accidentally picking up a distant ancestor
    /// that happens to have a `lib/` or `t/` directory unrelated to the current project.
    fn find_workspace_root(&self, path: &std::path::Path) -> Option<std::path::PathBuf> {
        // Tier 1: explicit workspace roots registered with the provider.
        if !self.workspace_roots.is_empty() {
            let canonical_path = path.canonicalize().map_err(|e| {
                tracing::debug!(path = %path.display(), error = %e, "workspace root: failed to canonicalize path");
            }).ok();
            for root in &self.workspace_roots {
                let Ok(canonical_root) = root.canonicalize() else { continue };
                if canonical_path.as_ref().is_some_and(|p| p.starts_with(&canonical_root)) {
                    return Some(root.clone());
                }
            }
        }

        // Perl distribution marker files that indicate a project root.
        const PROJECT_MARKERS: &[&str] =
            &["Makefile.PL", "Build.PL", "cpanfile", "dist.ini", "META.json", "META.yml", ".git"];

        let mut current = path.parent()?;

        // Tier 2: walk up looking for a Perl project marker first.
        let mut tier3_candidate: Option<std::path::PathBuf> = None;
        loop {
            // Check for definitive project markers.
            if PROJECT_MARKERS.iter().any(|m| current.join(m).exists()) {
                return Some(current.to_path_buf());
            }
            // Remember the first ancestor with lib/ or t/ for tier-3 fallback.
            if tier3_candidate.is_none()
                && (current.join("lib").is_dir() || current.join("t").is_dir())
            {
                tier3_candidate = Some(current.to_path_buf());
            }
            current = match current.parent() {
                Some(p) => p,
                None => break,
            };
        }

        // Tier 3: fall back to the nearest lib/t ancestor found above.
        tier3_candidate
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

    fn format_command_launch_failure(&self, command: &str, error: String) -> Value {
        json!({
            "success": false,
            "output": String::new(),
            "error": error,
            "command": command
        })
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

        let path = if raw_path.starts_with("file://") {
            crate::workspace_index::uri_to_fs_path(raw_path)
                .ok_or_else(|| format!("Failed to parse file URI: {raw_path}"))?
        } else {
            PathBuf::from(raw_path)
        };
        let normalized_path = path.to_string_lossy();
        if normalized_path.contains("..") {
            return Err("Path traversal attempt detected: path contains '..' component".to_string());
        }

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
        let cmd = if cfg!(windows) {
            let mut cmd = Command::new("where");
            cmd.arg(command);
            cmd
        } else {
            let mut cmd = Command::new("which");
            cmd.arg(command);
            cmd
        };
        crate::util::run_command_with_timeout(cmd, 2)
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}

pub(crate) fn select_test_runner(
    is_test_file: bool,
    yath_available: bool,
    prove_available: bool,
) -> TestRunner {
    if !is_test_file {
        TestRunner::Perl
    } else if yath_available {
        TestRunner::Yath
    } else if prove_available {
        TestRunner::Prove
    } else {
        TestRunner::Perl
    }
}

/// Check whether a command exists in the current PATH.
pub fn command_exists(command: &str) -> bool {
    let cmd = if cfg!(windows) {
        let mut cmd = std::process::Command::new("where");
        cmd.arg(command);
        cmd
    } else {
        let mut cmd = std::process::Command::new(command);
        cmd.arg("--version");
        cmd
    };
    crate::util::run_command_with_timeout(cmd, 2)
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Return the supported executeCommand identifiers.
pub fn get_supported_commands() -> Vec<String> {
    // Keep in sync with perl_lsp_protocol::capabilities::get_supported_commands
    vec![
        "perl.runTests".to_string(),
        "perl.runFile".to_string(),
        "perl.runTestSub".to_string(),
        "perl.runCritic".to_string(),
        "perl.runTest".to_string(),
        "perl.runTestFile".to_string(),
        "perl.runSubtest".to_string(),
        "perl.debugFile".to_string(),
        "perl.debugTest".to_string(),
        "perl.goToTest".to_string(),
        "perl.goToImplementation".to_string(),
    ]
}

#[cfg(test)]
mod normalize_path_tests {
    use super::normalize_path_for_external_command;
    use std::path::{Path, PathBuf};

    /// On Windows the `\\?\` extended-length prefix must be stripped so that
    /// external commands (perl.exe, prove, yath) can accept the path.
    #[test]
    #[cfg(target_os = "windows")]
    fn strips_extended_length_prefix_on_windows() {
        let prefixed = Path::new(r"\\?\C:\Users\test\file.pl");
        let result = normalize_path_for_external_command(prefixed);
        assert_eq!(
            result,
            PathBuf::from(r"C:\Users\test\file.pl"),
            "Extended-length prefix should be stripped: got {:?}",
            result
        );
    }

    /// On Windows, paths without the prefix are returned unchanged.
    #[test]
    #[cfg(target_os = "windows")]
    fn passthrough_plain_windows_path() {
        let plain = Path::new(r"C:\Users\test\file.pl");
        let result = normalize_path_for_external_command(plain);
        assert_eq!(result, PathBuf::from(r"C:\Users\test\file.pl"));
    }

    /// On non-Windows, the helper is a pass-through identity — paths are
    /// returned exactly as given regardless of content.
    #[test]
    #[cfg(not(target_os = "windows"))]
    fn passthrough_on_non_windows() {
        let path = Path::new("/tmp/test_valid.pl");
        let result = normalize_path_for_external_command(path);
        assert_eq!(result, PathBuf::from("/tmp/test_valid.pl"));
    }

    /// Verify the helper handles a synthetic Windows extended-length prefix
    /// as a string: even on non-Windows the conditional compilation means the
    /// prefix is left untouched (since there is no `\\?\` on Unix paths).
    /// This test documents the cross-platform contract.
    #[test]
    #[cfg(not(target_os = "windows"))]
    fn no_stripping_on_non_windows_even_for_unc_like_string() {
        // On Linux/macOS this is just a literal path string — no stripping.
        let path = Path::new(r"\\?\C:\foo\bar");
        let result = normalize_path_for_external_command(path);
        assert_eq!(result, PathBuf::from(r"\\?\C:\foo\bar"));
    }

    /// On Windows, the UNC extended-length form `\\?\UNC\server\share\...` must
    /// become `\\server\share\...` — NOT `UNC\server\share\...`.
    ///
    /// `Path::canonicalize` on Windows returns `\\?\UNC\...` for network paths.
    /// Simply stripping `\\?\` would leave `UNC\server\share\...` which perl.exe
    /// cannot resolve.  The correct result is a plain UNC path `\\server\share\...`.
    #[test]
    #[cfg(target_os = "windows")]
    fn strips_extended_length_unc_prefix_on_windows() {
        let prefixed = Path::new(r"\\?\UNC\fileserver\share\project\test.pl");
        let result = normalize_path_for_external_command(prefixed);
        assert_eq!(
            result,
            PathBuf::from(r"\\fileserver\share\project\test.pl"),
            "UNC extended-length prefix should become plain UNC path: got {:?}",
            result
        );
    }

    /// On non-Windows, a UNC extended-length string is also left untouched —
    /// the conditional compilation means the entire stripping block is absent.
    #[test]
    #[cfg(not(target_os = "windows"))]
    fn no_stripping_on_non_windows_even_for_unc_extended_string() {
        let path = Path::new(r"\\?\UNC\fileserver\share\project\test.pl");
        let result = normalize_path_for_external_command(path);
        assert_eq!(result, PathBuf::from(r"\\?\UNC\fileserver\share\project\test.pl"));
    }
}
