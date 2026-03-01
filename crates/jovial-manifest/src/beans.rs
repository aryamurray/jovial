use serde::{Deserialize, Serialize};

/// A Spring bean definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bean {
    pub name: String,
    pub class_name: String,
    pub scope: BeanScope,
    pub dependencies: Vec<Dependency>,
    pub proxy_info: Option<ProxyInfo>,
}

/// Bean scope.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BeanScope {
    #[default]
    Singleton,
    Prototype,
    Request,
    Session,
}

/// A bean dependency (injected via constructor, field, or setter).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub bean_name: String,
    pub type_name: String,
    pub required: bool,
}

/// Proxy information for AOP-wrapped beans.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyInfo {
    pub proxy_type: ProxyType,
    pub interfaces: Vec<String>,
}

/// Kind of proxy wrapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProxyType {
    Jdk,
    Cglib,
}
