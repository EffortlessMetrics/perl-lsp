#![cfg(windows)]

use perl_module::path::canonicalize_path_long_form;

use std::ffi::OsString;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};

use windows_sys::Win32::Storage::FileSystem::GetShortPathNameW;

#[test]
fn canonicalize_long_form_expands_short_path() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let long_dir = temp.path().join("Long Module Directory");
    let module_file = long_dir.join("Long Module Name.pm");

    std::fs::create_dir_all(&long_dir)?;
    std::fs::write(&module_file, "package Long::Module::Name; 1;")?;

    let short_path = short_path_for(&module_file)?;
    let normalized = canonicalize_path_long_form(&short_path);

    assert_eq!(normalized, module_file);
    Ok(())
}

fn short_path_for(path: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let wide_path: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();

    // SAFETY: Input is a valid NUL-terminated UTF-16 buffer and output pointer is null for size query.
    let required_len = unsafe { GetShortPathNameW(wide_path.as_ptr(), std::ptr::null_mut(), 0) };
    if required_len == 0 {
        return Err("GetShortPathNameW length query failed".into());
    }

    let mut output = vec![0_u16; required_len as usize];
    // SAFETY: Input is valid and `output` has space for `required_len` UTF-16 units.
    let written_len =
        unsafe { GetShortPathNameW(wide_path.as_ptr(), output.as_mut_ptr(), required_len) };
    if written_len == 0 {
        return Err("GetShortPathNameW write failed".into());
    }

    Ok(PathBuf::from(OsString::from_wide(&output[..written_len as usize])))
}
