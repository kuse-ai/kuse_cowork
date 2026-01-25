use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use super::reader::compute_checksum;
use super::types::*;

/// Manages file watching for Excel files
pub struct ExcelWatcher {
    watchers: Arc<Mutex<HashMap<String, WatcherHandle>>>,
    event_sender: Sender<FileChangeEvent>,
}

struct WatcherHandle {
    #[allow(dead_code)]
    watcher: RecommendedWatcher,
    path: PathBuf,
    last_checksum: String,
}

impl ExcelWatcher {
    /// Create a new Excel watcher
    pub fn new(event_sender: Sender<FileChangeEvent>) -> Self {
        ExcelWatcher {
            watchers: Arc::new(Mutex::new(HashMap::new())),
            event_sender,
        }
    }

    /// Start watching a file for changes
    pub fn watch_file(&self, path: &str) -> Result<(), ExcelError> {
        let path_buf = PathBuf::from(path);

        if !path_buf.exists() {
            return Err(ExcelError::file_not_found(path));
        }

        let mut watchers = self.watchers.lock()
            .map_err(|_| ExcelError::new("Failed to acquire lock", ExcelErrorType::WatchError))?;

        // If already watching, return early
        if watchers.contains_key(path) {
            return Ok(());
        }

        // Compute initial checksum
        let initial_checksum = compute_checksum(path)?;

        // Create event channel for this watcher
        let (tx, rx): (Sender<Result<Event, notify::Error>>, Receiver<Result<Event, notify::Error>>) = channel();

        // Create watcher
        let mut watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            Config::default().with_poll_interval(Duration::from_secs(2)),
        ).map_err(|e| ExcelError::new(format!("Failed to create watcher: {}", e), ExcelErrorType::WatchError))?;

        // Start watching
        watcher.watch(&path_buf, RecursiveMode::NonRecursive)
            .map_err(|e| ExcelError::new(format!("Failed to watch file: {}", e), ExcelErrorType::WatchError))?;

        // Store the watcher handle
        let handle = WatcherHandle {
            watcher,
            path: path_buf.clone(),
            last_checksum: initial_checksum,
        };

        let path_key = path.to_string();
        watchers.insert(path_key.clone(), handle);

        // Spawn a thread to handle events
        let event_sender = self.event_sender.clone();
        let watchers_ref = self.watchers.clone();
        let watched_path = path.to_string();

        thread::spawn(move || {
            process_watch_events(rx, event_sender, watchers_ref, watched_path);
        });

        Ok(())
    }

    /// Stop watching a file
    pub fn unwatch_file(&self, path: &str) -> Result<(), ExcelError> {
        let mut watchers = self.watchers.lock()
            .map_err(|_| ExcelError::new("Failed to acquire lock", ExcelErrorType::WatchError))?;

        watchers.remove(path);
        Ok(())
    }

    /// Check if a file is being watched
    pub fn is_watching(&self, path: &str) -> bool {
        self.watchers.lock()
            .map(|w| w.contains_key(path))
            .unwrap_or(false)
    }

    /// Get list of watched files
    pub fn get_watched_files(&self) -> Vec<String> {
        self.watchers.lock()
            .map(|w| w.keys().cloned().collect())
            .unwrap_or_default()
    }
}

/// Process watch events in a background thread
fn process_watch_events(
    rx: Receiver<Result<Event, notify::Error>>,
    event_sender: Sender<FileChangeEvent>,
    watchers: Arc<Mutex<HashMap<String, WatcherHandle>>>,
    path: String,
) {
    // Debounce mechanism - wait for events to settle
    let mut last_event_time = std::time::Instant::now();
    let debounce_duration = Duration::from_millis(500);

    loop {
        match rx.recv_timeout(Duration::from_secs(1)) {
            Ok(Ok(event)) => {
                // Check if enough time has passed since last event
                let now = std::time::Instant::now();
                if now.duration_since(last_event_time) < debounce_duration {
                    continue;
                }
                last_event_time = now;

                // Process the event
                if let Some(change_event) = process_event(&event, &watchers, &path) {
                    let _ = event_sender.send(change_event);
                }
            }
            Ok(Err(e)) => {
                eprintln!("Watch error for {}: {}", path, e);
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // Check if we should still be running
                let should_continue = watchers.lock()
                    .map(|w| w.contains_key(&path))
                    .unwrap_or(false);

                if !should_continue {
                    break;
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                break;
            }
        }
    }
}

/// Process a single watch event
fn process_event(
    event: &Event,
    watchers: &Arc<Mutex<HashMap<String, WatcherHandle>>>,
    path: &str,
) -> Option<FileChangeEvent> {
    let change_type = match event.kind {
        EventKind::Modify(_) => FileChangeType::Modified,
        EventKind::Remove(_) => FileChangeType::Deleted,
        EventKind::Create(_) => {
            // File might have been recreated after deletion
            FileChangeType::Modified
        }
        _ => return None,
    };

    // For modifications, check if the checksum actually changed
    if matches!(change_type, FileChangeType::Modified) {
        if let Ok(mut watchers_guard) = watchers.lock() {
            if let Some(handle) = watchers_guard.get_mut(path) {
                if let Ok(new_checksum) = compute_checksum(path) {
                    if new_checksum == handle.last_checksum {
                        // Checksum hasn't changed, ignore this event
                        return None;
                    }
                    handle.last_checksum = new_checksum.clone();

                    return Some(FileChangeEvent {
                        path: path.to_string(),
                        change_type,
                        new_checksum: Some(new_checksum),
                    });
                }
            }
        }
    }

    Some(FileChangeEvent {
        path: path.to_string(),
        change_type,
        new_checksum: None,
    })
}

/// Create a channel for receiving file change events
pub fn create_event_channel() -> (Sender<FileChangeEvent>, Receiver<FileChangeEvent>) {
    channel()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests would require creating actual files and waiting for events
}
