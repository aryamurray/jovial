use serde::{Deserialize, Serialize};

/// An HTTP endpoint extracted from Spring MVC annotations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endpoint {
    pub path: String,
    pub method: HttpMethod,
    pub handler_class: String,
    pub handler_method: String,
    pub parameters: Vec<EndpointParam>,
    pub produces: Vec<String>,
    pub consumes: Vec<String>,
}

/// HTTP method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
}

/// A parameter of an endpoint handler method.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointParam {
    pub name: String,
    pub param_type: String,
    pub source: ParamSource,
    pub required: bool,
    pub default_value: Option<String>,
}

/// Where the parameter value comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParamSource {
    Path,
    Query,
    Body,
    Header,
    Cookie,
}
