use crate::agent::ToolDefinition;
use serde_json::json;
use std::fs;
use std::path::Path;

pub fn definition() -> ToolDefinition {
    ToolDefinition {
        name: "read_file".to_string(),
        description: "Read the contents of a file at the specified path. Use this to understand existing code before making changes.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The path to the file to read (relative to project root or absolute)"
                },
                "offset": {
                    "type": "integer",
                    "description": "Line number to start reading from (1-indexed, optional)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of lines to read (optional)"
                }
            },
            "required": ["path"]
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
        .ok_or("Missing 'path' parameter")?;

    let offset = input
        .get("offset")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize);

    let limit = input
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize);

    // Resolve path
    let path = resolve_path(path_str, project_path)?;

    // Check if file exists
    if !path.exists() {
        return Err(format!("File not found: {}", path_str));
    }

    if !path.is_file() {
        return Err(format!("Path is not a file: {}", path_str));
    }

    // Read file
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    // Apply offset and limit
    let lines: Vec<&str> = content.lines().collect();
    let start = offset.unwrap_or(1).saturating_sub(1);
    let end = limit
        .map(|l| (start + l).min(lines.len()))
        .unwrap_or(lines.len());

    if start >= lines.len() {
        return Ok(String::new());
    }

    // Format with line numbers
    let result: Vec<String> = lines[start..end]
        .iter()
        .enumerate()
        .map(|(i, line)| format!("{:>6}\t{}", start + i + 1, line))
        .collect();

    Ok(result.join("\n"))
}

fn resolve_path(path_str: &str, project_path: Option<&str>) -> Result<std::path::PathBuf, String> {
    // Handle home directory expansion
    let expanded_path = if let Some(stripped) = path_str.strip_prefix("~/") {
        match dirs::home_dir() {
            Some(home) => home.join(stripped),
            None => return Err("Could not determine home directory".to_string()),
        }
    } else if path_str == "~" {
        match dirs::home_dir() {
            Some(home) => home,
            None => return Err("Could not determine home directory".to_string()),
        }
    } else {
        std::path::PathBuf::from(path_str)
    };

    let path = expanded_path.as_path();

    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else if let Some(project) = project_path {
        Ok(Path::new(project).join(path))
    } else {
        // Use current directory as fallback
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .map_err(|e| format!("Failed to get current directory: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_path_home_expansion() {
        // Test home directory expansion
        let result = resolve_path("~/.kuse-cowork/test", None);
        assert!(result.is_ok());

        let path = result.unwrap();
        assert!(path.is_absolute());
        assert!(path.to_string_lossy().contains(".kuse-cowork/test"));
        assert!(!path.to_string_lossy().starts_with("~"));
    }

    #[test]
    fn test_resolve_path_home_only() {
        let result = resolve_path("~", None);
        assert!(result.is_ok());

        let path = result.unwrap();
        assert!(path.is_absolute());
    }

    #[test]
    fn test_resolve_path_absolute() {
        let result = resolve_path("/tmp/test", None);
        assert!(result.is_ok());

        let path = result.unwrap();
        assert_eq!(path.to_string_lossy(), "/tmp/test");
    }
}
