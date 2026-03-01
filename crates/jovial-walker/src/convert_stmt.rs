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
    result.push(GoNode::RawCode {
        code: "// TODO: Java try-catch converted; needs error handling review".to_string(),
        span: span.clone(),
    });

    result.extend(flatten_block(walk_child(try_block)?));

    for catch in catches {
        result.extend(walk_child(catch)?);
    }

    if let Some(finally) = finally_block {
        result.push(GoNode::RawCode {
            code: "// finally:".to_string(),
            span: span.clone(),
        });
        result.extend(flatten_block(walk_child(finally)?));
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
    let mut result = Vec::new();
    result.push(GoNode::RawCode {
        code: format!("// catch ({}: {}) {{", parameter, type_names.join(" | ")),
        span: span.clone(),
    });
    result.extend(flatten_block(walk_child(body)?));
    result.push(GoNode::RawCode {
        code: "// }".to_string(),
        span: span.clone(),
    });
    Ok(result)
}

pub(crate) fn convert_throw_stmt(
    expression: &JavaNode,
    span: &Span,
    walk_child: &dyn Fn(&JavaNode) -> Result<Vec<GoNode>, WalkError>,
) -> Result<Vec<GoNode>, WalkError> {
    let expr_nodes = walk_child(expression)?;
    let expr_str = if let Some(GoNode::Ident { name, .. }) = expr_nodes.first() {
        name.clone()
    } else {
        "err".to_string()
    };
    Ok(vec![GoNode::RawCode {
        code: format!(
            "// TODO: throw converted; consider: return fmt.Errorf(\"%w\", {})",
            expr_str
        ),
        span: span.clone(),
    }])
}
