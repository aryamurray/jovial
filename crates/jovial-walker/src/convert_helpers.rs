use jovial_ast::go::{GoBinaryOp, GoLiteralValue, GoNode, GoParam, GoType, GoUnaryOp};
use jovial_ast::java::{BinaryOp, JavaNode, JavaType, LiteralValue, Modifier, UnaryOp};

use crate::type_map;

/// Convert a Java type to a Go type, handling generics, arrays, maps, Optional.
pub(crate) fn convert_java_type(jt: &JavaType) -> GoType {
    let base = type_map::java_to_go_type(&jt.name);

    // Handle array dimensions
    if jt.array_dimensions > 0 || jt.is_varargs {
        let element_type = GoType {
            name: if base == "[]" || base == "map" || base == "*" {
                jt.name.clone()
            } else {
                base.to_string()
            },
            package: extract_package(base),
            is_pointer: false,
            is_slice: false,
            is_map: false,
            key_type: None,
            value_type: None,
        };
        return GoType {
            name: element_type.name.clone(),
            package: element_type.package.clone(),
            is_pointer: false,
            is_slice: true,
            is_map: false,
            key_type: None,
            value_type: Some(Box::new(element_type)),
        };
    }

    match base {
        "[]" => {
            let elem = jt
                .type_args
                .first()
                .map(convert_java_type)
                .unwrap_or_else(|| simple_go_type("interface{}"));
            GoType {
                name: elem.name.clone(),
                package: elem.package.clone(),
                is_pointer: false,
                is_slice: true,
                is_map: false,
                key_type: None,
                value_type: Some(Box::new(elem)),
            }
        }
        "map" => {
            let key = jt
                .type_args
                .first()
                .map(convert_java_type)
                .unwrap_or_else(|| simple_go_type("string"));
            let val = jt
                .type_args
                .get(1)
                .map(convert_java_type)
                .unwrap_or_else(|| simple_go_type("interface{}"));
            GoType {
                name: "map".to_string(),
                package: None,
                is_pointer: false,
                is_slice: false,
                is_map: true,
                key_type: Some(Box::new(key)),
                value_type: Some(Box::new(val)),
            }
        }
        "*" => {
            let inner = jt
                .type_args
                .first()
                .map(convert_java_type)
                .unwrap_or_else(|| simple_go_type("interface{}"));
            GoType {
                name: inner.name.clone(),
                package: inner.package.clone(),
                is_pointer: true,
                is_slice: false,
                is_map: false,
                key_type: None,
                value_type: None,
            }
        }
        other if other.starts_with("time.") => {
            let type_name = other.strip_prefix("time.").unwrap_or(other);
            GoType {
                name: type_name.to_string(),
                package: Some("time".to_string()),
                is_pointer: false,
                is_slice: false,
                is_map: false,
                key_type: None,
                value_type: None,
            }
        }
        other => GoType {
            name: other.to_string(),
            package: None,
            is_pointer: false,
            is_slice: false,
            is_map: false,
            key_type: None,
            value_type: None,
        },
    }
}

fn extract_package(type_name: &str) -> Option<String> {
    if type_name.contains('.') {
        type_name
            .rsplit_once('.')
            .map(|(pkg, _)| pkg.to_string())
    } else {
        None
    }
}

/// Create a simple Go type with no modifiers.
pub(crate) fn simple_go_type(name: &str) -> GoType {
    GoType {
        name: name.to_string(),
        package: None,
        is_pointer: false,
        is_slice: false,
        is_map: false,
        key_type: None,
        value_type: None,
    }
}

/// Convert a Java literal to a Go literal.
pub(crate) fn convert_literal(lit: &LiteralValue) -> GoLiteralValue {
    match lit {
        LiteralValue::Int(v) => GoLiteralValue::Int(*v),
        LiteralValue::Float(v) => GoLiteralValue::Float(*v),
        LiteralValue::String(v) => GoLiteralValue::String(v.clone()),
        LiteralValue::Char(v) => GoLiteralValue::Rune(*v),
        LiteralValue::Bool(v) => GoLiteralValue::Bool(*v),
        LiteralValue::Null => GoLiteralValue::Nil,
    }
}

/// Convert a Java binary operator to a Go binary operator.
/// Returns `None` for `InstanceOf` (no direct Go equivalent).
pub(crate) fn convert_binary_op(op: &BinaryOp) -> Option<GoBinaryOp> {
    match op {
        BinaryOp::Add => Some(GoBinaryOp::Add),
        BinaryOp::Sub => Some(GoBinaryOp::Sub),
        BinaryOp::Mul => Some(GoBinaryOp::Mul),
        BinaryOp::Div => Some(GoBinaryOp::Div),
        BinaryOp::Mod => Some(GoBinaryOp::Mod),
        BinaryOp::And => Some(GoBinaryOp::And),
        BinaryOp::Or => Some(GoBinaryOp::Or),
        BinaryOp::BitwiseAnd => Some(GoBinaryOp::BitwiseAnd),
        BinaryOp::BitwiseOr => Some(GoBinaryOp::BitwiseOr),
        BinaryOp::BitwiseXor => Some(GoBinaryOp::BitwiseXor),
        BinaryOp::ShiftLeft => Some(GoBinaryOp::ShiftLeft),
        BinaryOp::ShiftRight => Some(GoBinaryOp::ShiftRight),
        BinaryOp::UnsignedShiftRight => Some(GoBinaryOp::ShiftRight),
        BinaryOp::Eq => Some(GoBinaryOp::Eq),
        BinaryOp::Ne => Some(GoBinaryOp::Ne),
        BinaryOp::Lt => Some(GoBinaryOp::Lt),
        BinaryOp::Gt => Some(GoBinaryOp::Gt),
        BinaryOp::Le => Some(GoBinaryOp::Le),
        BinaryOp::Ge => Some(GoBinaryOp::Ge),
        BinaryOp::InstanceOf => None,
    }
}

/// Convert a Java unary operator to a Go unary operator.
/// Returns `None` for pre/post increment/decrement (handled as assignment).
pub(crate) fn convert_unary_op(op: &UnaryOp) -> Option<GoUnaryOp> {
    match op {
        UnaryOp::Negate => Some(GoUnaryOp::Negate),
        UnaryOp::Not => Some(GoUnaryOp::Not),
        UnaryOp::BitwiseNot => Some(GoUnaryOp::BitwiseNot),
        UnaryOp::PreIncrement
        | UnaryOp::PreDecrement
        | UnaryOp::PostIncrement
        | UnaryOp::PostDecrement => None,
    }
}

/// Capitalize first letter for Go exported names.
pub(crate) fn java_name_to_go_exported(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Lowercase first letter for Go unexported names.
pub(crate) fn java_name_to_go_unexported(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) => c.to_lowercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Derive receiver name: first char of class name, lowercased.
pub(crate) fn receiver_name(class: &str) -> String {
    class
        .chars()
        .next()
        .unwrap_or('s')
        .to_lowercase()
        .to_string()
}

/// Check if modifiers contain `Static`.
pub(crate) fn is_static(modifiers: &[Modifier]) -> bool {
    modifiers.contains(&Modifier::Static)
}

/// Unwrap a single-element `BlockStmt` to its inner statements.
pub(crate) fn flatten_block(mut nodes: Vec<GoNode>) -> Vec<GoNode> {
    if nodes.len() == 1 && matches!(nodes[0], GoNode::BlockStmt { .. }) {
        if let GoNode::BlockStmt { statements, .. } = nodes.remove(0) {
            return statements;
        }
    }
    nodes
}

/// Convert Java parameters to Go params.
pub(crate) fn convert_params(parameters: &[JavaNode]) -> Vec<GoParam> {
    parameters
        .iter()
        .filter_map(|p| {
            if let JavaNode::Parameter {
                name, param_type, ..
            } = p
            {
                Some(GoParam {
                    name: name.clone(),
                    param_type: convert_java_type(param_type),
                })
            } else {
                None
            }
        })
        .collect()
}

/// Convert a return type to Go types vec (empty if void/absent).
pub(crate) fn return_type_to_go(return_type: Option<&JavaType>) -> Vec<GoType> {
    match return_type {
        Some(rt) => {
            let gt = convert_java_type(rt);
            if gt.name.is_empty() {
                vec![]
            } else {
                vec![gt]
            }
        }
        None => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_types() {
        let jt = JavaType {
            name: "int".to_string(),
            type_args: vec![],
            array_dimensions: 0,
            is_varargs: false,
        };
        let gt = convert_java_type(&jt);
        assert_eq!(gt.name, "int");
        assert!(!gt.is_pointer);
        assert!(!gt.is_slice);
        assert!(!gt.is_map);
    }

    #[test]
    fn test_list_string() {
        let jt = JavaType {
            name: "List".to_string(),
            type_args: vec![JavaType {
                name: "String".to_string(),
                type_args: vec![],
                array_dimensions: 0,
                is_varargs: false,
            }],
            array_dimensions: 0,
            is_varargs: false,
        };
        let gt = convert_java_type(&jt);
        assert!(gt.is_slice);
        assert_eq!(gt.value_type.as_ref().unwrap().name, "string");
    }

    #[test]
    fn test_map_string_integer() {
        let jt = JavaType {
            name: "Map".to_string(),
            type_args: vec![
                JavaType {
                    name: "String".to_string(),
                    type_args: vec![],
                    array_dimensions: 0,
                    is_varargs: false,
                },
                JavaType {
                    name: "Integer".to_string(),
                    type_args: vec![],
                    array_dimensions: 0,
                    is_varargs: false,
                },
            ],
            array_dimensions: 0,
            is_varargs: false,
        };
        let gt = convert_java_type(&jt);
        assert!(gt.is_map);
        assert_eq!(gt.key_type.as_ref().unwrap().name, "string");
        assert_eq!(gt.value_type.as_ref().unwrap().name, "int");
    }

    #[test]
    fn test_optional_foo() {
        let jt = JavaType {
            name: "Optional".to_string(),
            type_args: vec![JavaType {
                name: "Foo".to_string(),
                type_args: vec![],
                array_dimensions: 0,
                is_varargs: false,
            }],
            array_dimensions: 0,
            is_varargs: false,
        };
        let gt = convert_java_type(&jt);
        assert!(gt.is_pointer);
        assert_eq!(gt.name, "Foo");
    }

    #[test]
    fn test_array_type() {
        let jt = JavaType {
            name: "int".to_string(),
            type_args: vec![],
            array_dimensions: 1,
            is_varargs: false,
        };
        let gt = convert_java_type(&jt);
        assert!(gt.is_slice);
    }

    #[test]
    fn test_time_type() {
        let jt = JavaType {
            name: "LocalDateTime".to_string(),
            type_args: vec![],
            array_dimensions: 0,
            is_varargs: false,
        };
        let gt = convert_java_type(&jt);
        assert_eq!(gt.name, "Time");
        assert_eq!(gt.package.as_deref(), Some("time"));
    }

    #[test]
    fn test_literal_int() {
        assert_eq!(convert_literal(&LiteralValue::Int(42)), GoLiteralValue::Int(42));
    }

    #[test]
    fn test_literal_float() {
        assert_eq!(
            convert_literal(&LiteralValue::Float(3.14)),
            GoLiteralValue::Float(3.14)
        );
    }

    #[test]
    fn test_literal_string() {
        assert_eq!(
            convert_literal(&LiteralValue::String("hello".to_string())),
            GoLiteralValue::String("hello".to_string())
        );
    }

    #[test]
    fn test_literal_char_to_rune() {
        assert_eq!(
            convert_literal(&LiteralValue::Char('a')),
            GoLiteralValue::Rune('a')
        );
    }

    #[test]
    fn test_literal_bool() {
        assert_eq!(
            convert_literal(&LiteralValue::Bool(true)),
            GoLiteralValue::Bool(true)
        );
    }

    #[test]
    fn test_literal_null_to_nil() {
        assert_eq!(convert_literal(&LiteralValue::Null), GoLiteralValue::Nil);
    }

    #[test]
    fn test_binary_op_add() {
        assert_eq!(convert_binary_op(&BinaryOp::Add), Some(GoBinaryOp::Add));
    }

    #[test]
    fn test_binary_op_instanceof_none() {
        assert_eq!(convert_binary_op(&BinaryOp::InstanceOf), None);
    }

    #[test]
    fn test_binary_op_unsigned_shift() {
        assert_eq!(
            convert_binary_op(&BinaryOp::UnsignedShiftRight),
            Some(GoBinaryOp::ShiftRight)
        );
    }

    #[test]
    fn test_unary_op_negate() {
        assert_eq!(convert_unary_op(&UnaryOp::Negate), Some(GoUnaryOp::Negate));
    }

    #[test]
    fn test_unary_op_not() {
        assert_eq!(convert_unary_op(&UnaryOp::Not), Some(GoUnaryOp::Not));
    }

    #[test]
    fn test_unary_op_pre_increment_none() {
        assert_eq!(convert_unary_op(&UnaryOp::PreIncrement), None);
    }

    #[test]
    fn test_unary_op_post_decrement_none() {
        assert_eq!(convert_unary_op(&UnaryOp::PostDecrement), None);
    }

    #[test]
    fn test_go_exported() {
        assert_eq!(java_name_to_go_exported("getName"), "GetName");
        assert_eq!(java_name_to_go_exported("x"), "X");
        assert_eq!(java_name_to_go_exported(""), "");
    }

    #[test]
    fn test_go_unexported() {
        assert_eq!(java_name_to_go_unexported("GetName"), "getName");
        assert_eq!(java_name_to_go_unexported("X"), "x");
        assert_eq!(java_name_to_go_unexported(""), "");
    }

    #[test]
    fn test_receiver_name_single_word() {
        assert_eq!(receiver_name("Order"), "o");
    }

    #[test]
    fn test_receiver_name_multi_word() {
        assert_eq!(receiver_name("OrderService"), "o");
    }

    #[test]
    fn test_receiver_name_lowercase() {
        assert_eq!(receiver_name("service"), "s");
    }
}
