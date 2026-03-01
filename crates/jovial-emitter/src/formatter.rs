/// Simple code formatter with indentation tracking.
pub struct Formatter {
    indent_str: String,
    indent_level: usize,
    lines: Vec<String>,
    current_line: String,
}

impl Formatter {
    pub fn new() -> Self {
        Self {
            indent_str: "\t".to_string(),
            indent_level: 0,
            lines: Vec::new(),
            current_line: String::new(),
        }
    }

    /// Create a formatter with a custom indent string.
    pub fn with_indent(indent: impl Into<String>) -> Self {
        Self {
            indent_str: indent.into(),
            indent_level: 0,
            lines: Vec::new(),
            current_line: String::new(),
        }
    }

    /// Increase indentation level.
    pub fn indent(&mut self) {
        self.indent_level += 1;
    }

    /// Decrease indentation level.
    pub fn dedent(&mut self) {
        if self.indent_level > 0 {
            self.indent_level -= 1;
        }
    }

    /// Write a complete line with current indentation.
    pub fn write_line(&mut self, line: &str) {
        if line.is_empty() {
            self.lines.push(String::new());
        } else {
            let indent = self.indent_str.repeat(self.indent_level);
            self.lines.push(format!("{indent}{line}"));
        }
    }

    /// Write text without a newline (appends to current partial line).
    pub fn write(&mut self, text: &str) {
        if self.current_line.is_empty() {
            self.current_line = self.indent_str.repeat(self.indent_level);
        }
        self.current_line.push_str(text);
    }

    /// Flush the current partial line.
    pub fn flush_line(&mut self) {
        if !self.current_line.is_empty() {
            self.lines.push(std::mem::take(&mut self.current_line));
        }
    }

    /// Write a blank line.
    pub fn blank_line(&mut self) {
        self.lines.push(String::new());
    }

    /// Get the formatted output as a single string.
    pub fn output(&self) -> String {
        self.lines.join("\n")
    }

    /// Get the current indentation level.
    pub fn indent_level(&self) -> usize {
        self.indent_level
    }
}

impl Default for Formatter {
    fn default() -> Self {
        Self::new()
    }
}
