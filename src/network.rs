use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// The quality level of a network path
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PathQuality {
    Excellent,
    Good,
    Fair,
    Poor,
    Dead,
}

impl PathQuality {
    /// Get a human-readable label for the quality level
    pub fn label(&self) -> &'static str {
        match self {
            PathQuality::Excellent => "Excellent",
            PathQuality::Good => "Good",
            PathQuality::Fair => "Fair",
            PathQuality::Poor => "Poor",
            PathQuality::Dead => "Dead",
        }
    }

    /// Get a color code for terminal display (ratatui Color)
    pub fn color(&self) -> u8 {
        match self {
            PathQuality::Excellent => 2,  // Green
            PathQuality::Good => 10,      // Light green
            PathQuality::Fair => 3,       // Yellow
            PathQuality::Poor => 1,       // Red
            PathQuality::Dead => 8,       // Gray
        }
    }

    /// Determine quality from latency and packet loss
    pub fn from_metrics(latency_ms: f64, packet_loss_percent: f64) -> Self {
        if packet_loss_percent > 50.0 || latency_ms > 5000.0 {
            PathQuality::Dead
        } else if packet_loss_percent > 20.0 || latency_ms > 2000.0 {
            PathQuality::Poor
        } else if packet_loss_percent > 5.0 || latency_ms > 500.0 {
            PathQuality::Fair
        } else if packet_loss_percent > 1.0 || latency_ms > 100.0 {
            PathQuality::Good
        } else {
            PathQuality::Excellent
        }
    }
}

/// A network path to a destination node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPath {
    pub id: Uuid,
    /// The destination node this path leads to
    pub destination_hash: String,
    /// Number of hops to reach the destination
    pub hop_count: u32,
    /// Round-trip time in milliseconds
    pub latency_ms: f64,
    /// Packet loss percentage (0.0 - 100.0)
    pub packet_loss_percent: f64,
    /// When this path was last measured
    pub last_measured: DateTime<Utc>,
    /// Whether this path is currently active
    pub active: bool,
    /// Whether this is a redundant/backup path
    pub is_redundant: bool,
    /// The preferred path for this destination
    pub is_preferred: bool,
    /// Bytes sent via this path
    pub bytes_sent: u64,
    /// Bytes received via this path
    pub bytes_received: u64,
}

impl NetworkPath {
    pub fn new(destination_hash: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            destination_hash: destination_hash.into(),
            hop_count: 0,
            latency_ms: 0.0,
            packet_loss_percent: 0.0,
            last_measured: Utc::now(),
            active: true,
            is_redundant: false,
            is_preferred: false,
            bytes_sent: 0,
            bytes_received: 0,
        }
    }

    pub fn with_hops(mut self, hops: u32) -> Self {
        self.hop_count = hops;
        self
    }

    pub fn with_latency(mut self, latency_ms: f64) -> Self {
        self.latency_ms = latency_ms;
        self.last_measured = Utc::now();
        self
    }

    pub fn with_packet_loss(mut self, loss_percent: f64) -> Self {
        self.packet_loss_percent = loss_percent;
        self.last_measured = Utc::now();
        self
    }

    pub fn mark_redundant(mut self) -> Self {
        self.is_redundant = true;
        self
    }

    pub fn mark_preferred(mut self) -> Self {
        self.is_preferred = true;
        self
    }

    /// Update path metrics and recalculate quality
    pub fn update_metrics(&mut self, latency_ms: f64, packet_loss_percent: f64) {
        self.latency_ms = latency_ms;
        self.packet_loss_percent = packet_loss_percent;
        self.last_measured = Utc::now();
    }

    /// Get the quality assessment for this path
    pub fn quality(&self) -> PathQuality {
        PathQuality::from_metrics(self.latency_ms, self.packet_loss_percent)
    }

    /// Record bytes sent through this path
    pub fn record_sent(&mut self, bytes: u64) {
        self.bytes_sent += bytes;
    }

    /// Record bytes received through this path
    pub fn record_received(&mut self, bytes: u64) {
        self.bytes_received += bytes;
    }

    /// Check if this path has timed out (no measurement in last 60 seconds)
    pub fn is_stale(&self) -> bool {
        Utc::now().signed_duration_since(self.last_measured) > Duration::seconds(60)
    }

    /// Get total bytes transferred through this path
    pub fn total_bytes(&self) -> u64 {
        self.bytes_sent + self.bytes_received
    }
}

/// The status of a mesh node
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeStatus {
    Online,
    Degraded,
    Offline,
    Unknown,
}

impl NodeStatus {
    pub fn label(&self) -> &'static str {
        match self {
            NodeStatus::Online => "Online",
            NodeStatus::Degraded => "Degraded",
            NodeStatus::Offline => "Offline",
            NodeStatus::Unknown => "Unknown",
        }
    }
}

/// A discovered node in the mesh network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshNode {
    pub hash: String,
    pub name: Option<String>,
    pub status: NodeStatus,
    /// When this node was first discovered
    pub first_seen: DateTime<Utc>,
    /// When this node was last seen
    pub last_seen: DateTime<Utc>,
    /// Available paths to this node
    pub paths: Vec<NetworkPath>,
    /// Whether this node is trusted
    pub trusted: bool,
    /// Node capabilities/features
    pub capabilities: Vec<String>,
    /// Current bandwidth stats for this node
    pub bandwidth: BandwidthStats,
}

impl MeshNode {
    pub fn new(hash: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            hash: hash.into(),
            name: None,
            status: NodeStatus::Unknown,
            first_seen: now,
            last_seen: now,
            paths: Vec::new(),
            trusted: false,
            capabilities: Vec::new(),
            bandwidth: BandwidthStats::new(),
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

    pub fn with_capabilities(mut self, caps: Vec<String>) -> Self {
        self.capabilities = caps;
        self
    }

    /// Update the node's last seen timestamp and status
    pub fn update_seen(&mut self, status: NodeStatus) {
        self.last_seen = Utc::now();
        self.status = status;
    }

    /// Add or update a path to this node
    pub fn add_path(&mut self, path: NetworkPath) {
        if let Some(existing) = self.paths.iter_mut().find(|p| p.id == path.id) {
            *existing = path;
        } else {
            self.paths.push(path);
        }
        self.update_status_from_paths();
    }

    /// Remove a path by ID
    pub fn remove_path(&mut self, path_id: Uuid) {
        self.paths.retain(|p| p.id != path_id);
        self.update_status_from_paths();
    }

    /// Get the best available path (lowest latency, preferred if set)
    pub fn best_path(&self) -> Option<&NetworkPath> {
        self.paths
            .iter()
            .filter(|p| p.active && !p.is_stale())
            .min_by(|a, b| {
                // Prefer preferred paths
                if a.is_preferred && !b.is_preferred {
                    return std::cmp::Ordering::Less;
                }
                if !a.is_preferred && b.is_preferred {
                    return std::cmp::Ordering::Greater;
                }
                // Then sort by latency
                a.latency_ms.partial_cmp(&b.latency_ms).unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// Get all active paths sorted by quality
    pub fn active_paths(&self) -> Vec<&NetworkPath> {
        let mut paths: Vec<&NetworkPath> = self
            .paths
            .iter()
            .filter(|p| p.active && !p.is_stale())
            .collect();
        paths.sort_by(|a, b| {
            let qa = a.quality();
            let qb = b.quality();
            // Sort by quality enum ordinal (Excellent = 0, Dead = 4)
            let ord_a = quality_ordinal(&qa);
            let ord_b = quality_ordinal(&qb);
            ord_a.cmp(&ord_b)
        });
        paths
    }

    /// Get the number of redundant paths available
    pub fn redundant_path_count(&self) -> usize {
        self.paths
            .iter()
            .filter(|p| p.is_redundant && p.active && !p.is_stale())
            .count()
    }

    /// Update node status based on available paths
    fn update_status_from_paths(&mut self) {
        let active_count = self.paths.iter().filter(|p| p.active && !p.is_stale()).count();
        let has_preferred = self.paths.iter().any(|p| p.is_preferred && p.active && !p.is_stale());

        self.status = if active_count == 0 {
            NodeStatus::Offline
        } else if has_preferred {
            NodeStatus::Online
        } else {
            NodeStatus::Degraded
        };
    }

    /// Get how long ago this node was last seen
    pub fn time_since_last_seen(&self) -> Duration {
        Utc::now().signed_duration_since(self.last_seen)
    }

    /// Check if node is stale (not seen in 5 minutes)
    pub fn is_stale(&self) -> bool {
        self.time_since_last_seen() > Duration::minutes(5)
    }

    /// Get display name (name or truncated hash)
    pub fn display_name(&self) -> String {
        self.name
            .clone()
            .unwrap_or_else(|| format!("{:.8}", self.hash))
    }
}

fn quality_ordinal(q: &PathQuality) -> u8 {
    match q {
        PathQuality::Excellent => 0,
        PathQuality::Good => 1,
        PathQuality::Fair => 2,
        PathQuality::Poor => 3,
        PathQuality::Dead => 4,
    }
}

/// Bandwidth usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandwidthStats {
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Bytes sent in the current second (for rate calculation)
    pub current_second_sent: u64,
    /// Bytes received in the current second
    pub current_second_received: u64,
    /// Historical send rates (bytes/sec) for the last 60 seconds
    pub send_history: VecDeque<u64>,
    /// Historical receive rates (bytes/sec) for the last 60 seconds
    pub receive_history: VecDeque<u64>,
    /// When the current second started
    pub current_second_start: DateTime<Utc>,
    /// Peak send rate (bytes/sec)
    pub peak_send_rate: u64,
    /// Peak receive rate (bytes/sec)
    pub peak_receive_rate: u64,
}

impl BandwidthStats {
    pub fn new() -> Self {
        Self {
            bytes_sent: 0,
            bytes_received: 0,
            current_second_sent: 0,
            current_second_received: 0,
            send_history: VecDeque::with_capacity(60),
            receive_history: VecDeque::with_capacity(60),
            current_second_start: Utc::now(),
            peak_send_rate: 0,
            peak_receive_rate: 0,
        }
    }

    /// Record bytes sent
    pub fn record_sent(&mut self, bytes: u64) {
        self.bytes_sent += bytes;
        self.current_second_sent += bytes;
    }

    /// Record bytes received
    pub fn record_received(&mut self, bytes: u64) {
        self.bytes_received += bytes;
        self.current_second_received += bytes;
    }

    /// Tick the stats - should be called once per second
    pub fn tick(&mut self) {
        let now = Utc::now();
        let elapsed = now.signed_duration_since(self.current_second_start);

        if elapsed.num_seconds() >= 1 {
            // Record the past second's rates
            self.send_history.push_back(self.current_second_sent);
            self.receive_history.push_back(self.current_second_received);

            // Update peaks
            if self.current_second_sent > self.peak_send_rate {
                self.peak_send_rate = self.current_second_sent;
            }
            if self.current_second_received > self.peak_receive_rate {
                self.peak_receive_rate = self.current_second_received;
            }

            // Keep only last 60 seconds
            if self.send_history.len() > 60 {
                self.send_history.pop_front();
            }
            if self.receive_history.len() > 60 {
                self.receive_history.pop_front();
            }

            // Reset current second
            self.current_second_sent = 0;
            self.current_second_received = 0;
            self.current_second_start = now;
        }
    }

    /// Get current send rate (bytes/sec) - average over last 5 seconds
    pub fn current_send_rate(&self) -> u64 {
        let samples: Vec<u64> = self.send_history.iter().rev().take(5).copied().collect();
        if samples.is_empty() {
            0
        } else {
            samples.iter().sum::<u64>() / samples.len() as u64
        }
    }

    /// Get current receive rate (bytes/sec) - average over last 5 seconds
    pub fn current_receive_rate(&self) -> u64 {
        let samples: Vec<u64> = self.receive_history.iter().rev().take(5).copied().collect();
        if samples.is_empty() {
            0
        } else {
            samples.iter().sum::<u64>() / samples.len() as u64
        }
    }

    /// Format bytes as human-readable string
    pub fn format_bytes(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit_idx = 0;
        while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
            size /= 1024.0;
            unit_idx += 1;
        }
        format!("{:.2} {}", size, UNITS[unit_idx])
    }

    /// Format rate as human-readable string
    pub fn format_rate(bytes_per_sec: u64) -> String {
        format!("{}/s", Self::format_bytes(bytes_per_sec))
    }

    /// Get total bytes transferred
    pub fn total_bytes(&self) -> u64 {
        self.bytes_sent + self.bytes_received
    }
}

impl Default for BandwidthStats {
    fn default() -> Self {
        Self::new()
    }
}

/// A topology edge representing a connection between two nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyEdge {
    pub from: String,
    pub to: String,
    pub latency_ms: f64,
    pub quality: PathQuality,
}

/// The main mesh network manager
#[derive(Debug, Clone)]
pub struct MeshNetwork {
    /// Discovered nodes indexed by hash
    pub nodes: HashMap<String, MeshNode>,
    /// Our own node hash
    pub local_hash: String,
    /// Local bandwidth stats
    pub local_bandwidth: BandwidthStats,
    /// Network topology edges
    pub topology: Vec<TopologyEdge>,
    /// Whether automatic path redundancy is enabled
    pub auto_redundancy: bool,
    /// Minimum number of paths to maintain per node
    pub min_paths: usize,
    /// Maximum number of paths to maintain per node
    pub max_paths: usize,
    /// Quality threshold below which to seek redundant paths
    pub redundancy_threshold: PathQuality,
    /// Last time redundancy was checked
    pub last_redundancy_check: DateTime<Utc>,
}

impl MeshNetwork {
    pub fn new(local_hash: impl Into<String>) -> Self {
        Self {
            nodes: HashMap::new(),
            local_hash: local_hash.into(),
            local_bandwidth: BandwidthStats::new(),
            topology: Vec::new(),
            auto_redundancy: true,
            min_paths: 2,
            max_paths: 4,
            redundancy_threshold: PathQuality::Fair,
            last_redundancy_check: Utc::now(),
        }
    }

    /// Discover or update a node
    pub fn discover_node(&mut self, hash: impl Into<String>) -> &mut MeshNode {
        let hash = hash.into();
        let now = Utc::now();
        
        self.nodes.entry(hash.clone()).or_insert_with(|| {
            let mut node = MeshNode::new(&hash);
            node.first_seen = now;
            node
        })
    }

    /// Get a node by hash
    pub fn get_node(&self, hash: &str) -> Option<&MeshNode> {
        self.nodes.get(hash)
    }

    /// Get a mutable node by hash
    pub fn get_node_mut(&mut self, hash: &str) -> Option<&mut MeshNode> {
        self.nodes.get_mut(hash)
    }

    /// Remove a node
    pub fn remove_node(&mut self, hash: &str) {
        self.nodes.remove(hash);
        self.topology.retain(|e| e.from != hash && e.to != hash);
    }

    /// Add a path to a node
    pub fn add_path(&mut self, node_hash: &str, path: NetworkPath) -> Result<()> {
        let node = self.nodes.get_mut(node_hash)
            .ok_or_else(|| anyhow::anyhow!("Node not found: {}", node_hash))?;
        node.add_path(path);
        Ok(())
    }

    /// Remove a path from a node
    pub fn remove_path(&mut self, node_hash: &str, path_id: Uuid) -> Result<()> {
        let node = self.nodes.get_mut(node_hash)
            .ok_or_else(|| anyhow::anyhow!("Node not found: {}", node_hash))?;
        node.remove_path(path_id);
        Ok(())
    }

    /// Record local bandwidth usage
    pub fn record_bandwidth(&mut self, sent: u64, received: u64) {
        self.local_bandwidth.record_sent(sent);
        self.local_bandwidth.record_received(received);
    }

    /// Tick bandwidth stats (should be called once per second)
    pub fn tick_bandwidth(&mut self) {
        self.local_bandwidth.tick();
        for node in self.nodes.values_mut() {
            node.bandwidth.tick();
        }
    }

    /// Get all nodes sorted by name/hash
    pub fn sorted_nodes(&self) -> Vec<&MeshNode> {
        let mut nodes: Vec<&MeshNode> = self.nodes.values().collect();
        nodes.sort_by(|a, b| {
            let a_name = a.display_name();
            let b_name = b.display_name();
            a_name.cmp(&b_name)
        });
        nodes
    }

    /// Get online nodes
    pub fn online_nodes(&self) -> Vec<&MeshNode> {
        self.nodes
            .values()
            .filter(|n| n.status == NodeStatus::Online || n.status == NodeStatus::Degraded)
            .collect()
    }

    /// Get the total number of active paths
    pub fn total_active_paths(&self) -> usize {
        self.nodes
            .values()
            .map(|n| n.paths.iter().filter(|p| p.active && !p.is_stale()).count())
            .sum()
    }

    /// Get the total number of redundant paths
    pub fn total_redundant_paths(&self) -> usize {
        self.nodes
            .values()
            .map(|n| n.redundant_path_count())
            .sum()
    }

    /// Check and maintain path redundancy for all nodes
    pub fn check_redundancy(&mut self) {
        if !self.auto_redundancy {
            return;
        }

        self.last_redundancy_check = Utc::now();
        let node_hashes: Vec<String> = self.nodes.keys().cloned().collect();

        for hash in node_hashes {
            if let Some(node) = self.nodes.get(&hash) {
                let active_paths = node.paths.iter().filter(|p| p.active && !p.is_stale()).count();
                let has_poor_quality = node.paths.iter().any(|p| {
                    p.active
                        && !p.is_stale()
                        && quality_ordinal(&p.quality()) >= quality_ordinal(&self.redundancy_threshold)
                });

                // Need more paths if below minimum or quality is poor
                if active_paths < self.min_paths || has_poor_quality {
                    // In a real implementation, this would trigger path discovery
                    // For now, we just mark that redundancy is needed
                    tracing::debug!(
                        "Node {} needs path redundancy ({} paths, poor quality: {})",
                        hash,
                        active_paths,
                        has_poor_quality
                    );
                }

                // Trim excess paths
                if active_paths > self.max_paths {
                    if let Some(node) = self.nodes.get_mut(&hash) {
                        // Sort by quality (worst first) and trim
                        let mut paths_with_idx: Vec<(usize, &NetworkPath)> =
                            node.paths.iter().enumerate().collect();
                        paths_with_idx.sort_by(|(_, a), (_, b)| {
                            let qa = a.quality();
                            let qb = b.quality();
                            quality_ordinal(&qb).cmp(&quality_ordinal(&qa))
                        });

                        let to_remove = active_paths - self.max_paths;
                        let ids_to_remove: Vec<Uuid> = paths_with_idx
                            .iter()
                            .take(to_remove)
                            .map(|(_, p)| p.id)
                            .collect();

                        for id in ids_to_remove {
                            node.remove_path(id);
                        }
                    }
                }
            }
        }
    }

    /// Build topology edges from node paths
    pub fn rebuild_topology(&mut self) {
        self.topology.clear();
        for (hash, node) in &self.nodes {
            for path in &node.paths {
                if path.active && !path.is_stale() {
                    self.topology.push(TopologyEdge {
                        from: self.local_hash.clone(),
                        to: hash.clone(),
                        latency_ms: path.latency_ms,
                        quality: path.quality(),
                    });
                }
            }
        }
    }

    /// Get a simple ASCII visualization of the mesh topology
    pub fn visualize_topology(&self) -> String {
        let mut output = String::new();
        output.push_str("Mesh Network Topology\n");
        output.push_str("====================\n\n");

        // Local node
        output.push_str(&format!("[{}] (local)\n", self.local_hash.chars().take(8).collect::<String>()));

        let nodes = self.sorted_nodes();
        if nodes.is_empty() {
            output.push_str("  No discovered nodes\n");
            return output;
        }

        for node in nodes {
            let status_icon = match node.status {
                NodeStatus::Online => "●",
                NodeStatus::Degraded => "◐",
                NodeStatus::Offline => "○",
                NodeStatus::Unknown => "?",
            };

            let name = node.display_name();
            output.push_str(&format!("  ├── {} {} ", status_icon, name));

            if let Some(best) = node.best_path() {
                let quality_label = best.quality().label();
                output.push_str(&format!(
                    "[{}ms, {} hops, {}]",
                    best.latency_ms as u32,
                    best.hop_count,
                    quality_label
                ));
            }
            output.push('\n');

            // Show redundant paths
            let redundant = node.paths.iter().filter(|p| p.is_redundant && p.active).count();
            if redundant > 0 {
                output.push_str(&format!("  │     └─ {} redundant path(s)\n", redundant));
            }
        }

        output
    }

    /// Generate bandwidth usage report
    pub fn bandwidth_report(&self) -> String {
        let mut output = String::new();
        output.push_str("Bandwidth Usage Report\n");
        output.push_str("=====================\n\n");

        output.push_str(&format!(
            "Total Sent:     {}\n",
            BandwidthStats::format_bytes(self.local_bandwidth.bytes_sent)
        ));
        output.push_str(&format!(
            "Total Received: {}\n",
            BandwidthStats::format_bytes(self.local_bandwidth.bytes_received)
        ));
        output.push_str(&format!(
            "Current Send:   {}\n",
            BandwidthStats::format_rate(self.local_bandwidth.current_send_rate())
        ));
        output.push_str(&format!(
            "Current Recv:   {}\n",
            BandwidthStats::format_rate(self.local_bandwidth.current_receive_rate())
        ));
        output.push_str(&format!(
            "Peak Send:      {}\n",
            BandwidthStats::format_rate(self.local_bandwidth.peak_send_rate)
        ));
        output.push_str(&format!(
            "Peak Recv:      {}\n",
            BandwidthStats::format_rate(self.local_bandwidth.peak_receive_rate)
        ));

        output.push_str("\nPer-Node Bandwidth:\n");
        for node in self.sorted_nodes() {
            output.push_str(&format!(
                "  {}: sent={}, recv={}\n",
                node.display_name(),
                BandwidthStats::format_bytes(node.bandwidth.bytes_sent),
                BandwidthStats::format_bytes(node.bandwidth.bytes_received)
            ));
        }

        output
    }

    /// Get network summary statistics
    pub fn network_summary(&self) -> NetworkSummary {
        NetworkSummary {
            total_nodes: self.nodes.len(),
            online_nodes: self.online_nodes().len(),
            total_paths: self.total_active_paths(),
            redundant_paths: self.total_redundant_paths(),
            total_bytes_sent: self.local_bandwidth.bytes_sent,
            total_bytes_received: self.local_bandwidth.bytes_received,
            current_send_rate: self.local_bandwidth.current_send_rate(),
            current_receive_rate: self.local_bandwidth.current_receive_rate(),
        }
    }

    /// Prune stale nodes and paths
    pub fn prune_stale(&mut self) {
        let stale_hashes: Vec<String> = self
            .nodes
            .iter()
            .filter(|(_, node)| node.is_stale())
            .map(|(hash, _)| hash.clone())
            .collect();

        for hash in stale_hashes {
            self.nodes.remove(&hash);
        }

        // Also prune stale paths from remaining nodes
        for node in self.nodes.values_mut() {
            let stale_path_ids: Vec<Uuid> = node
                .paths
                .iter()
                .filter(|p| p.is_stale())
                .map(|p| p.id)
                .collect();
            for id in stale_path_ids {
                node.remove_path(id);
            }
        }

        self.rebuild_topology();
    }
}

/// Summary statistics for the network
#[derive(Debug, Clone)]
pub struct NetworkSummary {
    pub total_nodes: usize,
    pub online_nodes: usize,
    pub total_paths: usize,
    pub redundant_paths: usize,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
    pub current_send_rate: u64,
    pub current_receive_rate: u64,
}

impl NetworkSummary {
    pub fn format(&self) -> String {
        format!(
            "Nodes: {}/{} online | Paths: {} ({} redundant) | Bandwidth: ↓{} ↑{}",
            self.online_nodes,
            self.total_nodes,
            self.total_paths,
            self.redundant_paths,
            BandwidthStats::format_rate(self.current_receive_rate),
            BandwidthStats::format_rate(self.current_send_rate)
        )
    }
}

/// Shared mesh network state
pub type SharedMeshNetwork = Arc<RwLock<MeshNetwork>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_quality_from_metrics() {
        assert_eq!(
            PathQuality::from_metrics(10.0, 0.0),
            PathQuality::Excellent
        );
        assert_eq!(
            PathQuality::from_metrics(50.0, 2.0),
            PathQuality::Good
        );
        assert_eq!(
            PathQuality::from_metrics(200.0, 10.0),
            PathQuality::Fair
        );
        assert_eq!(
            PathQuality::from_metrics(5001.0, 0.0),
            PathQuality::Dead
        );
    }

    #[test]
    fn test_mesh_node_path_management() {
        let mut node = MeshNode::new("test_hash");
        
        let path1 = NetworkPath::new("test_hash").with_hops(1).with_latency(50.0);
        let path2 = NetworkPath::new("test_hash").with_hops(2).with_latency(100.0);
        
        node.add_path(path1.clone());
        node.add_path(path2.clone());
        
        assert_eq!(node.paths.len(), 2);
        assert_eq!(node.best_path().unwrap().latency_ms, 50.0);
    }

    #[test]
    fn test_bandwidth_stats() {
        let mut stats = BandwidthStats::new();
        stats.record_sent(1000);
        stats.record_received(500);
        
        assert_eq!(stats.bytes_sent, 1000);
        assert_eq!(stats.bytes_received, 500);
        assert_eq!(stats.current_second_sent, 1000);
        assert_eq!(stats.current_second_received, 500);
        
        assert!(stats.send_history.is_empty());
    }

    #[test]
    fn test_mesh_network_discovery() {
        let mut network = MeshNetwork::new("local");
        
        {
            let node = network.discover_node("node1");
            node.name = Some("Test Node".to_string());
            node.trusted = true;
        }
        
        assert_eq!(network.nodes.len(), 1);
        assert!(network.get_node("node1").unwrap().trusted);
        assert_eq!(network.get_node("node1").unwrap().display_name(), "Test Node");
    }

    #[test]
    fn test_path_redundancy() {
        let mut network = MeshNetwork::new("local");
        network.min_paths = 2;
        network.max_paths = 3;
        
        let node = network.discover_node("node1");
        node.add_path(NetworkPath::new("node1").with_hops(1).with_latency(50.0));
        node.add_path(NetworkPath::new("node1").with_hops(2).with_latency(100.0).mark_redundant());
        
        assert_eq!(network.total_redundant_paths(), 1);
        
        network.check_redundancy();
        // Should not remove paths since we're at minimum
        assert_eq!(network.get_node("node1").unwrap().paths.len(), 2);
    }

    #[test]
    fn test_bandwidth_formatting() {
        assert_eq!(BandwidthStats::format_bytes(512), "512.00 B");
        assert_eq!(BandwidthStats::format_bytes(1536), "1.50 KB");
        assert_eq!(BandwidthStats::format_bytes(1024 * 1024), "1.00 MB");
    }

    #[test]
    fn test_topology_rebuild() {
        let mut network = MeshNetwork::new("local");
        
        let node = network.discover_node("node1");
        node.add_path(NetworkPath::new("node1").with_hops(1).with_latency(50.0));
        
        network.rebuild_topology();
        
        assert_eq!(network.topology.len(), 1);
        assert_eq!(network.topology[0].to, "node1");
    }

    #[test]
    fn test_prune_stale() {
        let mut network = MeshNetwork::new("local");
        
        let node = network.discover_node("node1");
        node.last_seen = Utc::now() - Duration::minutes(10); // Stale
        
        network.prune_stale();
        
        assert!(network.get_node("node1").is_none());
    }
}
