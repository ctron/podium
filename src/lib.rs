pub mod app;
pub mod input;
pub mod k8s;

mod client;

use crate::app::{ui, AppReturn};
use crate::input::events::Events;
use crate::input::InputEvent;
use app::App;
use clap::Parser;
use std::io::stdout;
use std::sync::Arc;
use std::time::Duration;
use tui::backend::CrosstermBackend;
use tui::Terminal;

/// Watch pods
#[derive(Parser, Debug, Clone)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    /// Namespace
    #[clap(short, long, value_parser)]
    pub namespace: Option<String>,
    /// Verbose
    #[clap(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

pub async fn start_ui(app: &Arc<tokio::sync::Mutex<App>>) -> anyhow::Result<()> {
    // Configure Crossterm backend for tui
    let stdout = stdout();
    crossterm::terminal::enable_raw_mode()?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    terminal.hide_cursor()?;

    let render_rate = Duration::from_millis(200);
    let mut events = Events::new(render_rate);

    loop {
        // Handle inputs

        let event = events.next().await;
        {
            let mut app = app.lock().await;
            match event {
                InputEvent::Input(key) => {
                    if let AppReturn::Exit = app.do_action(key).await {
                        break;
                    }
                }
                InputEvent::Render => {}
                InputEvent::Quit => {
                    break;
                }
            }
            // always render after a change
            terminal.draw(|rect| ui::draw(rect, &app))?;
        }
    }

    // Restore the terminal and close application
    terminal.clear()?;
    terminal.show_cursor()?;
    crossterm::terminal::disable_raw_mode()?;

    Ok(())
}
