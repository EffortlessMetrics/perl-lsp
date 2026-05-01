use std::collections::HashMap;

use perl_parser::ast::{Node, NodeKind};

use super::{CallHierarchyItem, CallHierarchyProvider};

impl CallHierarchyProvider {
    pub(super) fn extract_qualified_call_name(&self, node: &Node) -> Option<String> {
        let snippet = self.source.get(node.location.start..node.location.end)?.trim();
        let callee = snippet.split('(').next()?.trim();
        callee.contains("::").then(|| callee.to_string())
    }

    fn looks_like_package_name(name: &str) -> bool {
        name.contains("::") || name.chars().next().is_some_and(|c| c.is_ascii_uppercase())
    }

    fn node_variable_name(node: &Node) -> Option<&str> {
        if let NodeKind::Variable { name, .. } = &node.kind { Some(name.as_str()) } else { None }
    }

    pub(super) fn current_package_for_function(
        &self,
        func_node: &Node,
        item: &CallHierarchyItem,
    ) -> Option<String> {
        item.package_name
            .clone()
            .or_else(|| {
                item.qualified_name.as_deref().and_then(|qualified| {
                    qualified.rsplit_once("::").map(|(package_name, _)| package_name.to_string())
                })
            })
            .or_else(|| {
                self.source.get(..func_node.location.start).and_then(|prefix| {
                    prefix.lines().rev().find_map(|line| {
                        let line = line.trim();
                        line.strip_prefix("package ")
                            .map(|rest| rest.trim_end_matches(';').trim())
                            .filter(|package_name| !package_name.is_empty())
                            .map(|package_name| package_name.to_string())
                    })
                })
            })
    }

    pub(super) fn infer_receiver_package(
        &self,
        object: &Node,
        current_package: Option<&str>,
        receiver_packages: &HashMap<String, String>,
    ) -> Option<String> {
        if let Some(name) = Self::node_variable_name(object) {
            if let Some(package_name) = receiver_packages.get(name) {
                return Some(package_name.clone());
            }

            if matches!(name, "self" | "class") {
                return current_package.map(|package_name| package_name.to_string());
            }

            if Self::looks_like_package_name(name) {
                return Some(name.to_string());
            }
        }

        None
    }

    fn infer_constructor_package(
        &self,
        rhs: &Node,
        current_package: Option<&str>,
        receiver_packages: &HashMap<String, String>,
    ) -> Option<String> {
        match &rhs.kind {
            NodeKind::MethodCall { method, object, .. } if method == "new" => {
                self.infer_receiver_package(object, current_package, receiver_packages)
            }
            NodeKind::FunctionCall { name, .. } => {
                name.rsplit_once("::").map(|(package_name, _)| package_name.to_string())
            }
            _ => None,
        }
    }

    pub(super) fn record_receiver_assignment(
        &self,
        lhs: &Node,
        rhs: &Node,
        current_package: Option<&str>,
        receiver_packages: &mut HashMap<String, String>,
    ) {
        if let Some(variable_name) = Self::node_variable_name(lhs) {
            if let Some(package_name) =
                self.infer_constructor_package(rhs, current_package, receiver_packages)
            {
                receiver_packages.insert(variable_name.to_string(), package_name);
            }
        }
    }

    pub(super) fn outgoing_call_key(item: &CallHierarchyItem) -> &str {
        item.qualified_name.as_deref().unwrap_or(item.name.as_str())
    }
}
