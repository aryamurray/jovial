//! Parameter extraction transforms for @PathVariable, @RequestParam, @RequestBody.

use jovial_plugin::prelude::*;

/// Transform Spring parameter annotations into gin context extractions.
pub fn transform_params(_node: &JavaNode) -> Result<Vec<GoNode>, PluginError> {
    todo!()
}
