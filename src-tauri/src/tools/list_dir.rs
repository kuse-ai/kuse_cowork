use crate::agent::ToolDefinition;
use serde_json::json;
use std::fs;
use std::path::Path;

pub fn definition() -> ToolDefinition {
    ToolDefinition {
        name: "list_dir".to_string(),
        description: "List the contents of a directory. Shows files and subdirectories with their types and sizes.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The directory path to list (defaults to project root)"
                },
                "recursive": {
                    "type": "boolean",
                    "description": "If true, list contents recursively (default: false)"
                },
                "max_depth": {
                    "type": "integer",
                    "description": "Maximum depth for recursive listing (default: 3)"
                }
            },
            "required": []
        }),
    }
}

pub fn execute(
    input: &serde_json::Value,
    project_path: Option<&str>,
) -> Result<String, String> {
    let path_str = input
        .get("path")
        .and_then(|v| v.as_str())
        .or(project_path)
        .unwrap_or(".");

    let recursive = input
        .get("recursive")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let max_depth = input
        .get("max_depth")
        .and_then(|v| v.as_u64())
        .unwrap_or(3) as usize;

    let path = resolve_path(path_str, project_path)?;

    if !path.exists() {
        return Err(format!("Directory not found: {}", path_str));
    }

    if !path.is_dir() {
        return Err(format!("Path is not a directory: {}", path_str));
    }

    let mut results = Vec::new();

    if recursive {
        list_recursive(&path, &path, 0, max_depth, &mut results)?;
    } else {
        list_single(&path, &mut results)?;
    }

    if results.is_empty() {
        return Ok("Directory is empty".to_string());
    }

    Ok(results.join("\n"))
}

fn list_single(path: &Path, results: &mut Vec<String>) -> Result<(), String> {
    let entries = fs::read_dir(path)
        .map_err(|e| format!("Failed to read directory: {}", e))?;

    let mut items: Vec<(String, bool, u64)> = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let metadata = entry.metadata().ok();
        let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
        let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
        let name = entry.file_name().to_string_lossy().to_string();

        items.push((name, is_dir, size));
    }

    // Sort: directories first, then by name
    items.sort_by(|a, b| {
        match (a.1, b.1) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.0.cmp(&b.0),
        }
    });

    for (name, is_dir, size) in items {
        if is_dir {
            results.push(format!("📁 {}/", name));
        } else {
            results.push(format!("📄 {} ({})", name, format_size(size)));
        }
    }

    Ok(())
}

fn list_recursive(
    _base: &Path,
    path: &Path,
    depth: usize,
    max_depth: usize,
    results: &mut Vec<String>,
) -> Result<(), String> {
    if depth > max_depth {
        return Ok(());
    }

    let entries = match fs::read_dir(path) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };

    let mut items: Vec<(String, bool, std::path::PathBuf)> = Vec::new();

    for entry in entries.flatten() {
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        let name = entry.file_name().to_string_lossy().to_string();

        // Skip hidden files and common ignore patterns
        if name.starts_with('.') || name == "node_modules" || name == "target" || name == "__pycache__" {
            continue;
        }

        items.push((name, is_dir, entry.path()));
    }

    items.sort_by(|a, b| {
        match (a.1, b.1) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.0.cmp(&b.0),
        }
    });

    let indent = "  ".repeat(depth);

    for (name, is_dir, full_path) in items {
        if is_dir {
            results.push(format!("{}📁 {}/", indent, name));
            list_recursive(_base, &full_path, depth + 1, max_depth, results)?;
        } else {
            results.push(format!("{}📄 {}", indent, name));
        }
    }

    Ok(())
}

fn format_size(size: u64) -> String {
    if size < 1024 {
        format!("{} B", size)
    } else if size < 1024 * 1024 {
        format!("{:.1} KB", size as f64 / 1024.0)
    } else if size < 1024 * 1024 * 1024 {
        format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", size as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

fn resolve_path(path_str: &str, project_path: Option<&str>) -> Result<std::path::PathBuf, String> {
    let path = Path::new(path_str);

    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else if let Some(project) = project_path {
        Ok(Path::new(project).join(path))
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .map_err(|e| format!("Failed to get current directory: {}", e))
    }
}
