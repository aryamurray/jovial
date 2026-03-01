//! ImmutableList/Map/Set transforms.

use jovial_plugin::prelude::*;

/// Transform Guava immutable collection calls to Go slices/maps.
pub fn transform_collection(_node: &JavaNode) -> Result<Vec<GoNode>, PluginError> {
    todo!()
}
