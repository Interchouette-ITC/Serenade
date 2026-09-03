//! Interactive console loop with ↑/↓ history (`rustyline`).

use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;

use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use serenade_di::Container;
use serenade_kernel::Environment;

use crate::{Application, ConsoleError, Input};
use std::sync::Arc;

/// Default history file under `$HOME`.
pub const HISTORY_FILE_NAME: &str = ".serenade_history";

/// Resolves `~/.serenade_history` when `HOME` is set.
#[must_use]
pub fn history_path() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(HISTORY_FILE_NAME))
}

impl Application {
    /// Runs an interactive prompt with line editing and persistent history.
    ///
    /// TTY: `rustyline` (↑/↓ history). Non-TTY: plain stdin lines.
    ///
    /// # Errors
    ///
    /// Returns [`ConsoleError`] on I/O or command failure.
    pub fn run_interactive(
        &self,
        environment: &Environment,
        debug: bool,
        container: Option<&Arc<Container>>,
    ) -> Result<(), ConsoleError> {
        let mut out = io::stdout();
        writeln!(
            out,
            "Serenade interactive console - type a command, `list`, or `quit` (↑ history)"
        )?;
        out.flush()?;
        self.print_list()?;

        if io::stdin().is_terminal() {
            self.run_interactive_readline(environment, debug, container)
        } else {
            self.run_interactive_plain(environment, debug, container)
        }
    }

    fn run_interactive_readline(
        &self,
        environment: &Environment,
        debug: bool,
        container: Option<&Arc<Container>>,
    ) -> Result<(), ConsoleError> {
        let mut rl =
            DefaultEditor::new().map_err(|error| ConsoleError::Failed(error.to_string()))?;
        let hist = history_path();
        if let Some(path) = hist.as_ref() {
            let _ = rl.load_history(path);
        }

        loop {
            match rl.readline("serenade> ") {
                Ok(line) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    let _ = rl.add_history_entry(trimmed);
                    if self.dispatch_interactive_line(trimmed, environment, debug, container)? {
                        break;
                    }
                }
                Err(ReadlineError::Interrupted) => {}
                Err(ReadlineError::Eof) => {
                    println!();
                    break;
                }
                Err(error) => return Err(ConsoleError::Failed(error.to_string())),
            }
        }

        if let Some(path) = hist.as_ref() {
            let _ = rl.save_history(path);
        }
        Ok(())
    }

    fn run_interactive_plain(
        &self,
        environment: &Environment,
        debug: bool,
        container: Option<&Arc<Container>>,
    ) -> Result<(), ConsoleError> {
        let mut out = io::stdout();
        loop {
            write!(out, "serenade> ")?;
            out.flush()?;
            let mut line = String::new();
            let n = io::stdin().read_line(&mut line)?;
            if n == 0 {
                writeln!(out)?;
                break;
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if self.dispatch_interactive_line(trimmed, environment, debug, container)? {
                break;
            }
        }
        Ok(())
    }

    /// Returns `true` when the session should exit.
    pub(crate) fn dispatch_interactive_line(
        &self,
        line: &str,
        environment: &Environment,
        debug: bool,
        container: Option<&Arc<Container>>,
    ) -> Result<bool, ConsoleError> {
        let mut parts = line.split_whitespace();
        let Some(name) = parts.next() else {
            return Ok(false);
        };
        if matches!(name, "quit" | "exit" | "q") {
            return Ok(true);
        }
        if name == "list" {
            self.print_list()?;
            return Ok(false);
        }
        let trailing: Vec<String> = parts.map(str::to_owned).collect();
        let Some(command) = self.find(name) else {
            eprintln!("command `{name}` was not found; type `list`");
            return Ok(false);
        };
        let input = Input::new(environment.clone(), debug, trailing, container.cloned());
        if let Err(error) = command.execute(&input) {
            eprintln!("{error}");
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Command;
    use std::sync::Arc;

    struct Ping;

    impl Command for Ping {
        fn name(&self) -> &'static str {
            "app:ping"
        }

        fn description(&self) -> &'static str {
            "pong"
        }

        fn execute(&self, _input: &Input) -> Result<(), ConsoleError> {
            Ok(())
        }
    }

    #[test]
    fn history_path_uses_home_when_set() {
        let path = history_path();
        if std::env::var_os("HOME").is_some() {
            let path = path.expect("HOME set");
            assert!(path.ends_with(HISTORY_FILE_NAME));
        }
    }

    #[test]
    fn dispatch_quit_and_unknown() {
        let mut app = Application::new();
        app.add(Arc::new(Ping));
        assert!(app
            .dispatch_interactive_line("quit", &Environment::Dev, true, None)
            .expect("quit"));
        assert!(!app
            .dispatch_interactive_line("missing:cmd", &Environment::Dev, true, None)
            .expect("missing"));
        assert!(!app
            .dispatch_interactive_line("app:ping", &Environment::Dev, true, None)
            .expect("ping"));
    }
}
