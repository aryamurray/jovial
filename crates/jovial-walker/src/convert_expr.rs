use std::collections::HashSet;

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
    current_class: Option<&str>,
    superclass: Option<&str>,
) -> Result<Vec<GoNode>, WalkError> {
    // super(...) constructor call → parent struct init via embedding
    if method_name == "super" && object.is_none() {
        if let Some(parent) = superclass {
            let walked_args: Vec<GoNode> = arguments
                .iter()
                .map(|a| walk_child(a).map(|nodes| nodes.into_iter().next()))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .flatten()
                .collect();

            let receiver = current_class
                .map(|cls| receiver_name(cls))
                .unwrap_or_else(|| "t".to_string());

            // t.ParentType = *NewParentType(args...)
            return Ok(vec![GoNode::AssignStmt {
                lhs: vec![GoNode::SelectorExpr {
                    object: Box::new(GoNode::Ident {
                        name: receiver,
                        span: span.clone(),
                    }),
                    field: parent.to_string(),
                    span: span.clone(),
                }],
                rhs: vec![GoNode::UnaryExpr {
                    op: GoUnaryOp::Deref,
                    operand: Box::new(GoNode::CallExpr {
                        function: Box::new(GoNode::Ident {
                            name: format!("New{}", parent),
                            span: span.clone(),
                        }),
                        args: walked_args,
                        span: span.clone(),
                    }),
                    span: span.clone(),
                }],
                define: false,
                span: span.clone(),
            }]);
        }
        // No superclass known — fall back to comment
        let arg_strs: Vec<String> = arguments
            .iter()
            .filter_map(|a| {
                walk_child(a).ok().and_then(|nodes| {
                    nodes.into_iter().next().and_then(|n| {
                        if let GoNode::Ident { name, .. } = &n {
                            Some(name.clone())
                        } else if let GoNode::Literal { value, .. } = &n {
                            Some(format!("{:?}", value))
                        } else {
                            Some("...".to_string())
                        }
                    })
                })
            })
            .collect();
        return Ok(vec![GoNode::RawCode {
            code: format!("// super({})", arg_strs.join(", ")),
            span: span.clone(),
        }]);
    }

    // this(...) constructor delegation → comment placeholder
    if method_name == "this" && object.is_none() {
        let arg_strs: Vec<String> = arguments
            .iter()
            .filter_map(|a| {
                walk_child(a).ok().and_then(|nodes| {
                    nodes.into_iter().next().and_then(|n| {
                        if let GoNode::Ident { name, .. } = &n {
                            Some(name.clone())
                        } else if let GoNode::Literal { value, .. } = &n {
                            Some(format!("{:?}", value))
                        } else {
                            Some("...".to_string())
                        }
                    })
                })
            })
            .collect();
        return Ok(vec![GoNode::RawCode {
            code: format!("// this({})", arg_strs.join(", ")),
            span: span.clone(),
        }]);
    }

    // super.method(...) → comment placeholder
    if let Some(obj) = object {
        if let JavaNode::NameExpr { name, .. } = obj {
            if name == "super" {
                let arg_strs: Vec<String> = arguments
                    .iter()
                    .filter_map(|a| {
                        walk_child(a).ok().and_then(|nodes| {
                            nodes.into_iter().next().and_then(|n| {
                                if let GoNode::Ident { name, .. } = &n {
                                    Some(name.clone())
                                } else {
                                    Some("...".to_string())
                                }
                            })
                        })
                    })
                    .collect();
                return Ok(vec![GoNode::RawCode {
                    code: format!(
                        "// super.{}({})",
                        method_name,
                        arg_strs.join(", ")
                    ),
                    span: span.clone(),
                }]);
            }
        }
    }

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
    class_fields: Option<&HashSet<String>>,
) -> Result<Vec<GoNode>, WalkError> {
    if name == "this" {
        let go_name = current_class
            .map(|c| receiver_name(c))
            .unwrap_or_else(|| "this".to_string());
        return Ok(vec![GoNode::Ident {
            name: go_name,
            span: span.clone(),
        }]);
    }

    // If we're inside a class and the name matches a field, emit receiver.FieldName
    if let (Some(cls), Some(fields)) = (current_class, class_fields) {
        if fields.contains(name) {
            return Ok(vec![GoNode::SelectorExpr {
                object: Box::new(GoNode::Ident {
                    name: receiver_name(cls),
                    span: span.clone(),
                }),
                field: java_name_to_go_exported(name),
                span: span.clone(),
            }]);
        }
    }

    Ok(vec![GoNode::Ident {
        name: name.to_string(),
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
            // instanceof → TypeAssertExpr (Go type assertion)
            let left_node = walk_child(left)?
                .into_iter()
                .next()
                .unwrap_or(GoNode::Ident {
                    name: "expr".to_string(),
                    span: span.clone(),
                });
            let right_type = walk_child(right)?
                .into_iter()
                .next()
                .and_then(|n| {
                    if let GoNode::TypeRef { go_type, .. } = n {
                        Some(go_type)
                    } else if let GoNode::Ident { name, .. } = &n {
                        Some(jovial_ast::go::GoType {
                            name: name.clone(),
                            package: None,
                            is_pointer: false,
                            is_slice: false,
                            is_map: false,
                            key_type: None,
                            value_type: None,
                        })
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| jovial_ast::go::GoType {
                    name: "interface{}".to_string(),
                    package: None,
                    is_pointer: false,
                    is_slice: false,
                    is_map: false,
                    key_type: None,
                    value_type: None,
                });
            Ok(vec![GoNode::TypeAssertExpr {
                expr: Box::new(left_node),
                assert_type: right_type,
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
            // Pre/Post increment/decrement → Go's i++ / i-- (statement only)
            let operand_node = walk_child(operand)?
                .into_iter()
                .next()
                .unwrap_or(GoNode::Ident {
                    name: "_".to_string(),
                    span: span.clone(),
                });
            let is_increment =
                matches!(op, UnaryOp::PreIncrement | UnaryOp::PostIncrement);
            Ok(vec![GoNode::IncDecStmt {
                operand: Box::new(operand_node),
                is_increment,
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
        let go_type = convert_java_type(class_type);
        // Maps and slices are already reference types in Go — no & needed
        let lit = GoNode::CompositeLit {
            lit_type: go_type.clone(),
            elements: vec![],
            span: span.clone(),
        };
        if go_type.is_map || go_type.is_slice {
            Ok(vec![lit])
        } else {
            Ok(vec![GoNode::UnaryExpr {
                op: GoUnaryOp::Addr,
                operand: Box::new(lit),
                span: span.clone(),
            }])
        }
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

    let then_vals = walk_child(then_expr)?;
    let else_vals = walk_child(else_expr)?;

    // Emit as an immediately-invoked function literal (IIFE):
    //   func() T { if cond { return thenVal } else { return elseVal } }()
    // This is a valid Go expression that works in any context (assignment, concat, arg).
    Ok(vec![GoNode::CallExpr {
        function: Box::new(GoNode::FuncDecl {
            name: String::new(), // anonymous
            receiver: None,
            params: vec![],
            returns: vec![], // Go infers return type
            body: vec![GoNode::IfStmt {
                init: None,
                condition: Box::new(cond),
                body: vec![GoNode::ReturnStmt {
                    values: then_vals,
                    span: span.clone(),
                }],
                else_body: Some(vec![GoNode::ReturnStmt {
                    values: else_vals,
                    span: span.clone(),
                }]),
                span: span.clone(),
            }],
            span: span.clone(),
        }),
        args: vec![],
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
