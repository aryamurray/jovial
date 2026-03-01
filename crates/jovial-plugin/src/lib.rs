pub mod context;
pub mod error;
pub mod registry;
pub mod traits;
pub mod types;

/// Generous prelude — `use jovial_plugin::prelude::*` gives you everything
/// needed to write a plugin.
pub mod prelude {
    pub use jovial_ast::go::*;
    pub use jovial_ast::java::*;
    pub use jovial_ast::type_resolver::TypeResolver;

    pub use crate::context::{MatchContext, TransformContext};
    pub use crate::error::{Diagnostic, PluginError, Severity};
    pub use crate::traits::Plugin;
    pub use crate::types::{ConfigValue, GoDependency, PluginGoImport};
}
