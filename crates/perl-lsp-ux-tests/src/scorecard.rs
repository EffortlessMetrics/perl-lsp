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
    /// Symbol query returned the expected symbol.
    pub symbol_correct: Option<bool>,
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
    let symbol_correctness_pct =
        percent_true(scenarios.iter().filter_map(|s| s.symbol_correct));
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
    use super::{ScenarioScore, aggregate_editor_ux_scorecard, percent_true};
    use anyhow::Result;
    use std::collections::BTreeMap;

    fn make_score(
        id: &str,
        hover: Option<bool>,
        top1: Option<bool>,
        top5: Option<bool>,
        def: Option<bool>,
        sym: Option<bool>,
        cross: Option<bool>,
        latency: &[(&str, f64)],
    ) -> ScenarioScore {
        ScenarioScore {
            scenario_id: id.to_string(),
            hover_correct: hover,
            completion_top1_correct: top1,
            completion_top5_correct: top5,
            definition_exact_hit: def,
            symbol_correct: sym,
            cross_file_success: cross,
            mean_latency_ms: latency
                .iter()
                .map(|(k, v)| ((*k).to_string(), *v))
                .collect(),
        }
    }

    #[test]
    fn aggregate_editor_ux_scorecard_computes_expected_rows() -> Result<()> {
        let s1 = make_score(
            "hover-and-def",
            Some(true), Some(false), Some(true), Some(true), Some(true), Some(true),
            &[("hover", 12.0), ("completion", 20.0), ("definition", 30.0)],
        );
        let s2 = make_score(
            "completion-and-cross-file",
            Some(false), Some(true), Some(true), Some(false), Some(false), Some(true),
            &[("hover", 8.0), ("completion", 40.0), ("workspace_symbols", 50.0)],
        );

        let scorecard = aggregate_editor_ux_scorecard(&[s1, s2]);

        assert_eq!(scorecard.scenario_count, 2);
        assert_eq!(scorecard.hover_correctness_pct, Some(50.0));
        assert_eq!(scorecard.completion_top1_pct, Some(50.0));
        assert_eq!(scorecard.completion_top5_pct, Some(100.0));
        assert_eq!(scorecard.definition_exact_hit_pct, Some(50.0));
        assert_eq!(scorecard.symbol_correctness_pct, Some(50.0));
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

        Ok(())
    }

    #[test]
    fn aggregate_editor_ux_scorecard_uses_none_when_metric_not_measured() -> Result<()> {
        let s = make_score(
            "symbols-only",
            None, None, None, None, None, None,
            &[("document_symbols", 18.0)],
        );

        let scorecard = aggregate_editor_ux_scorecard(&[s]);

        assert_eq!(scorecard.hover_correctness_pct, None);
        assert_eq!(scorecard.completion_top1_pct, None);
        assert_eq!(scorecard.completion_top5_pct, None);
        assert_eq!(scorecard.definition_exact_hit_pct, None);
        assert_eq!(scorecard.symbol_correctness_pct, None);
        assert_eq!(scorecard.cross_file_success_pct, None);
        assert_eq!(scorecard.mean_latency_ms_by_request.get("document_symbols"), Some(&18.0));

        let payload = scorecard.to_json();
        assert!(payload["rows"]["symbol_correctness_pct"].is_null());

        Ok(())
    }

    #[test]
    fn empty_scenarios_produces_zero_count_and_all_none() -> Result<()> {
        let scorecard = aggregate_editor_ux_scorecard(&[]);

        assert_eq!(scorecard.scenario_count, 0);
        assert_eq!(scorecard.hover_correctness_pct, None);
        assert_eq!(scorecard.completion_top1_pct, None);
        assert_eq!(scorecard.completion_top5_pct, None);
        assert_eq!(scorecard.definition_exact_hit_pct, None);
        assert_eq!(scorecard.symbol_correctness_pct, None);
        assert_eq!(scorecard.cross_file_success_pct, None);
        assert!(scorecard.mean_latency_ms_by_request.is_empty());

        Ok(())
    }

    #[test]
    fn all_false_correctness_produces_zero_pct() -> Result<()> {
        let s = make_score(
            "all-fail",
            Some(false), Some(false), Some(false), Some(false), Some(false), Some(false),
            &[],
        );

        let scorecard = aggregate_editor_ux_scorecard(&[s]);

        assert_eq!(scorecard.hover_correctness_pct, Some(0.0));
        assert_eq!(scorecard.completion_top1_pct, Some(0.0));
        assert_eq!(scorecard.completion_top5_pct, Some(0.0));
        assert_eq!(scorecard.definition_exact_hit_pct, Some(0.0));
        assert_eq!(scorecard.symbol_correctness_pct, Some(0.0));
        assert_eq!(scorecard.cross_file_success_pct, Some(0.0));

        Ok(())
    }

    #[test]
    fn single_scenario_all_true_produces_100_pct() -> Result<()> {
        let s = make_score(
            "all-pass",
            Some(true), Some(true), Some(true), Some(true), Some(true), Some(true),
            &[("hover", 15.0), ("completion", 25.0)],
        );

        let scorecard = aggregate_editor_ux_scorecard(&[s]);

        assert_eq!(scorecard.scenario_count, 1);
        assert_eq!(scorecard.hover_correctness_pct, Some(100.0));
        assert_eq!(scorecard.completion_top1_pct, Some(100.0));
        assert_eq!(scorecard.completion_top5_pct, Some(100.0));
        assert_eq!(scorecard.definition_exact_hit_pct, Some(100.0));
        assert_eq!(scorecard.symbol_correctness_pct, Some(100.0));
        assert_eq!(scorecard.cross_file_success_pct, Some(100.0));
        assert_eq!(scorecard.mean_latency_ms_by_request.get("hover"), Some(&15.0));
        assert_eq!(scorecard.mean_latency_ms_by_request.get("completion"), Some(&25.0));

        Ok(())
    }

    #[test]
    fn symbol_correctness_aggregates_across_scenarios() -> Result<()> {
        // 3 scenarios: true, true, false → 66.666...%
        let scenarios: Vec<ScenarioScore> = vec![
            make_score("s1", None, None, None, None, Some(true), None, &[]),
            make_score("s2", None, None, None, None, Some(true), None, &[]),
            make_score("s3", None, None, None, None, Some(false), None, &[]),
        ];

        let scorecard = aggregate_editor_ux_scorecard(&scenarios);

        let pct = scorecard.symbol_correctness_pct;
        assert!(pct.is_some_and(|v| (v - 66.666_666).abs() < 0.01), "expected ~66.67%, got {pct:?}");

        Ok(())
    }

    #[test]
    fn latency_aggregation_with_no_latency_data_is_empty() -> Result<()> {
        let s = make_score("no-latency", Some(true), None, None, None, None, None, &[]);

        let scorecard = aggregate_editor_ux_scorecard(&[s]);

        assert!(scorecard.mean_latency_ms_by_request.is_empty());

        Ok(())
    }

    #[test]
    fn latency_averages_across_same_request_class() -> Result<()> {
        let s1 = make_score("a", None, None, None, None, None, None, &[("hover", 10.0)]);
        let s2 = make_score("b", None, None, None, None, None, None, &[("hover", 30.0)]);
        let s3 = make_score("c", None, None, None, None, None, None, &[("hover", 20.0)]);

        let scorecard = aggregate_editor_ux_scorecard(&[s1, s2, s3]);

        // mean of 10 + 30 + 20 = 20.0
        assert_eq!(scorecard.mean_latency_ms_by_request.get("hover"), Some(&20.0));

        Ok(())
    }

    #[test]
    fn to_json_schema_version_and_subsystem_are_correct() {
        let scorecard = aggregate_editor_ux_scorecard(&[]);
        let payload = scorecard.to_json();

        assert_eq!(payload["schema_version"], 1);
        assert_eq!(payload["subsystem"], "editor_ux");
        assert_eq!(payload["scenario_count"], 0);
    }

    #[test]
    fn percent_true_returns_none_on_empty_iterator() {
        assert_eq!(percent_true(std::iter::empty::<bool>()), None);
    }

    #[test]
    fn percent_true_all_false_returns_zero() {
        assert_eq!(percent_true([false, false, false].iter().copied()), Some(0.0));
    }

    #[test]
    fn percent_true_all_true_returns_100() {
        assert_eq!(percent_true([true, true, true].iter().copied()), Some(100.0));
    }

    #[test]
    fn percent_true_single_true_returns_100() {
        assert_eq!(percent_true([true].iter().copied()), Some(100.0));
    }

    #[test]
    fn percent_true_single_false_returns_zero() {
        assert_eq!(percent_true([false].iter().copied()), Some(0.0));
    }

    #[test]
    fn percent_true_mixed_returns_correct_ratio() {
        // 2 true out of 5 = 40%
        let result = percent_true([true, false, true, false, false].iter().copied());
        assert_eq!(result, Some(40.0));
    }
}
