use serde::{Deserialize, Serialize};

/// Source location tracking for AST nodes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub file: String,
    pub line_start: usize,
    pub col_start: usize,
    pub line_end: usize,
    pub col_end: usize,
}

impl Span {
    pub fn new(
        file: impl Into<String>,
        line_start: usize,
        col_start: usize,
        line_end: usize,
        col_end: usize,
    ) -> Self {
        Self {
            file: file.into(),
            line_start,
            col_start,
            line_end,
            col_end,
        }
    }

    pub fn dummy() -> Self {
        Self {
            file: String::new(),
            line_start: 0,
            col_start: 0,
            line_end: 0,
            col_end: 0,
        }
    }
}
