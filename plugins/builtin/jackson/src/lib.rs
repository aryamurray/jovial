#![allow(dead_code)]

mod annotations;
mod object_mapper;

use jovial_plugin::prelude::*;

pub struct JacksonPlugin;

impl Plugin for JacksonPlugin {
    fn name(&self) -> &str {
        "jackson"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn priority(&self) -> i32 {
        40
    }

    fn description(&self) -> &str {
        "Transforms Jackson annotations to Go struct tags and encoding/json"
    }

    fn matches(&self, _ctx: &MatchContext) -> bool {
        todo!()
    }

    fn transform(&self, _ctx: &mut TransformContext) -> Result<Vec<GoNode>, PluginError> {
        todo!()
    }
}
