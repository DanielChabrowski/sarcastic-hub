use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct QueryProviders {}

#[derive(Debug, Clone, Deserialize)]
pub struct QueryResources {}

#[derive(Debug, Clone, Deserialize)]
pub enum Action {
    Play(uuid::Uuid),
    Stop,
    Pause,
}

#[derive(Debug, Clone, Deserialize)]
pub enum WebUiRequest {
    QueryProviders(QueryProviders),
    QueryResources(QueryResources),
    Action(Action),
}

#[derive(Debug, Clone, Serialize)]
pub struct Provider {
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Resource {
    pub uuid: uuid::Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemDetails {
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
pub enum WebUiResponse {
    Providers(Vec<Provider>),
    Resources(Vec<Resource>),
    Error(ProblemDetails),
}
