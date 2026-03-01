use jovial_manifest::beans::Bean;

/// Generates Go service structs from bean metadata.
pub struct ServiceGenerator {
    module_path: String,
}

impl ServiceGenerator {
    pub fn new(module_path: impl Into<String>) -> Self {
        Self {
            module_path: module_path.into(),
        }
    }

    /// Generate a Go service struct for a Spring bean.
    pub fn generate_service(&self, _bean: &Bean) -> Result<String, std::io::Error> {
        todo!()
    }
}
