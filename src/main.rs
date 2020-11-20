use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use log::debug;
use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, TcpStream};

mod web_ui_messages;
use web_ui_messages::WebUiRequest;

struct ServiceId(u32);

enum Category {
    Audio,
    Video,
    Other(String),
}

struct Resource {
    service_id: ServiceId,
    category: Category,
    url: Option<String>,
    local: bool,
}

struct Server {
    resources: Arc<Mutex<Vec<Resource>>>,
    sinks: Arc<Mutex<Vec<Resource>>>,
    clients: Arc<Mutex<Vec<WebClient>>>,
}

async fn accept_connection(stream: TcpStream) -> Result<()> {
    let addr = stream
        .peer_addr()
        .map_err(|_| anyhow!("peer address missing"))?;

    debug!("Accepted connection from {}", addr);

    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .map_err(|e| anyhow!("Error during the websocket handshake occurred: {}", e))?;

    debug!("New WebSocket connection: {}", addr);

    let (_write, mut read) = ws_stream.split();

    loop {
        let message = read.next().await;
        match message {
            Some(message) => {
                let message = message.unwrap();
                if message.is_close() {
                    break;
                }

                match message.to_text() {
                    Ok(message) => {
                        let message: Result<WebUiRequest> = serde_json::from_str(message)
                            .map_err(|e| anyhow!("Deserialization failed: {:?}", e));

                        debug!("Message: {:?}", message);
                    }
                    Err(e) => {
                        debug!("Invalid message: {:?}", e);
                    }
                }
            }
            None => break,
        }
    }

    debug!("Connection to {} closed", addr);

    Ok(())
}

struct WebClient {
    sender: u16,
}

struct WebServer {
    clients: Vec<WebClient>,
}

impl WebServer {
    fn new() -> Self {
        Self { clients: vec![] }
    }

    async fn listen(&self, addr: &str) -> () {
        let ws_listener = TcpListener::bind(addr).await.expect("WebSocket bound");

        while let Ok((stream, _)) = ws_listener.accept().await {
            tokio::spawn(accept_connection(stream));
        }
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let web_server = WebServer::new();
    web_server.listen("127.0.0.1:9023").await;
}
