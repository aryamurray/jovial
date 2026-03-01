use jovial_plugin::prelude::*;

pub struct SpringTxPlugin;

impl Plugin for SpringTxPlugin {
    fn name(&self) -> &str {
        "spring-tx"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn priority(&self) -> i32 {
        30
    }

    fn description(&self) -> &str {
        "Transforms @Transactional into inline transaction management"
    }

    fn matches(&self, _ctx: &MatchContext) -> bool {
        todo!()
    }

    fn transform(&self, _ctx: &mut TransformContext) -> Result<Vec<GoNode>, PluginError> {
        todo!()
    }
}
