use perl_module::path::module_name_to_path;
use perl_module::resolution::path::resolve_module_path;
use proptest::prelude::*;
use std::path::PathBuf;

fn module_name_strategy() -> impl Strategy<Value = String> {
    proptest::collection::vec("[A-Za-z][A-Za-z0-9_]{0,7}", 1..4)
        .prop_map(|segments| segments.join("::"))
}

proptest! {
    #[test]
    fn resolved_path_starts_with_root(module_name in module_name_strategy()) {
        let root = PathBuf::from("/workspace");
        let include_paths = vec![".".to_string(), "lib".to_string(), "..".to_string()];

        let resolved = resolve_module_path(&root, &module_name, &include_paths)
            .ok_or_else(|| TestCaseError::Fail("module path resolution should succeed".into()))?;

        prop_assert!(resolved.starts_with(&root));
        prop_assert!(resolved.ends_with(module_name_to_path(&module_name)));
    }
}
