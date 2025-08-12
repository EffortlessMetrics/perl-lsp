#![allow(dead_code)]

use std::{env, ffi::OsString};

/// Guard for temporarily setting environment variables in tests
/// Automatically restores the original value on drop
pub struct EnvGuard {
    key: String,
    prev: Option<OsString>,
}

impl EnvGuard {
    /// Set an environment variable and return a guard that will restore it
    pub fn set<K: Into<String>, V: AsRef<str>>(key: K, value: V) -> Self {
        let key = key.into();
        let prev = env::var_os(&key);
        unsafe { env::set_var(&key, value.as_ref()) };
        EnvGuard { key, prev }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match self.prev.take() {
            Some(v) => unsafe { env::set_var(&self.key, v) },
            None => unsafe { env::remove_var(&self.key) },
        }
    }
}