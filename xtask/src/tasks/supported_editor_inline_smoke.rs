use color_eyre::eyre::{Context, Result, bail};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const ROUTES: &[SupportedEditorRouteRequirement] = &[
    SupportedEditorRouteRequirement {
        route: "stdio_cli_smoke",
        claim: "the generic stdio LSP smoke covers static, dynamic, and disabled inline-completion clients",
        proof_surfaces: &[ProofSurfaceRequirement {
            path: "xtask/src/tasks/inline_completion_smoke.rs",
            markers: &[
                "fn run_static_client",
                "fn run_dynamic_client",
                "fn run_disabled_client",
                "textDocument/inlineCompletion",
                "perl-inlineCompletion",
                "assert_inline_completion_runtime",
            ],
        }],
    },
    SupportedEditorRouteRequirement {
        route: "lsp4ij_upstream_integration",
        claim: "the JetBrains path is documented as upstream LSP4IJ integration with dynamic inline-completion registration",
        proof_surfaces: &[
            ProofSurfaceRequirement {
                path: "docs/EDITORS/INTELLIJ_IDEA_SETUP.md",
                markers: &[
                    "Recommended: LSP4IJ Upstream Integration",
                    "The upstream LSP4IJ entry should launch `perllsp` with stdio transport",
                    "client/registerCapability",
                    "textDocument/inlineCompletion",
                    "inlineCompletionProvider",
                    "perllsp --stdio",
                ],
            },
            ProofSurfaceRequirement {
                path: "xtask/src/tasks/inline_completion_smoke.rs",
                markers: &[
                    "fn run_dynamic_client",
                    "client/registerCapability",
                    "textDocument/inlineCompletion",
                    "perl-inlineCompletion",
                ],
            },
        ],
    },
    SupportedEditorRouteRequirement {
        route: "vscode_extension_path",
        claim: "the VS Code route uses the managed extension path with configurable perllsp binary resolution",
        proof_surfaces: &[
            ProofSurfaceRequirement {
                path: "docs/EDITORS/VS_CODE_SETUP.md",
                markers: &[
                    "EffortlessMetrics.perl-lsp-rs",
                    "The extension auto-downloads the matching `perllsp` server by default.",
                    "\"perl-lsp.serverPath\"",
                    "\"perl-lsp.autoDownload\"",
                ],
            },
            ProofSurfaceRequirement {
                path: "vscode-extension/package.json",
                markers: &[
                    "\"perl-lsp.serverPath\"",
                    "\"perl-lsp.autoDownload\"",
                    "Absolute path to the `perllsp` binary. Leave empty to auto-download a release build.",
                    "Automatically download the `perllsp` binary if it is not found locally.",
                ],
            },
        ],
    },
    SupportedEditorRouteRequirement {
        route: "release_built_binary_smoke",
        claim: "the release gate requires building perllsp and running the inline-completion stdio smoke against that binary",
        proof_surfaces: &[
            ProofSurfaceRequirement {
                path: "docs/development/INLINE_COMPLETION_RELEASE_GATE.md",
                markers: &[
                    "./scripts/cargo-safe build -p perllsp --profile agent --locked",
                    "./scripts/cargo-safe xtask inline-completion-smoke --binary target/agent/perllsp",
                    "static clients receive top-level `inlineCompletionProvider`",
                    "dynamic clients omit the static provider",
                    "disabledFeatures: [\"lsp.inline_completion\"]",
                ],
            },
            ProofSurfaceRequirement {
                path: "xtask/src/tasks/inline_completion_smoke.rs",
                markers: &[
                    "pub fn run(binary: PathBuf) -> Result<()>",
                    "resolve_binary_path",
                    "run_static_client(&binary)?;",
                    "run_dynamic_client(&binary)?;",
                    "run_disabled_client(&binary)?;",
                ],
            },
        ],
    },
];

#[derive(Debug, Clone, Copy)]
struct SupportedEditorRouteRequirement {
    route: &'static str,
    claim: &'static str,
    proof_surfaces: &'static [ProofSurfaceRequirement],
}

#[derive(Debug, Clone, Copy)]
struct ProofSurfaceRequirement {
    path: &'static str,
    markers: &'static [&'static str],
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct SupportedEditorInlineSmokeReceipt {
    schema_version: &'static str,
    provider: &'static str,
    provider_action: &'static str,
    claim_boundary: &'static str,
    route_count: usize,
    all_supported_routes_registered: bool,
    supported_editor_routes: BTreeMap<&'static str, SupportedEditorRouteReceipt>,
    future_gated: BTreeMap<&'static str, &'static str>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct SupportedEditorRouteReceipt {
    status: &'static str,
    claim: &'static str,
    proof_surface_count: usize,
    proof_surfaces: Vec<ProofSurfaceReceipt>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct ProofSurfaceReceipt {
    path: &'static str,
    required_marker_count: usize,
    required_markers: Vec<&'static str>,
}

pub fn run(receipt: PathBuf) -> Result<()> {
    let root = crate::utils::project_root()?;
    let receipt_data = summarize_supported_editor_routes(&root, ROUTES)?;

    write_receipt(&receipt, &receipt_data)?;
    println!(
        "supported editor inline smoke receipt OK: {} routes, {}",
        receipt_data.route_count,
        receipt.display()
    );
    Ok(())
}

fn summarize_supported_editor_routes(
    root: &Path,
    requirements: &[SupportedEditorRouteRequirement],
) -> Result<SupportedEditorInlineSmokeReceipt> {
    let mut supported_editor_routes = BTreeMap::new();
    for route in requirements {
        let mut proof_surfaces = Vec::new();
        for surface in route.proof_surfaces {
            validate_proof_surface(root, route.route, surface)?;
            proof_surfaces.push(ProofSurfaceReceipt {
                path: surface.path,
                required_marker_count: surface.markers.len(),
                required_markers: surface.markers.to_vec(),
            });
        }
        supported_editor_routes.insert(
            route.route,
            SupportedEditorRouteReceipt {
                status: "registered",
                claim: route.claim,
                proof_surface_count: proof_surfaces.len(),
                proof_surfaces,
            },
        );
    }

    let future_gated = BTreeMap::from([
        ("live_vscode_ui_automation", "future_gated"),
        ("live_lsp4ij_ui_automation", "future_gated"),
        ("runtime_multiline_inline_completion", "future_gated"),
    ]);

    Ok(SupportedEditorInlineSmokeReceipt {
        schema_version: "supported-editor-inline-smoke.v1",
        provider: "inline_completion",
        provider_action: "supported_editor_inline_smoke_bundle",
        claim_boundary: "machine-readable supported-editor inline smoke bundle only; verifies repository proof surfaces and command contracts, not live editor UI automation, source mirror, release, AI, next-edit, or runtime multiline behavior",
        route_count: requirements.len(),
        all_supported_routes_registered: true,
        supported_editor_routes,
        future_gated,
    })
}

fn validate_proof_surface(
    root: &Path,
    route: &str,
    surface: &ProofSurfaceRequirement,
) -> Result<()> {
    let path = root.join(surface.path);
    let content =
        fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let missing = surface
        .markers
        .iter()
        .copied()
        .filter(|marker| !content.contains(marker))
        .collect::<Vec<_>>();

    if !missing.is_empty() {
        bail!(
            "supported editor inline smoke route `{route}` proof surface `{}` is missing markers: {}",
            surface.path,
            missing.join(", ")
        );
    }

    Ok(())
}

fn write_receipt(path: &Path, receipt: &SupportedEditorInlineSmokeReceipt) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(receipt)?;
    fs::write(path, format!("{json}\n"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use tempfile::TempDir;

    const TEST_PROOFS: &[ProofSurfaceRequirement] = &[ProofSurfaceRequirement {
        path: "proof.md",
        markers: &["present marker", "missing marker"],
    }];
    const TEST_ROUTES: &[SupportedEditorRouteRequirement] = &[SupportedEditorRouteRequirement {
        route: "test_route",
        claim: "test route claim",
        proof_surfaces: TEST_PROOFS,
    }];

    #[test]
    fn receipt_records_supported_editor_routes() -> Result<()> {
        let temp = TempDir::new()?;
        write_fixture_files(temp.path(), ROUTES)?;

        let receipt = summarize_supported_editor_routes(temp.path(), ROUTES)?;

        assert!(receipt.all_supported_routes_registered);
        assert_eq!(receipt.route_count, ROUTES.len());
        for route in ROUTES {
            let entry = receipt
                .supported_editor_routes
                .get(route.route)
                .ok_or_else(|| color_eyre::eyre::eyre!("missing route {}", route.route))?;
            assert_eq!(entry.status, "registered");
            assert_eq!(entry.claim, route.claim);
            assert_eq!(entry.proof_surface_count, route.proof_surfaces.len());
            assert_eq!(entry.proof_surfaces.len(), route.proof_surfaces.len());
            for (actual, expected) in entry.proof_surfaces.iter().zip(route.proof_surfaces.iter()) {
                assert_eq!(actual.path, expected.path);
                assert_eq!(actual.required_marker_count, expected.markers.len());
                assert_eq!(actual.required_markers, expected.markers.to_vec());
            }
        }
        assert_eq!(receipt.future_gated.get("live_vscode_ui_automation"), Some(&"future_gated"));
        assert_eq!(receipt.future_gated.get("live_lsp4ij_ui_automation"), Some(&"future_gated"));
        assert_eq!(
            receipt.future_gated.get("runtime_multiline_inline_completion"),
            Some(&"future_gated")
        );

        Ok(())
    }

    #[test]
    fn receipt_rejects_missing_proof_markers() -> Result<()> {
        let temp = TempDir::new()?;
        fs::write(temp.path().join("proof.md"), "present marker\n")?;

        let Err(error) = summarize_supported_editor_routes(temp.path(), TEST_ROUTES) else {
            bail!("missing proof marker must fail receipt generation");
        };
        assert!(
            error.to_string().contains("missing marker"),
            "error should identify missing marker, got {error}"
        );

        Ok(())
    }

    #[test]
    fn receipt_json_keeps_claim_boundary() -> Result<()> {
        let temp = TempDir::new()?;
        write_fixture_files(temp.path(), ROUTES)?;
        let receipt = summarize_supported_editor_routes(temp.path(), ROUTES)?;
        let value: Value = serde_json::to_value(&receipt)?;

        assert_eq!(
            value.get("schema_version").and_then(Value::as_str),
            Some("supported-editor-inline-smoke.v1")
        );
        assert_eq!(
            value.get("provider_action").and_then(Value::as_str),
            Some("supported_editor_inline_smoke_bundle")
        );
        let boundary = value
            .get("claim_boundary")
            .and_then(Value::as_str)
            .ok_or_else(|| color_eyre::eyre::eyre!("missing claim_boundary"))?;
        assert!(boundary.contains("not live editor UI automation"));
        assert!(boundary.contains("runtime multiline behavior"));

        Ok(())
    }

    #[test]
    fn write_receipt_creates_parent_dirs_and_serializes_routes() -> Result<()> {
        let temp = TempDir::new()?;
        write_fixture_files(temp.path(), ROUTES)?;
        let receipt = summarize_supported_editor_routes(temp.path(), ROUTES)?;
        let receipt_path = temp.path().join("nested").join("supported-editor.json");

        write_receipt(&receipt_path, &receipt)?;

        let value: Value = serde_json::from_str(&fs::read_to_string(receipt_path)?)?;
        assert_eq!(value.get("route_count").and_then(Value::as_u64), Some(ROUTES.len() as u64));
        assert_eq!(
            value
                .pointer("/supported_editor_routes/release_built_binary_smoke/status")
                .and_then(Value::as_str),
            Some("registered")
        );

        Ok(())
    }

    fn write_fixture_files(
        root: &Path,
        requirements: &[SupportedEditorRouteRequirement],
    ) -> Result<()> {
        let mut content_by_path = BTreeMap::<&str, Vec<&str>>::new();
        for route in requirements {
            for surface in route.proof_surfaces {
                content_by_path.entry(surface.path).or_default().extend(surface.markers);
            }
        }

        for (path, markers) in content_by_path {
            let full_path = root.join(path);
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(full_path, markers.join("\n"))?;
        }

        Ok(())
    }
}
