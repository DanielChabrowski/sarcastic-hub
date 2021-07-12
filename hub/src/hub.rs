use crate::filesystem_provider::FilesystemProvider;
use crate::provider::{Provider, ResourceProviderInterface};
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
use tokio::sync::mpsc::{unbounded_channel as channel, UnboundedSender as Sender};
use tokio::sync::RwLock;

type ResourceSender = Sender<ResourceProviderInterface>;
type Providers = Vec<Box<dyn Provider + Sync + Send>>;
type Resources = HashMap<uuid::Uuid, Resource>;
type WebClients = HashMap<uuid::Uuid, Sender<WebUiResponse>>;
type Sinks = HashMap<uuid::Uuid, Sender<SinkResponse>>;

pub struct Hub {
    providers: RwLock<Providers>,
    resources: Arc<RwLock<Resources>>,
    web_clients: Arc<RwLock<WebClients>>,
    sinks: Arc<RwLock<Sinks>>,
}

impl Hub {
    pub fn new(config: Config) -> Self {
        let mut zelf = Self {
            providers: RwLock::new(Providers::new()),
            resources: Arc::new(RwLock::new(Resources::new())),
            web_clients: Arc::new(RwLock::new(WebClients::new())),
            sinks: Arc::new(RwLock::new(Sinks::new())),
        };

        let (sender, _receiver_handle) = zelf.create_resource_receiver();

        let providers = create_providers(&config, sender);
        zelf.providers = RwLock::new(providers);

        zelf
    }

    fn create_resource_receiver(&self) -> (ResourceSender, tokio::task::JoinHandle<()>) {
        let (sender, mut receiver) = channel::<ResourceProviderInterface>();

        let resources = self.resources.clone();
        let handle = tokio::spawn(async move {
            while let Some(msg) = receiver.recv().await {
                let mut resources = resources.write().await;

                match msg {
                    ResourceProviderInterface::Add(new_resources) => {
                        log::debug!("Received Add of {} resources", new_resources.len());
                        new_resources.into_iter().for_each(|new| {
                            let uid = uuid::Uuid::new_v4();
                            resources.insert(uid, new);
                        });
                    }
                }
            }
        });

        (sender, handle)
    }

    async fn handle_query_providers(&self, _query: &QueryProviders) -> WebUiResponse {
        if self.providers.read().await.is_empty() {
            return WebUiResponse::Error(ProblemDetails {
                description: "There are no registered providers".into(),
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
        let resources = self.resources.read().await;
        let resources = resources
            .iter()
            .map(|(uid, res)| web_interface::Resource {
                uuid: *uid,
                name: res.name.clone(),
            })
            .collect();

        WebUiResponse::Resources(resources)
    }

    async fn handle_action(&self, query: &Action) -> WebUiResponse {
        {
            let sinks = self.sinks.read().await;
            if sinks.is_empty() {
                return WebUiResponse::Error(ProblemDetails {
                    description: "There are no registered sinks".to_string(),
                });
            }
        }

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

fn create_providers(config: &Config, resource_sender: ResourceSender) -> Providers {
    let mut providers = Providers::new();

    for provider in &config.providers {
        match provider {
            config::Provider::Filesystem(p) => {
                let fs_provider = FilesystemProvider::new(
                    p.name.clone(),
                    p.paths.clone(),
                    p.extensions.clone(),
                    resource_sender.clone(),
                );
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

    async fn add_connection(&self, sender: Sender<WebUiResponse>) -> uuid::Uuid {
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

    async fn add_connection(&self, sender: Sender<SinkResponse>) -> uuid::Uuid {
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
