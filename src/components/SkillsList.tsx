import { Component, For, Show, createSignal, onMount } from "solid-js";
import { getSkillsList, SkillMetadata } from "../lib/tauri-api";
import "./SkillsList.css";

const SkillsList: Component = () => {
  const [skills, setSkills] = createSignal<SkillMetadata[]>([]);
  const [loading, setLoading] = createSignal(true);
  const [selectedSkill, setSelectedSkill] = createSignal<string | null>(null);

  onMount(async () => {
    try {
      const skillsList = await getSkillsList();
      setSkills(skillsList);
    } catch (error) {
      console.error("Failed to load skills:", error);
    } finally {
      setLoading(false);
    }
  });

  return (
    <div class="skills-list">
      <div class="skills-header">
        <h2>Available Skills</h2>
        <p>Skills enhance Claude's ability to process specific file types and perform specialized tasks.</p>
      </div>

      <Show
        when={!loading()}
        fallback={
          <div class="skills-loading">
            <p>Loading skills...</p>
          </div>
        }
      >
        <Show
          when={skills().length > 0}
          fallback={
            <div class="skills-empty">
              <h3>No Skills Found</h3>
              <p>Skills will be automatically detected from the app data directory</p>
              <div class="skills-help">
                <h4>How skills work:</h4>
                <ol>
                  <li>Skills are stored in the app data directory for better compatibility</li>
                  <li>If you used a previous version, you may need to manually copy skills from ~/.kuse_cowork/skills/</li>
                  <li>Each skill is a folder with a SKILL.md file containing instructions</li>
                  <li>Skills include PDF, DOCX, XLSX, and PPTX processing capabilities</li>
                  <li>Skills are automatically mounted in Docker containers at /skills</li>
                </ol>
              </div>
            </div>
          }
        >
          <div class="skills-grid">
            <For each={skills()}>
              {(skill) => (
                <div class="skill-card">
                  <div class="skill-header">
                    <h3 class="skill-name">{skill.name}</h3>
                    <div class="skill-badge">Active</div>
                  </div>
                  <p class="skill-description">{skill.description}</p>
                  <div class="skill-actions">
                    <button
                      class="skill-button"
                      onClick={() => setSelectedSkill(skill.name)}
                    >
                      View Details
                    </button>
                  </div>
                </div>
              )}
            </For>
          </div>
        </Show>
      </Show>

      <Show when={selectedSkill()}>
        <div class="skill-modal-overlay" onClick={() => setSelectedSkill(null)}>
          <div class="skill-modal" onClick={(e) => e.stopPropagation()}>
            <div class="skill-modal-header">
              <h3>Skill: {selectedSkill()}</h3>
              <button
                class="skill-modal-close"
                onClick={() => setSelectedSkill(null)}
              >
                ×
              </button>
            </div>
            <div class="skill-modal-content">
              <p><strong>Location:</strong> App data directory/skills/{selectedSkill()}/</p>
              <p><strong>Files:</strong></p>
              <ul>
                <li>SKILL.md - Main skill documentation</li>
                <li>scripts/ - Executable Python scripts</li>
                <li>*.md - Additional reference documentation</li>
              </ul>
              <p><strong>Usage:</strong> Skills are automatically available to Claude when processing relevant file types.</p>
              <p><strong>Platform paths:</strong></p>
              <ul>
                <li>macOS: ~/Library/Application Support/kuse_cowork/skills/</li>
                <li>Windows: %APPDATA%\kuse_cowork\skills\</li>
                <li>Linux: ~/.local/share/kuse_cowork/skills/</li>
              </ul>
            </div>
          </div>
        </div>
      </Show>
    </div>
  );
};

export default SkillsList;