use crate::messaging::{Message, MessageEvent};

use crate::notification::NotificationService;
use crate::SharedState;
use anyhow::Result;
use tokio::sync::mpsc;
use uuid::Uuid;

/// Different modes the GUI can be in
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuiMode {
    Normal,
    Edit,
    Reply,
    Search,
    Thread,
    Pinned,
    Network,
    NetworkNode,
}

/// GUI Application state (using egui/eframe)
pub struct GuiApp {
    pub state: SharedState,
    pub messages: Vec<Message>,
    pub input: String,
    pub notification_service: NotificationService,
    pub should_quit: bool,
    pub selected_contact: usize,
    pub contacts: Vec<String>,
    pub event_rx: Option<mpsc::Receiver<MessageEvent>>,
    pub mode: GuiMode,
    pub selected_message: Option<usize>,
    pub search_query: String,
    pub search_results: Vec<usize>,
    pub thread_parent_id: Option<Uuid>,
    pub pinned_messages: Vec<Message>,
    pub editing_message_id: Option<Uuid>,
    pub replying_to_id: Option<Uuid>,
    pub current_user: String,
    pub show_search_results: bool,
    pub selected_node: usize,
    pub network_show_bandwidth: bool,
}

impl GuiApp {
    pub fn new(state: SharedState) -> Self {
        // Use the real identity name for the current user. The lock is held
        // only momentarily at startup; fall back to a placeholder if the
        // state is unavailable (e.g. lock contention during shutdown).
        let user_name = state
            .try_read()
            .map(|s| s.identity.name.clone())
            .unwrap_or_else(|_| "user".to_string());
        let mut notification_service = NotificationService::default();
        notification_service.set_current_user(&user_name);

        Self {
            state,
            messages: Vec::new(),
            input: String::new(),
            notification_service,
            should_quit: false,
            selected_contact: 0,
            contacts: vec!["General".to_string()],
            event_rx: None,
            mode: GuiMode::Normal,
            selected_message: None,
            search_query: String::new(),
            search_results: Vec::new(),
            thread_parent_id: None,
            pinned_messages: Vec::new(),
            editing_message_id: None,
            replying_to_id: None,
            current_user: user_name,
            show_search_results: false,
            selected_node: 0,
            network_show_bandwidth: false,
        }
    }

    pub fn run(self) -> Result<()> {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([900.0, 700.0]),
            ..Default::default()
        };

        eframe::run_native("ReticulumChat", options, Box::new(|_cc| Ok(Box::new(self))))
            .map_err(|e| anyhow::anyhow!("GUI error: {:?}", e))?;

        Ok(())
    }

    fn send_message(&mut self) {
        let content = self.input.trim();
        if content.is_empty() {
            return;
        }

        let msg = Message::new(
            self.current_user.clone(),
            self.contacts[self.selected_contact].clone(),
            content,
        );

        self.messages.push(msg);
        self.input.clear();
    }

    fn send_reply(&mut self, parent_id: Uuid) {
        let content = self.input.trim();
        if content.is_empty() {
            return;
        }

        let msg = Message::new(
            self.current_user.clone(),
            self.contacts[self.selected_contact].clone(),
            content,
        )
        .with_parent(parent_id);

        self.messages.push(msg);
        self.input.clear();
    }

    fn edit_message(&mut self, message_id: Uuid, new_content: String) {
        if let Some(msg) = self.messages.iter_mut().find(|m| m.id == message_id) {
            msg.edit(new_content);
        }
    }

    fn delete_message(&mut self, message_id: Uuid) {
        if let Some(msg) = self.messages.iter_mut().find(|m| m.id == message_id) {
            msg.mark_deleted();
        }
    }

    fn toggle_pin(&mut self, message_id: Uuid) {
        if let Some(msg) = self.messages.iter_mut().find(|m| m.id == message_id) {
            if msg.pinned {
                msg.unpin();
            } else {
                msg.pin(&self.current_user);
            }
        }
        self.update_pinned_messages();
    }

    fn update_pinned_messages(&mut self) {
        self.pinned_messages = self.messages.iter().filter(|m| m.pinned).cloned().collect();
    }

    fn perform_search(&mut self) {
        let query = self.search_query.to_lowercase();
        self.search_results = self
            .messages
            .iter()
            .enumerate()
            .filter(|(_, m)| !m.deleted && m.content.to_lowercase().contains(&query))
            .map(|(i, _)| i)
            .collect();
        self.show_search_results = true;
    }

    fn get_thread_messages(&self) -> Vec<&Message> {
        if let Some(parent_id) = self.thread_parent_id {
            let mut thread = vec![];
            if let Some(parent) = self.messages.iter().find(|m| m.id == parent_id) {
                thread.push(parent);
            }
            for msg in &self.messages {
                if msg.parent_id == Some(parent_id) {
                    thread.push(msg);
                }
            }
            thread
        } else {
            Vec::new()
        }
    }

    fn reset_mode(&mut self) {
        self.mode = GuiMode::Normal;
        self.selected_message = None;
        self.editing_message_id = None;
        self.replying_to_id = None;
        self.thread_parent_id = None;
        self.show_search_results = false;
        self.input.clear();
    }
}

impl eframe::App for GuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut events = Vec::new();
        if let Some(ref mut rx) = self.event_rx {
            while let Ok(event) = rx.try_recv() {
                events.push(event);
            }
        }

        for event in events {
            match event {
                MessageEvent::MessageReceived(msg) => {
                    if self.notification_service.is_mentioned(&msg.mentions) {
                        let _ = self
                            .notification_service
                            .notify_mention(&msg.sender, &msg.content);
                    } else {
                        let _ = self
                            .notification_service
                            .notify_message(&msg.sender, &msg.content);
                    }
                    self.messages.push(msg);
                }
                MessageEvent::DeliveryConfirmed(conf) => {
                    if let Some(msg) = self.messages.iter_mut().find(|m| m.id == conf.message_id) {
                        msg.delivery_status = conf.status;
                    }
                }
                MessageEvent::QueueUpdated { .. } => {}
                MessageEvent::MessageEdited {
                    message_id,
                    new_content,
                } => {
                    self.edit_message(message_id, new_content);
                }
                MessageEvent::MessageDeleted { message_id } => {
                    self.delete_message(message_id);
                }
                MessageEvent::MessagePinned { message_id, .. } => {
                    if let Some(msg) = self.messages.iter_mut().find(|m| m.id == message_id) {
                        msg.pinned = true;
                    }
                    self.update_pinned_messages();
                }
                MessageEvent::MessageUnpinned { message_id } => {
                    if let Some(msg) = self.messages.iter_mut().find(|m| m.id == message_id) {
                        msg.pinned = false;
                    }
                    self.update_pinned_messages();
                }
                MessageEvent::MentionReceived { message, .. } => {
                    if !self.messages.iter().any(|m| m.id == message.id) {
                        let _ = self
                            .notification_service
                            .notify_mention(&message.sender, &message.content);
                        self.messages.push(message);
                    }
                }
            }
        }

        match self.mode {
            GuiMode::Normal => self.draw_normal(ctx),
            GuiMode::Edit => self.draw_edit(ctx),
            GuiMode::Reply => self.draw_reply(ctx),
            GuiMode::Search => self.draw_search(ctx),
            GuiMode::Thread => self.draw_thread(ctx),
            GuiMode::Pinned => self.draw_pinned(ctx),
            GuiMode::Network => self.draw_network(ctx),
            GuiMode::NetworkNode => self.draw_network_node(ctx),
        }
    }

    fn on_exit(&mut self, _ctx: Option<&eframe::glow::Context>) {
        self.should_quit = true;
    }
}

impl GuiApp {
    fn draw_normal(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("ReticulumChat");
            ui.separator();

            // Mode buttons
            ui.horizontal(|ui| {
                if ui.button("Search (Ctrl+F)").clicked() {
                    self.mode = GuiMode::Search;
                    self.search_query.clear();
                    self.input.clear();
                }
                let pinned_count = self.messages.iter().filter(|m| m.pinned).count();
                if pinned_count > 0 && ui.button(format!("Pinned ({})", pinned_count)).clicked() {
                    self.update_pinned_messages();
                    self.mode = GuiMode::Pinned;
                }
                if ui.button("Network (Ctrl+N)").clicked() {
                    self.mode = GuiMode::Network;
                    self.selected_node = 0;
                    self.network_show_bandwidth = false;
                }
            });
            ui.separator();

            ui.horizontal(|ui| {
                // Contacts list
                ui.vertical(|ui| {
                    ui.label("Contacts");
                    for (i, contact) in self.contacts.iter().enumerate() {
                        let is_selected = self.selected_contact == i;
                        if ui.selectable_label(is_selected, contact).clicked() {
                            self.selected_contact = i;
                        }
                    }
                });

                ui.separator();

                // Chat area
                ui.vertical(|ui| {
                    ui.label(format!("Chat: {}", self.contacts[self.selected_contact]));

                    // Messages scroll area
                    let message_data: Vec<_> = self
                        .messages
                        .iter()
                        .enumerate()
                        .map(|(i, msg)| {
                            (
                                i,
                                msg.id,
                                msg.sender.clone(),
                                msg.timestamp,
                                msg.content.clone(),
                                msg.pinned,
                                msg.deleted,
                                msg.parent_id,
                                msg.edited_at,
                            )
                        })
                        .collect();

                    egui::ScrollArea::vertical()
                        .max_height(400.0)
                        .show(ui, |ui| {
                            for (
                                i,
                                msg_id,
                                sender,
                                timestamp,
                                content,
                                pinned,
                                deleted,
                                parent_id,
                                edited_at,
                            ) in message_data
                            {
                                let response = ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        if pinned {
                                            ui.label(
                                                egui::RichText::new("📌")
                                                    .color(egui::Color32::YELLOW),
                                            );
                                        }
                                        ui.label(
                                            egui::RichText::new(sender)
                                                .strong()
                                                .color(egui::Color32::from_rgb(100, 180, 255)),
                                        );
                                        ui.label(timestamp.format("%Y-%m-%d %H:%M").to_string());
                                    });

                                    if deleted {
                                        ui.label(
                                            egui::RichText::new("[deleted]")
                                                .italics()
                                                .color(egui::Color32::GRAY),
                                        );
                                    } else {
                                        if parent_id.is_some() {
                                            ui.label(
                                                egui::RichText::new("↳ Reply")
                                                    .color(egui::Color32::from_rgb(200, 100, 255)),
                                            );
                                        }
                                        ui.label(&content);
                                        if edited_at.is_some() {
                                            ui.label(
                                                egui::RichText::new("(edited)")
                                                    .italics()
                                                    .color(egui::Color32::GRAY),
                                            );
                                        }
                                    }
                                });

                                if response.response.clicked() {
                                    self.selected_message = Some(i);
                                }

                                response.response.context_menu(|ui| {
                                    if ui.button("Reply").clicked() {
                                        self.mode = GuiMode::Reply;
                                        self.replying_to_id = Some(msg_id);
                                        self.input.clear();
                                        ui.close_menu();
                                    }
                                    if ui.button("Edit").clicked() {
                                        self.mode = GuiMode::Edit;
                                        self.editing_message_id = Some(msg_id);
                                        self.input = content.clone();
                                        ui.close_menu();
                                    }
                                    if ui.button("Delete").clicked() {
                                        self.delete_message(msg_id);
                                        ui.close_menu();
                                    }
                                    if pinned {
                                        if ui.button("Unpin").clicked() {
                                            self.toggle_pin(msg_id);
                                            ui.close_menu();
                                        }
                                    } else {
                                        if ui.button("Pin").clicked() {
                                            self.toggle_pin(msg_id);
                                            ui.close_menu();
                                        }
                                    }
                                    if ui.button("View Thread").clicked() {
                                        self.mode = GuiMode::Thread;
                                        self.thread_parent_id = Some(msg_id);
                                        ui.close_menu();
                                    }
                                });
                            }
                        });

                    ui.separator();

                    // Input area
                    ui.horizontal(|ui| {
                        let response = ui.text_edit_singleline(&mut self.input);
                        if ui.button("Send").clicked()
                            || (response.lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                        {
                            self.send_message();
                        }
                    });
                });
            });
        });
    }

    fn draw_edit(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Edit Message");
            ui.separator();

            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut self.input);
                if ui.button("Save").clicked() {
                    if let Some(id) = self.editing_message_id {
                        let new_content = self.input.clone();
                        if !new_content.is_empty() {
                            self.edit_message(id, new_content);
                        }
                    }
                    self.reset_mode();
                }
                if ui.button("Cancel").clicked() {
                    self.reset_mode();
                }
            });
        });
    }

    fn draw_reply(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Reply to Message");
            ui.separator();

            // Show parent message
            if let Some(parent_id) = self.replying_to_id {
                if let Some(parent) = self.messages.iter().find(|m| m.id == parent_id) {
                    ui.group(|ui| {
                        ui.label(format!("{}: {}", parent.sender, parent.content));
                    });
                }
            }

            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut self.input);
                if ui.button("Send Reply").clicked() {
                    if let Some(parent_id) = self.replying_to_id {
                        self.send_reply(parent_id);
                    }
                    self.reset_mode();
                }
                if ui.button("Cancel").clicked() {
                    self.reset_mode();
                }
            });
        });
    }

    fn draw_search(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Search Messages");
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Query:");
                let response = ui.text_edit_singleline(&mut self.search_query);
                if ui.button("Search").clicked()
                    || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                {
                    self.perform_search();
                }
                if ui.button("Clear").clicked() {
                    self.search_query.clear();
                    self.search_results.clear();
                    self.show_search_results = false;
                }
                if ui.button("Back").clicked() {
                    self.reset_mode();
                }
            });

            if self.show_search_results {
                ui.separator();
                ui.label(format!("Found {} results", self.search_results.len()));

                egui::ScrollArea::vertical()
                    .max_height(500.0)
                    .show(ui, |ui| {
                        for &msg_idx in &self.search_results {
                            if let Some(msg) = self.messages.get(msg_idx) {
                                ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(&msg.sender)
                                                .strong()
                                                .color(egui::Color32::from_rgb(100, 180, 255)),
                                        );
                                        ui.label(
                                            msg.timestamp.format("%Y-%m-%d %H:%M").to_string(),
                                        );
                                    });
                                    ui.label(&msg.content);
                                });
                            }
                        }
                    });
            }
        });
    }

    fn draw_thread(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Thread View");
            ui.separator();

            if ui.button("Back").clicked() {
                self.reset_mode();
            }

            ui.separator();

            let thread_messages = self.get_thread_messages();
            if thread_messages.is_empty() {
                ui.label("No messages in this thread.");
            } else {
                egui::ScrollArea::vertical()
                    .max_height(500.0)
                    .show(ui, |ui| {
                        for msg in thread_messages {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(&msg.sender)
                                            .strong()
                                            .color(egui::Color32::from_rgb(100, 180, 255)),
                                    );
                                    ui.label(msg.timestamp.format("%Y-%m-%d %H:%M").to_string());
                                });
                                if msg.deleted {
                                    ui.label(
                                        egui::RichText::new("[deleted]")
                                            .italics()
                                            .color(egui::Color32::GRAY),
                                    );
                                } else {
                                    ui.label(&msg.content);
                                    if msg.edited_at.is_some() {
                                        ui.label(
                                            egui::RichText::new("(edited)")
                                                .italics()
                                                .color(egui::Color32::GRAY),
                                        );
                                    }
                                }
                            });
                        }
                    });
            }
        });
    }

    fn draw_pinned(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Pinned Messages");
            ui.separator();

            if ui.button("Back").clicked() {
                self.reset_mode();
            }

            ui.separator();

            if self.pinned_messages.is_empty() {
                ui.label("No pinned messages.");
            } else {
                egui::ScrollArea::vertical()
                    .max_height(500.0)
                    .show(ui, |ui| {
                        for msg in &self.pinned_messages {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label("📌");
                                    ui.label(
                                        egui::RichText::new(&msg.sender)
                                            .strong()
                                            .color(egui::Color32::from_rgb(100, 180, 255)),
                                    );
                                    ui.label(msg.timestamp.format("%Y-%m-%d %H:%M").to_string());
                                });
                                ui.label(&msg.content);
                                ui.label(
                                    egui::RichText::new(format!(
                                        "Pinned by {}",
                                        msg.pinned_by.as_deref().unwrap_or("unknown")
                                    ))
                                    .italics()
                                    .color(egui::Color32::YELLOW),
                                );
                            });
                        }
                    });
            }
        });
    }

    fn draw_network(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Mesh Network");
            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Back").clicked() {
                    self.reset_mode();
                }
                if ui
                    .button(if self.network_show_bandwidth {
                        "Show Topology"
                    } else {
                        "Show Bandwidth"
                    })
                    .clicked()
                {
                    self.network_show_bandwidth = !self.network_show_bandwidth;
                }
            });

            ui.separator();

            if self.network_show_bandwidth {
                self.draw_bandwidth_panel(ui);
            } else {
                self.draw_topology_panel(ui);
            }
        });
    }

    fn draw_topology_panel(&mut self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("Network Topology").strong().size(16.0));
        ui.separator();

        ui.label("Discovered Nodes:");
        ui.separator();

        egui::ScrollArea::vertical()
            .max_height(400.0)
            .show(ui, |ui| {
                ui.label("No nodes discovered yet.");
                ui.separator();

                ui.collapsing("Path Quality Legend", |ui| {
                    ui.label("Excellent: <1% loss, <100ms latency");
                    ui.label("Good: <5% loss, <500ms latency");
                    ui.label("Fair: <20% loss, <2000ms latency");
                    ui.label("Poor: <50% loss, <5000ms latency");
                    ui.label("Dead: >50% loss or >5000ms latency");
                });

                ui.collapsing("Features", |ui| {
                    ui.label("• Node discovery and status tracking");
                    ui.label("• Path quality indicators");
                    ui.label("• Automatic path redundancy");
                    ui.label("• Bandwidth usage statistics");
                });
            });
    }

    fn draw_bandwidth_panel(&mut self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("Bandwidth Usage").strong().size(16.0));
        ui.separator();

        ui.group(|ui| {
            ui.label("Local Node Statistics:");
            ui.label("Total Sent: 0 B");
            ui.label("Total Received: 0 B");
            ui.label("Current Send Rate: 0 B/s");
            ui.label("Current Receive Rate: 0 B/s");
            ui.label("Peak Send Rate: 0 B/s");
            ui.label("Peak Receive Rate: 0 B/s");
        });

        ui.separator();
        ui.label("Per-Node Bandwidth:");
        ui.label("No nodes to display.");
    }

    fn draw_network_node(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Node Details");
            ui.separator();

            if ui.button("Back").clicked() {
                self.mode = GuiMode::Network;
            }

            ui.separator();
            ui.label("Select a node from the network view to see path details.");
        });
    }
}
