use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Debug, Serialize, Deserialize)]
pub struct AddNodeRequest {
    pub port: u16,
    pub public_key: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListedNode {
    pub addr: SocketAddr,
    pub public_key: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListNodeResponse {
    pub nodes: Vec<ListedNode>,
}
