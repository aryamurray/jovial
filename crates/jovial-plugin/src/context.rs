use std::collections::HashMap;

use jovial_ast::java::JavaNode;
use jovial_ast::type_resolver::TypeResolver;

use crate::error::Diagnostic;
use crate::types::{ConfigValue, GoDependency, PluginGoImport};

/// Context provided to `Plugin::matches()` for deciding whether to handle a node.
pub struct MatchContext<'a> {
    pub node: &'a JavaNode,
    pub type_resolver: &'a dyn TypeResolver,
    pub config: &'a HashMap<String, ConfigValue>,
}

/// Context provided to `Plugin::transform()` with mutable output collectors.
pub struct TransformContext<'a> {
    pub node: &'a JavaNode,
    pub type_resolver: &'a dyn TypeResolver,
    pub config: &'a HashMap<String, ConfigValue>,
    imports: Vec<PluginGoImport>,
    dependencies: Vec<GoDependency>,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> TransformContext<'a> {
    pub fn new(
        node: &'a JavaNode,
        type_resolver: &'a dyn TypeResolver,
        config: &'a HashMap<String, ConfigValue>,
    ) -> Self {
        Self {
            node,
            type_resolver,
            config,
            imports: Vec::new(),
            dependencies: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    /// Add a Go import to the output file.
    pub fn add_import(&mut self, path: impl Into<String>, alias: Option<String>) {
        self.imports.push(PluginGoImport {
            path: path.into(),
            alias,
        });
    }

    /// Add a Go module dependency (for go.mod).
    pub fn add_dependency(&mut self, module: impl Into<String>, version: impl Into<String>) {
        self.dependencies.push(GoDependency {
            module: module.into(),
            version: version.into(),
        });
    }

    /// Emit a diagnostic message.
    pub fn add_diagnostic(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    /// Consume the context and return collected imports.
    pub fn into_imports(self) -> Vec<PluginGoImport> {
        self.imports
    }

    /// Get collected dependencies.
    pub fn dependencies(&self) -> &[GoDependency] {
        &self.dependencies
    }

    /// Get collected diagnostics.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}
