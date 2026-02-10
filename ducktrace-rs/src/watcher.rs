use anyhow::Result;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::data::ChartData;

/// Get the path to the data file
pub fn get_data_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".claude/ducktrace/current.json")
}

/// Load chart data from the file
pub fn load_data(path: &PathBuf) -> Result<ChartData> {
    let content = std::fs::read_to_string(path)?;
    let data: ChartData = serde_json::from_str(&content)?;
    Ok(data)
}

/// Watch the data file and send updates through the channel
pub async fn watch_file(tx: mpsc::Sender<ChartData>) -> Result<()> {
    let path = get_data_path();

    // Create directory if it doesn't exist
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Try to load initial data
    if path.exists() {
        if let Ok(data) = load_data(&path) {
            let _ = tx.send(data).await;
        }
    }

    // Set up file watcher
    let (watcher_tx, mut watcher_rx) = tokio::sync::mpsc::channel::<notify::Result<notify::Event>>(16);

    let mut watcher = RecommendedWatcher::new(
        move |res| {
            let _ = watcher_tx.blocking_send(res);
        },
        Config::default().with_poll_interval(Duration::from_millis(100)),
    )?;

    // Watch the parent directory
    if let Some(parent) = path.parent() {
        watcher.watch(parent, RecursiveMode::NonRecursive)?;
    }

    // Keep watcher alive and process events
    loop {
        if let Some(Ok(event)) = watcher_rx.recv().await {
            // Check if this event is for our file
            let is_our_file = event.paths.iter().any(|p| {
                p.file_name()
                    .map(|n| n == "current.json")
                    .unwrap_or(false)
            });

            if is_our_file {
                // Small delay to ensure file is fully written
                tokio::time::sleep(Duration::from_millis(50)).await;

                if let Ok(data) = load_data(&path) {
                    let _ = tx.send(data).await;
                }
            }
        }
    }
}
