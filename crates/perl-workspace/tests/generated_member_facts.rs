//! Workspace fact extraction tests for framework-generated members.

use perl_semantic_facts::{Confidence, EntityKind, Provenance};
use perl_workspace::workspace::workspace_index::WorkspaceIndex;
use std::io;

#[test]
fn moo_has_generates_member_entity_with_attribute_anchor() -> Result<(), Box<dyn std::error::Error>>
{
    let index = WorkspaceIndex::new();
    let uri = "file:///generated-member.pm";
    let source = "\
package Accuracy::GeneratedAccessor;

use Moo;

has name => (is => 'ro');

1;
";
    let expected_start =
        source.find("name =>").ok_or_else(|| io::Error::other("missing attribute name"))?;
    let expected_end = expected_start + "name".len();

    index.index_file(url::Url::parse(uri)?, source.to_string())?;
    let shard = index.file_fact_shard(uri).ok_or_else(|| io::Error::other("missing fact shard"))?;
    let entity = shard
        .entities
        .iter()
        .find(|entity| entity.canonical_name == "Accuracy::GeneratedAccessor::name")
        .ok_or_else(|| io::Error::other("missing generated member entity"))?;

    assert_eq!(entity.kind, EntityKind::GeneratedMember);
    assert_eq!(entity.provenance, Provenance::FrameworkSynthesis);
    assert_eq!(entity.confidence, Confidence::Medium);

    let anchor_id =
        entity.anchor_id.ok_or_else(|| io::Error::other("missing generated member anchor"))?;
    let anchor = shard
        .anchors
        .iter()
        .find(|anchor| anchor.id == anchor_id)
        .ok_or_else(|| io::Error::other("missing anchor fact"))?;
    assert_eq!(anchor.span_start_byte, expected_start as u32);
    assert_eq!(anchor.span_end_byte, expected_end as u32);
    assert_eq!(anchor.provenance, Provenance::FrameworkSynthesis);
    assert_eq!(anchor.confidence, Confidence::Medium);

    Ok(())
}

#[test]
fn plain_has_without_framework_does_not_generate_member_entity()
-> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let uri = "file:///plain-has.pm";
    let source = "\
package Plain::Package;

has name => (is => 'ro');

1;
";

    index.index_file(url::Url::parse(uri)?, source.to_string())?;
    let shard = index.file_fact_shard(uri).ok_or_else(|| io::Error::other("missing fact shard"))?;

    assert!(
        shard.entities.iter().all(|entity| entity.kind != EntityKind::GeneratedMember),
        "plain has without Moo/Moose/Mouse must not synthesize generated member facts"
    );

    Ok(())
}
