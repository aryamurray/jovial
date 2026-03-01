#![allow(unused_assignments)]

use miette::{Diagnostic, NamedSource, SourceSpan};
use std::fmt;

/// A parse error with rich source-annotated diagnostics.
#[derive(Debug, Diagnostic)]
#[diagnostic(code(jovial::parse_error))]
pub struct ParseError {
    #[source_code]
    pub src: NamedSource<String>,

    #[label("{message}")]
    pub span: SourceSpan,

    pub message: String,
}

impl ParseError {
    pub fn new(
        source: &str,
        filename: &str,
        byte_offset: usize,
        length: usize,
        message: impl Into<String>,
    ) -> Self {
        Self {
            src: NamedSource::new(filename, source.to_string()),
            span: SourceSpan::new(byte_offset.into(), length),
            message: message.into(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ParseError {}
