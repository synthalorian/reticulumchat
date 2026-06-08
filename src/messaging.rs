use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
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

/// A chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub sender: String,
    pub recipient: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub encrypted: bool,
    pub delivery_status: DeliveryStatus,
}

impl Message {
    pub fn new(sender: impl Into<String>, recipient: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            sender: sender.into(),
            recipient: recipient.into(),
            content: content.into(),
            timestamp: Utc::now(),
            encrypted: false,
            delivery_status: DeliveryStatus::Pending,
        }
    }

    pub fn with_encryption(mut self, encrypted: bool) -> Self {
        self.encrypted = encrypted;
        self
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

/// Events emitted by the messaging system
#[derive(Debug, Clone)]
pub enum MessageEvent {
    MessageReceived(Message),
    DeliveryConfirmed(DeliveryConfirmation),
    QueueUpdated { count: usize },
}

/// The messaging service handles sending, receiving, queuing, and confirmations
pub struct MessagingService {
    pub event_tx: mpsc::Sender<MessageEvent>,
    pub event_rx: mpsc::Receiver<MessageEvent>,
    pub offline_queue: OfflineQueue,
}

impl MessagingService {
    pub fn new() -> Self {
        let (event_tx, event_rx) = mpsc::channel(100);
        Self {
            event_tx,
            event_rx,
            offline_queue: OfflineQueue::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let (event_tx, event_rx) = mpsc::channel(100);
        Self {
            event_tx,
            event_rx,
            offline_queue: OfflineQueue::with_capacity(capacity),
        }
    }

    /// Queue a message for delivery
    pub fn queue_message(&mut self, message: Message) -> Result<()> {
        self.offline_queue.enqueue(message)?;
        let _ = self.event_tx.try_send(MessageEvent::QueueUpdated {
            count: self.offline_queue.len(),
        });
        Ok(())
    }

    /// Process delivery confirmation
    pub async fn confirm_delivery(&self, confirmation: DeliveryConfirmation) -> Result<()> {
        self.event_tx
            .send(MessageEvent::DeliveryConfirmed(confirmation))
            .await?;
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
}
