use std::collections::BTreeMap;

use serde_json::{Value, json};

/// Per-scenario UX measurements for scorecard aggregation.
#[derive(Debug, Clone, Default)]
pub struct ScenarioScore {
    /// Stable scenario identifier (for traceability in generated reports).
    pub scenario_id: String,
    /// Hover response matched expected output.
    pub hover_correct: Option<bool>,
    /// Completion top-1 item matched expected output.
    pub completion_top1_correct: Option<bool>,
    /// Completion top-5 contains expected output.
    pub completion_top5_correct: Option<bool>,
    /// Go-to-definition landed on exact expected location.
    pub definition_exact_hit: Option<bool>,
    /// Symbol search returned correct symbols.
    pub symbol_correct: Option<bool>,
    /// Diagnostics payload was correct after settling.
    pub diagnostics_correct: Option<bool>,
    /// Rename workflow completed with valid workspace edit.
    pub rename_success: Option<bool>,
    /// Cross-file workflow succeeded.
    pub cross_file_success: Option<bool>,
    /// Mean latency per request class in milliseconds for this scenario.
    ///
    /// Keys are request-class names such as `hover`, `completion`,
    /// `definition`, `document_symbols`, `workspace_symbols`, and `diagnostics`.
    pub mean_latency_ms: BTreeMap<String, f64>,
}

/// Aggregated UX scorecard rows suitable for CI artifacts and release notes.
#[derive(Debug, Clone, PartialEq)]
pub struct EditorUxScorecard {
    pub scenario_count: usize,
    pub hover_correctness_pct: Option<f64>,
    pub completion_top1_pct: Option<f64>,
    pub completion_top5_pct: Option<f64>,
    pub definition_exact_hit_pct: Option<f64>,
    pub symbol_correctness_pct: Option<f64>,
    pub diagnostics_correct_pct: Option<f64>,
    pub rename_success_pct: Option<f64>,
    pub cross_file_success_pct: Option<f64>,
    pub mean_latency_ms_by_request: BTreeMap<String, f64>,
}

impl EditorUxScorecard {
    /// Emit machine-consumable JSON payload for CI artifacts.
    pub fn to_json(&self) -> Value {
        json!({
            "schema_version": 1,
            "subsystem": "editor_ux",
            "scenario_count": self.scenario_count,
            "rows": {
                "hover_correctness_pct": self.hover_correctness_pct,
                "completion_top1_pct": self.completion_top1_pct,
                "completion_top5_pct": self.completion_top5_pct,
                "definition_exact_hit_pct": self.definition_exact_hit_pct,
                "symbol_correctness_pct": self.symbol_correctness_pct,
                "diagnostics_correct_pct": self.diagnostics_correct_pct,
                "rename_success_pct": self.rename_success_pct,
                "cross_file_success_pct": self.cross_file_success_pct,
                "mean_latency_ms_by_request": self.mean_latency_ms_by_request,
            }
        })
    }
}

/// Aggregate per-scenario UX measurements into release-facing scorecard rows.
pub fn aggregate_editor_ux_scorecard(scenarios: &[ScenarioScore]) -> EditorUxScorecard {
    let hover_correctness_pct = percent_true(scenarios.iter().filter_map(|s| s.hover_correct));
    let completion_top1_pct =
        percent_true(scenarios.iter().filter_map(|s| s.completion_top1_correct));
    let completion_top5_pct =
        percent_true(scenarios.iter().filter_map(|s| s.completion_top5_correct));
    let definition_exact_hit_pct =
        percent_true(scenarios.iter().filter_map(|s| s.definition_exact_hit));
    let symbol_correctness_pct = percent_true(scenarios.iter().filter_map(|s| s.symbol_correct));
    let diagnostics_correct_pct =
        percent_true(scenarios.iter().filter_map(|s| s.diagnostics_correct));
    let rename_success_pct = percent_true(scenarios.iter().filter_map(|s| s.rename_success));
    let cross_file_success_pct =
        percent_true(scenarios.iter().filter_map(|s| s.cross_file_success));

    let mut latency_accum: BTreeMap<String, (f64, usize)> = BTreeMap::new();
    for scenario in scenarios {
        for (request_class, latency_ms) in &scenario.mean_latency_ms {
            let entry = latency_accum.entry(request_class.clone()).or_insert((0.0, 0));
            entry.0 += latency_ms;
            entry.1 += 1;
        }
    }

    let mean_latency_ms_by_request = latency_accum
        .into_iter()
        .map(|(key, (sum, count))| (key, sum / count as f64))
        .collect::<BTreeMap<_, _>>();

    EditorUxScorecard {
        scenario_count: scenarios.len(),
        hover_correctness_pct,
        completion_top1_pct,
        completion_top5_pct,
        definition_exact_hit_pct,
        symbol_correctness_pct,
        diagnostics_correct_pct,
        rename_success_pct,
        cross_file_success_pct,
        mean_latency_ms_by_request,
    }
}

fn percent_true<I>(iter: I) -> Option<f64>
where
    I: Iterator<Item = bool>,
{
    let mut total = 0usize;
    let mut trues = 0usize;

    for value in iter {
        total += 1;
        if value {
            trues += 1;
        }
    }

    if total == 0 {
        return None;
    }

    Some((trues as f64 / total as f64) * 100.0)
}

#[cfg(test)]
mod tests {
    use super::{ScenarioScore, aggregate_editor_ux_scorecard};
    use anyhow::Result;
    use std::collections::BTreeMap;

    #[test]
    fn aggregate_editor_ux_scorecard_computes_expected_rows() -> Result<()> {
        let scenario_1 = ScenarioScore {
            scenario_id: "hover-and-def".to_string(),
            hover_correct: Some(true),
            completion_top1_correct: Some(false),
            completion_top5_correct: Some(true),
            definition_exact_hit: Some(true),
            symbol_correct: Some(true),
            diagnostics_correct: Some(true),
            rename_success: None,
            cross_file_success: Some(true),
            mean_latency_ms: BTreeMap::from([
                ("hover".to_string(), 12.0),
                ("completion".to_string(), 20.0),
                ("definition".to_string(), 30.0),
            ]),
        };
        let scenario_2 = ScenarioScore {
            scenario_id: "completion-and-cross-file".to_string(),
            hover_correct: Some(false),
            completion_top1_correct: Some(true),
            completion_top5_correct: Some(true),
            definition_exact_hit: Some(false),
            symbol_correct: Some(false),
            diagnostics_correct: Some(false),
            rename_success: Some(true),
            cross_file_success: Some(true),
            mean_latency_ms: BTreeMap::from([
                ("hover".to_string(), 8.0),
                ("completion".to_string(), 40.0),
                ("workspace_symbols".to_string(), 50.0),
            ]),
        };

        let scorecard = aggregate_editor_ux_scorecard(&[scenario_1, scenario_2]);

        assert_eq!(scorecard.scenario_count, 2);
        assert_eq!(scorecard.hover_correctness_pct, Some(50.0));
        assert_eq!(scorecard.completion_top1_pct, Some(50.0));
        assert_eq!(scorecard.completion_top5_pct, Some(100.0));
        assert_eq!(scorecard.definition_exact_hit_pct, Some(50.0));
        assert_eq!(scorecard.symbol_correctness_pct, Some(50.0));
        assert_eq!(scorecard.diagnostics_correct_pct, Some(50.0));
        assert_eq!(scorecard.rename_success_pct, Some(100.0));
        assert_eq!(scorecard.cross_file_success_pct, Some(100.0));
        assert_eq!(scorecard.mean_latency_ms_by_request.get("hover"), Some(&10.0));
        assert_eq!(scorecard.mean_latency_ms_by_request.get("completion"), Some(&30.0));
        assert_eq!(scorecard.mean_latency_ms_by_request.get("definition"), Some(&30.0));
        assert_eq!(scorecard.mean_latency_ms_by_request.get("workspace_symbols"), Some(&50.0));

        let payload = scorecard.to_json();
        assert_eq!(payload["schema_version"], 1);
        assert_eq!(payload["subsystem"], "editor_ux");
        assert_eq!(payload["rows"]["completion_top5_pct"], 100.0);
        assert_eq!(payload["rows"]["symbol_correctness_pct"], 50.0);
        assert_eq!(payload["rows"]["diagnostics_correct_pct"], 50.0);
        assert_eq!(payload["rows"]["rename_success_pct"], 100.0);

        Ok(())
    }

    #[test]
    fn aggregate_editor_ux_scorecard_uses_none_when_metric_not_measured() -> Result<()> {
        let scenario = ScenarioScore {
            scenario_id: "symbols-only".to_string(),
            hover_correct: None,
            completion_top1_correct: None,
            completion_top5_correct: None,
            definition_exact_hit: None,
            symbol_correct: None,
            diagnostics_correct: None,
            rename_success: None,
            cross_file_success: None,
            mean_latency_ms: BTreeMap::from([("document_symbols".to_string(), 18.0)]),
        };

        let scorecard = aggregate_editor_ux_scorecard(&[scenario]);

        assert_eq!(scorecard.hover_correctness_pct, None);
        assert_eq!(scorecard.completion_top1_pct, None);
        assert_eq!(scorecard.completion_top5_pct, None);
        assert_eq!(scorecard.definition_exact_hit_pct, None);
        assert_eq!(scorecard.symbol_correctness_pct, None);
        assert_eq!(scorecard.diagnostics_correct_pct, None);
        assert_eq!(scorecard.rename_success_pct, None);
        assert_eq!(scorecard.cross_file_success_pct, None);
        assert_eq!(scorecard.mean_latency_ms_by_request.get("document_symbols"), Some(&18.0));

        Ok(())
    }

    #[test]
    fn rename_success_aggregates_independently_of_other_metrics() -> Result<()> {
        let scenarios = vec![
            ScenarioScore {
                scenario_id: "rename-pass".to_string(),
                rename_success: Some(true),
                ..Default::default()
            },
            ScenarioScore {
                scenario_id: "rename-fail".to_string(),
                rename_success: Some(false),
                ..Default::default()
            },
            ScenarioScore {
                scenario_id: "no-rename".to_string(),
                hover_correct: Some(true),
                rename_success: None,
                ..Default::default()
            },
        ];
        let scorecard = aggregate_editor_ux_scorecard(&scenarios);
        // Only 2 scenarios measured rename; 1 passed → 50%
        assert_eq!(scorecard.rename_success_pct, Some(50.0));
        // Only 1 scenario measured hover; it passed → 100%
        assert_eq!(scorecard.hover_correctness_pct, Some(100.0));
        Ok(())
    }
}
