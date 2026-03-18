# perl-tdd-governance

Standalone SRP microcrate for ignored-test governance in the Perl LSP workspace.

## Overview

This crate owns the ignored-test governance domain model and validation logic:

- **Ignored test inventory** -- category/crate/priority counts with timestamps
- **Baseline management** -- absolute and percentage regression thresholds
- **Quality gates** -- documentation requirements and CI enforcement
- **Trend reporting** -- historical analysis and recommendations

`perl-tdd-support` re-exports this crate under `perl_tdd_support::governance` for backward compatibility.

## Usage

```rust
use perl_tdd_governance::{IgnoredTestGovernance, IgnoredTestGuardian};
# use std::collections::HashMap;
# use std::time::SystemTime;
# let governance = IgnoredTestGovernance {
#     inventory: perl_tdd_governance::IgnoredTestInventory {
#         total_count: 0,
#         by_category: HashMap::new(),
#         by_crate: HashMap::new(),
#         by_priority: HashMap::new(),
#         last_updated: SystemTime::now(),
#     },
#     baseline_management: perl_tdd_governance::BaselineManagement {
#         baseline_count: 0,
#         max_deviation: 0,
#         deviation_threshold_percent: 0.0,
#         baseline_date: SystemTime::now(),
#         next_review_date: SystemTime::now(),
#     },
#     quality_gates: perl_tdd_governance::QualityGates {
#         pre_commit: perl_tdd_governance::PreCommitValidation {
#             require_justification: true,
#             max_new_ignored_per_commit: 1,
#             documentation_requirements: perl_tdd_governance::DocumentationRequirements {
#                 require_issue_reference: true,
#                 require_timeline: true,
#                 require_success_criteria: true,
#                 require_complexity_assessment: true,
#             },
#         },
#         ci_validation: perl_tdd_governance::CiValidation {
#             block_on_count_increase: true,
#             max_ignored_per_crate: HashMap::new(),
#             min_quality_score: 70.0,
#         },
#         metrics_tracking: perl_tdd_governance::MetricsTracking {
#             track_trend: true,
#             trend_window_days: 30,
#             alert_on_negative_trend: true,
#         },
#     },
#     reporting: perl_tdd_governance::ReportingConfiguration {
#         daily_reports: false,
#         weekly_trends: true,
#         monthly_summaries: true,
#         output_formats: vec![perl_tdd_governance::ReportFormat::Json],
#     },
# };
let guardian = IgnoredTestGuardian::new(governance);
# let _ = guardian;
```

Part of the [perl-lsp](https://github.com/EffortlessMetrics/perl-lsp) workspace.
