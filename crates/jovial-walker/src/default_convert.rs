use jovial_ast::go::GoNode;
use jovial_ast::java::JavaNode;

/// Default mechanical converter for Java nodes that no plugin claims.
pub struct DefaultConverter;

impl DefaultConverter {
    pub fn new() -> Self {
        Self
    }

    /// Convert a Java AST node to Go AST node(s) using basic mechanical translation.
    pub fn convert(&self, _node: &JavaNode) -> Vec<GoNode> {
        todo!()
    }
}

impl Default for DefaultConverter {
    fn default() -> Self {
        Self::new()
    }
}
