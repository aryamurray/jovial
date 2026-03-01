use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

/// Jovial project configuration (jovial.yaml).
#[derive(Debug, Deserialize)]
pub struct JovialConfig {
    /// Path to the Java project root.
    pub java_project: String,

    /// Output directory for generated Go code.
    #[serde(default = "default_output_dir")]
    pub output_dir: String,

    /// Go module path for the generated project.
    pub go_module: Option<String>,

    /// List of plugins to enable.
    #[serde(default)]
    pub plugins: Vec<String>,

    /// Plugin-specific configuration.
    #[serde(default)]
    pub plugin_config: HashMap<String, HashMap<String, String>>,

    /// Path to pre-extracted manifest file.
    pub manifest: Option<String>,
}

fn default_output_dir() -> String {
    "./generated".to_string()
}

impl JovialConfig {
    /// Load configuration from a YAML file.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: JovialConfig = serde_yaml::from_str(&content)?;
        Ok(config)
    }
}
