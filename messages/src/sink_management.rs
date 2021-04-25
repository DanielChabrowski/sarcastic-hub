use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub enum SinkRequest {
    Register { name: String },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum SinkResponse {
    Play(PathBuf),
    Pause,
    Stop,
    Dummy,
}
