use std::collections::{HashMap, HashSet};

use thiserror::Error;

use jovial_ast::go::GoNode;
use jovial_ast::java::{JavaCompilationUnit, JavaNode};
use jovial_ast::type_resolver::TypeResolver;
use jovial_plugin::context::{MatchContext, TransformContext};
use jovial_plugin::registry::PluginRegistry;
use jovial_plugin::types::ConfigValue;

use crate::default_convert::DefaultConverter;

/// Errors during AST walking.
#[derive(Debug, Error)]
pub enum WalkError {
    #[error("walk failed: {0}")]
    WalkFailed(String),

    #[error("plugin error: {0}")]
    PluginError(#[from] jovial_plugin::error::PluginError),
}

/// Walks a Java AST, dispatching nodes to plugins or the default converter.
pub struct Walker<'a> {
    registry: &'a PluginRegistry,
    default_converter: DefaultConverter,
    type_resolver: &'a dyn TypeResolver,
    config: &'a HashMap<String, ConfigValue>,
}

impl<'a> Walker<'a> {
    pub fn new(
        registry: &'a PluginRegistry,
        type_resolver: &'a dyn TypeResolver,
        config: &'a HashMap<String, ConfigValue>,
    ) -> Self {
        Self {
            registry,
            default_converter: DefaultConverter::new(),
            type_resolver,
            config,
        }
    }

    /// Walk an entire compilation unit, producing Go AST nodes.
    pub fn walk(&self, unit: &JavaCompilationUnit) -> Result<Vec<GoNode>, WalkError> {
        let mut nodes = Vec::new();

        // Package declaration
        let package_name = derive_package_name(unit.package.as_deref());
        nodes.push(GoNode::Package {
            name: package_name,
        });

        // Walk each top-level type declaration
        for type_decl in &unit.types {
            let converted = self.walk_node(type_decl)?;
            nodes.extend(converted);
        }

        Ok(nodes)
    }

    /// Walk a single Java node.
    pub fn walk_node(&self, node: &JavaNode) -> Result<Vec<GoNode>, WalkError> {
        self.walk_node_internal(node, None, None, None)
    }

    /// Walk a single Java node within a class context.
    ///
    /// Plugin dispatch happens first. If no plugin matches, the default
    /// converter handles the node with the class context threaded through
    /// so that `this` and bare field names resolve correctly.
    pub fn walk_node_in_class(
        &self,
        node: &JavaNode,
        class_name: &str,
        class_fields: &HashSet<String>,
    ) -> Result<Vec<GoNode>, WalkError> {
        self.walk_node_internal(node, Some(class_name), Some(class_fields), None)
    }

    fn walk_node_internal(
        &self,
        node: &JavaNode,
        class_name: Option<&str>,
        class_fields: Option<&HashSet<String>>,
        superclass: Option<&str>,
    ) -> Result<Vec<GoNode>, WalkError> {
        // 1. Try plugin dispatch
        let ctx = MatchContext {
            node,
            type_resolver: self.type_resolver,
            config: self.config,
        };

        if let Some(plugin) = self.registry.find_match(&ctx) {
            // Plugin walk_child preserves class context
            let walk_fn = |child: &JavaNode| -> Result<Vec<GoNode>, jovial_plugin::error::PluginError> {
                self.walk_node_internal(child, class_name, class_fields, superclass)
                    .map_err(|e| jovial_plugin::error::PluginError::WalkError(e.to_string()))
            };
            let mut transform_ctx =
                TransformContext::new(node, self.type_resolver, self.config, &walk_fn);
            let result = plugin.transform(&mut transform_ctx)?;
            // TODO: collect side-effects (imports, dependencies, diagnostics)
            return Ok(result);
        }

        // 2. For ClassDecl, extract field names and establish class context
        //    so that children get both plugin dispatch AND class context.
        if let JavaNode::ClassDecl { name, superclass: sc, members, .. } = node {
            let mut field_names = HashSet::new();
            for member in members {
                if let JavaNode::FieldDecl {
                    name: field_name, ..
                } = member
                {
                    field_names.insert(field_name.clone());
                }
            }
            let sc_name = sc.as_ref().map(|s| s.name.as_str());
            return self.default_converter.convert(
                node,
                &|child| self.walk_node_internal(child, Some(name), Some(&field_names), sc_name),
                Some(name),
                Some(&field_names),
                sc_name,
            );
        }

        // 3. For MethodDecl/ConstructorDecl with parameters that shadow field names,
        //    create a filtered class_fields that excludes parameter names so that
        //    bare references inside the body resolve to the parameter, not the field.
        if let Some(fields) = class_fields {
            let param_names: Option<Vec<&str>> = match node {
                JavaNode::MethodDecl { parameters, .. }
                | JavaNode::ConstructorDecl { parameters, .. } => {
                    let names: Vec<&str> = parameters
                        .iter()
                        .filter_map(|p| {
                            if let JavaNode::Parameter { name, .. } = p {
                                Some(name.as_str())
                            } else {
                                None
                            }
                        })
                        .collect();
                    if names.iter().any(|n| fields.contains(*n)) {
                        Some(names)
                    } else {
                        None
                    }
                }
                _ => None,
            };

            if let Some(names) = param_names {
                let filtered: HashSet<String> = fields
                    .iter()
                    .filter(|f| !names.contains(&f.as_str()))
                    .cloned()
                    .collect();
                return self.default_converter.convert(
                    node,
                    &|child| self.walk_node_internal(child, class_name, Some(&filtered), superclass),
                    class_name,
                    Some(&filtered),
                    superclass,
                );
            }
        }

        // 4. Fall back to default converter with inherited class context
        self.default_converter.convert(
            node,
            &|child| self.walk_node_internal(child, class_name, class_fields, superclass),
            class_name,
            class_fields,
            superclass,
        )
    }
}

/// Derive a Go package name from a Java package.
/// Uses the last dot-segment, lowercased. Falls back to "main".
fn derive_package_name(package: Option<&str>) -> String {
    match package {
        Some(pkg) if !pkg.is_empty() => pkg
            .rsplit('.')
            .next()
            .unwrap_or("main")
            .to_lowercase(),
        _ => "main".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jovial_ast::java::{JavaNode, JavaType, LiteralValue, Modifier};
    use jovial_ast::span::Span;

    /// Stub type resolver for tests.
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

    fn empty_walker() -> (PluginRegistry, NoopTypeResolver, HashMap<String, ConfigValue>) {
        (PluginRegistry::new(), NoopTypeResolver, HashMap::new())
    }

    // ── derive_package_name tests ───────────────────────────────

    #[test]
    fn test_derive_package_name_dotted() {
        assert_eq!(derive_package_name(Some("com.example.svc")), "svc");
    }

    #[test]
    fn test_derive_package_name_single() {
        assert_eq!(derive_package_name(Some("mypackage")), "mypackage");
    }

    #[test]
    fn test_derive_package_name_none() {
        assert_eq!(derive_package_name(None), "main");
    }

    #[test]
    fn test_derive_package_name_empty() {
        assert_eq!(derive_package_name(Some("")), "main");
    }

    // ── Integration tests ───────────────────────────────────────

    #[test]
    fn test_empty_class() {
        let (registry, resolver, config) = empty_walker();
        let walker = Walker::new(&registry, &resolver, &config);

        let unit = JavaCompilationUnit {
            package: Some("com.example.app".to_string()),
            imports: vec![],
            types: vec![JavaNode::ClassDecl {
                name: "Greeter".to_string(),
                modifiers: vec![Modifier::Public],
                superclass: None,
                interfaces: vec![],
                annotations: vec![],
                members: vec![],
                span: dummy_span(),
            }],
            span: dummy_span(),
        };

        let result = walker.walk(&unit).unwrap();
        assert!(result.len() >= 2);
        assert!(matches!(&result[0], GoNode::Package { name } if name == "app"));
        assert!(matches!(&result[1], GoNode::StructDecl { name, fields, .. } if name == "Greeter" && fields.is_empty()));
    }

    #[test]
    fn test_class_with_method() {
        let (registry, resolver, config) = empty_walker();
        let walker = Walker::new(&registry, &resolver, &config);

        let unit = JavaCompilationUnit {
            package: Some("com.example".to_string()),
            imports: vec![],
            types: vec![JavaNode::ClassDecl {
                name: "Service".to_string(),
                modifiers: vec![],
                superclass: None,
                interfaces: vec![],
                annotations: vec![],
                members: vec![JavaNode::MethodDecl {
                    name: "process".to_string(),
                    modifiers: vec![Modifier::Public],
                    return_type: Some(JavaType {
                        name: "void".to_string(),
                        type_args: vec![],
                        array_dimensions: 0,
                        is_varargs: false,
                    }),
                    parameters: vec![],
                    annotations: vec![],
                    body: Some(Box::new(JavaNode::BlockStmt {
                        statements: vec![],
                        span: dummy_span(),
                    })),
                    throws: vec![],
                    span: dummy_span(),
                }],
                span: dummy_span(),
            }],
            span: dummy_span(),
        };

        let result = walker.walk(&unit).unwrap();
        // Package + StructDecl + FuncDecl
        assert!(result.len() >= 3);
        if let GoNode::FuncDecl {
            name, receiver, ..
        } = &result[2]
        {
            assert_eq!(name, "Process");
            assert!(receiver.is_some());
            let recv = receiver.as_ref().unwrap();
            assert_eq!(recv.name, "s");
            assert!(recv.is_pointer);
        } else {
            panic!("expected FuncDecl, got {:?}", result[2]);
        }
    }

    #[test]
    fn test_static_method() {
        let (registry, resolver, config) = empty_walker();
        let walker = Walker::new(&registry, &resolver, &config);

        let unit = JavaCompilationUnit {
            package: None,
            imports: vec![],
            types: vec![JavaNode::ClassDecl {
                name: "Utils".to_string(),
                modifiers: vec![],
                superclass: None,
                interfaces: vec![],
                annotations: vec![],
                members: vec![JavaNode::MethodDecl {
                    name: "helper".to_string(),
                    modifiers: vec![Modifier::Public, Modifier::Static],
                    return_type: Some(JavaType {
                        name: "int".to_string(),
                        type_args: vec![],
                        array_dimensions: 0,
                        is_varargs: false,
                    }),
                    parameters: vec![],
                    annotations: vec![],
                    body: Some(Box::new(JavaNode::BlockStmt {
                        statements: vec![JavaNode::ReturnStmt {
                            value: Some(Box::new(JavaNode::LiteralExpr {
                                value: LiteralValue::Int(42),
                                span: dummy_span(),
                            })),
                            span: dummy_span(),
                        }],
                        span: dummy_span(),
                    })),
                    throws: vec![],
                    span: dummy_span(),
                }],
                span: dummy_span(),
            }],
            span: dummy_span(),
        };

        let result = walker.walk(&unit).unwrap();
        if let GoNode::FuncDecl {
            name, receiver, ..
        } = &result[2]
        {
            assert_eq!(name, "Helper");
            assert!(receiver.is_none());
        } else {
            panic!("expected FuncDecl, got {:?}", result[2]);
        }
    }

    #[test]
    fn test_constructor() {
        let (registry, resolver, config) = empty_walker();
        let walker = Walker::new(&registry, &resolver, &config);

        let unit = JavaCompilationUnit {
            package: None,
            imports: vec![],
            types: vec![JavaNode::ClassDecl {
                name: "Widget".to_string(),
                modifiers: vec![],
                superclass: None,
                interfaces: vec![],
                annotations: vec![],
                members: vec![JavaNode::ConstructorDecl {
                    name: "Widget".to_string(),
                    modifiers: vec![Modifier::Public],
                    parameters: vec![JavaNode::Parameter {
                        name: "name".to_string(),
                        param_type: JavaType {
                            name: "String".to_string(),
                            type_args: vec![],
                            array_dimensions: 0,
                            is_varargs: false,
                        },
                        annotations: vec![],
                        span: dummy_span(),
                    }],
                    annotations: vec![],
                    body: Box::new(JavaNode::BlockStmt {
                        statements: vec![],
                        span: dummy_span(),
                    }),
                    throws: vec![],
                    span: dummy_span(),
                }],
                span: dummy_span(),
            }],
            span: dummy_span(),
        };

        let result = walker.walk(&unit).unwrap();
        if let GoNode::FuncDecl {
            name,
            receiver,
            returns,
            ..
        } = &result[2]
        {
            assert_eq!(name, "NewWidget");
            assert!(receiver.is_none());
            assert!(returns[0].is_pointer);
            assert_eq!(returns[0].name, "Widget");
        } else {
            panic!("expected FuncDecl, got {:?}", result[2]);
        }
    }

    #[test]
    fn test_interface() {
        let (registry, resolver, config) = empty_walker();
        let walker = Walker::new(&registry, &resolver, &config);

        let unit = JavaCompilationUnit {
            package: None,
            imports: vec![],
            types: vec![JavaNode::InterfaceDecl {
                name: "Repository".to_string(),
                modifiers: vec![],
                extends: vec![],
                annotations: vec![],
                members: vec![JavaNode::MethodDecl {
                    name: "findById".to_string(),
                    modifiers: vec![],
                    return_type: Some(JavaType {
                        name: "Entity".to_string(),
                        type_args: vec![],
                        array_dimensions: 0,
                        is_varargs: false,
                    }),
                    parameters: vec![JavaNode::Parameter {
                        name: "id".to_string(),
                        param_type: JavaType {
                            name: "long".to_string(),
                            type_args: vec![],
                            array_dimensions: 0,
                            is_varargs: false,
                        },
                        annotations: vec![],
                        span: dummy_span(),
                    }],
                    annotations: vec![],
                    body: None,
                    throws: vec![],
                    span: dummy_span(),
                }],
                span: dummy_span(),
            }],
            span: dummy_span(),
        };

        let result = walker.walk(&unit).unwrap();
        if let GoNode::InterfaceDecl { name, methods, .. } = &result[1] {
            assert_eq!(name, "Repository");
            assert_eq!(methods.len(), 1);
            if let GoNode::FuncDecl { name, .. } = &methods[0] {
                assert_eq!(name, "FindById");
            }
        } else {
            panic!("expected InterfaceDecl");
        }
    }

    #[test]
    fn test_foreach_to_range() {
        let (registry, resolver, config) = empty_walker();
        let walker = Walker::new(&registry, &resolver, &config);

        let foreach = JavaNode::ForEachStmt {
            variable: "item".to_string(),
            variable_type: JavaType {
                name: "String".to_string(),
                type_args: vec![],
                array_dimensions: 0,
                is_varargs: false,
            },
            iterable: Box::new(JavaNode::NameExpr {
                name: "items".to_string(),
                span: dummy_span(),
            }),
            body: Box::new(JavaNode::BlockStmt {
                statements: vec![],
                span: dummy_span(),
            }),
            span: dummy_span(),
        };

        let result = walker.walk_node(&foreach).unwrap();
        assert_eq!(result.len(), 1);
        if let GoNode::RangeStmt {
            key, value, ..
        } = &result[0]
        {
            assert_eq!(key.as_deref(), Some("_"));
            assert_eq!(value.as_deref(), Some("item"));
        } else {
            panic!("expected RangeStmt");
        }
    }

    #[test]
    fn test_while_to_for() {
        let (registry, resolver, config) = empty_walker();
        let walker = Walker::new(&registry, &resolver, &config);

        let while_stmt = JavaNode::WhileStmt {
            condition: Box::new(JavaNode::LiteralExpr {
                value: LiteralValue::Bool(true),
                span: dummy_span(),
            }),
            body: Box::new(JavaNode::BlockStmt {
                statements: vec![],
                span: dummy_span(),
            }),
            span: dummy_span(),
        };

        let result = walker.walk_node(&while_stmt).unwrap();
        assert_eq!(result.len(), 1);
        assert!(matches!(&result[0], GoNode::ForStmt { init: None, post: None, .. }));
    }

    #[test]
    fn test_try_catch_to_raw_code() {
        let (registry, resolver, config) = empty_walker();
        let walker = Walker::new(&registry, &resolver, &config);

        let try_catch = JavaNode::TryCatchStmt {
            try_block: Box::new(JavaNode::BlockStmt {
                statements: vec![JavaNode::ReturnStmt {
                    value: None,
                    span: dummy_span(),
                }],
                span: dummy_span(),
            }),
            catches: vec![JavaNode::CatchClause {
                parameter: "e".to_string(),
                exception_types: vec![JavaType {
                    name: "Exception".to_string(),
                    type_args: vec![],
                    array_dimensions: 0,
                    is_varargs: false,
                }],
                body: Box::new(JavaNode::BlockStmt {
                    statements: vec![],
                    span: dummy_span(),
                }),
                span: dummy_span(),
            }],
            finally_block: None,
            span: dummy_span(),
        };

        let result = walker.walk_node(&try_catch).unwrap();
        // Should have a RawCode comment + inlined body + catch clause
        assert!(result.iter().any(|n| matches!(n, GoNode::RawCode { .. })));
    }

    #[test]
    fn test_this_field_access() {
        let (registry, resolver, config) = empty_walker();
        let walker = Walker::new(&registry, &resolver, &config);

        // Build a class with a method that accesses this.name
        let unit = JavaCompilationUnit {
            package: None,
            imports: vec![],
            types: vec![JavaNode::ClassDecl {
                name: "Person".to_string(),
                modifiers: vec![],
                superclass: None,
                interfaces: vec![],
                annotations: vec![],
                members: vec![
                    JavaNode::FieldDecl {
                        name: "name".to_string(),
                        modifiers: vec![],
                        field_type: JavaType {
                            name: "String".to_string(),
                            type_args: vec![],
                            array_dimensions: 0,
                            is_varargs: false,
                        },
                        initializer: None,
                        annotations: vec![],
                        span: dummy_span(),
                    },
                    JavaNode::MethodDecl {
                        name: "getName".to_string(),
                        modifiers: vec![],
                        return_type: Some(JavaType {
                            name: "String".to_string(),
                            type_args: vec![],
                            array_dimensions: 0,
                            is_varargs: false,
                        }),
                        parameters: vec![],
                        annotations: vec![],
                        body: Some(Box::new(JavaNode::BlockStmt {
                            statements: vec![JavaNode::ReturnStmt {
                                value: Some(Box::new(JavaNode::FieldAccessExpr {
                                    object: Box::new(JavaNode::NameExpr {
                                        name: "this".to_string(),
                                        span: dummy_span(),
                                    }),
                                    field: "name".to_string(),
                                    span: dummy_span(),
                                })),
                                span: dummy_span(),
                            }],
                            span: dummy_span(),
                        })),
                        throws: vec![],
                        span: dummy_span(),
                    },
                ],
                span: dummy_span(),
            }],
            span: dummy_span(),
        };

        let result = walker.walk(&unit).unwrap();
        // Find the FuncDecl for GetName
        let func = result.iter().find(|n| {
            matches!(n, GoNode::FuncDecl { name, .. } if name == "GetName")
        });
        assert!(func.is_some(), "should have GetName function");

        // The body should contain a return with a SelectorExpr
        if let GoNode::FuncDecl { body, .. } = func.unwrap() {
            if let Some(GoNode::ReturnStmt { values, .. }) = body.first() {
                if let Some(GoNode::SelectorExpr { object, field, .. }) = values.first() {
                    assert_eq!(field, "Name");
                    // The walker threads class context through, so "this"
                    // resolves to the receiver name "p" (first char of "Person").
                    if let GoNode::Ident { name, .. } = object.as_ref() {
                        assert_eq!(name, "p");
                    } else {
                        panic!("expected Ident for object");
                    }
                } else {
                    panic!("expected SelectorExpr in return");
                }
            } else {
                panic!("expected ReturnStmt");
            }
        }
    }

    #[test]
    fn test_bare_field_name_gets_receiver_prefix() {
        let (registry, resolver, config) = empty_walker();
        let walker = Walker::new(&registry, &resolver, &config);

        // class Person { String name; String getName() { return name; } }
        // bare "name" inside method body should become "p.Name"
        let unit = JavaCompilationUnit {
            package: None,
            imports: vec![],
            types: vec![JavaNode::ClassDecl {
                name: "Person".to_string(),
                modifiers: vec![],
                superclass: None,
                interfaces: vec![],
                annotations: vec![],
                members: vec![
                    JavaNode::FieldDecl {
                        name: "name".to_string(),
                        modifiers: vec![],
                        field_type: JavaType {
                            name: "String".to_string(),
                            type_args: vec![],
                            array_dimensions: 0,
                            is_varargs: false,
                        },
                        initializer: None,
                        annotations: vec![],
                        span: dummy_span(),
                    },
                    JavaNode::MethodDecl {
                        name: "getName".to_string(),
                        modifiers: vec![],
                        return_type: Some(JavaType {
                            name: "String".to_string(),
                            type_args: vec![],
                            array_dimensions: 0,
                            is_varargs: false,
                        }),
                        parameters: vec![],
                        annotations: vec![],
                        body: Some(Box::new(JavaNode::BlockStmt {
                            statements: vec![JavaNode::ReturnStmt {
                                value: Some(Box::new(JavaNode::NameExpr {
                                    name: "name".to_string(),
                                    span: dummy_span(),
                                })),
                                span: dummy_span(),
                            }],
                            span: dummy_span(),
                        })),
                        throws: vec![],
                        span: dummy_span(),
                    },
                ],
                span: dummy_span(),
            }],
            span: dummy_span(),
        };

        let result = walker.walk(&unit).unwrap();
        let func = result.iter().find(|n| {
            matches!(n, GoNode::FuncDecl { name, .. } if name == "GetName")
        }).expect("should have GetName function");

        if let GoNode::FuncDecl { body, .. } = func {
            if let Some(GoNode::ReturnStmt { values, .. }) = body.first() {
                // Bare "name" should become SelectorExpr: p.Name
                if let Some(GoNode::SelectorExpr { object, field, .. }) = values.first() {
                    assert_eq!(field, "Name");
                    if let GoNode::Ident { name, .. } = object.as_ref() {
                        assert_eq!(name, "p");
                    } else {
                        panic!("expected Ident receiver, got {:?}", object);
                    }
                } else {
                    panic!("expected SelectorExpr, got {:?}", values.first());
                }
            } else {
                panic!("expected ReturnStmt");
            }
        }
    }

}
