//! @OneToMany, @ManyToOne etc. → GORM association transforms.

use jovial_plugin::prelude::*;

/// Transform JPA relationship annotations into GORM association tags.
pub fn transform_relationship(_node: &JavaNode) -> Result<Vec<GoNode>, PluginError> {
    todo!()
}
