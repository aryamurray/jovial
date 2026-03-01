use std::path::Path;

use thiserror::Error;

/// Errors during project generation.
#[derive(Debug, Error)]
pub enum ProjectError {
    #[error("failed to create directory: {0}")]
    CreateDir(String),

    #[error("failed to write file: {0}")]
    WriteFile(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Generates the Go project directory structure.
pub struct ProjectGenerator {
    output_dir: String,
    module_path: String,
}

impl ProjectGenerator {
    pub fn new(output_dir: impl Into<String>, module_path: impl Into<String>) -> Self {
        Self {
            output_dir: output_dir.into(),
            module_path: module_path.into(),
        }
    }

    /// Scaffold the output Go project directory structure.
    pub fn scaffold(&self, _manifest: &jovial_manifest::Manifest) -> Result<(), ProjectError> {
        todo!()
    }

    /// Get the output directory path.
    pub fn output_dir(&self) -> &Path {
        Path::new(&self.output_dir)
    }

    /// Get the Go module path.
    pub fn module_path(&self) -> &str {
        &self.module_path
    }
}
