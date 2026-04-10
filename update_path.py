import re

with open("crates/perl-module-resolution-path/src/lib.rs", "r") as f:
    content = f.read()

replacement = """pub fn resolve_module_path(
    root: &Path,
    module_name: &str,
    include_paths: &[String],
) -> Option<PathBuf> {
    let relative_path = module_name_to_path(module_name);

    for base_str in include_paths {
        let base_path = Path::new(base_str);

        let (candidate, safe_root) = if base_path.is_absolute() {
            (base_path.join(&relative_path), base_path)
        } else if base_str == "." {
            (root.join(&relative_path), root)
        } else {
            (root.join(base_path).join(&relative_path), root)
        };

        let safe_candidate = match validate_workspace_path(&candidate, safe_root) {
            Ok(path) => path,
            Err(_) => continue,
        };

        if safe_candidate.exists() {
            return Some(safe_candidate);
        }
    }

    Some(root.join("lib").join(relative_path))
}"""

content = re.sub(
    r"pub fn resolve_module_path\(.*?\) -> Option<PathBuf> \{.*?\n    Some\(root\.join\(\"lib\"\)\.join\(relative_path\)\)\n\}",
    replacement,
    content,
    flags=re.DOTALL
)

with open("crates/perl-module-resolution-path/src/lib.rs", "w") as f:
    f.write(content)
