// Test for Task 6: parser_stats.rs top-20 slowest files
//
// CURRENT BEHAVIOR: parser_stats writes top-5 slowest files to .ci/metrics/parser.json
// EXPECTED BEHAVIOR: parser_stats should write top-20 slowest files
//
// The relevant code in parser_stats.rs (line ~183):
//   let slowest: Vec<SlowEntry> = entries
//       .iter()
//       .take(5)   // <-- This should be .take(20)
//       .map(|(name, e)| SlowEntry { ... })
//       .collect();

use xtask::tasks::metrics::parser_stats::{write_json_output, BenchmarkFile, BenchmarkEntry, TimingStat};

#[test]
fn test_slowest_list_contains_top_20() -> xtask::Result<()> {
    use tempfile::TempDir;

    // Create a BenchmarkFile with 25 benchmark entries
    let mut benchmarks = std::collections::BTreeMap::new();

    // Add 25 entries with decreasing mean times
    for i in 0..25 {
        let name = format!("benchmark_{:02}", i);
        let nanoseconds = 100000.0 + (25 - i) as f64 * 10000.0; // 100000 to 340000
        benchmarks.insert(
            name,
            BenchmarkEntry {
                mean: Some(TimingStat {
                    nanoseconds,
                    microseconds: nanoseconds / 1000.0,
                }),
                median: None,
                std_dev: None,
                source_lines: Some(100),
            },
        );
    }

    let file = BenchmarkFile { benchmarks };
    let tmp = TempDir::new()?;
    write_json_output(tmp.path(), &file)?;

    let out_path = tmp.path().join(".ci").join("metrics").join("parser.json");
    let raw = std::fs::read_to_string(&out_path)?;
    let parsed: serde_json::Value = serde_json::from_str(&raw)?;

    let slowest = parsed["metrics"]["slowest"]
        .as_array()
        .expect("slowest must be an array");

    // ASSERTION: slowest list should have 20 entries (not 5!)
    assert_eq!(
        slowest.len(),
        20,
        "slowest list should contain top 20 entries, but got {} entries",
        slowest.len()
    );

    // Verify ordering: first entry should be the slowest (highest mean)
    assert_eq!(
        slowest[0]["name"],
        "benchmark_00",
        "first entry should be the slowest benchmark (benchmark_00)"
    );

    // Last entry should be benchmark_19 (20th slowest)
    assert_eq!(
        slowest[19]["name"],
        "benchmark_19",
        "20th entry should be benchmark_19"
    );

    // benchmark_20 through benchmark_24 should NOT be in the list
    let names: Vec<&str> = slowest
        .iter()
        .filter_map(|v| v["name"].as_str())
        .collect();

    assert!(
        !names.contains(&"benchmark_20"),
        "benchmark_20 should NOT be in top-20"
    );
    assert!(
        !names.contains(&"benchmark_24"),
        "benchmark_24 should NOT be in top-20"
    );

    Ok(())
}
