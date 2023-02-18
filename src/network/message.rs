use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize)]
pub struct GetBlocksMessage {
    pub from: u32,
    // If to is 0 the maximum blocks will be returned
    pub to: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetStatusMessage {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusMessage {
    // The id of the Server
    pub id: String,
    pub version: u32,
    pub current_height: u32,
}

impl StatusMessage {
    pub fn new(id: String, version: u32, current_height: u32) -> Self {
        Self {
            id,
            version,
            current_height,
        }
    }
}
