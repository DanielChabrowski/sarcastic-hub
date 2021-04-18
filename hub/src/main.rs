mod config;
mod filesystem_provider;
mod hub;
mod provider;
mod resource;
mod resource_manager;
mod sink_management_messages;
mod web_ui_messages;
mod ws_server;

use crate::ws_server::WebSocketServer;
use sink_management_messages::{SinkRequest, SinkResponse};
use std::sync::Arc;
use web_ui_messages::{WebUiRequest, WebUiResponse};

#[tokio::main]
async fn main() {
    env_logger::init();

    log::info!(
        "Starting {} {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );

    let config = config::load_config("config.json").expect("configuration file read");
    log::debug!("{:#?}", config);

    let web_ui_address = config.web_ui_address;
    let sink_management_address = config.sink_management_address;

    let hub = Arc::new(hub::Hub::new(config));

    let web_ui = WebSocketServer::<WebUiRequest, WebUiResponse>::new(hub.clone());
    let web_ui = web_ui.listen(web_ui_address);

    let sink_management = WebSocketServer::<SinkRequest, SinkResponse>::new(hub.clone());
    let sink_management = sink_management.listen(sink_management_address);

    let _ = tokio::join!(sink_management, web_ui);
}
