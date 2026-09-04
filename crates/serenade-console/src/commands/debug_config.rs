//! `debug:config` - flattened parameters (plain or ratatui), secured.

use std::io::stdout;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::prelude::{Constraint, CrosstermBackend, Layout, Rect, Style, Terminal};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};

use crate::application::stdout_is_terminal;
use crate::{Command, ConsoleError, Input};

/// Dumps DI parameters. Requires debug mode; redacts secret-like keys by default.
#[derive(Clone, Copy, Debug, Default)]
pub struct DebugConfigCommand;

impl Command for DebugConfigCommand {
    fn name(&self) -> &'static str {
        "debug:config"
    }

    fn description(&self) -> &'static str {
        "Dump config parameters (debug only; secrets redacted unless --reveal)"
    }

    fn execute(&self, input: &Input) -> Result<(), ConsoleError> {
        if !input.debug() {
            return Err(ConsoleError::Failed(
                "debug:config requires debug mode (omit --no-debug; refuse in non-debug prod)"
                    .to_owned(),
            ));
        }
        let reveal = input.args().iter().any(|arg| arg == "--reveal");
        let want_plain = input.args().iter().any(|arg| arg == "--plain");
        let prefix = input
            .args()
            .iter()
            .find(|arg| !arg.starts_with('-'))
            .map(String::as_str);
        let rows = collect_rows(input, prefix, reveal);
        if want_plain || !stdout_is_terminal() {
            print_plain(input, reveal, &rows);
            return Ok(());
        }
        run_tui(input, reveal, &rows)
    }
}

fn collect_rows(input: &Input, prefix: Option<&str>, reveal: bool) -> Vec<(String, String)> {
    let Some(container) = input.container() else {
        return Vec::new();
    };
    let mut rows: Vec<(String, String)> = container
        .parameters()
        .iter()
        .filter(|(key, _)| prefix.map_or(true, |prefix| key.starts_with(prefix)))
        .map(|(key, value)| {
            let display = if reveal || !is_sensitive(key) {
                value.to_owned()
            } else {
                "***".to_owned()
            };
            (key.to_owned(), display)
        })
        .collect();
    rows.sort_by(|left, right| left.0.cmp(&right.0));
    rows
}

fn is_sensitive(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    [
        "password",
        "passwd",
        "secret",
        "token",
        "api_key",
        "apikey",
        "private_key",
        "credential",
        "dsn",
        "auth",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn print_plain(input: &Input, reveal: bool, rows: &[(String, String)]) {
    println!("Environment: {}", input.environment());
    println!("Debug: {}", input.debug());
    println!("Reveal: {reveal}");
    if rows.is_empty() {
        println!("Parameters: (none - pass a container via Application::run_with)");
        return;
    }
    println!("Parameters ({}):", rows.len());
    for (key, value) in rows {
        println!("  {key} = {value}");
    }
}

fn run_tui(input: &Input, reveal: bool, rows: &[(String, String)]) -> Result<(), ConsoleError> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let result = draw_loop(input, reveal, rows);
    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;
    result
}

fn draw_loop(input: &Input, reveal: bool, rows: &[(String, String)]) -> Result<(), ConsoleError> {
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            render(frame, area, input, reveal, rows);
        })?;

        if event::poll(std::time::Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press
                    && matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
                {
                    break;
                }
            }
        }
    }
    Ok(())
}

fn render(
    frame: &mut ratatui::Frame,
    area: Rect,
    input: &Input,
    reveal: bool,
    rows: &[(String, String)],
) {
    let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(3)]).split(area);
    let header = Paragraph::new(format!(
        "debug:config  env={}  debug={}  reveal={}  (q/Esc quit)",
        input.environment(),
        input.debug(),
        reveal
    ))
    .block(Block::default().borders(Borders::ALL).title("Serenade"));
    frame.render_widget(header, chunks[0]);

    let table_rows: Vec<Row> = if rows.is_empty() {
        vec![Row::new(vec!["(no parameters)".to_owned(), String::new()])]
    } else {
        rows.iter()
            .map(|(key, value)| Row::new(vec![key.clone(), value.clone()]))
            .collect()
    };
    let table = Table::new(
        table_rows,
        [Constraint::Percentage(45), Constraint::Percentage(55)],
    )
    .header(Row::new(vec!["Key", "Value"]).style(Style::new().bold()))
    .block(Block::default().borders(Borders::ALL).title("Parameters"));
    frame.render_widget(table, chunks[1]);
}
