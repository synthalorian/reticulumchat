pub mod crypto;
pub mod gui;
pub mod identity;
pub mod messaging;
pub mod network;
pub mod notification;
pub mod reticulum;
pub mod tui;

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Application configuration shared across modes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub identity_path: String,
    pub reticulum_host: String,
    pub reticulum_port: u16,
    pub enable_notifications: bool,
    pub enable_encryption: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            identity_path: "~/.reticulumchat/identity".to_string(),
            reticulum_host: "127.0.0.1".to_string(),
            reticulum_port: 3742,
            enable_notifications: true,
            enable_encryption: true,
        }
    }
}

/// Shared application state
pub type SharedState = Arc<RwLock<AppState>>;

#[derive(Debug)]
pub struct AppState {
    pub config: Config,
    pub identity: identity::Identity,
    pub message_queue: messaging::OfflineQueue,
    pub network: network::MeshNetwork,
}

impl AppState {
    pub fn new(config: Config, identity: identity::Identity) -> Self {
        let local_hash = identity.destination_hash();
        Self {
            config,
            identity,
            message_queue: messaging::OfflineQueue::new(),
            network: network::MeshNetwork::new(local_hash),
        }
    }
}

/// Application runtime handle
pub struct AppRuntime {
    pub state: SharedState,
}

impl AppRuntime {
    pub fn new(state: SharedState) -> Self {
        Self { state }
    }
}
