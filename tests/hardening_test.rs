//! Hardening tests added during the v1.0.0 RC audit.
//!
//! These are regression tests for bugs found during adversarial review,
//! plus coverage of config parsing, message serialization, and identity
//! handling. All tests are hermetic: no live Reticulum instance required.

use reticulumchat::crypto::E2ECipher;
use reticulumchat::identity::Identity;
use reticulumchat::messaging::{DeliveryStatus, Message, MessageHistory, OfflineQueue};
use reticulumchat::{migrate_config, Config};

// ============================================================================
// Regression: MessageHistory::with_capacity(0) panicked on add()
// ============================================================================

#[test]
fn test_history_zero_capacity_does_not_panic() {
    let mut history = MessageHistory::with_capacity(0);
    history.add(Message::new("alice", "bob", "hello"));
    assert_eq!(history.len(), 0);
    assert!(history.is_empty());
    assert!(history.get(uuid::Uuid::new_v4()).is_none());
}

#[test]
fn test_history_capacity_one_evicts_oldest() {
    let mut history = MessageHistory::with_capacity(1);
    let first = Message::new("alice", "bob", "first");
    let second = Message::new("alice", "bob", "second");
    history.add(first.clone());
    history.add(second.clone());
    assert_eq!(history.len(), 1);
    assert!(history.get(first.id).is_none());
    assert!(history.get(second.id).is_some());
}

// ============================================================================
// Regression: thread_root() overflowed the stack on parent cycles
// ============================================================================

#[test]
fn test_thread_root_self_cycle_terminates() {
    let mut history = MessageHistory::new();
    // A malformed message that is its own parent
    let msg = Message::new("alice", "bob", "loopy");
    let id = msg.id;
    let msg = msg.with_parent(id);
    history.add(msg);
    // Must terminate; the message itself is the effective root
    let root = history.thread_root(id).unwrap();
    assert_eq!(root.id, id);
}

#[test]
fn test_thread_root_two_node_cycle_terminates() {
    let mut history = MessageHistory::new();
    let a = Message::new("alice", "bob", "msg a");
    let b = Message::new("bob", "alice", "msg b").with_parent(a.id);
    let a = a.with_parent(b.id); // cycle a <-> b
    let a_id = a.id;
    let b_id = b.id;
    history.add(a);
    history.add(b);
    // Must terminate and return one of the two messages
    let root_a = history.thread_root(a_id).unwrap();
    let root_b = history.thread_root(b_id).unwrap();
    assert!(root_a.id == a_id || root_a.id == b_id);
    assert!(root_b.id == a_id || root_b.id == b_id);
}

#[test]
fn test_thread_root_missing_parent_terminates() {
    let mut history = MessageHistory::new();
    // Parent ID points to a message not in history
    let msg = Message::new("alice", "bob", "orphan").with_parent(uuid::Uuid::new_v4());
    let id = msg.id;
    history.add(msg);
    let root = history.thread_root(id).unwrap();
    assert_eq!(root.id, id);
}

#[test]
fn test_thread_root_unknown_id_returns_none() {
    let history = MessageHistory::new();
    assert!(history.thread_root(uuid::Uuid::new_v4()).is_none());
}

// ============================================================================
// Regression: v0 config (no version field) failed to parse, so the
// config-migrate command could never actually migrate anything
// ============================================================================

#[test]
fn test_config_v0_without_version_parses_and_migrates() {
    let json = r#"{
        "identity_path": "~/.reticulumchat/identity",
        "reticulum_host": "127.0.0.1",
        "reticulum_port": 3742,
        "enable_notifications": true,
        "enable_encryption": true
    }"#;
    let config: Config = serde_json::from_str(json).expect("v0 config should parse");
    assert_eq!(config.version, 0);

    let (migrated, result) = migrate_config(config).unwrap();
    assert_eq!(migrated.version, 1);
    assert_eq!(result.from_version, 0);
    assert_eq!(result.to_version, 1);
    assert!(!result.is_noop());
    assert!(!result.changes.is_empty());
}

#[test]
fn test_config_partial_fields_use_defaults() {
    let json = r#"{ "reticulum_port": 4242 }"#;
    let config: Config = serde_json::from_str(json).expect("partial config should parse");
    assert_eq!(config.reticulum_port, 4242);
    // Everything else falls back to Config::default()
    assert_eq!(config.reticulum_host, "127.0.0.1");
    assert_eq!(config.identity_path, "~/.reticulumchat/identity");
    assert!(config.enable_notifications);
    assert!(config.enable_encryption);
}

#[test]
fn test_config_current_version_migration_is_noop() {
    let config = Config::default();
    let (_, result) = migrate_config(config).unwrap();
    assert!(result.is_noop());
    assert!(result.changes.is_empty());
}

#[test]
fn test_config_corrupt_json_errors_cleanly() {
    let garbage = r#"{ this is not json "#;
    let result: Result<Config, _> = serde_json::from_str(garbage);
    assert!(result.is_err());

    let wrong_types = r#"{ "reticulum_port": "not a number" }"#;
    let result: Result<Config, _> = serde_json::from_str(wrong_types);
    assert!(result.is_err());
}

#[test]
fn test_config_roundtrip() {
    let config = Config::default();
    let json = serde_json::to_string_pretty(&config).unwrap();
    let parsed: Config = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.version, config.version);
    assert_eq!(parsed.reticulum_port, config.reticulum_port);
    assert_eq!(parsed.reticulum_host, config.reticulum_host);
}

// ============================================================================
// Message serialization
// ============================================================================

#[test]
fn test_message_serialization_roundtrip() {
    let parent = Message::new("alice", "bob", "parent");
    let msg = Message::new("bob", "alice", "reply @alice!")
        .with_parent(parent.id)
        .with_encryption(true);

    let json = serde_json::to_string(&msg).unwrap();
    let parsed: Message = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.id, msg.id);
    assert_eq!(parsed.sender, "bob");
    assert_eq!(parsed.parent_id, Some(parent.id));
    assert!(parsed.encrypted);
    assert_eq!(parsed.delivery_status, DeliveryStatus::Pending);
    assert_eq!(parsed.mentions, vec!["alice".to_string()]);
}

#[test]
fn test_message_malformed_json_errors_cleanly() {
    let result: Result<Message, _> = serde_json::from_str(r#"{"id": 123}"#);
    assert!(result.is_err());
}

// ============================================================================
// Identity handling
// ============================================================================

#[test]
fn test_identity_serialization_roundtrip() {
    let identity = Identity::generate("alice").unwrap();
    let json = serde_json::to_string_pretty(&identity).unwrap();
    let parsed: Identity = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.name, identity.name);
    assert_eq!(parsed.private_key, identity.private_key);
    assert_eq!(parsed.public_key, identity.public_key);
    assert_eq!(parsed.address, identity.address);
    // The parsed identity must still yield a usable age identity
    assert!(parsed.age_identity().is_ok());
}

#[test]
fn test_identity_garbage_private_key_errors_not_panics() {
    let identity = Identity {
        name: "mallory".to_string(),
        private_key: "definitely-not-an-age-key".to_string(),
        public_key: "age1qqqqqq".to_string(),
        address: "age1qqqqqq".to_string(),
    };
    assert!(identity.age_identity().is_err());
}

#[test]
fn test_identity_malformed_json_errors_cleanly() {
    let result: Result<Identity, _> = serde_json::from_str(r#"{"name": 42}"#);
    assert!(result.is_err());
}

#[test]
fn test_identity_display_format() {
    let identity = Identity::generate("alice").unwrap();
    let display = format!("{}", identity);
    assert!(display.starts_with("alice@"));
    assert!(display.contains(&identity.address));
}

// ============================================================================
// Crypto robustness (no panics on malformed input)
// ============================================================================

#[test]
fn test_decrypt_garbage_ciphertext_errors_not_panics() {
    let identity = Identity::generate("alice").unwrap();
    let age_id = identity.age_identity().unwrap();
    assert!(E2ECipher::decrypt("not an armored age file", &age_id).is_err());
    assert!(E2ECipher::decrypt("", &age_id).is_err());
    assert!(E2ECipher::decrypt(
        "-----BEGIN AGE ENCRYPTED FILE-----\n-----END AGE ENCRYPTED FILE-----",
        &age_id
    )
    .is_err());
}

#[test]
fn test_decrypt_with_wrong_identity_fails() {
    let alice = Identity::generate("alice").unwrap();
    let eve = Identity::generate("eve").unwrap();
    let ciphertext = E2ECipher::encrypt("secret", &alice.public_key).unwrap();
    assert!(E2ECipher::decrypt(&ciphertext, &eve.age_identity().unwrap()).is_err());
}

#[test]
fn test_encrypt_with_garbage_recipient_key_errors() {
    assert!(E2ECipher::encrypt("hello", "not-a-key").is_err());
    assert!(E2ECipher::encrypt("hello", "").is_err());
}

// ============================================================================
// Offline queue edge cases
// ============================================================================

#[test]
fn test_offline_queue_zero_capacity_rejects() {
    let mut queue = OfflineQueue::with_capacity(0);
    assert!(queue.enqueue(Message::new("a", "b", "x")).is_err());
    assert!(queue.is_empty());
}

#[test]
fn test_offline_queue_default_matches_new() {
    let queue = OfflineQueue::default();
    assert_eq!(queue.len(), 0);
    assert!(queue.is_empty());
}
