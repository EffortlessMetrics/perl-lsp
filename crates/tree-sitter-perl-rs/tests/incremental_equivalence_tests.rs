//! Seeded facade equivalence checks for the one-edit token-replay contract.

use perl_parser_core::edit::Edit;
use perl_position_tracking::Position;
use tree_sitter_perl_rs::{Node, Parser, Tree};

#[derive(Debug, PartialEq, Eq)]
struct NodeShape {
    kind: String,
    field_name: Option<&'static str>,
    start_byte: usize,
    end_byte: usize,
    start_point: (usize, usize),
    end_point: (usize, usize),
    text: String,
    children: Vec<NodeShape>,
}

fn shape(node: Node<'_>) -> Result<NodeShape, String> {
    shape_with_field(node, None)
}

fn shape_with_field(node: Node<'_>, field_name: Option<&'static str>) -> Result<NodeShape, String> {
    let start = node.start_position();
    let end = node.end_position();
    let text = node
        .utf8_text(node.tree_source().as_bytes())
        .map_err(|error| error.to_string())?
        .to_owned();
    let mut children = Vec::with_capacity(node.child_count());
    for index in 0..node.child_count() {
        let child = node.child(index).ok_or_else(|| format!("missing child {index}"))?;
        children.push(shape_with_field(child, node.field_name_for_child(index))?);
    }
    Ok(NodeShape {
        kind: node.kind(),
        field_name,
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
        start_point: (start.row, start.column),
        end_point: (end.row, end.column),
        text,
        children,
    })
}

fn position_at(source: &str, byte: usize) -> Result<Position, String> {
    if byte > source.len() || !source.is_char_boundary(byte) {
        return Err(format!("invalid position byte offset {byte}"));
    }
    let prefix = &source[..byte];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = prefix.rsplit('\n').next().map_or(0, str::len) + 1;
    let line = u32::try_from(line).map_err(|_| "line number exceeds u32".to_owned())?;
    let column = u32::try_from(column).map_err(|_| "column exceeds u32".to_owned())?;
    Ok(Position::new(byte, line, column))
}

fn edit(source: &str, old_text: &str, new_text: &str) -> Result<(String, Edit), String> {
    let start = source.find(old_text).ok_or_else(|| format!("missing edit text {old_text:?}"))?;
    let old_end = start + old_text.len();
    let new_end = start + new_text.len();
    let mut new_source = String::with_capacity(source.len() - old_text.len() + new_text.len());
    new_source.push_str(&source[..start]);
    new_source.push_str(new_text);
    new_source.push_str(&source[old_end..]);
    let descriptor = Edit::new(
        start,
        old_end,
        new_end,
        position_at(source, start)?,
        position_at(source, old_end)?,
        position_at(&new_source, new_end)?,
    );
    Ok((new_source, descriptor))
}

fn assert_equivalent(incremental: &Tree, fresh: &Tree) -> Result<(), String> {
    if incremental.root_node().to_sexp() != fresh.root_node().to_sexp() {
        return Err("S-expression mismatch".to_owned());
    }
    if incremental.diagnostics() != fresh.diagnostics() {
        return Err("diagnostic mismatch".to_owned());
    }
    if incremental.has_error() != fresh.has_error() {
        return Err("error-status mismatch".to_owned());
    }
    if shape(incremental.root_node())? != shape(fresh.root_node())? {
        return Err("node shape mismatch".to_owned());
    }
    Ok(())
}

#[test]
fn checkpoint_left_boundary_matches_fresh_parse() -> Result<(), String> {
    let identifier = "x".repeat(254);
    let source = format!("my ${identifier};\nmy $tail = 1;\n");
    let boundary = 4 + identifier.len();
    let new_source = format!("my ${identifier}z;\nmy $tail = 1;\n");
    let descriptor = Edit::new(
        boundary,
        boundary,
        boundary + 1,
        position_at(&source, boundary)?,
        position_at(&source, boundary)?,
        position_at(&new_source, boundary + 1)?,
    );

    let mut parser = Parser::new();
    let old = parser.parse(&source).ok_or("initial parse failed")?;
    let fresh = parser.parse(&new_source).ok_or("fresh parse failed")?;
    let mut edited = old;
    edited.edit(&descriptor);
    let replayed = parser.parse_with_old_tree(&new_source, &edited).ok_or("replay parse failed")?;

    assert_equivalent(&replayed, &fresh)?;
    if replayed.reparse_mode() != Some(tree_sitter_perl_rs::ReparseMode::TokenReplay) {
        return Err(format!("unexpected mode: {:?}", replayed.reparse_mode()));
    }
    Ok(())
}

#[test]
fn sequential_edits_remain_equivalent_to_fresh_parses() -> Result<(), String> {
    let mut parser = Parser::new();
    let source = "my $value = 1;\n".repeat(40);
    let old = parser.parse(&source).ok_or("initial parse failed")?;

    let (first_source, first_edit) = edit(&source, "1", "22")?;
    let mut first_old = old;
    first_old.edit(&first_edit);
    let first_replayed =
        parser.parse_with_old_tree(&first_source, &first_old).ok_or("first replay failed")?;
    let first_fresh = parser.parse(&first_source).ok_or("first fresh parse failed")?;
    assert_equivalent(&first_replayed, &first_fresh)?;

    let (second_source, second_edit) = edit(&first_source, "22", "3")?;
    let mut second_old = first_replayed;
    second_old.edit(&second_edit);
    let second_replayed =
        parser.parse_with_old_tree(&second_source, &second_old).ok_or("second replay failed")?;
    let second_fresh = parser.parse(&second_source).ok_or("second fresh parse failed")?;
    assert_equivalent(&second_replayed, &second_fresh)
}

#[test]
fn recovered_edit_remains_equivalent_without_stale_diagnostics() -> Result<(), String> {
    let source = "my $value = ;\n";
    let (new_source, descriptor) = edit(source, "= ;", "= 1;")?;
    let mut parser = Parser::new();
    let old = parser.parse(source).ok_or("initial parse failed")?;
    let fresh = parser.parse(&new_source).ok_or("fresh parse failed")?;
    let mut edited = old;
    edited.edit(&descriptor);
    let replayed =
        parser.parse_with_old_tree(&new_source, &edited).ok_or("fallback parse failed")?;

    assert_equivalent(&replayed, &fresh)?;
    Ok(())
}
