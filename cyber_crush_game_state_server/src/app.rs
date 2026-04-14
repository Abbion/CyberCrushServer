use ratatui::{
    text::Line,
    layout::{ Layout, Rect, Direction, Constraint },
    widgets::{ ListState, ListItem, Borders, Block, List, Paragraph },
    Frame,
    DefaultTerminal,
    prelude::Stylize,
    style::{ Style, Modifier, Color },
};

use crossterm;
use crossterm::event::{ KeyCode, KeyEventKind };

use tokio::sync::Mutex;
use std::sync::Arc;

use std::io;

#[derive(Clone)]
pub struct GameState {
    pub is_online: bool,
    pub offline_message_info: String,
}

enum ScreenState {
    MainMenu,
    GameState,
    GameOfflineInfo,
}

#[derive(PartialEq)]
enum PopupType {
    TextInput,
    ExitConfirmation,
}

pub struct App {
    is_running: bool,
    screen_state: ScreenState,
    menu_options: Vec<String>,
    game_state_options: Vec<String>,
    offline_info_options: Vec<String>,
    exit_options: Vec<String>,
    popup_type: Option<PopupType>,
    select_state: ListState,
    select_popup_state: ListState,

    text_input: String,
    game_state: Arc<Mutex<GameState>>,
    game_state_snapshot: Option<GameState>,
}

impl App {
    pub fn new(game_state: Arc<Mutex<GameState>>) -> App {
        App {
            is_running: true,
            screen_state: ScreenState::MainMenu,
            menu_options: vec![
                "Change game state".into(),
                "Change game offline info".into(),
                "Exit".into()
            ],
            game_state_options: vec![
                "Online".into(),
                "Offline".into(),
                "Back".into(),
            ],
            offline_info_options: vec![
                "Default offline info".into(),
                "Custom info".into(),
                "Back".into(),
            ],
            exit_options: vec![
                "yes".into(),
                "no".into(),
            ],

            popup_type: None,
            select_state: ListState::default(),
            select_popup_state: ListState::default(),
            text_input: "".into(),

            game_state,
            game_state_snapshot: None
        }
    }

    pub async fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        self.reset_select();
        self.reset_popup_select();

        while self.is_running {
            {
                let game_state_lock = self.game_state.lock().await;
                self.game_state_snapshot = Some(game_state_lock.clone());
            }

            terminal.draw(|frame| self.draw(frame) ).unwrap();

            match crossterm::event::read()? {
                crossterm::event::Event::Key(key_event) => self.handle_key_event(key_event).await?,
                _ => {}
            }
        }

        Ok(())
    }

    fn get_current_opiton_size(&self) -> usize {
        if let Some(popup) = &self.popup_type {
            match popup {
                PopupType::TextInput => { 
                    return 0;
                }
                PopupType::ExitConfirmation => {
                    return self.exit_options.len();
                }
            }
        }
        else {
            match self.screen_state {
                ScreenState::MainMenu => {
                    return self.menu_options.len();
                },
                ScreenState::GameState => {
                    return self.game_state_options.len();
                }
                ScreenState::GameOfflineInfo => {
                    return self.offline_info_options.len();
                }
            }
        }
    }

    fn char_entered(&mut self, character: char) {
        if self.popup_type == Some(PopupType::TextInput) {
            self.text_input.push(character);
        }
    }

    fn perform_backspace(&mut self) {
        if self.popup_type == Some(PopupType::TextInput) {
            self.text_input.pop();
        }
    }

    fn select_next(&mut self) {
        if self.popup_type == None {
           let index = match self.select_state.selected() {
                Some(i) => {
                    if i >= self.get_current_opiton_size() - 1 { 0 } else { i + 1 }
                },
                None => 0,
            };

            self.select_state.select(Some(index));
        }
        else {
           let index = match self.select_popup_state.selected() {
                Some(i) => {
                    if i >= self.get_current_opiton_size() - 1 { 0 } else { i + 1 }
                },
                None => 0,
            };

            self.select_popup_state.select(Some(index));
        }
    }

    fn select_previus(&mut self) {
        if self.popup_type == None {
           let index = match self.select_state.selected() {
                Some(i) => {
                    if i <= 0 { self.get_current_opiton_size() - 1 } else { i - 1 }
                },
                None => 0,
            };

            self.select_state.select(Some(index));
        }
        else {
           let index = match self.select_popup_state.selected() {
                Some(i) => {
                    if i <= 0 { self.get_current_opiton_size() - 1 } else { i - 1 }
                },
                None => 0,
            };

            self.select_popup_state.select(Some(index));
        }
    }

    fn reset_select(&mut self) {
        self.select_state.select(Some(0));
    }

    fn reset_popup_select(&mut self) {
        self.select_popup_state.select(Some(0));
    }

    async fn perform_action(&mut self) {
        if let Some(popup) = &self.popup_type {
           let index = match self.select_popup_state.selected() {
                Some(i) => i,
                None => { return },
            };

            match popup {
                PopupType::TextInput => {
                    if !self.text_input.is_empty() {
                        let mut lock = self.game_state.lock().await;
                        lock.offline_message_info = self.text_input.clone();
                    }

                    self.text_input.clear();
                    self.popup_type = None;
                }
                PopupType::ExitConfirmation => {
                    match index {
                        0 => {
                            self.is_running = false;
                        }
                        _ => {
                            self.popup_type = None;
                        }
                    }

                    self.reset_popup_select();
                }
            }
        }
        else {
           let index = match self.select_state.selected() {
                Some(i) => i,
                None => { return },
            };

            match self.screen_state {
                ScreenState::MainMenu => {
                    match index {
                        0 => { 
                            self.screen_state = ScreenState::GameState;
                            self.reset_select();
                        }
                        1 => { 
                            self.screen_state = ScreenState::GameOfflineInfo;
                            self.reset_select();
                        }
                        2 => {
                            self.popup_type = Some(PopupType::ExitConfirmation);
                            self.reset_popup_select();
                        },
                        _ => { return; }
                    }
                }
                ScreenState::GameState => {
                    match index {
                        0 => {
                            let mut lock = self.game_state.lock().await;
                            lock.is_online = true;
                        }
                        1 => {
                            let mut lock = self.game_state.lock().await;
                            lock.is_online = false;
                        }
                        2 => {
                            self.screen_state = ScreenState::MainMenu;
                            self.reset_select();
                        }
                        _ => { return; }
                    }
                }
                ScreenState::GameOfflineInfo => {
                    match index {
                        0 => {
                            let mut lock = self.game_state.lock().await;
                            lock.offline_message_info = "#TR-GAME_IS_OFFLINE".into();
                        }
                        1 => {
                            self.popup_type = Some(PopupType::TextInput);
                        }
                        2 => {
                            self.screen_state = ScreenState::MainMenu;
                            self.reset_select();
                        }
                        _ => { return; }
                    }
                }
            }
        }
    }

    fn perform_escape(&mut self) {
        if self.popup_type != None {
            self.popup_type = None;
        }
    }

     fn draw(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(frame.area());

        let title_block = Block::default()
            .borders(Borders::ALL);

        let title_paragraph = Paragraph::new(" Game state manager ")
            .block(title_block);

        frame.render_widget(title_paragraph, chunks[0]);

        match self.screen_state {
            ScreenState::MainMenu => {
                let items: Vec<ListItem> = self.menu_options
                    .iter()
                    .map(|i| ListItem::new(i.as_str()))
                    .collect();

                let outer_block = Block::default()
                    .title(" Main menu ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::White));

                let list = List::new(items)
                    .block(outer_block)
                    .highlight_style(
                        Style::default()
                        .bg(Color::Yellow)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol(">> ");

                frame.render_stateful_widget(list, chunks[1], &mut self.select_state);

                if self.popup_type == Some(PopupType::ExitConfirmation) {
                    let popup_block = Block::default()
                    .title("Do you want to exit?")
                    .borders(Borders::ALL)
                    .style(Style::default().bg(Color::DarkGray));

                    let popup_area = centered_popup(50, 15, chunks[1]);

                    let items: Vec<ListItem> = self.exit_options
                        .iter()
                        .map(|i| ListItem::new(i.as_str()))
                        .collect();

                    let list = List::new(items)
                        .block(popup_block)
                        .highlight_style(
                            Style::default()
                            .bg(Color::Yellow)
                            .fg(Color::Black)
                            .add_modifier(Modifier::BOLD),
                        )
                        .highlight_symbol(">> ");

                    frame.render_stateful_widget(list, popup_area, &mut self.select_popup_state);
                }
            },
            ScreenState::GameState => {
                let items: Vec<ListItem> = self.game_state_options
                    .iter()
                    .map(|i| ListItem::new(i.as_str()))
                    .collect();

                let outer_block = Block::default()
                    .title(" Game state menu ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::White));

                let inner_area = outer_block.inner(chunks[1]);
                frame.render_widget(outer_block, chunks[1]);

                let inner_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(1),
                        Constraint::Length(1),
                        Constraint::Min(0),
                    ])
                    .split(inner_area);
                

                let is_online = if let Some(game_state) = &self.game_state_snapshot {
                    if game_state.is_online { "Online" } else { "Offline" }
                } 
                else {
                    "Unknown status"
                };

                let header_text = Paragraph::new(format!(" Current game state: {}", is_online))
                    .style(Style::default().fg(Color::Gray));

                frame.render_widget(header_text, inner_layout[0]);

                let separator = Block::default()
                    .borders(Borders::TOP)
                    .border_style(Style::default().fg(Color::DarkGray));
                    frame.render_widget(separator, inner_layout[1]);
                
                let list = List::new(items)
                    .highlight_style(
                        Style::default()
                        .bg(Color::Yellow)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol(">> ");

                frame.render_stateful_widget(list, inner_layout[2], &mut self.select_state);
            }
            ScreenState::GameOfflineInfo => {
                let items: Vec<ListItem> = self.offline_info_options
                    .iter()
                    .map(|i| ListItem::new(i.as_str()))
                    .collect();

                let outer_block = Block::default()
                    .title(" Offline info ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::White));

                let inner_area = outer_block.inner(chunks[1]);
                frame.render_widget(outer_block, chunks[1]);

                let inner_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(1),
                        Constraint::Length(1),
                        Constraint::Min(0),
                    ])
                    .split(inner_area);
                

                let current_message = if let Some(game_state) = &self.game_state_snapshot {
                    game_state.offline_message_info.clone()
                } 
                else {
                    "Unknown message".into()
                };

                let header_text = Paragraph::new(format!(" Current offline message: {}", current_message))
                    .style(Style::default().fg(Color::Gray));

                frame.render_widget(header_text, inner_layout[0]);

                let separator = Block::default()
                    .borders(Borders::TOP)
                    .border_style(Style::default().fg(Color::DarkGray));
                    frame.render_widget(separator, inner_layout[1]);
                
                let list = List::new(items)
                    .highlight_style(
                        Style::default()
                        .bg(Color::Yellow)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol(">> ");

                frame.render_stateful_widget(list, inner_layout[2], &mut self.select_state);
                
                if self.popup_type == Some(PopupType::TextInput) {
                    let instructions = Line::from(vec![
                        " Confirm ".into(),
                        "<Enter>".bold(),
                        " Cancel ".into(),
                        "<ESC>".bold(),
                    ]);

                    let popup_block = Block::default()
                    .title("Enter offline text")
                    .title_bottom(instructions.centered())
                    .borders(Borders::ALL)
                    .style(Style::default().bg(Color::DarkGray));

                    let popup_area = centered_popup(50, 15, chunks[1]);

                    let entered_text = Paragraph::new(format!("{}█", self.text_input.clone()))
                        .block(popup_block);

                    frame.render_widget(entered_text, popup_area);
                }
            }
        }
    }

    async fn handle_key_event(&mut self, key_event: crossterm::event::KeyEvent) -> io::Result<()> {
        if key_event.kind == KeyEventKind::Press {
            match key_event.code {
                KeyCode::Char(character) => self.char_entered(character),
                KeyCode::Backspace => self.perform_backspace(),
                KeyCode::Down => self.select_next(),
                KeyCode::Up => self.select_previus(),
                KeyCode::Enter => self.perform_action().await,
                KeyCode::Esc => self.perform_escape(),
                _ => {}
            }
        }

        Ok(())
    }
}

fn centered_popup(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    // Cut the given rectangle into three vertical pieces
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    // Then cut the middle vertical piece into three width-wise pieces
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
    .split(popup_layout[1])[1] // Return the middle chunk
}
