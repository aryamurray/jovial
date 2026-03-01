use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Jovial project configuration (jovial.yaml).
#[derive(Debug, Deserialize)]
pub struct JovialConfig {
    /// Source configuration.
    #[serde(default)]
    pub source: SourceConfig,

    /// Classpath configuration for the JVM extractor.
    #[serde(default)]
    pub classpath: ClasspathConfig,

    /// JVM extractor configuration.
    #[serde(default)]
    pub extractor: ExtractorConfig,

    /// Target (Go) project configuration.
    pub target: TargetConfig,

    /// Plugin references.
    #[serde(default)]
    pub plugins: Vec<PluginRef>,

    /// Global options.
    #[serde(default)]
    pub options: GlobalOptions,

    /// Plugin-specific configuration values.
    #[serde(default)]
    pub plugin_config: HashMap<String, HashMap<String, String>>,
}

/// Where to find Java source files.
#[derive(Debug, Deserialize)]
pub struct SourceConfig {
    /// Source root directories relative to project root.
    #[serde(default = "default_source_roots")]
    pub roots: Vec<String>,

    /// Main class FQCN (for entry-point detection).
    pub main_class: Option<String>,

    /// Spring profile to activate during extraction.
    pub profile: Option<String>,

    /// Java version of the source project.
    #[serde(default = "default_java_version")]
    pub java_version: String,
}

impl Default for SourceConfig {
    fn default() -> Self {
        Self {
            roots: default_source_roots(),
            main_class: None,
            profile: None,
            java_version: default_java_version(),
        }
    }
}

fn default_source_roots() -> Vec<String> {
    vec!["src/main/java".to_string()]
}

fn default_java_version() -> String {
    "17".to_string()
}

/// Classpath entries for the JVM extractor.
#[derive(Debug, Default, Deserialize)]
pub struct ClasspathConfig {
    /// Directory containing dependency JARs.
    pub lib_dir: Option<String>,

    /// Directory containing compiled .class files.
    pub classes_dir: Option<String>,

    /// Additional classpath entries.
    #[serde(default)]
    pub extra: Vec<String>,
}

/// JVM extractor settings.
#[derive(Debug, Deserialize)]
pub struct ExtractorConfig {
    /// Skip extraction entirely (use manifest_path or empty manifest).
    #[serde(default)]
    pub skip: bool,

    /// Path to a pre-generated manifest JSON file.
    pub manifest_path: Option<String>,

    /// Extra JVM options for the extractor process.
    #[serde(default)]
    pub jvm_opts: Vec<String>,

    /// Timeout in seconds for the extractor process.
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

impl Default for ExtractorConfig {
    fn default() -> Self {
        Self {
            skip: false,
            manifest_path: None,
            jvm_opts: Vec::new(),
            timeout: default_timeout(),
        }
    }
}

fn default_timeout() -> u64 {
    120
}

/// Go output project configuration.
#[derive(Debug, Deserialize)]
pub struct TargetConfig {
    /// Go module path (e.g. "github.com/example/myapp").
    pub module: String,

    /// Go version for go.mod.
    #[serde(default = "default_go_version")]
    pub go_version: String,

    /// Output directory for generated Go code.
    #[serde(default = "default_output_dir")]
    pub output_dir: String,
}

fn default_go_version() -> String {
    "1.21".to_string()
}

fn default_output_dir() -> String {
    "./generated".to_string()
}

/// A reference to a plugin (builtin or external).
#[derive(Debug, Clone, Deserialize)]
pub struct PluginRef {
    /// Plugin name (must match a builtin or registry name).
    pub name: String,

    /// Whether the plugin is enabled (default true).
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Path to a local plugin (not yet supported).
    pub path: Option<String>,

    /// Plugin-specific options.
    #[serde(default)]
    pub options: HashMap<String, String>,
}

fn default_true() -> bool {
    true
}

/// Global transpilation options.
#[derive(Debug, Deserialize)]
pub struct GlobalOptions {
    /// How to handle errors: "continue" or "fail".
    #[serde(default = "default_error_handling")]
    pub error_handling: String,

    /// Generate test stub files.
    #[serde(default)]
    pub test_stubs: bool,

    /// Verbose logging.
    #[serde(default)]
    pub verbose: bool,
}

impl Default for GlobalOptions {
    fn default() -> Self {
        Self {
            error_handling: default_error_handling(),
            test_stubs: false,
            verbose: false,
        }
    }
}

fn default_error_handling() -> String {
    "continue".to_string()
}

impl JovialConfig {
    /// Load configuration from a specific YAML file path.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: JovialConfig = serde_yaml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    /// Search for jovial.yaml in the current directory and parent directories.
    pub fn load_or_default() -> anyhow::Result<Self> {
        let mut dir = std::env::current_dir()?;
        loop {
            let candidate = dir.join("jovial.yaml");
            if candidate.exists() {
                return Self::load(&candidate);
            }
            let candidate = dir.join("jovial.yml");
            if candidate.exists() {
                return Self::load(&candidate);
            }
            if !dir.pop() {
                break;
            }
        }
        anyhow::bail!(
            "no jovial.yaml found in current directory or any parent directory"
        )
    }

    /// Validate the configuration.
    fn validate(&self) -> anyhow::Result<()> {
        if self.source.roots.is_empty() {
            anyhow::bail!("source.roots must not be empty");
        }
        if self.target.module.is_empty() {
            anyhow::bail!("target.module must not be empty");
        }
        if let Some(ref classes_dir) = self.classpath.classes_dir {
            let p = PathBuf::from(classes_dir);
            if !p.exists() {
                log::warn!("classpath.classes_dir does not exist: {}", classes_dir);
            }
        }
        Ok(())
    }
}
