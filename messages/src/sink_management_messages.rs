use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub enum SinkRequest {
    Register { name: String },
}

#[derive(Debug, Clone, Serialize)]
pub enum SinkResponse {
    Dummy,
}
