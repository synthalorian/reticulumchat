use anyhow::Result;
use notify_rust::{Notification, Urgency};

/// Desktop notification service for ReticulumChat
pub struct NotificationService {
    enabled: bool,
    app_name: String,
}

impl NotificationService {
    pub fn new(app_name: impl Into<String>) -> Self {
        Self {
            enabled: true,
            app_name: app_name.into(),
        }
    }

    pub fn disabled() -> Self {
        Self {
            enabled: false,
            app_name: "ReticulumChat".to_string(),
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
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

    /// Send a notification for a delivery confirmation
    pub fn notify_delivery(&self, recipient: &str, status: &str) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        Notification::new()
            .summary(&format!("Message {}", status))
            .body(&format!("Your message to {} has been {}", recipient, status))
            .icon("dialog-information")
            .appname(&self.app_name)
            .urgency(Urgency::Low)
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
