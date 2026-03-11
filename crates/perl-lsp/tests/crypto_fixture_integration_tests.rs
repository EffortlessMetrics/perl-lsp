mod support;

use perl_tdd_support::crypto::{CorruptPem, RsaKeyFixture, scoped_seed};
use support::TempWorkspace;

#[test]
fn writes_deterministic_rsa_fixtures_into_temp_workspace() -> Result<(), Box<dyn std::error::Error>>
{
    let seed = scoped_seed(module_path!(), "writes_deterministic_rsa_fixtures_into_temp_workspace");

    let first = RsaKeyFixture::rs256(&seed, "perl-lsp-test-signer")?;
    let second = RsaKeyFixture::rs256(&seed, "perl-lsp-test-signer")?;

    assert_eq!(first, second, "same seed + label should generate identical runtime fixtures");

    let workspace = TempWorkspace::new().map_err(std::io::Error::other)?;
    let fixture_dir = workspace.dir.path().join("test-fixtures/crypto");
    let paths = first.write_into(&fixture_dir, "signing")?;

    assert!(paths.private_key_pem.starts_with(workspace.dir.path()));
    assert!(paths.public_key_pem.starts_with(workspace.dir.path()));

    let private_key = std::fs::read_to_string(&paths.private_key_pem)?;
    let public_key = std::fs::read_to_string(&paths.public_key_pem)?;

    assert!(
        private_key.contains("-----BEGIN PRIVATE KEY-----"),
        "private key fixture should be written as PKCS#8 PEM"
    );
    assert!(
        public_key.contains("-----BEGIN PUBLIC KEY-----"),
        "public key fixture should be written as SPKI PEM"
    );

    let bad_private_key = first.corrupt_private_key_pem(CorruptPem::BadHeader);
    assert_ne!(
        bad_private_key,
        first.private_key_pkcs8_pem(),
        "negative fixture should differ from the valid PEM"
    );
    assert!(
        bad_private_key.contains("CORRUPTED")
            || !bad_private_key.starts_with("-----BEGIN PRIVATE KEY-----"),
        "negative fixture should visibly break the PEM header"
    );

    Ok(())
}
