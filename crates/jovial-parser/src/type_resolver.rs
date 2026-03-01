use std::collections::HashMap;

use jovial_ast::type_resolver::TypeResolver;

/// Default implementation of TypeResolver that uses import information
/// and a class hierarchy to resolve types.
pub struct DefaultTypeResolver {
    imports: HashMap<String, String>,
    superclasses: HashMap<String, String>,
    interfaces: HashMap<String, Vec<String>>,
}

impl DefaultTypeResolver {
    pub fn new() -> Self {
        Self {
            imports: HashMap::new(),
            superclasses: HashMap::new(),
            interfaces: HashMap::new(),
        }
    }

    /// Register a type mapping (simple name → FQCN).
    pub fn add_type(&mut self, simple_name: String, fqcn: String) {
        self.imports.insert(simple_name, fqcn);
    }

    /// Register a superclass relationship.
    pub fn add_superclass(&mut self, fqcn: String, superclass: String) {
        self.superclasses.insert(fqcn, superclass);
    }

    /// Register interfaces for a type.
    pub fn add_interfaces(&mut self, fqcn: String, ifaces: Vec<String>) {
        self.interfaces.insert(fqcn, ifaces);
    }
}

impl Default for DefaultTypeResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeResolver for DefaultTypeResolver {
    fn resolve(&self, simple_name: &str) -> Option<String> {
        self.imports.get(simple_name).cloned()
    }

    fn is_assignable_to(&self, _from: &str, _to: &str) -> bool {
        todo!()
    }

    fn superclass_of(&self, fqcn: &str) -> Option<String> {
        self.superclasses.get(fqcn).cloned()
    }

    fn interfaces_of(&self, fqcn: &str) -> Vec<String> {
        self.interfaces.get(fqcn).cloned().unwrap_or_default()
    }
}
