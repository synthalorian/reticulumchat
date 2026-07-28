use anyhow::Result;
use notify_rust::{Notification, Urgency};

/// Desktop notification service for ReticulumChat
pub struct NotificationService {
    enabled: bool,
    app_name: String,
    current_user: String,
}

impl NotificationService {
    pub fn new(app_name: impl Into<String>) -> Self {
        Self {
            enabled: true,
            app_name: app_name.into(),
            current_user: String::new(),
        }
    }

    pub fn disabled() -> Self {
        Self {
            enabled: false,
            app_name: "ReticulumChat".to_string(),
            current_user: String::new(),
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_current_user(&mut self, user: impl Into<String>) {
        self.current_user = user.into();
    }

    pub fn current_user(&self) -> &str {
        &self.current_user
    }

    /// Check if a message mentions the current user
    pub fn is_mentioned(&self, mentions: &[String]) -> bool {
        if self.current_user.is_empty() {
            return false;
        }
        let current_lower = self.current_user.to_lowercase();
        mentions.iter().any(|m| m.to_lowercase() == current_lower)
    }

    /// Send a notification for a new message
    pub fn notify_message(&self, sender: &str, preview: &str) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        Notification::new()
            .summary(&format!("New message from {}", sender))
            .body(preview)
            .icon("dialog-information")
            .appname(&self.app_name)
            .urgency(Urgency::Normal)
            .show()?;

        Ok(())
    }

    /// Send a notification for a mention
    pub fn notify_mention(&self, sender: &str, preview: &str) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        Notification::new()
            .summary(&format!("{} mentioned you", sender))
            .body(preview)
            .icon("dialog-warning")
            .appname(&self.app_name)
            .urgency(Urgency::Critical)
            .show()?;

        Ok(())
    }

    /// Send a notification for a reply to your message
    pub fn notify_reply(&self, sender: &str, preview: &str) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        Notification::new()
            .summary(&format!("{} replied to you", sender))
            .body(preview)
            .icon("dialog-information")
            .appname(&self.app_name)
            .urgency(Urgency::Normal)
            .show()?;

        Ok(())
    }

    /// Send a notification for a delivery confirmation
    pub fn notify_delivery(&self, recipient: &str, status: &str) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        Notification::new()
            .summary(&format!("Message {}", status))
            .body(&format!(
                "Your message to {} has been {}",
                recipient, status
            ))
            .icon("dialog-information")
            .appname(&self.app_name)
            .urgency(Urgency::Low)
            .show()?;

        Ok(())
    }

    /// Send a notification for a pinned message
    pub fn notify_pinned(&self, pinner: &str, preview: &str) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        Notification::new()
            .summary(&format!("{} pinned a message", pinner))
            .body(preview)
            .icon("dialog-information")
            .appname(&self.app_name)
            .urgency(Urgency::Normal)
            .show()?;

        Ok(())
    }

    /// Send a generic notification
    pub fn notify(&self, title: &str, body: &str) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        Notification::new()
            .summary(title)
            .body(body)
            .icon("dialog-information")
            .appname(&self.app_name)
            .urgency(Urgency::Normal)
            .show()?;

        Ok(())
    }
}

impl Default for NotificationService {
    fn default() -> Self {
        Self::new("ReticulumChat")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mention_detection() {
        let mut service = NotificationService::new("TestApp");
        service.set_current_user("alice");

        assert!(service.is_mentioned(&["bob".to_string(), "alice".to_string()]));
        assert!(service.is_mentioned(&["ALICE".to_string()])); // case insensitive
        assert!(!service.is_mentioned(&["bob".to_string(), "charlie".to_string()]));
    }

    #[test]
    fn test_disabled_notifications() {
        let service = NotificationService::disabled();
        assert!(service.notify_message("test", "test").is_ok()); // Should not error when disabled
    }
}
