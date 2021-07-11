mod pulsewatcher;
use pulsewatcher::{PulseMessage, PulseWatcher};

mod player;
use player::Player;

use anyhow::Result;
use futures_util::StreamExt;
use log::{debug, error, info, warn};
use messages::sink_management::SinkResponse;
use tokio::sync::mpsc::unbounded_channel as channel;
use tokio_tungstenite::tungstenite::Message;

#[tokio::main]
async fn main() -> Result<()> {
    if cfg!(debug_assertions) {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    } else {
        env_logger::init();
    }

    info!(
        "Starting {} {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );

    let (sender, mut receiver) = channel::<PulseMessage>();

    let pulse_audio_connection = tokio::task::spawn_blocking(move || {
        let pulse_watcher = PulseWatcher::new(sender).unwrap();
        pulse_watcher.run().unwrap()
    });

    let player = Player::new()?;

    let server = tokio::spawn(async move {
        while let Some(msg) = receiver.recv().await {
            debug!("Received {:?}", msg);
        }
    });

    let (ws_stream, _http_response) = tokio_tungstenite::connect_async(
        url::Url::parse("ws://127.0.0.1:9024").expect("Valid Hub url"),
    )
    .await
    .expect("WebSocket connection to Hub failed");

    let (mut ws_write, mut ws_read) = ws_stream.split();

    let hub_connection = tokio::spawn(async move {
        while let Some(msg) = ws_read.next().await {
            match msg {
                Ok(msg) => match msg {
                    Message::Text(msg) => {
                        let msg: SinkResponse = serde_json::from_str(&msg).unwrap();
                        debug!("Received message: {:?}", msg);

                        match msg {
                            SinkResponse::Play(path) => {
                                player.set_uri(&format!("file://{}", path.to_str().unwrap()));
                                player.play();
                            }
                            SinkResponse::Pause => {
                                player.pause();
                            }
                            SinkResponse::Stop => {
                                player.stop();
                            }
                            SinkResponse::Dummy => {}
                        }
                    }
                    Message::Close(close_frame) => {
                        debug!("Closing HubConnection: {:?}", close_frame);
                        break;
                    }
                    _ => {
                        warn!("HubConnection unsupported message type");
                    }
                },
                Err(e) => {
                    error!("HubConnection WebSocket error: {:?}", e);
                }
            }
        }
    });

    tokio::try_join!(server, hub_connection, pulse_audio_connection)?;

    Ok(())
}
