/// Trait for resolving Java types to their fully-qualified class names.
///
/// Lives in jovial-ast so both jovial-parser (provides DefaultTypeResolver impl)
/// and jovial-plugin (exposes via MatchContext) can depend on it without circular deps.
pub trait TypeResolver {
    /// Resolve a simple name to its fully-qualified class name.
    fn resolve(&self, simple_name: &str) -> Option<String>;

    /// Check if a type is assignable to another type.
    fn is_assignable_to(&self, from: &str, to: &str) -> bool;

    /// Get the superclass of a type, if any.
    fn superclass_of(&self, fqcn: &str) -> Option<String>;

    /// Get interfaces implemented by a type.
    fn interfaces_of(&self, fqcn: &str) -> Vec<String>;
}
