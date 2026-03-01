use std::path::{Path, PathBuf};

use thiserror::Error;

use jovial_emitter::go_mod::GoModGenerator;
use jovial_manifest::Manifest;

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
    pub fn scaffold(&self, manifest: &Manifest) -> Result<(), ProjectError> {
        let root = PathBuf::from(&self.output_dir);

        // Create standard directory tree
        let dirs = [
            "handlers",
            "models",
            "services",
            "repositories",
            "internal/wire",
            "internal/config",
            "pkg",
        ];
        for dir in &dirs {
            let path = root.join(dir);
            std::fs::create_dir_all(&path).map_err(|e| {
                ProjectError::CreateDir(format!("{}: {}", path.display(), e))
            })?;
        }

        // Generate go.mod
        let deps = compute_deps(manifest);
        let go_mod_gen = GoModGenerator::new(&self.module_path, "1.21");
        let go_mod_content = go_mod_gen.generate(&deps);
        write_file(&root.join("go.mod"), &go_mod_content)?;

        // Generate main.go
        let main_go = generate_main_go(&self.module_path);
        write_file(&root.join("main.go"), &main_go)?;

        log::info!("scaffolded project at {}", root.display());
        Ok(())
    }

    /// Write emitted Go source files to the output directory.
    pub fn write_emitted_files(
        &self,
        files: &[(PathBuf, String)],
    ) -> Result<(), ProjectError> {
        let root = PathBuf::from(&self.output_dir);

        for (relative_path, content) in files {
            let dest = root.join(relative_path);
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    ProjectError::CreateDir(format!("{}: {}", parent.display(), e))
                })?;
            }
            write_file(&dest, content)?;
        }

        Ok(())
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

/// Compute go.mod dependencies based on manifest contents.
fn compute_deps(manifest: &Manifest) -> Vec<(String, String)> {
    let mut deps = Vec::new();

    if !manifest.endpoints.is_empty() {
        deps.push((
            "github.com/gin-gonic/gin".to_string(),
            "v1.9.1".to_string(),
        ));
    }

    if !manifest.entities.is_empty() {
        deps.push((
            "gorm.io/gorm".to_string(),
            "v1.25.7".to_string(),
        ));
        deps.push((
            "gorm.io/driver/postgres".to_string(),
            "v1.5.6".to_string(),
        ));
    }

    deps
}

/// Generate main.go content.
fn generate_main_go(module_path: &str) -> String {
    format!(
        r#"package main

import (
	"{module_path}/internal/wire"
)

func main() {{
	app, err := wire.InitializeApp()
	if err != nil {{
		panic(err)
	}}

	if err := app.Router.Run(":8080"); err != nil {{
		panic(err)
	}}
}}
"#
    )
}

/// Write a file, reporting errors as ProjectError.
fn write_file(path: &Path, content: &str) -> Result<(), ProjectError> {
    std::fs::write(path, content)
        .map_err(|e| ProjectError::WriteFile(format!("{}: {}", path.display(), e)))
}
