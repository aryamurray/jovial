use serde::{Deserialize, Serialize};

/// An item that could not be fully resolved during extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnresolvedItem {
    pub kind: UnresolvedKind,
    pub name: String,
    pub location: String,
    pub reason: String,
}

/// Kind of unresolved item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnresolvedKind {
    Bean,
    Endpoint,
    Entity,
    Type,
    Dependency,
}
