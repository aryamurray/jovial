#[cfg(test)]
mod tests {
    use crate::parse_java;
    use jovial_ast::java::{JavaCompilationUnit, JavaNode};

    fn parse(src: &str) -> JavaCompilationUnit {
        parse_java(src, "Test.java").expect("parse failed")
    }

    fn parse_in_class(src: &str) -> Vec<JavaNode> {
        let wrapped = format!("class Foo {{ {} }}", src);
        let cu = parse(&wrapped);
        match &cu.types[0] {
            JavaNode::ClassDecl { members, .. } => members.clone(),
            other => panic!("expected ClassDecl, got: {:?}", other),
        }
    }

    fn parse_in_method(src: &str) -> Vec<JavaNode> {
        let members = parse_in_class(&format!("void test() {{ {} }}", src));
        match &members[0] {
            JavaNode::MethodDecl { body: Some(body), .. } => {
                match body.as_ref() {
                    JavaNode::BlockStmt { statements, .. } => statements.clone(),
                    other => panic!("expected BlockStmt, got: {:?}", other),
                }
            }
            other => panic!("expected MethodDecl, got: {:?}", other),
        }
    }

    // ── Unit tests ──────────────────────────────────────────────────

    #[test]
    fn empty_class() {
        let cu = parse("class Foo {}");
        assert_eq!(cu.types.len(), 1);
        match &cu.types[0] {
            JavaNode::ClassDecl { name, members, .. } => {
                assert_eq!(name, "Foo");
                assert!(members.is_empty());
            }
            other => panic!("expected ClassDecl, got: {:?}", other),
        }
    }

    #[test]
    fn class_with_extends_implements() {
        let cu = parse("class Foo extends Bar implements Baz, Qux {}");
        match &cu.types[0] {
            JavaNode::ClassDecl { name, superclass, interfaces, .. } => {
                assert_eq!(name, "Foo");
                assert_eq!(superclass.as_ref().unwrap().name, "Bar");
                assert_eq!(interfaces.len(), 2);
                assert_eq!(interfaces[0].name, "Baz");
                assert_eq!(interfaces[1].name, "Qux");
            }
            other => panic!("expected ClassDecl, got: {:?}", other),
        }
    }

    #[test]
    fn method_with_params() {
        let members = parse_in_class("public int add(int a, int b) { return a + b; }");
        assert_eq!(members.len(), 1);
        match &members[0] {
            JavaNode::MethodDecl { name, parameters, return_type, modifiers, .. } => {
                assert_eq!(name, "add");
                assert_eq!(parameters.len(), 2);
                assert!(return_type.is_some());
                assert_eq!(return_type.as_ref().unwrap().name, "int");
                assert!(!modifiers.is_empty());
            }
            other => panic!("expected MethodDecl, got: {:?}", other),
        }
    }

    #[test]
    fn field_with_annotation() {
        let members = parse_in_class("@Deprecated private String name;");
        assert_eq!(members.len(), 1);
        match &members[0] {
            JavaNode::FieldDecl { name, annotations, .. } => {
                assert_eq!(name, "name");
                assert_eq!(annotations.len(), 1);
                match annotations[0].as_ref() {
                    JavaNode::AnnotationExpr { name, .. } => {
                        assert_eq!(name, "Deprecated");
                    }
                    other => panic!("expected AnnotationExpr, got: {:?}", other),
                }
            }
            other => panic!("expected FieldDecl, got: {:?}", other),
        }
    }

    #[test]
    fn constructor() {
        let members = parse_in_class("public Foo(int x) { this.x = x; }");
        assert_eq!(members.len(), 1);
        match &members[0] {
            JavaNode::ConstructorDecl { name, parameters, .. } => {
                assert_eq!(name, "Foo");
                assert_eq!(parameters.len(), 1);
            }
            other => panic!("expected ConstructorDecl, got: {:?}", other),
        }
    }

    #[test]
    fn package_and_imports() {
        let cu = parse("package com.example;\nimport java.util.List;\nimport java.io.*;\nclass A {}");
        assert_eq!(cu.package, Some("com.example".to_string()));
        assert_eq!(cu.imports.len(), 2);
        assert_eq!(cu.imports[0], "java.util.List");
        assert_eq!(cu.imports[1], "java.io.*");
    }

    #[test]
    fn ternary_expression() {
        let stmts = parse_in_method("int x = a > 0 ? a : -a;");
        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            JavaNode::VariableDecl { initializer: Some(init), .. } => {
                assert!(matches!(init.as_ref(), JavaNode::TernaryExpr { .. }));
            }
            other => panic!("expected VariableDecl with ternary, got: {:?}", other),
        }
    }

    #[test]
    fn enum_decl() {
        let cu = parse("enum Color { RED, GREEN, BLUE }");
        match &cu.types[0] {
            JavaNode::EnumDecl { name, constants, .. } => {
                assert_eq!(name, "Color");
                assert_eq!(constants, &["RED", "GREEN", "BLUE"]);
            }
            other => panic!("expected EnumDecl, got: {:?}", other),
        }
    }

    #[test]
    fn interface_with_default_method() {
        let cu = parse("interface Greeter { default String greet() { return \"hello\"; } }");
        match &cu.types[0] {
            JavaNode::InterfaceDecl { name, members, .. } => {
                assert_eq!(name, "Greeter");
                assert_eq!(members.len(), 1);
                assert!(matches!(&members[0], JavaNode::MethodDecl { .. }));
            }
            other => panic!("expected InterfaceDecl, got: {:?}", other),
        }
    }

    #[test]
    fn lambda_expression() {
        let stmts = parse_in_method("Runnable r = () -> System.out.println(\"hi\");");
        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            JavaNode::VariableDecl { initializer: Some(init), .. } => {
                assert!(matches!(init.as_ref(), JavaNode::LambdaExpr { .. }));
            }
            other => panic!("expected VariableDecl with lambda, got: {:?}", other),
        }
    }

    #[test]
    fn try_catch_finally() {
        let stmts = parse_in_method(
            "try { foo(); } catch (IOException | RuntimeException e) { log(e); } finally { close(); }"
        );
        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            JavaNode::TryCatchStmt { catches, finally_block, .. } => {
                assert_eq!(catches.len(), 1);
                match &catches[0] {
                    JavaNode::CatchClause { exception_types, .. } => {
                        assert_eq!(exception_types.len(), 2);
                        assert_eq!(exception_types[0].name, "IOException");
                        assert_eq!(exception_types[1].name, "RuntimeException");
                    }
                    other => panic!("expected CatchClause, got: {:?}", other),
                }
                assert!(finally_block.is_some());
            }
            other => panic!("expected TryCatchStmt, got: {:?}", other),
        }
    }

    #[test]
    fn enhanced_for() {
        let stmts = parse_in_method("for (String s : items) { process(s); }");
        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            JavaNode::ForEachStmt { variable, variable_type, .. } => {
                assert_eq!(variable, "s");
                assert_eq!(variable_type.name, "String");
            }
            other => panic!("expected ForEachStmt, got: {:?}", other),
        }
    }

    // ── Snapshot tests ──────────────────────────────────────────────

    #[test]
    fn snapshot_spring_controller() {
        let cu = parse(r#"
package com.example.web;

import org.springframework.web.bind.annotation.*;
import java.util.List;

@RestController
@RequestMapping("/api/users")
public class UserController {
    @Autowired
    private UserService userService;

    @GetMapping("/{id}")
    public ResponseEntity<User> getUser(@PathVariable Long id) {
        return ResponseEntity.ok(userService.findById(id));
    }

    @PostMapping
    public List<User> createUsers(@RequestBody List<User> users) {
        return userService.saveAll(users);
    }
}
"#);
        insta::assert_yaml_snapshot!(cu);
    }

    #[test]
    fn snapshot_complex_expressions() {
        let cu = parse(r#"
class Expr {
    void test() {
        int a = 1 + 2 * 3;
        boolean b = x > 0 && y < 10;
        String s = flag ? "yes" : "no";
        Object o = (String) obj;
        items.stream().filter(x -> x > 0).count();
    }
}
"#);
        insta::assert_yaml_snapshot!(cu);
    }

    #[test]
    fn snapshot_enum_with_body() {
        let cu = parse(r#"
public enum Planet {
    MERCURY, VENUS, EARTH;

    public double surfaceGravity() {
        return 9.8;
    }
}
"#);
        insta::assert_yaml_snapshot!(cu);
    }

    #[test]
    fn plugin_testdata_spring_web() {
        let src = include_str!("../../../plugins/builtin/spring-web/testdata/input.java");
        parse_java(src, "input.java").expect("spring-web testdata should parse");
    }

    #[test]
    fn plugin_testdata_lombok() {
        let src = include_str!("../../../plugins/builtin/lombok/testdata/input.java");
        parse_java(src, "input.java").expect("lombok testdata should parse");
    }

    #[test]
    fn plugin_testdata_jackson() {
        let src = include_str!("../../../plugins/builtin/jackson/testdata/input.java");
        parse_java(src, "input.java").expect("jackson testdata should parse");
    }

    #[test]
    fn plugin_testdata_guava() {
        let src = include_str!("../../../plugins/builtin/guava/testdata/input.java");
        parse_java(src, "input.java").expect("guava testdata should parse");
    }

    #[test]
    fn plugin_testdata_slf4j() {
        let src = include_str!("../../../plugins/builtin/slf4j/testdata/input.java");
        parse_java(src, "input.java").expect("slf4j testdata should parse");
    }

    #[test]
    fn plugin_testdata_spring_data() {
        let src = include_str!("../../../plugins/builtin/spring-data/testdata/input.java");
        parse_java(src, "input.java").expect("spring-data testdata should parse");
    }

    #[test]
    fn plugin_testdata_spring_tx() {
        let src = include_str!("../../../plugins/builtin/spring-tx/testdata/input.java");
        parse_java(src, "input.java").expect("spring-tx testdata should parse");
    }
}
