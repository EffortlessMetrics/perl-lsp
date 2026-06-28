//! Tests for DAP variables response pagination support.
//!
//! Verifies that `VariablesResponseBody` correctly serializes `totalVariables`,
//! and that the field is omitted (not `null`) when the count is not known.

use perl_dap::protocol::{ProtocolVariable, VariablesResponseBody};

fn make_variable(name: &str) -> ProtocolVariable {
    ProtocolVariable {
        name: name.to_string(),
        value: "test".to_string(),
        type_: None,
        variables_reference: 0,
        named_variables: None,
        indexed_variables: None,
    }
}

/// When `total_variables` is `Some(n)`, the serialized JSON must include
/// `"totalVariables": n`.
#[test]
fn variables_response_body_total_variables_present_when_some()
-> Result<(), Box<dyn std::error::Error>> {
    let body =
        VariablesResponseBody { variables: vec![make_variable("$x")], total_variables: Some(42) };

    let json = serde_json::to_string(&body)?;
    let parsed: serde_json::Value = serde_json::from_str(&json)?;

    let total = parsed.get("totalVariables").and_then(|v| v.as_i64());
    assert_eq!(total, Some(42), "totalVariables must be 42 when Some(42) is set");
    Ok(())
}

/// When `total_variables` is `None`, the serialized JSON must NOT include the
/// `"totalVariables"` key at all — not even as `null`. DAP spec treats the
/// field as optional; emitting `null` is semantically wrong and confuses
/// clients that use presence-checks for pagination UI.
#[test]
fn variables_response_body_total_variables_absent_when_none()
-> Result<(), Box<dyn std::error::Error>> {
    let body =
        VariablesResponseBody { variables: vec![make_variable("$y")], total_variables: None };

    let json = serde_json::to_string(&body)?;
    let parsed: serde_json::Value = serde_json::from_str(&json)?;

    assert!(
        parsed.get("totalVariables").is_none(),
        "totalVariables must be absent (not null) when None; got: {json}"
    );
    Ok(())
}

/// Round-trip serde: a body with `totalVariables` deserializes correctly.
#[test]
fn variables_response_body_round_trip_with_total_variables()
-> Result<(), Box<dyn std::error::Error>> {
    let original = VariablesResponseBody {
        variables: vec![make_variable("$a"), make_variable("$b"), make_variable("$c")],
        total_variables: Some(150),
    };

    let json = serde_json::to_string(&original)?;
    let deserialized: VariablesResponseBody = serde_json::from_str(&json)?;

    assert_eq!(deserialized.variables.len(), 3);
    assert_eq!(deserialized.total_variables, Some(150));

    // Pagination invariant: returned window <= total
    assert!(
        deserialized.variables.len() as i64 <= deserialized.total_variables.unwrap(),
        "window ({}) must be <= total ({})",
        deserialized.variables.len(),
        deserialized.total_variables.unwrap()
    );
    Ok(())
}
