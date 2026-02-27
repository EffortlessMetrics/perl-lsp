use perl_qualified_name::{split_qualified_name, validate_perl_qualified_name};

fn run_workspace_rename_like_transform(old_name: &str, new_name: &str) -> Option<(String, String)> {
    let (old_package, old_bare) = split_qualified_name(old_name);
    let (new_package, new_bare) = split_qualified_name(new_name);

    if old_package != new_package {
        return None;
    }

    if validate_perl_qualified_name(old_name).is_err()
        || validate_perl_qualified_name(new_name).is_err()
    {
        return None;
    }

    Some((old_bare.to_string(), new_bare.to_string()))
}

#[test]
fn integrates_with_package_scoped_rename_flow() {
    let mapped = run_workspace_rename_like_transform("Utils::process", "Utils::render");
    assert_eq!(mapped, Some(("process".to_string(), "render".to_string())));
}

#[test]
fn rejects_cross_package_rename_without_matching_scopes() {
    assert!(run_workspace_rename_like_transform("Utils::process", "Worker::process").is_none());
}

#[test]
fn supports_toplevel_to_toplevel_without_package_prefix() {
    let mapped = run_workspace_rename_like_transform("process", "render");
    assert_eq!(mapped, Some(("process".to_string(), "render".to_string())));
}
