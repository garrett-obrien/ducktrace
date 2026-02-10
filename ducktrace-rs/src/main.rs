mod app;
mod data;
mod db;
mod ui;
mod watcher;

use std::fs::File;
use std::io;
use std::sync::OnceLock;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use log::{info, error, debug};
use ratatui::prelude::*;
use simplelog::{Config, LevelFilter, WriteLogger};
use tokio::sync::mpsc;

use app::App;
use data::{ChartData, ExplainData};
use db::QueryExecutor;

/// Lazy-initialized MotherDuck executor (connects on first drill-down)
static EXECUTOR: OnceLock<Option<QueryExecutor>> = OnceLock::new();

enum AppEvent {
    Key(crossterm::event::KeyEvent),
    Mouse(crossterm::event::MouseEvent),
    FileChange(Box<ChartData>),
    DrillDownResult(Result<ExplainData, String>),
    Tick,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file (from current dir or parent dirs)
    let _ = dotenvy::dotenv();

    // Initialize file logger
    let log_path = dirs::home_dir()
        .unwrap_or_default()
        .join(".claude/ducktrace/ducktrace.log");
    if let Ok(log_file) = File::create(&log_path) {
        let _ = WriteLogger::init(LevelFilter::Debug, Config::default(), log_file);
        info!("DuckTrace TUI started");
    }

    // Set up terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new();

    // Set up event channel
    let (tx, mut rx) = mpsc::channel::<AppEvent>(32);

    // Spawn file watcher with adapter channel
    let watcher_tx = tx.clone();
    tokio::spawn(async move {
        let (data_tx, mut data_rx) = mpsc::channel::<ChartData>(16);

        // Spawn the watcher
        let watcher_handle = tokio::spawn(async move {
            if let Err(e) = watcher::watch_file(data_tx).await {
                eprintln!("File watcher error: {}", e);
            }
        });

        // Forward data events to main channel
        while let Some(data) = data_rx.recv().await {
            if watcher_tx.send(AppEvent::FileChange(Box::new(data))).await.is_err() {
                break;
            }
        }

        let _ = watcher_handle.await;
    });

    // Spawn input event handler (keyboard + mouse)
    let input_tx = tx.clone();
    tokio::spawn(async move {
        loop {
            if event::poll(Duration::from_millis(50)).unwrap_or(false) {
                match event::read() {
                    Ok(Event::Key(key)) => {
                        if key.kind == KeyEventKind::Press
                            && input_tx.send(AppEvent::Key(key)).await.is_err()
                        {
                            break;
                        }
                    }
                    Ok(Event::Mouse(mouse)) => {
                        if input_tx.send(AppEvent::Mouse(mouse)).await.is_err() {
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }
    });

    // Clone tx for drilldown queries before moving to tick generator
    let drilldown_tx = tx.clone();

    // Spawn tick generator for animations
    let tick_tx = tx;
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(100));
        loop {
            interval.tick().await;
            if tick_tx.send(AppEvent::Tick).await.is_err() {
                break;
            }
        }
    });

    // Main event loop
    loop {
        // Draw UI
        terminal.draw(|f| ui::render(f, &mut app))?;

        // Check for drill-down request
        if let Some(query) = app.take_pending_drill_down() {
            info!("Drill-down query requested");
            debug!("Query: {}", query);
            let tx_clone = drilldown_tx.clone();
            tokio::task::spawn_blocking(move || {
                // Lazy-initialize executor on first drill-down
                let executor = EXECUTOR.get_or_init(|| {
                    info!("Initializing MotherDuck connection");
                    match QueryExecutor::connect() {
                        Ok(exec) => {
                            info!("MotherDuck connection successful");
                            Some(exec)
                        }
                        Err(e) => {
                            error!("MotherDuck connection failed: {}", e);
                            None
                        }
                    }
                });

                let event = match executor {
                    Some(exec) => {
                        info!("Executing drill-down query");
                        match exec.execute_drill_down(&query) {
                            Ok((columns, rows)) => {
                                info!("Drill-down success: {} columns, {} rows", columns.len(), rows.len());
                                let explain_data = ExplainData {
                                    title: "Drill-Down Results".to_string(),
                                    response_to_command: None,
                                    columns,
                                    rows,
                                    total_count: None,
                                };
                                AppEvent::DrillDownResult(Ok(explain_data))
                            }
                            Err(e) => {
                                error!("Drill-down query failed: {}", e);
                                AppEvent::DrillDownResult(Err(e.to_string()))
                            }
                        }
                    }
                    None => {
                        error!("No MotherDuck executor available");
                        AppEvent::DrillDownResult(Err(
                            "MotherDuck not connected. Set MOTHERDUCK_TOKEN environment variable.".to_string()
                        ))
                    }
                };
                let _ = tx_clone.blocking_send(event);
            });
        }

        // Handle events
        if let Some(event) = rx.recv().await {
            match event {
                AppEvent::Key(key) => app.handle_key(key),
                AppEvent::Mouse(mouse) => app.handle_mouse(mouse),
                AppEvent::FileChange(data) => app.on_data_update(*data),
                AppEvent::DrillDownResult(result) => match result {
                    Ok(data) => app.on_drill_down_success(data),
                    Err(e) => app.on_drill_down_error(e),
                },
                AppEvent::Tick => app.tick(),
            }
        }

        if !app.running {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Force exit — background tokio tasks (watcher, input, tick) would otherwise keep the process alive
    std::process::exit(0);
}
