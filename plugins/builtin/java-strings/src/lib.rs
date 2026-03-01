use jovial_plugin::prelude::*;

pub struct JavaStringsPlugin;

/// Instance methods matched by name on any receiver.
const INSTANCE_METHODS: &[&str] = &[
    "equals",
    "equalsIgnoreCase",
    "length",
    "isEmpty",
    "contains",
    "startsWith",
    "endsWith",
    "indexOf",
    "toLowerCase",
    "toUpperCase",
    "trim",
    "replace",
    "substring",
    "charAt",
    "toCharArray",
];

/// Static methods matched when object is `String`.
const STATIC_METHODS: &[&str] = &["valueOf", "format"];

impl Plugin for JavaStringsPlugin {
    fn name(&self) -> &str {
        "java-strings"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn priority(&self) -> i32 {
        50
    }

    fn description(&self) -> &str {
        "Converts Java String methods to idiomatic Go equivalents"
    }

    fn matches(&self, ctx: &MatchContext) -> bool {
        if let JavaNode::MethodCallExpr {
            object,
            name,
            ..
        } = ctx.node
        {
            let method = name.as_str();

            // Static methods: String.valueOf(x), String.format(f, ...)
            if STATIC_METHODS.contains(&method) {
                return is_name_expr(object.as_deref(), "String");
            }

            // Instance methods: require a receiver
            if object.is_some() && INSTANCE_METHODS.contains(&method) {
                return true;
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

        match method {
            // s.equals(x) -> s == x
            "equals" => {
                let recv = walk_object(ctx, object)?;
                let arg = walk_first_arg(ctx, arguments)?;
                Ok(vec![GoNode::BinaryExpr {
                    left: Box::new(recv),
                    op: GoBinaryOp::Eq,
                    right: Box::new(arg),
                    span: span.clone(),
                }])
            }

            // s.equalsIgnoreCase(x) -> strings.EqualFold(s, x)
            "equalsIgnoreCase" => {
                ctx.add_import("strings", None);
                let recv = walk_object(ctx, object)?;
                let arg = walk_first_arg(ctx, arguments)?;
                Ok(vec![strings_call("EqualFold", vec![recv, arg], span)])
            }

            // s.length() -> len(s)
            "length" => {
                let recv = walk_object(ctx, object)?;
                Ok(vec![builtin_call("len", vec![recv], span)])
            }

            // s.isEmpty() -> len(s) == 0
            "isEmpty" => {
                let recv = walk_object(ctx, object)?;
                Ok(vec![GoNode::BinaryExpr {
                    left: Box::new(builtin_call("len", vec![recv], span)),
                    op: GoBinaryOp::Eq,
                    right: Box::new(int_literal(0, span)),
                    span: span.clone(),
                }])
            }

            // s.contains(x) -> strings.Contains(s, x)
            "contains" => {
                ctx.add_import("strings", None);
                let recv = walk_object(ctx, object)?;
                let arg = walk_first_arg(ctx, arguments)?;
                Ok(vec![strings_call("Contains", vec![recv, arg], span)])
            }

            // s.startsWith(x) -> strings.HasPrefix(s, x)
            "startsWith" => {
                ctx.add_import("strings", None);
                let recv = walk_object(ctx, object)?;
                let arg = walk_first_arg(ctx, arguments)?;
                Ok(vec![strings_call("HasPrefix", vec![recv, arg], span)])
            }

            // s.endsWith(x) -> strings.HasSuffix(s, x)
            "endsWith" => {
                ctx.add_import("strings", None);
                let recv = walk_object(ctx, object)?;
                let arg = walk_first_arg(ctx, arguments)?;
                Ok(vec![strings_call("HasSuffix", vec![recv, arg], span)])
            }

            // s.indexOf(x) -> strings.Index(s, x)
            "indexOf" => {
                ctx.add_import("strings", None);
                let recv = walk_object(ctx, object)?;
                let arg = walk_first_arg(ctx, arguments)?;
                Ok(vec![strings_call("Index", vec![recv, arg], span)])
            }

            // s.toLowerCase() -> strings.ToLower(s)
            "toLowerCase" => {
                ctx.add_import("strings", None);
                let recv = walk_object(ctx, object)?;
                Ok(vec![strings_call("ToLower", vec![recv], span)])
            }

            // s.toUpperCase() -> strings.ToUpper(s)
            "toUpperCase" => {
                ctx.add_import("strings", None);
                let recv = walk_object(ctx, object)?;
                Ok(vec![strings_call("ToUpper", vec![recv], span)])
            }

            // s.trim() -> strings.TrimSpace(s)
            "trim" => {
                ctx.add_import("strings", None);
                let recv = walk_object(ctx, object)?;
                Ok(vec![strings_call("TrimSpace", vec![recv], span)])
            }

            // s.replace(a, b) -> strings.ReplaceAll(s, a, b)
            "replace" => {
                ctx.add_import("strings", None);
                let recv = walk_object(ctx, object)?;
                let walked_args = ctx.walk_children(arguments)?;
                let mut all_args = vec![recv];
                all_args.extend(walked_args);
                Ok(vec![strings_call("ReplaceAll", all_args, span)])
            }

            // s.substring(a) -> s[a:]
            // s.substring(a, b) -> s[a:b]
            "substring" => {
                let recv = walk_object(ctx, object)?;
                let walked_args = ctx.walk_children(arguments)?;
                let recv_code = inline_code(&recv);
                if walked_args.len() == 1 {
                    let a_code = inline_code(&walked_args[0]);
                    Ok(vec![GoNode::RawCode {
                        code: format!("{recv_code}[{a_code}:]"),
                        span: span.clone(),
                    }])
                } else if walked_args.len() >= 2 {
                    let a_code = inline_code(&walked_args[0]);
                    let b_code = inline_code(&walked_args[1]);
                    Ok(vec![GoNode::RawCode {
                        code: format!("{recv_code}[{a_code}:{b_code}]"),
                        span: span.clone(),
                    }])
                } else {
                    Err(PluginError::TransformFailed("substring requires at least 1 argument".into()))
                }
            }

            // s.charAt(i) -> s[i]
            "charAt" => {
                let recv = walk_object(ctx, object)?;
                let arg = walk_first_arg(ctx, arguments)?;
                let recv_code = inline_code(&recv);
                let arg_code = inline_code(&arg);
                Ok(vec![GoNode::RawCode {
                    code: format!("{recv_code}[{arg_code}]"),
                    span: span.clone(),
                }])
            }

            // s.toCharArray() -> []rune(s)
            "toCharArray" => {
                let recv = walk_object(ctx, object)?;
                let recv_code = inline_code(&recv);
                Ok(vec![GoNode::RawCode {
                    code: format!("[]rune({recv_code})"),
                    span: span.clone(),
                }])
            }

            // String.valueOf(x) -> fmt.Sprint(x)
            "valueOf" => {
                ctx.add_import("fmt", None);
                let walked_args = ctx.walk_children(arguments)?;
                Ok(vec![pkg_call("fmt", "Sprint", walked_args, span)])
            }

            // String.format(fmt, args...) -> fmt.Sprintf(fmt, args...)
            "format" => {
                ctx.add_import("fmt", None);
                let walked_args = ctx.walk_children(arguments)?;
                Ok(vec![pkg_call("fmt", "Sprintf", walked_args, span)])
            }

            _ => Err(PluginError::TransformFailed(format!(
                "unhandled string method: {method}"
            ))),
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────

fn is_name_expr(node: Option<&JavaNode>, expected: &str) -> bool {
    matches!(node, Some(JavaNode::NameExpr { name, .. }) if name == expected)
}

/// Walk the receiver object, expecting exactly one GoNode.
fn walk_object(
    ctx: &TransformContext,
    object: &Option<Box<JavaNode>>,
) -> Result<GoNode, PluginError> {
    let obj = object
        .as_ref()
        .ok_or_else(|| PluginError::TransformFailed("expected receiver object".into()))?;
    let nodes = ctx.walk_child(obj)?;
    nodes
        .into_iter()
        .next()
        .ok_or_else(|| PluginError::TransformFailed("receiver produced no nodes".into()))
}

/// Walk the first argument, expecting exactly one GoNode.
fn walk_first_arg(
    ctx: &TransformContext,
    arguments: &[JavaNode],
) -> Result<GoNode, PluginError> {
    let arg = arguments
        .first()
        .ok_or_else(|| PluginError::TransformFailed("expected at least one argument".into()))?;
    let nodes = ctx.walk_child(arg)?;
    nodes
        .into_iter()
        .next()
        .ok_or_else(|| PluginError::TransformFailed("argument produced no nodes".into()))
}

/// Create a `strings.Func(args...)` call expression.
fn strings_call(func_name: &str, args: Vec<GoNode>, span: &Span) -> GoNode {
    pkg_call("strings", func_name, args, span)
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

/// Create a builtin function call like `len(x)`.
fn builtin_call(name: &str, args: Vec<GoNode>, span: &Span) -> GoNode {
    GoNode::CallExpr {
        function: Box::new(GoNode::Ident {
            name: name.to_string(),
            span: span.clone(),
        }),
        args,
        span: span.clone(),
    }
}

/// Create an integer literal.
fn int_literal(value: i64, span: &Span) -> GoNode {
    GoNode::Literal {
        value: GoLiteralValue::Int(value),
        span: span.clone(),
    }
}

/// Simple inline code representation for a GoNode.
/// Used when we need to embed an expression in a RawCode template.
fn inline_code(node: &GoNode) -> String {
    match node {
        GoNode::Ident { name, .. } => name.clone(),
        GoNode::Literal { value, .. } => match value {
            GoLiteralValue::Int(v) => v.to_string(),
            GoLiteralValue::Float(v) => v.to_string(),
            GoLiteralValue::String(s) => format!("\"{s}\""),
            GoLiteralValue::Rune(c) => format!("'{c}'"),
            GoLiteralValue::Bool(b) => b.to_string(),
            GoLiteralValue::Nil => "nil".to_string(),
        },
        GoNode::SelectorExpr { object, field, .. } => {
            format!("{}.{field}", inline_code(object))
        }
        GoNode::RawCode { code, .. } => code.clone(),
        GoNode::CallExpr { function, args, .. } => {
            let func = inline_code(function);
            let arg_strs: Vec<String> = args.iter().map(|a| inline_code(a)).collect();
            format!("{func}({})", arg_strs.join(", "))
        }
        GoNode::BinaryExpr { left, op, right, .. } => {
            let op_str = match op {
                GoBinaryOp::Add => "+",
                GoBinaryOp::Sub => "-",
                GoBinaryOp::Mul => "*",
                GoBinaryOp::Div => "/",
                GoBinaryOp::Mod => "%",
                GoBinaryOp::Eq => "==",
                GoBinaryOp::Ne => "!=",
                GoBinaryOp::Lt => "<",
                GoBinaryOp::Gt => ">",
                GoBinaryOp::Le => "<=",
                GoBinaryOp::Ge => ">=",
                GoBinaryOp::And => "&&",
                GoBinaryOp::Or => "||",
                GoBinaryOp::BitwiseAnd => "&",
                GoBinaryOp::BitwiseOr => "|",
                GoBinaryOp::BitwiseXor => "^",
                GoBinaryOp::ShiftLeft => "<<",
                GoBinaryOp::ShiftRight => ">>",
            };
            format!("{} {op_str} {}", inline_code(left), inline_code(right))
        }
        _ => format!("/* TODO: complex expr */"),
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

    fn make_method_call(
        object: Option<JavaNode>,
        name: &str,
        arguments: Vec<JavaNode>,
    ) -> JavaNode {
        JavaNode::MethodCallExpr {
            object: object.map(Box::new),
            name: name.to_string(),
            arguments,
            type_args: vec![],
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

    fn make_match_ctx(_node: &JavaNode) -> (NoopTypeResolver, HashMap<String, ConfigValue>) {
        (NoopTypeResolver, HashMap::new())
    }

    #[test]
    fn test_matches_equals() {
        let plugin = JavaStringsPlugin;
        let node = make_method_call(Some(name_expr("s")), "equals", vec![string_literal("x")]);
        let (resolver, config) = make_match_ctx(&node);
        let ctx = MatchContext {
            node: &node,
            type_resolver: &resolver,
            config: &config,
        };
        assert!(plugin.matches(&ctx));
    }

    #[test]
    fn test_matches_static_valueof() {
        let plugin = JavaStringsPlugin;
        let node = make_method_call(Some(name_expr("String")), "valueOf", vec![name_expr("x")]);
        let (resolver, config) = make_match_ctx(&node);
        let ctx = MatchContext {
            node: &node,
            type_resolver: &resolver,
            config: &config,
        };
        assert!(plugin.matches(&ctx));
    }

    #[test]
    fn test_no_match_unknown_method() {
        let plugin = JavaStringsPlugin;
        let node = make_method_call(Some(name_expr("s")), "fooBar", vec![]);
        let (resolver, config) = make_match_ctx(&node);
        let ctx = MatchContext {
            node: &node,
            type_resolver: &resolver,
            config: &config,
        };
        assert!(!plugin.matches(&ctx));
    }

    #[test]
    fn test_no_match_no_receiver() {
        let plugin = JavaStringsPlugin;
        let node = make_method_call(None, "length", vec![]);
        let (resolver, config) = make_match_ctx(&node);
        let ctx = MatchContext {
            node: &node,
            type_resolver: &resolver,
            config: &config,
        };
        assert!(!plugin.matches(&ctx));
    }

    #[test]
    fn test_transform_equals() {
        let plugin = JavaStringsPlugin;
        let node = make_method_call(Some(name_expr("s")), "equals", vec![string_literal("hello")]);
        let (resolver, config) = make_match_ctx(&node);

        let noop_walk = |child: &JavaNode| -> Result<Vec<GoNode>, PluginError> {
            match child {
                JavaNode::NameExpr { name, .. } => Ok(vec![GoNode::Ident {
                    name: name.clone(),
                    span: Span::dummy(),
                }]),
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
        assert!(matches!(
            &result[0],
            GoNode::BinaryExpr { op: GoBinaryOp::Eq, .. }
        ));
    }

    #[test]
    fn test_transform_length() {
        let plugin = JavaStringsPlugin;
        let node = make_method_call(Some(name_expr("s")), "length", vec![]);
        let (resolver, config) = make_match_ctx(&node);

        let noop_walk = |child: &JavaNode| -> Result<Vec<GoNode>, PluginError> {
            match child {
                JavaNode::NameExpr { name, .. } => Ok(vec![GoNode::Ident {
                    name: name.clone(),
                    span: Span::dummy(),
                }]),
                _ => Ok(vec![]),
            }
        };
        let mut ctx = TransformContext::new(&node, &resolver, &config, &noop_walk);
        let result = plugin.transform(&mut ctx).unwrap();

        assert_eq!(result.len(), 1);
        if let GoNode::CallExpr { function, args, .. } = &result[0] {
            if let GoNode::Ident { name, .. } = function.as_ref() {
                assert_eq!(name, "len");
            } else {
                panic!("expected len ident");
            }
            assert_eq!(args.len(), 1);
        } else {
            panic!("expected CallExpr, got {:?}", result[0]);
        }
    }

    #[test]
    fn test_transform_contains() {
        let plugin = JavaStringsPlugin;
        let node = make_method_call(Some(name_expr("s")), "contains", vec![string_literal("x")]);
        let (resolver, config) = make_match_ctx(&node);

        let noop_walk = |child: &JavaNode| -> Result<Vec<GoNode>, PluginError> {
            match child {
                JavaNode::NameExpr { name, .. } => Ok(vec![GoNode::Ident {
                    name: name.clone(),
                    span: Span::dummy(),
                }]),
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
                    assert_eq!(name, "strings");
                }
                assert_eq!(field, "Contains");
            } else {
                panic!("expected SelectorExpr");
            }
            assert_eq!(args.len(), 2); // receiver + arg
        } else {
            panic!("expected CallExpr, got {:?}", result[0]);
        }
    }

    #[test]
    fn test_transform_substring_one_arg() {
        let plugin = JavaStringsPlugin;
        let node = make_method_call(Some(name_expr("s")), "substring", vec![int_lit(5)]);
        let (resolver, config) = make_match_ctx(&node);

        let noop_walk = |child: &JavaNode| -> Result<Vec<GoNode>, PluginError> {
            match child {
                JavaNode::NameExpr { name, .. } => Ok(vec![GoNode::Ident {
                    name: name.clone(),
                    span: Span::dummy(),
                }]),
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
        if let GoNode::RawCode { code, .. } = &result[0] {
            assert_eq!(code, "s[5:]");
        } else {
            panic!("expected RawCode, got {:?}", result[0]);
        }
    }

    #[test]
    fn test_transform_substring_two_args() {
        let plugin = JavaStringsPlugin;
        let node = make_method_call(
            Some(name_expr("s")),
            "substring",
            vec![int_lit(1), int_lit(4)],
        );
        let (resolver, config) = make_match_ctx(&node);

        let noop_walk = |child: &JavaNode| -> Result<Vec<GoNode>, PluginError> {
            match child {
                JavaNode::LiteralExpr {
                    value: LiteralValue::Int(v),
                    ..
                } => Ok(vec![GoNode::Literal {
                    value: GoLiteralValue::Int(*v),
                    span: Span::dummy(),
                }]),
                JavaNode::NameExpr { name, .. } => Ok(vec![GoNode::Ident {
                    name: name.clone(),
                    span: Span::dummy(),
                }]),
                _ => Ok(vec![]),
            }
        };
        let mut ctx = TransformContext::new(&node, &resolver, &config, &noop_walk);
        let result = plugin.transform(&mut ctx).unwrap();

        assert_eq!(result.len(), 1);
        if let GoNode::RawCode { code, .. } = &result[0] {
            assert_eq!(code, "s[1:4]");
        } else {
            panic!("expected RawCode, got {:?}", result[0]);
        }
    }

    #[test]
    fn test_transform_valueof() {
        let plugin = JavaStringsPlugin;
        let node = make_method_call(Some(name_expr("String")), "valueOf", vec![name_expr("x")]);
        let (resolver, config) = make_match_ctx(&node);

        let noop_walk = |child: &JavaNode| -> Result<Vec<GoNode>, PluginError> {
            match child {
                JavaNode::NameExpr { name, .. } => Ok(vec![GoNode::Ident {
                    name: name.clone(),
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
                assert_eq!(field, "Sprint");
            }
            assert_eq!(args.len(), 1);
        } else {
            panic!("expected CallExpr, got {:?}", result[0]);
        }
    }

    #[test]
    fn test_transform_isempty() {
        let plugin = JavaStringsPlugin;
        let node = make_method_call(Some(name_expr("s")), "isEmpty", vec![]);
        let (resolver, config) = make_match_ctx(&node);

        let noop_walk = |child: &JavaNode| -> Result<Vec<GoNode>, PluginError> {
            match child {
                JavaNode::NameExpr { name, .. } => Ok(vec![GoNode::Ident {
                    name: name.clone(),
                    span: Span::dummy(),
                }]),
                _ => Ok(vec![]),
            }
        };
        let mut ctx = TransformContext::new(&node, &resolver, &config, &noop_walk);
        let result = plugin.transform(&mut ctx).unwrap();

        assert_eq!(result.len(), 1);
        // len(s) == 0
        if let GoNode::BinaryExpr { left, op, right, .. } = &result[0] {
            assert_eq!(*op, GoBinaryOp::Eq);
            assert!(matches!(left.as_ref(), GoNode::CallExpr { .. }));
            assert!(matches!(
                right.as_ref(),
                GoNode::Literal { value: GoLiteralValue::Int(0), .. }
            ));
        } else {
            panic!("expected BinaryExpr, got {:?}", result[0]);
        }
    }
}
