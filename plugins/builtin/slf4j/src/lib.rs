use jovial_plugin::prelude::*;

pub struct Slf4jPlugin;

impl Plugin for Slf4jPlugin {
    fn name(&self) -> &str {
        "slf4j"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn priority(&self) -> i32 {
        60
    }

    fn description(&self) -> &str {
        "Transforms SLF4J logging to Go log/slog"
    }

    fn matches(&self, _ctx: &MatchContext) -> bool {
        todo!()
    }

    fn transform(&self, _ctx: &mut TransformContext) -> Result<Vec<GoNode>, PluginError> {
        todo!()
    }
}
