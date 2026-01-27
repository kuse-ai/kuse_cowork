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
  onTraceClick: () => void;
  onBrowserClick: () => void;
  onActivityClick: () => void;
  onDocClick: () => void;
  onSelectDoc: (doc: Document) => void;
  showDataPanels: boolean;
  showTracePanel: boolean;
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

      <div class="sidebar-footer">
        <button
          class={`footer-btn primary-btn ${props.showDocEditor ? "active" : ""}`}
          onClick={props.onDocClick}
          title="Document Editor"
        >
          Docs
        </button>
        <button
          class={`footer-btn primary-btn ${props.showActivityPanel ? "active" : ""}`}
          onClick={props.onActivityClick}
          title="Activity Timeline"
        >
          Activity
        </button>
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
