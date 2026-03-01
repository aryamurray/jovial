use std::collections::HashMap;

use jovial_ast::go::{GoFile, GoNode};
use jovial_ast::type_resolver::TypeResolver;
use jovial_emitter::emitter::GoEmitter;
use jovial_parser::parse_java;
use jovial_plugin::registry::PluginRegistry;
use jovial_plugin::types::ConfigValue;
use jovial_plugin_java_collections::JavaCollectionsPlugin;
use jovial_plugin_java_io::JavaIoPlugin;
use jovial_plugin_java_strings::JavaStringsPlugin;
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

#[test]
fn io_plugin_system_out_println() {
    let java = r#"
public class App {
    public static void main(String[] args) {
        System.out.println("Hello, World!");
    }
}
"#;

    let mut registry = PluginRegistry::new();
    registry.register(Box::new(JavaIoPlugin));

    let go = transpile_with_registry(java, registry);
    println!("=== Generated Go (java-io) ===\n{go}");

    assert!(
        go.contains("fmt.Println("),
        "expected fmt.Println from plugin, got:\n{go}"
    );
}

#[test]
fn io_plugin_system_err_println() {
    let java = r#"
public class App {
    public static void main(String[] args) {
        System.err.println("error!");
    }
}
"#;

    let mut registry = PluginRegistry::new();
    registry.register(Box::new(JavaIoPlugin));

    let go = transpile_with_registry(java, registry);
    println!("=== Generated Go (java-io stderr) ===\n{go}");

    assert!(
        go.contains("fmt.Fprintln(os.Stderr,"),
        "expected fmt.Fprintln(os.Stderr, ...) from plugin, got:\n{go}"
    );
}

#[test]
fn io_plugin_system_exit() {
    let java = r#"
public class App {
    public static void main(String[] args) {
        System.exit(1);
    }
}
"#;

    let mut registry = PluginRegistry::new();
    registry.register(Box::new(JavaIoPlugin));

    let go = transpile_with_registry(java, registry);
    println!("=== Generated Go (java-io exit) ===\n{go}");

    assert!(
        go.contains("os.Exit("),
        "expected os.Exit from plugin, got:\n{go}"
    );
}

#[test]
fn strings_plugin_equals() {
    let java = r#"
public class Checker {
    public static boolean check(String a, String b) {
        return a.equals(b);
    }
}
"#;

    let mut registry = PluginRegistry::new();
    registry.register(Box::new(JavaStringsPlugin));

    let go = transpile_with_registry(java, registry);
    println!("=== Generated Go (java-strings equals) ===\n{go}");

    // a.equals(b) should become a == b, not a.Equals(b)
    assert!(
        go.contains("=="),
        "expected == operator from equals(), got:\n{go}"
    );
    assert!(
        !go.contains("Equals("),
        "should NOT have Equals() method call, got:\n{go}"
    );
}

#[test]
fn strings_plugin_contains_and_length() {
    let java = r#"
public class StringUtils {
    public static boolean hasContent(String s, String sub) {
        return s.contains(sub);
    }

    public static int getLen(String s) {
        return s.length();
    }
}
"#;

    let mut registry = PluginRegistry::new();
    registry.register(Box::new(JavaStringsPlugin));

    let go = transpile_with_registry(java, registry);
    println!("=== Generated Go (java-strings contains+length) ===\n{go}");

    assert!(
        go.contains("strings.Contains("),
        "expected strings.Contains from plugin, got:\n{go}"
    );
    assert!(
        go.contains("len("),
        "expected len() from length(), got:\n{go}"
    );
}

#[test]
fn strings_plugin_starts_ends_with() {
    let java = r#"
public class PathUtils {
    public static boolean isAbsolute(String path) {
        return path.startsWith("/");
    }

    public static boolean isJava(String name) {
        return name.endsWith(".java");
    }
}
"#;

    let mut registry = PluginRegistry::new();
    registry.register(Box::new(JavaStringsPlugin));

    let go = transpile_with_registry(java, registry);
    println!("=== Generated Go (java-strings prefix/suffix) ===\n{go}");

    assert!(
        go.contains("strings.HasPrefix("),
        "expected strings.HasPrefix, got:\n{go}"
    );
    assert!(
        go.contains("strings.HasSuffix("),
        "expected strings.HasSuffix, got:\n{go}"
    );
}

#[test]
fn collections_plugin_list_add_and_size() {
    let java = r#"
public class Collector {
    public static int collect(String item) {
        List<String> items = new ArrayList<String>();
        items.add(item);
        return items.size();
    }
}
"#;

    let mut registry = PluginRegistry::new();
    registry.register(Box::new(JavaCollectionsPlugin));

    let go = transpile_with_registry(java, registry);
    println!("=== Generated Go (collections add+size) ===\n{go}");

    assert!(
        go.contains("append("),
        "expected append() from list.add(), got:\n{go}"
    );
    assert!(
        go.contains("len("),
        "expected len() from list.size(), got:\n{go}"
    );
}

#[test]
fn collections_plugin_map_put() {
    let java = r#"
public class Config {
    public static void setup() {
        Map<String, Integer> m = new HashMap<String, Integer>();
        m.put("port", 8080);
    }
}
"#;

    let mut registry = PluginRegistry::new();
    registry.register(Box::new(JavaCollectionsPlugin));

    let go = transpile_with_registry(java, registry);
    println!("=== Generated Go (collections map.put) ===\n{go}");

    // map.put(k, v) should become m["port"] = 8080
    assert!(
        go.contains("[\"port\"] = 8080") || go.contains("m[\"port\"] = 8080"),
        "expected index assignment from map.put(), got:\n{go}"
    );
}
