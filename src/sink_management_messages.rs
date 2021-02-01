use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub enum SinkRequest {}

#[derive(Debug, Clone, Serialize)]
pub enum SinkResponse {
    Dummy,
}
