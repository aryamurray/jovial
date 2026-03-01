#![allow(dead_code)]

//! Apache HttpClient plugin example — multi-file plugin demonstrating
//! transformation of Apache HttpClient calls to Go net/http.

mod client;
mod request;

use jovial_plugin::prelude::*;

pub struct ApacheHttpClientPlugin;

impl Plugin for ApacheHttpClientPlugin {
    fn name(&self) -> &str {
        "apache-httpclient"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn priority(&self) -> i32 {
        50
    }

    fn description(&self) -> &str {
        "Transforms Apache HttpClient calls to Go net/http"
    }

    fn matches(&self, _ctx: &MatchContext) -> bool {
        todo!()
    }

    fn transform(&self, _ctx: &mut TransformContext) -> Result<Vec<GoNode>, PluginError> {
        todo!()
    }
}
