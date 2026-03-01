use serde::{Deserialize, Serialize};

/// AOP advice chain information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdviceChain {
    pub target_class: String,
    pub target_method: String,
    pub transaction: Option<TransactionSpec>,
    pub cache: Option<CacheSpec>,
}

/// Transaction specification from @Transactional.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionSpec {
    pub propagation: String,
    pub isolation: String,
    pub read_only: bool,
    pub timeout: Option<i32>,
    pub rollback_for: Vec<String>,
}

/// Cache specification from @Cacheable/@CacheEvict.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheSpec {
    pub cache_names: Vec<String>,
    pub key_expression: Option<String>,
    pub evict: bool,
}
