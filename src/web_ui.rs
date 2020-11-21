use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use log::debug;
use tokio::net::{TcpListener, TcpStream};

use crate::web_ui_messages::WebUiRequest;

pub struct WebUiServer {}

impl WebUiServer {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn listen(&self, addr: &str) -> Result<()> {
        let ws_listener = TcpListener::bind(addr)
            .await
            .map_err(|e| anyhow!("WebUi binding failed: {:?}", e))?;

        tokio::spawn(async move {
            while let Ok((stream, _)) = ws_listener.accept().await {
                tokio::spawn(accept_connection(stream));
            }
        })
        .await
        .map_err(|e| anyhow!("Listening for new connections failed: {:?}", e))
    }
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
                        match message {
                            Ok(message) => {
                                debug!("Message: {:?}", message);
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
