pub mod crash_reporter;
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
    /// Configuration format version for migration tracking
    pub version: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            identity_path: "~/.reticulumchat/identity".to_string(),
            reticulum_host: "127.0.0.1".to_string(),
            reticulum_port: 3742,
            enable_notifications: true,
            enable_encryption: true,
            version: 1,
        }
    }
}

/// Config migration result indicating what changed
#[derive(Debug, Clone)]
pub struct MigrationResult {
    pub from_version: u32,
    pub to_version: u32,
    pub changes: Vec<String>,
    pub backup_path: Option<String>,
}

impl MigrationResult {
    pub fn new(from: u32, to: u32) -> Self {
        Self {
            from_version: from,
            to_version: to,
            changes: Vec::new(),
            backup_path: None,
        }
    }

    pub fn with_change(mut self, change: impl Into<String>) -> Self {
        self.changes.push(change.into());
        self
    }

    pub fn with_backup(mut self, path: impl Into<String>) -> Self {
        self.backup_path = Some(path.into());
        self
    }

    pub fn is_noop(&self) -> bool {
        self.from_version == self.to_version
    }
}

/// Migrate a config from an older version to the current version
pub fn migrate_config(mut config: Config) -> anyhow::Result<(Config, MigrationResult)> {
    let current_version = 1;
    let mut result = MigrationResult::new(config.version, current_version);

    if config.version == current_version {
        return Ok((config, result));
    }

    if config.version == 0 {
        result = result.with_change("Added version field (was v0)");
        config.version = 1;
    }

    Ok((config, result))
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
