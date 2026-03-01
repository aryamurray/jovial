#![allow(dead_code)]

mod controller;
mod params;
mod response;

use jovial_plugin::prelude::*;

pub struct SpringWebPlugin;

impl Plugin for SpringWebPlugin {
    fn name(&self) -> &str {
        "spring-web"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn priority(&self) -> i32 {
        10
    }

    fn description(&self) -> &str {
        "Transforms Spring @RestController endpoints to gin-gonic handlers"
    }

    fn matches(&self, _ctx: &MatchContext) -> bool {
        todo!()
    }

    fn transform(&self, _ctx: &mut TransformContext) -> Result<Vec<GoNode>, PluginError> {
        todo!()
    }
}
