mod player;
mod pulsewatcher;

use std::sync::Arc;

use anyhow::{bail, Result};
use futures_util::{SinkExt, StreamExt};
use log::{debug, info, warn};
use messages::sink_management::{SinkRequest, SinkResponse};
use player::Player;
use pulsewatcher::{PulseMessage, PulseWatcher};
use tokio::{select, sync::mpsc::unbounded_channel as channel};
use tokio_tungstenite::tungstenite::Message;

async fn handle_hub_message(msg: Message, player: Arc<Player>) -> Result<()> {
    match msg {
        Message::Text(msg) => {
            let msg: SinkResponse = serde_json::from_str(&msg).unwrap();
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
            bail!("Connection to Hub closed")
        }
        _ => {
            warn!("HubConnection unsupported message type");
        }
    }

    Ok(())
}

fn create_register_request() -> Result<Message> {
    let request = SinkRequest::Register {
        name: "Local test sink".to_string(),
    };
    let request = serde_json::to_string(&request)?;
    Ok(Message::Text(request))
}

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

    let (pulse_sender, mut pulse_receiver) = channel::<PulseMessage>();

    let pulse_audio_connection = tokio::task::spawn_blocking(move || {
        let pulse_watcher = PulseWatcher::new(pulse_sender).unwrap();
        pulse_watcher.run().unwrap()
    });

    let player = Player::new()?;
    let player = Arc::new(player);

    let (ws_stream, _http_response) = tokio_tungstenite::connect_async(
        url::Url::parse("ws://127.0.0.1:9024").expect("Valid Hub url"),
    )
    .await
    .expect("WebSocket connection to Hub failed");

    let (mut ws_write, mut ws_read) = ws_stream.split();

    let register_request = create_register_request()?;
    ws_write.send(register_request).await?;

    loop {
        select! {
            Some(msg) = ws_read.next() => {
                debug!("WebSocket message: {:?}", msg);
                if let Err(_) = handle_hub_message(msg?, player.clone()).await {
                    break;
                }
            }

            Some(msg) = pulse_receiver.recv() => {
                debug!("PulseAudio message: {:?}", msg);
            }
        }
    }

    tokio::try_join!(pulse_audio_connection)?;

    Ok(())
}
