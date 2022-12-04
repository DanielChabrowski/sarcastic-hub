use crate::filesystem_provider::FilesystemProvider;
use crate::provider::{Provider, ResourceProviderInterface};
use crate::resource::Resource;
use crate::{
    config::{self, Config},
    ws_server::WebSocketHandler,
};
use anyhow::Result;
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
type Sinks = HashMap<uuid::Uuid, Sink>;

enum Sink {
    Registered(RegisteredSink),
    Unregistered(Sender<SinkResponse>),
}

impl Sink {
    fn send_to_registered(&self, msg: SinkResponse) -> Result<()> {
        match self {
            Sink::Registered(sink) => Ok(sink.sender.send(msg)?),
            Sink::Unregistered(_) => Ok(()),
        }
    }
}

struct RegisteredSink {
    sender: Sender<SinkResponse>,
    name: String,
}

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

    async fn handle_query_sinks(&self) -> WebUiResponse {
        let mut sinks = Vec::<web_interface::Sink>::new();

        for (uid, sink) in self.sinks.read().await.iter() {
            if let Sink::Registered(sink) = sink {
                sinks.push(web_interface::Sink {
                    uid: *uid,
                    name: sink.name.clone(),
                });
            }
        }

        WebUiResponse::Sinks(sinks)
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
            .map(|v| web_interface::Provider {
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
                let resource = resources.get(uid);

                match resource {
                    Some(r) => {
                        log::debug!("Received action for resource {:?}", r);
                        let msg = SinkResponse::Play(r.path.clone());

                        let sinks = self.sinks.read().await;
                        let (_uid, sink) = sinks.iter().next().unwrap();
                        sink.send_to_registered(msg).expect("Message sent to sink");
                    }
                    None => {
                        log::debug!("Could not find a resource identified by {}", uid);
                    }
                }
            }
            Action::Stop => {
                let msg = SinkResponse::Stop;
                let sinks = self.sinks.read().await;
                let (_uid, sink) = sinks.iter().next().unwrap();
                sink.send_to_registered(msg).expect("Message sent to sink");
            }
            Action::Pause => {
                let msg = SinkResponse::Pause;
                let sinks = self.sinks.read().await;
                let (_uid, sink) = sinks.iter().next().unwrap();
                sink.send_to_registered(msg).expect("Message sent to sink");
            }
        }

        WebUiResponse::Error(ProblemDetails {
            description: "Action not implemented".into(),
        })
    }

    async fn notify_web_clients(&self, msg: WebUiResponse) {
        let web_clients = self.web_clients.read().await;

        for (_, client) in web_clients.iter() {
            client.send(msg.clone()).expect("Message sent to WebClient");
        }
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
    async fn handle(&self, _: uuid::Uuid, req: WebUiRequest) -> WebUiResponse {
        match req {
            WebUiRequest::QuerySinks => self.handle_query_sinks().await,
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
    async fn handle(&self, id: uuid::Uuid, req: SinkRequest) -> SinkResponse {
        match req {
            SinkRequest::Register { name } => {
                let mut needs_notification = false;

                {
                    let mut sink_lock = self.sinks.write().await;
                    let sink = sink_lock.get_mut(&id);
                    if let Some(sink) = sink {
                        match sink {
                            Sink::Registered(_) => {
                                // TODO: Incorrect state, disconnect
                                todo!()
                            }
                            Sink::Unregistered(sender) => {
                                *sink = Sink::Registered(RegisteredSink {
                                    sender: sender.clone(),
                                    name,
                                });
                                needs_notification = true;
                            }
                        }
                    }
                }

                if needs_notification {
                    self.notify_web_clients(self.handle_query_sinks().await)
                        .await;
                }
            }
        }

        SinkResponse::Dummy
    }

    async fn add_connection(&self, sender: Sender<SinkResponse>) -> uuid::Uuid {
        let uid = uuid::Uuid::new_v4();
        log::debug!("Adding new sink {}", uid);
        self.sinks
            .write()
            .await
            .insert(uid, Sink::Unregistered(sender));
        uid
    }

    async fn remove_connection(&self, uid: uuid::Uuid) {
        log::debug!("Removing sink: {}", uid);
        self.sinks.write().await.remove(&uid);
    }
}
