use serde::{Deserialize, Serialize};

/// A Go import to be added to the output file.
///
/// Named `PluginGoImport` to avoid conflict with `jovial_ast::go::GoImport`
/// which is re-exported in the prelude.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginGoImport {
    pub path: String,
    pub alias: Option<String>,
}

/// A Go module dependency to be added to go.mod.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoDependency {
    pub module: String,
    pub version: String,
}

/// A plugin configuration value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConfigValue {
    String(String),
    Bool(bool),
    Int(i64),
    Float(f64),
    List(Vec<ConfigValue>),
}
