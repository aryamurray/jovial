use jovial_ast::java::{JavaNode, JavaType};
use tree_sitter::Node;

use crate::lower::Lowerer;
use crate::lower_expr::lower_expr;
use crate::lower_type::lower_type;

/// Lower a tree-sitter statement node into a `JavaNode`.
pub(crate) fn lower_stmt(lowerer: &Lowerer, node: Node) -> JavaNode {
    match node.kind() {
        // ── Block ───────────────────────────────────────────────────
        "block" => {
            let mut stmts = Vec::new();
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                stmts.push(lower_stmt(lowerer, child));
            }
            JavaNode::BlockStmt {
                statements: stmts,
                span: lowerer.span(node),
            }
        }

        // ── Return ──────────────────────────────────────────────────
        "return_statement" => {
            let value = {
                let mut cursor = node.walk();
                let first = node.named_children(&mut cursor).next();
                first.map(|v| Box::new(lower_expr(lowerer, v)))
            };
            JavaNode::ReturnStmt {
                value,
                span: lowerer.span(node),
            }
        }

        // ── If ──────────────────────────────────────────────────────
        "if_statement" => {
            let condition = node.child_by_field_name("condition")
                .map(|c| lower_expr(lowerer, c))
                .unwrap_or_else(|| JavaNode::NameExpr {
                    name: "<missing>".to_string(),
                    span: lowerer.span(node),
                });
            let then_branch = node.child_by_field_name("consequence")
                .map(|t| lower_stmt(lowerer, t))
                .unwrap_or_else(|| JavaNode::BlockStmt {
                    statements: Vec::new(),
                    span: lowerer.span(node),
                });
            let else_branch = node.child_by_field_name("alternative")
                .map(|e| Box::new(lower_stmt(lowerer, e)));

            JavaNode::IfStmt {
                condition: Box::new(condition),
                then_branch: Box::new(then_branch),
                else_branch,
                span: lowerer.span(node),
            }
        }

        // ── For ─────────────────────────────────────────────────────
        "for_statement" => {
            // tree-sitter-java for_statement children:
            // init, condition, update are accessed positionally via named children
            let mut init = None;
            let mut condition = None;
            let mut update = Vec::new();
            let mut body = None;

            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                match child.kind() {
                    "local_variable_declaration" => {
                        init = Some(Box::new(lower_stmt(lowerer, child)));
                    }
                    "block" => {
                        body = Some(lower_stmt(lowerer, child));
                    }
                    "update_expression" | "assignment_expression" | "method_invocation" => {
                        // Could be condition or update depending on position
                        if body.is_none() {
                            if condition.is_none() && init.is_some() {
                                // Could still be an expression in init or condition
                                update.push(lower_expr(lowerer, child));
                            } else {
                                update.push(lower_expr(lowerer, child));
                            }
                        }
                    }
                    _ => {
                        if body.is_none() && child.kind() != "(" && child.kind() != ")" {
                            // Try to parse as condition if we don't have one yet
                            if init.is_some() && condition.is_none() && update.is_empty() {
                                condition = Some(Box::new(lower_expr(lowerer, child)));
                            } else if condition.is_some() {
                                update.push(lower_expr(lowerer, child));
                            } else if init.is_none() {
                                // Expression statement as init
                                init = Some(Box::new(lower_expr(lowerer, child)));
                            }
                        }
                    }
                }
            }

            // Fallback: use field names if available
            if init.is_none() {
                init = node.child_by_field_name("init")
                    .map(|i| Box::new(lower_stmt(lowerer, i)));
            }
            if condition.is_none() {
                condition = node.child_by_field_name("condition")
                    .map(|c| Box::new(lower_expr(lowerer, c)));
            }

            let body = body.unwrap_or_else(|| {
                node.child_by_field_name("body")
                    .map(|b| lower_stmt(lowerer, b))
                    .unwrap_or(JavaNode::BlockStmt {
                        statements: Vec::new(),
                        span: lowerer.span(node),
                    })
            });

            JavaNode::ForStmt {
                init,
                condition,
                update,
                body: Box::new(body),
                span: lowerer.span(node),
            }
        }

        // ── Enhanced for ────────────────────────────────────────────
        "enhanced_for_statement" => {
            let var_type = node.child_by_field_name("type")
                .map(|t| lower_type(lowerer, t))
                .unwrap_or_else(|| JavaType {
                    name: "var".to_string(),
                    type_args: Vec::new(),
                    array_dimensions: 0,
                    is_varargs: false,
                });
            let variable = node.child_by_field_name("name")
                .map(|n| lowerer.node_text(n).to_string())
                .unwrap_or_default();
            let iterable = node.child_by_field_name("value")
                .map(|v| lower_expr(lowerer, v))
                .unwrap_or_else(|| JavaNode::NameExpr {
                    name: "<missing>".to_string(),
                    span: lowerer.span(node),
                });
            let body = node.child_by_field_name("body")
                .map(|b| lower_stmt(lowerer, b))
                .unwrap_or_else(|| JavaNode::BlockStmt {
                    statements: Vec::new(),
                    span: lowerer.span(node),
                });

            JavaNode::ForEachStmt {
                variable,
                variable_type: var_type,
                iterable: Box::new(iterable),
                body: Box::new(body),
                span: lowerer.span(node),
            }
        }

        // ── While ───────────────────────────────────────────────────
        "while_statement" => {
            let condition = node.child_by_field_name("condition")
                .map(|c| lower_expr(lowerer, c))
                .unwrap_or_else(|| JavaNode::NameExpr {
                    name: "<missing>".to_string(),
                    span: lowerer.span(node),
                });
            let body = node.child_by_field_name("body")
                .map(|b| lower_stmt(lowerer, b))
                .unwrap_or_else(|| JavaNode::BlockStmt {
                    statements: Vec::new(),
                    span: lowerer.span(node),
                });

            JavaNode::WhileStmt {
                condition: Box::new(condition),
                body: Box::new(body),
                span: lowerer.span(node),
            }
        }

        // ── Try-catch-finally ───────────────────────────────────────
        "try_statement" => {
            let try_block = node.child_by_field_name("body")
                .map(|b| lower_stmt(lowerer, b))
                .unwrap_or_else(|| JavaNode::BlockStmt {
                    statements: Vec::new(),
                    span: lowerer.span(node),
                });

            let catches: Vec<JavaNode> = lowerer.children_by_kind(node, "catch_clause")
                .into_iter()
                .map(|c| lower_catch_clause(lowerer, c))
                .collect();

            let finally_block = lowerer.child_by_kind(node, "finally_clause")
                .and_then(|f| lowerer.child_by_kind(f, "block"))
                .map(|b| Box::new(lower_stmt(lowerer, b)));

            JavaNode::TryCatchStmt {
                try_block: Box::new(try_block),
                catches,
                finally_block,
                span: lowerer.span(node),
            }
        }

        // ── Throw ───────────────────────────────────────────────────
        "throw_statement" => {
            let expression = {
                let mut cursor = node.walk();
                let first = node.named_children(&mut cursor).next();
                first.map(|e| lower_expr(lowerer, e))
                    .unwrap_or_else(|| JavaNode::NameExpr {
                        name: "<missing>".to_string(),
                        span: lowerer.span(node),
                    })
            };

            JavaNode::ThrowStmt {
                expression: Box::new(expression),
                span: lowerer.span(node),
            }
        }

        // ── Expression statement ────────────────────────────────────
        "expression_statement" => {
            let expr = {
                let mut cursor = node.walk();
                let first = node.named_children(&mut cursor).next();
                first.map(|e| lower_expr(lowerer, e))
                    .unwrap_or_else(|| JavaNode::NameExpr {
                        name: "<missing>".to_string(),
                        span: lowerer.span(node),
                    })
            };
            expr
        }

        // ── Local variable declaration ──────────────────────────────
        "local_variable_declaration" => {
            let var_type = node.child_by_field_name("type")
                .map(|t| lower_type(lowerer, t));

            let is_final = {
                let mods = lowerer.child_by_kind(node, "modifiers");
                if let Some(m) = mods {
                    let mut cursor = m.walk();
                    let result = m.children(&mut cursor)
                        .any(|c| lowerer.node_text(c) == "final");
                    result
                } else {
                    false
                }
            };

            // Get the first declarator
            let declarator = lowerer.child_by_kind(node, "variable_declarator");
            if let Some(decl) = declarator {
                let name = lowerer.child_by_kind(decl, "identifier")
                    .map(|n| lowerer.node_text(n).to_string())
                    .unwrap_or_default();

                let initializer = decl.child_by_field_name("value")
                    .map(|v| Box::new(lower_expr(lowerer, v)));

                JavaNode::VariableDecl {
                    name,
                    var_type,
                    initializer,
                    is_final,
                    span: lowerer.span(node),
                }
            } else {
                JavaNode::VariableDecl {
                    name: "<missing>".to_string(),
                    var_type,
                    initializer: None,
                    is_final,
                    span: lowerer.span(node),
                }
            }
        }

        // ── Explicit constructor invocation ─────────────────────────
        "explicit_constructor_invocation" => {
            let mut cursor = node.walk();
            let children: Vec<_> = node.children(&mut cursor).collect();

            let name = children.iter()
                .find(|c| c.kind() == "this" || c.kind() == "super")
                .map(|c| lowerer.node_text(*c).to_string())
                .unwrap_or_else(|| "this".to_string());

            let arguments = lowerer.child_by_kind(node, "argument_list")
                .map(|args| {
                    let mut cursor2 = args.walk();
                    args.named_children(&mut cursor2)
                        .map(|child| lower_expr(lowerer, child))
                        .collect()
                })
                .unwrap_or_default();

            JavaNode::MethodCallExpr {
                object: None,
                name,
                arguments,
                type_args: Vec::new(),
                span: lowerer.span(node),
            }
        }

        // ── Fallback: try as expression ─────────────────────────────
        _ => lower_expr(lowerer, node),
    }
}

fn lower_catch_clause(lowerer: &Lowerer, node: Node) -> JavaNode {
    let catch_param = lowerer.child_by_kind(node, "catch_formal_parameter");

    let (parameter, exception_types) = if let Some(cp) = catch_param {
        let name = lowerer.child_by_kind(cp, "identifier")
            .map(|n| lowerer.node_text(n).to_string())
            .unwrap_or_else(|| "e".to_string());

        let catch_type_node = lowerer.child_by_kind(cp, "catch_type");
        let types: Vec<JavaType> = if let Some(ct) = catch_type_node {
            let mut cursor = ct.walk();
            ct.named_children(&mut cursor)
                .map(|t| lower_type(lowerer, t))
                .collect()
        } else {
            // Fallback: check for direct type child
            cp.child_by_field_name("type")
                .map(|t| vec![lower_type(lowerer, t)])
                .unwrap_or_default()
        };

        (name, types)
    } else {
        ("e".to_string(), Vec::new())
    };

    let body = lowerer.child_by_kind(node, "block")
        .map(|b| lower_stmt(lowerer, b))
        .unwrap_or_else(|| JavaNode::BlockStmt {
            statements: Vec::new(),
            span: lowerer.span(node),
        });

    JavaNode::CatchClause {
        parameter,
        exception_types,
        body: Box::new(body),
        span: lowerer.span(node),
    }
}
