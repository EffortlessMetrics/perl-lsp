//! Virtual document content support for LSP 3.18
//!
//! Provides support for workspace/textDocumentContent to serve virtual documents
//! like perldoc:// URIs for Perl documentation.

use super::super::*;
use crate::documentation_targets::PerlDocumentationTarget;
#[cfg(not(target_arch = "wasm32"))]
use perl_lsp_rs_core::config::PerlOracleEnv;
use perl_lsp_rs_core::config::WorkspaceConfig;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

impl LspServer {
    /// Handle workspace/textDocumentContent request
    pub(crate) fn handle_text_document_content(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let params = params.ok_or_else(|| JsonRpcError {
            code: crate::protocol::INVALID_PARAMS,
            message: "Missing params".to_string(),
            data: None,
        })?;

        let uri = params.get("uri").and_then(|u| u.as_str()).ok_or_else(|| JsonRpcError {
            code: crate::protocol::INVALID_PARAMS,
            message: "Missing or invalid URI".to_string(),
            data: None,
        })?;

        if !is_valid_virtual_content_uri(uri) {
            return Err(JsonRpcError {
                code: crate::protocol::INVALID_PARAMS,
                message: "Missing or invalid URI".to_string(),
                data: None,
            });
        }

        if let Some(content) = self.fetch_virtual_content(uri) {
            Ok(Some(json!({ "text": content })))
        } else {
            Err(JsonRpcError {
                code: -32600,
                message: format!("Unsupported URI scheme or content not found: {}", uri),
                data: None,
            })
        }
    }

    /// Request client to refresh virtual document content
    pub fn request_text_document_content_refresh(&self, uri: &str) -> io::Result<()> {
        self.send_request("workspace/textDocumentContent/refresh", json!({ "uri": uri }))
            .map(|_| ())
    }
}

fn is_valid_virtual_content_uri(uri: &str) -> bool {
    let Some((scheme, rest)) = uri.split_once("://") else {
        return false;
    };

    if scheme == "perldoc" && PerlDocumentationTarget::from_perldoc_uri(uri).is_none() {
        return false;
    }

    !scheme.is_empty()
        && !rest.is_empty()
        && scheme.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '-' | '.'))
        && !uri.chars().any(char::is_whitespace)
}

/// Fetch content for a virtual URI
impl LspServer {
    fn fetch_virtual_content(&self, uri: &str) -> Option<String> {
        if let Some(target) = PerlDocumentationTarget::from_perldoc_uri(uri) {
            self.fetch_workspace_perldoc(&target)
                .or_else(|| {
                    let workspace_config = self.workspace_config.lock().clone();
                    fetch_perldoc(target.name(), &workspace_config)
                })
                .map(|content| enrich_core_pragma_perldoc(target.name(), content))
        } else {
            None
        }
    }

    fn fetch_workspace_perldoc(&self, target: &PerlDocumentationTarget) -> Option<String> {
        if self.root_path.lock().is_none() && self.workspace_folders.lock().is_empty() {
            return None;
        }

        let module_name = target.name();
        let path = self.resolve_module_path(module_name, None)?;
        let source = match fs::read_to_string(&path) {
            Ok(source) => source,
            Err(error) => {
                tracing::warn!(module = module_name, path = %path.display(), %error, "Failed to read local POD");
                return None;
            }
        };
        let pod = perl_pod::extract_pod(&source);
        let related_links = workspace_pod_related_perldoc_uris(module_name, &source);

        format_workspace_pod_virtual_content(
            module_name,
            target.section(),
            &path,
            &pod,
            &related_links,
        )
    }
}

fn enrich_core_pragma_perldoc(module_name: &str, content: String) -> String {
    let related_name = match module_name {
        "strict" => "warnings",
        "warnings" => "strict",
        _ => return content,
    };
    let Some(related_target) = PerlDocumentationTarget::new(related_name) else {
        return content;
    };
    let related_uri = related_target.perldoc_uri();

    format!("Related virtual perldoc:\n- {related_uri}\n\n{content}")
}

fn format_workspace_pod_virtual_content(
    module_name: &str,
    section: Option<&str>,
    path: &Path,
    pod: &perl_pod::PodDoc,
    related_links: &[String],
) -> Option<String> {
    if pod.is_empty() {
        return None;
    }

    let mut header =
        format!("Workspace virtual perldoc\nModule: {module_name}\nSource: {}", path.display());
    if let Some(section) = section {
        header.push_str(&format!("\nSection: {section}"));
    }
    let mut sections = vec![header];

    if !related_links.is_empty() {
        let links =
            related_links.iter().map(|uri| format!("- {uri}")).collect::<Vec<_>>().join("\n");
        sections.push(format!("Related virtual perldoc:\n{links}"));
    }

    if let Some(name) = &pod.name {
        sections.push(format!("NAME\n{name}"));
    }
    if let Some(synopsis) = &pod.synopsis {
        sections.push(format!("SYNOPSIS\n{synopsis}"));
    }
    if let Some(description) = &pod.description {
        sections.push(format!("DESCRIPTION\n{description}"));
    }

    let mut method_names: Vec<&String> = pod.methods.keys().collect();
    method_names.sort();
    for method_name in method_names {
        if let Some(method_doc) = pod.methods.get(method_name) {
            sections.push(format!("METHOD {method_name}\n{method_doc}"));
        }
    }

    Some(sections.join("\n\n"))
}

fn workspace_pod_related_perldoc_uris(module_name: &str, source: &str) -> Vec<String> {
    let mut uris = BTreeSet::new();
    let mut in_pod = false;

    for line in source.lines() {
        if starts_pod_block(line) {
            in_pod = true;
        }

        if !in_pod {
            continue;
        }

        if line.starts_with("=cut") {
            in_pod = false;
            continue;
        }

        collect_simple_pod_module_links(line, module_name, &mut uris);
    }

    uris.into_iter().collect()
}

fn starts_pod_block(line: &str) -> bool {
    line.starts_with("=head")
        || line.starts_with("=pod")
        || line.starts_with("=over")
        || line.starts_with("=begin")
        || line.starts_with("=for")
        || line.starts_with("=encoding")
        || line.starts_with("=item")
}

fn collect_simple_pod_module_links(line: &str, current_module: &str, uris: &mut BTreeSet<String>) {
    let mut rest = line;
    while let Some(start) = rest.find("L<") {
        let after_open = &rest[start + 2..];
        let Some(end) = after_open.find('>') else {
            break;
        };
        let target = after_open[..end].trim();
        if let Some(link_target) =
            PerlDocumentationTarget::from_workspace_pod_link_target(target, current_module)
        {
            if link_target.name() != current_module || link_target.section().is_some() {
                uris.insert(link_target.perldoc_uri());
            }
        }
        rest = &after_open[end + 1..];
    }
}

/// Fetch Perl documentation using perldoc
#[cfg(not(target_arch = "wasm32"))]
fn fetch_perldoc(module: &str, config: &WorkspaceConfig) -> Option<String> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let oracle = PerlOracleEnv::for_perldoc(config, cwd);
    let timeout_secs = oracle.timeout.as_secs();

    // Run perldoc -T Module::Name to get plain text documentation
    // Use -- to prevent argument injection if module starts with -
    let mut cmd = oracle.into_command();
    cmd.arg("-T").arg("--").arg(module);
    let output = match crate::util::run_command_with_timeout(cmd, timeout_secs) {
        Ok(output) => output,
        Err(e) => {
            tracing::warn!(module, error = %e, "Failed to run perldoc");
            return None;
        }
    };

    if output.status.success() {
        String::from_utf8(output.stdout)
            .map_err(|e| tracing::warn!(module, error = %e, "Invalid UTF-8 in perldoc output"))
            .ok()
    } else {
        None
    }
}

/// Fetch Perl documentation using perldoc.
#[cfg(target_arch = "wasm32")]
fn fetch_perldoc(_module: &str, _config: &WorkspaceConfig) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_tdd_support::must;
    use std::fs;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn parser_fetch_perldoc_strict() {
        // Try to fetch documentation for the 'strict' module
        // This test will be skipped if perldoc is not available
        let config = WorkspaceConfig::default();
        if let Some(content) = fetch_perldoc("strict", &config) {
            assert!(content.contains("strict") || content.contains("STRICT"));
            assert!(content.len() > 100); // Should have some substantial content
        } else {
            eprintln!("Skipping test: perldoc not available or strict module not found");
        }
    }

    #[test]
    fn parser_fetch_perldoc_invalid() {
        // Try to fetch documentation for a non-existent module
        let config = WorkspaceConfig::default();
        let result = fetch_perldoc("ThisModuleDefinitelyDoesNotExist12345", &config);
        assert!(result.is_none());
    }

    #[test]
    fn parser_virtual_content_perldoc_uri() {
        let uri = "perldoc://strict";
        let config = WorkspaceConfig::default();
        let server = LspServer::new();
        *server.workspace_config.lock() = config;
        let content = server.fetch_virtual_content(uri);
        // May be None if perldoc is not available
        if let Some(content) = content {
            assert!(!content.is_empty());
        }
    }

    #[test]
    fn parser_virtual_content_invalid_scheme() {
        let uri = "invalid://some/path";
        let config = WorkspaceConfig::default();
        let server = LspServer::new();
        *server.workspace_config.lock() = config;
        let content = server.fetch_virtual_content(uri);
        assert!(content.is_none());
    }

    #[test]
    fn parser_virtual_content_rejects_malformed_perldoc_target() {
        let server = LspServer::new();

        for uri in ["perldoc://Local/Doc", "perldoc://Local::>", "perldoc:// Local::Doc"] {
            require(
                server.fetch_virtual_content(uri).is_none(),
                format!("expected malformed target {uri} to be rejected"),
            );
        }
    }

    #[test]
    fn parser_virtual_content_rejects_malformed_uri() {
        for uri in [
            "not a uri",
            "perldoc://",
            "perldoc://Local/Doc",
            "perldoc://Local::>",
            "perldoc:// Local::Doc",
        ] {
            require(!is_valid_virtual_content_uri(uri), format!("expected {uri} to be invalid"));
        }
        for uri in ["perldoc://strict", "perldoc://Module::Name"] {
            require(is_valid_virtual_content_uri(uri), format!("expected {uri} to be valid"));
        }
    }

    #[test]
    fn parser_fetch_perldoc_argument_injection() {
        // Try to fetch documentation with a flag-like string
        // This should not crash or execute unexpected commands
        // perldoc -T -- -f should look for module named "-f" which likely doesn't exist
        let config = WorkspaceConfig::default();
        let result = fetch_perldoc("-f", &config);
        assert!(result.is_none());
    }

    #[test]
    fn parser_enriches_strict_perldoc_with_warnings_link() {
        let content = enrich_core_pragma_perldoc("strict", "strict docs".to_string());

        assert!(content.starts_with("Related virtual perldoc:\n- perldoc://warnings\n\n"));
        assert!(content.ends_with("strict docs"));
    }

    #[test]
    fn parser_enriches_warnings_perldoc_with_strict_link() {
        let content = enrich_core_pragma_perldoc("warnings", "warnings docs".to_string());

        assert!(content.starts_with("Related virtual perldoc:\n- perldoc://strict\n\n"));
        assert!(content.ends_with("warnings docs"));
    }

    #[test]
    fn parser_leaves_other_perldoc_content_unchanged() {
        let content = enrich_core_pragma_perldoc("vars", "vars docs".to_string());

        assert_eq!(content, "vars docs");
    }

    #[test]
    fn parser_fetch_workspace_perldoc_requires_workspace() -> TestResult {
        let server = LspServer::new();
        let target = PerlDocumentationTarget::new("Local::Doc").ok_or("expected doc target")?;

        assert!(server.fetch_workspace_perldoc(&target).is_none());
        Ok(())
    }

    #[test]
    fn parser_fetch_workspace_perldoc_reads_local_pod() -> TestResult {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("workspace");
        let module_dir = root.join("lib").join("Local");
        fs::create_dir_all(&module_dir)?;
        fs::write(
            module_dir.join("Doc.pm"),
            "package Local::Doc;\n\n=head1 NAME\n\nLocal::Doc - local docs\n\n=head1 DESCRIPTION\n\nLocal POD.\n\n=head2 reset\n\nReset local state.\n\n=cut\n\n1;\n",
        )?;

        let server = LspServer::new();
        let workspace_uri =
            url::Url::from_directory_path(&root).map_err(|_| "failed to create workspace URI")?;
        *server.workspace_folders.lock() = vec![
            crate::runtime::workspace_folder::WorkspaceFolderState::new(workspace_uri.to_string())
                .with_path(root),
        ];
        {
            let mut config = server.workspace_config.lock();
            config.include_paths = vec!["lib".to_string()];
            config.use_perl5lib = false;
            config.use_system_inc = false;
        }

        let target = PerlDocumentationTarget::new("Local::Doc").ok_or("expected doc target")?;
        let content =
            server.fetch_workspace_perldoc(&target).ok_or("expected local workspace POD")?;

        assert!(content.contains("Workspace virtual perldoc"));
        assert!(content.contains("Module: Local::Doc"));
        assert!(content.contains("Local::Doc - local docs"));
        assert!(content.contains("DESCRIPTION\nLocal POD."));
        assert!(content.contains("METHOD reset\nReset local state."));
        Ok(())
    }

    #[test]
    fn parser_fetch_workspace_perldoc_ignores_missing_workspace_module() -> TestResult {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("workspace");
        fs::create_dir_all(root.join("lib"))?;

        let server = LspServer::new();
        let workspace_uri =
            url::Url::from_directory_path(&root).map_err(|_| "failed to create workspace URI")?;
        *server.workspace_folders.lock() = vec![
            crate::runtime::workspace_folder::WorkspaceFolderState::new(workspace_uri.to_string())
                .with_path(root),
        ];
        {
            let mut config = server.workspace_config.lock();
            config.include_paths = vec!["lib".to_string()];
            config.use_perl5lib = false;
            config.use_system_inc = false;
        }

        let target =
            PerlDocumentationTarget::new("Local::Missing").ok_or("expected missing doc target")?;
        assert!(server.fetch_workspace_perldoc(&target).is_none());
        Ok(())
    }

    #[test]
    fn parser_workspace_text_document_content_returns_local_pod() -> TestResult {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("workspace");
        let module_dir = root.join("lib").join("Local");
        fs::create_dir_all(&module_dir)?;
        fs::write(
            module_dir.join("Doc.pm"),
            "package Local::Doc;\n\n=head1 NAME\n\nLocal::Doc - local docs\n\n=head1 DESCRIPTION\n\nLocal POD.\n\n=cut\n\n1;\n",
        )?;

        let server = LspServer::new();
        let workspace_uri =
            url::Url::from_directory_path(&root).map_err(|_| "failed to create workspace URI")?;
        *server.workspace_folders.lock() = vec![
            crate::runtime::workspace_folder::WorkspaceFolderState::new(workspace_uri.to_string())
                .with_path(root),
        ];
        {
            let mut config = server.workspace_config.lock();
            config.include_paths = vec!["lib".to_string()];
            config.use_perl5lib = false;
            config.use_system_inc = false;
        }

        let result = server
            .handle_text_document_content(Some(json!({ "uri": "perldoc://Local::Doc" })))?
            .ok_or("expected workspace textDocumentContent result")?;
        let text = result.get("text").and_then(Value::as_str).ok_or("expected text result")?;

        assert!(text.contains("Workspace virtual perldoc"));
        assert!(text.contains("Local::Doc - local docs"));
        Ok(())
    }

    #[test]
    fn parser_fetch_workspace_perldoc_ignores_local_module_without_pod() -> TestResult {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("workspace");
        let module_dir = root.join("lib").join("Local");
        fs::create_dir_all(&module_dir)?;
        fs::write(module_dir.join("NoPod.pm"), "package Local::NoPod;\n1;\n")?;

        let server = LspServer::new();
        let workspace_uri =
            url::Url::from_directory_path(&root).map_err(|_| "failed to create workspace URI")?;
        *server.workspace_folders.lock() = vec![
            crate::runtime::workspace_folder::WorkspaceFolderState::new(workspace_uri.to_string())
                .with_path(root),
        ];
        {
            let mut config = server.workspace_config.lock();
            config.include_paths = vec!["lib".to_string()];
            config.use_perl5lib = false;
            config.use_system_inc = false;
        }

        let target = PerlDocumentationTarget::new("Local::NoPod").ok_or("expected doc target")?;
        assert!(server.fetch_workspace_perldoc(&target).is_none());
        Ok(())
    }

    #[test]
    fn parser_formats_workspace_pod_virtual_content() -> TestResult {
        let mut pod = perl_pod::PodDoc {
            name: Some("Local::Doc - local docs".to_string()),
            synopsis: Some("use Local::Doc;".to_string()),
            description: Some("Local documentation from the workspace.".to_string()),
            methods: std::collections::HashMap::new(),
            ..Default::default()
        };
        pod.methods.insert("reset".to_string(), "Reset local state.".to_string());

        let content = format_workspace_pod_virtual_content(
            "Local::Doc",
            None,
            Path::new("lib/Local/Doc.pm"),
            &pod,
            &[],
        )
        .ok_or("expected workspace POD content")?;

        assert!(content.contains("Workspace virtual perldoc"));
        assert!(content.contains("Module: Local::Doc"));
        assert!(content.contains("Local::Doc - local docs"));
        assert!(content.contains("METHOD reset\nReset local state."));
        Ok(())
    }

    #[test]
    fn parser_formats_workspace_pod_virtual_content_with_requested_section() -> TestResult {
        let mut pod = perl_pod::PodDoc {
            name: Some("Local::Doc - local docs".to_string()),
            synopsis: None,
            description: Some("Local documentation from the workspace.".to_string()),
            methods: std::collections::HashMap::new(),
            ..Default::default()
        };
        pod.methods.insert("reset".to_string(), "Reset local state.".to_string());

        let content = format_workspace_pod_virtual_content(
            "Local::Doc",
            Some("reset"),
            Path::new("lib/Local/Doc.pm"),
            &pod,
            &[],
        )
        .ok_or("expected workspace POD content")?;

        assert!(content.contains("Module: Local::Doc"));
        assert!(content.contains("Section: reset"));
        assert!(content.contains("METHOD reset\nReset local state."));
        Ok(())
    }

    #[test]
    fn parser_formats_workspace_pod_virtual_content_with_related_links() -> TestResult {
        let pod = perl_pod::PodDoc {
            name: Some("Local::Doc - local docs".to_string()),
            synopsis: None,
            description: None,
            methods: std::collections::HashMap::new(),
            ..Default::default()
        };

        let content = format_workspace_pod_virtual_content(
            "Local::Doc",
            None,
            Path::new("lib/Local/Doc.pm"),
            &pod,
            &["perldoc://Alpha::First".to_string(), "perldoc://Zoo::Last".to_string()],
        )
        .ok_or("expected workspace POD content")?;

        assert!(
            content.contains(
                "Related virtual perldoc:\n- perldoc://Alpha::First\n- perldoc://Zoo::Last"
            )
        );
        assert!(content.contains("NAME\nLocal::Doc - local docs"));
        Ok(())
    }

    #[test]
    fn parser_workspace_pod_related_perldoc_links_are_sorted_and_filtered() {
        let source = r#"package Local::Doc;

=head1 NAME

Local::Doc - local docs

=head1 DESCRIPTION

See L<Zoo::Last>, L<Alpha::First>, L<Zoo::Last>, and L<Local::Doc>.
Labeled module links such as L<beta docs|Beta::Labeled> stay navigable.
Core pragma links L<strict>, L<warnings>, and L<strict docs|strict> are valid virtual perldoc targets.
Local section links such as L</reset> and L<section docs|/SEE ALSO> stay navigable.
Module section links such as L<build docs|Local::Helper/build> stay navigable.
Ignore L<display|https://example.invalid>, L<|Beta::EmptyLabel>, L<display|Broken::>, L<NotAModule>, L<NotAModule/reset>, and L<broken docs|Local::Helper/bad/section>.

=cut

my $non_pod = 'L<Code::Reference>';

1;
"#;

        let links = workspace_pod_related_perldoc_uris("Local::Doc", source);

        assert_eq!(
            links,
            vec![
                "perldoc://Alpha::First",
                "perldoc://Beta::Labeled",
                "perldoc://Local::Doc#SEE%20ALSO",
                "perldoc://Local::Doc#reset",
                "perldoc://Local::Helper#build",
                "perldoc://Zoo::Last",
                "perldoc://strict",
                "perldoc://warnings"
            ]
        );
    }

    #[test]
    fn collect_simple_pod_module_links_boundary_discriminator() {
        let mut uris = BTreeSet::new();
        let current_module = "Local::Doc";
        let link_target = PerlDocumentationTarget::from_simple_pod_link_target("Local::Other");

        assert!(
            link_target.as_ref().is_some_and(|link_target| link_target.name() != current_module),
            "link_target.name() != current_module must keep related module links",
        );

        collect_simple_pod_module_links(
            "See L<Local::Doc> for this module and L<Local::Other> for the neighbor.",
            current_module,
            &mut uris,
        );

        assert_eq!(
            uris.into_iter().collect::<Vec<_>>(),
            vec!["perldoc://Local::Other".to_string()]
        );
    }

    #[test]
    fn parser_workspace_pod_related_perldoc_links_ignore_malformed_and_empty_targets() {
        let source = r#"package Local::Doc;

=head1 DESCRIPTION

Malformed links do not leak: L<Broken::Target
Empty module segments do not leak: L<Broken::>.
The next line still scans after the malformed line: L<Alpha::First>.

=cut

Plain code after cut does not leak: L<Code::Reference>.

1;
"#;

        let links = workspace_pod_related_perldoc_uris("Local::Doc", source);

        assert_eq!(links, vec!["perldoc://Alpha::First"]);
    }

    fn require(condition: bool, message: String) {
        if !condition {
            must(Err::<(), _>(message));
        }
    }
}
