/// Manages Go import blocks with stdlib/external grouping.
pub struct ImportBlock {
    stdlib: Vec<String>,
    external: Vec<String>,
}

impl ImportBlock {
    pub fn new() -> Self {
        Self {
            stdlib: Vec::new(),
            external: Vec::new(),
        }
    }

    /// Add an import path. Automatically categorizes as stdlib or external.
    pub fn add(&mut self, path: impl Into<String>) {
        let path = path.into();
        if Self::is_stdlib(&path) {
            if !self.stdlib.contains(&path) {
                self.stdlib.push(path);
            }
        } else if !self.external.contains(&path) {
            self.external.push(path);
        }
    }

    /// Render the import block as Go source.
    pub fn render(&self) -> String {
        todo!()
    }

    /// Check if an import path is from the Go standard library.
    fn is_stdlib(path: &str) -> bool {
        // Stdlib packages don't contain dots in their path
        !path.contains('.')
    }

    /// Whether the import block is empty.
    pub fn is_empty(&self) -> bool {
        self.stdlib.is_empty() && self.external.is_empty()
    }
}

impl Default for ImportBlock {
    fn default() -> Self {
        Self::new()
    }
}
