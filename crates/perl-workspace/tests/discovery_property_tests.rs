//! Property tests for workspace discovery invariants.

use perl_workspace::discovery::{discover_perl_files, is_perl_discovery_path};
use perl_workspace::ignore::path_contains_skipped_component;
use proptest::prelude::*;
use proptest::test_runner::Config as ProptestConfig;
use std::collections::HashSet;
use std::fs;

fn extension_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("pl".to_string()),
        Just("PL".to_string()),
        Just("pm".to_string()),
        Just("Pm".to_string()),
        Just("t".to_string()),
        Just("T".to_string()),
        Just("psgi".to_string()),
        Just("PSGI".to_string()),
        Just("i".to_string()),
        Just("I".to_string()),
        Just("xs".to_string()),
        Just("XS".to_string()),
        Just("ep".to_string()),
        Just("EP".to_string()),
        Just("tt".to_string()),
        Just("TT".to_string()),
        Just("tt2".to_string()),
        Just("TT2".to_string()),
        Just("md".to_string()),
        Just("txt".to_string()),
        Just("json".to_string()),
    ]
}

fn supported_extension_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("pl".to_string()),
        Just("pm".to_string()),
        Just("t".to_string()),
        Just("psgi".to_string()),
        Just("i".to_string()),
        Just("xs".to_string()),
        Just("ep".to_string()),
        Just("tt".to_string()),
        Just("tt2".to_string()),
    ]
}

fn randomize_case(input: &str, uppercase: &[bool]) -> String {
    input
        .chars()
        .zip(uppercase.iter().copied())
        .map(
            |(ch, use_upper)| {
                if use_upper { ch.to_ascii_uppercase() } else { ch.to_ascii_lowercase() }
            },
        )
        .collect()
}

proptest! {
    #![proptest_config(ProptestConfig {
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn prop_discovery_returns_all_and_only_perl_files(
        specs in prop::collection::vec(("[a-z]{1,10}", extension_strategy()), 1..24)
    ) {
        let tmp = match tempfile::tempdir() {
            Ok(dir) => dir,
            Err(_) => return Ok(()),
        };
        let root = tmp.path();

        let mut expected = HashSet::new();

        for (idx, (stem, ext)) in specs.iter().enumerate() {
            let relative = format!("src/file_{idx}_{stem}.{ext}");
            let path = root.join(relative);

            let parent = match path.parent() {
                Some(parent) => parent,
                None => {
                    prop_assert!(false, "path.parent() returned None for joined path: {:?}", path);
                    return Ok(());
                }
            };

            prop_assert!(fs::create_dir_all(parent).is_ok());
            prop_assert!(fs::write(&path, "# generated\n").is_ok());

            if is_perl_discovery_path(&path) {
                expected.insert(path);
            }
        }

        let result = discover_perl_files(root);
        let discovered: HashSet<_> = result.files.iter().cloned().collect();

        // No duplicates in discovery output.
        prop_assert_eq!(discovered.len(), result.files.len());

        for path in &discovered {
            prop_assert!(path.starts_with(root));
            prop_assert!(is_perl_discovery_path(path));
        }

        prop_assert_eq!(discovered, expected);
    }

    #[test]
    fn prop_discovery_never_returns_skipped_directories(stem in "[a-z]{3,12}") {
        let tmp = match tempfile::tempdir() {
            Ok(dir) => dir,
            Err(_) => return Ok(()),
        };
        let root = tmp.path();

        let skipped_dirs = [
            ".git",
            ".hg",
            ".svn",
            "target",
            "node_modules",
            ".cache",
            "blib",
            "local",
            "vendor",
        ];

        for directory in skipped_dirs {
            let path = root.join(directory).join(format!("{stem}.pm"));
            if let Some(parent) = path.parent() {
                prop_assert!(fs::create_dir_all(parent).is_ok());
            }
            prop_assert!(fs::write(path, "# skipped\n").is_ok());
        }

        let visible = root.join(format!("lib/{stem}.pm"));
        if let Some(parent) = visible.parent() {
            prop_assert!(fs::create_dir_all(parent).is_ok());
        }
        prop_assert!(fs::write(&visible, "# visible\n").is_ok());

        let result = discover_perl_files(root);

        for path in &result.files {
            let relative = match path.strip_prefix(root) {
                Ok(relative) => relative,
                Err(_) => {
                    prop_assert!(false, "discovered path is outside workspace root: {:?}", path);
                    return Ok(());
                }
            };
            prop_assert!(!path_contains_skipped_component(relative));
        }

        prop_assert!(result.files.iter().any(|path| path.ends_with(&visible)));
    }

    #[test]
    fn prop_discovery_output_is_lexically_sorted(
        specs in prop::collection::vec(("[a-z]{1,8}", extension_strategy()), 1..36)
    ) {
        let tmp = match tempfile::tempdir() {
            Ok(dir) => dir,
            Err(_) => return Ok(()),
        };
        let root = tmp.path();

        for (idx, (stem, ext)) in specs.iter().enumerate() {
            let nested = idx % 3;
            let relative = format!("lib/{nested}/f_{stem}_{idx}.{ext}");
            let path = root.join(relative);
            if let Some(parent) = path.parent() {
                prop_assert!(fs::create_dir_all(parent).is_ok());
            }
            prop_assert!(fs::write(path, "# generated\n").is_ok());
        }

        let result = discover_perl_files(root);
        let mut expected = result.files.clone();
        expected.sort_unstable_by(|left, right| left.as_os_str().cmp(right.as_os_str()));
        prop_assert_eq!(result.files, expected);
    }

    #[test]
    fn prop_supported_extensions_are_case_insensitive(
        stem in "[a-z]{1,10}",
        base_ext in supported_extension_strategy(),
        case_mask in prop::collection::vec(any::<bool>(), 2..8)
    ) {
        let tmp = match tempfile::tempdir() {
            Ok(dir) => dir,
            Err(_) => return Ok(()),
        };
        let root = tmp.path();

        let mask_len = base_ext.len();
        let mut normalized_mask = vec![false; mask_len];
        for (idx, value) in case_mask.into_iter().cycle().take(mask_len).enumerate() {
            normalized_mask[idx] = value;
        }

        let ext = randomize_case(&base_ext, &normalized_mask);
        let path = root.join(format!("lib/{stem}.{ext}"));
        if let Some(parent) = path.parent() {
            prop_assert!(fs::create_dir_all(parent).is_ok());
        }
        prop_assert!(fs::write(&path, "# generated\n").is_ok());

        let result = discover_perl_files(root);
        prop_assert!(result.files.iter().any(|candidate| candidate == &path));
    }
}
