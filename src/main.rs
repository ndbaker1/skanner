use std::io::stdout;

use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::*, widgets::*};
use sneakr::{OmniScanner, Process, ValueScanner};

fn main() -> Result<()> {
    let pid = std::env::args().nth(1).unwrap().parse()?;

    let proc = Process::new(pid);
    let scanner = OmniScanner::new(&proc);

    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut app = App {
        scanner,
        address_list: Default::default(),
        address_list_state: ListState::default().with_selected(Some(0)),
    };

    loop {
        terminal.draw(|f| ui(f, &mut app))?;
        if run_app(&mut app)? {
            break;
        }
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

struct App<'a> {
    scanner: OmniScanner<'a>,
    address_list_state: ListState,
    address_list: Vec<String>,
}

fn run_app(app: &mut App) -> Result<bool> {
    if event::poll(std::time::Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') => return Ok(true),
                    KeyCode::Char('/') => return Ok(true),
                    KeyCode::Char('j') => {
                        app.address_list_state.select(
                            app.address_list_state
                                .selected()
                                .map(|o| o.saturating_add(1).max(app.address_list.len())),
                        );
                    }
                    KeyCode::Char('k') => {
                        app.address_list_state.select(
                            app.address_list_state
                                .selected()
                                .map(|o| o.saturating_sub(1)),
                        );
                    }
                    KeyCode::Char('s') => {
                        app.address_list = app
                            .scanner
                            .find_values(&4.0, |a, b| a == b, sneakr::ValueScanType::First)?
                            .into_iter()
                            .map(|s| format!("{:#02x}", s))
                            .collect();
                    }
                    KeyCode::Char('4') => {
                        app.address_list = app
                            .scanner
                            .find_values(&4.0, |a, b| a == b, sneakr::ValueScanType::Next)?
                            .into_iter()
                            .map(|s| format!("{:#02x}", s))
                            .collect();
                    }
                    KeyCode::Char('3') => {
                        app.address_list = app
                            .scanner
                            .find_values(&3.0, |a, b| a == b, sneakr::ValueScanType::Next)?
                            .into_iter()
                            .map(|s| format!("{:#02x}", s))
                            .collect();
                    }
                    KeyCode::Char('2') => {
                        app.address_list = app
                            .scanner
                            .find_values(&2.0, |a, b| a == b, sneakr::ValueScanType::Next)?
                            .into_iter()
                            .map(|s| format!("{:#02x}", s))
                            .collect();
                    }
                    KeyCode::Char('1') => {
                        app.address_list = app
                            .scanner
                            .find_values(&1.0, |a, b| a == b, sneakr::ValueScanType::Next)?
                            .into_iter()
                            .map(|s| format!("{:#02x}", s))
                            .collect();
                    }
                    _ => (),
                };
            };
        }
    }
    Ok(false)
}

fn ui(frame: &mut Frame, app: &mut App) {
    let list = List::new(app.address_list.clone())
        .block(Block::default().title("List").borders(Borders::ALL))
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
        .highlight_symbol(">>")
        .repeat_highlight_symbol(true);

    frame.render_stateful_widget(list, frame.size(), &mut app.address_list_state);
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
