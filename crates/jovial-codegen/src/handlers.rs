use jovial_manifest::endpoints::Endpoint;

/// Generates HTTP handler functions from endpoint metadata.
pub struct HandlerGenerator {
    module_path: String,
}

impl HandlerGenerator {
    pub fn new(module_path: impl Into<String>) -> Self {
        Self {
            module_path: module_path.into(),
        }
    }

    /// Generate a handler function for a single endpoint.
    pub fn generate_handler(&self, _endpoint: &Endpoint) -> Result<String, std::io::Error> {
        todo!()
    }

    /// Generate the router setup function that registers all routes.
    pub fn generate_router_setup(
        &self,
        _endpoints: &[Endpoint],
    ) -> Result<String, std::io::Error> {
        todo!()
    }
}
