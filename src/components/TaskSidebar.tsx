import { Component, For, Show, createEffect, onMount } from "solid-js";
import { Task, Document } from "../lib/tauri-api";
import { useDocs } from "../stores/docs";
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
  onCaptureClick: () => void;
  onBrowserClick: () => void;
  onActivityClick: () => void;
  onDocClick: () => void;
  onSelectDoc: (doc: Document) => void;
  showDataPanels: boolean;
  showCapturePanel: boolean;
  showBrowserPanel: boolean;
  showActivityPanel: boolean;
  showDocEditor: boolean;
  activeDocId: string | null;
}

const TaskSidebar: Component<TaskSidebarProps> = (props) => {
  const { documents, loadDocuments, createDocument, deleteDocument } = useDocs();

  onMount(() => {
    loadDocuments();
  });

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

  const handleNewDocument = async () => {
    const doc = await createDocument({ title: "Untitled Document" });
    if (doc) {
      props.onSelectDoc(doc);
    }
  };

  const handleDeleteDoc = async (e: Event, docId: string) => {
    e.stopPropagation();
    await deleteDocument(docId);
  };

  const SidebarIcons = {
    Docs: () => (
      <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
        <polyline points="14 2 14 8 20 8" />
        <line x1="16" y1="13" x2="8" y2="13" />
        <line x1="16" y1="17" x2="8" y2="17" />
        <polyline points="10 9 9 9 8 9" />
      </svg>
    ),
    Activity: () => (
      <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <polyline points="22 12 18 12 15 21 9 3 6 12 2 12" />
      </svg>
    ),
    Trace: () => (
      <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <polyline points="4 7 4 4 20 4 20 7" />
        <line x1="9" y1="20" x2="15" y2="20" />
        <line x1="12" y1="4" x2="12" y2="20" />
      </svg>
    ),
    Browse: () => (
      <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="12" cy="12" r="10" />
        <line x1="2" y1="12" x2="22" y2="12" />
        <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1 4-10 15.3 15.3 0 0 1 4-10z" />
      </svg>
    ),
    Data: () => (
      <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <ellipse cx="12" cy="5" rx="9" ry="3" />
        <path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3" />
        <path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5" />
      </svg>
    ),
    Skills: () => (
      <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2" />
      </svg>
    ),
    MCPs: () => (
      <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <rect x="2" y="3" width="20" height="14" rx="2" ry="2" />
        <line x1="8" y1="21" x2="16" y2="21" />
        <line x1="12" y1="17" x2="12" y2="21" />
      </svg>
    ),
    Settings: () => (
      <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="12" cy="12" r="3" />
        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
      </svg>
    )
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

      {/* Documents Section */}
      <div class="doc-list">
        <div class="doc-list-header">
          <span>Documents</span>
          <button class="new-doc-btn" onClick={handleNewDocument} title="New Document">
            +
          </button>
        </div>
        <Show
          when={documents().length > 0}
          fallback={
            <div class="no-docs">
              <p>No documents</p>
            </div>
          }
        >
          <For each={documents()}>
            {(doc) => (
              <div
                class={`doc-item ${props.activeDocId === doc.id ? "active" : ""}`}
                onClick={() => props.onSelectDoc(doc)}
              >
                <span class="doc-icon">📄</span>
                <div class="doc-info">
                  <div class="doc-item-title">{doc.title || "Untitled"}</div>
                  <div class="doc-date">{formatDate(doc.updated_at)}</div>
                </div>
                <button
                  class="doc-delete-btn"
                  onClick={(e) => handleDeleteDoc(e, doc.id)}
                  title="Delete document"
                >
                  ×
                </button>
              </div>
            )}
          </For>
        </Show>
      </div>

      <div class="sidebar-nav">
        <div class="nav-section">
          <button
            class={`nav-item ${props.showDocEditor ? "active" : ""}`}
            onClick={props.onDocClick}
          >
            <span class="nav-icon"><SidebarIcons.Docs /></span>
            <span class="nav-label">Docs</span>
          </button>
          <button
            class={`nav-item ${props.showActivityPanel ? "active" : ""}`}
            onClick={props.onActivityClick}
          >
            <span class="nav-icon"><SidebarIcons.Activity /></span>
            <span class="nav-label">Activity</span>
          </button>
          <button
            class={`nav-item ${props.showCapturePanel ? "active" : ""}`}
            onClick={props.onCaptureClick}
          >
            <span class="nav-icon"><SidebarIcons.Trace /></span>
            <span class="nav-label">Capture</span>
          </button>
          <button
            class={`nav-item ${props.showBrowserPanel ? "active" : ""}`}
            onClick={props.onBrowserClick}
          >
            <span class="nav-icon"><SidebarIcons.Browse /></span>
            <span class="nav-label">Browse</span>
          </button>
          <button
            class={`nav-item ${props.showDataPanels ? "active" : ""}`}
            onClick={props.onDataClick}
          >
            <span class="nav-icon"><SidebarIcons.Data /></span>
            <span class="nav-label">Data</span>
          </button>
        </div>

        <div class="nav-section divider">
          <button
            class="nav-item"
            onClick={props.onSkillsClick}
          >
            <span class="nav-icon"><SidebarIcons.Skills /></span>
            <span class="nav-label">Skills</span>
          </button>
          <button
            class="nav-item"
            onClick={props.onMCPClick}
          >
            <span class="nav-icon"><SidebarIcons.MCPs /></span>
            <span class="nav-label">MCPs</span>
          </button>
          <button 
            class="nav-item" 
            onClick={props.onSettingsClick}
          >
            <span class="nav-icon"><SidebarIcons.Settings /></span>
            <span class="nav-label">Settings</span>
          </button>
        </div>
      </div>
    </aside>
  );
};

export default TaskSidebar;
