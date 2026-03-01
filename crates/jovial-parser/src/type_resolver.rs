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

    fn is_assignable_to(&self, from: &str, to: &str) -> bool {
        if from == to {
            return true;
        }
        // Walk superclass chain
        if let Some(super_class) = self.superclasses.get(from) {
            if self.is_assignable_to(super_class, to) {
                return true;
            }
        }
        // Check interfaces
        if let Some(ifaces) = self.interfaces.get(from) {
            if ifaces.iter().any(|i| i == to) {
                return true;
            }
        }
        false
    }

    fn superclass_of(&self, fqcn: &str) -> Option<String> {
        self.superclasses.get(fqcn).cloned()
    }

    fn interfaces_of(&self, fqcn: &str) -> Vec<String> {
        self.interfaces.get(fqcn).cloned().unwrap_or_default()
    }
}
