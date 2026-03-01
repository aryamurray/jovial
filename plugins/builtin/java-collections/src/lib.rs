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
        "Converts Java collection constructors to idiomatic Go (make/slice literals)"
    }

    fn matches(&self, ctx: &MatchContext) -> bool {
        if let JavaNode::NewExpr {
            class_type,
            arguments,
            ..
        } = ctx.node
        {
            if !arguments.is_empty() {
                return false;
            }
            let name = class_type.name.as_str();
            MAP_TYPES.contains(&name) || LIST_SET_TYPES.contains(&name)
        } else {
            false
        }
    }

    fn transform(&self, ctx: &mut TransformContext) -> Result<Vec<GoNode>, PluginError> {
        let (class_type, span) = match ctx.node {
            JavaNode::NewExpr {
                class_type, span, ..
            } => (class_type, span),
            _ => return Err(PluginError::TransformFailed("expected NewExpr".into())),
        };

        let name = class_type.name.as_str();

        if MAP_TYPES.contains(&name) {
            // new HashMap<K,V>() → make(map[K]V)
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
            // new ArrayList<T>() → []T{}
            let go_type = convert_java_type_to_slice(class_type);
            Ok(vec![GoNode::CompositeLit {
                lit_type: go_type,
                elements: vec![],
                span: span.clone(),
            }])
        }
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

        let mut ctx = TransformContext::new(&node, &resolver, &config);
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

        let mut ctx = TransformContext::new(&node, &resolver, &config);
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

        let mut ctx = TransformContext::new(&node, &resolver, &config);
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
