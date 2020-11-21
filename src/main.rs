mod web_ui;
mod web_ui_messages;

use crate::web_ui::WebUiServer;

#[tokio::main]
async fn main() {
    env_logger::init();

    let web_ui = WebUiServer::new();
    let web_ui = web_ui.listen("127.0.0.1:9023");

    let _ = tokio::join!(web_ui);
}
