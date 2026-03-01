use jovial_manifest::entities::Entity;

/// Generates Go model structs from JPA entity metadata.
pub struct ModelGenerator {
    module_path: String,
}

impl ModelGenerator {
    pub fn new(module_path: impl Into<String>) -> Self {
        Self {
            module_path: module_path.into(),
        }
    }

    /// Generate a Go struct (GORM model) for a JPA entity.
    pub fn generate_model(&self, _entity: &Entity) -> Result<String, std::io::Error> {
        todo!()
    }
}
