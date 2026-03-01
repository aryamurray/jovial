use std::collections::HashSet;

use jovial_ast::go::{GoNode, GoReceiver, GoType, GoUnaryOp};
use jovial_ast::java::{JavaNode, JavaType, Modifier};
use jovial_ast::span::Span;

use crate::convert_helpers::*;
use crate::default_convert::DefaultConverter;
use crate::walker::WalkError;

pub(crate) fn convert_class_decl(
    converter: &DefaultConverter,
    name: &str,
    members: &[JavaNode],
    span: &Span,
    walk_child: &dyn Fn(&JavaNode) -> Result<Vec<GoNode>, WalkError>,
) -> Result<Vec<GoNode>, WalkError> {
    let mut fields = Vec::new();
    let mut methods = Vec::new();

    // Collect field names so NameExpr can resolve bare field references
    let mut field_names: HashSet<String> = HashSet::new();
    for member in members {
        if let JavaNode::FieldDecl { name: field_name, .. } = member {
            field_names.insert(field_name.clone());
        }
    }

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
                let converted = converter.walk_in_class(member, walk_child, name, &field_names)?;
                methods.extend(converted);
            }
        }
    }

    let mut result = vec![GoNode::StructDecl {
        name: name.to_string(),
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
    for (i, constant) in constants.iter().enumerate() {
        let value_node = if i == 0 {
            GoNode::Ident {
                name: "iota".to_string(),
                span: span.clone(),
            }
        } else {
            GoNode::Ident {
                name: "".to_string(),
                span: span.clone(),
            }
        };
        result.push(GoNode::ConstDecl {
            name: format!("{}_{}", name, constant),
            const_type: Some(simple_go_type(name)),
            value: Box::new(value_node),
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

    let mut go_body = flatten_block(walk_child(body)?);

    let has_return = go_body.iter().any(|n| matches!(n, GoNode::ReturnStmt { .. }));
    if !has_return {
        go_body.push(GoNode::ReturnStmt {
            values: vec![GoNode::UnaryExpr {
                op: GoUnaryOp::Addr,
                operand: Box::new(GoNode::CompositeLit {
                    lit_type: simple_go_type(class_name),
                    elements: vec![],
                    span: span.clone(),
                }),
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
