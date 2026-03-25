use chrono::{DateTime, Duration as ChronoDuration, Utc};
use color_eyre::eyre::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{self, Value};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::utils::project_root;

const COST_PER_MINUTE: f64 = 0.008;
const MONTHLY_BUDGET_TARGET: f64 = 60.0;
const ANNUAL_BUDGET_TARGET: f64 = 720.0;

#[derive(Deserialize)]
struct RepoInfo {
    owner: RepoOwner,
    name: String,
}

#[derive(Deserialize)]
struct RepoOwner {
    login: String,
}

#[derive(Serialize)]
struct CiCostWorkflow {
    name: String,
    runs: u64,
    total_minutes: u64,
    average_minutes: f64,
    cost: f64,
}

#[derive(Serialize)]
struct CostProjection {
    minutes: u64,
    cost: f64,
    budget_target: f64,
    budget_percentage: f64,
}

#[derive(Serialize)]
struct AnnualProjection {
    cost: f64,
    budget_target: f64,
}

#[derive(Serialize)]
pub struct CiCostReport {
    period_days: u64,
    start_date: String,
    repository: String,
    total_runs: u64,
    successful_runs: u64,
    failed_runs: u64,
    total_minutes: u64,
    total_cost: f64,
    monthly_projection: CostProjection,
    annual_projection: AnnualProjection,
    workflows: Vec<CiCostWorkflow>,
}

#[derive(Default)]
struct CostCounters {
    runs: u64,
    minutes: u64,
    successful_runs: u64,
    failed_runs: u64,
}

#[derive(Default)]
struct BaselineCounters {
    total_runs: u64,
    success_count: u64,
    failure_count: u64,
    skipped_count: u64,
    durations: Vec<u64>,
    billable_minutes: u64,
    name: String,
}

#[derive(Serialize)]
struct BaselineWorkflow {
    name: String,
    total_runs: u64,
    completed_runs: u64,
    success_count: u64,
    failure_count: u64,
    skipped_count: u64,
    success_rate_percent: f64,
    median_duration_seconds: u64,
    p95_duration_seconds: u64,
    avg_duration_seconds: u64,
    billable_minutes: u64,
}

#[derive(Serialize)]
struct BaselineSummary {
    total_runs: u64,
    total_billable_minutes: u64,
    overall_success_rate_percent: f64,
}

#[derive(Serialize)]
struct BaselineReport {
    generated_at: String,
    branch: String,
    days_analyzed: u64,
    workflows: BTreeMap<String, BaselineWorkflow>,
    summary: BaselineSummary,
}

pub fn run_cost_monitor(days: u64, json_output: bool) -> Result<()> {
    let root = project_root()?;
    if days == 0 {
        bail!("--days must be greater than zero");
    }

    run_gh_auth_check(&root)?;
    let repo_info = parse_repo_info(&root)?;
    let repository = format!("{}/{}", repo_info.owner.login, repo_info.name);

    let now = Utc::now();
    let start_time = now - ChronoDuration::days(days as i64);
    let start_date = start_time.format("%Y-%m-%dT%H:%M:%SZ").to_string();

    let runs_json = run_gh_command(
        &root,
        "querying action runs",
        vec![
            "api".to_string(),
            format!("repos/{repository}/actions/runs"),
            "--paginate".to_string(),
            "-X".to_string(),
            "GET".to_string(),
            "-F".to_string(),
            format!("created=>{start_date}"),
            "-F".to_string(),
            "per_page=100".to_string(),
        ],
    )?;

    let mut workflow_stats: HashMap<String, CostCounters> = HashMap::new();
    let mut total_runs = 0_u64;
    let mut successful_runs = 0_u64;
    let mut failed_runs = 0_u64;
    let mut total_minutes = 0_u64;

    for page in serde_json::Deserializer::from_str(&runs_json).into_iter::<Value>() {
        let page = page.context("failed to parse gh api response page")?;

        let runs = if let Some(runs) = page.get("workflow_runs").and_then(Value::as_array) {
            runs
        } else if let Value::Array(values) = &page {
            values
        } else {
            continue;
        };

        for run in runs {
            let created = match read_timestamp(run, &["created_at", "createdAt"]) {
                Some(value) => value,
                None => continue,
            };
            if created < start_time {
                continue;
            }

            let start = read_timestamp(run, &["run_started_at", "runStartedAt"]).unwrap_or(created);
            let end = read_timestamp(run, &["updated_at", "updatedAt"])
                .or_else(|| read_timestamp(run, &["completed_at", "completedAt"]))
                .or_else(|| read_timestamp(run, &["finished_at", "finishedAt"]));

            let elapsed_seconds = match (start, end) {
                (Some(start_ts), Some(end_ts)) => {
                    let elapsed = (end_ts - start_ts).num_seconds();
                    if elapsed > 0 {
                        u64::try_from(elapsed).ok()
                    } else {
                        None
                    }
                }
                _ => None,
            };

            let Some(elapsed_seconds) = elapsed_seconds else {
                continue;
            };
            let elapsed_minutes = elapsed_seconds / 60;

            let workflow_name = run
                .get("name")
                .and_then(Value::as_str)
                .or_else(|| run.get("workflow_name").and_then(Value::as_str))
                .or_else(|| run.get("workflowName").and_then(Value::as_str))
                .unwrap_or("(unknown workflow)")
                .to_string();

            let conclusion = run.get("conclusion").and_then(Value::as_str).unwrap_or("");

            let entry = workflow_stats.entry(workflow_name).or_default();
            entry.runs += 1;
            entry.minutes += elapsed_minutes;

            total_runs += 1;
            total_minutes += elapsed_minutes;

            if conclusion == "success" {
                entry.successful_runs += 1;
                successful_runs += 1;
            } else {
                entry.failed_runs += 1;
                failed_runs += 1;
            }
        }
    }

    if total_runs == 0 {
        if json_output {
            println!(
                "{{\"error\": \"No workflow runs found\", \"period_days\": {days}}}",
            );
        } else {
            println!("No workflow runs found in the last {days} days");
        }
        return Ok(());
    }

    let mut sorted_workflows: Vec<(String, CostCounters)> = workflow_stats
        .into_iter()
        .map(|(name, counters)| (name, counters))
        .collect();

    sorted_workflows.sort_by(|(name_a, counters_a), (name_b, counters_b)| {
        let cost_a = counters_a.minutes as f64 * COST_PER_MINUTE;
        let cost_b = counters_b.minutes as f64 * COST_PER_MINUTE;
        cost_b
            .partial_cmp(&cost_a)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| name_a.cmp(name_b))
    });

    let mut workflow_payloads = Vec::with_capacity(sorted_workflows.len());
    for (name, counters) in sorted_workflows {
        let average_minutes = if counters.runs > 0 {
            counters.minutes as f64 / counters.runs as f64
        } else {
            0.0
        };

        workflow_payloads.push(CiCostWorkflow {
            name,
            runs: counters.runs,
            total_minutes: counters.minutes,
            average_minutes,
            cost: round_two_decimals(counters.minutes as f64 * COST_PER_MINUTE),
        });
    }

    let monthly_minutes = (total_minutes as f64) * 30.0 / (days as f64);
    let monthly_minutes = if monthly_minutes.is_sign_negative() {
        0
    } else {
        monthly_minutes.round() as u64
    };
    let monthly_cost = round_two_decimals(monthly_minutes as f64 * COST_PER_MINUTE);
    let annual_cost = round_two_decimals(monthly_cost * 12.0);
    let total_cost = round_two_decimals(total_minutes as f64 * COST_PER_MINUTE);
    let budget_percentage = if MONTHLY_BUDGET_TARGET == 0.0 {
        0.0
    } else {
        (monthly_cost / MONTHLY_BUDGET_TARGET) * 100.0
    };

    let report = CiCostReport {
        period_days: days,
        start_date,
        repository,
        total_runs,
        successful_runs,
        failed_runs,
        total_minutes,
        total_cost,
        monthly_projection: CostProjection {
            minutes: monthly_minutes,
            cost: monthly_cost,
            budget_target: MONTHLY_BUDGET_TARGET,
            budget_percentage: round_one_decimal(budget_percentage),
        },
        annual_projection: AnnualProjection {
            cost: annual_cost,
            budget_target: ANNUAL_BUDGET_TARGET,
        },
        workflows: workflow_payloads,
    };

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).context("failed to serialize cost report")?
        );
        return Ok(());
    }

    println!("===============================================================================" );
    println!("                    CI Cost Analysis Report");
    println!("===============================================================================");
    println!("Period: Last {days} days (since {})", report.start_date);
    println!("Repository: {}", report.repository);
    println!("===============================================================================");
    println!("                             Summary");
    println!("===============================================================================");
    println!("{:<30} {:>10}", "Total workflow runs:", report.total_runs);
    println!(
        "{:<30} {:>10} ({:.1}%)",
        "  Successful:",
        report.successful_runs,
        percent(report.successful_runs, report.total_runs)
    );
    println!(
        "{:<30} {:>10} ({:.1}%)",
        "  Failed:",
        report.failed_runs,
        percent(report.failed_runs, report.total_runs)
    );
    println!(
        "{:<30} {:>10} minutes",
        "Total CI time:",
        report.total_minutes
    );
    println!("{:<30} ${:.2}", "Total cost:", report.total_cost);

    println!("===============================================================================");
    println!("                     Monthly Projection");
    println!("===============================================================================");
    println!(
        "{:<30} {:>10} minutes",
        "Estimated monthly usage:",
        monthly_minutes
    );
    println!("{:<30} ${:.2}", "Estimated monthly cost:", monthly_cost);
    println!(
        "{:<30} ${:.0}",
        "Monthly budget target:",
        MONTHLY_BUDGET_TARGET
    );
    println!(
        "{:<30} {:.1}%",
        "Budget utilization:",
        budget_percentage
    );
    if monthly_cost <= MONTHLY_BUDGET_TARGET {
        println!("Budget utilization: within budget");
    } else {
        println!("Budget utilization: over budget");
    }

    println!("===============================================================================");
    println!("                      Annual Projection");
    println!("===============================================================================");
    println!("{:<30} ${:.2}", "Estimated annual cost:", annual_cost);
    println!(
        "{:<30} ${:.0}",
        "Annual budget target:",
        ANNUAL_BUDGET_TARGET
    );
    if annual_cost <= ANNUAL_BUDGET_TARGET {
        println!("Annual projection within budget");
    } else {
        let needed = round_two_decimals(annual_cost - ANNUAL_BUDGET_TARGET);
        println!("Annual projection exceeds budget by ${needed}");
    }

    println!("===============================================================================");
    println!("                   Per-Workflow Breakdown");
    println!("===============================================================================");
    println!(
        "{:<35} {:>8} {:>12} {:>12} {:>10}",
        "Workflow", "Runs", "Total Min", "Avg Min", "Cost"
    );
    println!("-------------------------------------------------------------------------------");
    for workflow in &report.workflows {
        println!(
            "{:<35} {:>8} {:>12} {:>12.1} ${:>9.2}",
            workflow.name,
            workflow.runs,
            workflow.total_minutes,
            workflow.average_minutes,
            workflow.cost
        );
    }

    println!("===============================================================================");
    println!("                        Recommendations");
    println!("===============================================================================");
    println!();

    if let Some(most_expensive) = report
        .workflows
        .iter()
        .max_by(|a, b| a.cost.partial_cmp(&b.cost).unwrap_or(std::cmp::Ordering::Equal))
    {
        let share = if total_cost > 0.0 {
            round_one_decimal(most_expensive.cost * 100.0 / total_cost)
        } else {
            0.0
        };
        println!(
            "1. Most Expensive Workflow: '{}' costs ${:.2} ({share:.1}% of total)",
            most_expensive.name, most_expensive.cost
        );
    }

    if failed_runs > 0 {
        println!(
            "2. Failed Runs: {} failed runs ({:.1}% failure rate)",
            failed_runs,
            percent(failed_runs, report.total_runs)
        );
    }

    println!("3. Concurrency Cancellation:");
    println!("   Use cancel-in-progress for all scheduled workflows.");
    println!("4. Caching Strategy:");
    println!("   Add or tune caching for expensive build/test steps.");
    println!("5. Gating:");
    println!("   Gate expensive workflows behind labels or workflow_dispatch where possible.");

    Ok(())
}

pub fn run_ci_baseline(
    branch: String,
    days: u64,
    limit: usize,
    output_dir: PathBuf,
) -> Result<()> {
    let root = project_root()?;
    if days == 0 {
        bail!("--days must be greater than zero");
    }

    run_gh_auth_check(&root)?;

    let runs_json = run_gh_command(
        &root,
        "listing workflow runs",
        vec![
            "run".to_string(),
            "list".to_string(),
            "--limit".to_string(),
            limit.to_string(),
            "--branch".to_string(),
            branch.clone(),
            "--json".to_string(),
            "name,conclusion,createdAt,updatedAt,databaseId,workflowName,status,runStartedAt"
                .to_string(),
        ],
    )?;

    let runs: Vec<Value> = serde_json::from_str(&runs_json)
        .context("failed to parse JSON output from gh run list")?;

    let cutoff = Utc::now() - ChronoDuration::days(days as i64);
    let mut workflow_counters: BTreeMap<String, BaselineCounters> = BTreeMap::new();

    for run in &runs {
        let created = match read_timestamp(run, &["createdAt", "created_at"]) {
            Some(value) => value,
            None => continue,
        };
        if created < cutoff {
            continue;
        }

        let workflow_name = run
            .get("workflowName")
            .and_then(Value::as_str)
            .or_else(|| run.get("name").and_then(Value::as_str))
            .unwrap_or("(unknown workflow)");

        let conclusion = run.get("conclusion").and_then(Value::as_str).unwrap_or("");

        let mut duration_seconds = 0_u64;
        let start = read_timestamp(run, &["runStartedAt", "run_started_at"])
            .or_else(|| read_timestamp(run, &["createdAt", "created_at"]));
        let end = read_timestamp(run, &["updatedAt", "updated_at"]);
        if let (Some(start_ts), Some(end_ts)) = (start, end) {
            let elapsed = (end_ts - start_ts).num_seconds();
            if elapsed > 0 {
                duration_seconds = u64::try_from(elapsed).unwrap_or_default();
            }
        }

        let key = workflow_key(workflow_name);
        let counters = workflow_counters.entry(key).or_default();
        counters.name = workflow_name.to_string();
        counters.total_runs += 1;

        match conclusion {
            "success" => counters.success_count += 1,
            "failure" => counters.failure_count += 1,
            "skipped" => counters.skipped_count += 1,
            _ => {}
        }

        if conclusion == "skipped" {
            continue;
        }

        if duration_seconds > 0 {
            counters.durations.push(duration_seconds);
            counters.billable_minutes += (duration_seconds + 59) / 60;
        }
    }

    if workflow_counters.is_empty() {
        println!("No workflow runs found in requested period");
        return Ok(());
    }

    let mut workflow_reports = BTreeMap::new();
    for (key, counters) in workflow_counters {
        let mut durations = counters.durations.clone();
        durations.sort_unstable();

        let median_seconds = percentile(&durations, 50.0);
        let p95_seconds = percentile(&durations, 95.0);
        let avg_seconds = if durations.is_empty() {
            0
        } else {
            let sum: u64 = durations.iter().sum();
            sum / u64::try_from(durations.len()).unwrap_or(1)
        };

        let completed_runs = counters.success_count + counters.failure_count;
        let success_rate_percent = if completed_runs > 0 {
            (counters.success_count as f64 * 100.0) / (completed_runs as f64)
        } else {
            0.0
        };

        workflow_reports.insert(
            key,
            BaselineWorkflow {
                name: counters.name,
                total_runs: counters.total_runs,
                completed_runs,
                success_count: counters.success_count,
                failure_count: counters.failure_count,
                skipped_count: counters.skipped_count,
                success_rate_percent,
                median_duration_seconds: median_seconds,
                p95_duration_seconds: p95_seconds,
                avg_duration_seconds: avg_seconds,
                billable_minutes: counters.billable_minutes,
            },
        );
    }

    let total_runs: u64 = workflow_reports
        .values()
        .map(|workflow| workflow.total_runs)
        .sum();

    let total_billable: u64 = workflow_reports
        .values()
        .map(|workflow| workflow.billable_minutes)
        .sum();

    let total_success: u64 = workflow_reports.values().map(|workflow| workflow.success_count).sum();
    let total_completed: u64 = workflow_reports
        .values()
        .map(|workflow| workflow.success_count + workflow.failure_count)
        .sum();

    let overall_success_rate_percent = if total_completed > 0 {
        (total_success as f64 * 100.0) / (total_completed as f64)
    } else {
        0.0
    };

    let report = BaselineReport {
        generated_at: Utc::now().to_rfc3339(),
        branch: branch.clone(),
        days_analyzed: days,
        workflows: workflow_reports,
        summary: BaselineSummary {
            total_runs,
            total_billable_minutes: total_billable,
            overall_success_rate_percent,
        },
    };

    let output_dir = root.join(output_dir);
    fs::create_dir_all(&output_dir).context("failed to create output directory")?;

    let json_path = output_dir.join("ci_baseline.json");
    fs::write(
        &json_path,
        serde_json::to_string_pretty(&report).context("failed to serialize baseline report")?,
    )
    .with_context(|| format!("failed to write {}", json_path.display()))?;

    let md_path = output_dir.join("ci_baseline.md");
    let markdown = build_baseline_markdown(&report)?;
    fs::write(&md_path, markdown).with_context(|| format!("failed to write {}", md_path.display()))?;

    println!("");
    println!("======================================");
    println!("CI Baseline Summary");
    println!("======================================");
    println!("Branch:              {}", report.branch);
    println!("Analysis period:     Last {} days", report.days_analyzed);
    println!("Total runs:          {}", report.summary.total_runs);
    println!("Total billable:      {}m", report.summary.total_billable_minutes);
    println!(
        "Overall success:     {:.1}%",
        report.summary.overall_success_rate_percent
    );
    println!("Output JSON:         {}", json_path.display());
    println!("Output markdown:     {}", md_path.display());
    println!("======================================");

    Ok(())
}

fn run_gh_auth_check(root: &Path) -> Result<()> {
    let status = Command::new("gh")
        .current_dir(root)
        .args(["auth", "status"])
        .status()
        .context("failed to run gh auth status")?;

    if !status.success() {
        bail!("gh CLI is not authenticated. Run 'gh auth login'.");
    }

    Ok(())
}

fn parse_repo_info(root: &Path) -> Result<RepoInfo> {
    let repo_json = run_gh_command(root, "reading repository info", vec![
        "repo".to_string(),
        "view".to_string(),
        "--json".to_string(),
        "owner,name".to_string(),
    ])?;

    serde_json::from_str(&repo_json).context("failed to parse gh repo view JSON")
}

fn run_gh_command(root: &Path, action: &str, args: Vec<String>) -> Result<String> {
    let output = Command::new("gh")
        .current_dir(root)
        .args(&args)
        .output()
        .with_context(|| format!("failed to execute gh command while {action}"))?;

    if !output.status.success() {
        bail!(
            "gh command failed while {action}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(String::from_utf8(output.stdout).context("gh output was not valid UTF-8")?)
}

fn read_timestamp(run: &Value, keys: &[&str]) -> Option<DateTime<Utc>> {
    for key in keys {
        let value = run.get(*key).and_then(Value::as_str)?;
        if let Ok(timestamp) = DateTime::parse_from_rfc3339(value) {
            return Some(timestamp.with_timezone(&Utc));
        }
    }
    None
}

fn round_two_decimals(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn round_one_decimal(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

fn percent(part: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        round_one_decimal((part as f64 * 100.0) / (total as f64))
    }
}

fn percentile(values: &[u64], percentile: f64) -> u64 {
    if values.is_empty() {
        return 0;
    }

    let index = (((values.len() - 1) as f64) * (percentile / 100.0)).floor() as usize;
    values[index]
}

fn workflow_key(name: &str) -> String {
    let mut key = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            key.push(ch);
        } else if ch == ' ' {
            key.push('_');
        }
    }

    if key.is_empty() {
        "workflow".to_string()
    } else {
        key
    }
}

fn build_baseline_markdown(report: &BaselineReport) -> Result<String> {
    let mut out = String::new();
    out.push_str("# CI Baseline Metrics Report\n\n");
    out.push_str(&format!(
        "**Generated:** {}\n",
        report.generated_at.replace('T', " ")
    ));
    out.push_str(&format!("**Branch:** {}\n", report.branch));
    out.push_str(&format!("**Analysis Period:** Last {} days\n\n", report.days_analyzed));

    out.push_str("## Summary\n\n");
    out.push_str("| Metric | Value |\n|--------|-------|\n");
    out.push_str(&format!("| Total Runs | {} |\n", report.summary.total_runs));
    out.push_str(&format!(
        "| Overall Success Rate | {:.1}% |\n",
        report.summary.overall_success_rate_percent
    ));
    out.push_str(&format!(
        "| Total Billable Minutes | {}m |\n\n",
        report.summary.total_billable_minutes
    ));

    out.push_str("## Workflow Details\n\n");
    out.push_str("| Workflow | Runs | Success Rate | Median | P95 | Billable |\n");
    out.push_str("|----------|------|--------------|--------|-----|----------|\n");

    for workflow in report.workflows.values() {
        out.push_str(&format!(
            "| {} | {} | {:.1}% | {}s | {}s | {}m |\n",
            workflow.name,
            workflow.completed_runs,
            workflow.success_rate_percent,
            workflow.median_duration_seconds,
            workflow.p95_duration_seconds,
            workflow.billable_minutes
        ));
    }

    out.push_str("\n## Notes\n\n");
    out.push_str("- Median Duration: 50th percentile of run duration (in seconds)\n");
    out.push_str("- P95 Duration: 95th percentile of run duration (in seconds)\n");
    out.push_str(
        "- Billable Minutes: Estimated billable time (each run rounded up to nearest minute)\n",
    );
    out.push_str("- Success Rate: Calculated excluding skipped runs\n\n");

    out.push_str("## Recommendations\n\n");
    out.push_str("1. Monitor P95 durations for workflow variance.\n");
    out.push_str("2. Track success-rate trends over time and investigate failures.\n");
    out.push_str("3. Focus optimization on high billable workflows.\n\n");
    out.push_str("---\nGenerated by cargo xtask ci-baseline\n");

    Ok(out)
}
