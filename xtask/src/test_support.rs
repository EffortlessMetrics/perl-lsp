#[cfg(unix)]
use color_eyre::eyre::{Context, Result, eyre};
#[cfg(unix)]
use std::{
    env,
    ffi::OsString,
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::{Mutex, MutexGuard},
};
#[cfg(unix)]
use tempfile::TempDir;

#[cfg(unix)]
static ENV_LOCK: Mutex<()> = Mutex::new(());

#[cfg(unix)]
pub struct FakeCargo {
    _guard: MutexGuard<'static, ()>,
    _dir: TempDir,
    log_path: PathBuf,
    previous_path: Option<OsString>,
    previous_log: Option<OsString>,
    previous_metadata: Option<OsString>,
}

#[cfg(unix)]
impl FakeCargo {
    pub fn install() -> Result<Self> {
        let guard = ENV_LOCK.lock().map_err(|_| eyre!("fake cargo environment lock poisoned"))?;
        let dir = TempDir::new().context("create fake cargo tempdir")?;
        let log_path = dir.path().join("cargo.log");
        let metadata_path = dir.path().join("metadata.json");
        let manifest_path = dir.path().join("Cargo.toml");
        fs::write(
            &manifest_path,
            "[package]\nname = \"fake\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .context("write fake manifest")?;
        let manifest_json =
            serde_json::to_string(&manifest_path.to_string_lossy()).context("encode manifest")?;
        fs::write(
            &metadata_path,
            format!(
                "{{\"packages\":[{{\"id\":\"fake 0.1.0 (path+file:///fake)\",\"name\":\"fake\",\"manifest_path\":{manifest_json}}}],\"workspace_members\":[\"fake 0.1.0 (path+file:///fake)\"]}}"
            ),
        )
        .context("write fake metadata")?;

        let cargo_path = dir.path().join("cargo");
        fs::write(
            &cargo_path,
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"$XTASK_FAKE_CARGO_LOG\"\nif [ \"$1\" = \"metadata\" ]; then\n  cat \"$XTASK_FAKE_CARGO_METADATA\"\n  exit 0\nfi\nexit 0\n",
        )
        .context("write fake cargo script")?;
        let mut permissions =
            fs::metadata(&cargo_path).context("stat fake cargo script")?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&cargo_path, permissions).context("chmod fake cargo script")?;

        let previous_path = env::var_os("PATH");
        let previous_log = env::var_os("XTASK_FAKE_CARGO_LOG");
        let previous_metadata = env::var_os("XTASK_FAKE_CARGO_METADATA");
        let mut path_entries = vec![dir.path().to_path_buf()];
        if let Some(path) = &previous_path {
            path_entries.extend(env::split_paths(path));
        }
        let joined_path = env::join_paths(path_entries).context("join fake cargo PATH")?;

        // SAFETY: this helper holds ENV_LOCK for its lifetime and restores the
        // process environment in Drop before releasing the lock.
        unsafe {
            env::set_var("PATH", joined_path);
            env::set_var("XTASK_FAKE_CARGO_LOG", &log_path);
            env::set_var("XTASK_FAKE_CARGO_METADATA", &metadata_path);
        }

        Ok(Self {
            _guard: guard,
            _dir: dir,
            log_path,
            previous_path,
            previous_log,
            previous_metadata,
        })
    }

    pub fn invocations(&self) -> Vec<String> {
        let raw = fs::read_to_string(&self.log_path).unwrap_or_default();
        raw.lines().map(str::to_string).collect()
    }
}

#[cfg(unix)]
impl Drop for FakeCargo {
    fn drop(&mut self) {
        restore_env("PATH", self.previous_path.as_ref());
        restore_env("XTASK_FAKE_CARGO_LOG", self.previous_log.as_ref());
        restore_env("XTASK_FAKE_CARGO_METADATA", self.previous_metadata.as_ref());
    }
}

#[cfg(unix)]
fn restore_env(key: &str, value: Option<&OsString>) {
    match value {
        Some(value) => {
            // SAFETY: callers restore while FakeCargo holds ENV_LOCK.
            unsafe { env::set_var(key, value) }
        }
        None => {
            // SAFETY: callers restore while FakeCargo holds ENV_LOCK.
            unsafe { env::remove_var(key) }
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::{ENV_LOCK, FakeCargo, restore_env};
    use color_eyre::eyre::{Context, Result, eyre};
    use std::{env, process::Command};

    #[test]
    fn fake_cargo_records_invocations_through_log_path() -> Result<()> {
        let fake_cargo = FakeCargo::install()?;

        let status = Command::new("cargo")
            .args(["fmt", "-p", "xtask"])
            .status()
            .context("run fake cargo")?;

        assert!(status.success(), "fake cargo command should succeed");
        assert_eq!(fake_cargo.invocations(), vec!["fmt -p xtask"]);
        Ok(())
    }

    #[test]
    fn restore_env_removes_vars_that_were_absent() -> Result<()> {
        let _guard = ENV_LOCK.lock().map_err(|_| eyre!("fake cargo environment lock poisoned"))?;
        let key = "XTASK_FAKE_CARGO_RESTORE_NONE_TEST";

        // SAFETY: this test holds ENV_LOCK while mutating process environment.
        unsafe { env::set_var(key, "present") };
        restore_env(key, None);

        assert!(env::var_os(key).is_none(), "restore_env(None) should remove {key}");
        Ok(())
    }
}
