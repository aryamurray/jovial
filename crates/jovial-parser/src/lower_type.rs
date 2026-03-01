use jovial_ast::java::JavaType;
use tree_sitter::Node;

use crate::lower::Lowerer;

/// Lower a tree-sitter type node into a `JavaType`.
pub(crate) fn lower_type(lowerer: &Lowerer, node: Node) -> JavaType {
    match node.kind() {
        "void_type" => JavaType {
            name: "void".to_string(),
            type_args: Vec::new(),
            array_dimensions: 0,
            is_varargs: false,
        },
        "integral_type" | "floating_point_type" | "boolean_type" => JavaType {
            name: lowerer.node_text(node).to_string(),
            type_args: Vec::new(),
            array_dimensions: 0,
            is_varargs: false,
        },
        "type_identifier" => JavaType {
            name: lowerer.node_text(node).to_string(),
            type_args: Vec::new(),
            array_dimensions: 0,
            is_varargs: false,
        },
        "scoped_type_identifier" => JavaType {
            name: lowerer.node_text(node).to_string(),
            type_args: Vec::new(),
            array_dimensions: 0,
            is_varargs: false,
        },
        "generic_type" => {
            let base = {
                let mut cursor = node.walk();
                let found = node.named_children(&mut cursor)
                    .find(|c| c.kind() == "type_identifier" || c.kind() == "scoped_type_identifier");
                found.map(|n| lowerer.node_text(n).to_string())
                    .unwrap_or_default()
            };

            let type_args = lowerer.child_by_kind(node, "type_arguments")
                .map(|ta| lower_type_args(lowerer, ta))
                .unwrap_or_default();

            JavaType {
                name: base,
                type_args,
                array_dimensions: 0,
                is_varargs: false,
            }
        }
        "array_type" => {
            let element = node.child_by_field_name("element")
                .map(|e| lower_type(lowerer, e))
                .unwrap_or_else(|| {
                    // Fallback: first named child is the element type
                    let mut cursor = node.walk();
                    let first = node.named_children(&mut cursor).next();
                    first.map(|e| lower_type(lowerer, e))
                        .unwrap_or_else(|| JavaType {
                            name: "unknown".to_string(),
                            type_args: Vec::new(),
                            array_dimensions: 0,
                            is_varargs: false,
                        })
                });
            let dims = lowerer.children_by_kind(node, "dimensions").len();
            JavaType {
                name: element.name,
                type_args: element.type_args,
                array_dimensions: element.array_dimensions + dims.max(1),
                is_varargs: false,
            }
        }
        "wildcard" => {
            // ? extends T or ? super T or just ?
            let children = {
                let mut cursor = node.walk();
                node.named_children(&mut cursor).collect::<Vec<_>>()
            };
            if children.is_empty() {
                JavaType {
                    name: "?".to_string(),
                    type_args: Vec::new(),
                    array_dimensions: 0,
                    is_varargs: false,
                }
            } else {
                let bound = lower_type(lowerer, children[children.len() - 1]);
                // Check for extends vs super
                let text = lowerer.node_text(node);
                let prefix = if text.contains("super") {
                    "? super "
                } else {
                    "? extends "
                };
                JavaType {
                    name: format!("{}{}", prefix, bound.name),
                    type_args: bound.type_args,
                    array_dimensions: 0,
                    is_varargs: false,
                }
            }
        }
        _ => {
            // Fallback — use the raw text
            log::debug!("unknown type kind: {} text: {}", node.kind(), lowerer.node_text(node));
            JavaType {
                name: lowerer.node_text(node).to_string(),
                type_args: Vec::new(),
                array_dimensions: 0,
                is_varargs: false,
            }
        }
    }
}

/// Lower `type_arguments` node: `<T, U, ...>`.
fn lower_type_args(lowerer: &Lowerer, node: Node) -> Vec<JavaType> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .map(|child| lower_type(lowerer, child))
        .collect()
}
