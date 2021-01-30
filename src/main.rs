mod config;
mod filesystem_provider;
mod hub;
mod provider;
mod resource;
mod resource_manager;
mod web_ui;
mod web_ui_messages;

use crate::web_ui::WebUiServer;
use std::sync::Arc;
use web_ui_messages::{WebUiRequest, WebUiResponse};

#[tokio::main]
async fn main() {
    env_logger::init();

    let config = config::load_config("config.json").expect("configuration file read");
    log::debug!("{:#?}", config);

    let web_ui_address = config.web_ui_address;

    let hub = Arc::new(hub::Hub::new(config));

    let web_ui = WebUiServer::<WebUiRequest, WebUiResponse>::new(hub.clone());
    let web_ui = web_ui.listen(web_ui_address);

    let _ = tokio::join!(web_ui);
}
