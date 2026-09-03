//! Guided ratatui UI for picking and applying a recipe (`serenade tui`).

use std::io::{self, stdout, IsTerminal};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::prelude::{Constraint, CrosstermBackend, Layout, Style, Terminal};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use crate::error::CliError;
use crate::recipe::{apply_recipe, list_recipes, print_hints, ApplyOptions, Recipe};

/// Runs an interactive recipe picker and applies the selection.
///
/// # Errors
///
/// Non-TTY, IO, apply failure, or user cancel (`q` / Esc).
pub fn run_recipe_tui(options: &ApplyOptions) -> Result<Recipe, CliError> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(CliError::NotATty);
    }

    let recipes = list_recipes();
    if recipes.is_empty() {
        return Err(CliError::InvalidRecipe("no embedded recipes".into()));
    }

    let selection = pick_recipe(&recipes, options)?;
    let Some(id) = selection else {
        return Err(CliError::Cancelled);
    };

    let recipe = apply_recipe(&id, options)?;
    println!(
        "applied recipe `{}` into {}",
        recipe.id,
        options.root.display()
    );
    print_hints(&recipe);
    Ok(recipe)
}

fn pick_recipe(
    recipes: &[(String, String)],
    options: &ApplyOptions,
) -> Result<Option<String>, CliError> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let result = run_picker(recipes, options);
    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;
    result
}

fn run_picker(
    recipes: &[(String, String)],
    options: &ApplyOptions,
) -> Result<Option<String>, CliError> {
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    let mut state = ListState::default().with_selected(Some(0));

    loop {
        terminal.draw(|frame| {
            render(frame, recipes, options, &mut state);
        })?;

        if !event::poll(std::time::Duration::from_millis(200))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(None),
            KeyCode::Enter => {
                let index = state.selected().unwrap_or(0);
                return Ok(recipes.get(index).map(|(id, _)| id.clone()));
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let next = state
                    .selected()
                    .map_or(0, |i| (i + 1).min(recipes.len() - 1));
                state.select(Some(next));
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let next = state.selected().map_or(0, |i| i.saturating_sub(1));
                state.select(Some(next));
            }
            _ => {}
        }
    }
}

fn render(
    frame: &mut ratatui::Frame,
    recipes: &[(String, String)],
    options: &ApplyOptions,
    state: &mut ListState,
) {
    let area = frame.area();
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(5),
        Constraint::Length(3),
    ])
    .split(area);

    let header = Paragraph::new(format!(
        "root={}  force={}  no-cargo={}  (↑/↓ move, Enter apply, q quit)",
        options.root.display(),
        options.force,
        options.no_cargo
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("serenade tui - pick a recipe"),
    );
    frame.render_widget(header, chunks[0]);

    let items: Vec<ListItem> = recipes
        .iter()
        .map(|(id, description)| ListItem::new(format!("{id}  -  {description}")))
        .collect();
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Recipes"))
        .highlight_style(Style::new().bold().reversed())
        .highlight_symbol("> ");
    frame.render_stateful_widget(list, chunks[1], state);

    let footer = Paragraph::new("Same apply path as `serenade recipe apply <id>`")
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, chunks[2]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_a_tty_without_terminal() {
        // Non-interactive CI: stdin/stdout are typically not TTYs.
        if io::stdin().is_terminal() && io::stdout().is_terminal() {
            return;
        }
        let err = run_recipe_tui(&ApplyOptions {
            no_cargo: true,
            ..ApplyOptions::default()
        })
        .expect_err("tui on pipe");
        assert!(matches!(err, CliError::NotATty));
    }
}
