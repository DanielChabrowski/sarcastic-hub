use crate::hub::Hub;
use crate::web_ui_messages::WebUiRequest;
use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use log::debug;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;

pub struct WebUiServer {
    hub: Arc<Hub>,
}

impl WebUiServer {
    pub fn new(hub: Arc<Hub>) -> Self {
        Self { hub }
    }

    pub async fn listen(&self, addr: &str) -> Result<()> {
        let ws_listener = TcpListener::bind(addr)
            .await
            .map_err(|e| anyhow!("WebUi binding failed: {:?}", e))?;

        let hub = self.hub.clone();
        tokio::spawn(async move {
            while let Ok((stream, _)) = ws_listener.accept().await {
                tokio::spawn(accept_connection(stream, hub.clone()));
            }
        })
        .await
        .map_err(|e| anyhow!("Listening for new connections failed: {:?}", e))
    }
}

async fn accept_connection(stream: TcpStream, hub: Arc<Hub>) -> Result<()> {
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
                        let message: Result<WebUiRequest, _> = serde_json::from_str(message);
                        debug!("Message received: {:?}", message);

                        match message {
                            Ok(message) => {
                                let response = hub.handle_web_ui_request(&message);

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
