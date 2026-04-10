import re

with open("crates/perl-module-resolution-uri/src/lib.rs", "r") as f:
    content = f.read()

replacement = """        for include_path in include_paths {
            if start_time.elapsed() > timeout {
                return ModuleUriResolution::TimedOut;
            }

            let base_path = PathBuf::from(include_path);
            let (full_path, safe_root) = if base_path.is_absolute() {
                (base_path.join(&relative_path), base_path)
            } else if include_path == "." {
                (workspace_path.join(&relative_path), workspace_path.clone())
            } else {
                (workspace_path.join(&base_path).join(&relative_path), workspace_path.clone())
            };

            let full_path = match validate_workspace_path(&full_path, &safe_root) {
                Ok(path) => path,
                Err(_) => continue,
            };

            if full_path.is_file()
                && let Ok(url) = Url::from_file_path(&full_path)
            {
                return ModuleUriResolution::Resolved(url.to_string());
            }
        }"""

content = re.sub(
    r"        for include_path in include_paths \{\n            if start_time.elapsed\(\) > timeout \{\n                return ModuleUriResolution::TimedOut;\n            \}\n\n            let full_path = if include_path == \"\.\" \{\n                workspace_path\.join\(&relative_path\)\n            \} else \{\n                workspace_path\.join\(include_path\)\.join\(&relative_path\)\n            \};\n\n            let full_path = match validate_workspace_path\(&full_path, &workspace_path\) \{\n                Ok\(path\) => path,\n                Err\(_\) => continue,\n            \};\n\n            if full_path\.is_file\(\)\n                && let Ok\(url\) = Url::from_file_path\(&full_path\)\n            \{\n                return ModuleUriResolution::Resolved\(url\.to_string\(\)\);\n            \}\n        \}",
    replacement,
    content,
    flags=re.DOTALL
)

with open("crates/perl-module-resolution-uri/src/lib.rs", "w") as f:
    f.write(content)
