use jovial_plugin::prelude::*;

pub struct JavaIoPlugin;

impl Plugin for JavaIoPlugin {
    fn name(&self) -> &str {
        "java-io"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn priority(&self) -> i32 {
        50
    }

    fn description(&self) -> &str {
        "Converts Java System.out/err and System.exit to idiomatic Go equivalents"
    }

    fn matches(&self, ctx: &MatchContext) -> bool {
        if let JavaNode::MethodCallExpr {
            object,
            name,
            ..
        } = ctx.node
        {
            let method = name.as_str();

            // System.exit(n)
            if method == "exit" {
                return is_name_expr(object.as_deref(), "System");
            }

            // System.out.println/print/printf or System.err.println/print
            if matches!(method, "println" | "print" | "printf") {
                if let Some(obj) = object.as_deref() {
                    return is_system_out(obj) || is_system_err(obj);
                }
            }
        }
        false
    }

    fn transform(&self, ctx: &mut TransformContext) -> Result<Vec<GoNode>, PluginError> {
        let (object, method, arguments, span) = match ctx.node {
            JavaNode::MethodCallExpr {
                object,
                name,
                arguments,
                span,
                ..
            } => (object, name.as_str(), arguments, span),
            _ => return Err(PluginError::TransformFailed("expected MethodCallExpr".into())),
        };

        // System.exit(n) -> os.Exit(n)
        if method == "exit" {
            ctx.add_import("os", None);
            let walked_args = ctx.walk_children(arguments)?;
            return Ok(vec![pkg_call("os", "Exit", walked_args, span)]);
        }

        let obj = object
            .as_deref()
            .ok_or_else(|| PluginError::TransformFailed("expected receiver".into()))?;
        let is_stderr = is_system_err(obj);
        let walked_args = ctx.walk_children(arguments)?;

        if is_stderr {
            // System.err.println(x) -> fmt.Fprintln(os.Stderr, x)
            // System.err.print(x)   -> fmt.Fprint(os.Stderr, x)
            ctx.add_import("fmt", None);
            ctx.add_import("os", None);

            let stderr_node = GoNode::SelectorExpr {
                object: Box::new(GoNode::Ident {
                    name: "os".to_string(),
                    span: span.clone(),
                }),
                field: "Stderr".to_string(),
                span: span.clone(),
            };

            let func_name = match method {
                "println" => "Fprintln",
                "print" => "Fprint",
                "printf" => "Fprintf",
                _ => return Err(PluginError::TransformFailed(format!("unhandled: {method}"))),
            };

            let mut args = vec![stderr_node];
            args.extend(walked_args);
            Ok(vec![pkg_call("fmt", func_name, args, span)])
        } else {
            // System.out.println(x) -> fmt.Println(x)
            // System.out.print(x)   -> fmt.Print(x)
            // System.out.printf(f, args...) -> fmt.Printf(f, args...)
            ctx.add_import("fmt", None);

            let func_name = match method {
                "println" => "Println",
                "print" => "Print",
                "printf" => "Printf",
                _ => return Err(PluginError::TransformFailed(format!("unhandled: {method}"))),
            };

            Ok(vec![pkg_call("fmt", func_name, walked_args, span)])
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────

fn is_name_expr(node: Option<&JavaNode>, expected: &str) -> bool {
    matches!(node, Some(JavaNode::NameExpr { name, .. }) if name == expected)
}

/// Check if a node is `System.out` (FieldAccessExpr { object: NameExpr("System"), field: "out" }).
fn is_system_out(node: &JavaNode) -> bool {
    matches!(
        node,
        JavaNode::FieldAccessExpr { object, field, .. }
        if field == "out" && matches!(object.as_ref(), JavaNode::NameExpr { name, .. } if name == "System")
    )
}

/// Check if a node is `System.err` (FieldAccessExpr { object: NameExpr("System"), field: "err" }).
fn is_system_err(node: &JavaNode) -> bool {
    matches!(
        node,
        JavaNode::FieldAccessExpr { object, field, .. }
        if field == "err" && matches!(object.as_ref(), JavaNode::NameExpr { name, .. } if name == "System")
    )
}

/// Create a `pkg.Func(args...)` call expression.
fn pkg_call(pkg: &str, func_name: &str, args: Vec<GoNode>, span: &Span) -> GoNode {
    GoNode::CallExpr {
        function: Box::new(GoNode::SelectorExpr {
            object: Box::new(GoNode::Ident {
                name: pkg.to_string(),
                span: span.clone(),
            }),
            field: func_name.to_string(),
            span: span.clone(),
        }),
        args,
        span: span.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jovial_ast::span::Span;
    use std::collections::HashMap;

    struct NoopTypeResolver;
    impl TypeResolver for NoopTypeResolver {
        fn resolve(&self, _: &str) -> Option<String> {
            None
        }
        fn is_assignable_to(&self, _: &str, _: &str) -> bool {
            false
        }
        fn superclass_of(&self, _: &str) -> Option<String> {
            None
        }
        fn interfaces_of(&self, _: &str) -> Vec<String> {
            vec![]
        }
    }

    fn dummy_span() -> Span {
        Span::dummy()
    }

    fn system_out() -> JavaNode {
        JavaNode::FieldAccessExpr {
            object: Box::new(JavaNode::NameExpr {
                name: "System".to_string(),
                span: dummy_span(),
            }),
            field: "out".to_string(),
            span: dummy_span(),
        }
    }

    fn system_err() -> JavaNode {
        JavaNode::FieldAccessExpr {
            object: Box::new(JavaNode::NameExpr {
                name: "System".to_string(),
                span: dummy_span(),
            }),
            field: "err".to_string(),
            span: dummy_span(),
        }
    }

    fn name_expr(name: &str) -> JavaNode {
        JavaNode::NameExpr {
            name: name.to_string(),
            span: dummy_span(),
        }
    }

    fn string_literal(s: &str) -> JavaNode {
        JavaNode::LiteralExpr {
            value: LiteralValue::String(s.to_string()),
            span: dummy_span(),
        }
    }

    fn int_lit(v: i64) -> JavaNode {
        JavaNode::LiteralExpr {
            value: LiteralValue::Int(v),
            span: dummy_span(),
        }
    }

    #[test]
    fn test_matches_system_out_println() {
        let plugin = JavaIoPlugin;
        let resolver = NoopTypeResolver;
        let config = HashMap::new();

        let node = JavaNode::MethodCallExpr {
            object: Some(Box::new(system_out())),
            name: "println".to_string(),
            arguments: vec![string_literal("hello")],
            type_args: vec![],
            span: dummy_span(),
        };

        let ctx = MatchContext {
            node: &node,
            type_resolver: &resolver,
            config: &config,
        };
        assert!(plugin.matches(&ctx));
    }

    #[test]
    fn test_matches_system_err_println() {
        let plugin = JavaIoPlugin;
        let resolver = NoopTypeResolver;
        let config = HashMap::new();

        let node = JavaNode::MethodCallExpr {
            object: Some(Box::new(system_err())),
            name: "println".to_string(),
            arguments: vec![string_literal("error")],
            type_args: vec![],
            span: dummy_span(),
        };

        let ctx = MatchContext {
            node: &node,
            type_resolver: &resolver,
            config: &config,
        };
        assert!(plugin.matches(&ctx));
    }

    #[test]
    fn test_matches_system_exit() {
        let plugin = JavaIoPlugin;
        let resolver = NoopTypeResolver;
        let config = HashMap::new();

        let node = JavaNode::MethodCallExpr {
            object: Some(Box::new(name_expr("System"))),
            name: "exit".to_string(),
            arguments: vec![int_lit(1)],
            type_args: vec![],
            span: dummy_span(),
        };

        let ctx = MatchContext {
            node: &node,
            type_resolver: &resolver,
            config: &config,
        };
        assert!(plugin.matches(&ctx));
    }

    #[test]
    fn test_no_match_regular_println() {
        let plugin = JavaIoPlugin;
        let resolver = NoopTypeResolver;
        let config = HashMap::new();

        // logger.println("x") — not System.out
        let node = JavaNode::MethodCallExpr {
            object: Some(Box::new(name_expr("logger"))),
            name: "println".to_string(),
            arguments: vec![string_literal("x")],
            type_args: vec![],
            span: dummy_span(),
        };

        let ctx = MatchContext {
            node: &node,
            type_resolver: &resolver,
            config: &config,
        };
        assert!(!plugin.matches(&ctx));
    }

    #[test]
    fn test_transform_println() {
        let plugin = JavaIoPlugin;
        let resolver = NoopTypeResolver;
        let config = HashMap::new();

        let node = JavaNode::MethodCallExpr {
            object: Some(Box::new(system_out())),
            name: "println".to_string(),
            arguments: vec![string_literal("hello")],
            type_args: vec![],
            span: dummy_span(),
        };

        let noop_walk = |child: &JavaNode| -> Result<Vec<GoNode>, PluginError> {
            match child {
                JavaNode::LiteralExpr {
                    value: LiteralValue::String(s),
                    ..
                } => Ok(vec![GoNode::Literal {
                    value: GoLiteralValue::String(s.clone()),
                    span: Span::dummy(),
                }]),
                _ => Ok(vec![]),
            }
        };
        let mut ctx = TransformContext::new(&node, &resolver, &config, &noop_walk);
        let result = plugin.transform(&mut ctx).unwrap();

        assert_eq!(result.len(), 1);
        if let GoNode::CallExpr { function, args, .. } = &result[0] {
            if let GoNode::SelectorExpr { object, field, .. } = function.as_ref() {
                if let GoNode::Ident { name, .. } = object.as_ref() {
                    assert_eq!(name, "fmt");
                }
                assert_eq!(field, "Println");
            }
            assert_eq!(args.len(), 1);
        } else {
            panic!("expected CallExpr, got {:?}", result[0]);
        }
    }

    #[test]
    fn test_transform_err_println() {
        let plugin = JavaIoPlugin;
        let resolver = NoopTypeResolver;
        let config = HashMap::new();

        let node = JavaNode::MethodCallExpr {
            object: Some(Box::new(system_err())),
            name: "println".to_string(),
            arguments: vec![string_literal("error")],
            type_args: vec![],
            span: dummy_span(),
        };

        let noop_walk = |child: &JavaNode| -> Result<Vec<GoNode>, PluginError> {
            match child {
                JavaNode::LiteralExpr {
                    value: LiteralValue::String(s),
                    ..
                } => Ok(vec![GoNode::Literal {
                    value: GoLiteralValue::String(s.clone()),
                    span: Span::dummy(),
                }]),
                _ => Ok(vec![]),
            }
        };
        let mut ctx = TransformContext::new(&node, &resolver, &config, &noop_walk);
        let result = plugin.transform(&mut ctx).unwrap();

        assert_eq!(result.len(), 1);
        if let GoNode::CallExpr { function, args, .. } = &result[0] {
            if let GoNode::SelectorExpr { object, field, .. } = function.as_ref() {
                if let GoNode::Ident { name, .. } = object.as_ref() {
                    assert_eq!(name, "fmt");
                }
                assert_eq!(field, "Fprintln");
            }
            // First arg is os.Stderr, second is the message
            assert_eq!(args.len(), 2);
            if let GoNode::SelectorExpr { object, field, .. } = &args[0] {
                if let GoNode::Ident { name, .. } = object.as_ref() {
                    assert_eq!(name, "os");
                }
                assert_eq!(field, "Stderr");
            } else {
                panic!("expected os.Stderr as first arg");
            }
        } else {
            panic!("expected CallExpr, got {:?}", result[0]);
        }
    }

    #[test]
    fn test_transform_system_exit() {
        let plugin = JavaIoPlugin;
        let resolver = NoopTypeResolver;
        let config = HashMap::new();

        let node = JavaNode::MethodCallExpr {
            object: Some(Box::new(name_expr("System"))),
            name: "exit".to_string(),
            arguments: vec![int_lit(1)],
            type_args: vec![],
            span: dummy_span(),
        };

        let noop_walk = |child: &JavaNode| -> Result<Vec<GoNode>, PluginError> {
            match child {
                JavaNode::LiteralExpr {
                    value: LiteralValue::Int(v),
                    ..
                } => Ok(vec![GoNode::Literal {
                    value: GoLiteralValue::Int(*v),
                    span: Span::dummy(),
                }]),
                _ => Ok(vec![]),
            }
        };
        let mut ctx = TransformContext::new(&node, &resolver, &config, &noop_walk);
        let result = plugin.transform(&mut ctx).unwrap();

        assert_eq!(result.len(), 1);
        if let GoNode::CallExpr { function, args, .. } = &result[0] {
            if let GoNode::SelectorExpr { object, field, .. } = function.as_ref() {
                if let GoNode::Ident { name, .. } = object.as_ref() {
                    assert_eq!(name, "os");
                }
                assert_eq!(field, "Exit");
            }
            assert_eq!(args.len(), 1);
        } else {
            panic!("expected CallExpr, got {:?}", result[0]);
        }
    }
}
