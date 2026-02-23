use perl_module_path::module_name_to_path;
use perl_module_resolution::{ModuleUriResolution, resolve_module_path, resolve_module_uri};
use proptest::prelude::*;
use std::path::PathBuf;
use std::time::Duration;

fn module_name_strategy() -> impl Strategy<Value = String> {
    proptest::collection::vec("[A-Za-z_][A-Za-z0-9_]{0,7}", 1..5)
        .prop_map(|segments| segments.join("::"))
}

proptest! {
    #[test]
    fn fallback_resolution_stays_within_root(module_name in module_name_strategy()) {
        let root = PathBuf::from("/workspace");
        let include_paths = vec![".".to_string(), "lib".to_string(), "..".to_string()];

        let resolved = resolve_module_path(&root, &module_name, &include_paths);
        prop_assert!(resolved.is_some());

        let path = match resolved {
            Some(path) => path,
            None => return Ok(()),
        };

        let expected_suffix = module_name_to_path(&module_name);
        prop_assert!(path.starts_with(&root));
        prop_assert!(path.to_string_lossy().ends_with(&expected_suffix));
    }

    #[test]
    fn open_document_precedence_is_deterministic(module_name in module_name_strategy(), prefix in "[a-z]{1,8}") {
        let rel = module_name_to_path(&module_name);
        let open_uri = format!("file:///open/{prefix}/{rel}");

        let result = resolve_module_uri(
            &module_name,
            std::slice::from_ref(&open_uri),
            &["file:///workspace".to_string()],
            &["lib".to_string()],
            false,
            &[],
            Duration::from_millis(20),
        );

        prop_assert_eq!(result, ModuleUriResolution::Resolved(open_uri));
    }
}
