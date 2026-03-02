use std::io;
use std::time::Duration;

use crossterm::event::{KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::app::App;
use crate::error::TuiError;
use crate::events::{EventHandler, TerminalEvent};
use crate::ui;

pub fn run(mut app: App) -> Result<(), TuiError> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).map_err(|e| TuiError::Io(io::Error::other(e)))?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_loop(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .map_err(|e| TuiError::Io(io::Error::other(e)))?;
    terminal.show_cursor()?;

    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<(), TuiError> {
    let event_handler = EventHandler::new(Duration::from_millis(250));

    loop {
        terminal.draw(|frame| ui::draw(frame, app))?;

        match event_handler.next()? {
            TerminalEvent::Key(key) if key.kind == KeyEventKind::Press => {
                handle_key(app, key.code);
            }
            TerminalEvent::Key(_) | TerminalEvent::Tick | TerminalEvent::Resize(_, _) => {}
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn handle_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('q') | KeyCode::Esc => app.quit(),
        KeyCode::Char('j') | KeyCode::Down => app.next(),
        KeyCode::Char('k') | KeyCode::Up => app.previous(),
        KeyCode::Enter => app.toggle_detail(),
        _ => {}
    }
}
