use jovial_plugin::prelude::*;

pub struct JavaCollectionsPlugin;

const MAP_TYPES: &[&str] = &[
    "HashMap",
    "TreeMap",
    "LinkedHashMap",
    "ConcurrentHashMap",
    "java.util.HashMap",
    "java.util.TreeMap",
    "java.util.LinkedHashMap",
    "java.util.concurrent.ConcurrentHashMap",
];

const LIST_SET_TYPES: &[&str] = &[
    "ArrayList",
    "LinkedList",
    "HashSet",
    "TreeSet",
    "LinkedHashSet",
    "java.util.ArrayList",
    "java.util.LinkedList",
    "java.util.HashSet",
    "java.util.TreeSet",
    "java.util.LinkedHashSet",
];

/// Collection method names that this plugin handles.
const COLLECTION_METHODS: &[&str] = &[
    "add",
    "get",
    "set",
    "size",
    "isEmpty",
    "contains",
    "put",
    "containsKey",
    "keySet",
];

impl Plugin for JavaCollectionsPlugin {
    fn name(&self) -> &str {
        "java-collections"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn priority(&self) -> i32 {
        50
    }

    fn description(&self) -> &str {
        "Converts Java collection constructors and methods to idiomatic Go"
    }

    fn matches(&self, ctx: &MatchContext) -> bool {
        match ctx.node {
            JavaNode::NewExpr {
                class_type,
                arguments,
                ..
            } => {
                if !arguments.is_empty() {
                    return false;
                }
                let name = class_type.name.as_str();
                MAP_TYPES.contains(&name) || LIST_SET_TYPES.contains(&name)
            }
            JavaNode::MethodCallExpr {
                object: Some(_),
                name,
                arguments,
                ..
            } => {
                let method = name.as_str();
                if !COLLECTION_METHODS.contains(&method) {
                    return false;
                }
                // Validate arg count to avoid matching non-collection methods
                // (e.g., Future.get() with 0 args vs List.get(index) with 1 arg)
                match method {
                    "get" | "add" | "contains" | "containsKey" => arguments.len() >= 1,
                    "put" | "set" => arguments.len() >= 2,
                    "size" | "isEmpty" | "keySet" => true,
                    _ => true,
                }
            }
            _ => false,
        }
    }

    fn transform(&self, ctx: &mut TransformContext) -> Result<Vec<GoNode>, PluginError> {
        match ctx.node {
            JavaNode::NewExpr { .. } => self.transform_new_expr(ctx),
            JavaNode::MethodCallExpr { .. } => self.transform_method_call(ctx),
            _ => Err(PluginError::TransformFailed(
                "expected NewExpr or MethodCallExpr".into(),
            )),
        }
    }
}

impl JavaCollectionsPlugin {
    fn transform_new_expr(&self, ctx: &mut TransformContext) -> Result<Vec<GoNode>, PluginError> {
        let (class_type, span) = match ctx.node {
            JavaNode::NewExpr {
                class_type, span, ..
            } => (class_type, span),
            _ => return Err(PluginError::TransformFailed("expected NewExpr".into())),
        };

        let name = class_type.name.as_str();

        if MAP_TYPES.contains(&name) {
            let go_type = convert_java_type_to_map(class_type);
            Ok(vec![GoNode::CallExpr {
                function: Box::new(GoNode::Ident {
                    name: "make".to_string(),
                    span: span.clone(),
                }),
                args: vec![GoNode::TypeRef {
                    go_type,
                    span: span.clone(),
                }],
                span: span.clone(),
            }])
        } else {
            let go_type = convert_java_type_to_slice(class_type);
            Ok(vec![GoNode::CompositeLit {
                lit_type: go_type,
                elements: vec![],
                span: span.clone(),
            }])
        }
    }

    fn transform_method_call(
        &self,
        ctx: &mut TransformContext,
    ) -> Result<Vec<GoNode>, PluginError> {
        let (object, method, arguments, span) = match ctx.node {
            JavaNode::MethodCallExpr {
                object,
                name,
                arguments,
                span,
                ..
            } => (object, name.as_str(), arguments, span),
            _ => {
                return Err(PluginError::TransformFailed(
                    "expected MethodCallExpr".into(),
                ))
            }
        };

        match method {
            // list.add(x) -> list = append(list, x)
            "add" => {
                let recv = walk_object(ctx, object)?;
                let arg = walk_first_arg(ctx, arguments)?;
                Ok(vec![GoNode::AssignStmt {
                    lhs: vec![recv.clone()],
                    rhs: vec![GoNode::CallExpr {
                        function: Box::new(GoNode::Ident {
                            name: "append".to_string(),
                            span: span.clone(),
                        }),
                        args: vec![recv, arg],
                        span: span.clone(),
                    }],
                    define: false,
                    span: span.clone(),
                }])
            }

            // list.get(i) -> list[i]
            // map.get(k) -> map[k]
            "get" => {
                let recv = walk_object(ctx, object)?;
                let arg = walk_first_arg(ctx, arguments)?;
                let recv_code = inline_code(&recv);
                let arg_code = inline_code(&arg);
                Ok(vec![GoNode::RawCode {
                    code: format!("{recv_code}[{arg_code}]"),
                    span: span.clone(),
                }])
            }

            // list.set(i, x) -> list[i] = x
            "set" => {
                let recv = walk_object(ctx, object)?;
                let walked_args = ctx.walk_children(arguments)?;
                if walked_args.len() < 2 {
                    return Err(PluginError::TransformFailed(
                        "set requires 2 arguments".into(),
                    ));
                }
                let recv_code = inline_code(&recv);
                let idx_code = inline_code(&walked_args[0]);
                let val_code = inline_code(&walked_args[1]);
                Ok(vec![GoNode::RawCode {
                    code: format!("{recv_code}[{idx_code}] = {val_code}"),
                    span: span.clone(),
                }])
            }

            // list.size() / map.size() -> len(x)
            "size" => {
                let recv = walk_object(ctx, object)?;
                Ok(vec![builtin_call("len", vec![recv], span)])
            }

            // list.isEmpty() / map.isEmpty() -> len(x) == 0
            "isEmpty" => {
                let recv = walk_object(ctx, object)?;
                Ok(vec![GoNode::BinaryExpr {
                    left: Box::new(builtin_call("len", vec![recv], span)),
                    op: GoBinaryOp::Eq,
                    right: Box::new(GoNode::Literal {
                        value: GoLiteralValue::Int(0),
                        span: span.clone(),
                    }),
                    span: span.clone(),
                }])
            }

            // list.contains(x) -> slices.Contains(list, x)
            "contains" => {
                ctx.add_import("slices", None);
                let recv = walk_object(ctx, object)?;
                let arg = walk_first_arg(ctx, arguments)?;
                Ok(vec![pkg_call("slices", "Contains", vec![recv, arg], span)])
            }

            // map.put(k, v) -> map[k] = v
            "put" => {
                let recv = walk_object(ctx, object)?;
                let walked_args = ctx.walk_children(arguments)?;
                if walked_args.len() < 2 {
                    return Err(PluginError::TransformFailed(
                        "put requires 2 arguments".into(),
                    ));
                }
                let recv_code = inline_code(&recv);
                let key_code = inline_code(&walked_args[0]);
                let val_code = inline_code(&walked_args[1]);
                Ok(vec![GoNode::RawCode {
                    code: format!("{recv_code}[{key_code}] = {val_code}"),
                    span: span.clone(),
                }])
            }

            // map.containsKey(k) -> /* _, ok := map[k]; ok */
            "containsKey" => {
                let recv = walk_object(ctx, object)?;
                let arg = walk_first_arg(ctx, arguments)?;
                let recv_code = inline_code(&recv);
                let arg_code = inline_code(&arg);
                Ok(vec![GoNode::RawCode {
                    code: format!(
                        "/* _, ok := {recv_code}[{arg_code}]; ok */ // TODO: containsKey needs context-aware rewrite"
                    ),
                    span: span.clone(),
                }])
            }

            // map.keySet() -> // TODO: map.keySet()
            "keySet" => {
                let recv = walk_object(ctx, object)?;
                let recv_code = inline_code(&recv);
                Ok(vec![GoNode::RawCode {
                    code: format!("// TODO: {recv_code}.keySet() — no clean Go equivalent"),
                    span: span.clone(),
                }])
            }

            _ => Err(PluginError::TransformFailed(format!(
                "unhandled collection method: {method}"
            ))),
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────

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
fn walk_first_arg(ctx: &TransformContext, arguments: &[JavaNode]) -> Result<GoNode, PluginError> {
    let arg = arguments
        .first()
        .ok_or_else(|| PluginError::TransformFailed("expected at least one argument".into()))?;
    let nodes = ctx.walk_child(arg)?;
    nodes
        .into_iter()
        .next()
        .ok_or_else(|| PluginError::TransformFailed("argument produced no nodes".into()))
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

/// Simple inline code representation for a GoNode.
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
        _ => "/* TODO: complex expr */".to_string(),
    }
}

fn simple_go_type(name: &str) -> GoType {
    GoType {
        name: name.to_string(),
        package: None,
        is_pointer: false,
        is_slice: false,
        is_map: false,
        key_type: None,
        value_type: None,
    }
}

fn java_type_to_go_simple(jt: &JavaType) -> GoType {
    let name = match jt.name.as_str() {
        "boolean" | "Boolean" | "java.lang.Boolean" => "bool",
        "byte" | "Byte" | "java.lang.Byte" => "byte",
        "short" | "Short" | "java.lang.Short" => "int16",
        "int" | "Integer" | "java.lang.Integer" => "int",
        "long" | "Long" | "java.lang.Long" => "int64",
        "float" | "Float" | "java.lang.Float" => "float32",
        "double" | "Double" | "java.lang.Double" => "float64",
        "char" | "Character" | "java.lang.Character" => "rune",
        "String" | "java.lang.String" => "string",
        "Object" | "java.lang.Object" => "interface{}",
        other => other,
    };
    simple_go_type(name)
}

fn convert_java_type_to_map(jt: &JavaType) -> GoType {
    let key = jt
        .type_args
        .first()
        .map(java_type_to_go_simple)
        .unwrap_or_else(|| simple_go_type("string"));
    let val = jt
        .type_args
        .get(1)
        .map(java_type_to_go_simple)
        .unwrap_or_else(|| simple_go_type("interface{}"));
    GoType {
        name: "map".to_string(),
        package: None,
        is_pointer: false,
        is_slice: false,
        is_map: true,
        key_type: Some(Box::new(key)),
        value_type: Some(Box::new(val)),
    }
}

fn convert_java_type_to_slice(jt: &JavaType) -> GoType {
    let elem = jt
        .type_args
        .first()
        .map(java_type_to_go_simple)
        .unwrap_or_else(|| simple_go_type("interface{}"));
    GoType {
        name: elem.name.clone(),
        package: None,
        is_pointer: false,
        is_slice: true,
        is_map: false,
        key_type: None,
        value_type: Some(Box::new(elem)),
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

    #[test]
    fn test_matches_hashmap() {
        let plugin = JavaCollectionsPlugin;
        let resolver = NoopTypeResolver;
        let config = HashMap::new();

        let node = JavaNode::NewExpr {
            class_type: JavaType {
                name: "HashMap".to_string(),
                type_args: vec![
                    JavaType {
                        name: "String".to_string(),
                        type_args: vec![],
                        array_dimensions: 0,
                        is_varargs: false,
                    },
                    JavaType {
                        name: "Integer".to_string(),
                        type_args: vec![],
                        array_dimensions: 0,
                        is_varargs: false,
                    },
                ],
                array_dimensions: 0,
                is_varargs: false,
            },
            arguments: vec![],
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
    fn test_matches_arraylist() {
        let plugin = JavaCollectionsPlugin;
        let resolver = NoopTypeResolver;
        let config = HashMap::new();

        let node = JavaNode::NewExpr {
            class_type: JavaType {
                name: "ArrayList".to_string(),
                type_args: vec![JavaType {
                    name: "String".to_string(),
                    type_args: vec![],
                    array_dimensions: 0,
                    is_varargs: false,
                }],
                array_dimensions: 0,
                is_varargs: false,
            },
            arguments: vec![],
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
    fn test_no_match_on_regular_new() {
        let plugin = JavaCollectionsPlugin;
        let resolver = NoopTypeResolver;
        let config = HashMap::new();

        let node = JavaNode::NewExpr {
            class_type: JavaType {
                name: "MyClass".to_string(),
                type_args: vec![],
                array_dimensions: 0,
                is_varargs: false,
            },
            arguments: vec![],
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
    fn test_no_match_on_new_with_args() {
        let plugin = JavaCollectionsPlugin;
        let resolver = NoopTypeResolver;
        let config = HashMap::new();

        // new HashMap(16) — has arguments, skip (let default converter handle)
        let node = JavaNode::NewExpr {
            class_type: JavaType {
                name: "HashMap".to_string(),
                type_args: vec![],
                array_dimensions: 0,
                is_varargs: false,
            },
            arguments: vec![JavaNode::LiteralExpr {
                value: LiteralValue::Int(16),
                span: dummy_span(),
            }],
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
    fn test_transform_hashmap_to_make() {
        let plugin = JavaCollectionsPlugin;
        let resolver = NoopTypeResolver;
        let config = HashMap::new();

        let node = JavaNode::NewExpr {
            class_type: JavaType {
                name: "HashMap".to_string(),
                type_args: vec![
                    JavaType {
                        name: "String".to_string(),
                        type_args: vec![],
                        array_dimensions: 0,
                        is_varargs: false,
                    },
                    JavaType {
                        name: "Integer".to_string(),
                        type_args: vec![],
                        array_dimensions: 0,
                        is_varargs: false,
                    },
                ],
                array_dimensions: 0,
                is_varargs: false,
            },
            arguments: vec![],
            span: dummy_span(),
        };

        let noop_walk = |_: &JavaNode| -> Result<Vec<GoNode>, PluginError> { Ok(vec![]) };
        let mut ctx = TransformContext::new(&node, &resolver, &config, &noop_walk);
        let result = plugin.transform(&mut ctx).unwrap();

        assert_eq!(result.len(), 1);
        if let GoNode::CallExpr { function, args, .. } = &result[0] {
            if let GoNode::Ident { name, .. } = function.as_ref() {
                assert_eq!(name, "make");
            } else {
                panic!("expected make Ident, got {:?}", function);
            }
            assert_eq!(args.len(), 1);
            if let GoNode::TypeRef { go_type, .. } = &args[0] {
                assert!(go_type.is_map);
                assert_eq!(go_type.key_type.as_ref().unwrap().name, "string");
                assert_eq!(go_type.value_type.as_ref().unwrap().name, "int");
            } else {
                panic!("expected TypeRef arg, got {:?}", args[0]);
            }
        } else {
            panic!("expected CallExpr, got {:?}", result[0]);
        }
    }

    #[test]
    fn test_transform_arraylist_to_slice() {
        let plugin = JavaCollectionsPlugin;
        let resolver = NoopTypeResolver;
        let config = HashMap::new();

        let node = JavaNode::NewExpr {
            class_type: JavaType {
                name: "ArrayList".to_string(),
                type_args: vec![JavaType {
                    name: "String".to_string(),
                    type_args: vec![],
                    array_dimensions: 0,
                    is_varargs: false,
                }],
                array_dimensions: 0,
                is_varargs: false,
            },
            arguments: vec![],
            span: dummy_span(),
        };

        let noop_walk = |_: &JavaNode| -> Result<Vec<GoNode>, PluginError> { Ok(vec![]) };
        let mut ctx = TransformContext::new(&node, &resolver, &config, &noop_walk);
        let result = plugin.transform(&mut ctx).unwrap();

        assert_eq!(result.len(), 1);
        if let GoNode::CompositeLit {
            lit_type, elements, ..
        } = &result[0]
        {
            assert!(lit_type.is_slice);
            assert_eq!(lit_type.value_type.as_ref().unwrap().name, "string");
            assert!(elements.is_empty());
        } else {
            panic!("expected CompositeLit, got {:?}", result[0]);
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

    fn simple_walk(child: &JavaNode) -> Result<Vec<GoNode>, PluginError> {
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
            JavaNode::LiteralExpr {
                value: LiteralValue::String(s),
                ..
            } => Ok(vec![GoNode::Literal {
                value: GoLiteralValue::String(s.clone()),
                span: Span::dummy(),
            }]),
            _ => Ok(vec![]),
        }
    }

    #[test]
    fn test_matches_method_call_add() {
        let plugin = JavaCollectionsPlugin;
        let resolver = NoopTypeResolver;
        let config = HashMap::new();
        let node = make_method_call(Some(name_expr("list")), "add", vec![name_expr("x")]);
        let ctx = MatchContext {
            node: &node,
            type_resolver: &resolver,
            config: &config,
        };
        assert!(plugin.matches(&ctx));
    }

    #[test]
    fn test_matches_method_call_size() {
        let plugin = JavaCollectionsPlugin;
        let resolver = NoopTypeResolver;
        let config = HashMap::new();
        let node = make_method_call(Some(name_expr("list")), "size", vec![]);
        let ctx = MatchContext {
            node: &node,
            type_resolver: &resolver,
            config: &config,
        };
        assert!(plugin.matches(&ctx));
    }

    #[test]
    fn test_no_match_unknown_method() {
        let plugin = JavaCollectionsPlugin;
        let resolver = NoopTypeResolver;
        let config = HashMap::new();
        let node = make_method_call(Some(name_expr("list")), "fooBar", vec![]);
        let ctx = MatchContext {
            node: &node,
            type_resolver: &resolver,
            config: &config,
        };
        assert!(!plugin.matches(&ctx));
    }

    #[test]
    fn test_transform_list_add() {
        let plugin = JavaCollectionsPlugin;
        let resolver = NoopTypeResolver;
        let config = HashMap::new();
        let node = make_method_call(Some(name_expr("items")), "add", vec![name_expr("x")]);
        let mut ctx = TransformContext::new(&node, &resolver, &config, &simple_walk);
        let result = plugin.transform(&mut ctx).unwrap();

        assert_eq!(result.len(), 1);
        // items = append(items, x)
        if let GoNode::AssignStmt { lhs, rhs, .. } = &result[0] {
            assert_eq!(lhs.len(), 1);
            if let GoNode::Ident { name, .. } = &lhs[0] {
                assert_eq!(name, "items");
            }
            assert_eq!(rhs.len(), 1);
            if let GoNode::CallExpr { function, args, .. } = &rhs[0] {
                if let GoNode::Ident { name, .. } = function.as_ref() {
                    assert_eq!(name, "append");
                }
                assert_eq!(args.len(), 2);
            }
        } else {
            panic!("expected AssignStmt, got {:?}", result[0]);
        }
    }

    #[test]
    fn test_transform_list_get() {
        let plugin = JavaCollectionsPlugin;
        let resolver = NoopTypeResolver;
        let config = HashMap::new();
        let node = make_method_call(Some(name_expr("list")), "get", vec![int_lit(0)]);
        let mut ctx = TransformContext::new(&node, &resolver, &config, &simple_walk);
        let result = plugin.transform(&mut ctx).unwrap();

        assert_eq!(result.len(), 1);
        if let GoNode::RawCode { code, .. } = &result[0] {
            assert_eq!(code, "list[0]");
        } else {
            panic!("expected RawCode, got {:?}", result[0]);
        }
    }

    #[test]
    fn test_transform_list_size() {
        let plugin = JavaCollectionsPlugin;
        let resolver = NoopTypeResolver;
        let config = HashMap::new();
        let node = make_method_call(Some(name_expr("list")), "size", vec![]);
        let mut ctx = TransformContext::new(&node, &resolver, &config, &simple_walk);
        let result = plugin.transform(&mut ctx).unwrap();

        assert_eq!(result.len(), 1);
        if let GoNode::CallExpr { function, args, .. } = &result[0] {
            if let GoNode::Ident { name, .. } = function.as_ref() {
                assert_eq!(name, "len");
            }
            assert_eq!(args.len(), 1);
        } else {
            panic!("expected CallExpr, got {:?}", result[0]);
        }
    }

    #[test]
    fn test_transform_list_isempty() {
        let plugin = JavaCollectionsPlugin;
        let resolver = NoopTypeResolver;
        let config = HashMap::new();
        let node = make_method_call(Some(name_expr("list")), "isEmpty", vec![]);
        let mut ctx = TransformContext::new(&node, &resolver, &config, &simple_walk);
        let result = plugin.transform(&mut ctx).unwrap();

        assert_eq!(result.len(), 1);
        assert!(matches!(
            &result[0],
            GoNode::BinaryExpr { op: GoBinaryOp::Eq, .. }
        ));
    }

    #[test]
    fn test_transform_list_contains() {
        let plugin = JavaCollectionsPlugin;
        let resolver = NoopTypeResolver;
        let config = HashMap::new();
        let node = make_method_call(Some(name_expr("list")), "contains", vec![name_expr("x")]);
        let mut ctx = TransformContext::new(&node, &resolver, &config, &simple_walk);
        let result = plugin.transform(&mut ctx).unwrap();

        assert_eq!(result.len(), 1);
        if let GoNode::CallExpr { function, args, .. } = &result[0] {
            if let GoNode::SelectorExpr { object, field, .. } = function.as_ref() {
                if let GoNode::Ident { name, .. } = object.as_ref() {
                    assert_eq!(name, "slices");
                }
                assert_eq!(field, "Contains");
            }
            assert_eq!(args.len(), 2);
        } else {
            panic!("expected CallExpr, got {:?}", result[0]);
        }
    }

    #[test]
    fn test_transform_map_put() {
        let plugin = JavaCollectionsPlugin;
        let resolver = NoopTypeResolver;
        let config = HashMap::new();
        let node = make_method_call(
            Some(name_expr("m")),
            "put",
            vec![string_literal("key"), int_lit(42)],
        );
        let mut ctx = TransformContext::new(&node, &resolver, &config, &simple_walk);
        let result = plugin.transform(&mut ctx).unwrap();

        assert_eq!(result.len(), 1);
        if let GoNode::RawCode { code, .. } = &result[0] {
            assert_eq!(code, "m[\"key\"] = 42");
        } else {
            panic!("expected RawCode, got {:?}", result[0]);
        }
    }

    #[test]
    fn test_transform_map_containskey() {
        let plugin = JavaCollectionsPlugin;
        let resolver = NoopTypeResolver;
        let config = HashMap::new();
        let node = make_method_call(
            Some(name_expr("m")),
            "containsKey",
            vec![string_literal("k")],
        );
        let mut ctx = TransformContext::new(&node, &resolver, &config, &simple_walk);
        let result = plugin.transform(&mut ctx).unwrap();

        assert_eq!(result.len(), 1);
        if let GoNode::RawCode { code, .. } = &result[0] {
            assert!(code.contains("_, ok :="));
            assert!(code.contains("m[\"k\"]"));
        } else {
            panic!("expected RawCode, got {:?}", result[0]);
        }
    }

    #[test]
    fn test_transform_list_set() {
        let plugin = JavaCollectionsPlugin;
        let resolver = NoopTypeResolver;
        let config = HashMap::new();
        let node = make_method_call(
            Some(name_expr("list")),
            "set",
            vec![int_lit(0), string_literal("val")],
        );
        let mut ctx = TransformContext::new(&node, &resolver, &config, &simple_walk);
        let result = plugin.transform(&mut ctx).unwrap();

        assert_eq!(result.len(), 1);
        if let GoNode::RawCode { code, .. } = &result[0] {
            assert_eq!(code, "list[0] = \"val\"");
        } else {
            panic!("expected RawCode, got {:?}", result[0]);
        }
    }

    #[test]
    fn test_transform_hashset_to_slice() {
        let plugin = JavaCollectionsPlugin;
        let resolver = NoopTypeResolver;
        let config = HashMap::new();

        let node = JavaNode::NewExpr {
            class_type: JavaType {
                name: "HashSet".to_string(),
                type_args: vec![JavaType {
                    name: "Integer".to_string(),
                    type_args: vec![],
                    array_dimensions: 0,
                    is_varargs: false,
                }],
                array_dimensions: 0,
                is_varargs: false,
            },
            arguments: vec![],
            span: dummy_span(),
        };

        let noop_walk = |_: &JavaNode| -> Result<Vec<GoNode>, PluginError> { Ok(vec![]) };
        let mut ctx = TransformContext::new(&node, &resolver, &config, &noop_walk);
        let result = plugin.transform(&mut ctx).unwrap();

        assert_eq!(result.len(), 1);
        if let GoNode::CompositeLit {
            lit_type, elements, ..
        } = &result[0]
        {
            assert!(lit_type.is_slice);
            assert_eq!(lit_type.value_type.as_ref().unwrap().name, "int");
            assert!(elements.is_empty());
        } else {
            panic!("expected CompositeLit, got {:?}", result[0]);
        }
    }
}
