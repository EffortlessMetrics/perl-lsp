# CI Measurement Scripts

This directory contains scripts for measuring and analyzing CI performance metrics.

## Scripts

### measure-ci-baseline.sh

Collects workflow run data from GitHub Actions and calculates baseline metrics.

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
- [jq](https://stedolan.github.io/jq/) - for JSON processing

**Installation of prerequisites:**

```bash
# macOS
brew install gh jq

# Ubuntu/Debian
sudo apt install gh jq

# Then authenticate
gh auth login
```

**Usage:**

```bash
# Basic usage (analyzes master branch, last 30 days)
./measure-ci-baseline.sh

# Analyze a different branch
./measure-ci-baseline.sh --branch main

# Analyze last 7 days only
./measure-ci-baseline.sh --days 7

# Fetch more runs for higher accuracy
./measure-ci-baseline.sh --limit 500

# Custom output directory
./measure-ci-baseline.sh --output ./reports

# All options
./measure-ci-baseline.sh --branch master --days 30 --limit 200 --output .ci
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
      "billable_minutes": 200
    }
  },
  "summary": {
    "total_runs": 500,
    "total_billable_minutes": 1500,
    "overall_success_rate_percent": 92.5
  }
}
```

## Use Cases

### 1. Establish Baseline Before Optimization

```bash
# Run before making CI changes
./measure-ci-baseline.sh --output .ci/before

# After changes
./measure-ci-baseline.sh --output .ci/after

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
./measure-ci-baseline.sh --branch main --output .ci/main-baseline
./measure-ci-baseline.sh --branch feature-x --output .ci/feature-baseline
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

### "jq: command not found"

Install jq for your platform:
```bash
# macOS
brew install jq

# Ubuntu/Debian  
sudo apt install jq

# RHEL/CentOS
sudo yum install jq
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
