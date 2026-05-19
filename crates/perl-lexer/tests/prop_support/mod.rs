use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, FileFailurePersistence};

pub fn persisted_config(regress_dir: &'static str, default_cases: u32) -> ProptestConfig {
    ProptestConfig {
        cases: proptest_cases(default_cases),
        failure_persistence: Some(Box::new(FileFailurePersistence::Direct(regress_dir))),
        ..ProptestConfig::default()
    }
}

pub fn mixed_source(max_parts: usize) -> impl Strategy<Value = String> {
    let atom = prop_oneof![
        any::<char>().prop_map(|ch| ch.to_string()),
        Just("\r\n".to_string()),
        Just("\n".to_string()),
        Just("\t".to_string()),
        "[a-zA-Z_][a-zA-Z0-9_]{0,8}".prop_map(|id| format!("my ${id};")),
        "[\\p{L}_][\\p{L}\\p{N}_]{0,6}".prop_map(|id| format!("${id}")),
        "[{}()\\[\\];,=>]{1,6}".prop_map(|text| text.to_string()),
    ];

    prop::collection::vec(atom, 0..max_parts).prop_map(|parts| parts.concat())
}

fn proptest_cases(default_cases: u32) -> u32 {
    std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default_cases)
}
