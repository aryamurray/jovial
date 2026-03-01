//! Minimal plugin example — the simplest possible Jovial plugin.

use jovial_plugin::prelude::*;

pub struct MinimalPlugin;

impl Plugin for MinimalPlugin {
    fn name(&self) -> &str {
        "minimal-example"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn description(&self) -> &str {
        "A minimal example plugin that does nothing"
    }

    fn matches(&self, _ctx: &MatchContext) -> bool {
        false
    }

    fn transform(&self, _ctx: &mut TransformContext) -> Result<Vec<GoNode>, PluginError> {
        Ok(vec![])
    }
}
