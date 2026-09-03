//! `debug:container` - service list (plain or ratatui).

use std::io::stdout;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::prelude::{Constraint, CrosstermBackend, Layout, Rect, Style, Stylize, Terminal};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};

use crate::application::stdout_is_terminal;
use crate::{Command, ConsoleError, Input};

/// Lists DI service ids: plain text, or a small TUI when stdout is a TTY.
#[derive(Clone, Copy, Debug, Default)]
pub struct DebugContainerCommand;

impl Command for DebugContainerCommand {
    fn name(&self) -> &'static str {
        "debug:container"
    }

    fn description(&self) -> &'static str {
        "List DI services (ratatui when interactive)"
    }

    fn execute(&self, input: &Input) -> Result<(), ConsoleError> {
        let ids = service_ids(input);
        let want_plain = input.args().iter().any(|arg| arg == "--plain");
        if want_plain || !stdout_is_terminal() {
            print_plain(input, &ids);
            return Ok(());
        }
        run_tui(input, &ids)
    }
}

fn service_ids(input: &Input) -> Vec<String> {
    input.container().map_or_else(Vec::new, |container| {
        let mut ids: Vec<String> = container
            .definitions()
            .iter()
            .map(|definition| definition.id().to_owned())
            .collect();
        ids.sort();
        ids
    })
}

fn print_plain(input: &Input, ids: &[String]) {
    println!("Environment: {}", input.environment());
    println!("Debug: {}", input.debug());
    if ids.is_empty() {
        println!("Services: (none — pass a container via Application::run_with)");
        return;
    }
    println!("Services ({}):", ids.len());
    for id in ids {
        println!("  {id}");
    }
}

fn run_tui(input: &Input, ids: &[String]) -> Result<(), ConsoleError> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let result = draw_loop(input, ids);
    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;
    result
}

fn draw_loop(input: &Input, ids: &[String]) -> Result<(), ConsoleError> {
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            render(frame, area, input, ids);
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

fn render(frame: &mut ratatui::Frame, area: Rect, input: &Input, ids: &[String]) {
    let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(3)]).split(area);
    let header = Paragraph::new(format!(
        "debug:container  env={}  debug={}  (q/Esc quit)",
        input.environment(),
        input.debug()
    ))
    .block(Block::default().borders(Borders::ALL).title("Serenade"));
    frame.render_widget(header, chunks[0]);

    let rows: Vec<Row> = if ids.is_empty() {
        vec![Row::new(vec!["(no services)".to_owned()])]
    } else {
        ids.iter().map(|id| Row::new(vec![id.clone()])).collect()
    };
    let table = Table::new(rows, [Constraint::Percentage(100)])
        .header(Row::new(vec!["Service id"]).style(Style::new().bold()))
        .block(Block::default().borders(Borders::ALL).title("Container"));
    frame.render_widget(table, chunks[1]);
}
