mod pulsewatcher;
use pulsewatcher::{PulseMessage, PulseWatcher};

use futures_util::SinkExt;
use futures_util::StreamExt;
use gstreamer::prelude::*;
use log::{debug, error, info};
use tokio::sync::mpsc::unbounded_channel as channel;

#[tokio::main]
async fn main() {
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

    let player = gstreamer_player::Player::new(None, None);
    player.connect_error(|_, err| {
        error!("{:?}", err);
    });

    player.connect_state_changed(|p, _err| {
        debug!("state changed: {:?}", p);
    });

    player.connect_buffering(|_, p| {
        debug!("buffering {}", p);
    });

    use glib::GString;

    let pipeline = player.get_pipeline();

    let pulsesink = gstreamer::ElementFactory::make("pulsesink", Some("dantesink")).unwrap();

    pulsesink.connect_notify(Some("current-device"), |zelf, _| {
        let current_device = zelf.get_property("current-device").unwrap();
        debug!(
            "Current device: {:?}",
            current_device.get::<GString>().unwrap().unwrap().as_str()
        );
    });

    pipeline.set_property("audio-sink", &pulsesink).unwrap();

    player.play();

    let (sender, mut receiver) = channel::<PulseMessage>();

    let pulse_audio_connection = tokio::task::spawn_blocking(move || {
        let pulse_watcher = PulseWatcher::new(sender).unwrap();
        pulse_watcher.run().unwrap()
    });

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
            debug!("Message from Hub: {:?}", msg);
        }
    });

    ws_write
        .send(tokio_tungstenite::tungstenite::Message::Text(
            "{ \"Register\": { \"name\": \"audio-sink\" } }".into(),
        ))
        .await
        .expect("Message send");

    tokio::try_join!(server, hub_connection, pulse_audio_connection).expect("Joined");
}
