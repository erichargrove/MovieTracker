use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, stdout};
use std::path::Path;

#[derive(Clone, Serialize, Deserialize)]
struct Movie {
    year: u32,
    watched: bool,
    movie: String,
}

struct App {
    movies: Vec<Movie>,
    selected: usize,
    save_path: String,
    list_state: ListState,
}

impl App {
    fn new(save_path: &str) -> Self {
        let movies = Self::load_from_file(save_path);
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        
        Self {
            movies,
            selected: 0,
            save_path: save_path.to_string(),
            list_state,
        }
    }

    fn load_from_file(path: &str) -> Vec<Movie> {
        if Path::new(path).exists() {
            match fs::read_to_string(path) {
                Ok(contents) => {
                    match serde_json::from_str::<Vec<Movie>>(&contents) {
                        Ok(movies) => return movies,
                        Err(e) => {
                            eprintln!("Error parsing JSON: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error reading file: {}", e);
                }
            }
        }
        
        // Default empty list if file doesn't exist or can't be loaded
        vec![]
    }

    fn save_to_file(&self) -> io::Result<()> {
        let json = serde_json::to_string_pretty(&self.movies)?;
        fs::write(&self.save_path, json)?;
        Ok(())
    }

    fn toggle_current(&mut self) {
        if !self.movies.is_empty() {
            self.movies[self.selected].watched = !self.movies[self.selected].watched;
            let _ = self.save_to_file(); // Auto-save on change
        }
    }

    fn next(&mut self) {
        if !self.movies.is_empty() {
            self.selected = (self.selected + 1) % self.movies.len();
            self.list_state.select(Some(self.selected));
        }
    }

    fn previous(&mut self) {
        if !self.movies.is_empty() {
            self.selected = if self.selected == 0 {
                self.movies.len() - 1
            } else {
                self.selected - 1
            };
            self.list_state.select(Some(self.selected));
        }
    }

    fn get_stats(&self) -> (usize, usize) {
        let watched = self.movies.iter().filter(|m| m.watched).count();
        let total = self.movies.len();
        (watched, total)
    }
}

fn main() -> io::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let save_path = "movies.json";
    let mut app = App::new(save_path);
    let mut should_quit = false;

    while !should_quit {
        // Draw UI
        terminal.draw(|frame| {
            let chunks = Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(3),
                ])
                .split(frame.area());

            // Title with stats
            let (watched, total) = app.get_stats();
            let title = Paragraph::new(format!(
                "Movie Watchlist ({}/{} watched)",
                watched, total
            ))
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL));
            frame.render_widget(title, chunks[0]);

            // Movie list
            let items: Vec<ListItem> = app
                .movies
                .iter()
                .map(|movie| {
                    let checkbox = if movie.watched { "[✓]" } else { "[ ]" };
                    let content = format!("{} {} - {}", checkbox, movie.year, movie.movie);
                    
                    let style = if movie.watched {
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::DIM)
                    } else {
                        Style::default().fg(Color::White)
                    };

                    ListItem::new(Line::from(Span::styled(content, style)))
                })
                .collect();

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("Movies"))
                .highlight_style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                )
                .highlight_symbol("► ");
            
            frame.render_stateful_widget(list, chunks[1], &mut app.list_state);

            // Help text
            let help = Paragraph::new("↑/↓: Navigate | Space: Toggle Watched | q: Quit")
                .style(Style::default().fg(Color::Gray))
                .block(Block::default().borders(Borders::ALL));
            frame.render_widget(help, chunks[2]);
        })?;

        // Handle input
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => should_quit = true,
                        KeyCode::Char(' ') => app.toggle_current(),
                        KeyCode::Down | KeyCode::Char('j') => app.next(),
                        KeyCode::Up | KeyCode::Char('k') => app.previous(),
                        _ => {}
                    }
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    Ok(())
}
