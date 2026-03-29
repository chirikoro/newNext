use std::path::Path;
use std::process::Command;
use tokio::sync::watch;

/// Start the development server with file watching
pub async fn run(port: u16) {
    tracing::info!("🚀 Starting Hayabusa dev server on port {}", port);
    tracing::info!("   Watching for file changes...");

    let (tx, mut rx) = watch::channel(false);

    // Spawn file watcher
    let _watcher_handle = tokio::spawn(async move {
        watch_files(tx).await;
    });

    // Initial build and serve
    tracing::info!("   Building project...");

    let status = Command::new("cargo")
        .args(["build"])
        .status();

    match status {
        Ok(s) if s.success() => {
            tracing::info!("   Build successful. Starting server...");

            // Run the project
            let mut child = Command::new("cargo")
                .args(["run", "--", "--port", &port.to_string()])
                .spawn()
                .expect("Failed to start server");

            // Watch for rebuild signals
            loop {
                rx.changed().await.ok();
                tracing::info!("   File change detected. Rebuilding...");

                // Kill the old process
                let _ = child.kill();

                // Rebuild
                let status = Command::new("cargo")
                    .args(["build"])
                    .status();

                if let Ok(s) = status {
                    if s.success() {
                        tracing::info!("   Rebuild successful. Restarting server...");
                        child = Command::new("cargo")
                            .args(["run", "--", "--port", &port.to_string()])
                            .spawn()
                            .expect("Failed to restart server");
                    } else {
                        tracing::error!("   Build failed. Waiting for changes...");
                    }
                }
            }
        }
        _ => {
            tracing::error!("   Initial build failed. Please check your code.");
        }
    }
}

async fn watch_files(tx: watch::Sender<bool>) {
    use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
    use std::sync::mpsc;
    use std::time::Duration;

    let (file_tx, file_rx) = mpsc::channel::<notify::Result<Event>>();

    let mut watcher = RecommendedWatcher::new(file_tx, Config::default())
        .expect("Failed to create file watcher");

    // Watch src/ and app/ directories
    for dir in &["src", "app"] {
        if Path::new(dir).exists() {
            watcher
                .watch(Path::new(dir), RecursiveMode::Recursive)
                .ok();
        }
    }

    // Debounce: only trigger after 500ms of no changes
    let mut last_event = std::time::Instant::now();

    loop {
        match file_rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Ok(_event)) => {
                last_event = std::time::Instant::now();
            }
            Ok(Err(e)) => {
                tracing::warn!("Watch error: {}", e);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if last_event.elapsed() > Duration::from_millis(500)
                    && last_event.elapsed() < Duration::from_millis(600)
                {
                    let _ = tx.send(true);
                }
            }
            Err(_) => break,
        }
    }
}
