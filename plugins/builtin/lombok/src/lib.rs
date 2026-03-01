use jovial_plugin::prelude::*;

pub struct LombokPlugin;

impl Plugin for LombokPlugin {
    fn name(&self) -> &str {
        "lombok"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn priority(&self) -> i32 {
        5
    }

    fn description(&self) -> &str {
        "Strips Lombok annotations, generates plain Go structs"
    }

    fn matches(&self, _ctx: &MatchContext) -> bool {
        todo!()
    }

    fn transform(&self, _ctx: &mut TransformContext) -> Result<Vec<GoNode>, PluginError> {
        todo!()
    }
}
