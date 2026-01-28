use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::time::Duration;
use tokio::sync::Mutex;
use crate::capture::{
    CaptureBuffer, ClipboardCapture, CaptureConfig,
    hash_content, generate_preview, MAX_CLIPBOARD_SIZE, PREVIEW_LENGTH
};

/// Clipboard monitor that polls the system clipboard
pub struct ClipboardMonitor {
    buffer: Arc<CaptureBuffer>,
    config: Arc<Mutex<CaptureConfig>>,
    running: Arc<AtomicBool>,
    /// Current active source URL (set by BrowserPanel when user is on a page)
    active_source_url: Arc<Mutex<Option<String>>>,
    /// Current active source title
    active_source_title: Arc<Mutex<Option<String>>>,
}

impl ClipboardMonitor {
    pub fn new(buffer: Arc<CaptureBuffer>, config: Arc<Mutex<CaptureConfig>>) -> Self {
        Self {
            buffer,
            config,
            running: Arc::new(AtomicBool::new(false)),
            active_source_url: Arc::new(Mutex::new(None)),
            active_source_title: Arc::new(Mutex::new(None)),
        }
    }

    /// Set the current active source (called when user views a page)
    pub async fn set_active_source(&self, url: Option<String>, title: Option<String>) {
        *self.active_source_url.lock().await = url;
        *self.active_source_title.lock().await = title;
    }

    /// Start the clipboard monitoring loop
    pub async fn start(&self) {
        if self.running.load(Ordering::SeqCst) {
            return;
        }

        self.running.store(true, Ordering::SeqCst);

        let buffer = self.buffer.clone();
        let config = self.config.clone();
        let running = self.running.clone();
        let active_url = self.active_source_url.clone();
        let active_title = self.active_source_title.clone();

        tokio::spawn(async move {
            let mut last_content_hash: Option<String> = None;

            while running.load(Ordering::SeqCst) {
                let cfg = config.lock().await;

                if !cfg.clipboard_enabled {
                    // Clipboard monitoring disabled, wait and check again
                    drop(cfg);
                    tokio::time::sleep(Duration::from_millis(1000)).await;
                    continue;
                }

                let poll_interval = cfg.clipboard_poll_ms as u64;
                drop(cfg);

                // Read clipboard content
                if let Some(content) = read_clipboard_content() {
                    // Skip if content is too large
                    if content.len() > MAX_CLIPBOARD_SIZE {
                        tokio::time::sleep(Duration::from_millis(poll_interval)).await;
                        continue;
                    }

                    // Skip empty content
                    if content.trim().is_empty() {
                        tokio::time::sleep(Duration::from_millis(poll_interval)).await;
                        continue;
                    }

                    // Calculate hash
                    let content_hash = hash_content(&content);

                    // Skip if same as last capture
                    if last_content_hash.as_ref() == Some(&content_hash) {
                        tokio::time::sleep(Duration::from_millis(poll_interval)).await;
                        continue;
                    }

                    last_content_hash = Some(content_hash.clone());

                    // Get current active source
                    let source_url = active_url.lock().await.clone();
                    let source_title = active_title.lock().await.clone();

                    // Create capture
                    let capture = ClipboardCapture {
                        id: uuid::Uuid::new_v4().to_string(),
                        content_hash,
                        content_preview: generate_preview(&content, PREVIEW_LENGTH),
                        source_url,
                        source_title,
                        captured_at: chrono::Utc::now().timestamp_millis(),
                    };

                    // Push to buffer (will deduplicate)
                    buffer.push_clipboard(capture);
                }

                tokio::time::sleep(Duration::from_millis(poll_interval)).await;
            }
        });
    }

    /// Stop the clipboard monitoring loop
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Check if monitor is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

/// Read text content from system clipboard
fn read_clipboard_content() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let output = Command::new("pbpaste")
            .output()
            .ok()?;

        if output.status.success() {
            String::from_utf8(output.stdout).ok()
        } else {
            None
        }
    }

    #[cfg(target_os = "windows")]
    {
        // Use powershell to read clipboard on Windows
        use std::process::Command;
        let output = Command::new("powershell")
            .args(["-command", "Get-Clipboard"])
            .output()
            .ok()?;

        if output.status.success() {
            String::from_utf8(output.stdout).ok()
        } else {
            None
        }
    }

    #[cfg(target_os = "linux")]
    {
        // Try xclip first, then xsel
        use std::process::Command;

        let output = Command::new("xclip")
            .args(["-selection", "clipboard", "-o"])
            .output()
            .ok();

        if let Some(o) = output {
            if o.status.success() {
                return String::from_utf8(o.stdout).ok();
            }
        }

        // Fallback to xsel
        let output = Command::new("xsel")
            .args(["--clipboard", "--output"])
            .output()
            .ok()?;

        if output.status.success() {
            String::from_utf8(output.stdout).ok()
        } else {
            None
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_clipboard() {
        // This test just verifies the function doesn't panic
        // Actual clipboard content depends on system state
        let _ = read_clipboard_content();
    }
}
