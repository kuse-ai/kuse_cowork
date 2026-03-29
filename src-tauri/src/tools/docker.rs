use crate::agent::{ToolDefinition, ToolResult, ToolUse};
use bollard::container::{
    Config, CreateContainerOptions, RemoveContainerOptions, StartContainerOptions,
    WaitContainerOptions,
};
use bollard::image::CreateImageOptions;
use bollard::models::HostConfig;
use bollard::Docker;
use futures::StreamExt;
use serde_json::json;

/// Get Docker tool definitions
pub fn get_docker_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "docker_run".to_string(),
            description: "Run a command in a Docker container with optional volume mounts. The container is automatically removed after execution.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "image": {
                        "type": "string",
                        "description": "Docker image to use (default: 'python:3.11-alpine', also 'ubuntu:latest', 'node:20', 'rust:alpine')"
                    },
                    "command": {
                        "type": "string",
                        "description": "Command to run inside the container"
                    },
                    "workdir": {
                        "type": "string",
                        "description": "Working directory inside the container (default: /workspace)"
                    },
                    "mounts": {
                        "type": "array",
                        "items": {
                            "type": "string"
                        },
                        "description": "Volume mounts in format 'host_path:container_path' (e.g., '/Users/you/project:/workspace')"
                    }
                },
                "required": ["image", "command"]
            }),
        },
        ToolDefinition {
            name: "docker_list".to_string(),
            description: "List running Docker containers".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "all": {
                        "type": "boolean",
                        "description": "Show all containers (including stopped)"
                    }
                }
            }),
        },
        ToolDefinition {
            name: "docker_images".to_string(),
            description: "List available Docker images".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
    ]
}

/// Execute a Docker tool (sync wrapper for non-async contexts)
pub fn execute_docker_tool(tool_use: &ToolUse, project_path: &Option<String>) -> ToolResult {
    // Use a separate thread to avoid blocking the async runtime
    std::thread::scope(|s| {
        s.spawn(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                execute_docker_tool_inner(tool_use, project_path).await
            })
        }).join().unwrap()
    })
}

async fn execute_docker_tool_inner(tool_use: &ToolUse, project_path: &Option<String>) -> ToolResult {
    let docker = match Docker::connect_with_local_defaults() {
        Ok(d) => d,
        Err(e) => {
            return ToolResult::error(
                tool_use.id.clone(),
                format!("Failed to connect to Docker: {}. Make sure Docker Desktop is running.", e),
            );
        }
    };

    match tool_use.name.as_str() {
        "docker_run" => docker_run(&docker, tool_use, project_path).await,
        "docker_list" => docker_list(&docker, tool_use).await,
        "docker_images" => docker_images(&docker, tool_use).await,
        _ => ToolResult::error(tool_use.id.clone(), format!("Unknown docker tool: {}", tool_use.name)),
    }
}

async fn docker_run(docker: &Docker, tool_use: &ToolUse, project_path: &Option<String>) -> ToolResult {
    let image = tool_use.input.get("image")
        .and_then(|v| v.as_str())
        .unwrap_or("python:3.11-alpine");

    let command = match tool_use.input.get("command").and_then(|v| v.as_str()) {
        Some(c) => c,
        None => return ToolResult::error(tool_use.id.clone(), "Missing 'command' parameter".to_string()),
    };

    let workdir = tool_use.input.get("workdir")
        .and_then(|v| v.as_str())
        .unwrap_or("/workspace");

    // Build volume mounts
    let mut binds: Vec<String> = Vec::new();

    // Add project path mount if available
    if let Some(path) = project_path {
        binds.push(format!("{}:/workspace", path));
    }

    // Auto-mount skills directory
    let skills_dir = crate::skills::ensure_skills_directory();
    binds.push(format!("{}:/skills:ro", skills_dir.display()));

    // Add custom mounts
    if let Some(mounts) = tool_use.input.get("mounts").and_then(|v| v.as_array()) {
        for mount in mounts {
            if let Some(m) = mount.as_str() {
                binds.push(m.to_string());
            }
        }
    }

    // Try to pull image if not exists
    let _ = pull_image_if_needed(docker, image).await;

    // Create container
    let container_name = format!("kuse_cowork-{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap());

    let host_config = HostConfig {
        binds: if binds.is_empty() { None } else { Some(binds) },
        auto_remove: Some(true),
        ..Default::default()
    };

    let config = Config {
        image: Some(image.to_string()),
        cmd: Some(vec!["sh".to_string(), "-c".to_string(), command.to_string()]),
        working_dir: Some(workdir.to_string()),
        host_config: Some(host_config),
        tty: Some(false),
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        ..Default::default()
    };

    let options = CreateContainerOptions {
        name: container_name.as_str(),
        platform: None,
    };

    let container = match docker.create_container(Some(options), config).await {
        Ok(c) => c,
        Err(e) => return ToolResult::error(tool_use.id.clone(), format!("Failed to create container: {}", e)),
    };

    // Start container
    if let Err(e) = docker.start_container(&container.id, None::<StartContainerOptions<String>>).await {
        return ToolResult::error(tool_use.id.clone(), format!("Failed to start container: {}", e));
    }

    // Wait for container to finish and collect logs
    let mut output = String::new();

    // Collect logs
    let log_options = bollard::container::LogsOptions::<String> {
        follow: true,
        stdout: true,
        stderr: true,
        ..Default::default()
    };

    let mut log_stream = docker.logs(&container.id, Some(log_options));

    while let Some(log_result) = log_stream.next().await {
        match log_result {
            Ok(log) => {
                output.push_str(&log.to_string());
            }
            Err(e) => {
                output.push_str(&format!("\n[Log error: {}]", e));
                break;
            }
        }
    }

    // Wait for container to exit
    let mut wait_stream = docker.wait_container(&container.id, None::<WaitContainerOptions<String>>);
    let mut exit_code = 0i64;

    while let Some(wait_result) = wait_stream.next().await {
        match wait_result {
            Ok(wait_response) => {
                exit_code = wait_response.status_code;
            }
            Err(_) => break,
        }
    }

    // Clean up container (in case auto_remove didn't work)
    let _ = docker.remove_container(
        &container.id,
        Some(RemoveContainerOptions {
            force: true,
            ..Default::default()
        }),
    ).await;

    if output.is_empty() {
        output = "(no output)".to_string();
    }

    let result = format!("Exit code: {}\n\n{}", exit_code, output.trim());

    if exit_code != 0 {
        ToolResult::error(tool_use.id.clone(), result)
    } else {
        ToolResult::success(tool_use.id.clone(), result)
    }
}

async fn docker_list(docker: &Docker, tool_use: &ToolUse) -> ToolResult {
    let all = tool_use.input.get("all")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let options = bollard::container::ListContainersOptions::<String> {
        all,
        ..Default::default()
    };

    match docker.list_containers(Some(options)).await {
        Ok(containers) => {
            if containers.is_empty() {
                return ToolResult::success(tool_use.id.clone(), "No containers found".to_string());
            }

            let mut output = String::new();
            output.push_str("CONTAINER ID\tIMAGE\tSTATUS\tNAMES\n");

            for c in containers {
                let id = c.id.as_deref().unwrap_or("-").chars().take(12).collect::<String>();
                let image = c.image.as_deref().unwrap_or("-");
                let status = c.status.as_deref().unwrap_or("-");
                let names = c.names.map(|n| n.join(", ")).unwrap_or_else(|| "-".to_string());

                output.push_str(&format!("{}\t{}\t{}\t{}\n", id, image, status, names));
            }

            ToolResult::success(tool_use.id.clone(), output)
        }
        Err(e) => ToolResult::error(tool_use.id.clone(), format!("Failed to list containers: {}", e)),
    }
}

async fn docker_images(docker: &Docker, tool_use: &ToolUse) -> ToolResult {
    match docker.list_images::<String>(None).await {
        Ok(images) => {
            if images.is_empty() {
                return ToolResult::success(tool_use.id.clone(), "No images found".to_string());
            }

            let mut output = String::new();
            output.push_str("REPOSITORY:TAG\tSIZE\n");

            for img in images {
                let tags = img.repo_tags.join(", ");
                let size_mb = img.size / 1_000_000;

                if !tags.is_empty() && tags != "<none>:<none>" {
                    output.push_str(&format!("{}\t{}MB\n", tags, size_mb));
                }
            }

            ToolResult::success(tool_use.id.clone(), output)
        }
        Err(e) => ToolResult::error(tool_use.id.clone(), format!("Failed to list images: {}", e)),
    }
}

async fn pull_image_if_needed(docker: &Docker, image: &str) -> Result<(), String> {
    // Check if image exists
    if docker.inspect_image(image).await.is_ok() {
        return Ok(());
    }

    // Pull image with timeout (120 seconds)
    let pull_future = async {
        let options = CreateImageOptions {
            from_image: image,
            ..Default::default()
        };

        let mut stream = docker.create_image(Some(options), None, None);

        while let Some(result) = stream.next().await {
            if let Err(e) = result {
                return Err(format!("Failed to pull image: {}", e));
            }
        }

        Ok(())
    };

    match tokio::time::timeout(std::time::Duration::from_secs(120), pull_future).await {
        Ok(result) => result,
        Err(_) => Err(format!("Timeout pulling image '{}'. Please run 'docker pull {}' manually first.", image, image)),
    }
}

