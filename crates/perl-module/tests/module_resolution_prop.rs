use perl_module::resolution::{ModuleUriResolution, resolve_module_uri};
use proptest::prelude::*;
use std::time::Duration;

fn module_name_strategy() -> impl Strategy<Value = String> {
    proptest::collection::vec("[A-Za-z_][A-Za-z0-9_]{0,7}", 1..5)
        .prop_map(|segments| segments.join("::"))
}

proptest! {
    #[test]
    fn open_document_precedence_is_deterministic(module_name in module_name_strategy(), prefix in "[a-z]{1,8}") {
        let rel = format!("{}.pm", module_name.replace("::", "/"));
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
