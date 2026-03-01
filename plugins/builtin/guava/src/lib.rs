#![allow(dead_code)]

mod collections;
mod optional;
mod preconditions;
mod strings;

use jovial_plugin::prelude::*;

pub struct GuavaPlugin;

impl Plugin for GuavaPlugin {
    fn name(&self) -> &str {
        "guava"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn priority(&self) -> i32 {
        50
    }

    fn description(&self) -> &str {
        "Transforms Google Guava APIs to Go standard library equivalents"
    }

    fn matches(&self, _ctx: &MatchContext) -> bool {
        todo!()
    }

    fn transform(&self, _ctx: &mut TransformContext) -> Result<Vec<GoNode>, PluginError> {
        todo!()
    }
}
