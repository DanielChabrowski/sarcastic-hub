use crate::filesystem_provider::FilesystemProvider;
use crate::provider::Provider;
use crate::resource::Resource;
use crate::web_ui_messages::{
    self, Action, ProblemDetails, QueryProviders, QueryResources, WebUiRequest, WebUiResponse,
};
use crate::{
    config::{self, Config},
    web_ui::WebSocketHandler,
};
use std::collections::HashMap;
use tokio::sync::RwLock;

type Providers = HashMap<String, Box<dyn Provider + Sync + Send>>;
type Resources = HashMap<String, Resource>;

pub struct Hub {
    _config: Config,
    providers: RwLock<Providers>,
    resources: RwLock<Resources>,
}

impl Hub {
    pub fn new(config: Config) -> Self {
        let providers = create_providers(&config);
        Self {
            _config: config,
            providers: RwLock::new(providers),
            resources: RwLock::new(Resources::new()),
        }
    }

    async fn handle_query_providers(&self, _query: &QueryProviders) -> WebUiResponse {
        if self.providers.read().await.is_empty() {
            return WebUiResponse::Error(ProblemDetails {
                description: "There are no providers registered".into(),
            });
        }

        let providers = &*self.providers.read().await;

        for (_, provider) in providers {
            provider.fetch().await;
        }

        let providers = providers
            .into_iter()
            .map(|(ref key, _)| web_ui_messages::Provider {
                name: key.as_str().to_string(),
            })
            .collect::<Vec<web_ui_messages::Provider>>();

        WebUiResponse::Providers(providers.to_vec())
    }

    async fn handle_query_resources(&self, _query: &QueryResources) -> WebUiResponse {
        let providers = &*self.providers.read().await;

        let mut resources = Vec::new();
        for (_, provider) in providers {
            let mut provided: Vec<_> = provider
                .fetch()
                .await
                .iter()
                .map(|res| web_ui_messages::Resource {
                    name: res.name.clone(),
                })
                .collect();
            resources.append(&mut provided);
        }

        WebUiResponse::Resources(resources)
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
                let fs_provider =
                    FilesystemProvider::new(p.name.clone(), p.paths.clone(), p.extensions.clone());
                providers.insert(p.name.clone(), Box::new(fs_provider));
            }
        }
    }

    providers
}

#[async_trait::async_trait]
impl WebSocketHandler<WebUiRequest, WebUiResponse> for Hub {
    async fn handle(&self, req: WebUiRequest) -> WebUiResponse {
        match req {
            WebUiRequest::QueryProviders(q) => self.handle_query_providers(&q).await,
            WebUiRequest::QueryResources(q) => self.handle_query_resources(&q).await,
            WebUiRequest::Action(q) => self.handle_action(&q),
        }
    }
}
