# CI Measurement Scripts

This directory contains scripts for measuring and analyzing CI performance metrics.

## Scripts

### measure-ci-baseline.sh

Collects workflow run data from GitHub Actions and calculates baseline metrics via
`cargo xtask ci-baseline` (the underlying implementation), with this wrapper
script retained as a convenience entrypoint.

**Features:**
- Fetches workflow runs from specified branch
- Calculates per-workflow statistics:
  - Median duration
  - P95 (95th percentile) duration
  - Success rate (excluding skipped runs)
  - Approximate billable minutes
- Outputs both JSON and Markdown reports

**Prerequisites:**
- [GitHub CLI (gh)](https://cli.github.com/) - installed and authenticated
- Rust toolchain - for the workspace xtask binary
- `cargo` - comes with Rust, used to run `cargo xtask`

**Installation of prerequisites:**

```bash
# macOS
brew install gh

# Ubuntu/Debian
sudo apt install gh

# Authenticate once for CLI queries
gh auth login
```

**Usage (canonical):**

```bash
# Basic usage (analyzes master branch, last 30 days)
cargo xtask ci-baseline

# Analyze a different branch
cargo xtask ci-baseline --branch main

# Analyze last 7 days only
cargo xtask ci-baseline --days 7

# Fetch more runs for higher accuracy
cargo xtask ci-baseline --limit 500

# Custom output directory
cargo xtask ci-baseline --output ./reports

# All options
cargo xtask ci-baseline --branch master --days 30 --limit 200 --output .ci
```

**Usage (wrapper shim):**

```bash
# Run the shim script (delegates to `cargo xtask ci-baseline`)
./measure-ci-baseline.sh
```

**Options:**

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--branch` | `-b` | master | Branch to analyze |
| `--days` | `-d` | 30 | Number of days to look back |
| `--limit` | `-l` | 200 | Maximum runs to fetch |
| `--output` | `-o` | .ci | Output directory |
| `--help` | `-h` | - | Show help message |

**Output Files:**

| File | Description |
|------|-------------|
| `ci_baseline.json` | Machine-readable metrics in JSON format (generated on demand) |
| `ci_baseline.md` | Human-readable Markdown report (generated on demand) |

## Output Format

### JSON Schema

```json
{
  "generated_at": "2024-01-15T10:30:00Z",
  "branch": "master",
  "days_analyzed": 30,
  "workflows": {
    "Workflow_Name": {
      "name": "Workflow Name",
      "total_runs": 100,
      "completed_runs": 85,
      "success_count": 80,
      "failure_count": 5,
      "skipped_count": 15,
      "success_rate_percent": 94.1,
      "median_duration_seconds": 120,
      "p95_duration_seconds": 300,
      "avg_duration_seconds": 150,
      "billable_minutes": 200,
      "unique_catch_count": 3,
      "unique_catch_rate_percent": 60.0,
      "signal_per_dollar": 1.88
    }
  },
  "summary": {
    "total_runs": 500,
    "total_billable_minutes": 1500,
    "overall_success_rate_percent": 92.5,
    "total_unique_catches": 9,
    "overall_signal_per_dollar": 0.75
  }
}
```

## Use Cases

### 1. Establish Baseline Before Optimization

```bash
# Run before making CI changes
cargo xtask ci-baseline --output .ci/before

# After changes
cargo xtask ci-baseline --output .ci/after

# Compare the JSON files to measure improvement
```

### 2. Weekly CI Health Check

```bash
# Add to a weekly cron job or scheduled workflow
./measure-ci-baseline.sh --days 7 --output .ci/weekly/$(date +%Y-%W)
```

### 3. PR Impact Analysis

```bash
# Compare feature branch to main
cargo xtask ci-baseline --branch main --output .ci/main-baseline
cargo xtask ci-baseline --branch feature-x --output .ci/feature-baseline
```

## Interpreting Results

### Success Rate

| Rate | Status | Action |
|------|--------|--------|
| > 95% | Healthy | Monitor |
| 85-95% | Warning | Investigate failures |
| < 85% | Critical | Immediate attention |

### Duration Variance (P95 / Median)

| Ratio | Interpretation |
|-------|----------------|
| < 1.5 | Consistent performance |
| 1.5-2.0 | Some variability |
| > 2.0 | High variability, investigate |

### Billable Minutes

Use this to:
- Track CI costs over time
- Identify expensive workflows for optimization
- Set budget alerts

## Troubleshooting

### "gh is not authenticated"

```bash
gh auth login
# Follow the prompts to authenticate
```

### No workflow runs found

- Verify the branch name exists
- Check if workflows are configured for that branch
- Increase the `--days` parameter

## Contributing

When adding new measurement scripts:

1. Follow the naming convention: `measure-*.sh` or `analyze-*.sh`
2. Include usage documentation in the script header
3. Output both JSON (machine-readable) and Markdown (human-readable)
4. Update this README with script documentation
