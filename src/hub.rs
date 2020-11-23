use crate::web_ui_messages::{ProblemDetails, WebUiRequest, WebUiResponse};

pub struct Hub {}

impl Hub {
    pub fn new() -> Self {
        Self {}
    }

    pub fn handle_web_ui_request(&self, _req: &WebUiRequest) -> WebUiResponse {
        WebUiResponse::Error(ProblemDetails {
            description: "Not implemented".into(),
        })
    }
}
