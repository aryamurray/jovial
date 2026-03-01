use jovial_ast::java::{
    BinaryOp, JavaNode, JavaType, LiteralValue, UnaryOp,
};
use tree_sitter::Node;

use crate::lower::Lowerer;
use crate::lower_stmt::lower_stmt;
use crate::lower_type::lower_type;

/// Lower a tree-sitter expression node into a `JavaNode`.
pub(crate) fn lower_expr(lowerer: &Lowerer, node: Node) -> JavaNode {
    match node.kind() {
        // ── Literals ────────────────────────────────────────────────
        "decimal_integer_literal" | "hex_integer_literal" | "octal_integer_literal"
        | "binary_integer_literal" => {
            let text = lowerer.node_text(node);
            let cleaned = text.replace('_', "")
                .trim_end_matches(['l', 'L'])
                .to_string();
            let val = if text.starts_with("0x") || text.starts_with("0X") {
                i64::from_str_radix(cleaned.trim_start_matches("0x").trim_start_matches("0X"), 16)
                    .unwrap_or(0)
            } else if text.starts_with("0b") || text.starts_with("0B") {
                i64::from_str_radix(cleaned.trim_start_matches("0b").trim_start_matches("0B"), 2)
                    .unwrap_or(0)
            } else if text.starts_with('0') && cleaned.len() > 1 && !cleaned.contains('.') {
                i64::from_str_radix(&cleaned, 8).unwrap_or(0)
            } else {
                cleaned.parse::<i64>().unwrap_or(0)
            };
            JavaNode::LiteralExpr {
                value: LiteralValue::Int(val),
                span: lowerer.span(node),
            }
        }
        "decimal_floating_point_literal" | "hex_floating_point_literal" => {
            let text = lowerer.node_text(node);
            let cleaned = text.replace('_', "")
                .trim_end_matches(['f', 'F', 'd', 'D'])
                .to_string();
            let val = cleaned.parse::<f64>().unwrap_or(0.0);
            JavaNode::LiteralExpr {
                value: LiteralValue::Float(val),
                span: lowerer.span(node),
            }
        }
        "string_literal" => {
            let text = lowerer.node_text(node);
            // Strip surrounding quotes
            let inner = text.strip_prefix('"').unwrap_or(text);
            let inner = inner.strip_suffix('"').unwrap_or(inner);
            JavaNode::LiteralExpr {
                value: LiteralValue::String(inner.to_string()),
                span: lowerer.span(node),
            }
        }
        "character_literal" => {
            let text = lowerer.node_text(node);
            let inner = text.strip_prefix('\'').unwrap_or(text);
            let inner = inner.strip_suffix('\'').unwrap_or(inner);
            let ch = if inner.starts_with('\\') {
                match inner.chars().nth(1) {
                    Some('n') => '\n',
                    Some('t') => '\t',
                    Some('r') => '\r',
                    Some('\\') => '\\',
                    Some('\'') => '\'',
                    Some('"') => '"',
                    Some('0') => '\0',
                    _ => inner.chars().next().unwrap_or(' '),
                }
            } else {
                inner.chars().next().unwrap_or(' ')
            };
            JavaNode::LiteralExpr {
                value: LiteralValue::Char(ch),
                span: lowerer.span(node),
            }
        }
        "true" => JavaNode::LiteralExpr {
            value: LiteralValue::Bool(true),
            span: lowerer.span(node),
        },
        "false" => JavaNode::LiteralExpr {
            value: LiteralValue::Bool(false),
            span: lowerer.span(node),
        },
        "null_literal" => JavaNode::LiteralExpr {
            value: LiteralValue::Null,
            span: lowerer.span(node),
        },

        // ── Names ───────────────────────────────────────────────────
        "identifier" | "this" | "super" => JavaNode::NameExpr {
            name: lowerer.node_text(node).to_string(),
            span: lowerer.span(node),
        },

        // ── Binary expressions ──────────────────────────────────────
        "binary_expression" => {
            let left = node.child_by_field_name("left")
                .map(|n| lower_expr(lowerer, n));
            let right = node.child_by_field_name("right")
                .map(|n| lower_expr(lowerer, n));
            let op = node.child_by_field_name("operator")
                .map(|n| lowerer.node_text(n))
                .and_then(text_to_binary_op)
                .unwrap_or(BinaryOp::Add);

            JavaNode::BinaryExpr {
                left: Box::new(left.unwrap_or_else(|| placeholder(lowerer, node))),
                op,
                right: Box::new(right.unwrap_or_else(|| placeholder(lowerer, node))),
                span: lowerer.span(node),
            }
        }

        // ── Unary expressions ───────────────────────────────────────
        "unary_expression" => {
            let operand = node.child_by_field_name("operand")
                .map(|n| lower_expr(lowerer, n))
                .unwrap_or_else(|| placeholder(lowerer, node));
            let op = node.child_by_field_name("operator")
                .map(|n| lowerer.node_text(n))
                .and_then(text_to_prefix_unary_op)
                .unwrap_or(UnaryOp::Negate);

            JavaNode::UnaryExpr {
                op,
                operand: Box::new(operand),
                span: lowerer.span(node),
            }
        }

        "update_expression" => {
            // ++x, x++, --x, x--
            // Determine pre vs post by checking if operator comes first
            let mut cursor = node.walk();
            let children: Vec<_> = node.children(&mut cursor).collect();
            let (op, operand) = if children.len() >= 2 {
                let first_text = lowerer.node_text(children[0]);
                if first_text == "++" || first_text == "--" {
                    // Prefix
                    let op = if first_text == "++" { UnaryOp::PreIncrement } else { UnaryOp::PreDecrement };
                    let operand = lower_expr(lowerer, children[1]);
                    (op, operand)
                } else {
                    // Postfix
                    let last_text = lowerer.node_text(children[children.len() - 1]);
                    let op = if last_text == "++" { UnaryOp::PostIncrement } else { UnaryOp::PostDecrement };
                    let operand = lower_expr(lowerer, children[0]);
                    (op, operand)
                }
            } else {
                (UnaryOp::PreIncrement, placeholder(lowerer, node))
            };

            JavaNode::UnaryExpr {
                op,
                operand: Box::new(operand),
                span: lowerer.span(node),
            }
        }

        // ── Assignment ──────────────────────────────────────────────
        "assignment_expression" => {
            let target = node.child_by_field_name("left")
                .map(|n| lower_expr(lowerer, n))
                .unwrap_or_else(|| placeholder(lowerer, node));
            let value = node.child_by_field_name("right")
                .map(|n| lower_expr(lowerer, n))
                .unwrap_or_else(|| placeholder(lowerer, node));

            JavaNode::AssignExpr {
                target: Box::new(target),
                value: Box::new(value),
                span: lowerer.span(node),
            }
        }

        // ── Parenthesized ───────────────────────────────────────────
        "parenthesized_expression" => {
            let inner = {
                let mut cursor = node.walk();
                let first = node.named_children(&mut cursor).next();
                first
            };
            inner
                .map(|n| lower_expr(lowerer, n))
                .unwrap_or_else(|| placeholder(lowerer, node))
        }

        // ── Cast ────────────────────────────────────────────────────
        "cast_expression" => {
            let target_type = node.child_by_field_name("type")
                .map(|t| lower_type(lowerer, t))
                .unwrap_or_else(|| JavaType {
                    name: "unknown".to_string(),
                    type_args: Vec::new(),
                    array_dimensions: 0,
                    is_varargs: false,
                });
            let expression = node.child_by_field_name("value")
                .map(|v| lower_expr(lowerer, v))
                .unwrap_or_else(|| placeholder(lowerer, node));

            JavaNode::CastExpr {
                target_type,
                expression: Box::new(expression),
                span: lowerer.span(node),
            }
        }

        // ── Method invocation ───────────────────────────────────────
        "method_invocation" => {
            let name = node.child_by_field_name("name")
                .map(|n| lowerer.node_text(n).to_string())
                .unwrap_or_default();

            let object = node.child_by_field_name("object")
                .map(|o| Box::new(lower_expr(lowerer, o)));

            let arguments = node.child_by_field_name("arguments")
                .map(|args| lower_argument_list(lowerer, args))
                .unwrap_or_default();

            let type_args = lowerer.child_by_kind(node, "type_arguments")
                .map(|ta| lower_type_arg_list(lowerer, ta))
                .unwrap_or_default();

            JavaNode::MethodCallExpr {
                object,
                name,
                arguments,
                type_args,
                span: lowerer.span(node),
            }
        }

        // ── Field access ────────────────────────────────────────────
        "field_access" => {
            let object = node.child_by_field_name("object")
                .map(|o| lower_expr(lowerer, o))
                .unwrap_or_else(|| placeholder(lowerer, node));
            let field = node.child_by_field_name("field")
                .map(|f| lowerer.node_text(f).to_string())
                .unwrap_or_default();

            JavaNode::FieldAccessExpr {
                object: Box::new(object),
                field,
                span: lowerer.span(node),
            }
        }

        // ── Object creation ─────────────────────────────────────────
        "object_creation_expression" => {
            let class_type = node.child_by_field_name("type")
                .map(|t| lower_type(lowerer, t))
                .unwrap_or_else(|| JavaType {
                    name: "unknown".to_string(),
                    type_args: Vec::new(),
                    array_dimensions: 0,
                    is_varargs: false,
                });

            let arguments = node.child_by_field_name("arguments")
                .map(|args| lower_argument_list(lowerer, args))
                .unwrap_or_default();

            JavaNode::NewExpr {
                class_type,
                arguments,
                span: lowerer.span(node),
            }
        }

        // ── Array creation ──────────────────────────────────────────
        "array_creation_expression" => {
            let element_type = node.child_by_field_name("type")
                .map(|t| lower_type(lowerer, t))
                .unwrap_or_else(|| JavaType {
                    name: "unknown".to_string(),
                    type_args: Vec::new(),
                    array_dimensions: 0,
                    is_varargs: false,
                });

            JavaNode::NewExpr {
                class_type: JavaType {
                    array_dimensions: element_type.array_dimensions + 1,
                    ..element_type
                },
                arguments: Vec::new(),
                span: lowerer.span(node),
            }
        }

        // ── Ternary ─────────────────────────────────────────────────
        "ternary_expression" => {
            let condition = node.child_by_field_name("condition")
                .map(|n| lower_expr(lowerer, n))
                .unwrap_or_else(|| placeholder(lowerer, node));
            let then_expr = node.child_by_field_name("consequence")
                .map(|n| lower_expr(lowerer, n))
                .unwrap_or_else(|| placeholder(lowerer, node));
            let else_expr = node.child_by_field_name("alternative")
                .map(|n| lower_expr(lowerer, n))
                .unwrap_or_else(|| placeholder(lowerer, node));

            JavaNode::TernaryExpr {
                condition: Box::new(condition),
                then_expr: Box::new(then_expr),
                else_expr: Box::new(else_expr),
                span: lowerer.span(node),
            }
        }

        // ── instanceof ──────────────────────────────────────────────
        "instanceof_expression" => {
            let left = node.child_by_field_name("left")
                .map(|n| lower_expr(lowerer, n))
                .unwrap_or_else(|| placeholder(lowerer, node));
            let right = node.child_by_field_name("right")
                .map(|n| JavaNode::TypeRef {
                    java_type: lower_type(lowerer, n),
                    span: lowerer.span(n),
                })
                .unwrap_or_else(|| placeholder(lowerer, node));

            JavaNode::BinaryExpr {
                left: Box::new(left),
                op: BinaryOp::InstanceOf,
                right: Box::new(right),
                span: lowerer.span(node),
            }
        }

        // ── Lambda ──────────────────────────────────────────────────
        "lambda_expression" => {
            let parameters = lower_lambda_params(lowerer, node);

            let body_node = node.child_by_field_name("body")
                .unwrap_or(node);
            let body = match body_node.kind() {
                "block" => lower_stmt(lowerer, body_node),
                _ => lower_expr(lowerer, body_node),
            };

            JavaNode::LambdaExpr {
                parameters,
                body: Box::new(body),
                span: lowerer.span(node),
            }
        }

        // ── Array access ────────────────────────────────────────────
        "array_access" => {
            let array = node.child_by_field_name("array")
                .map(|n| lower_expr(lowerer, n))
                .unwrap_or_else(|| placeholder(lowerer, node));
            let index = node.child_by_field_name("index")
                .map(|n| lower_expr(lowerer, n))
                .unwrap_or_else(|| placeholder(lowerer, node));

            // Model array access as a method call to "get" for simplicity
            JavaNode::MethodCallExpr {
                object: Some(Box::new(array)),
                name: "[]".to_string(),
                arguments: vec![index],
                type_args: Vec::new(),
                span: lowerer.span(node),
            }
        }

        // ── Class literal (e.g., String.class) ──────────────────────
        "class_literal" => {
            JavaNode::FieldAccessExpr {
                object: Box::new(JavaNode::NameExpr {
                    name: lowerer.node_text(node).trim_end_matches(".class").to_string(),
                    span: lowerer.span(node),
                }),
                field: "class".to_string(),
                span: lowerer.span(node),
            }
        }

        // ── Fallback ────────────────────────────────────────────────
        _ => {
            log::debug!("unknown expr kind: {} text: {}", node.kind(), lowerer.node_text(node));
            JavaNode::NameExpr {
                name: lowerer.node_text(node).to_string(),
                span: lowerer.span(node),
            }
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

fn placeholder(lowerer: &Lowerer, node: Node) -> JavaNode {
    JavaNode::NameExpr {
        name: "<missing>".to_string(),
        span: lowerer.span(node),
    }
}

fn text_to_binary_op(text: &str) -> Option<BinaryOp> {
    match text {
        "+" => Some(BinaryOp::Add),
        "-" => Some(BinaryOp::Sub),
        "*" => Some(BinaryOp::Mul),
        "/" => Some(BinaryOp::Div),
        "%" => Some(BinaryOp::Mod),
        "&&" => Some(BinaryOp::And),
        "||" => Some(BinaryOp::Or),
        "&" => Some(BinaryOp::BitwiseAnd),
        "|" => Some(BinaryOp::BitwiseOr),
        "^" => Some(BinaryOp::BitwiseXor),
        "<<" => Some(BinaryOp::ShiftLeft),
        ">>" => Some(BinaryOp::ShiftRight),
        ">>>" => Some(BinaryOp::UnsignedShiftRight),
        "==" => Some(BinaryOp::Eq),
        "!=" => Some(BinaryOp::Ne),
        "<" => Some(BinaryOp::Lt),
        ">" => Some(BinaryOp::Gt),
        "<=" => Some(BinaryOp::Le),
        ">=" => Some(BinaryOp::Ge),
        "instanceof" => Some(BinaryOp::InstanceOf),
        _ => None,
    }
}

fn text_to_prefix_unary_op(text: &str) -> Option<UnaryOp> {
    match text {
        "-" => Some(UnaryOp::Negate),
        "!" => Some(UnaryOp::Not),
        "~" => Some(UnaryOp::BitwiseNot),
        "+" => Some(UnaryOp::Negate), // unary plus treated as identity
        _ => None,
    }
}

fn lower_argument_list(lowerer: &Lowerer, node: Node) -> Vec<JavaNode> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .map(|child| lower_expr(lowerer, child))
        .collect()
}

fn lower_type_arg_list(lowerer: &Lowerer, node: Node) -> Vec<JavaType> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .map(|child| lower_type(lowerer, child))
        .collect()
}

fn lower_lambda_params(lowerer: &Lowerer, node: Node) -> Vec<JavaNode> {
    // Lambda parameters can be: identifier, formal_parameters, or inferred_parameters
    let params_node = node.child_by_field_name("parameters");
    match params_node {
        Some(p) if p.kind() == "formal_parameters" => {
            let mut result = Vec::new();
            let mut cursor = p.walk();
            for child in p.named_children(&mut cursor) {
                if child.kind() == "formal_parameter" || child.kind() == "spread_parameter" {
                    result.push(lowerer.lower_parameter(child));
                }
            }
            result
        }
        Some(p) if p.kind() == "inferred_parameters" => {
            let mut result = Vec::new();
            let mut cursor = p.walk();
            for child in p.named_children(&mut cursor) {
                if child.kind() == "identifier" {
                    result.push(JavaNode::Parameter {
                        name: lowerer.node_text(child).to_string(),
                        param_type: JavaType {
                            name: "var".to_string(),
                            type_args: Vec::new(),
                            array_dimensions: 0,
                            is_varargs: false,
                        },
                        annotations: Vec::new(),
                        span: lowerer.span(child),
                    });
                }
            }
            result
        }
        Some(p) if p.kind() == "identifier" => {
            vec![JavaNode::Parameter {
                name: lowerer.node_text(p).to_string(),
                param_type: JavaType {
                    name: "var".to_string(),
                    type_args: Vec::new(),
                    array_dimensions: 0,
                    is_varargs: false,
                },
                annotations: Vec::new(),
                span: lowerer.span(p),
            }]
        }
        _ => Vec::new(),
    }
}
