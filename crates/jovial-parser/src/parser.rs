use thiserror::Error;

use jovial_ast::java::JavaCompilationUnit;

use crate::lexer::Token;

/// Parser error.
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("unexpected token {found:?}, expected {expected}")]
    UnexpectedToken { found: Token, expected: String },

    #[error("unexpected end of input")]
    UnexpectedEof,

    #[error("parse error: {0}")]
    Other(String),
}

/// Recursive-descent Java parser.
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    /// Parse a complete Java source file.
    pub fn parse_file(&mut self) -> Result<JavaCompilationUnit, ParseError> {
        self.parse_compilation_unit()
    }

    /// Parse a compilation unit (package + imports + type declarations).
    pub fn parse_compilation_unit(&mut self) -> Result<JavaCompilationUnit, ParseError> {
        todo!()
    }
}
