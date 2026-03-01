#![allow(dead_code)]

mod entity;
mod relationships;
mod repository;

use jovial_plugin::prelude::*;

pub struct SpringDataPlugin;

impl Plugin for SpringDataPlugin {
    fn name(&self) -> &str {
        "spring-data"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn priority(&self) -> i32 {
        20
    }

    fn description(&self) -> &str {
        "Transforms JPA entities and Spring Data repositories to GORM"
    }

    fn matches(&self, _ctx: &MatchContext) -> bool {
        todo!()
    }

    fn transform(&self, _ctx: &mut TransformContext) -> Result<Vec<GoNode>, PluginError> {
        todo!()
    }
}
