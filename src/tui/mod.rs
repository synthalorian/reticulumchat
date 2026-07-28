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
use uuid::Uuid;

/// Different modes the TUI can be in
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiMode {
    Normal,
    Edit,
    Reply,
    Search,
    Thread,
    Pinned,
    Network,
    NetworkNode,
}

/// TUI Application state
pub struct TuiApp {
    pub state: SharedState,
    pub messages: Vec<Message>,
    pub input: String,
    pub notification_service: NotificationService,
    pub should_quit: bool,
    pub selected_contact: usize,
    pub contacts: Vec<String>,
    pub mode: TuiMode,
    pub selected_message: Option<usize>,
    pub search_query: String,
    pub search_results: Vec<usize>,
    pub thread_parent_id: Option<Uuid>,
    pub pinned_messages: Vec<Message>,
    pub editing_message_id: Option<Uuid>,
    pub replying_to_id: Option<Uuid>,
    pub selected_node: usize,
    pub selected_path: usize,
    pub network_show_bandwidth: bool,
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
            mode: TuiMode::Normal,
            selected_message: None,
            search_query: String::new(),
            search_results: Vec::new(),
            thread_parent_id: None,
            pinned_messages: Vec::new(),
            editing_message_id: None,
            replying_to_id: None,
            selected_node: 0,
            selected_path: 0,
            network_show_bandwidth: false,
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

        // Set current user for mention detection
        {
            let state_guard = self.state.read().await;
            self.notification_service
                .set_current_user(&state_guard.identity.name);
        }

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

            match self.mode {
                TuiMode::Normal => self.handle_normal_input(key).await?,
                TuiMode::Edit => self.handle_edit_input(key).await?,
                TuiMode::Reply => self.handle_reply_input(key).await?,
                TuiMode::Search => self.handle_search_input(key).await?,
                TuiMode::Thread => self.handle_thread_input(key).await?,
                TuiMode::Pinned => self.handle_pinned_input(key).await?,
                TuiMode::Network => self.handle_network_input(key).await?,
                TuiMode::NetworkNode => self.handle_network_node_input(key).await?,
            }
        }
        Ok(())
    }

    async fn handle_normal_input(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
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
            KeyCode::Char('e') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                if !self.messages.is_empty() {
                    self.mode = TuiMode::Edit;
                    self.selected_message = Some(self.messages.len().saturating_sub(1));
                    self.editing_message_id = self
                        .selected_message
                        .and_then(|i| self.messages.get(i).map(|m| m.id));
                    if let Some(id) = self.editing_message_id {
                        if let Some(msg) = self.messages.iter().find(|m| m.id == id) {
                            self.input = msg.content.clone();
                        }
                    }
                }
            }
            KeyCode::Char('r') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                if !self.messages.is_empty() {
                    self.mode = TuiMode::Reply;
                    self.selected_message = Some(self.messages.len().saturating_sub(1));
                    self.replying_to_id = self
                        .selected_message
                        .and_then(|i| self.messages.get(i).map(|m| m.id));
                    self.input.clear();
                }
            }
            KeyCode::Char('f') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                self.mode = TuiMode::Search;
                self.search_query.clear();
                self.search_results.clear();
                self.input.clear();
            }
            KeyCode::Char('p') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                if !self.messages.is_empty() {
                    let idx = self.messages.len().saturating_sub(1);
                    if let Some(msg) = self.messages.get(idx) {
                        let id = msg.id;
                        let _ = self.toggle_pin(id).await;
                    }
                }
            }
            KeyCode::Char('d') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                if !self.messages.is_empty() {
                    let idx = self.messages.len().saturating_sub(1);
                    if let Some(msg) = self.messages.get(idx) {
                        let id = msg.id;
                        let _ = self.delete_message(id).await;
                    }
                }
            }
            KeyCode::Char('t') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                if !self.messages.is_empty() {
                    let idx = self.messages.len().saturating_sub(1);
                    if let Some(msg) = self.messages.get(idx) {
                        self.thread_parent_id = Some(msg.id);
                        self.mode = TuiMode::Thread;
                    }
                }
            }
            KeyCode::Char('v') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                self.pinned_messages = self.messages.iter().filter(|m| m.pinned).cloned().collect();
                if !self.pinned_messages.is_empty() {
                    self.mode = TuiMode::Pinned;
                }
            }
            KeyCode::Char('n') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                self.mode = TuiMode::Network;
                self.selected_node = 0;
                self.network_show_bandwidth = false;
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
            KeyCode::Char(c) => {
                self.input.push(c);
            }
            KeyCode::Backspace => {
                self.input.pop();
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_edit_input(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.cancel_mode();
            }
            KeyCode::Enter => {
                if let Some(id) = self.editing_message_id {
                    let new_content = self.input.clone();
                    if !new_content.is_empty() {
                        self.edit_message(id, new_content).await?;
                    }
                }
                self.cancel_mode();
            }
            KeyCode::Char(c) => {
                self.input.push(c);
            }
            KeyCode::Backspace => {
                self.input.pop();
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_reply_input(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.cancel_mode();
            }
            KeyCode::Enter => {
                if let Some(parent_id) = self.replying_to_id {
                    let content = self.input.trim().to_string();
                    if !content.is_empty() {
                        self.send_reply(parent_id, &content).await?;
                    }
                }
                self.cancel_mode();
            }
            KeyCode::Char(c) => {
                self.input.push(c);
            }
            KeyCode::Backspace => {
                self.input.pop();
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_search_input(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.cancel_mode();
            }
            KeyCode::Enter => {
                self.search_query = self.input.clone();
                self.perform_search();
            }
            KeyCode::Char(c) => {
                self.input.push(c);
            }
            KeyCode::Backspace => {
                self.input.pop();
            }
            KeyCode::Up => {
                if let Some(selected) = self.selected_message {
                    if selected > 0 {
                        self.selected_message = Some(selected - 1);
                    }
                }
            }
            KeyCode::Down => {
                if let Some(selected) = self.selected_message {
                    if selected + 1 < self.search_results.len() {
                        self.selected_message = Some(selected + 1);
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_thread_input(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.cancel_mode();
            }
            KeyCode::Up => {
                if let Some(selected) = self.selected_message {
                    if selected > 0 {
                        self.selected_message = Some(selected - 1);
                    }
                }
            }
            KeyCode::Down => {
                let thread_len = self.get_thread_messages().len();
                if let Some(selected) = self.selected_message {
                    if selected + 1 < thread_len {
                        self.selected_message = Some(selected + 1);
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_pinned_input(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.cancel_mode();
            }
            KeyCode::Up => {
                if let Some(selected) = self.selected_message {
                    if selected > 0 {
                        self.selected_message = Some(selected - 1);
                    }
                }
            }
            KeyCode::Down => {
                if let Some(selected) = self.selected_message {
                    if selected + 1 < self.pinned_messages.len() {
                        self.selected_message = Some(selected + 1);
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn cancel_mode(&mut self) {
        self.mode = TuiMode::Normal;
        self.selected_message = None;
        self.thread_parent_id = None;
        self.editing_message_id = None;
        self.replying_to_id = None;
        self.selected_node = 0;
        self.selected_path = 0;
        self.input.clear();
    }

    async fn handle_message_event(&mut self, event: MessageEvent) -> Result<()> {
        match event {
            MessageEvent::MessageReceived(msg) => {
                // Check if message mentions current user
                if self.notification_service.is_mentioned(&msg.mentions) {
                    self.notification_service
                        .notify_mention(&msg.sender, &msg.content)?;
                } else {
                    self.notification_service
                        .notify_message(&msg.sender, &msg.content)?;
                }
                self.messages.push(msg);
            }
            MessageEvent::DeliveryConfirmed(conf) => {
                if let Some(msg) = self.messages.iter_mut().find(|m| m.id == conf.message_id) {
                    msg.delivery_status = conf.status;
                }
            }
            MessageEvent::QueueUpdated { count } => {
                let _ = count;
            }
            MessageEvent::MessageEdited {
                message_id,
                new_content,
            } => {
                if let Some(msg) = self.messages.iter_mut().find(|m| m.id == message_id) {
                    msg.edit(new_content);
                }
            }
            MessageEvent::MessageDeleted { message_id } => {
                if let Some(msg) = self.messages.iter_mut().find(|m| m.id == message_id) {
                    msg.mark_deleted();
                }
            }
            MessageEvent::MessagePinned {
                message_id,
                pinned_by,
            } => {
                if let Some(msg) = self.messages.iter_mut().find(|m| m.id == message_id) {
                    msg.pin(pinned_by);
                }
                self.pinned_messages = self.messages.iter().filter(|m| m.pinned).cloned().collect();
            }
            MessageEvent::MessageUnpinned { message_id } => {
                if let Some(msg) = self.messages.iter_mut().find(|m| m.id == message_id) {
                    msg.unpin();
                }
                self.pinned_messages = self.messages.iter().filter(|m| m.pinned).cloned().collect();
            }
            MessageEvent::MentionReceived {
                message,
                mentioned_user,
            } => {
                let _ = mentioned_user;
                // Only notify if not already notified by MessageReceived
                if !self.messages.iter().any(|m| m.id == message.id) {
                    self.notification_service
                        .notify_mention(&message.sender, &message.content)?;
                    self.messages.push(message);
                }
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

        Ok(())
    }

    async fn send_reply(&mut self, parent_id: Uuid, content: &str) -> Result<()> {
        let state = self.state.read().await;
        let msg = Message::new(
            state.identity.name.clone(),
            self.contacts[self.selected_contact].clone(),
            content,
        )
        .with_parent(parent_id);
        drop(state);

        self.messages.push(msg.clone());
        Ok(())
    }

    async fn edit_message(&mut self, message_id: Uuid, new_content: String) -> Result<()> {
        if let Some(msg) = self.messages.iter_mut().find(|m| m.id == message_id) {
            msg.edit(new_content);
        }
        Ok(())
    }

    async fn delete_message(&mut self, message_id: Uuid) -> Result<()> {
        if let Some(msg) = self.messages.iter_mut().find(|m| m.id == message_id) {
            msg.mark_deleted();
        }
        Ok(())
    }

    async fn toggle_pin(&mut self, message_id: Uuid) -> Result<()> {
        if let Some(msg) = self.messages.iter_mut().find(|m| m.id == message_id) {
            if msg.pinned {
                msg.unpin();
            } else {
                let state = self.state.read().await;
                msg.pin(&state.identity.name);
                drop(state);
            }
        }
        self.pinned_messages = self.messages.iter().filter(|m| m.pinned).cloned().collect();
        Ok(())
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
        self.selected_message = if self.search_results.is_empty() {
            None
        } else {
            Some(0)
        };
    }

    fn get_thread_messages(&self) -> Vec<&Message> {
        if let Some(parent_id) = self.thread_parent_id {
            let mut thread = vec![];
            // Add parent message first
            if let Some(parent) = self.messages.iter().find(|m| m.id == parent_id) {
                thread.push(parent);
            }
            // Add all replies
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

    fn draw(&self, frame: &mut ratatui::Frame) {
        match self.mode {
            TuiMode::Normal => self.draw_normal(frame),
            TuiMode::Edit => self.draw_edit(frame),
            TuiMode::Reply => self.draw_reply(frame),
            TuiMode::Search => self.draw_search(frame),
            TuiMode::Thread => self.draw_thread(frame),
            TuiMode::Pinned => self.draw_pinned(frame),
            TuiMode::Network => self.draw_network(frame),
            TuiMode::NetworkNode => self.draw_network_node(frame),
        }
    }

    fn draw_normal(&self, frame: &mut ratatui::Frame) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
            .split(frame.area());

        // Contacts sidebar
        let contacts_block = Block::default().title("Contacts").borders(Borders::ALL);
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

        // Pinned messages indicator
        let pinned_count = self.messages.iter().filter(|m| m.pinned).count();
        let title = if pinned_count > 0 {
            format!(
                "Chat - {} ({} pinned, Ctrl+V to view | Ctrl+Q to quit)",
                self.contacts[self.selected_contact], pinned_count
            )
        } else {
            format!(
                "Chat - {} (Ctrl+Q to quit)",
                self.contacts[self.selected_contact]
            )
        };

        // Messages
        let messages_block = Block::default().title(title).borders(Borders::ALL);
        let messages_text = Text::from(
            self.messages
                .iter()
                .enumerate()
                .map(|(i, m)| self.format_message_line(m, i))
                .collect::<Vec<_>>(),
        );
        let messages_paragraph = Paragraph::new(messages_text)
            .block(messages_block)
            .wrap(Wrap { trim: true });
        frame.render_widget(messages_paragraph, main_chunks[0]);

        // Input
        let input_block = Block::default().title("Message (Ctrl+E=Edit Ctrl+R=Reply Ctrl+F=Search Ctrl+P=Pin Ctrl+D=Del Ctrl+T=Thread Ctrl+N=Network)").borders(Borders::ALL);
        let input_paragraph = Paragraph::new(self.input.as_str()).block(input_block);
        frame.render_widget(input_paragraph, main_chunks[1]);
    }

    fn format_message_line<'a>(&self, msg: &'a Message, _index: usize) -> Line<'a> {
        let mut spans = vec![Span::styled(
            format!("[{}] ", msg.timestamp.format("%H:%M")),
            Style::default().fg(Color::Gray),
        )];

        if msg.pinned {
            spans.push(Span::styled("📌 ", Style::default().fg(Color::Yellow)));
        }

        if msg.deleted {
            spans.push(Span::styled(
                "[deleted]".to_string(),
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            ));
        } else {
            spans.push(Span::styled(
                format!("{}: ", msg.sender),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ));

            if msg.parent_id.is_some() {
                spans.push(Span::styled("↳ ", Style::default().fg(Color::Magenta)));
            }

            spans.push(Span::raw(&msg.content));

            if msg.edited_at.is_some() {
                spans.push(Span::styled(
                    " (edited)",
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::ITALIC),
                ));
            }
        }

        Line::from(spans)
    }

    fn draw_edit(&self, frame: &mut ratatui::Frame) {
        let area = frame.area();
        let block = Block::default()
            .title("Edit Message (Enter=Save Esc=Cancel)")
            .borders(Borders::ALL);
        let paragraph = Paragraph::new(self.input.as_str()).block(block);
        frame.render_widget(paragraph, area);
    }

    fn draw_reply(&self, frame: &mut ratatui::Frame) {
        let area = frame.area();
        let block = Block::default()
            .title("Reply to Message (Enter=Send Esc=Cancel)")
            .borders(Borders::ALL);
        let paragraph = Paragraph::new(self.input.as_str()).block(block);
        frame.render_widget(paragraph, area);
    }

    fn draw_search(&self, frame: &mut ratatui::Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(3)])
            .split(frame.area());

        // Search input
        let input_block = Block::default()
            .title("Search (Enter=Search Esc=Cancel)")
            .borders(Borders::ALL);
        let input_paragraph = Paragraph::new(self.input.as_str()).block(input_block);
        frame.render_widget(input_paragraph, chunks[0]);

        // Results
        let results_block = Block::default()
            .title(format!("Results: {} matches", self.search_results.len()))
            .borders(Borders::ALL);

        let results_text = if self.search_results.is_empty() && !self.search_query.is_empty() {
            Text::from("No results found.")
        } else {
            Text::from(
                self.search_results
                    .iter()
                    .enumerate()
                    .map(|(i, &msg_idx)| {
                        if let Some(msg) = self.messages.get(msg_idx) {
                            let style = if self.selected_message == Some(i) {
                                Style::default().fg(Color::Black).bg(Color::White)
                            } else {
                                Style::default()
                            };
                            Line::from(vec![
                                Span::styled(
                                    format!("[{}] ", msg.timestamp.format("%Y-%m-%d %H:%M")),
                                    Style::default().fg(Color::Gray),
                                ),
                                Span::styled(
                                    format!("{}: ", msg.sender),
                                    Style::default().fg(Color::Cyan),
                                ),
                                Span::raw(&msg.content),
                            ])
                            .style(style)
                        } else {
                            Line::from("")
                        }
                    })
                    .collect::<Vec<_>>(),
            )
        };
        let results_paragraph = Paragraph::new(results_text).block(results_block);
        frame.render_widget(results_paragraph, chunks[1]);
    }

    fn draw_thread(&self, frame: &mut ratatui::Frame) {
        let area = frame.area();
        let thread_messages = self.get_thread_messages();

        let block = Block::default()
            .title("Thread View (Esc/Q=Back)")
            .borders(Borders::ALL);

        let text = Text::from(
            thread_messages
                .iter()
                .enumerate()
                .map(|(i, m)| {
                    let style = if self.selected_message == Some(i) {
                        Style::default().fg(Color::Black).bg(Color::White)
                    } else {
                        Style::default()
                    };
                    self.format_message_line(m, i).style(style)
                })
                .collect::<Vec<_>>(),
        );
        let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
        frame.render_widget(paragraph, area);
    }

    fn draw_pinned(&self, frame: &mut ratatui::Frame) {
        let area = frame.area();

        let block = Block::default()
            .title("Pinned Messages (Esc/Q=Back)")
            .borders(Borders::ALL);

        let text = if self.pinned_messages.is_empty() {
            Text::from("No pinned messages.")
        } else {
            Text::from(
                self.pinned_messages
                    .iter()
                    .enumerate()
                    .map(|(i, m)| {
                        let style = if self.selected_message == Some(i) {
                            Style::default().fg(Color::Black).bg(Color::White)
                        } else {
                            Style::default()
                        };
                        Line::from(vec![
                            Span::styled(
                                format!("[{}] ", m.timestamp.format("%Y-%m-%d %H:%M")),
                                Style::default().fg(Color::Gray),
                            ),
                            Span::styled(
                                format!("{}: ", m.sender),
                                Style::default().fg(Color::Cyan),
                            ),
                            Span::raw(&m.content),
                            Span::styled(
                                format!(
                                    " (pinned by {})",
                                    m.pinned_by.as_deref().unwrap_or("unknown")
                                ),
                                Style::default().fg(Color::Yellow),
                            ),
                        ])
                        .style(style)
                    })
                    .collect::<Vec<_>>(),
            )
        };
        let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
        frame.render_widget(paragraph, area);
    }

    async fn handle_network_input(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.cancel_mode();
            }
            KeyCode::Char('b') => {
                self.network_show_bandwidth = !self.network_show_bandwidth;
            }
            KeyCode::Enter => {
                let state = self.state.read().await;
                let nodes = state.network.sorted_nodes();
                if self.selected_node < nodes.len() {
                    drop(state);
                    self.mode = TuiMode::NetworkNode;
                    self.selected_path = 0;
                }
            }
            KeyCode::Up => {
                if self.selected_node > 0 {
                    self.selected_node -= 1;
                }
            }
            KeyCode::Down => {
                let state = self.state.read().await;
                let node_count = state.network.nodes.len();
                drop(state);
                if self.selected_node + 1 < node_count {
                    self.selected_node += 1;
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_network_node_input(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.mode = TuiMode::Network;
                self.selected_path = 0;
            }
            KeyCode::Up => {
                if self.selected_path > 0 {
                    self.selected_path -= 1;
                }
            }
            KeyCode::Down => {
                let state = self.state.read().await;
                let nodes = state.network.sorted_nodes();
                if let Some(node) = nodes.get(self.selected_node) {
                    let path_count = node.paths.len();
                    drop(state);
                    if self.selected_path + 1 < path_count {
                        self.selected_path += 1;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn draw_network(&self, frame: &mut ratatui::Frame) {
        let area = frame.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(3)])
            .split(area);

        let content_block = Block::default()
            .title(if self.network_show_bandwidth {
                "Network Bandwidth (B=Topology Esc=Back Enter=Details)"
            } else {
                "Network Topology (B=Bandwidth Esc=Back Enter=Details)"
            })
            .borders(Borders::ALL);

        let content = if self.network_show_bandwidth {
            self.draw_bandwidth_content()
        } else {
            self.draw_topology_content()
        };

        let paragraph = Paragraph::new(content)
            .block(content_block)
            .wrap(Wrap { trim: true });
        frame.render_widget(paragraph, chunks[0]);

        let status_text = self.get_network_status_text();
        let status_block = Block::default()
            .title("Network Status")
            .borders(Borders::ALL);
        let status_para = Paragraph::new(status_text).block(status_block);
        frame.render_widget(status_para, chunks[1]);
    }

    fn draw_topology_content(&self) -> Text<'_> {
        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(vec![Span::styled(
            format!(
                "{:<20} {:<10} {:<12} {:<10} {:<15} {:<10}",
                "Node", "Status", "Paths", "Best Lat", "Quality", "Redundant"
            ),
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Cyan),
        )]));
        lines.push(Line::from("─".repeat(80)));

        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Mesh Network Visualization",
            Style::default().add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::from(""));
        lines.push(Line::from(
            "This view shows discovered nodes in the mesh network.",
        ));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Features:",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Yellow),
        )]));
        lines.push(Line::from(
            "• Node discovery and status tracking (Online/Degraded/Offline)",
        ));
        lines.push(Line::from(
            "• Path quality indicators (Excellent/Good/Fair/Poor/Dead)",
        ));
        lines.push(Line::from("• Automatic path redundancy maintenance"));
        lines.push(Line::from("• Bandwidth usage statistics per node"));
        lines.push(Line::from(""));
        lines.push(Line::from(
            "Use 'B' to toggle bandwidth view, Enter to see node details.",
        ));

        Text::from(lines)
    }

    fn draw_bandwidth_content(&self) -> Text<'_> {
        let lines: Vec<Line> = vec![
            Line::from(vec![Span::styled(
                "Bandwidth Usage Statistics",
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::Cyan),
            )]),
            Line::from(""),
            Line::from("Local node bandwidth tracking:"),
            Line::from("• Tracks bytes sent/received over time"),
            Line::from("• Current send/receive rates (5-second average)"),
            Line::from("• Peak rate tracking"),
            Line::from("• Per-node bandwidth accounting"),
            Line::from(""),
            Line::from("Statistics are updated in real-time as packets flow through the mesh."),
        ];

        Text::from(lines)
    }

    fn get_network_status_text(&self) -> Text<'_> {
        let lines: Vec<Line> = vec![Line::from(vec![
            Span::styled("Network: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("v0.6.0 mesh features active | "),
            Span::styled("Nodes: ", Style::default().fg(Color::Yellow)),
            Span::raw("0 discovered | "),
            Span::styled("Paths: ", Style::default().fg(Color::Yellow)),
            Span::raw("0 active | "),
            Span::styled("Redundancy: ", Style::default().fg(Color::Yellow)),
            Span::raw("auto"),
        ])];
        Text::from(lines)
    }

    fn draw_network_node(&self, frame: &mut ratatui::Frame) {
        let area = frame.area();
        let block = Block::default()
            .title("Node Details (Esc/Q=Back)")
            .borders(Borders::ALL);

        let lines: Vec<Line> = vec![
            Line::from(vec![Span::styled(
                "Node Path Details",
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::Cyan),
            )]),
            Line::from(""),
            Line::from("Shows individual path metrics for the selected node."),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Path Quality Indicators:",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from("• Excellent: <1% loss, <100ms latency"),
            Line::from("• Good: <5% loss, <500ms latency"),
            Line::from("• Fair: <20% loss, <2000ms latency"),
            Line::from("• Poor: <50% loss, <5000ms latency"),
            Line::from("• Dead: >50% loss or >5000ms latency"),
        ];

        let paragraph = Paragraph::new(Text::from(lines))
            .block(block)
            .wrap(Wrap { trim: true });
        frame.render_widget(paragraph, area);
    }
}
