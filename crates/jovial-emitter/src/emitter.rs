use thiserror::Error;

use jovial_ast::go::{GoFile, GoNode};

/// Errors during Go code emission.
#[derive(Debug, Error)]
pub enum EmitError {
    #[error("emit failed for node: {0}")]
    EmitFailed(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Emits Go source code from Go AST nodes.
pub struct GoEmitter {
    indent_level: usize,
    output: String,
}

impl GoEmitter {
    pub fn new() -> Self {
        Self {
            indent_level: 0,
            output: String::new(),
        }
    }

    /// Emit a complete Go source file.
    pub fn emit_file(&mut self, _file: &GoFile) -> Result<String, EmitError> {
        todo!()
    }

    /// Emit a single Go AST node.
    pub fn emit_node(&mut self, _node: &GoNode) -> Result<(), EmitError> {
        todo!()
    }
}

impl Default for GoEmitter {
    fn default() -> Self {
        Self::new()
    }
}
