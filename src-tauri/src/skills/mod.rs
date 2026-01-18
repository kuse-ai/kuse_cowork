use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,
}

/// Get the skills directory path (app data directory only)
pub fn get_skills_directory() -> PathBuf {
    let app_data = dirs::data_dir()
        .expect("Could not determine app data directory");

    app_data.join("kuse-cowork").join("skills")
}

/// Ensure skills directory exists and install default skills if needed
pub fn ensure_skills_directory() -> PathBuf {
    let skills_dir = get_skills_directory();

    // Create directory if it doesn't exist
    if let Err(e) = fs::create_dir_all(&skills_dir) {
        eprintln!("Failed to create skills directory: {}", e);
    }

    // Install default skills if directory is empty
    install_default_skills_if_needed(&skills_dir);

    skills_dir
}

/// Install default skills if the skills directory is empty
fn install_default_skills_if_needed(skills_dir: &Path) {
    // Check if directory is empty
    if let Ok(entries) = fs::read_dir(skills_dir) {
        if entries.count() > 0 {
            return; // Already has skills
        }
    }

    println!("Installing default skills to {}", skills_dir.display());

    // Install 4 core skills
    install_skill(skills_dir, "pdf", include_str!("../../bundled-skills/pdf.skill.md"));
    install_skill(skills_dir, "docx", include_str!("../../bundled-skills/docx.skill.md"));
    install_skill(skills_dir, "xlsx", include_str!("../../bundled-skills/xlsx.skill.md"));
    install_skill(skills_dir, "pptx", include_str!("../../bundled-skills/pptx.skill.md"));

    println!("Default skills installed successfully!");
}

/// Install a single skill from bundled content
fn install_skill(skills_dir: &Path, skill_name: &str, skill_content: &str) {
    let skill_dir = skills_dir.join(skill_name);

    if let Err(e) = fs::create_dir_all(&skill_dir) {
        eprintln!("Failed to create skill directory {}: {}", skill_name, e);
        return;
    }

    let skill_file = skill_dir.join("SKILL.md");
    if let Err(e) = fs::write(&skill_file, skill_content) {
        eprintln!("Failed to write skill file {}: {}", skill_name, e);
    }
}

/// Get the skills directory path as a string for use in prompts
pub fn get_skills_directory_path() -> String {
    get_skills_directory().to_string_lossy().to_string()
}

/// Parse YAML frontmatter from SKILL.md file
fn parse_skill_metadata(content: &str) -> Option<SkillMetadata> {
    if !content.starts_with("---") {
        return None;
    }

    let end_pos = content[3..].find("---")?;
    let yaml_content = &content[3..end_pos + 3];

    // Simple YAML parsing for name and description
    let mut name = None;
    let mut description = None;

    for line in yaml_content.lines() {
        let line = line.trim();
        if let Some(stripped) = line.strip_prefix("name:") {
            name = Some(stripped.trim().to_string());
        } else if let Some(stripped) = line.strip_prefix("description:") {
            description = Some(stripped.trim().to_string());
        }
    }

    Some(SkillMetadata {
        name: name?,
        description: description?,
    })
}

/// Get available skills by scanning the skills directory
pub fn get_available_skills() -> Vec<SkillMetadata> {
    let skills_dir = ensure_skills_directory();

    let mut skills = Vec::new();

    if let Ok(entries) = fs::read_dir(&skills_dir) {
        for entry in entries.flatten() {
            let skill_dir = entry.path();
            if skill_dir.is_dir() {
                let skill_file = skill_dir.join("SKILL.md");
                if skill_file.exists() {
                    if let Ok(content) = fs::read_to_string(&skill_file) {
                        if let Some(metadata) = parse_skill_metadata(&content) {
                            skills.push(metadata);
                        }
                    }
                }
            }
        }
    }

    // Sort skills by name for consistent ordering
    skills.sort_by(|a, b| a.name.cmp(&b.name));
    skills
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_parse_skill_metadata() {
        let content = r#"---
name: pdf
description: Comprehensive PDF manipulation toolkit
license: Proprietary
---

# PDF Processing Guide
Some content here...
"#;

        let metadata = parse_skill_metadata(content).unwrap();
        assert_eq!(metadata.name, "pdf");
        assert_eq!(metadata.description, "Comprehensive PDF manipulation toolkit");
    }

    #[test]
    fn test_skills_directory_creation() {
        // This will create the skills directory and install default skills
        let skills_dir = ensure_skills_directory();

        // Verify directory exists
        assert!(skills_dir.exists());
        assert!(skills_dir.is_dir());

        // Verify basic skills are installed
        assert!(skills_dir.join("pdf").join("SKILL.md").exists());
        assert!(skills_dir.join("docx").join("SKILL.md").exists());
        assert!(skills_dir.join("xlsx").join("SKILL.md").exists());
        assert!(skills_dir.join("pptx").join("SKILL.md").exists());
    }

    #[test]
    fn test_get_available_skills() {
        let skills = get_available_skills();

        // Should have 4 default skills
        assert_eq!(skills.len(), 4);

        // Check that all expected skills are present
        let skill_names: Vec<&str> = skills.iter().map(|s| s.name.as_str()).collect();
        assert!(skill_names.contains(&"pdf"));
        assert!(skill_names.contains(&"docx"));
        assert!(skill_names.contains(&"xlsx"));
        assert!(skill_names.contains(&"pptx"));
    }
}