use std::collections::HashMap;

/// Resolves simple class names to fully-qualified class names using import statements.
pub struct ImportResolver {
    imports: HashMap<String, String>,
    star_imports: Vec<String>,
}

impl ImportResolver {
    pub fn new() -> Self {
        Self {
            imports: HashMap::new(),
            star_imports: Vec::new(),
        }
    }

    /// Add an import statement.
    pub fn add_import(&mut self, import: &str) {
        if import.ends_with(".*") {
            self.star_imports
                .push(import.trim_end_matches(".*").to_string());
        } else if let Some(simple_name) = import.rsplit('.').next() {
            self.imports
                .insert(simple_name.to_string(), import.to_string());
        }
    }

    /// Resolve a simple name to its fully-qualified class name.
    pub fn resolve_fqcn(&self, simple_name: &str) -> Option<String> {
        self.imports.get(simple_name).cloned()
    }

    /// Get all star-import packages.
    pub fn star_imports(&self) -> &[String] {
        &self.star_imports
    }
}

impl Default for ImportResolver {
    fn default() -> Self {
        Self::new()
    }
}
