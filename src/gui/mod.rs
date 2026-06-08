use crate::messaging::{Message, MessageEvent};
use crate::notification::NotificationService;
use crate::SharedState;
use anyhow::Result;
use tokio::sync::mpsc;

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
}

impl GuiApp {
    pub fn new(state: SharedState) -> Self {
        Self {
            state,
            messages: Vec::new(),
            input: String::new(),
            notification_service: NotificationService::default(),
            should_quit: false,
            selected_contact: 0,
            contacts: vec!["General".to_string()],
            event_rx: None,
        }
    }

    pub fn run(self) -> Result<()> {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([800.0, 600.0]),
            ..Default::default()
        };

        eframe::run_native(
            "ReticulumChat",
            options,
            Box::new(|_cc| Ok(Box::new(self))),
        )
        .map_err(|e| anyhow::anyhow!("GUI error: {:?}", e))?;

        Ok(())
    }

    fn send_message(&mut self) {
        let content = self.input.trim();
        if content.is_empty() {
            return;
        }

        // We need to get the identity name synchronously
        // For the GUI, we'll use a placeholder since async runtime access is tricky in egui
        let sender = "user".to_string();

        let msg = Message::new(
            sender,
            self.contacts[self.selected_contact].clone(),
            content,
        );

        self.messages.push(msg);
        self.input.clear();
    }
}

impl eframe::App for GuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process any pending message events
        if let Some(ref mut rx) = self.event_rx {
            while let Ok(event) = rx.try_recv() {
                match event {
                    MessageEvent::MessageReceived(msg) => {
                        let _ = self
                            .notification_service
                            .notify_message(&msg.sender, &msg.content);
                        self.messages.push(msg);
                    }
                    MessageEvent::DeliveryConfirmed(conf) => {
                        if let Some(msg) = self.messages.iter_mut().find(|m| m.id == conf.message_id)
                        {
                            msg.delivery_status = conf.status;
                        }
                    }
                    MessageEvent::QueueUpdated { .. } => {}
                }
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("ReticulumChat");
            ui.separator();

            // Contacts list
            ui.horizontal(|ui| {
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
                    egui::ScrollArea::vertical()
                        .max_height(400.0)
                        .show(ui, |ui| {
                            for msg in &self.messages {
                                ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(&msg.sender)
                                                .strong()
                                                .color(egui::Color32::from_rgb(100, 180, 255)),
                                        );
                                        ui.label(
                                            msg.timestamp
                                                .format("%Y-%m-%d %H:%M")
                                                .to_string(),
                                        );
                                    });
                                    ui.label(&msg.content);
                                });
                            }
                        });

                    ui.separator();

                    // Input area
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.input);
                        if ui.button("Send").clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter))
                        {
                            self.send_message();
                        }
                    });
                });
            });
        });
    }

    fn on_exit(&mut self, _ctx: Option<&eframe::glow::Context>) {
        self.should_quit = true;
    }
}
