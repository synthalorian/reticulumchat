use reticulumchat::{
    messaging::{
        DeliveryConfirmation, DeliveryStatus, Message, MessageHistory, MessagingService,
        OfflineQueue,
    },
    network::{BandwidthStats, MeshNetwork, MeshNode, NetworkPath, PathQuality},
    reticulum::{Destination, ReticulumClient},
};

// ============================================================================
// Network & Mesh Integration Tests
// ============================================================================

#[test]
fn test_mesh_network_full_lifecycle() {
    let mut network = MeshNetwork::new("local_node");
    
    // Discover multiple nodes
    let node1 = network.discover_node("node1");
    node1.name = Some("Alice".to_string());
    node1.trusted = true;
    
    let node2 = network.discover_node("node2");
    node2.name = Some("Bob".to_string());
    
    let node3 = network.discover_node("node3");
    node3.name = Some("Charlie".to_string());
    
    assert_eq!(network.nodes.len(), 3);
    
    // Add paths to nodes
    network.add_path("node1", NetworkPath::new("node1").with_hops(1).with_latency(50.0)).unwrap();
    network.add_path("node1", NetworkPath::new("node1").with_hops(2).with_latency(100.0).mark_redundant()).unwrap();
    
    network.add_path("node2", NetworkPath::new("node2").with_hops(1).with_latency(75.0)).unwrap();
    
    network.add_path("node3", NetworkPath::new("node3").with_hops(3).with_latency(250.0)).unwrap();
    
    // Verify topology
    network.rebuild_topology();
    assert_eq!(network.topology.len(), 4);
    
    // Check redundancy
    assert_eq!(network.total_redundant_paths(), 1);
    
    // Check sorted nodes
    let sorted = network.sorted_nodes();
    assert_eq!(sorted.len(), 3);
    assert_eq!(sorted[0].display_name(), "Alice");
    
    // Network summary
    let summary = network.network_summary();
    assert_eq!(summary.total_nodes, 3);
    assert_eq!(summary.total_paths, 4);
    assert_eq!(summary.redundant_paths, 1);
    
    // Remove a node
    network.remove_node("node2");
    assert_eq!(network.nodes.len(), 2);
    assert!(network.get_node("node2").is_none());
}

#[test]
fn test_path_quality_edge_cases() {
    // Test exact boundary conditions
    // Excellent: <=1% loss AND <=100ms latency
    assert_eq!(PathQuality::from_metrics(10.0, 0.0), PathQuality::Excellent);
    assert_eq!(PathQuality::from_metrics(100.0, 1.0), PathQuality::Excellent);
    assert_eq!(PathQuality::from_metrics(100.1, 0.5), PathQuality::Good);
    assert_eq!(PathQuality::from_metrics(50.0, 1.1), PathQuality::Good);
    
    // Good: <=5% loss AND <=500ms latency (but above Excellent thresholds)
    assert_eq!(PathQuality::from_metrics(500.0, 5.0), PathQuality::Good);
    assert_eq!(PathQuality::from_metrics(500.1, 2.0), PathQuality::Fair);
    assert_eq!(PathQuality::from_metrics(200.0, 5.1), PathQuality::Fair);
    
    // Fair: <=20% loss AND <=2000ms latency
    assert_eq!(PathQuality::from_metrics(2000.0, 20.0), PathQuality::Fair);
    assert_eq!(PathQuality::from_metrics(2000.1, 10.0), PathQuality::Poor);
    assert_eq!(PathQuality::from_metrics(1000.0, 20.1), PathQuality::Poor);
    
    // Poor: <=50% loss AND <=5000ms latency
    assert_eq!(PathQuality::from_metrics(5000.0, 50.0), PathQuality::Poor);
    assert_eq!(PathQuality::from_metrics(5000.1, 30.0), PathQuality::Dead);
    assert_eq!(PathQuality::from_metrics(3000.0, 50.1), PathQuality::Dead);
    
    // Test zero metrics
    assert_eq!(PathQuality::from_metrics(0.0, 0.0), PathQuality::Excellent);
}

#[test]
fn test_mesh_node_path_selection() {
    let mut node = MeshNode::new("test_node");
    
    // Add paths with different qualities
    let path1 = NetworkPath::new("test_node")
        .with_hops(1)
        .with_latency(10.0)
        .with_packet_loss(0.0)
        .mark_preferred();
    
    let path2 = NetworkPath::new("test_node")
        .with_hops(2)
        .with_latency(5.0)
        .with_packet_loss(0.0);
    
    let path3 = NetworkPath::new("test_node")
        .with_hops(3)
        .with_latency(5000.0)
        .with_packet_loss(60.0);
    
    node.add_path(path1.clone());
    node.add_path(path2.clone());
    node.add_path(path3.clone());
    
    // Best path should be preferred path even if not lowest latency
    let best = node.best_path().unwrap();
    assert!(best.is_preferred);
    
    // Active paths include all non-stale paths regardless of quality
    let active = node.active_paths();
    assert_eq!(active.len(), 3);
    
    // Check path quality calculations
    assert_eq!(node.paths[0].quality(), PathQuality::Excellent);
    assert_eq!(node.paths[1].quality(), PathQuality::Excellent);
    assert_eq!(node.paths[2].quality(), PathQuality::Dead);
}

#[test]
fn test_bandwidth_stats_rolling_window() {
    let mut stats = BandwidthStats::new();
    
    for i in 1..=10u64 {
        stats.send_history.push_back(i * 100);
        stats.receive_history.push_back(i * 50);
    }
    
    stats.peak_send_rate = 1000;
    stats.peak_receive_rate = 500;
    
    assert_eq!(stats.send_history.len(), 10);
    assert_eq!(stats.receive_history.len(), 10);
    
    let expected_send_rate = (6 + 7 + 8 + 9 + 10) * 100 / 5;
    let expected_recv_rate = (6 + 7 + 8 + 9 + 10) * 50 / 5;
    
    assert_eq!(stats.current_send_rate(), expected_send_rate);
    assert_eq!(stats.current_receive_rate(), expected_recv_rate);
    
    stats.record_sent(1000);
    stats.record_received(500);
    assert_eq!(stats.bytes_sent, 1000);
    assert_eq!(stats.bytes_received, 500);
    assert_eq!(stats.total_bytes(), 1500);
}

#[test]
fn test_bandwidth_formatting() {
    assert_eq!(BandwidthStats::format_bytes(0), "0.00 B");
    assert_eq!(BandwidthStats::format_bytes(1), "1.00 B");
    assert_eq!(BandwidthStats::format_bytes(1023), "1023.00 B");
    assert_eq!(BandwidthStats::format_bytes(1024), "1.00 KB");
    assert_eq!(BandwidthStats::format_bytes(1536), "1.50 KB");
    assert_eq!(BandwidthStats::format_bytes(1024 * 1024), "1.00 MB");
    assert_eq!(BandwidthStats::format_bytes(1024 * 1024 * 1024), "1.00 GB");
}

#[test]
fn test_network_path_updates() {
    let mut path = NetworkPath::new("dest").with_hops(2).with_latency(100.0);
    
    path.update_metrics(50.0, 2.0);
    assert_eq!(path.latency_ms, 50.0);
    assert_eq!(path.packet_loss_percent, 2.0);
    assert_eq!(path.quality(), PathQuality::Good);
    
    path.record_sent(1000);
    path.record_received(500);
    assert_eq!(path.bytes_sent, 1000);
    assert_eq!(path.bytes_received, 500);
    assert_eq!(path.total_bytes(), 1500);
    
    // Path should not be stale immediately
    assert!(!path.is_stale());
}

#[test]
fn test_mesh_network_redundancy_check() {
    let mut network = MeshNetwork::new("local");
    network.min_paths = 2;
    network.max_paths = 3;
    network.auto_redundancy = true;
    
    {
        let node = network.discover_node("node1");
        node.add_path(NetworkPath::new("node1").with_hops(1).with_latency(50.0));
    }
    
    // Only 1 path, below minimum of 2
    network.check_redundancy();
    
    {
        let node = network.get_node_mut("node1").unwrap();
        // Add another path to meet minimum
        node.add_path(NetworkPath::new("node1").with_hops(2).with_latency(100.0));
        
        // Add excess paths
        node.add_path(NetworkPath::new("node1").with_hops(3).with_latency(3000.0));
        node.add_path(NetworkPath::new("node1").with_hops(4).with_latency(4000.0));
    }
    
    network.check_redundancy();
    
    // Should trim to max_paths (3), removing worst quality paths first
    assert!(network.get_node("node1").unwrap().paths.len() <= 3);
}

// ============================================================================
// Messaging Integration Tests
// ============================================================================

#[test]
fn test_message_history_comprehensive() {
    let mut history = MessageHistory::new();
    
    // Add messages
    let msg1 = Message::new("alice", "bob", "First message");
    let msg2 = Message::new("bob", "alice", "Second message");
    let msg3 = Message::new("alice", "bob", "Third message with @bob mention");
    
    history.add(msg1.clone());
    history.add(msg2.clone());
    history.add(msg3.clone());
    
    // Search tests
    let results = history.search("message");
    assert_eq!(results.len(), 3);
    
    let results = history.search("First");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].content, "First message");
    
    let results = history.search("nonexistent");
    assert!(results.is_empty());
    
    // Sender search
    let alice_msgs = history.search_by_sender("alice");
    assert_eq!(alice_msgs.len(), 2);
    
    // Mention search
    let mentions = history.search_by_mention("bob");
    assert_eq!(mentions.len(), 1);
    
    // Visible messages
    let visible = history.visible_messages();
    assert_eq!(visible.len(), 3);
}

#[test]
fn test_message_thread_navigation() {
    let mut history = MessageHistory::new();
    
    let parent = Message::new("alice", "general", "Discussion topic");
    history.add(parent.clone());
    
    let reply1 = Message::new("bob", "general", "I agree").with_parent(parent.id);
    let reply2 = Message::new("charlie", "general", "Me too").with_parent(parent.id);
    let nested_reply = Message::new("alice", "general", "Thanks!").with_parent(reply1.id);
    
    history.add(reply1.clone());
    history.add(reply2.clone());
    history.add(nested_reply.clone());
    
    // Thread replies
    let replies = history.thread_replies(parent.id);
    assert_eq!(replies.len(), 2); // Only direct replies
    
    // Thread root from nested reply
    let root = history.thread_root(nested_reply.id).unwrap();
    assert_eq!(root.id, parent.id);
    
    // Thread root from parent itself
    let root = history.thread_root(parent.id).unwrap();
    assert_eq!(root.id, parent.id);
}

#[test]
fn test_offline_queue_capacity() {
    let mut queue = OfflineQueue::with_capacity(5);
    
    // Fill queue to capacity
    for i in 0..5 {
        let msg = Message::new("alice", "bob", format!("message {}", i));
        queue.enqueue(msg).unwrap();
    }
    
    assert_eq!(queue.len(), 5);
    
    // Should fail when full
    let msg = Message::new("alice", "bob", "overflow");
    assert!(queue.enqueue(msg).is_err());
    
    // Remove by ID
    let first_id = queue.peek().unwrap().id;
    let removed = queue.remove(first_id);
    assert!(removed.is_some());
    assert_eq!(queue.len(), 4);
    
    // Remove non-existent
    let removed = queue.remove(uuid::Uuid::new_v4());
    assert!(removed.is_none());
}

#[test]
fn test_message_edit_and_delete_workflow() {
    let mut history = MessageHistory::new();
    
    let msg = Message::new("alice", "bob", "Original content");
    history.add(msg.clone());
    
    // Edit
    history.edit_message(msg.id, "Edited content").unwrap();
    let edited = history.get(msg.id).unwrap();
    assert_eq!(edited.content, "Edited content");
    assert!(edited.edited_at.is_some());
    
    // Delete
    history.delete_message(msg.id).unwrap();
    let deleted = history.get(msg.id).unwrap();
    assert!(deleted.deleted);
    assert!(deleted.content.is_empty());
    
    // Should not appear in search
    let results = history.search("Edited");
    assert!(results.is_empty());
    
    // Should not appear in visible messages
    assert!(history.visible_messages().is_empty());
}

#[test]
fn test_message_pin_workflow() {
    let mut history = MessageHistory::new();
    
    let msg1 = Message::new("alice", "bob", "Important info");
    let msg2 = Message::new("bob", "alice", "Another message");
    let msg3 = Message::new("alice", "bob", "Also important");
    
    history.add(msg1.clone());
    history.add(msg2.clone());
    history.add(msg3.clone());
    
    // Pin messages
    history.pin_message(msg1.id, "alice").unwrap();
    history.pin_message(msg3.id, "bob").unwrap();
    
    let pinned = history.pinned_messages();
    assert_eq!(pinned.len(), 2);
    
    // Unpin one
    history.unpin_message(msg1.id).unwrap();
    let pinned = history.pinned_messages();
    assert_eq!(pinned.len(), 1);
    assert_eq!(pinned[0].id, msg3.id);
    
    // Try to pin non-existent message
    assert!(history.pin_message(uuid::Uuid::new_v4(), "alice").is_err());
}

#[test]
fn test_message_mention_extraction_edge_cases() {
    // Simple mentions
    let mentions = Message::extract_mentions("@alice @bob");
    assert_eq!(mentions, vec!["alice", "bob"]);
    
    // Mention with punctuation
    let mentions = Message::extract_mentions("Hey @alice! How are you?");
    assert_eq!(mentions, vec!["alice"]);
    
    // Multiple mentions in one word (not supported, but shouldn't crash)
    let mentions = Message::extract_mentions("@alice-bob");
    assert_eq!(mentions, vec!["alice-bob"]);
    
    // No mentions
    let mentions = Message::extract_mentions("Hello world");
    assert!(mentions.is_empty());
    
    // Empty mention
    let mentions = Message::extract_mentions("@");
    assert!(mentions.is_empty());
    
    // Case normalization
    let mentions = Message::extract_mentions("@ALICE @Bob");
    assert_eq!(mentions, vec!["alice", "bob"]);
}

#[tokio::test]
async fn test_messaging_service_event_flow() {
    let mut service = MessagingService::with_capacity(10);
    
    let msg = Message::new("alice", "bob", "Hello with @bob mention");
    
    // Receive a message
    service.receive_message(msg.clone()).await.unwrap();
    
    // Queue a message
    service.queue_message(Message::new("alice", "bob", "Queued message")).unwrap();
    assert_eq!(service.offline_queue.len(), 1);
    
    // Confirm delivery
    let confirmation = DeliveryConfirmation::new(msg.id, "bob", DeliveryStatus::Delivered);
    service.confirm_delivery(confirmation).await.unwrap();
    
    // Edit message
    service.edit_message(msg.id, "Edited content").await.unwrap();
    assert_eq!(service.history.get(msg.id).unwrap().content, "Edited content");
    
    // Delete message
    service.delete_message(msg.id).await.unwrap();
    assert!(service.history.get(msg.id).unwrap().deleted);
}

// ============================================================================
// Protocol / Reticulum Integration Tests
// ============================================================================

#[tokio::test]
async fn test_reticulum_client_connection_flow() {
    let mut client = ReticulumClient::new("127.0.0.1", 3742);
    
    // Initial state
    assert!(!client.is_connected());
    
    // Connect
    client.connect().await.expect("connect should succeed");
    assert!(client.is_connected());
    
    // Operations while connected
    let identity = reticulumchat::identity::Identity::generate("test").unwrap();
    client.announce(&identity).await.expect("announce should succeed");
    client.send_packet("dest", b"data").await.expect("send should succeed");
    
    // Disconnect
    client.disconnect().await.expect("disconnect should succeed");
    assert!(!client.is_connected());
}

#[test]
fn test_destination_builder_patterns() {
    // Minimal destination
    let dest = Destination::new("hash1");
    assert_eq!(dest.hash, "hash1");
    assert!(dest.name.is_none());
    assert!(!dest.trusted);
    
    // With name
    let dest = Destination::new("hash2").with_name("Node 2");
    assert_eq!(dest.name, Some("Node 2".to_string()));
    
    // Trusted
    let dest = Destination::new("hash3").trusted();
    assert!(dest.trusted);
    
    // Full builder
    let dest = Destination::new("hash4")
        .with_name("Full Node")
        .trusted();
    assert_eq!(dest.hash, "hash4");
    assert_eq!(dest.name, Some("Full Node".to_string()));
    assert!(dest.trusted);
}

// ============================================================================
// End-to-End Integration Test
// ============================================================================

#[test]
fn test_full_chat_workflow() {
    // Simulate a complete chat workflow
    let mut history = MessageHistory::new();
    let mut network = MeshNetwork::new("local_node");
    
    // Discover contacts as mesh nodes
    network.discover_node("alice").name = Some("Alice".to_string());
    network.discover_node("bob").name = Some("Bob".to_string());
    
    // Alice sends a message
    let msg1 = Message::new("alice", "bob", "Hey @bob, want to chat?");
    history.add(msg1.clone());
    
    // Bob replies
    let msg2 = Message::new("bob", "alice", "Sure! What's up?").with_parent(msg1.id);
    history.add(msg2.clone());
    
    // Alice edits her message
    history.edit_message(msg1.id, "Hey @bob, are you free to chat?").unwrap();
    
    // Pin important message
    history.pin_message(msg2.id, "alice").unwrap();
    
    // Search for messages
    let search_results = history.search("chat");
    assert_eq!(search_results.len(), 1);
    
    // Check mentions
    let mentions = history.mentions_for("bob");
    assert_eq!(mentions.len(), 1);
    
    // Check thread
    let thread = history.thread_replies(msg1.id);
    assert_eq!(thread.len(), 1);
    assert_eq!(thread[0].content, "Sure! What's up?");
    
    // Verify network state
    assert_eq!(network.nodes.len(), 2);
    assert_eq!(network.get_node("alice").unwrap().display_name(), "Alice");
}
