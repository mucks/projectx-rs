use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetStatusMessage {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusMessage {
    // The id of the Server
    id: String,
    version: u32,
    current_height: u32,
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
