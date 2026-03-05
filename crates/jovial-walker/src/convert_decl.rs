use jovial_ast::go::{GoNode, GoReceiver, GoType, GoUnaryOp};
use jovial_ast::java::{JavaNode, JavaType, Modifier};
use jovial_ast::span::Span;

use crate::convert_helpers::*;
use crate::default_convert::DefaultConverter;
use crate::walker::WalkError;

pub(crate) fn convert_class_decl(
    _converter: &DefaultConverter,
    name: &str,
    superclass: Option<&JavaType>,
    members: &[JavaNode],
    span: &Span,
    walk_child: &dyn Fn(&JavaNode) -> Result<Vec<GoNode>, WalkError>,
) -> Result<Vec<GoNode>, WalkError> {
    let mut fields = Vec::new();
    let mut methods = Vec::new();

    for member in members {
        match member {
            JavaNode::FieldDecl {
                name: field_name,
                field_type,
                span: f_span,
                ..
            } => {
                fields.push(GoNode::FieldDecl {
                    name: java_name_to_go_exported(field_name),
                    field_type: convert_java_type(field_type),
                    tag: None,
                    span: f_span.clone(),
                });
            }
            _ => {
                // walk_child goes through the Walker which handles both
                // plugin dispatch and class context threading.
                let converted = walk_child(member)?;
                methods.extend(converted);
            }
        }
    }

    let embedded = superclass
        .map(|sc| vec![convert_java_type(sc)])
        .unwrap_or_default();

    let mut result = vec![GoNode::StructDecl {
        name: name.to_string(),
        embedded,
        fields,
        span: span.clone(),
    }];
    result.extend(methods);
    Ok(result)
}

pub(crate) fn convert_interface_decl(
    name: &str,
    members: &[JavaNode],
    span: &Span,
) -> Result<Vec<GoNode>, WalkError> {
    let mut method_sigs = Vec::new();
    for member in members {
        if let JavaNode::MethodDecl {
            name: method_name,
            return_type,
            parameters,
            span: m_span,
            ..
        } = member
        {
            method_sigs.push(GoNode::FuncDecl {
                name: java_name_to_go_exported(method_name),
                receiver: None,
                params: convert_params(parameters),
                returns: return_type_to_go(return_type.as_ref()),
                body: vec![],
                span: m_span.clone(),
            });
        }
    }
    Ok(vec![GoNode::InterfaceDecl {
        name: name.to_string(),
        methods: method_sigs,
        span: span.clone(),
    }])
}

pub(crate) fn convert_enum_decl(
    name: &str,
    constants: &[String],
    span: &Span,
) -> Result<Vec<GoNode>, WalkError> {
    let mut result = Vec::new();
    result.push(GoNode::RawCode {
        code: format!("type {} string", name),
        span: span.clone(),
    });
    let decls: Vec<GoNode> = constants
        .iter()
        .map(|constant| GoNode::ConstDecl {
            name: format!("{}_{}", name, constant),
            const_type: Some(simple_go_type(name)),
            value: Box::new(GoNode::Literal {
                value: jovial_ast::go::GoLiteralValue::String(constant.clone()),
                span: span.clone(),
            }),
            span: span.clone(),
        })
        .collect();
    if !decls.is_empty() {
        result.push(GoNode::ConstBlock {
            decls,
            span: span.clone(),
        });
    }
    Ok(result)
}

pub(crate) fn convert_method_decl(
    method_name: &str,
    modifiers: &[Modifier],
    return_type: Option<&JavaType>,
    parameters: &[JavaNode],
    body: Option<&JavaNode>,
    span: &Span,
    walk_child: &dyn Fn(&JavaNode) -> Result<Vec<GoNode>, WalkError>,
    current_class: Option<&str>,
) -> Result<Vec<GoNode>, WalkError> {
    let is_static = is_static(modifiers);
    let receiver = if is_static {
        None
    } else {
        current_class.map(|cls| GoReceiver {
            name: receiver_name(cls),
            receiver_type: simple_go_type(cls),
            is_pointer: true,
        })
    };

    let go_body = match body {
        Some(b) => flatten_block(walk_child(b)?),
        None => vec![],
    };

    Ok(vec![GoNode::FuncDecl {
        name: java_name_to_go_exported(method_name),
        receiver,
        params: convert_params(parameters),
        returns: return_type_to_go(return_type),
        body: go_body,
        span: span.clone(),
    }])
}

pub(crate) fn convert_constructor_decl(
    class_name: &str,
    parameters: &[JavaNode],
    body: &JavaNode,
    span: &Span,
    walk_child: &dyn Fn(&JavaNode) -> Result<Vec<GoNode>, WalkError>,
) -> Result<Vec<GoNode>, WalkError> {
    let return_type = GoType {
        name: class_name.to_string(),
        package: None,
        is_pointer: true,
        is_slice: false,
        is_map: false,
        key_type: None,
        value_type: None,
    };

    // Build a map from Go exported field name → Java parameter name.
    // In Java constructors, `this.title = title` means the param and field share a name.
    // After walking, both sides resolve to `t.Title = t.Title` because the walker
    // sees `title` as a field reference. We post-process to fix the RHS back to the param.
    let mut field_to_param: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for p in parameters {
        if let JavaNode::Parameter { name, .. } = p {
            let go_field = java_name_to_go_exported(name);
            field_to_param.insert(go_field, name.clone());
        }
    }

    let receiver = receiver_name(class_name);

    let mut go_body = flatten_block(walk_child(body)?);

    // Post-process: fix self-assignments like `t.Title = t.Title` → `t.Title = title`
    fix_constructor_self_assignments(&mut go_body, &receiver, &field_to_param);

    // Prepend: t := &ClassName{}
    go_body.insert(
        0,
        GoNode::AssignStmt {
            lhs: vec![GoNode::Ident {
                name: receiver.clone(),
                span: span.clone(),
            }],
            rhs: vec![GoNode::UnaryExpr {
                op: GoUnaryOp::Addr,
                operand: Box::new(GoNode::CompositeLit {
                    lit_type: simple_go_type(class_name),
                    elements: vec![],
                    span: span.clone(),
                }),
                span: span.clone(),
            }],
            define: true,
            span: span.clone(),
        },
    );

    // Append or fix return: use `return t` instead of `return &ClassName{}`
    let has_return = go_body.iter().any(|n| matches!(n, GoNode::ReturnStmt { .. }));
    if !has_return {
        go_body.push(GoNode::ReturnStmt {
            values: vec![GoNode::Ident {
                name: receiver.clone(),
                span: span.clone(),
            }],
            span: span.clone(),
        });
    }

    Ok(vec![GoNode::FuncDecl {
        name: format!("New{}", class_name),
        receiver: None,
        params: convert_params(parameters),
        returns: vec![return_type],
        body: go_body,
        span: span.clone(),
    }])
}

/// Fix constructor self-assignments: `receiver.Field = receiver.Field` → `receiver.Field = param`
///
/// When Java has `this.title = title`, the walker converts both sides to `t.Title`
/// because bare `title` matches a class field. This function detects that pattern and
/// replaces the RHS with the original parameter name.
fn fix_constructor_self_assignments(
    body: &mut Vec<GoNode>,
    receiver: &str,
    field_to_param: &std::collections::HashMap<String, String>,
) {
    for node in body.iter_mut() {
        // First, extract info without holding borrows
        let replacement = match node {
            GoNode::AssignStmt { lhs, rhs, .. } => {
                match (lhs.first(), rhs.first()) {
                    (
                        Some(GoNode::SelectorExpr { object: lhs_obj, field: lhs_field, .. }),
                        Some(GoNode::SelectorExpr { object: rhs_obj, field: rhs_field, .. }),
                    ) => {
                        let lhs_is_receiver = matches!(lhs_obj.as_ref(), GoNode::Ident { name, .. } if name == receiver);
                        let rhs_is_receiver = matches!(rhs_obj.as_ref(), GoNode::Ident { name, .. } if name == receiver);
                        if lhs_is_receiver && rhs_is_receiver && lhs_field == rhs_field {
                            field_to_param.get(lhs_field.as_str()).cloned()
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            }
            _ => None,
        };

        // Now apply the replacement
        if let Some(param_name) = replacement {
            if let GoNode::AssignStmt { rhs, span, .. } = node {
                *rhs = vec![GoNode::Ident {
                    name: param_name,
                    span: span.clone(),
                }];
            }
        }
    }
}

pub(crate) fn convert_field_decl(
    field_name: &str,
    field_type: &JavaType,
    initializer: Option<&JavaNode>,
    span: &Span,
    walk_child: &dyn Fn(&JavaNode) -> Result<Vec<GoNode>, WalkError>,
    current_class: Option<&str>,
) -> Result<Vec<GoNode>, WalkError> {
    if current_class.is_none() {
        let value = match initializer {
            Some(init) => {
                let walked = walk_child(init)?;
                walked.into_iter().next().map(Box::new)
            }
            None => None,
        };
        Ok(vec![GoNode::VarDecl {
            name: field_name.to_string(),
            var_type: Some(convert_java_type(field_type)),
            value,
            span: span.clone(),
        }])
    } else {
        Ok(vec![GoNode::FieldDecl {
            name: java_name_to_go_exported(field_name),
            field_type: convert_java_type(field_type),
            tag: None,
            span: span.clone(),
        }])
    }
}
