use crate::agent::ToolDefinition;
use serde_json::json;
use std::fs;
use std::path::Path;

pub fn definition() -> ToolDefinition {
    ToolDefinition {
        name: "grep".to_string(),
        description: "Search for a pattern in files. Returns matching lines with file paths and line numbers. Supports regex patterns.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "The regex pattern to search for"
                },
                "path": {
                    "type": "string",
                    "description": "File or directory to search in"
                },
                "glob": {
                    "type": "string",
                    "description": "File pattern to filter (e.g., '*.rs', '*.ts')"
                },
                "case_insensitive": {
                    "type": "boolean",
                    "description": "Ignore case when matching (default: false)"
                },
                "context": {
                    "type": "integer",
                    "description": "Number of context lines before and after match (default: 0)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of matches to return (default: 50)"
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

    let search_path = input
        .get("path")
        .and_then(|v| v.as_str())
        .or(project_path)
        .unwrap_or(".");

    let file_glob = input
        .get("glob")
        .and_then(|v| v.as_str());

    let case_insensitive = input
        .get("case_insensitive")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let context = input
        .get("context")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;

    let limit = input
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(50) as usize;

    // Build regex
    let regex = if case_insensitive {
        regex::RegexBuilder::new(pattern)
            .case_insensitive(true)
            .build()
    } else {
        regex::Regex::new(pattern)
    }.map_err(|e| format!("Invalid regex pattern: {}", e))?;

    let path = Path::new(search_path);
    let mut results: Vec<String> = Vec::new();
    let mut match_count = 0;

    if path.is_file() {
        search_file(path, &regex, context, limit, &mut results, &mut match_count, project_path)?;
    } else if path.is_dir() {
        search_directory(path, &regex, file_glob, context, limit, &mut results, &mut match_count, project_path)?;
    } else {
        return Err(format!("Path not found: {}", search_path));
    }

    if results.is_empty() {
        return Ok(format!("No matches found for pattern: {}", pattern));
    }

    let mut output = results.join("\n");

    if match_count > limit {
        output.push_str(&format!(
            "\n\n[Showing {} of {} matches]",
            limit, match_count
        ));
    }

    Ok(output)
}

fn search_file(
    path: &Path,
    regex: &regex::Regex,
    context: usize,
    limit: usize,
    results: &mut Vec<String>,
    match_count: &mut usize,
    project_path: Option<&str>,
) -> Result<(), String> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Ok(()), // Skip binary or unreadable files
    };

    let lines: Vec<&str> = content.lines().collect();
    let display_path = if let Some(project) = project_path {
        path.strip_prefix(project)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| path.to_string_lossy().to_string())
    } else {
        path.to_string_lossy().to_string()
    };

    for (i, line) in lines.iter().enumerate() {
        if regex.is_match(line) {
            *match_count += 1;

            if results.len() < limit {
                // Add context lines before
                let start = i.saturating_sub(context);
                for (j, line_content) in lines.iter().enumerate().take(i).skip(start) {
                    results.push(format!("{}:{}: {}", display_path, j + 1, line_content));
                }

                // Add matching line
                results.push(format!("{}:{}> {}", display_path, i + 1, line));

                // Add context lines after
                let end = (i + context + 1).min(lines.len());
                for (j, line_content) in lines.iter().enumerate().take(end).skip(i + 1) {
                    results.push(format!("{}:{}: {}", display_path, j + 1, line_content));
                }

                if context > 0 {
                    results.push("---".to_string());
                }
            }
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn search_directory(
    path: &Path,
    regex: &regex::Regex,
    file_glob: Option<&str>,
    context: usize,
    limit: usize,
    results: &mut Vec<String>,
    match_count: &mut usize,
    project_path: Option<&str>,
) -> Result<(), String> {
    let glob_pattern = file_glob.unwrap_or("**/*");
    let full_pattern = format!("{}/{}", path.to_string_lossy(), glob_pattern);

    let entries = glob::glob(&full_pattern)
        .map_err(|e| format!("Invalid glob pattern: {}", e))?;

    for entry in entries {
        if results.len() >= limit {
            break;
        }

        if let Ok(file_path) = entry {
            if file_path.is_file() {
                search_file(&file_path, regex, context, limit, results, match_count, project_path)?;
            }
        }
    }

    Ok(())
}
