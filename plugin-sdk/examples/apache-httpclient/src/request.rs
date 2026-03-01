//! HTTP request building transforms.

use jovial_plugin::prelude::*;

/// Transform HttpGet/HttpPost/etc. to Go http.NewRequest.
pub fn transform_request(_node: &JavaNode) -> Result<Vec<GoNode>, PluginError> {
    todo!()
}
