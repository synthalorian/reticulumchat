use crate::messaging::{Message, MessageEvent, MessagingService};
use crate::notification::NotificationService;
use crate::SharedState;
use anyhow::Result;
use crossterm::event::{self, Event as CEvent, KeyCode, KeyEventKind};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::Terminal;
use std::io;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};

/// TUI Application state
pub struct TuiApp {
    pub state: SharedState,
    pub messages: Vec<Message>,
    pub input: String,
    pub notification_service: NotificationService,
    pub should_quit: bool,
    pub selected_contact: usize,
    pub contacts: Vec<String>,
}

impl TuiApp {
    pub fn new(state: SharedState) -> Self {
        Self {
            state,
            messages: Vec::new(),
            input: String::new(),
            notification_service: NotificationService::default(),
            should_quit: false,
            selected_contact: 0,
            contacts: vec!["General".to_string()],
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        // Setup terminal
        crossterm::terminal::enable_raw_mode()?;
        let mut stdout = io::stdout();
        crossterm::execute!(
            stdout,
            crossterm::terminal::EnterAlternateScreen,
            crossterm::event::EnableMouseCapture
        )?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Create messaging service
        let messaging = MessagingService::new();
        let mut event_rx = messaging.event_rx;

        // Main loop
        let mut tick = interval(Duration::from_millis(250));
        let result = self.run_loop(&mut terminal, &mut event_rx, &mut tick).await;

        // Restore terminal
        crossterm::terminal::disable_raw_mode()?;
        crossterm::execute!(
            terminal.backend_mut(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::event::DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        result
    }

    async fn run_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        event_rx: &mut mpsc::Receiver<MessageEvent>,
        tick: &mut tokio::time::Interval,
    ) -> Result<()> {
        while !self.should_quit {
            // Draw UI
            terminal.draw(|f| self.draw(f))?;

            tokio::select! {
                _ = tick.tick() => {}
                Some(event) = event_rx.recv() => {
                    self.handle_message_event(event).await?;
                }
                result = Self::read_crossterm_event() => {
                    if let Some(event) = result? {
                        self.handle_input(event).await?;
                    }
                }
            }
        }
        Ok(())
    }

    async fn read_crossterm_event() -> Result<Option<CEvent>> {
        // Poll for crossterm events with a timeout
        if event::poll(Duration::from_millis(100))? {
            Ok(Some(event::read()?))
        } else {
            Ok(None)
        }
    }

    async fn handle_input(&mut self, event: CEvent) -> Result<()> {
        if let CEvent::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return Ok(());
            }
            match key.code {
                KeyCode::Char('q') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                    self.should_quit = true;
                }
                KeyCode::Esc => {
                    self.should_quit = true;
                }
                KeyCode::Enter => {
                    self.send_message().await?;
                }
                KeyCode::Char(c) => {
                    self.input.push(c);
                }
                KeyCode::Backspace => {
                    self.input.pop();
                }
                KeyCode::Up => {
                    if self.selected_contact > 0 {
                        self.selected_contact -= 1;
                    }
                }
                KeyCode::Down => {
                    if self.selected_contact + 1 < self.contacts.len() {
                        self.selected_contact += 1;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    async fn handle_message_event(&mut self, event: MessageEvent) -> Result<()> {
        match event {
            MessageEvent::MessageReceived(msg) => {
                self.notification_service
                    .notify_message(&msg.sender, &msg.content)?;
                self.messages.push(msg);
            }
            MessageEvent::DeliveryConfirmed(conf) => {
                if let Some(msg) = self.messages.iter_mut().find(|m| m.id == conf.message_id) {
                    msg.delivery_status = conf.status;
                }
            }
            MessageEvent::QueueUpdated { count } => {
                // Could show queue status in UI
                let _ = count;
            }
        }
        Ok(())
    }

    async fn send_message(&mut self) -> Result<()> {
        let content = self.input.trim();
        if content.is_empty() {
            return Ok(());
        }

        let state = self.state.read().await;
        let msg = Message::new(
            state.identity.name.clone(),
            self.contacts[self.selected_contact].clone(),
            content,
        );
        drop(state);

        self.messages.push(msg.clone());
        self.input.clear();

        // In a real implementation, this would send over Reticulum
        // For now, we just add it to the local message list
        Ok(())
    }

    fn draw(&self, frame: &mut ratatui::Frame) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
            .split(frame.area());

        // Contacts sidebar
        let contacts_block = Block::default()
            .title("Contacts")
            .borders(Borders::ALL);
        let contacts: Vec<ListItem> = self
            .contacts
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let style = if i == self.selected_contact {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(c.as_str()).style(style)
            })
            .collect();
        let contacts_list = List::new(contacts).block(contacts_block);
        frame.render_widget(contacts_list, chunks[0]);

        // Main chat area
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(3)])
            .split(chunks[1]);

        // Messages
        let messages_block = Block::default()
            .title(format!(
                "Chat - {} (Ctrl+Q to quit)",
                self.contacts[self.selected_contact]
            ))
            .borders(Borders::ALL);
        let messages_text = Text::from(
            self.messages
                .iter()
                .map(|m| {
                    Line::from(vec![
                        Span::styled(
                            format!("[{}] ", m.timestamp.format("%H:%M")),
                            Style::default().fg(Color::Gray),
                        ),
                        Span::styled(
                            format!("{}: ", m.sender),
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(&m.content),
                    ])
                })
                .collect::<Vec<_>>(),
        );
        let messages_paragraph = Paragraph::new(messages_text)
            .block(messages_block)
            .wrap(Wrap { trim: true });
        frame.render_widget(messages_paragraph, main_chunks[0]);

        // Input
        let input_block = Block::default().title("Message").borders(Borders::ALL);
        let input_paragraph = Paragraph::new(self.input.as_str()).block(input_block);
        frame.render_widget(input_paragraph, main_chunks[1]);
    }
}
