mod hub;
mod web_ui;
mod web_ui_messages;

use crate::web_ui::WebUiServer;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    env_logger::init();

    let hub = Arc::new(hub::Hub::new());

    let web_ui = WebUiServer::new(hub.clone());
    let web_ui = web_ui.listen("0:9023");

    let _ = tokio::join!(web_ui);
}
