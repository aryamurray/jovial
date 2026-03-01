pub mod advice;
pub mod beans;
pub mod endpoints;
pub mod entities;
pub mod unresolved;

use serde::{Deserialize, Serialize};

use crate::advice::AdviceChain;
use crate::beans::Bean;
use crate::endpoints::Endpoint;
use crate::entities::Entity;
use crate::unresolved::UnresolvedItem;

/// The complete manifest produced by the JVM extractor.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Manifest {
    pub beans: Vec<Bean>,
    pub endpoints: Vec<Endpoint>,
    pub entities: Vec<Entity>,
    pub advice_chains: Vec<AdviceChain>,
    pub unresolved: Vec<UnresolvedItem>,
}

impl Manifest {
    /// Deserialize a manifest from JSON.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}
