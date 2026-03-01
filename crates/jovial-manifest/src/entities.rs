use serde::{Deserialize, Serialize};

/// A JPA entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub class_name: String,
    pub table_name: String,
    pub fields: Vec<EntityField>,
    pub relationships: Vec<Relationship>,
}

/// A field in a JPA entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityField {
    pub name: String,
    pub column_name: String,
    pub field_type: String,
    pub nullable: bool,
    pub is_id: bool,
    pub is_generated: bool,
}

/// A relationship between entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub field_name: String,
    pub target_entity: String,
    pub relationship_type: RelationshipType,
    pub cascade: Vec<CascadeType>,
    pub fetch: FetchType,
    pub mapped_by: Option<String>,
}

/// JPA relationship types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationshipType {
    OneToOne,
    OneToMany,
    ManyToOne,
    ManyToMany,
}

/// JPA cascade types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CascadeType {
    All,
    Persist,
    Merge,
    Remove,
    Refresh,
    Detach,
}

/// JPA fetch types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FetchType {
    Eager,
    Lazy,
}
