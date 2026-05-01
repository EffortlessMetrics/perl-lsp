use serde_json::{json, Value};

use super::{CallHierarchyIncomingCall, CallHierarchyItem, CallHierarchyOutgoingCall};

/// Convert to JSON for LSP
impl CallHierarchyItem {
    /// Convert the call hierarchy item to JSON format for LSP protocol.
    ///
    /// # Returns
    /// A JSON value containing the item name, symbol kind, URI, and range information
    /// compatible with LSP CallHierarchyItem specification.
    pub fn to_json(&self) -> Value {
        let mut item = json!({
            "name": self.name,
            "kind": match self.kind.as_str() {
                "function" => 12, // SymbolKind.Function
                "method" => 6,    // SymbolKind.Method
                _ => 12,
            },
            "uri": self.uri,
            "range": {
                "start": {
                    "line": self.range.start.line,
                    "character": self.range.start.character
                },
                "end": {
                    "line": self.range.end.line,
                    "character": self.range.end.character
                }
            },
            "selectionRange": {
                "start": {
                    "line": self.selection_range.start.line,
                    "character": self.selection_range.start.character
                },
                "end": {
                    "line": self.selection_range.end.line,
                    "character": self.selection_range.end.character
                }
            }
        });

        if let Some(detail) = &self.detail {
            item["detail"] = json!(detail);
        }

        if self.package_name.is_some() || self.qualified_name.is_some() {
            item["data"] = json!({
                "packageName": self.package_name,
                "qualifiedName": self.qualified_name,
            });
        }

        item
    }
}

impl CallHierarchyIncomingCall {
    /// Convert the incoming call to JSON format for LSP protocol.
    ///
    /// # Returns
    /// A JSON value containing the source item and ranges where the call originates.
    pub fn to_json(&self) -> Value {
        json!({
            "from": self.from.to_json(),
            "fromRanges": self.from_ranges.iter().map(|r| json!({
                "start": {
                    "line": r.start.line,
                    "character": r.start.character
                },
                "end": {
                    "line": r.end.line,
                    "character": r.end.character
                }
            })).collect::<Vec<_>>()
        })
    }
}

impl CallHierarchyOutgoingCall {
    /// Convert the outgoing call to JSON format for LSP protocol.
    ///
    /// # Returns
    /// A JSON value containing the target item and ranges where the call is made.
    pub fn to_json(&self) -> Value {
        json!({
            "to": self.to.to_json(),
            "fromRanges": self.from_ranges.iter().map(|r| json!({
                "start": {
                    "line": r.start.line,
                    "character": r.start.character
                },
                "end": {
                    "line": r.end.line,
                    "character": r.end.character
                }
            })).collect::<Vec<_>>()
        })
    }
}
