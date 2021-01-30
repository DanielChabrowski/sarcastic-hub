use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use log::debug;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};
use tokio_tungstenite::tungstenite::Message;

#[async_trait::async_trait]
pub trait WebSocketHandler<Request, Response> {
    async fn handle(&self, request: Request) -> Response;
}

pub struct WebUiServer<Request, Response> {
    handler: Arc<dyn WebSocketHandler<Request, Response> + Send + Sync>,
}

impl<Request, Response> WebUiServer<Request, Response>
where
    Request: serde::de::DeserializeOwned + Send + Sync + 'static,
    Response: serde::Serialize + Send + Sync + 'static,
{
    pub fn new(handler: Arc<dyn WebSocketHandler<Request, Response> + Send + Sync>) -> Self {
        Self { handler }
    }

    pub async fn listen<A: ToSocketAddrs>(&self, addr: A) -> Result<()> {
        let ws_listener = TcpListener::bind(addr)
            .await
            .map_err(|e| anyhow!("WebUi binding failed: {:?}", e))?;

        let handler = self.handler.clone();
        tokio::spawn(async move {
            while let Ok((stream, _)) = ws_listener.accept().await {
                tokio::spawn(accept_connection(stream, handler.clone()));
            }
        })
        .await
        .map_err(|e| anyhow!("Listening for new connections failed: {:?}", e))
    }
}

async fn accept_connection<Request, Response>(
    stream: TcpStream,
    hub: Arc<dyn WebSocketHandler<Request, Response> + Send + Sync>,
) -> Result<()>
where
    Request: serde::de::DeserializeOwned + Send + Sync,
    Response: serde::Serialize + Send + Sync,
{
    let addr = stream
        .peer_addr()
        .map_err(|_| anyhow!("peer address missing"))?;

    debug!("Accepted connection from {}", addr);

    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .map_err(|e| anyhow!("Error during the websocket handshake occurred: {}", e))?;

    debug!("New WebSocket connection: {}", addr);

    let (mut write, mut read) = ws_stream.split();

    loop {
        let message = read.next().await;

        if message.is_none() {
            debug!("Couldn't read next message");
            break;
        }

        match message.unwrap() {
            Ok(message) => {
                if message.is_close() {
                    break;
                }

                match message.to_text() {
                    Ok(message) => {
                        let message: Result<Request, _> = serde_json::from_str(message);
                        // debug!("Message received: {:?}", message);

                        match message {
                            Ok(message) => {
                                let response = hub.handle(message).await;

                                let response = serde_json::to_string(&response)
                                    .map_err(|e| anyhow!("Serialization failed: {}", e))?;

                                use futures_util::SinkExt;
                                write
                                    .send(Message::Text(response))
                                    .await // Blocking receive task
                                    .map_err(|e| anyhow!("Sending message failed: {}", e))?;
                            }
                            Err(e) => {
                                debug!("Deserialization failed: {:?}", e);
                            }
                        }
                    }
                    Err(e) => {
                        debug!("Message is not in text format: {:?}", e);
                    }
                }
            }
            Err(e) => {
                debug!("Invalid message: {:?}", e);
            }
        }
    }

    debug!("Connection to {} closed", addr);

    Ok(())
}
