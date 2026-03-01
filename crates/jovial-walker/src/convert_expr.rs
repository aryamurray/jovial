use jovial_ast::go::{GoBinaryOp, GoLiteralValue, GoNode, GoUnaryOp};
use jovial_ast::java::{BinaryOp, JavaNode, JavaType, LiteralValue, UnaryOp};
use jovial_ast::span::Span;

use crate::convert_helpers::*;
use crate::walker::WalkError;

pub(crate) fn convert_method_call_expr(
    object: Option<&JavaNode>,
    method_name: &str,
    arguments: &[JavaNode],
    span: &Span,
    walk_child: &dyn Fn(&JavaNode) -> Result<Vec<GoNode>, WalkError>,
) -> Result<Vec<GoNode>, WalkError> {
    let mut args = Vec::new();
    for arg in arguments {
        args.extend(walk_child(arg)?);
    }

    let function = match object {
        Some(obj) => {
            let obj_node = walk_child(obj)?
                .into_iter()
                .next()
                .unwrap_or(GoNode::Ident {
                    name: "obj".to_string(),
                    span: span.clone(),
                });
            GoNode::SelectorExpr {
                object: Box::new(obj_node),
                field: java_name_to_go_exported(method_name),
                span: span.clone(),
            }
        }
        None => GoNode::Ident {
            name: java_name_to_go_exported(method_name),
            span: span.clone(),
        },
    };

    Ok(vec![GoNode::CallExpr {
        function: Box::new(function),
        args,
        span: span.clone(),
    }])
}

pub(crate) fn convert_field_access_expr(
    object: &JavaNode,
    field: &str,
    span: &Span,
    walk_child: &dyn Fn(&JavaNode) -> Result<Vec<GoNode>, WalkError>,
) -> Result<Vec<GoNode>, WalkError> {
    let obj_node = walk_child(object)?
        .into_iter()
        .next()
        .unwrap_or(GoNode::Ident {
            name: "obj".to_string(),
            span: span.clone(),
        });
    Ok(vec![GoNode::SelectorExpr {
        object: Box::new(obj_node),
        field: java_name_to_go_exported(field),
        span: span.clone(),
    }])
}

pub(crate) fn convert_name_expr(
    name: &str,
    span: &Span,
    current_class: Option<&str>,
) -> Result<Vec<GoNode>, WalkError> {
    let go_name = if name == "this" {
        current_class
            .map(|c| receiver_name(c))
            .unwrap_or_else(|| "this".to_string())
    } else {
        name.to_string()
    };
    Ok(vec![GoNode::Ident {
        name: go_name,
        span: span.clone(),
    }])
}

pub(crate) fn convert_literal_expr(
    value: &LiteralValue,
    span: &Span,
) -> Result<Vec<GoNode>, WalkError> {
    Ok(vec![GoNode::Literal {
        value: convert_literal(value),
        span: span.clone(),
    }])
}

pub(crate) fn convert_binary_expr(
    left: &JavaNode,
    op: &BinaryOp,
    right: &JavaNode,
    span: &Span,
    walk_child: &dyn Fn(&JavaNode) -> Result<Vec<GoNode>, WalkError>,
) -> Result<Vec<GoNode>, WalkError> {
    match convert_binary_op(op) {
        Some(go_op) => {
            let left_node = walk_child(left)?
                .into_iter()
                .next()
                .unwrap_or(GoNode::Ident {
                    name: "_".to_string(),
                    span: span.clone(),
                });
            let right_node = walk_child(right)?
                .into_iter()
                .next()
                .unwrap_or(GoNode::Ident {
                    name: "_".to_string(),
                    span: span.clone(),
                });
            Ok(vec![GoNode::BinaryExpr {
                left: Box::new(left_node),
                op: go_op,
                right: Box::new(right_node),
                span: span.clone(),
            }])
        }
        None => {
            // instanceof → RawCode
            let left_str = walk_child(left)?
                .first()
                .and_then(|n| {
                    if let GoNode::Ident { name, .. } = n {
                        Some(name.clone())
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| "expr".to_string());
            let right_str = walk_child(right)?
                .first()
                .and_then(|n| {
                    if let GoNode::Ident { name, .. } = n {
                        Some(name.clone())
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| "Type".to_string());
            Ok(vec![GoNode::RawCode {
                code: format!(
                    "// TODO: instanceof; consider type assertion: _, ok := {}.({});",
                    left_str, right_str
                ),
                span: span.clone(),
            }])
        }
    }
}

pub(crate) fn convert_unary_expr(
    op: &UnaryOp,
    operand: &JavaNode,
    span: &Span,
    walk_child: &dyn Fn(&JavaNode) -> Result<Vec<GoNode>, WalkError>,
) -> Result<Vec<GoNode>, WalkError> {
    match convert_unary_op(op) {
        Some(go_op) => {
            let operand_node = walk_child(operand)?
                .into_iter()
                .next()
                .unwrap_or(GoNode::Ident {
                    name: "_".to_string(),
                    span: span.clone(),
                });
            Ok(vec![GoNode::UnaryExpr {
                op: go_op,
                operand: Box::new(operand_node),
                span: span.clone(),
            }])
        }
        None => {
            // Pre/Post increment/decrement → x = x + 1 / x = x - 1
            let operand_node = walk_child(operand)?
                .into_iter()
                .next()
                .unwrap_or(GoNode::Ident {
                    name: "_".to_string(),
                    span: span.clone(),
                });
            let bin_op = if matches!(op, UnaryOp::PreIncrement | UnaryOp::PostIncrement) {
                GoBinaryOp::Add
            } else {
                GoBinaryOp::Sub
            };
            Ok(vec![GoNode::AssignStmt {
                lhs: vec![operand_node.clone()],
                rhs: vec![GoNode::BinaryExpr {
                    left: Box::new(operand_node),
                    op: bin_op,
                    right: Box::new(GoNode::Literal {
                        value: GoLiteralValue::Int(1),
                        span: span.clone(),
                    }),
                    span: span.clone(),
                }],
                define: false,
                span: span.clone(),
            }])
        }
    }
}

pub(crate) fn convert_assign_expr(
    target: &JavaNode,
    value: &JavaNode,
    span: &Span,
    walk_child: &dyn Fn(&JavaNode) -> Result<Vec<GoNode>, WalkError>,
) -> Result<Vec<GoNode>, WalkError> {
    let lhs = walk_child(target)?;
    let rhs = walk_child(value)?;
    Ok(vec![GoNode::AssignStmt {
        lhs,
        rhs,
        define: false,
        span: span.clone(),
    }])
}

pub(crate) fn convert_variable_decl(
    var_name: &str,
    var_type: Option<&JavaType>,
    initializer: Option<&JavaNode>,
    span: &Span,
    walk_child: &dyn Fn(&JavaNode) -> Result<Vec<GoNode>, WalkError>,
) -> Result<Vec<GoNode>, WalkError> {
    match initializer {
        Some(init) => {
            let rhs = walk_child(init)?;
            Ok(vec![GoNode::AssignStmt {
                lhs: vec![GoNode::Ident {
                    name: var_name.to_string(),
                    span: span.clone(),
                }],
                rhs,
                define: true,
                span: span.clone(),
            }])
        }
        None => Ok(vec![GoNode::VarDecl {
            name: var_name.to_string(),
            var_type: var_type.map(convert_java_type),
            value: None,
            span: span.clone(),
        }]),
    }
}

pub(crate) fn convert_new_expr(
    class_type: &JavaType,
    arguments: &[JavaNode],
    span: &Span,
    walk_child: &dyn Fn(&JavaNode) -> Result<Vec<GoNode>, WalkError>,
) -> Result<Vec<GoNode>, WalkError> {
    if arguments.is_empty() {
        Ok(vec![GoNode::UnaryExpr {
            op: GoUnaryOp::Addr,
            operand: Box::new(GoNode::CompositeLit {
                lit_type: convert_java_type(class_type),
                elements: vec![],
                span: span.clone(),
            }),
            span: span.clone(),
        }])
    } else {
        let mut args = Vec::new();
        for arg in arguments {
            args.extend(walk_child(arg)?);
        }
        Ok(vec![GoNode::CallExpr {
            function: Box::new(GoNode::Ident {
                name: format!("New{}", class_type.name),
                span: span.clone(),
            }),
            args,
            span: span.clone(),
        }])
    }
}

pub(crate) fn convert_ternary_expr(
    condition: &JavaNode,
    then_expr: &JavaNode,
    else_expr: &JavaNode,
    span: &Span,
    walk_child: &dyn Fn(&JavaNode) -> Result<Vec<GoNode>, WalkError>,
) -> Result<Vec<GoNode>, WalkError> {
    let cond = walk_child(condition)?
        .into_iter()
        .next()
        .unwrap_or(GoNode::Ident {
            name: "true".to_string(),
            span: span.clone(),
        });

    Ok(vec![GoNode::IfStmt {
        init: None,
        condition: Box::new(cond),
        body: walk_child(then_expr)?,
        else_body: Some(walk_child(else_expr)?),
        span: span.clone(),
    }])
}

pub(crate) fn convert_cast_expr(
    target_type: &JavaType,
    expression: &JavaNode,
    span: &Span,
    walk_child: &dyn Fn(&JavaNode) -> Result<Vec<GoNode>, WalkError>,
) -> Result<Vec<GoNode>, WalkError> {
    let go_type = convert_java_type(target_type);
    Ok(vec![GoNode::CallExpr {
        function: Box::new(GoNode::Ident {
            name: go_type.name.clone(),
            span: span.clone(),
        }),
        args: walk_child(expression)?,
        span: span.clone(),
    }])
}

pub(crate) fn convert_lambda_expr(
    parameters: &[JavaNode],
    body: &JavaNode,
    span: &Span,
    walk_child: &dyn Fn(&JavaNode) -> Result<Vec<GoNode>, WalkError>,
) -> Result<Vec<GoNode>, WalkError> {
    Ok(vec![GoNode::FuncDecl {
        name: String::new(),
        receiver: None,
        params: convert_params(parameters),
        returns: vec![],
        body: flatten_block(walk_child(body)?),
        span: span.clone(),
    }])
}

pub(crate) fn convert_type_ref(
    java_type: &JavaType,
    span: &Span,
) -> Result<Vec<GoNode>, WalkError> {
    Ok(vec![GoNode::TypeRef {
        go_type: convert_java_type(java_type),
        span: span.clone(),
    }])
}
