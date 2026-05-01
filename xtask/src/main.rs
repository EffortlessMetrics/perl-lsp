//! Xtask automation for tree-sitter-perl
//!
//! This binary provides custom automation tasks for building, testing,
//! and maintaining the tree-sitter-perl project.

use clap::{CommandFactory, Parser};
use color_eyre::eyre::{Result, eyre};
use std::path::PathBuf;

mod cli;
mod tasks;
mod types;
mod utils;
use tasks::check_test_wiring;
use tasks::dead_code::DeadCodeConfig;
use types::TestSuite;
use crate::cli::{
    AgentCommand, AgentLeaseCommand, AgentReceiptCommand, Cli, Commands, CpanCorpusCommand,
    FeaturesCommand, FixForwardCommand, GatePolicyCommand, GateReceiptsCommand,
    GateReceiptsFormat, GeneratedFilesCommand, MergeReadyCommand, MetricsCommand,
    ParserRatchetCommand, PrepCratesMode, QueueCommand, ReleaseCommand, UxScorecardOutputFormat,
};
use tasks::metrics;
use tasks::unwired_scan::UnwiredScanConfig;
use tasks::ux_scorecard::UxScorecardFormat;
use tasks::*;

fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

    match cli.command {
        Commands::List => {
            print_top_level_commands();
            Ok(())
        }
        Commands::Ci => ci::run(),
        Commands::CheckOnly => ci::check_only(),
        Commands::CheckToolchain { doctor } => check_toolchain::run(doctor),
        Commands::Queue { command } => match command {
            QueueCommand::Snapshot { out, fixture } => queue_snapshot::run_snapshot(out, fixture),
            QueueCommand::Health { receipt, fixture } => {
                queue_health::run(queue_health::QueueHealthArgs { receipt, fixture })
            }
        },
        Commands::Build { release, features, c_scanner, rust_scanner } => {
            build::run(release, features, c_scanner, rust_scanner)
        }
        Commands::Test { release, suite, features, verbose, coverage } => {
            test::run(release, suite, features, verbose, coverage)
        }
        Commands::Bench { name, save, output } => bench::run(name, save, output),
        Commands::BenchRun { output, quick, category } => {
            benchmarks::run_benchmarks(output, quick, category)
        }
        Commands::BenchCompare { fail_on_regression } => {
            benchmarks::compare_benchmarks(fail_on_regression)
        }
        Commands::BenchFormat { receipt, markdown } => {
            benchmarks::format_benchmarks(receipt, markdown)
        }
        Commands::BenchExtract { base_path, output } => {
            benchmarks::extract_criterion(base_path, output)
        }
        Commands::BenchAlert { format, check } => benchmarks::alert_benchmarks(format, check),
        Commands::BenchAlertTest => benchmarks::test_alert_system(),
        Commands::InjectShaAssets {
            version,
            owner,
            repo,
            prefix,
            checksums,
            brew_out,
            asset_map_out,
        } => inject_sha_assets::run(inject_sha_assets::InjectShaAssetsConfig {
            version,
            owner,
            repo,
            prefix,
            checksums,
            brew_out,
            asset_map_out,
        }),
        Commands::UpdateHomebrew { version, owner, repo, prefix, output } => {
            update_homebrew::run(update_homebrew::UpdateHomebrewConfig {
                version,
                owner,
                repo,
                prefix,
                output,
            })
        }
        Commands::Compare {
            c_only,
            rust_only,
            scanner_only,
            validate_only,
            output_dir,
            check_gates,
            report,
        } => compare::run(
            c_only,
            rust_only,
            scanner_only,
            validate_only,
            output_dir,
            check_gates,
            report,
        ),
        Commands::Doc { open, all_features } => doc::run(open, all_features),
        Commands::Check { clippy, fmt, all } => check::run(clippy, fmt, all),
        Commands::Fmt { check, package } => fmt::run(check, package),
        #[cfg(feature = "legacy")]
        Commands::Corpus { path, scanner, diagnose, test } => {
            corpus::run(path, scanner, diagnose, test)
        }
        #[cfg(feature = "parser-tasks")]
        Commands::Highlight { path, scanner } => highlight::run(path, scanner),
        Commands::Clean { all } => clean::run(all),
        Commands::DeadCode { mode, strict } => dead_code::run(DeadCodeConfig { mode, strict }),
        #[cfg(feature = "parser-tasks")]
        Commands::Bindings { header, output } => bindings::run(header, output),
        Commands::Dev { watch, port } => dev::run(watch, port),
        Commands::DevexDoctor => devex_doctor::run(),
        Commands::ParseRust { source, sexp, ast, bench } => {
            parse_rust::run(source, sexp, ast, bench)
        }
        Commands::Release { command } => match command {
            ReleaseCommand::Prepare { version, yes } => release::run(version, yes),
            ReleaseCommand::Evidence { version, out } => release_evidence::scaffold(&version, &out),
            ReleaseCommand::VerifyEvidence { version, receipt, bundle_dir } => {
                let effective_bundle_dir = bundle_dir.unwrap_or_else(|| {
                    PathBuf::from(format!("target/release-evidence/v{version}"))
                });
                release_evidence::verify(&version, &effective_bundle_dir, &receipt)
            }
        },
        Commands::ReleaseNotes { tag, output, root } => release_notes::run(tag, output, root),
        Commands::ReleaseTurnkey {
            version,
            positional_version,
            prerelease,
            dry_run,
            skip_crates,
            skip_extension,
            skip_docker,
            base_branch,
            no_auto_merge,
            no_wait_pr_merge,
            no_wait_release,
            workflow_timeout,
        } => release_turnkey::run(release_turnkey::ReleaseTurnkeyConfig {
            version,
            positional_version,
            prerelease,
            dry_run,
            skip_crates,
            skip_extension,
            skip_docker,
            base_branch,
            no_auto_merge,
            no_wait_pr_merge,
            no_wait_release,
            workflow_timeout,
        }),
        Commands::PrepCratesIoLaunch { mode } => {
            prep_crates_io_launch::run(matches!(mode, PrepCratesMode::All))
        }
        Commands::TestHeredoc { release, verbose } => {
            // Run heredoc tests using the test module with heredoc suite
            test::run(
                release,
                Some(TestSuite::Heredoc),
                Some(vec!["pure-rust".to_string()]),
                verbose,
                false,
            )
        }
        Commands::TestEdgeCases { bench, coverage, test } => edge_cases::run(bench, coverage, test),
        Commands::CiAuditWorkflows => ci_audit_workflows::run(),
        Commands::WorkflowPolicyLint { receipt, fixture } => {
            workflow_policy_lint::run(workflow_policy_lint::WorkflowPolicyLintConfig {
                receipt,
                fixture,
            })
        }
        Commands::CiMeasure => ci_measure::run(),
        Commands::CiCostMonitor { days, json } => ci_metrics::run_cost_monitor(days, json),
        Commands::CiBaseline { branch, days, limit, output } => {
            ci_metrics::run_ci_baseline(branch, days, limit, output)
        }
        Commands::CiScope { base, format } => {
            ci_scope::run(ci_scope::CiScopeConfig { base, format })
        }

        Commands::WorkflowTriggerLint { policy, receipt, fixture, format } => {
            workflow_trigger_lint::run(policy, receipt, fixture, format)
        }
        Commands::CheckVersionSync => check_version_sync::run(),
        Commands::CheckFromRaw => ci_policy::check_from_raw(),
        Commands::SecurityHardening => hardening::security_hardening(),
        Commands::PerformanceHardening => hardening::performance_hardening(),
        Commands::ProductionGatesValidation => hardening::production_gates_validation(),
        Commands::ForensicsHarvest { pr } => forensics::run_harvest(&pr),
        Commands::ForensicsTemporal { pr } => forensics::run_temporal(&pr),
        Commands::ForensicsTelemetryQuick { pr } => forensics::run_telemetry_quick(&pr),
        Commands::ForensicsTelemetryFull { pr } => forensics::run_telemetry_full(&pr),
        Commands::ForensicsDossier { pr } => forensics::run_dossier(&pr),
        Commands::ForensicsRender { pr, format } => forensics::run_render(&pr, &format),
        Commands::VerifyPublicationFacts { args } => publication_facts::run(args),
        Commands::GhLabels => github::run_labels(),
        Commands::GhTriage { limit } => github::run_issues_needing_triage(limit),
        Commands::GhBackfillPrefixedLabels { apply } => github::run_backfill_prefixed_labels(apply),
        Commands::CorpusAudit { corpus_path, output, check, fresh } => {
            corpus_audit::run(corpus_audit::AuditConfig {
                corpus_path,
                output_path: output,
                timeout: std::time::Duration::from_secs(30),
                fresh,
                check,
            })
        }
        Commands::ParserMatrix { report, output } => parser_matrix::run_with_paths(report, output),
        #[cfg(feature = "parser-tasks")]
        Commands::CompareThree { verbose, format } => {
            compare_parsers::run_three_way(verbose, format.as_str())
        }
        Commands::TestLsp { create_only, test, cleanup } => {
            test_lsp::run(create_only, test, cleanup)
        }
        Commands::BumpVersion { version } => bump_version::run(version),
        Commands::PublishCrates { yes, dry_run } => publish::publish_crates(yes, dry_run),
        Commands::PublishRelease { version, dry_run, git_ref } => {
            publish::publish_release(version, dry_run, git_ref)
        }
        Commands::HookCheck => hook_checks::run_hook_check(),
        Commands::HookRegistryCheck => hook_checks::run_hook_registry_check(),
        Commands::HookTests => hook_checks::run_hook_tests(),
        Commands::ForbidFatalConstructs { args } => forbid_fatal_constructs::run(args),
        Commands::CiHygiene { command, args } => ci_hygiene::run(command, args),
        Commands::PublishVscode { yes, token } => publish::publish_vscode(yes, token),
        Commands::PublishClosure { crate_name } => publish_closure::run(crate_name),
        Commands::PublishedCrateCount => count_ratchet::run(),
        Commands::PublishManifestCheck => publish_manifest_check::run(),
        Commands::SmokeTestRelease { version } => publish::smoke_test_release(version),
        Commands::PublishReceipts { date } => publish_receipts::run(date),
        Commands::ParserCorpusSweep {
            roots,
            manifest,
            output,
            baseline,
            enforce,
            verbose,
            receipt,
        } => {
            let base_roots = roots.unwrap_or_else(parser_corpus_sweep::default_base_roots);
            let corpus_roots = parser_corpus_sweep::resolve_corpus_roots(&base_roots);
            parser_corpus_sweep::run(parser_corpus_sweep::SweepConfig {
                corpus_profile: None,
                base_roots,
                corpus_roots,
                manifest_path: manifest,
                manifest_perl5lib: Vec::new(),
                output_path: output,
                baseline_path: baseline,
                enforce,
                verbose,
                receipt,
            })
        }
        Commands::ParserRatchet { command } => match command {
            ParserRatchetCommand::Run { profile, base, head, receipt, force_selected } => {
                parser_ratchet::run(parser_ratchet::ParserRatchetRunConfig {
                    profile,
                    base,
                    head,
                    receipt,
                    force_selected,
                })
            }
        },
        Commands::CpanCorpus { command } => {
            let mut config = cpan_corpus::CpanCorpusConfig::default();
            match command {
                CpanCorpusCommand::FetchList { top_n, output } => {
                    config.top_n = top_n;
                    if let Some(out) = output {
                        config.dist_list = out;
                    }
                    cpan_corpus::fetch_list(&config)
                }
                CpanCorpusCommand::Install { dist_list, install_dir, verbose, reset } => {
                    if let Some(dl) = dist_list {
                        config.dist_list = dl;
                    }
                    config.force_reset = reset;
                    if let Some(id) = install_dir {
                        config.install_dir = id;
                    }
                    config.verbose = verbose;
                    cpan_corpus::install(&config)
                }
                CpanCorpusCommand::Sweep { output, enforce, verbose, install_dir } => {
                    if let Some(id) = install_dir {
                        config.install_dir = id;
                    }
                    config.verbose = verbose;
                    cpan_corpus::sweep(&config, output, enforce)
                }
                CpanCorpusCommand::Ratchet { verbose, install_dir } => {
                    if let Some(id) = install_dir {
                        config.install_dir = id;
                    }
                    config.verbose = verbose;
                    cpan_corpus::ratchet(&config)
                }
            }
        }
        Commands::Receipts { tests_only, docs_only, output_dir, test_threads } => {
            receipts::run(receipts::ReceiptsConfig {
                tests_only,
                docs_only,
                output_dir,
                test_threads,
            })
        }
        Commands::AggregateReceipts { check, inputs, output, allow_noop } => {
            aggregate_receipts::run(aggregate_receipts::AggregateReceiptsConfig {
                check,
                inputs,
                output,
                allow_noop,
            })
        }
        Commands::FinalizeCheck { receipt, allow_noop, fail_on_advisory } => {
            finalize_check::run(finalize_check::FinalizeCheckConfig {
                receipt,
                allow_noop,
                fail_on_advisory,
            })
        }
        Commands::MergeReady { command } => match command {
            MergeReadyCommand::Emit { pr, receipt } => merge_ready::emit(pr, receipt),
            MergeReadyCommand::Verify { pr, fixture } => merge_ready::verify(pr, fixture),
            MergeReadyCommand::Reconcile { apply, dry_run } => {
                let run_dry = !apply || dry_run;
                merge_ready::reconcile(run_dry)
            }
            MergeReadyCommand::ReconcileQueue { apply: _, dry_run, pr, receipt } => {
                // Apply is the default. Only switch to dry-run when --dry-run is explicitly passed.
                let do_apply = !dry_run;
                queue_reconciler::reconcile_queue(do_apply, pr, receipt)
            }
        },
        Commands::IgnoredTests { update, check, verbose } => {
            ignored_tests::run(update, check, verbose)
        }
        Commands::DebtReport { check, json, summary, expired, ledger } => {
            debt_report::run(debt_report::DebtReportConfig {
                check,
                json,
                summary,
                expired,
                ledger,
            })
        }
        Commands::DocClaims => doc_claims::run(),
        Commands::IntentDiffGate { pr, fixture, receipt } => {
            intent_diff_gate::run(intent_diff_gate::IntentDiffGateConfig { pr, fixture, receipt })
        }
        Commands::Features { command } => match command {
            FeaturesCommand::SyncDocs => features::sync_docs(),
            FeaturesCommand::Verify => features::verify(),
            FeaturesCommand::Invariants => features::invariants(),
            FeaturesCommand::Report => features::report(),
        },
        Commands::Agent { command } => match command {
            AgentCommand::Lease { command } => match command {
                AgentLeaseCommand::Acquire { task, out } => agent_lease::acquire(&task, &out),
                AgentLeaseCommand::Verify { lease, current } => {
                    agent_lease::verify(&lease, &current)
                }
            },
            AgentCommand::Receipt { command } => match command {
                AgentReceiptCommand::Validate { receipt } => agent_receipt::validate(&receipt),
            },
            AgentCommand::Worktree { command } => worktree_allocator::run(command),
        },
        Commands::FixForward { command } => match command {
            FixForwardCommand::Classify { receipt, output } => {
                fix_forward::classify(receipt, output)
            }
            FixForwardCommand::ListPlaybooks => fix_forward::list_playbooks(),
        },
        Commands::UpdateStatus { write, check, only } => update_status::run(write, check, only),
        Commands::SrpMicrocrates { output } => srp_microcrates::run(output),
        Commands::UnwiredScan { json, check, lsp_crate } => {
            unwired_scan::run(UnwiredScanConfig { lsp_crate, json, check })
        }
        Commands::CheckTestWiring => check_test_wiring::run(),
        Commands::Metrics { command } => match command {
            MetricsCommand::ParserStats { input, json } => metrics::parser_stats::run(input, json),
            MetricsCommand::LspStats { json, receipt_dir } => {
                metrics::lsp_stats::run_with_receipt_dir(json, receipt_dir.as_deref())
            }
            MetricsCommand::WorkspaceStats => metrics::workspace_stats::run(),
            MetricsCommand::DiagnosticsStats => metrics::diagnostics_stats::run(),
            MetricsCommand::Memory => metrics::memory::run(),
            MetricsCommand::ReleaseHealth { days, json } => {
                metrics::release_health::run(days, json)
            }
            MetricsCommand::RatchetCheck { subsystem, current, record } => {
                let root = utils::project_root()?;
                metrics::ratchet::run_ratchet_check(&root, &subsystem, current, record)
            }
            MetricsCommand::PromoteBaseline { subsystem, delta_pct } => {
                let root = utils::project_root()?;
                metrics::ratchet::run_promote_baseline(&root, &subsystem, delta_pct)
            }
            MetricsCommand::SweepStats { input } => metrics::sweep_stats::run(input),
        },
        Commands::UxScorecard { format, input, output, status_md, ratchet_check } => {
            let format = match format {
                UxScorecardOutputFormat::Human => UxScorecardFormat::Human,
                UxScorecardOutputFormat::Json => UxScorecardFormat::Json,
            };
            ux_scorecard::run(format, input, output, status_md, ratchet_check)
        }
        Commands::UxRegressionReceipt { input, receipt, sha } => {
            ux_regression_receipt::run(ux_regression_receipt::UxRegressionReceiptConfig {
                input,
                receipt,
                sha,
            })
        }
        Commands::ValidateMemoryProfiler => compare::validate_memory_profiling(),
        Commands::E2eValidate { workspace_size, report, skip_workspace, skip_bench, verbose } => {
            e2e_validate::run(e2e_validate::E2eConfig {
                workspace_size,
                report_path: report,
                skip_workspace,
                skip_bench,
                verbose,
            })
        }
        Commands::Gates {
            tier,
            gate,
            base,
            list,
            format,
            receipt,
            receipt_path,
            diff,
            fail_fast,
            parallel,
            verbose,
        } => gates::run(gates::GateRunnerConfig {
            tier,
            gate_filter: gate,
            base_ref: base,
            output_format: format,
            emit_receipt: receipt,
            receipt_path,
            diff_baseline: diff,
            list_only: list,
            fail_fast,
            parallel,
            verbose,
        }),
        Commands::GatePolicy { command } => match command {
            GatePolicyCommand::Check => tasks::gate_policy::check(),
            GatePolicyCommand::Effective { profile } => tasks::gate_policy::effective(profile),
        },
        Commands::GateReceipts { command } => match command {
            GateReceiptsCommand::List { format } => {
                gate_receipts::list(convert_gate_receipts_format(format))
                    .map_err(|error| eyre!(error.to_string()))
            }
            GateReceiptsCommand::Validate { path, format } => {
                gate_receipts::validate(&path, convert_gate_receipts_format(format))
                    .map_err(|error| eyre!(error.to_string()))
            }
            GateReceiptsCommand::ValidateAll { dir, format } => {
                gate_receipts::validate_all(&dir, convert_gate_receipts_format(format))
                    .map_err(|error| eyre!(error.to_string()))
            }
        },
        Commands::MethodologyGate { fixture, pr, receipt, dry_run, enforce, format } => {
            methodology_gate::run(methodology_gate::MethodologyGateConfig {
                fixture,
                pr,
                receipt,
                dry_run,
                enforce,
                format,
            })
        }
        Commands::TargetedChecks { base, mode } => targeted_checks::run(base, mode),
        Commands::ResolvePackageName { crate_dir } => {
            // Use the current working directory as workspace root so this subcommand
            // works correctly both in the main workspace and in test synthetic workspaces.
            let root = std::env::current_dir()
                .map_err(|e| eyre!("Failed to get current working directory: {e}"))?;
            let name = tasks::targeted_checks::resolve_single_package_name(&root, &crate_dir)?;
            println!("{name}");
            Ok(())
        }
        Commands::WorktreeCleanup => worktrees::cleanup(),
        Commands::SwarmSummary { ops_dir, since, limit, format } => {
            swarm_summary::run(swarm_summary::SwarmSummaryConfig { ops_dir, since, limit, format })
        }
        Commands::PopulateBook => populate_book::run(),
        Commands::LayerCheck => layer_check::run(),
        Commands::ValidateWorkspaceExclusions => validate_workspace_exclusions::run(),
        Commands::BuildTimingReceipt { clean, incremental, tests, output, baseline } => {
            build_timing::run_receipt(clean, incremental, tests, output, baseline)
        }
        Commands::CompareBuildTiming { baseline, current } => {
            build_timing::run_compare(baseline, current)
        }
        Commands::GeneratedFiles { command } => match command {
            GeneratedFilesCommand::List { fixture } => generated_files::list(fixture),
            GeneratedFilesCommand::Check {
                receipt,
                fixture,
                generator_receipt,
                allow_manual_edits,
            } => generated_files::check(receipt, fixture, generator_receipt, allow_manual_edits),
        },
    }
}

fn print_top_level_commands() {
    let mut command_names = Cli::command()
        .get_subcommands()
        .map(|subcommand| subcommand.get_name().to_string())
        .collect::<Vec<_>>();
    command_names.sort_unstable();

    for command_name in command_names {
        println!("{command_name}");
    }
}

fn convert_gate_receipts_format(format: GateReceiptsFormat) -> gate_receipts::OutputFormat {
    match format {
        GateReceiptsFormat::Human => gate_receipts::OutputFormat::Human,
        GateReceiptsFormat::Json => gate_receipts::OutputFormat::Json,
    }
}
