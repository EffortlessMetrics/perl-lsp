# Implementation Checklist — Scorecard Metrics Ratchet Infrastructure (#4105)

## Step 1: Create Baseline JSON Files
**Files**: `.ci/metrics/baselines/parser.json`, `.ci/metrics/baselines/engineering_health.json`
**Action**: CREATE — new JSON files with floor/improvement metrics
**Dependencies**: None (static data files)
**Source baseline data from**: Existing `.ci/parser-corpus-baseline.json` and `.ci/cpan-corpus-baseline.json` (do not hand-wave)

**parser.json content** (~25 lines):
```json
{
  "schema_version": 1,
  "measured_at": "<current ISO-8601 timestamp>",
  "subsystem": "parser",
  "commit": "<current master SHA>",
  "floor_metrics": {
    "system_clean_rate": 0.971,
    "cpan_clean_rate": 0.953,
    "system_crash_count": 0,
    "system_files_unreadable": 48,
    "system_total_error_nodes": 604,
    "cpan_total_error_nodes": 3015,
    "strict_clean_subset_pass_rate": 1.0
  },
  "improvement_metrics": {
    "node_kind_coverage": 0.941,
    "error_density_per_1k_loc": null,
    "recovery_salvage_rate": null
  },
  "tolerance_pct": 0.005
}
```

**engineering_health.json content** (~20 lines): mostly null improvement_metrics, strict_clean_subset_pass_rate at 1.0

**Verify**: `jq . .ci/metrics/baselines/parser.json` parses without error

---

## Step 2: Create Metrics Module Structure
**Files**: 
- `xtask/src/tasks/metrics/mod.rs` (new)
- `xtask/src/tasks/metrics/ratchet.rs` (~150 lines)
- `xtask/src/tasks/metrics/stable_wins.rs` (~80 lines)

**Action**: CREATE — three new Rust files

**ratchet.rs public API**:
```rust
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsystemBaseline {
    pub schema_version: u32,
    pub measured_at: String,
    pub subsystem: String,
    pub commit: String,
    pub floor_metrics: BTreeMap<String, Option<f64>>,
    pub improvement_metrics: BTreeMap<String, Option<f64>>,
    #[serde(default = "default_tolerance")]
    pub tolerance_pct: f64,
}

#[derive(Debug, Clone)]
pub struct RatchetViolation {
    pub metric: String,
    pub baseline_value: f64,
    pub current_value: f64,
    pub regression_pct: f64,
}

pub fn load_baseline(repo_root: &Path, subsystem: &str) -> Result<SubsystemBaseline>
pub fn check_floor_metrics(
    baseline: &SubsystemBaseline,
    current: &BTreeMap<String, Option<f64>>,
) -> Vec<RatchetViolation>
```

**stable_wins.rs public API**:
```rust
pub const STABLE_WIN_THRESHOLD: usize = 3;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StableWinsState {
    pub subsystem: String,
    pub recent_runs: BTreeMap<String, Vec<MetricRun>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricRun {
    pub commit: String,
    pub value: f64,
    pub timestamp: String,
}

pub fn record_run(state: &mut StableWinsState, commit: &str, timestamp: &str, metrics: &BTreeMap<String, Option<f64>>)
pub fn stable_improvements(state: &StableWinsState, baseline: &BTreeMap<String, Option<f64>>, material_delta_pct: f64) -> Vec<String>
```

**mod.rs**: Two lines exporting the modules

**Verify**: `cargo build -p xtask` compiles without error

---

## Step 3: Update xtask/src/tasks/mod.rs
**File**: `xtask/src/tasks/mod.rs`
**Action**: ADD one line: `pub mod metrics;`
**Dependencies**: Step 2 complete
**Verify**: `cargo build -p xtask` still compiles

---

## Step 4: Add Commands to xtask/src/main.rs
**File**: `xtask/src/main.rs`
**Action**: ADD two enum variants to Commands + handler implementations (~40 lines total)

**Commands enum additions**:
```rust
/// Check scorecard floor metrics against committed baseline.
MetricsRatchetCheck {
    subsystem: String,
    /// Defaults to target/receipts/metrics/<subsystem>.json
    #[arg(long)]
    current: Option<PathBuf>,
    /// Record this run in target/metrics/stable_wins/<subsystem>.json
    #[arg(long)]
    record: bool,
},
/// Show which improvement metrics are stable enough to raise the floor baseline.
MetricsPromoteBaseline {
    subsystem: String,
    #[arg(long, default_value_t = 0.01)]
    delta_pct: f64,
},
```

**MetricsRatchetCheck handler logic** (~20 lines):
1. Load baseline via `load_baseline(repo_root, &subsystem)`
2. Load current metrics from `--current` or default path; fallback to existing `read_sweep_report()` 
3. Call `check_floor_metrics()` — on violations, print each and exit with code 1
4. If `--record`, call `record_run()` and write to `target/metrics/stable_wins/<subsystem>.json`
5. Print improvement metric summary (informational)

**MetricsPromoteBaseline handler logic** (~15 lines):
1. Load stable-wins state from `target/metrics/stable_wins/<subsystem>.json`
2. Load current baseline
3. Call `stable_improvements()`
4. Print list of metrics ready to promote

**Verify**: `cargo build -p xtask` compiles, `cargo xtask metrics --help` shows both commands

---

## Step 5: Wire into justfile
**File**: `justfile`
**Action**: ADD two recipes and update ci-gate target (~15 lines)

**New recipes**:
```makefile
ci-metrics-ratchet:
    @echo "Checking scorecard floor metrics..."
    @cargo xtask metrics ratchet-check parser
    @cargo xtask metrics ratchet-check engineering_health
    @echo "Scorecard ratchet passed"

pr-fast-metrics-ratchet:
    @cargo xtask metrics ratchet-check parser --current .ci/metrics/baselines/parser.json
```

**Update ci-gate target** (around line 724): Add `just ci-metrics-ratchet &&` before final `exit 0`

**Verify**: `just ci-metrics-ratchet` succeeds, `just pr-fast` succeeds, `just ci-gate` includes ratchet (will show ratchet pass if baseline is current)

---

## Step 6: Update .ci/gate-policy.yaml
**File**: `.ci/gate-policy.yaml`
**Action**: ADD entry for ci-metrics-ratchet under workflow_integration.job_mapping.ci-gate (~5 lines)

**Content**:
```yaml
- job_id: ci-metrics-ratchet
  job_name: "ci-metrics-ratchet"
  required: true
  run_condition: "on every merge"
```

**Verify**: YAML syntax via `yq . .ci/gate-policy.yaml` without error

---

## Step 7: Create Scorecard Labels (Operational Step)
**File**: None (gh CLI commands)
**Action**: Run gh label create commands (~7 labels)

```bash
gh label create "scorecard/parser"              --color "fbca04" --description "Issue tracked by parser scorecard"
gh label create "scorecard/diagnostics"         --color "fbca04" --description "Issue tracked by diagnostics scorecard"
gh label create "scorecard/editor-intelligence" --color "fbca04" --description "Issue tracked by editor intelligence scorecard"
gh label create "scorecard/module-resolution"   --color "fbca04" --description "Issue tracked by @INC scorecard"
gh label create "scorecard/workspace"           --color "fbca04" --description "Issue tracked by workspace/indexing scorecard"
gh label create "scorecard/dap"                 --color "fbca04" --description "Issue tracked by DAP scorecard"
gh label create "scorecard/engineering-health"  --color "fbca04" --description "Issue tracked by engineering health scorecard"
```

**Apply retroactively**: 
```bash
gh issue edit 3496  --add-label "scorecard/parser"
gh issue edit 3499  --add-label "scorecard/parser"
gh issue edit 3485  --add-label "scorecard/diagnostics"
gh issue edit 3489  --add-label "scorecard/diagnostics"
gh issue edit 3472  --add-label "scorecard/module-resolution"
gh issue edit 3475  --add-label "scorecard/module-resolution"
gh issue edit 3476  --add-label "scorecard/module-resolution"
gh issue edit 3482  --add-label "scorecard/module-resolution"
gh issue edit 3513  --add-label "scorecard/workspace"
gh issue edit 3515  --add-label "scorecard/workspace"
```

**Verify**: `gh label list --json name | jq '.[] | select(.name | startswith("scorecard"))' | wc -l` returns 7

---

## Summary

| File | Action | Lines | Step |
|------|--------|-------|------|
| `.ci/metrics/baselines/parser.json` | CREATE | 25 | 1 |
| `.ci/metrics/baselines/engineering_health.json` | CREATE | 20 | 1 |
| `xtask/src/tasks/metrics/mod.rs` | CREATE | 5 | 2 |
| `xtask/src/tasks/metrics/ratchet.rs` | CREATE | 150 | 2 |
| `xtask/src/tasks/metrics/stable_wins.rs` | CREATE | 80 | 2 |
| `xtask/src/tasks/mod.rs` | ADD | 1 | 3 |
| `xtask/src/main.rs` | ADD | 40 | 4 |
| `justfile` | ADD | 15 | 5 |
| `.ci/gate-policy.yaml` | ADD | 5 | 6 |
| gh label create (7 labels) | RUN | 0 | 7 |

**Total scope**: ~340 lines across 8 files + 7 operational label-creation commands

**Compilation gates** (verify at each step):
- Step 2: `cargo build -p xtask`
- Step 3: `cargo build -p xtask`
- Step 4: `cargo build -p xtask`, `cargo xtask metrics --help`
- Step 5: `just ci-metrics-ratchet`, `just pr-fast`, `just ci-gate`
- Step 6: YAML syntax `yq .` 
- Step 7: `gh label list`

**Builder next**: Implement Steps 1-6 in order, execute verifications, run test suite, commit and push.
