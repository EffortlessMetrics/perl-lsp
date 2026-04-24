use perl_module::resolution::path::resolve_module_path;
use std::path::PathBuf;

#[test]
fn resolves_existing_module_under_include_path() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    let module_file = workspace.join("lib").join("Demo").join("Worker.pm");

    std::fs::create_dir_all(module_file.parent().unwrap_or(&workspace))?;
    std::fs::write(&module_file, "package Demo::Worker; 1;")?;

    let resolved = resolve_module_path(&workspace, "Demo::Worker", &["lib".to_string()]);

    assert_eq!(resolved, Some(module_file));
    Ok(())
}

#[test]
fn returns_lib_fallback_when_no_include_path_matches() {
    let workspace = PathBuf::from("/workspace");
    let resolved =
        resolve_module_path(&workspace, "Missing::Module", &["nonexistent/include".to_string()]);

    assert_eq!(resolved, Some(workspace.join("lib").join("Missing/Module.pm")));
}

#[cfg(windows)]
#[test]
fn normalizes_short_paths_to_long_form_when_available() -> Result<(), Box<dyn std::error::Error>> {
    use std::iter;
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use windows_sys::Win32::Storage::FileSystem::GetShortPathNameW;

    fn to_wide(path: &std::path::Path) -> Vec<u16> {
        path.as_os_str().encode_wide().chain(iter::once(0)).collect::<Vec<u16>>()
    }

    fn get_short_path(path: &std::path::Path) -> Option<PathBuf> {
        let input_wide = to_wide(path);
        // SAFETY: input pointer is valid and NUL-terminated, and output pointer is null for probe call.
        let required_len =
            unsafe { GetShortPathNameW(input_wide.as_ptr(), std::ptr::null_mut(), 0) };
        if required_len == 0 {
            return None;
        }

        let mut short_wide = vec![0_u16; required_len as usize + 1];
        // SAFETY: both pointers are valid for the provided lengths and input is NUL-terminated.
        let written_len = unsafe {
            GetShortPathNameW(input_wide.as_ptr(), short_wide.as_mut_ptr(), short_wide.len() as u32)
        };
        if written_len == 0 {
            return None;
        }

        Some(PathBuf::from(std::ffi::OsString::from_wide(&short_wide[..written_len as usize])))
    }

    let temp = tempfile::tempdir()?;
    let workspace_long = temp.path().join("workspace_with_long_segments");
    let module_file_long = workspace_long.join("lib").join("Demo").join("Worker.pm");

    std::fs::create_dir_all(module_file_long.parent().unwrap_or(&workspace_long))?;
    std::fs::write(&module_file_long, "package Demo::Worker; 1;")?;

    let Some(workspace_short) = get_short_path(&workspace_long) else {
        return Ok(());
    };
    let Some(module_short) = get_short_path(&module_file_long) else {
        return Ok(());
    };
    if workspace_short == workspace_long || module_short == module_file_long {
        return Ok(());
    }

    let resolved = resolve_module_path(&workspace_short, "Demo::Worker", &["lib".to_string()]);
    assert_eq!(resolved, Some(module_file_long));
    Ok(())
}
