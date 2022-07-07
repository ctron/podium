use clap::Parser;
use log::LevelFilter;
use podwatch::{app::App, start_ui, Args};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let app = Arc::new(tokio::sync::Mutex::new(App::new(args.clone())));
    let app_ui = Arc::clone(&app);

    // Configure log
    let level = match args.verbose {
        0 => LevelFilter::Warn,
        1 => LevelFilter::Info,
        2 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };
    tui_logger::init_logger(level).unwrap();
    tui_logger::set_default_level(level);

    start_ui(&app_ui).await?;
    Ok(())
}
