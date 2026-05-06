//! Generated member extraction from Moo/Moose/Class::Tiny `has` declarations and
//! `Class::Accessor` declarations.
//!
//! Leverages the existing [`ClassModelBuilder`] to parse `has` declarations,
//! then maps each [`Attribute`] to one or more [`GeneratedMember`] entries
//! based on the accessor mode (`is` option).
//!
//! # Mapping Rules
//!
//! | `has` declaration                        | Generated members                |
//! |------------------------------------------|----------------------------------|
//! | `has 'x'` (no `is`)                     | Accessor (`x`)                   |
//! | `has 'x' => (is => 'rw')`               | Accessor (`x`)                  |
//! | `has 'x' => (is => 'ro')`               | Getter (`x`)                     |
//! | `has 'x' => (is => 'lazy')`             | Getter (`x`)                     |
//! | `has 'x' => (is => 'bare')`             | *(none)*                         |
//! | `has 'x' => (predicate => 1)`           | Predicate (`has_x`)              |
//! | `has 'x' => (clearer => 1)`             | Clearer (`clear_x`)              |
//! | `has 'x' => (builder => 1)`             | Builder (`_build_x`)             |
//!
//! All emitted members carry `provenance = FrameworkSynthesis` and
//! `confidence = Medium`.

use crate::analysis::class_model::{
    AccessorType, ClassAccessorMode, ClassModel, ClassModelBuilder, Framework,
};
use crate::ast::Node;
use perl_semantic_facts::{
    AnchorId, Confidence, EntityId, FileId, GeneratedMember, GeneratedMemberKind, Provenance,
};

/// Extractor that walks an AST to produce [`GeneratedMember`] entries for
/// each supported framework declaration found.
pub struct GeneratedMemberExtractor;

impl GeneratedMemberExtractor {
    /// Walk the entire AST and return [`GeneratedMember`] entries for each
    /// accessor, getter, setter, predicate, clearer, or builder generated
    /// by Moo/Moose/Mouse `has` declarations or Class::Accessor calls.
    ///
    /// The `package` parameter is used as a fallback when the class model
    /// does not provide a package name. Typically this is the file-level
    /// package or `"main"`.
    pub fn extract(ast: &Node, package: &str, _file_id: FileId) -> Vec<GeneratedMember> {
        let models = ClassModelBuilder::new().build(ast);
        Self::extract_from_models(&models, package)
    }

    /// Convert already-built class models into generated member facts.
    ///
    /// This lets [`crate::analysis::semantic::SemanticAnalyzer`] reuse its
    /// class-model pass rather than rebuilding framework metadata.
    pub fn extract_from_models(models: &[ClassModel], package: &str) -> Vec<GeneratedMember> {
        let mut members = Vec::new();

        for model in models {
            let pkg = if model.name.is_empty() { package } else { &model.name };

            if is_accessor_framework(model.framework) {
                collect_has_members(model, pkg, &mut members);
            }

            if model.framework == Framework::ClassAccessor {
                collect_class_accessor_members(model, pkg, &mut members);
            }
        }

        members
    }
}

/// Returns `true` for frameworks that generate accessors from `has` declarations.
fn is_accessor_framework(framework: Framework) -> bool {
    matches!(framework, Framework::Moo | Framework::Moose | Framework::Mouse | Framework::ClassTiny)
}

fn collect_has_members(model: &ClassModel, package: &str, members: &mut Vec<GeneratedMember>) {
    for attr in &model.attributes {
        let anchor_id = AnchorId(attr.location.start as u64);

        // Primary accessor/getter/setter based on `is` option.
        match attr.is {
            None => {
                // Bare `has 'x'` with no `is` — Moo/Moose generates a
                // combined accessor by default.
                members.push(make_member(
                    &attr.accessor_name,
                    GeneratedMemberKind::Accessor,
                    anchor_id,
                    package,
                ));
            }
            Some(AccessorType::Rw) => {
                // `is => 'rw'` — combined getter/setter method.
                members.push(make_member(
                    &attr.accessor_name,
                    GeneratedMemberKind::Accessor,
                    anchor_id,
                    package,
                ));
            }
            Some(AccessorType::Ro | AccessorType::Lazy) => {
                // `is => 'ro'` or `is => 'lazy'` — getter only.
                members.push(make_member(
                    &attr.accessor_name,
                    GeneratedMemberKind::Getter,
                    anchor_id,
                    package,
                ));
            }
            Some(AccessorType::Bare) => {
                // `is => 'bare'` — no accessor generated.
            }
        }

        // Predicate method (e.g. `has_x`).
        if let Some(pred_name) = &attr.predicate {
            members.push(make_member(
                pred_name,
                GeneratedMemberKind::Predicate,
                anchor_id,
                package,
            ));
        }

        // Clearer method (e.g. `clear_x`).
        if let Some(clear_name) = &attr.clearer {
            members.push(make_member(clear_name, GeneratedMemberKind::Clearer, anchor_id, package));
        }

        // Builder method (e.g. `_build_x`).
        if let Some(builder_name) = &attr.builder {
            members.push(make_member(
                builder_name,
                GeneratedMemberKind::Builder,
                anchor_id,
                package,
            ));
        }
    }
}

fn collect_class_accessor_members(
    model: &ClassModel,
    package: &str,
    members: &mut Vec<GeneratedMember>,
) {
    for method in &model.methods {
        let Some(accessor_mode) = method.accessor_mode else {
            continue;
        };
        if !method.synthetic {
            continue;
        }

        members.push(make_member(
            &method.name,
            class_accessor_kind(accessor_mode),
            AnchorId(method.location.start as u64),
            package,
        ));
    }
}

fn class_accessor_kind(mode: ClassAccessorMode) -> GeneratedMemberKind {
    match mode {
        ClassAccessorMode::Rw => GeneratedMemberKind::Accessor,
        ClassAccessorMode::Ro => GeneratedMemberKind::Getter,
        ClassAccessorMode::Wo => GeneratedMemberKind::Setter,
    }
}

/// Build a [`GeneratedMember`] with deterministic entity ID derived from
/// the package name, member name, and source anchor.
fn make_member(
    name: &str,
    kind: GeneratedMemberKind,
    source_anchor_id: AnchorId,
    package: &str,
) -> GeneratedMember {
    // Deterministic entity ID: hash of (package, name, anchor offset).
    let entity_id = deterministic_entity_id(package, name, source_anchor_id);
    GeneratedMember::new(
        entity_id,
        name.to_string(),
        kind,
        source_anchor_id,
        package.to_string(),
        Provenance::FrameworkSynthesis,
        Confidence::Medium,
    )
}

/// Produce a deterministic [`EntityId`] from the package name, member name,
/// and source anchor offset. Uses a simple FNV-1a–style hash to avoid
/// pulling in extra dependencies.
fn deterministic_entity_id(package: &str, name: &str, anchor_id: AnchorId) -> EntityId {
    // FNV-1a 64-bit
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0100_0000_01b3;

    let mut hash = FNV_OFFSET;
    for byte in package.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    // Separator to avoid collisions between "Foo" + "bar" and "Foob" + "ar".
    hash ^= 0xFF;
    hash = hash.wrapping_mul(FNV_PRIME);
    for byte in name.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash ^= anchor_id.0;
    hash = hash.wrapping_mul(FNV_PRIME);

    EntityId(hash)
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;

    /// Parse Perl source and extract generated members.
    fn parse_and_extract(code: &str) -> Vec<GeneratedMember> {
        let mut parser = Parser::new(code);
        let ast = match parser.parse() {
            Ok(ast) => ast,
            Err(_) => return Vec::new(),
        };
        GeneratedMemberExtractor::extract(&ast, "main", FileId(1))
    }

    /// Helper: find all members with a given name.
    fn members_named<'a>(members: &'a [GeneratedMember], name: &str) -> Vec<&'a GeneratedMember> {
        members.iter().filter(|m| m.name == name).collect()
    }

    // ── has 'x' (bare, no is) → Accessor ───────────────────────────────

    #[test]
    fn bare_has_generates_accessor() -> Result<(), String> {
        let code = "package MyApp::User;\nuse Moo;\nhas 'username';\n1;";
        let members = parse_and_extract(code);
        let matched = members_named(&members, "username");
        let member = matched.first().ok_or("expected a GeneratedMember for 'username'")?;

        assert_eq!(member.kind, GeneratedMemberKind::Accessor);
        assert_eq!(member.package, "MyApp::User");
        assert_eq!(member.provenance, Provenance::FrameworkSynthesis);
        assert_eq!(member.confidence, Confidence::Medium);
        Ok(())
    }

    // ── has 'x' => (is => 'rw') → Accessor ─────────────────────────────

    #[test]
    fn rw_has_generates_accessor() -> Result<(), String> {
        let code = "package MyApp::User;\nuse Moose;\nhas 'email' => (is => 'rw');\n1;";
        let members = parse_and_extract(code);
        let matched = members_named(&members, "email");
        assert_eq!(matched.len(), 1, "expected one accessor, got {}", matched.len());
        assert_eq!(matched[0].kind, GeneratedMemberKind::Accessor);
        assert_eq!(matched[0].provenance, Provenance::FrameworkSynthesis);
        assert_eq!(matched[0].confidence, Confidence::Medium);
        Ok(())
    }

    // ── has 'x' => (is => 'ro') → Getter only ──────────────────────────

    #[test]
    fn ro_has_generates_getter_only() -> Result<(), String> {
        let code = "package MyApp::User;\nuse Moo;\nhas 'name' => (is => 'ro');\n1;";
        let members = parse_and_extract(code);
        let matched = members_named(&members, "name");
        assert_eq!(matched.len(), 1, "expected one member, got {}", matched.len());

        let member = matched[0];
        assert_eq!(member.kind, GeneratedMemberKind::Getter);
        assert_eq!(member.provenance, Provenance::FrameworkSynthesis);
        assert_eq!(member.confidence, Confidence::Medium);
        Ok(())
    }

    // ── has 'x' => (is => 'lazy') → Getter only ────────────────────────

    #[test]
    fn lazy_has_generates_getter_only() -> Result<(), String> {
        let code = "package MyApp::Config;\nuse Moo;\nhas 'settings' => (is => 'lazy');\n1;";
        let members = parse_and_extract(code);
        let matched = members_named(&members, "settings");
        assert_eq!(matched.len(), 1, "expected one member, got {}", matched.len());
        assert_eq!(matched[0].kind, GeneratedMemberKind::Getter);
        Ok(())
    }

    // ── has 'x' => (is => 'bare') → no accessor ────────────────────────

    #[test]
    fn bare_is_generates_no_accessor() -> Result<(), String> {
        let code = "package MyApp::Internal;\nuse Moose;\nhas '_data' => (is => 'bare');\n1;";
        let members = parse_and_extract(code);
        let matched = members_named(&members, "_data");
        assert!(matched.is_empty(), "expected no members for bare accessor, got {matched:?}");
        Ok(())
    }

    // ── Predicate, clearer, builder ─────────────────────────────────────

    #[test]
    fn predicate_clearer_builder_generated() -> Result<(), String> {
        let code = r#"
package MyApp::User;
use Moose;
has 'nickname' => (
    is        => 'rw',
    predicate => 1,
    clearer   => 1,
    builder   => 1,
);
1;
"#;
        let members = parse_and_extract(code);

        let pred = members.iter().find(|m| m.name == "has_nickname");
        let pred = pred.ok_or("expected predicate 'has_nickname'")?;
        assert_eq!(pred.kind, GeneratedMemberKind::Predicate);

        let clear = members.iter().find(|m| m.name == "clear_nickname");
        let clear = clear.ok_or("expected clearer 'clear_nickname'")?;
        assert_eq!(clear.kind, GeneratedMemberKind::Clearer);

        let builder = members.iter().find(|m| m.name == "_build_nickname");
        let builder = builder.ok_or("expected builder '_build_nickname'")?;
        assert_eq!(builder.kind, GeneratedMemberKind::Builder);
        Ok(())
    }

    // ── Custom predicate/clearer/builder names ──────────────────────────

    #[test]
    fn custom_predicate_clearer_builder_names() -> Result<(), String> {
        let code = r#"
package MyApp::User;
use Moo;
has 'age' => (
    is        => 'ro',
    predicate => 'has_user_age',
    clearer   => 'reset_age',
    builder   => 'compute_age',
);
1;
"#;
        let members = parse_and_extract(code);

        let pred = members.iter().find(|m| m.name == "has_user_age");
        assert!(pred.is_some(), "expected custom predicate 'has_user_age'");

        let clear = members.iter().find(|m| m.name == "reset_age");
        assert!(clear.is_some(), "expected custom clearer 'reset_age'");

        let builder = members.iter().find(|m| m.name == "compute_age");
        assert!(builder.is_some(), "expected custom builder 'compute_age'");
        Ok(())
    }

    // ── Multiple attributes in one has ──────────────────────────────────

    #[test]
    fn multiple_attributes_in_one_has() -> Result<(), String> {
        let code = r#"
package MyApp::Point;
use Moo;
has ['x', 'y'] => (is => 'ro');
1;
"#;
        let members = parse_and_extract(code);
        let x_members = members_named(&members, "x");
        let y_members = members_named(&members, "y");

        assert_eq!(x_members.len(), 1, "expected one member for 'x'");
        assert_eq!(y_members.len(), 1, "expected one member for 'y'");
        assert_eq!(x_members[0].kind, GeneratedMemberKind::Getter);
        assert_eq!(y_members[0].kind, GeneratedMemberKind::Getter);
        Ok(())
    }

    #[test]
    fn bare_identifier_has_generates_getter() -> Result<(), String> {
        let code = "package MyApp::User;\nuse Moo;\nhas name => (is => 'ro');\n1;";
        let members = parse_and_extract(code);
        let matched = members_named(&members, "name");
        assert_eq!(matched.len(), 1, "expected one generated member for bare identifier attr");
        assert_eq!(matched[0].kind, GeneratedMemberKind::Getter);
        assert_eq!(matched[0].provenance, Provenance::FrameworkSynthesis);
        assert_eq!(matched[0].confidence, Confidence::Medium);
        Ok(())
    }

    #[test]
    fn augmented_attribute_strips_plus_prefix() -> Result<(), String> {
        let code = r#"
package MyApp::User;
use Moo;
has '+name' => (is => 'ro', builder => 1, predicate => 1, clearer => 1);
1;
"#;
        let members = parse_and_extract(code);

        let getter = members.iter().find(|m| m.name == "name");
        assert!(getter.is_some(), "expected getter named `name`, got {members:?}");
        assert!(
            members.iter().any(|m| m.name == "_build_name"),
            "expected builder `_build_name`, got {members:?}"
        );
        assert!(
            members.iter().any(|m| m.name == "has_name"),
            "expected predicate `has_name`, got {members:?}"
        );
        assert!(
            members.iter().any(|m| m.name == "clear_name"),
            "expected clearer `clear_name`, got {members:?}"
        );
        assert!(
            members.iter().all(|m| !m.name.starts_with('+')),
            "generated member names should not retain `+`: {members:?}"
        );
        Ok(())
    }

    #[test]
    fn class_accessor_methods_emit_generated_members() -> Result<(), String> {
        let code = r#"
package MyApp::Accessor;
use parent 'Class::Accessor';
__PACKAGE__->mk_accessors(qw(foo bar));
__PACKAGE__->mk_rw_accessors(qw(read_write));
__PACKAGE__->mk_ro_accessors(qw(read_only));
__PACKAGE__->mk_wo_accessors(qw(write_only));
1;
"#;
        let members = parse_and_extract(code);

        let foo = members_named(&members, "foo");
        let read_write = members_named(&members, "read_write");
        let read_only = members_named(&members, "read_only");
        let write_only = members_named(&members, "write_only");

        assert_eq!(foo.len(), 1, "expected Class::Accessor `foo` member");
        assert_eq!(foo[0].kind, GeneratedMemberKind::Accessor);
        assert_eq!(read_write.len(), 1, "expected Class::Accessor rw member");
        assert_eq!(read_write[0].kind, GeneratedMemberKind::Accessor);
        assert_eq!(read_only.len(), 1, "expected Class::Accessor ro member");
        assert_eq!(read_only[0].kind, GeneratedMemberKind::Getter);
        assert_eq!(write_only.len(), 1, "expected Class::Accessor wo member");
        assert_eq!(write_only[0].kind, GeneratedMemberKind::Setter);

        for member in &members {
            assert_eq!(member.package, "MyApp::Accessor");
            assert_eq!(member.provenance, Provenance::FrameworkSynthesis);
            assert_eq!(member.confidence, Confidence::Medium);
        }
        Ok(())
    }

    // ── Non-Moo/Moose/Mouse packages are skipped ───────────────────────

    #[test]
    fn plain_oo_has_is_ignored() -> Result<(), String> {
        let code = "package PlainPkg;\nuse parent 'Base';\nhas 'x' => (is => 'rw');\n1;";
        let members = parse_and_extract(code);
        // PlainOO framework — `has` is not a Moo/Moose keyword here.
        // The class model builder may or may not produce attributes for
        // non-framework packages, but the extractor should not emit members.
        let matched = members_named(&members, "x");
        assert!(
            matched.is_empty(),
            "expected no generated members for plain OO package, got {matched:?}"
        );
        Ok(())
    }

    // ── Mouse framework is supported ────────────────────────────────────

    #[test]
    fn mouse_has_generates_members() -> Result<(), String> {
        let code = "package MyApp::Tiny;\nuse Mouse;\nhas 'value' => (is => 'rw');\n1;";
        let members = parse_and_extract(code);
        let matched = members_named(&members, "value");
        assert_eq!(matched.len(), 1, "expected accessor for Mouse class");
        assert_eq!(matched[0].kind, GeneratedMemberKind::Accessor);
        Ok(())
    }

    // ── Provenance and confidence invariants ────────────────────────────

    #[test]
    fn all_members_have_framework_synthesis_provenance() -> Result<(), String> {
        let code = r#"
package MyApp::Full;
use Moose;
has 'attr1' => (is => 'rw', predicate => 1, clearer => 1, builder => 1);
has 'attr2' => (is => 'ro');
1;
"#;
        let members = parse_and_extract(code);
        assert!(!members.is_empty(), "expected at least one generated member");

        for member in &members {
            assert_eq!(
                member.provenance,
                Provenance::FrameworkSynthesis,
                "member '{}' has wrong provenance: {:?}",
                member.name,
                member.provenance
            );
            assert_eq!(
                member.confidence,
                Confidence::Medium,
                "member '{}' has wrong confidence: {:?}",
                member.name,
                member.confidence
            );
        }
        Ok(())
    }

    // ── Deterministic entity IDs ────────────────────────────────────────

    #[test]
    fn entity_ids_are_deterministic() -> Result<(), String> {
        let code = "package MyApp::User;\nuse Moo;\nhas 'name' => (is => 'ro');\n1;";
        let members1 = parse_and_extract(code);
        let members2 = parse_and_extract(code);

        assert_eq!(members1.len(), members2.len(), "member count differs across runs");
        for (a, b) in members1.iter().zip(members2.iter()) {
            assert_eq!(a.entity_id, b.entity_id, "entity_id differs for '{}'", a.name);
        }
        Ok(())
    }

    // ── Package context is correct ──────────────────────────────────────

    #[test]
    fn multiple_packages_have_correct_package_names() -> Result<(), String> {
        let code = r#"
package Foo;
use Moo;
has 'a' => (is => 'ro');

package Bar;
use Moose;
has 'b' => (is => 'rw');
1;
"#;
        let members = parse_and_extract(code);

        let a_members = members_named(&members, "a");
        let a = a_members.first().ok_or("expected member 'a'")?;
        assert_eq!(a.package, "Foo");

        let b_members = members_named(&members, "b");
        // b has getter + setter
        for m in &b_members {
            assert_eq!(m.package, "Bar");
        }
        Ok(())
    }
}
