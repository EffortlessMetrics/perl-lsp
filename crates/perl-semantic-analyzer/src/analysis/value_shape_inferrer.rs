//! Lightweight value-shape inference from Perl AST patterns.
//!
//! Walks the AST to infer [`ValueShape`] approximations for variables based
//! on common Perl idioms:
//!
//! | Perl pattern                             | Inferred shape                                |
//! |------------------------------------------|-----------------------------------------------|
//! | `Foo->new(...)`                          | `Object { "Foo", High }`                      |
//! | `bless $ref, 'Pkg'`                      | `Object { "Pkg", Low }`                       |
//! | `$self` in method body                   | `Object { <enclosing package>, Medium }`       |
//! | unknown                                  | `Unknown`                                     |
//!
//! The inferrer does **not** perform full type inference — it recognises
//! syntactic patterns and assigns conservative shapes.

use crate::ast::{Node, NodeKind};
use perl_semantic_facts::{Confidence, EntityId, FileId, ValueShape};

/// Inferrer that walks an AST to produce `(EntityId, ValueShape)` pairs.
///
/// Each pair maps a variable's deterministic entity ID to its inferred shape.
pub struct ValueShapeInferrer;

impl ValueShapeInferrer {
    /// Walk the entire AST and return `(EntityId, ValueShape)` pairs for
    /// every variable whose shape can be inferred from syntactic patterns.
    pub fn infer(ast: &Node, _file_id: FileId) -> Vec<(EntityId, ValueShape)> {
        let mut state = InferrerState {
            current_package: "main".to_string(),
            in_method: false,
            results: Vec::new(),
        };
        state.walk(ast);
        state.results
    }
}

/// Internal state for the recursive AST walk.
struct InferrerState {
    /// Current package context (updated when `package Foo;` is encountered).
    current_package: String,
    /// Whether we are currently inside a subroutine/method body.
    in_method: bool,
    /// Accumulated (EntityId, ValueShape) pairs.
    results: Vec<(EntityId, ValueShape)>,
}

impl InferrerState {
    /// Recursive AST walker.
    fn walk(&mut self, node: &Node) {
        match &node.kind {
            // Statement containers — walk children in order.
            NodeKind::Program { statements } | NodeKind::Block { statements } => {
                for stmt in statements {
                    self.walk(stmt);
                }
                return;
            }

            // `package Foo { ... }` (block form) — scoped package context.
            NodeKind::Package { name, block: Some(block), .. } => {
                let prev = self.current_package.clone();
                self.current_package = name.clone();
                self.walk(block);
                self.current_package = prev;
                return;
            }

            // `package Foo;` (semicolon form) — updates current package.
            NodeKind::Package { name, block: None, .. } => {
                self.current_package = name.clone();
                return;
            }

            // Subroutine / method body — track that we are inside a method
            // so `$self` references can be inferred.
            NodeKind::Subroutine { body, .. } | NodeKind::Method { body, .. } => {
                let prev_in_method = self.in_method;
                self.in_method = true;
                self.walk(body);
                self.in_method = prev_in_method;
                return;
            }

            // Variable declaration with initializer:
            // `my $obj = Foo->new(...)` or `my $obj = bless ...`
            NodeKind::VariableDeclaration { variable, initializer: Some(init), .. } => {
                if let Some(shape) = self.infer_from_rhs(init) {
                    let entity_id = entity_id_from_variable(variable);
                    self.results.push((entity_id, shape));
                }
            }

            // Assignment: `$obj = Foo->new(...)` or `$obj = bless ...`
            NodeKind::Assignment { lhs, rhs, .. } => {
                if let Some(shape) = self.infer_from_rhs(rhs) {
                    let entity_id = entity_id_from_variable(lhs);
                    self.results.push((entity_id, shape));
                }
            }

            // `$self` reference inside a method body.
            NodeKind::Variable { sigil, name } if sigil == "$" && name == "self" => {
                if self.in_method {
                    let entity_id = entity_id_from_node(node);
                    self.results.push((
                        entity_id,
                        ValueShape::Object {
                            package: self.current_package.clone(),
                            confidence: Confidence::Medium,
                        },
                    ));
                }
            }

            _ => {}
        }

        // Recurse into children for all other node types.
        for child in node.children() {
            self.walk(child);
        }
    }

    /// Try to infer a [`ValueShape`] from the right-hand side of an
    /// assignment or variable declaration.
    fn infer_from_rhs(&self, rhs: &Node) -> Option<ValueShape> {
        match &rhs.kind {
            // `Foo->new(...)` — constructor call.
            NodeKind::MethodCall { object, method, .. } if method == "new" => {
                if let Some(pkg) = package_name_from_node(object) {
                    return Some(ValueShape::Object { package: pkg, confidence: Confidence::High });
                }
                None
            }

            // `bless $ref, 'Pkg'` — bless call.
            NodeKind::FunctionCall { name, args } if name == "bless" => {
                // Second argument is the package name.
                if let Some(pkg_node) = args.get(1) {
                    if let Some(pkg) = string_value(pkg_node) {
                        return Some(ValueShape::Object {
                            package: pkg,
                            confidence: Confidence::Low,
                        });
                    }
                }
                // `bless $ref` with no explicit package — uses current package.
                if args.len() == 1 {
                    return Some(ValueShape::Object {
                        package: self.current_package.clone(),
                        confidence: Confidence::Low,
                    });
                }
                None
            }

            _ => None,
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

/// Extract a package name from a node that represents a class/package
/// (e.g. the `Foo` in `Foo->new`).
fn package_name_from_node(node: &Node) -> Option<String> {
    match &node.kind {
        NodeKind::Identifier { name } => Some(name.clone()),
        NodeKind::String { value, .. } => normalize_package_string(value),
        _ => None,
    }
}

/// Extract a string value from a string literal node.
fn string_value(node: &Node) -> Option<String> {
    match &node.kind {
        NodeKind::String { value, .. } => normalize_package_string(value),
        NodeKind::Identifier { name } => Some(name.clone()),
        _ => None,
    }
}

fn normalize_package_string(value: &str) -> Option<String> {
    let normalized = value.trim().trim_matches('\'').trim_matches('"').trim();
    if normalized.is_empty() { None } else { Some(normalized.to_string()) }
}

/// Derive a deterministic [`EntityId`] from a variable node using its
/// byte-offset span.
fn entity_id_from_variable(node: &Node) -> EntityId {
    entity_id_from_node(node)
}

/// Derive a deterministic [`EntityId`] from a node's byte-offset span.
///
/// Uses a simple FNV-1a–style hash to produce a stable ID.
fn entity_id_from_node(node: &Node) -> EntityId {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0100_0000_01b3;

    let mut hash = FNV_OFFSET;
    for byte in (node.location.start as u64).to_le_bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    for byte in (node.location.end as u64).to_le_bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    EntityId(hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;

    /// Parse Perl source and infer value shapes.
    fn parse_and_infer(code: &str) -> Vec<(EntityId, ValueShape)> {
        let mut parser = Parser::new(code);
        let ast = match parser.parse() {
            Ok(ast) => ast,
            Err(_) => return Vec::new(),
        };
        ValueShapeInferrer::infer(&ast, FileId(1))
    }

    /// Helper: find the first Object shape in results.
    fn first_object(results: &[(EntityId, ValueShape)]) -> Option<(&str, Confidence)> {
        for (_, shape) in results {
            if let ValueShape::Object { package, confidence } = shape {
                return Some((package.as_str(), *confidence));
            }
        }
        None
    }

    // ── Constructor call: Foo->new(...) ─────────────────────────────────

    #[test]
    fn constructor_call_infers_object_high() -> Result<(), String> {
        let results = parse_and_infer("my $obj = Foo->new();\n");
        let (pkg, conf) = first_object(&results).ok_or("expected Object shape from Foo->new()")?;
        assert_eq!(pkg, "Foo");
        assert_eq!(conf, Confidence::High);
        Ok(())
    }

    #[test]
    fn qualified_constructor_call_infers_object() -> Result<(), String> {
        let results = parse_and_infer("my $obj = My::App->new();\n");
        let (pkg, conf) =
            first_object(&results).ok_or("expected Object shape from My::App->new()")?;
        assert_eq!(pkg, "My::App");
        assert_eq!(conf, Confidence::High);
        Ok(())
    }

    // ── bless $ref, 'Pkg' ───────────────────────────────────────────────

    #[test]
    fn bless_with_package_infers_object_low() -> Result<(), String> {
        let code = "package Foo;\nsub new { my $self = bless {}, 'Foo'; }\n";
        let results = parse_and_infer(code);
        let (pkg, conf) =
            first_object(&results).ok_or("expected Object shape from bless {}, 'Foo'")?;
        assert_eq!(pkg, "Foo");
        assert_eq!(conf, Confidence::Low);
        Ok(())
    }

    // ── $self in method body ────────────────────────────────────────────

    #[test]
    fn self_in_method_infers_enclosing_package() -> Result<(), String> {
        let code = "package Bar;\nsub greet { my $msg = $self->name(); }\n";
        let results = parse_and_infer(code);
        // $self should be inferred as Object { Bar, Medium }
        let has_bar_medium = results.iter().any(|(_, shape)| {
            matches!(shape, ValueShape::Object { package, confidence }
                if package == "Bar" && *confidence == Confidence::Medium)
        });
        assert!(
            has_bar_medium,
            "expected $self to infer Object {{ Bar, Medium }}, got {results:?}"
        );
        Ok(())
    }

    // ── Unknown fallback ────────────────────────────────────────────────

    #[test]
    fn plain_scalar_produces_no_shape() -> Result<(), String> {
        let results = parse_and_infer("my $x = 42;\n");
        // No Object shapes should be inferred for a plain scalar.
        assert!(first_object(&results).is_none(), "plain scalar should not produce Object shape");
        Ok(())
    }

    // ── Multiple packages ───────────────────────────────────────────────

    #[test]
    fn multiple_packages_track_context() -> Result<(), String> {
        let code = r#"
package Alpha;
sub new { my $self = bless {}, 'Alpha'; }

package Beta;
sub new { my $self = bless {}, 'Beta'; }
"#;
        let results = parse_and_infer(code);
        let has_alpha = results.iter().any(
            |(_, shape)| matches!(shape, ValueShape::Object { package, .. } if package == "Alpha"),
        );
        let has_beta = results.iter().any(
            |(_, shape)| matches!(shape, ValueShape::Object { package, .. } if package == "Beta"),
        );
        assert!(has_alpha, "expected Alpha object shape");
        assert!(has_beta, "expected Beta object shape");
        Ok(())
    }
}
