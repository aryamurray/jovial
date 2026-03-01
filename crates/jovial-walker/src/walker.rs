use thiserror::Error;

use jovial_ast::go::GoNode;
use jovial_ast::java::JavaCompilationUnit;
use jovial_plugin::registry::PluginRegistry;

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
}

impl<'a> Walker<'a> {
    pub fn new(registry: &'a PluginRegistry) -> Self {
        Self {
            registry,
            default_converter: DefaultConverter::new(),
        }
    }

    /// Walk an entire compilation unit, producing Go AST nodes.
    pub fn walk(&self, _unit: &JavaCompilationUnit) -> Result<Vec<GoNode>, WalkError> {
        todo!()
    }

    /// Walk a single Java node.
    pub fn walk_node(
        &self,
        _node: &jovial_ast::java::JavaNode,
    ) -> Result<Vec<GoNode>, WalkError> {
        todo!()
    }
}
