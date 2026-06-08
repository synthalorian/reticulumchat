use crate::identity::Identity;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Stub for Reticulum network integration
///
/// In v0.2.0 this will connect to a local Reticulum instance
/// via TCP (default port 3742) or Unix socket and handle
/// destination registration, packet sending, and receipt handling.
#[derive(Debug, Clone)]
pub struct ReticulumClient {
    host: String,
    port: u16,
    connected: bool,
}

impl ReticulumClient {
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
            connected: false,
        }
    }

    pub async fn connect(&mut self) -> Result<()> {
        // TODO(v0.2.0): Implement actual TCP connection to Reticulum instance
        tracing::info!("Connecting to Reticulum at {}:{}", self.host, self.port);
        self.connected = true;
        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        tracing::info!("Disconnecting from Reticulum");
        self.connected = false;
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub async fn announce(&self, _identity: &Identity) -> Result<()> {
        // TODO(v0.2.0): Send announcement packet to Reticulum
        tracing::info!("Announcing identity to Reticulum");
        Ok(())
    }

    pub async fn send_packet(&self, _destination: &str, _data: &[u8]) -> Result<()> {
        // TODO(v0.2.0): Send data packet via Reticulum
        tracing::debug!("Sending packet to destination");
        Ok(())
    }
}

/// A Reticulum destination address
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Destination {
    pub hash: String,
    pub name: Option<String>,
    pub trusted: bool,
}

impl Destination {
    pub fn new(hash: impl Into<String>) -> Self {
        Self {
            hash: hash.into(),
            name: None,
            trusted: false,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn trusted(mut self) -> Self {
        self.trusted = true;
        self
    }
}

/// Link status for Reticulum connections
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkStatus {
    Down,
    Connecting,
    Up,
}
