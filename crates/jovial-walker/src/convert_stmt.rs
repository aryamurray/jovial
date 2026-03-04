use jovial_ast::go::GoNode;
use jovial_ast::java::JavaNode;
use jovial_ast::span::Span;

use crate::convert_helpers::flatten_block;
use crate::walker::WalkError;

pub(crate) fn convert_block_stmt(
    statements: &[JavaNode],
    span: &Span,
    walk_child: &dyn Fn(&JavaNode) -> Result<Vec<GoNode>, WalkError>,
) -> Result<Vec<GoNode>, WalkError> {
    let mut stmts = Vec::new();
    for s in statements {
        stmts.extend(walk_child(s)?);
    }
    Ok(vec![GoNode::BlockStmt {
        statements: stmts,
        span: span.clone(),
    }])
}

pub(crate) fn convert_return_stmt(
    value: Option<&JavaNode>,
    span: &Span,
    walk_child: &dyn Fn(&JavaNode) -> Result<Vec<GoNode>, WalkError>,
) -> Result<Vec<GoNode>, WalkError> {
    let values = match value {
        Some(v) => walk_child(v)?,
        None => vec![],
    };
    Ok(vec![GoNode::ReturnStmt {
        values,
        span: span.clone(),
    }])
}

pub(crate) fn convert_if_stmt(
    condition: &JavaNode,
    then_branch: &JavaNode,
    else_branch: Option<&JavaNode>,
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

    let body = flatten_block(walk_child(then_branch)?);

    let else_body = match else_branch {
        Some(eb) => Some(flatten_block(walk_child(eb)?)),
        None => None,
    };

    Ok(vec![GoNode::IfStmt {
        init: None,
        condition: Box::new(cond),
        body,
        else_body,
        span: span.clone(),
    }])
}

pub(crate) fn convert_for_stmt(
    init: Option<&JavaNode>,
    condition: Option<&JavaNode>,
    update: &[JavaNode],
    body: &JavaNode,
    span: &Span,
    walk_child: &dyn Fn(&JavaNode) -> Result<Vec<GoNode>, WalkError>,
) -> Result<Vec<GoNode>, WalkError> {
    let go_init = match init {
        Some(i) => walk_child(i)?.into_iter().next().map(Box::new),
        None => None,
    };
    let go_cond = match condition {
        Some(c) => walk_child(c)?.into_iter().next().map(Box::new),
        None => None,
    };
    let go_post = if !update.is_empty() {
        walk_child(&update[0])?.into_iter().next().map(Box::new)
    } else {
        None
    };

    Ok(vec![GoNode::ForStmt {
        init: go_init,
        condition: go_cond,
        post: go_post,
        body: flatten_block(walk_child(body)?),
        span: span.clone(),
    }])
}

pub(crate) fn convert_foreach_stmt(
    variable: &str,
    iterable: &JavaNode,
    body: &JavaNode,
    span: &Span,
    walk_child: &dyn Fn(&JavaNode) -> Result<Vec<GoNode>, WalkError>,
) -> Result<Vec<GoNode>, WalkError> {
    let iter_node = walk_child(iterable)?
        .into_iter()
        .next()
        .unwrap_or(GoNode::Ident {
            name: "collection".to_string(),
            span: span.clone(),
        });

    Ok(vec![GoNode::RangeStmt {
        key: Some("_".to_string()),
        value: Some(variable.to_string()),
        iterable: Box::new(iter_node),
        body: flatten_block(walk_child(body)?),
        span: span.clone(),
    }])
}

pub(crate) fn convert_while_stmt(
    condition: &JavaNode,
    body: &JavaNode,
    span: &Span,
    walk_child: &dyn Fn(&JavaNode) -> Result<Vec<GoNode>, WalkError>,
) -> Result<Vec<GoNode>, WalkError> {
    let cond = walk_child(condition)?.into_iter().next().map(Box::new);

    Ok(vec![GoNode::ForStmt {
        init: None,
        condition: cond,
        post: None,
        body: flatten_block(walk_child(body)?),
        span: span.clone(),
    }])
}

pub(crate) fn convert_try_catch_stmt(
    try_block: &JavaNode,
    catches: &[JavaNode],
    finally_block: Option<&JavaNode>,
    span: &Span,
    walk_child: &dyn Fn(&JavaNode) -> Result<Vec<GoNode>, WalkError>,
) -> Result<Vec<GoNode>, WalkError> {
    let mut result = Vec::new();

    // Emit finally block as defer first (Go defers run LIFO, so declaring first is correct)
    if let Some(finally) = finally_block {
        let finally_body = flatten_block(walk_child(finally)?);
        result.push(GoNode::DeferStmt {
            call: Box::new(GoNode::CallExpr {
                function: Box::new(GoNode::FuncDecl {
                    name: String::new(),
                    receiver: None,
                    params: vec![],
                    returns: vec![],
                    body: finally_body,
                    span: span.clone(),
                }),
                args: vec![],
                span: span.clone(),
            }),
            span: span.clone(),
        });
    }

    // Try body: emit inline with error variable capture
    // Wrap in a block comment for context
    result.push(GoNode::RawCode {
        code: "// try".to_string(),
        span: span.clone(),
    });
    result.extend(flatten_block(walk_child(try_block)?));

    // Catch clauses: each becomes `if err != nil { ... }`
    if !catches.is_empty() {
        for catch in catches {
            result.extend(walk_child(catch)?);
        }
    }

    Ok(result)
}

pub(crate) fn convert_catch_clause(
    parameter: &str,
    exception_types: &[jovial_ast::java::JavaType],
    body: &JavaNode,
    span: &Span,
    walk_child: &dyn Fn(&JavaNode) -> Result<Vec<GoNode>, WalkError>,
) -> Result<Vec<GoNode>, WalkError> {
    let type_names: Vec<String> = exception_types.iter().map(|t| t.name.clone()).collect();
    let catch_body = flatten_block(walk_child(body)?);

    // Emit as: if err != nil { // catch ExceptionType \n <body> }
    let mut guarded_body = vec![GoNode::RawCode {
        code: format!("// catch: {} ({})", type_names.join(" | "), parameter),
        span: span.clone(),
    }];
    guarded_body.extend(catch_body);

    Ok(vec![GoNode::IfStmt {
        init: None,
        condition: Box::new(GoNode::BinaryExpr {
            left: Box::new(GoNode::Ident {
                name: parameter.to_string(),
                span: span.clone(),
            }),
            op: jovial_ast::go::GoBinaryOp::Ne,
            right: Box::new(GoNode::Literal {
                value: jovial_ast::go::GoLiteralValue::Nil,
                span: span.clone(),
            }),
            span: span.clone(),
        }),
        body: guarded_body,
        else_body: None,
        span: span.clone(),
    }])
}

pub(crate) fn convert_throw_stmt(
    expression: &JavaNode,
    span: &Span,
    walk_child: &dyn Fn(&JavaNode) -> Result<Vec<GoNode>, WalkError>,
) -> Result<Vec<GoNode>, WalkError> {
    let expr_nodes = walk_child(expression)?;

    // If expression is a NewExpr (throw new Exception("msg")), emit: return fmt.Errorf("msg")
    // Otherwise emit: return err
    if expr_nodes.len() == 1 {
        if let GoNode::CallExpr { args, .. } = &expr_nodes[0] {
            // throw new SomeException(args...) → return fmt.Errorf(args...)
            return Ok(vec![GoNode::ReturnStmt {
                values: vec![GoNode::CallExpr {
                    function: Box::new(GoNode::SelectorExpr {
                        object: Box::new(GoNode::Ident {
                            name: "fmt".to_string(),
                            span: span.clone(),
                        }),
                        field: "Errorf".to_string(),
                        span: span.clone(),
                    }),
                    args: if args.is_empty() {
                        vec![GoNode::Literal {
                            value: jovial_ast::go::GoLiteralValue::String("%w".to_string()),
                            span: span.clone(),
                        }]
                    } else {
                        // Prepend format string
                        let mut new_args = vec![GoNode::Literal {
                            value: jovial_ast::go::GoLiteralValue::String("%v".to_string()),
                            span: span.clone(),
                        }];
                        new_args.extend(args.clone());
                        new_args
                    },
                    span: span.clone(),
                }],
                span: span.clone(),
            }]);
        }
    }

    // Fallback: return the expression as-is (likely an error variable)
    Ok(vec![GoNode::ReturnStmt {
        values: expr_nodes,
        span: span.clone(),
    }])
}
