mod common;

use perl_semantic_facts::EntityKind;
use perl_workspace::workspace::workspace_index::FileFactShard;
use std::path::PathBuf;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn cpan_style_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("semantic_real_workspace")
        .join("cpan_style")
}

fn use_lib_script_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("multi_file")
        .join("use_lib_script")
}

fn has_entity(shards: &[FileFactShard], name: &str, kind: EntityKind) -> bool {
    shards
        .iter()
        .flat_map(|shard| shard.entities.iter())
        .any(|entity| entity.canonical_name == name && entity.kind == kind)
}

/// (a) Load the cpan_style fixture and assert a known package entity exists.
#[test]
fn harness_cpan_style_package_entity_exists() -> Result<()> {
    let root = cpan_style_root();
    let (_index, shards) = common::load_fixture_workspace(&root)?;

    assert!(has_entity(&shards, "RealBaseline::App", EntityKind::Package));

    Ok(())
}

/// (b) Load the use_lib_script fixture and assert the MyThing package entity exists.
#[test]
fn harness_use_lib_script_my_thing_package_exists() -> Result<()> {
    let root = use_lib_script_root();
    let (_index, shards) = common::load_fixture_workspace(&root)?;

    assert!(has_entity(&shards, "MyThing", EntityKind::Package));

    Ok(())
}

/// (c) Load the use_lib_script fixture and assert the exported subroutine exists.
#[test]
fn harness_use_lib_script_exported_subroutine_exists() -> Result<()> {
    let root = use_lib_script_root();
    let (_index, shards) = common::load_fixture_workspace(&root)?;

    assert!(has_entity(&shards, "MyThing::greet", EntityKind::Subroutine));

    Ok(())
}

/// (d) Assert missing fixture roots fail instead of silently producing no shards.
#[test]
fn harness_missing_fixture_root_is_error() -> Result<()> {
    let missing = use_lib_script_root().join("missing");
    let Err(err) = common::load_fixture_workspace(&missing) else {
        return Err("missing fixture root should be rejected".into());
    };

    let message = err.to_string();
    assert!(
        message.contains("contains no .pm or .pl files"),
        "unexpected missing fixture error: {message}"
    );

    Ok(())
}

/// (e) Assert the entity helper rejects a matching name with the wrong kind.
#[test]
fn harness_assert_entity_exists_rejects_wrong_kind() -> Result<()> {
    let root = use_lib_script_root();
    let (_index, shards) = common::load_fixture_workspace(&root)?;

    assert!(!has_entity(&shards, "MyThing::greet", EntityKind::Package));
    assert!(has_entity(&shards, "MyThing::greet", EntityKind::Subroutine));

    Ok(())
}

/// (f) Assert use_lib_script fixture loads exactly 2 shards (one .pm + one .pl).
#[test]
fn harness_use_lib_script_shard_count_is_two() -> Result<()> {
    let root = use_lib_script_root();
    let (_index, shards) = common::load_fixture_workspace(&root)?;

    assert_eq!(
        shards.len(),
        2,
        "use_lib_script fixture should produce exactly 2 shards (MyThing.pm + main.pl), got {}",
        shards.len()
    );

    Ok(())
}
