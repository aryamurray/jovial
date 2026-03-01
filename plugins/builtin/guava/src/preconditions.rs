//! Preconditions.check* transforms.

use jovial_plugin::prelude::*;

/// Transform Guava Preconditions checks to Go panic/error checks.
pub fn transform_precondition(_node: &JavaNode) -> Result<Vec<GoNode>, PluginError> {
    todo!()
}
