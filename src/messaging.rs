use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use tokio::sync::mpsc;
use uuid::Uuid;

/// The status of a message in transit
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryStatus {
    Pending,
    Sent,
    Delivered,
    Failed,
    Read,
}

/// A chat message with support for editing, deletion, replies, mentions, and pinning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub sender: String,
    pub recipient: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub encrypted: bool,
    pub delivery_status: DeliveryStatus,
    /// When the message was last edited (None if never edited)
    pub edited_at: Option<DateTime<Utc>>,
    /// Whether the message has been deleted
    pub deleted: bool,
    /// ID of the parent message if this is a reply
    pub parent_id: Option<Uuid>,
    /// List of mentioned usernames extracted from content
    pub mentions: Vec<String>,
    /// Whether the message is pinned
    pub pinned: bool,
    /// Who pinned the message
    pub pinned_by: Option<String>,
    /// When the message was pinned
    pub pinned_at: Option<DateTime<Utc>>,
}

impl Message {
    pub fn new(sender: impl Into<String>, recipient: impl Into<String>, content: impl Into<String>) -> Self {
        let content_str = content.into();
        let mentions = Self::extract_mentions(&content_str);
        Self {
            id: Uuid::new_v4(),
            sender: sender.into(),
            recipient: recipient.into(),
            content: content_str,
            timestamp: Utc::now(),
            encrypted: false,
            delivery_status: DeliveryStatus::Pending,
            edited_at: None,
            deleted: false,
            parent_id: None,
            mentions,
            pinned: false,
            pinned_by: None,
            pinned_at: None,
        }
    }

    pub fn with_encryption(mut self, encrypted: bool) -> Self {
        self.encrypted = encrypted;
        self
    }

    pub fn with_parent(mut self, parent_id: Uuid) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    /// Extract @mentions from message content
    pub fn extract_mentions(content: &str) -> Vec<String> {
        let mut mentions = Vec::new();
        for word in content.split_whitespace() {
            if word.starts_with('@') {
                let mention = word.trim_start_matches('@').trim_end_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != '-');
                if !mention.is_empty() {
                    mentions.push(mention.to_lowercase());
                }
            }
        }
        mentions
    }

    /// Edit the message content, updating edited_at and re-extracting mentions
    pub fn edit(&mut self, new_content: impl Into<String>) {
        self.content = new_content.into();
        self.mentions = Self::extract_mentions(&self.content);
        self.edited_at = Some(Utc::now());
    }

    /// Mark the message as deleted
    pub fn mark_deleted(&mut self) {
        self.deleted = true;
        self.content = String::new();
        self.mentions.clear();
    }

    /// Pin the message
    pub fn pin(&mut self, pinned_by: impl Into<String>) {
        self.pinned = true;
        self.pinned_by = Some(pinned_by.into());
        self.pinned_at = Some(Utc::now());
    }

    /// Unpin the message
    pub fn unpin(&mut self) {
        self.pinned = false;
        self.pinned_by = None;
        self.pinned_at = None;
    }
}

/// A delivery confirmation receipt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryConfirmation {
    pub message_id: Uuid,
    pub recipient: String,
    pub status: DeliveryStatus,
    pub timestamp: DateTime<Utc>,
}

impl DeliveryConfirmation {
    pub fn new(message_id: Uuid, recipient: impl Into<String>, status: DeliveryStatus) -> Self {
        Self {
            message_id,
            recipient: recipient.into(),
            status,
            timestamp: Utc::now(),
        }
    }
}

/// An offline message queue for storing messages when the network is unavailable
#[derive(Debug, Clone)]
pub struct OfflineQueue {
    queue: VecDeque<Message>,
    max_size: usize,
}

impl OfflineQueue {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            max_size: 1000,
        }
    }

    pub fn with_capacity(max_size: usize) -> Self {
        Self {
            queue: VecDeque::new(),
            max_size,
        }
    }

    /// Queue a message for later delivery
    pub fn enqueue(&mut self, message: Message) -> Result<()> {
        if self.queue.len() >= self.max_size {
            anyhow::bail!("Offline queue is full (max {} messages)", self.max_size);
        }
        self.queue.push_back(message);
        Ok(())
    }

    /// Take the next message from the queue
    pub fn dequeue(&mut self) -> Option<Message> {
        self.queue.pop_front()
    }

    /// Peek at the next message without removing it
    pub fn peek(&self) -> Option<&Message> {
        self.queue.front()
    }

    /// Number of queued messages
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Get all queued messages
    pub fn messages(&self) -> &VecDeque<Message> {
        &self.queue
    }

    /// Clear all queued messages
    pub fn clear(&mut self) {
        self.queue.clear();
    }

    /// Remove a specific message by ID
    pub fn remove(&mut self, id: Uuid) -> Option<Message> {
        if let Some(pos) = self.queue.iter().position(|m| m.id == id) {
            self.queue.remove(pos)
        } else {
            None
        }
    }
}

/// Message history for a conversation with search and thread support
#[derive(Debug, Clone)]
pub struct MessageHistory {
    messages: Vec<Message>,
    /// Index from message ID to position for O(1) lookups
    id_index: HashMap<Uuid, usize>,
    max_size: usize,
}

impl MessageHistory {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            id_index: HashMap::new(),
            max_size: 10000,
        }
    }

    pub fn with_capacity(max_size: usize) -> Self {
        Self {
            messages: Vec::new(),
            id_index: HashMap::new(),
            max_size,
        }
    }

    /// Add a message to history
    pub fn add(&mut self, message: Message) {
        if self.messages.len() >= self.max_size {
            // Remove oldest message from index
            if let Some(oldest) = self.messages.first() {
                self.id_index.remove(&oldest.id);
            }
            self.messages.remove(0);
        }
        let idx = self.messages.len();
        self.id_index.insert(message.id, idx);
        self.messages.push(message);
        // Rebuild index if we've removed from the front
        self.rebuild_index();
    }

    /// Get a message by ID
    pub fn get(&self, id: Uuid) -> Option<&Message> {
        self.id_index.get(&id).and_then(|&idx| self.messages.get(idx))
    }

    /// Get a mutable message by ID
    pub fn get_mut(&mut self, id: Uuid) -> Option<&mut Message> {
        if let Some(&idx) = self.id_index.get(&id) {
            self.messages.get_mut(idx)
        } else {
            None
        }
    }

    /// Edit a message by ID
    pub fn edit_message(&mut self, id: Uuid, new_content: impl Into<String>) -> Result<()> {
        let msg = self.get_mut(id)
            .ok_or_else(|| anyhow::anyhow!("Message not found: {}", id))?;
        msg.edit(new_content);
        Ok(())
    }

    /// Delete a message by ID
    pub fn delete_message(&mut self, id: Uuid) -> Result<()> {
        let msg = self.get_mut(id)
            .ok_or_else(|| anyhow::anyhow!("Message not found: {}", id))?;
        msg.mark_deleted();
        Ok(())
    }

    /// Pin a message by ID
    pub fn pin_message(&mut self, id: Uuid, pinned_by: impl Into<String>) -> Result<()> {
        let msg = self.get_mut(id)
            .ok_or_else(|| anyhow::anyhow!("Message not found: {}", id))?;
        msg.pin(pinned_by);
        Ok(())
    }

    /// Unpin a message by ID
    pub fn unpin_message(&mut self, id: Uuid) -> Result<()> {
        let msg = self.get_mut(id)
            .ok_or_else(|| anyhow::anyhow!("Message not found: {}", id))?;
        msg.unpin();
        Ok(())
    }

    /// Get all pinned messages, sorted by pin time (newest first)
    pub fn pinned_messages(&self) -> Vec<&Message> {
        let mut pinned: Vec<&Message> = self.messages.iter()
            .filter(|m| m.pinned)
            .collect();
        pinned.sort_by(|a, b| b.pinned_at.cmp(&a.pinned_at));
        pinned
    }

    /// Get replies to a specific message (thread)
    pub fn thread_replies(&self, parent_id: Uuid) -> Vec<&Message> {
        self.messages.iter()
            .filter(|m| m.parent_id == Some(parent_id))
            .collect()
    }

    /// Get the root message of a thread given any message in it
    pub fn thread_root(&self, message_id: Uuid) -> Option<&Message> {
        let msg = self.get(message_id)?;
        if let Some(parent_id) = msg.parent_id {
            self.thread_root(parent_id)
        } else {
            Some(msg)
        }
    }

    /// Search messages by content (case-insensitive)
    pub fn search(&self, query: &str) -> Vec<&Message> {
        let query_lower = query.to_lowercase();
        self.messages.iter()
            .filter(|m| !m.deleted && m.content.to_lowercase().contains(&query_lower))
            .collect()
    }

    /// Search messages by sender
    pub fn search_by_sender(&self, sender: &str) -> Vec<&Message> {
        let sender_lower = sender.to_lowercase();
        self.messages.iter()
            .filter(|m| m.sender.to_lowercase() == sender_lower)
            .collect()
    }

    /// Search messages by mention
    pub fn search_by_mention(&self, username: &str) -> Vec<&Message> {
        let username_lower = username.to_lowercase();
        self.messages.iter()
            .filter(|m| m.mentions.contains(&username_lower))
            .collect()
    }

    /// Get all messages mentioning a specific user
    pub fn mentions_for(&self, username: &str) -> Vec<&Message> {
        self.search_by_mention(username)
    }

    /// Get all messages in chronological order
    pub fn all_messages(&self) -> &[Message] {
        &self.messages
    }

    /// Get messages excluding deleted ones
    pub fn visible_messages(&self) -> Vec<&Message> {
        self.messages.iter()
            .filter(|m| !m.deleted)
            .collect()
    }

    /// Number of messages in history
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Clear all messages
    pub fn clear(&mut self) {
        self.messages.clear();
        self.id_index.clear();
    }

    fn rebuild_index(&mut self) {
        self.id_index.clear();
        for (idx, msg) in self.messages.iter().enumerate() {
            self.id_index.insert(msg.id, idx);
        }
    }
}

impl Default for MessageHistory {
    fn default() -> Self {
        Self::new()
    }
}

/// Events emitted by the messaging system
#[derive(Debug, Clone)]
pub enum MessageEvent {
    MessageReceived(Message),
    DeliveryConfirmed(DeliveryConfirmation),
    QueueUpdated { count: usize },
    MessageEdited { message_id: Uuid, new_content: String },
    MessageDeleted { message_id: Uuid },
    MessagePinned { message_id: Uuid, pinned_by: String },
    MessageUnpinned { message_id: Uuid },
    MentionReceived { message: Message, mentioned_user: String },
}

/// The messaging service handles sending, receiving, queuing, and confirmations
pub struct MessagingService {
    pub event_tx: mpsc::Sender<MessageEvent>,
    pub event_rx: mpsc::Receiver<MessageEvent>,
    pub offline_queue: OfflineQueue,
    pub history: MessageHistory,
}

impl MessagingService {
    pub fn new() -> Self {
        let (event_tx, event_rx) = mpsc::channel(100);
        Self {
            event_tx,
            event_rx,
            offline_queue: OfflineQueue::new(),
            history: MessageHistory::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let (event_tx, event_rx) = mpsc::channel(100);
        Self {
            event_tx,
            event_rx,
            offline_queue: OfflineQueue::with_capacity(capacity),
            history: MessageHistory::with_capacity(capacity),
        }
    }

    /// Queue a message for delivery
    pub fn queue_message(&mut self, message: Message) -> Result<()> {
        self.offline_queue.enqueue(message.clone())?;
        self.history.add(message);
        let _ = self.event_tx.try_send(MessageEvent::QueueUpdated {
            count: self.offline_queue.len(),
        });
        Ok(())
    }

    /// Receive a message and emit appropriate events
    pub async fn receive_message(&mut self, message: Message) -> Result<()> {
        // Emit mention events for each mentioned user
        for mention in &message.mentions {
            let _ = self.event_tx.send(MessageEvent::MentionReceived {
                message: message.clone(),
                mentioned_user: mention.clone(),
            }).await;
        }
        
        self.history.add(message.clone());
        self.event_tx.send(MessageEvent::MessageReceived(message)).await?;
        Ok(())
    }

    /// Process delivery confirmation
    pub async fn confirm_delivery(&self, confirmation: DeliveryConfirmation) -> Result<()> {
        self.event_tx
            .send(MessageEvent::DeliveryConfirmed(confirmation))
            .await?;
        Ok(())
    }

    /// Edit a message in history and emit event
    pub async fn edit_message(&mut self, message_id: Uuid, new_content: impl Into<String>) -> Result<()> {
        let new_content_str = new_content.into();
        self.history.edit_message(message_id, new_content_str.clone())?;
        self.event_tx.send(MessageEvent::MessageEdited {
            message_id,
            new_content: new_content_str,
        }).await?;
        Ok(())
    }

    /// Delete a message from history and emit event
    pub async fn delete_message(&mut self, message_id: Uuid) -> Result<()> {
        self.history.delete_message(message_id)?;
        self.event_tx.send(MessageEvent::MessageDeleted { message_id }).await?;
        Ok(())
    }

    /// Pin a message and emit event
    pub async fn pin_message(&mut self, message_id: Uuid, pinned_by: impl Into<String>) -> Result<()> {
        let pinned_by_str = pinned_by.into();
        self.history.pin_message(message_id, pinned_by_str.clone())?;
        self.event_tx.send(MessageEvent::MessagePinned {
            message_id,
            pinned_by: pinned_by_str,
        }).await?;
        Ok(())
    }

    /// Unpin a message and emit event
    pub async fn unpin_message(&mut self, message_id: Uuid) -> Result<()> {
        self.history.unpin_message(message_id)?;
        self.event_tx.send(MessageEvent::MessageUnpinned { message_id }).await?;
        Ok(())
    }
}

impl Default for MessagingService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = Message::new("alice", "bob", "hello");
        assert_eq!(msg.sender, "alice");
        assert_eq!(msg.recipient, "bob");
        assert_eq!(msg.content, "hello");
        assert_eq!(msg.delivery_status, DeliveryStatus::Pending);
        assert!(!msg.deleted);
        assert!(msg.edited_at.is_none());
        assert!(msg.parent_id.is_none());
        assert!(!msg.pinned);
    }

    #[test]
    fn test_message_with_mentions() {
        let msg = Message::new("alice", "bob", "Hey @bob and @charlie, check this out!");
        assert!(msg.mentions.contains(&"bob".to_string()));
        assert!(msg.mentions.contains(&"charlie".to_string()));
        assert!(!msg.mentions.contains(&"alice".to_string()));
    }

    #[test]
    fn test_message_edit() {
        let mut msg = Message::new("alice", "bob", "hello");
        let original_id = msg.id;
        msg.edit("hello world");
        assert_eq!(msg.content, "hello world");
        assert!(msg.edited_at.is_some());
        assert_eq!(msg.id, original_id);
    }

    #[test]
    fn test_message_delete() {
        let mut msg = Message::new("alice", "bob", "hello");
        msg.mark_deleted();
        assert!(msg.deleted);
        assert!(msg.content.is_empty());
    }

    #[test]
    fn test_message_pin() {
        let mut msg = Message::new("alice", "bob", "hello");
        msg.pin("bob");
        assert!(msg.pinned);
        assert_eq!(msg.pinned_by, Some("bob".to_string()));
        assert!(msg.pinned_at.is_some());
    }

    #[test]
    fn test_message_reply() {
        let parent = Message::new("alice", "bob", "original");
        let reply = Message::new("bob", "alice", "reply").with_parent(parent.id);
        assert_eq!(reply.parent_id, Some(parent.id));
    }

    #[test]
    fn test_offline_queue() {
        let mut queue = OfflineQueue::new();
        let msg = Message::new("alice", "bob", "hello");
        queue.enqueue(msg.clone()).unwrap();
        assert_eq!(queue.len(), 1);

        let dequeued = queue.dequeue().unwrap();
        assert_eq!(dequeued.content, "hello");
        assert!(queue.is_empty());
    }

    #[test]
    fn test_delivery_confirmation() {
        let msg = Message::new("alice", "bob", "hello");
        let confirmation = DeliveryConfirmation::new(msg.id, "bob", DeliveryStatus::Delivered);
        assert_eq!(confirmation.message_id, msg.id);
        assert_eq!(confirmation.status, DeliveryStatus::Delivered);
    }

    #[test]
    fn test_message_history() {
        let mut history = MessageHistory::new();
        let msg1 = Message::new("alice", "bob", "first message");
        let msg2 = Message::new("bob", "alice", "second message");
        
        history.add(msg1.clone());
        history.add(msg2.clone());
        
        assert_eq!(history.len(), 2);
        assert_eq!(history.get(msg1.id).unwrap().content, "first message");
        assert_eq!(history.get(msg2.id).unwrap().content, "second message");
    }

    #[test]
    fn test_message_history_search() {
        let mut history = MessageHistory::new();
        history.add(Message::new("alice", "bob", "hello world"));
        history.add(Message::new("bob", "alice", "goodbye world"));
        history.add(Message::new("alice", "bob", "hello again"));
        
        let results = history.search("hello");
        assert_eq!(results.len(), 2);
        
        let results = history.search("goodbye");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_message_history_edit_and_delete() {
        let mut history = MessageHistory::new();
        let msg = Message::new("alice", "bob", "original");
        history.add(msg.clone());
        
        history.edit_message(msg.id, "edited").unwrap();
        assert_eq!(history.get(msg.id).unwrap().content, "edited");
        
        history.delete_message(msg.id).unwrap();
        assert!(history.get(msg.id).unwrap().deleted);
    }

    #[test]
    fn test_message_history_pin() {
        let mut history = MessageHistory::new();
        let msg = Message::new("alice", "bob", "important");
        history.add(msg.clone());
        
        history.pin_message(msg.id, "bob").unwrap();
        let pinned = history.pinned_messages();
        assert_eq!(pinned.len(), 1);
        assert_eq!(pinned[0].content, "important");
        
        history.unpin_message(msg.id).unwrap();
        assert!(history.pinned_messages().is_empty());
    }

    #[test]
    fn test_message_history_threads() {
        let mut history = MessageHistory::new();
        let parent = Message::new("alice", "bob", "original");
        history.add(parent.clone());
        
        let reply1 = Message::new("bob", "alice", "reply 1").with_parent(parent.id);
        let reply2 = Message::new("alice", "bob", "reply 2").with_parent(parent.id);
        history.add(reply1);
        history.add(reply2);
        
        let replies = history.thread_replies(parent.id);
        assert_eq!(replies.len(), 2);
    }

    #[test]
    fn test_message_history_search_by_mention() {
        let mut history = MessageHistory::new();
        history.add(Message::new("alice", "bob", "Hey @bob!"));
        history.add(Message::new("bob", "alice", "Hi @alice!"));
        history.add(Message::new("charlie", "general", "Hello everyone"));
        
        let mentions_bob = history.search_by_mention("bob");
        assert_eq!(mentions_bob.len(), 1);
        
        let mentions_alice = history.search_by_mention("alice");
        assert_eq!(mentions_alice.len(), 1);
    }
}
