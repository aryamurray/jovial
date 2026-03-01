use std::collections::HashMap;

use jovial_ast::go::{GoFile, GoNode};
use jovial_ast::type_resolver::TypeResolver;
use jovial_emitter::emitter::GoEmitter;
use jovial_parser::parse_java;
use jovial_plugin::registry::PluginRegistry;
use jovial_plugin::types::ConfigValue;
use jovial_plugin_java_collections::JavaCollectionsPlugin;
use jovial_walker::walker::Walker;

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

fn transpile(java_source: &str) -> String {
    transpile_with_registry(java_source, PluginRegistry::new())
}

fn transpile_with_registry(java_source: &str, registry: PluginRegistry) -> String {
    // Stage 1: Parse Java → Java AST
    let java_ast = parse_java(java_source, "Test.java").expect("parse failed");

    // Stage 2: Walk Java AST → Go AST nodes
    let resolver = NoopTypeResolver;
    let config: HashMap<String, ConfigValue> = HashMap::new();
    let walker = Walker::new(&registry, &resolver, &config);
    let go_nodes = walker.walk(&java_ast).expect("walk failed");

    // Stage 3: Build GoFile and emit
    let mut package = "main".to_string();
    let mut nodes = Vec::new();
    for node in go_nodes {
        match node {
            GoNode::Package { name } => package = name,
            other => nodes.push(other),
        }
    }

    let go_file = GoFile {
        package,
        imports: vec![],
        nodes,
    };

    let mut emitter = GoEmitter::new();
    emitter.emit_file(&go_file).expect("emit failed")
}

#[test]
fn hello_world_class() {
    let java = r#"
package com.example.app;

public class Greeter {
    private String name;

    public Greeter(String name) {
        this.name = name;
    }

    public String greet() {
        return this.name;
    }
}
"#;

    let go = transpile(java);
    println!("=== Generated Go ===\n{go}");
    assert!(go.contains("package app"));
    assert!(go.contains("type Greeter struct"));
    assert!(go.contains("func NewGreeter("));
    assert!(go.contains("func (g *Greeter) Greet()"));
}

#[test]
fn static_utility() {
    let java = r#"
public class MathUtils {
    public static int add(int a, int b) {
        return a + b;
    }
}
"#;

    let go = transpile(java);
    println!("=== Generated Go ===\n{go}");
    assert!(go.contains("func Add("));
    assert!(go.contains("return"));
}

#[test]
fn interface_conversion() {
    let java = r#"
public interface Repository {
    Object findById(long id);
    void save(Object entity);
}
"#;

    let go = transpile(java);
    println!("=== Generated Go ===\n{go}");
    assert!(go.contains("type Repository interface"));
    assert!(go.contains("FindById("));
    assert!(go.contains("Save("));
}

#[test]
fn collections_plugin_hashmap() {
    let java = r#"
package com.example.svc;

public class Registry {
    private Map<String, Integer> counts;

    public Registry() {
        this.counts = new HashMap<String, Integer>();
    }

    public Map<String, Integer> getCounts() {
        return this.counts;
    }
}
"#;

    let mut registry = PluginRegistry::new();
    registry.register(Box::new(JavaCollectionsPlugin));

    let go = transpile_with_registry(java, registry);
    println!("=== Java Input ===\n{java}");
    println!("=== Generated Go (with java-collections plugin) ===\n{go}");

    // Plugin should convert `new HashMap<String, Integer>()` → map[string]int
    assert!(go.contains("map[string]int"), "expected map[string]int from plugin, got:\n{go}");
    assert!(go.contains("type Registry struct"));
    assert!(go.contains("func NewRegistry("));
    assert!(go.contains("func (r *Registry) GetCounts()"));
}

#[test]
fn collections_plugin_arraylist() {
    let java = r#"
public class TaskQueue {
    private List<String> items;

    public TaskQueue() {
        this.items = new ArrayList<String>();
    }

    public List<String> getItems() {
        return this.items;
    }
}
"#;

    let mut registry = PluginRegistry::new();
    registry.register(Box::new(JavaCollectionsPlugin));

    let go = transpile_with_registry(java, registry);
    println!("=== Java Input ===\n{java}");
    println!("=== Generated Go (with java-collections plugin) ===\n{go}");

    // Plugin should convert `new ArrayList<String>()` → `[]string{}`
    assert!(go.contains("[]string{}"), "expected []string{{}} from plugin, got:\n{go}");
    assert!(go.contains("type TaskQueue struct"));
    assert!(go.contains("func NewTaskQueue("));
}
