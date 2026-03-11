//! Deterministic cryptographic fixtures for test code.
//!
//! This module keeps secrets-shaped fixtures out of version control by
//! generating them at runtime from stable seeds.

use anyhow::{Context, Result, anyhow, ensure};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use uselesskey::{Factory, RsaFactoryExt, RsaSpec, Seed};

pub use uselesskey::negative::{CorruptPem, corrupt_pem};

/// Create a stable seed string scoped to a module and test case.
pub fn scoped_seed(scope: &str, case: &str) -> String {
    let scope = scope.trim();
    let case = case.trim();

    match (scope.is_empty(), case.is_empty()) {
        (true, true) => String::new(),
        (false, true) => scope.to_string(),
        (true, false) => case.to_string(),
        (false, false) => format!("{scope}::{case}"),
    }
}

/// Build a deterministic `uselesskey` factory from a seed string.
pub fn deterministic_factory(seed_input: &str) -> Result<Factory> {
    let seed = Seed::from_env_value(seed_input)
        .map_err(|err| anyhow!("failed to derive uselesskey seed: {err}"))?;
    Ok(Factory::deterministic(seed))
}

/// In-memory RSA fixture material suitable for signing and parser/path tests.
#[derive(Clone, Eq, PartialEq)]
pub struct RsaKeyFixture {
    private_key_pkcs8_pem: String,
    public_key_spki_pem: String,
}

impl RsaKeyFixture {
    /// Generate a deterministic RSA-2048 / RS256 fixture.
    pub fn rs256(seed_input: &str, label: &str) -> Result<Self> {
        let factory = deterministic_factory(seed_input)?;
        let keypair = factory.rsa(label, RsaSpec::rs256());

        Ok(Self {
            private_key_pkcs8_pem: keypair.private_key_pkcs8_pem().to_string(),
            public_key_spki_pem: keypair.public_key_spki_pem().to_string(),
        })
    }

    /// Return the PKCS#8 private key PEM.
    pub fn private_key_pkcs8_pem(&self) -> &str {
        &self.private_key_pkcs8_pem
    }

    /// Return the SPKI public key PEM.
    pub fn public_key_spki_pem(&self) -> &str {
        &self.public_key_spki_pem
    }

    /// Produce a deterministically corrupted private-key PEM for sad-path tests.
    pub fn corrupt_private_key_pem(&self, variant: CorruptPem) -> String {
        corrupt_pem(self.private_key_pkcs8_pem(), variant)
    }

    /// Write both PEM files into an existing directory with a predictable stem.
    pub fn write_into<P: AsRef<Path>>(&self, dir: P, stem: &str) -> Result<RsaKeyFixturePaths> {
        ensure!(!stem.trim().is_empty(), "fixture file stem must not be empty");

        let dir = dir.as_ref();
        fs::create_dir_all(dir)
            .with_context(|| format!("failed to create fixture directory {}", dir.display()))?;

        let paths = RsaKeyFixturePaths {
            private_key_pem: dir.join(format!("{stem}.private-key.pem")),
            public_key_pem: dir.join(format!("{stem}.public-key.pem")),
        };

        write_pem_file(&paths.private_key_pem, self.private_key_pkcs8_pem(), "private key")?;
        write_pem_file(&paths.public_key_pem, self.public_key_spki_pem(), "public key")?;

        Ok(paths)
    }
}

impl fmt::Debug for RsaKeyFixture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RsaKeyFixture")
            .field("private_key_pkcs8_pem", &"<redacted>")
            .field("public_key_spki_pem", &"<redacted>")
            .finish()
    }
}

/// Concrete paths written by [`RsaKeyFixture::write_into`].
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RsaKeyFixturePaths {
    /// Filesystem path to the private key PEM.
    pub private_key_pem: PathBuf,
    /// Filesystem path to the public key PEM.
    pub public_key_pem: PathBuf,
}

fn write_pem_file(path: &Path, contents: &str, description: &str) -> Result<()> {
    fs::write(path, contents)
        .with_context(|| format!("failed to write {description} fixture to {}", path.display()))
}
