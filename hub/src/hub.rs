use crate::filesystem_provider::FilesystemProvider;
use crate::provider::Provider;
use crate::resource::Resource;
use crate::{
    config::{self, Config},
    ws_server::WebSocketHandler,
};
use messages::sink_management::{SinkRequest, SinkResponse};
use messages::web_interface::{
    self, Action, ProblemDetails, QueryProviders, QueryResources, WebUiRequest, WebUiResponse,
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::mpsc::UnboundedSender as Sender;
use tokio::sync::RwLock;

type Providers = Vec<Box<dyn Provider + Sync + Send>>;
type Resources = HashMap<uuid::Uuid, Resource>;
type WebClients = HashMap<uuid::Uuid, Sender<WebUiResponse>>;
type Sinks = HashMap<uuid::Uuid, Sender<SinkResponse>>;

pub struct Hub {
    _config: Config,
    providers: RwLock<Providers>,
    resources: Arc<RwLock<Resources>>,
    web_clients: Arc<RwLock<WebClients>>,
    sinks: Arc<RwLock<Sinks>>,
}

impl Hub {
    pub fn new(config: Config) -> Self {
        let providers = create_providers(&config);
        Self {
            _config: config,
            providers: RwLock::new(providers),
            resources: Arc::new(RwLock::new(Resources::new())),
            web_clients: Arc::new(RwLock::new(WebClients::new())),
            sinks: Arc::new(RwLock::new(Sinks::new())),
        }
    }

    async fn handle_query_providers(&self, _query: &QueryProviders) -> WebUiResponse {
        if self.providers.read().await.is_empty() {
            return WebUiResponse::Error(ProblemDetails {
                description: "There are no providers registered".into(),
            });
        }

        let providers = self.providers.read().await;
        let providers = providers
            .iter()
            .map(|ref v| web_interface::Provider {
                name: v.get_name().to_string(),
            })
            .collect::<Vec<web_interface::Provider>>();

        WebUiResponse::Providers(providers.to_vec())
    }

    async fn handle_query_resources(&self, _query: &QueryResources) -> WebUiResponse {
        let providers = self.providers.read().await;

        let mut resources = Vec::new();
        for provider in &*providers {
            let provided: Vec<_> = provider.fetch().await;

            {
                let mut resources = self.resources.write().await;
                for resource in &provided {
                    resources.insert(resource.uuid, resource.clone());
                }
            }

            let mut provided = provided
                .into_iter()
                .map(|res| web_interface::Resource {
                    uuid: res.uuid,
                    name: res.name,
                })
                .collect();

            resources.append(&mut provided);
        }

        WebUiResponse::Resources(resources)
    }

    async fn handle_action(&self, query: &Action) -> WebUiResponse {
        match query {
            Action::Play(uid) => {
                let resources = self.resources.read().await;
                let resource = resources.get(&uid);

                match resource {
                    Some(r) => {
                        log::debug!("Received action for resource {:?}", r);
                        let msg = SinkResponse::Play(r.path.clone());

                        let sinks = self.sinks.read().await;
                        let (_uid, sender) = sinks.iter().next().unwrap();
                        sender.send(msg).expect("Message sent to sink");
                    }
                    None => {
                        log::debug!("Could not find a resource identified by {}", uid);
                    }
                }
            }
            Action::Stop => {
                let msg = SinkResponse::Stop;
                let sinks = self.sinks.read().await;
                let (_uid, sender) = sinks.iter().next().unwrap();
                sender.send(msg).expect("Message sent to sink");
            }
            Action::Pause => {
                let msg = SinkResponse::Pause;
                let sinks = self.sinks.read().await;
                let (_uid, sender) = sinks.iter().next().unwrap();
                sender.send(msg).expect("Message sent to sink");
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
                providers.push(Box::new(fs_provider));
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
        sender: tokio::sync::mpsc::UnboundedSender<SinkResponse>,
    ) -> uuid::Uuid {
        let uid = uuid::Uuid::new_v4();
        log::debug!("Adding new sink {}", uid);
        self.sinks.write().await.insert(uid, sender);
        uid
    }

    async fn remove_connection(&self, uid: uuid::Uuid) {
        log::debug!("Removing sink: {}", uid);
        self.sinks.write().await.remove(&uid);
    }
}
