use crate::agent::ToolDefinition;
use serde_json::json;

pub fn definition() -> ToolDefinition {
    ToolDefinition {
        name: "glob".to_string(),
        description: "Find files matching a glob pattern. Returns a list of matching file paths. Use patterns like '**/*.rs' for recursive search or 'src/*.ts' for specific directories.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "The glob pattern to match (e.g., '**/*.rs', 'src/**/*.ts')"
                },
                "path": {
                    "type": "string",
                    "description": "Base directory to search in (optional, defaults to project root)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of results to return (default: 100)"
                }
            },
            "required": ["pattern"]
        }),
    }
}

pub fn execute(
    input: &serde_json::Value,
    project_path: Option<&str>,
) -> Result<String, String> {
    let pattern = input
        .get("pattern")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'pattern' parameter")?;

    let base_path = input
        .get("path")
        .and_then(|v| v.as_str())
        .or(project_path)
        .unwrap_or(".");

    let limit = input
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(100) as usize;

    // Construct full pattern
    let full_pattern = if pattern.starts_with('/') || pattern.starts_with('.') {
        pattern.to_string()
    } else {
        format!("{}/{}", base_path, pattern)
    };

    // Use glob crate
    let entries = glob::glob(&full_pattern)
        .map_err(|e| format!("Invalid glob pattern: {}", e))?;

    let mut results: Vec<String> = Vec::new();
    let mut total_count = 0;

    for entry in entries {
        match entry {
            Ok(path) => {
                total_count += 1;
                if results.len() < limit {
                    // Make path relative to project if possible
                    let display_path = if let Some(project) = project_path {
                        path.strip_prefix(project)
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_else(|_| path.to_string_lossy().to_string())
                    } else {
                        path.to_string_lossy().to_string()
                    };
                    results.push(display_path);
                }
            }
            Err(_) => {
                // Skip files we can't access
                continue;
            }
        }
    }

    if results.is_empty() {
        return Ok(format!("No files found matching pattern: {}", pattern));
    }

    let mut output = results.join("\n");

    if total_count > limit {
        output.push_str(&format!(
            "\n\n[Showing {} of {} matches]",
            limit, total_count
        ));
    }

    Ok(output)
}
