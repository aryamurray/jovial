use crate::traits::Plugin;

/// Priority-ordered registry of plugins.
pub struct PluginRegistry {
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    /// Register a plugin, maintaining priority order (lower priority value = earlier in list).
    pub fn register(&mut self, plugin: Box<dyn Plugin>) {
        let priority = plugin.priority();
        let pos = self
            .plugins
            .iter()
            .position(|p| p.priority() > priority)
            .unwrap_or(self.plugins.len());
        self.plugins.insert(pos, plugin);
    }

    /// Get all registered plugins in priority order.
    pub fn plugins(&self) -> &[Box<dyn Plugin>] {
        &self.plugins
    }

    /// Find the first plugin that matches the given context.
    pub fn find_match(&self, ctx: &crate::context::MatchContext) -> Option<&dyn Plugin> {
        self.plugins.iter().find(|p| p.matches(ctx)).map(|p| p.as_ref())
    }

    /// Number of registered plugins.
    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
