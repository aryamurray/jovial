use jovial_ast::go::GoNode;

use crate::context::{MatchContext, TransformContext};
use crate::error::PluginError;

/// Trait that all Jovial plugins must implement.
pub trait Plugin: Send + Sync {
    /// Unique name of this plugin.
    fn name(&self) -> &str;

    /// Version string (semver).
    fn version(&self) -> &str;

    /// Execution priority (lower = runs first).
    fn priority(&self) -> i32 {
        100
    }

    /// Human-readable description.
    fn description(&self) -> &str {
        ""
    }

    /// Return true if this plugin wants to handle the given Java node.
    fn matches(&self, ctx: &MatchContext) -> bool;

    /// Transform a matched Java node into Go node(s).
    fn transform(&self, ctx: &mut TransformContext) -> Result<Vec<GoNode>, PluginError>;
}
