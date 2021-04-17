use crate::filesystem_provider::FilesystemProvider;
use crate::provider::Provider;
use crate::resource::Resource;
use crate::sink_management_messages::{SinkRequest, SinkResponse};
use crate::web_ui_messages::{
    self, Action, ProblemDetails, QueryProviders, QueryResources, WebUiRequest, WebUiResponse,
};
use crate::{
    config::{self, Config},
    ws_server::WebSocketHandler,
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::mpsc::UnboundedSender as Sender;
use tokio::sync::RwLock;

type Providers = HashMap<String, Box<dyn Provider + Sync + Send>>;
type Resources = HashMap<uuid::Uuid, Resource>;
type WebClients = HashMap<uuid::Uuid, Sender<WebUiResponse>>;

pub struct Hub {
    _config: Config,
    providers: RwLock<Providers>,
    resources: Arc<RwLock<Resources>>,
    web_clients: Arc<RwLock<WebClients>>,
}

impl Hub {
    pub fn new(config: Config) -> Self {
        let providers = create_providers(&config);
        Self {
            _config: config,
            providers: RwLock::new(providers),
            resources: Arc::new(RwLock::new(Resources::new())),
            web_clients: Arc::new(RwLock::new(WebClients::new())),
        }
    }

    async fn handle_query_providers(&self, _query: &QueryProviders) -> WebUiResponse {
        if self.providers.read().await.is_empty() {
            return WebUiResponse::Error(ProblemDetails {
                description: "There are no providers registered".into(),
            });
        }

        let providers = &*self.providers.read().await;

        for provider in providers.values() {
            provider.fetch().await;
        }

        let providers = providers
            .iter()
            .map(|(ref key, _)| web_ui_messages::Provider {
                name: key.as_str().to_string(),
            })
            .collect::<Vec<web_ui_messages::Provider>>();

        WebUiResponse::Providers(providers.to_vec())
    }

    async fn handle_query_resources(&self, _query: &QueryResources) -> WebUiResponse {
        let providers = &*self.providers.read().await;

        let mut resources = Vec::new();
        for provider in providers.values() {
            let provided: Vec<_> = provider.fetch().await;

            {
                let mut resources = self.resources.write().await;
                for resource in &provided {
                    resources.insert(resource.uuid, resource.clone());
                }
            }

            let mut provided = provided
                .into_iter()
                .map(|res| web_ui_messages::Resource {
                    uuid: res.uuid,
                    name: res.name,
                })
                .collect();

            resources.append(&mut provided);
        }

        WebUiResponse::Resources(resources)
    }

    async fn handle_action(&self, query: &Action) -> WebUiResponse {
        let resources = self.resources.read().await;
        let resource = resources.get(&query.resource_uuid);

        match resource {
            Some(r) => {
                log::debug!("Received action for resource {:?}", r);
            }
            None => {
                log::debug!(
                    "Could not find a resource identified by {}",
                    query.resource_uuid
                );
            }
        }

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
            WebUiRequest::Action(q) => self.handle_action(&q).await,
        }
    }

    async fn add_connection(
        &self,
        sender: tokio::sync::mpsc::UnboundedSender<WebUiResponse>,
    ) -> uuid::Uuid {
        let uid = uuid::Uuid::new_v4();
        log::debug!("Adding new web client {}", uid);
        self.web_clients.write().await.insert(uid, sender);
        uid
    }

    async fn remove_connection(&self, uid: uuid::Uuid) {
        log::debug!("Removing web client {}", uid);
        self.web_clients.write().await.remove(&uid);
    }
}

#[async_trait::async_trait]
impl WebSocketHandler<SinkRequest, SinkResponse> for Hub {
    async fn handle(&self, req: SinkRequest) -> SinkResponse {
        log::debug!("{:?}", req);
        SinkResponse::Dummy
    }

    async fn add_connection(
        &self,
        _sender: tokio::sync::mpsc::UnboundedSender<SinkResponse>,
    ) -> uuid::Uuid {
        uuid::Uuid::new_v4()
    }

    async fn remove_connection(&self, _id: uuid::Uuid) {
        todo!()
    }
}
