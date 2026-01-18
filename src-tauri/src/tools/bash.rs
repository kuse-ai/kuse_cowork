use crate::agent::ToolDefinition;
use serde_json::json;
use std::process::{Command, Stdio};
use std::time::Duration;

pub fn definition() -> ToolDefinition {
    ToolDefinition {
        name: "bash".to_string(),
        description: "Execute a shell command. Use for running builds, tests, git commands, etc. Commands run in a sandboxed environment with timeouts.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                },
                "cwd": {
                    "type": "string",
                    "description": "Working directory for the command (optional)"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in seconds (default: 60, max: 300)"
                }
            },
            "required": ["command"]
        }),
    }
}

// Dangerous commands that should be blocked
const BLOCKED_PATTERNS: &[&str] = &[
    "rm -rf /",
    "rm -rf /*",
    ":(){ :|:& };:",  // Fork bomb
    "> /dev/sda",
    "mkfs.",
    "dd if=",
    "wget | sh",
    "curl | sh",
    "wget | bash",
    "curl | bash",
];

pub fn execute(
    input: &serde_json::Value,
    project_path: Option<&str>,
) -> Result<String, String> {
    let command = input
        .get("command")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'command' parameter")?;

    let cwd = input
        .get("cwd")
        .and_then(|v| v.as_str())
        .or(project_path);

    let timeout_secs = input
        .get("timeout")
        .and_then(|v| v.as_u64())
        .unwrap_or(60)
        .min(300);

    // Security check
    for pattern in BLOCKED_PATTERNS {
        if command.contains(pattern) {
            return Err(format!(
                "Command blocked for safety: contains dangerous pattern '{}'",
                pattern
            ));
        }
    }

    // Build command
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(command);

    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    // Execute with timeout
    let child = cmd.spawn()
        .map_err(|e| format!("Failed to spawn command: {}", e))?;

    let output = wait_with_timeout(child, Duration::from_secs(timeout_secs))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let exit_code = output.status.code().unwrap_or(-1);

    // Format output
    let mut result = String::new();

    if !stdout.is_empty() {
        result.push_str(&stdout);
    }

    if !stderr.is_empty() {
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str("[stderr]\n");
        result.push_str(&stderr);
    }

    if exit_code != 0 {
        result.push_str(&format!("\n[exit code: {}]", exit_code));
    }

    // Truncate if too long
    if result.len() > 50000 {
        result = format!(
            "{}...\n\n[Output truncated. Total length: {} chars]",
            &result[..50000],
            result.len()
        );
    }

    if result.is_empty() {
        result = "[Command completed with no output]".to_string();
    }

    Ok(result)
}

fn wait_with_timeout(
    child: std::process::Child,
    timeout: Duration,
) -> Result<std::process::Output, String> {
    use std::thread;
    use std::sync::mpsc;

    let (tx, rx) = mpsc::channel();

    let handle = thread::spawn(move || {
        let result = child.wait_with_output();
        let _ = tx.send(result);
    });

    match rx.recv_timeout(timeout) {
        Ok(result) => {
            let _ = handle.join();
            result.map_err(|e| format!("Command failed: {}", e))
        }
        Err(_) => {
            // Timeout - try to kill the process
            Err(format!(
                "Command timed out after {} seconds",
                timeout.as_secs()
            ))
        }
    }
}
