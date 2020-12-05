use crate::web_ui_messages::{
    Action, ProblemDetails, QueryProviders, QueryResources, WebUiRequest, WebUiResponse,
};

pub struct Hub {}

impl Hub {
    pub fn new() -> Self {
        Self {}
    }

    pub fn handle_web_ui_request(&self, req: &WebUiRequest) -> WebUiResponse {
        match req {
            WebUiRequest::QueryProviders(q) => self.handle_query_providers(q),
            WebUiRequest::QueryResources(q) => self.handle_query_resources(q),
            WebUiRequest::Action(q) => self.handle_action(q),
        }
    }

    fn handle_query_providers(&self, _query: &QueryProviders) -> WebUiResponse {
        WebUiResponse::Error(ProblemDetails {
            description: "QueryProviders not implemented".into(),
        })
    }

    fn handle_query_resources(&self, _query: &QueryResources) -> WebUiResponse {
        WebUiResponse::Error(ProblemDetails {
            description: "QueryResources not implemented".into(),
        })
    }

    fn handle_action(&self, _query: &Action) -> WebUiResponse {
        WebUiResponse::Error(ProblemDetails {
            description: "Action not implemented".into(),
        })
    }
}
