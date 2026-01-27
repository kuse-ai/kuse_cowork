import { Component, For, Show } from "solid-js";
import { Task } from "../lib/tauri-api";
import "./TaskSidebar.css";

interface TaskSidebarProps {
  tasks: Task[];
  activeTaskId: string | null;
  onSelectTask: (task: Task) => void;
  onDeleteTask: (taskId: string) => void;
  onSettingsClick: () => void;
  onSkillsClick: () => void;
  onMCPClick: () => void;
  onDataClick: () => void;
  onTraceClick: () => void;
  onBrowserClick: () => void;
  showDataPanels: boolean;
  showTracePanel: boolean;
  showBrowserPanel: boolean;
}

const TaskSidebar: Component<TaskSidebarProps> = (props) => {
  const getStatusIcon = (status: string) => {
    switch (status) {
      case "completed":
        return "✓";
      case "running":
        return "●";
      case "failed":
        return "✗";
      default:
        return "○";
    }
  };

  const formatDate = (timestamp: number) => {
    const date = new Date(timestamp);
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const days = Math.floor(diff / (1000 * 60 * 60 * 24));

    if (days === 0) {
      return "Today";
    } else if (days === 1) {
      return "Yesterday";
    } else if (days < 7) {
      return `${days} days ago`;
    } else {
      return date.toLocaleDateString();
    }
  };

  return (
    <aside class="task-sidebar">
      <div class="sidebar-header">
        <div class="logo-container">
          <img src="/logo.png" alt="Kuse Cowork" class="logo-image" />
          <h1 class="app-title">Kuse Cowork</h1>
        </div>
      </div>

      <div class="task-list">
        <div class="task-list-header">Tasks</div>
        <Show
          when={props.tasks.length > 0}
          fallback={
            <div class="no-tasks">
              <p>No tasks yet</p>
              <p class="hint">Create a new task to get started</p>
            </div>
          }
        >
          <For each={props.tasks}>
            {(task) => (
              <div
                class={`task-item ${props.activeTaskId === task.id ? "active" : ""} ${task.status}`}
                onClick={() => props.onSelectTask(task)}
              >
                <span class={`task-icon ${task.status}`}>{getStatusIcon(task.status)}</span>
                <div class="task-info">
                  <div class="task-item-title">{task.title}</div>
                  <div class="task-date">{formatDate(task.updated_at)}</div>
                </div>
                <button
                  class="task-delete-btn"
                  onClick={(e) => {
                    e.stopPropagation();
                    props.onDeleteTask(task.id);
                  }}
                  title="Delete task"
                >
                  ×
                </button>
              </div>
            )}
          </For>
        </Show>
      </div>

      <div class="sidebar-footer">
        <button
          class={`footer-btn primary-btn ${props.showTracePanel ? "active" : ""}`}
          onClick={props.onTraceClick}
          title="Activity Trace"
        >
          Trace
        </button>
        <button
          class={`footer-btn primary-btn ${props.showBrowserPanel ? "active" : ""}`}
          onClick={props.onBrowserClick}
          title="Embedded Browser"
        >
          Browse
        </button>
        <button
          class={`footer-btn primary-btn ${props.showDataPanels ? "active" : ""}`}
          onClick={props.onDataClick}
          title="Data Panels"
        >
          Data
        </button>
        <button
          class="footer-btn primary-btn"
          onClick={props.onSkillsClick}
        >
          Skills
        </button>
        <button
          class="footer-btn primary-btn"
          onClick={props.onMCPClick}
        >
          MCPs
        </button>
        <button class="footer-btn primary-btn" onClick={props.onSettingsClick}>
          Settings
        </button>
      </div>
    </aside>
  );
};

export default TaskSidebar;
