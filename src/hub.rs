use crate::config::{self, Config};
use crate::filesystem_provider::FilesystemProvider;
use crate::provider::Provider;
use crate::web_ui_messages::{
    self, Action, ProblemDetails, QueryProviders, QueryResources, WebUiRequest, WebUiResponse,
};
use std::collections::HashMap;
use tokio::sync::RwLock;

type Providers = HashMap<String, Box<dyn Provider + Sync + Send>>;

pub struct Hub {
    config: Config,
    providers: RwLock<Providers>,
}

impl Hub {
    pub fn new(config: Config) -> Self {
        let providers = create_providers(&config);
        Self {
            config,
            providers: RwLock::new(providers),
        }
    }

    pub async fn handle_web_ui_request(&self, req: &WebUiRequest) -> WebUiResponse {
        match req {
            WebUiRequest::QueryProviders(q) => self.handle_query_providers(q).await,
            WebUiRequest::QueryResources(q) => self.handle_query_resources(q),
            WebUiRequest::Action(q) => self.handle_action(q),
        }
    }

    async fn handle_query_providers(&self, _query: &QueryProviders) -> WebUiResponse {
        if self.providers.read().await.is_empty() {
            return WebUiResponse::Error(ProblemDetails {
                description: "There are no providers registered".into(),
            });
        }

        let providers = &*self.providers.read().await;
        let providers = providers
            .into_iter()
            .map(|(ref key, _)| web_ui_messages::Provider {
                name: key.as_str().to_string(),
            })
            .collect::<Vec<web_ui_messages::Provider>>();

        WebUiResponse::Providers(providers.to_vec())
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

fn create_providers(config: &Config) -> Providers {
    let mut providers = Providers::new();

    for provider in &config.providers {
        match provider {
            config::Provider::Filesystem(p) => {
                let fs_provider = FilesystemProvider::new(p.name.clone(), p.paths.clone());
                providers.insert(p.name.clone(), Box::new(fs_provider));
            }
            _ => {}
        }
    }

    providers
}
