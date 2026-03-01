/// Generates go.mod file content.
///
/// Takes `&[(String, String)]` (module, version) tuples instead of `GoDependency`
/// to keep jovial-emitter free from jovial-plugin dependency.
pub struct GoModGenerator {
    module_path: String,
    go_version: String,
}

impl GoModGenerator {
    pub fn new(module_path: impl Into<String>, go_version: impl Into<String>) -> Self {
        Self {
            module_path: module_path.into(),
            go_version: go_version.into(),
        }
    }

    /// Generate go.mod content from a list of (module, version) dependencies.
    pub fn generate(&self, _deps: &[(String, String)]) -> String {
        todo!()
    }
}
