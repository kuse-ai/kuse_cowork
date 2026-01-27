import { Component, For, Show, createSignal, createEffect, onMount } from "solid-js";
import {
  useWorkStream,
  WorkBlock,
  ManualBlockInput,
  getDisplaySummary,
  formatDuration,
  formatRelativeTime,
} from "../stores/workstream";
import "./WorkStreamPanel.css";

interface WorkStreamPanelProps {
  contextId?: string;
  contextType?: string;
  onClose?: () => void;
}

const WorkStreamPanel: Component<WorkStreamPanelProps> = (props) => {
  const {
    blocks,
    isLoading,
    isEnhancing,
    contentBuffer,
    loadTimeline,
    createSnapshot,
    addManualEntry,
    editBlock,
    removeBlock,
    pinBlock,
    enhanceBlockSummary,
    runCleanup,
  } = useWorkStream();

  const [expandedBlockId, setExpandedBlockId] = createSignal<string | null>(null);
  const [editingBlockId, setEditingBlockId] = createSignal<string | null>(null);
  const [showManualForm, setShowManualForm] = createSignal(false);

  // Manual entry form state
  const [manualSummary, setManualSummary] = createSignal("");
  const [manualNotes, setManualNotes] = createSignal("");
  const [manualTags, setManualTags] = createSignal("");
  const [manualDuration, setManualDuration] = createSignal(30); // minutes

  // Edit form state
  const [editSummary, setEditSummary] = createSignal("");
  const [editNotes, setEditNotes] = createSignal("");
  const [editTags, setEditTags] = createSignal("");

  onMount(() => {
    loadTimeline();
  });

  createEffect(() => {
    if (props.contextId || props.contextType) {
      loadTimeline({ context_id: props.contextId, context_type: props.contextType });
    }
  });

  const handleToggleExpand = (blockId: string) => {
    if (expandedBlockId() === blockId) {
      setExpandedBlockId(null);
    } else {
      setExpandedBlockId(blockId);
    }
  };

  const handleStartEdit = (block: WorkBlock) => {
    setEditingBlockId(block.id);
    setEditSummary(block.user_summary || "");
    setEditNotes(block.notes || "");
    setEditTags(block.tags.join(", "));
  };

  const handleSaveEdit = async (blockId: string) => {
    const tags = editTags()
      .split(",")
      .map((t) => t.trim())
      .filter((t) => t.length > 0);

    await editBlock(blockId, {
      user_summary: editSummary() || undefined,
      notes: editNotes() || undefined,
      tags: tags.length > 0 ? tags : undefined,
    });

    setEditingBlockId(null);
  };

  const handleCancelEdit = () => {
    setEditingBlockId(null);
    setEditSummary("");
    setEditNotes("");
    setEditTags("");
  };

  const handleTogglePin = async (block: WorkBlock) => {
    await pinBlock(block.id, !block.is_pinned);
  };

  const handleDelete = async (blockId: string) => {
    if (confirm("Delete this work block?")) {
      await removeBlock(blockId);
      if (expandedBlockId() === blockId) {
        setExpandedBlockId(null);
      }
    }
  };

  const handleEnhance = async (blockId: string) => {
    await enhanceBlockSummary(blockId);
  };

  const handleCreateSnapshot = async () => {
    await createSnapshot();
  };

  const handleSubmitManualEntry = async () => {
    if (!manualSummary().trim()) return;

    const now = Date.now();
    const durationMs = manualDuration() * 60 * 1000;
    const tags = manualTags()
      .split(",")
      .map((t) => t.trim())
      .filter((t) => t.length > 0);

    const input: ManualBlockInput = {
      started_at: now - durationMs,
      ended_at: now,
      user_summary: manualSummary().trim(),
      notes: manualNotes().trim() || undefined,
      tags: tags.length > 0 ? tags : undefined,
      context_type: props.contextType || "manual",
      context_id: props.contextId,
    };

    await addManualEntry(input);

    // Reset form
    setManualSummary("");
    setManualNotes("");
    setManualTags("");
    setManualDuration(30);
    setShowManualForm(false);
  };

  const handleCleanup = async () => {
    const result = await runCleanup();
    if (result && result.blocks_deleted > 0) {
      alert(`Cleaned up ${result.blocks_deleted} old blocks`);
    }
  };

  const getContextIcon = (contextType: string): string => {
    switch (contextType) {
      case "document": return "D";
      case "task": return "T";
      case "browser": return "B";
      case "manual": return "M";
      case "mixed": return "X";
      default: return "?";
    }
  };

  const pendingContentCount = () => contentBuffer().length;
  const hasContent = () => pendingContentCount() >= 1;

  return (
    <div class="workstream-panel">
      <div class="workstream-header">
        <h3>WorkStream</h3>
        <div class="workstream-header-actions">
          <button
            class="ws-btn ws-btn-text"
            onClick={() => setShowManualForm(!showManualForm())}
            title="Add manual entry"
          >
            + Add
          </button>
          <button
            class="ws-btn ws-btn-text"
            onClick={handleCleanup}
            title="Clean up old entries"
          >
            Cleanup
          </button>
          {props.onClose && (
            <button class="ws-close-btn" onClick={props.onClose}>
              ×
            </button>
          )}
        </div>
      </div>

      {/* Manual Entry Form */}
      <Show when={showManualForm()}>
        <div class="manual-entry-form">
          <div class="form-header">
            <span>Add Manual Entry</span>
            <button class="ws-btn ws-btn-text" onClick={() => setShowManualForm(false)}>
              Cancel
            </button>
          </div>
          <div class="form-field">
            <label>What did you work on?</label>
            <input
              type="text"
              value={manualSummary()}
              onInput={(e) => setManualSummary(e.currentTarget.value)}
              placeholder="e.g., Reviewed Q3 budget proposal"
            />
          </div>
          <div class="form-field">
            <label>Notes (optional)</label>
            <textarea
              value={manualNotes()}
              onInput={(e) => setManualNotes(e.currentTarget.value)}
              placeholder="Additional context or details..."
              rows={2}
            />
          </div>
          <div class="form-row">
            <div class="form-field form-field-half">
              <label>Duration (minutes)</label>
              <input
                type="number"
                value={manualDuration()}
                onInput={(e) => setManualDuration(parseInt(e.currentTarget.value) || 30)}
                min={1}
                max={480}
              />
            </div>
            <div class="form-field form-field-half">
              <label>Tags (comma separated)</label>
              <input
                type="text"
                value={manualTags()}
                onInput={(e) => setManualTags(e.currentTarget.value)}
                placeholder="meeting, review"
              />
            </div>
          </div>
          <button
            class="ws-btn ws-btn-primary"
            onClick={handleSubmitManualEntry}
            disabled={!manualSummary().trim()}
          >
            Add Entry
          </button>
        </div>
      </Show>

      {/* Content Buffer Status */}
      <Show when={pendingContentCount() > 0}>
        <div class="event-buffer-status">
          <div class="buffer-info">
            <span class="buffer-dot" />
            <span class="buffer-text">{pendingContentCount()} content captured</span>
          </div>
          <Show when={hasContent()}>
            <button
              class="ws-btn ws-btn-small"
              onClick={handleCreateSnapshot}
              disabled={isLoading()}
            >
              Save Now
            </button>
          </Show>
        </div>
      </Show>

      {/* Timeline */}
      <div class="workstream-timeline">
        <Show
          when={!isLoading()}
          fallback={<div class="ws-loading">Loading timeline...</div>}
        >
          <Show
            when={blocks().length > 0}
            fallback={
              <div class="ws-empty">
                <p>No work recorded yet</p>
                <p class="hint">Your activity will appear here automatically</p>
                <button
                  class="ws-btn ws-btn-outline"
                  onClick={() => setShowManualForm(true)}
                >
                  Add Manual Entry
                </button>
              </div>
            }
          >
            <div class="ws-blocks-list">
              <For each={blocks()}>
                {(block) => (
                  <WorkBlockCard
                    block={block}
                    isExpanded={expandedBlockId() === block.id}
                    isEditing={editingBlockId() === block.id}
                    isEnhancing={isEnhancing()}
                    editSummary={editSummary}
                    setEditSummary={setEditSummary}
                    editNotes={editNotes}
                    setEditNotes={setEditNotes}
                    editTags={editTags}
                    setEditTags={setEditTags}
                    onToggle={() => handleToggleExpand(block.id)}
                    onStartEdit={() => handleStartEdit(block)}
                    onSaveEdit={() => handleSaveEdit(block.id)}
                    onCancelEdit={handleCancelEdit}
                    onTogglePin={() => handleTogglePin(block)}
                    onDelete={() => handleDelete(block.id)}
                    onEnhance={() => handleEnhance(block.id)}
                    getContextIcon={getContextIcon}
                  />
                )}
              </For>
            </div>
          </Show>
        </Show>
      </div>
    </div>
  );
};

interface WorkBlockCardProps {
  block: WorkBlock;
  isExpanded: boolean;
  isEditing: boolean;
  isEnhancing: boolean;
  editSummary: () => string;
  setEditSummary: (v: string) => void;
  editNotes: () => string;
  setEditNotes: (v: string) => void;
  editTags: () => string;
  setEditTags: (v: string) => void;
  onToggle: () => void;
  onStartEdit: () => void;
  onSaveEdit: () => void;
  onCancelEdit: () => void;
  onTogglePin: () => void;
  onDelete: () => void;
  onEnhance: () => void;
  getContextIcon: (type: string) => string;
}

const WorkBlockCard: Component<WorkBlockCardProps> = (props) => {
  const summary = () => getDisplaySummary(props.block);
  const duration = () => formatDuration(props.block.duration_secs);
  const timeAgo = () => formatRelativeTime(props.block.started_at);

  return (
    <div
      class={`ws-block-card ${props.isExpanded ? "expanded" : ""} ${
        props.block.is_pinned ? "pinned" : ""
      } ${props.block.is_manual ? "manual" : ""}`}
    >
      <div class="ws-block-header" onClick={props.onToggle}>
        <div class="ws-block-main">
          <span class={`ws-context-icon ${props.block.context_type}`}>
            {props.getContextIcon(props.block.context_type)}
          </span>
          <div class="ws-block-content">
            <span class="ws-block-summary">{summary()}</span>
            <div class="ws-block-meta">
              <span class="ws-block-time">{timeAgo()}</span>
              <span class="ws-block-duration">{duration()}</span>
              <Show when={props.block.is_manual}>
                <span class="ws-tag ws-tag-manual">manual</span>
              </Show>
              <Show when={props.block.is_pinned}>
                <span class="ws-tag ws-tag-pinned">pinned</span>
              </Show>
              <Show when={props.block.tags.length > 0}>
                <For each={props.block.tags.slice(0, 2)}>
                  {(tag) => <span class="ws-tag">{tag}</span>}
                </For>
                <Show when={props.block.tags.length > 2}>
                  <span class="ws-tag">+{props.block.tags.length - 2}</span>
                </Show>
              </Show>
            </div>
          </div>
        </div>
        <span class="ws-expand-icon">{props.isExpanded ? "▼" : "▶"}</span>
      </div>

      <Show when={props.isExpanded}>
        <div class="ws-block-details">
          <Show when={!props.isEditing}>
            {/* View Mode */}
            <div class="ws-detail-section">
              <div class="ws-detail-row">
                <span class="ws-detail-label">Context</span>
                <span class="ws-detail-value">
                  {props.block.context_title || props.block.context_type}
                </span>
              </div>
              <Show when={props.block.notes}>
                <div class="ws-detail-row">
                  <span class="ws-detail-label">Notes</span>
                  <span class="ws-detail-value ws-detail-notes">{props.block.notes}</span>
                </div>
              </Show>
              <Show when={props.block.research_urls.length > 0}>
                <div class="ws-detail-row">
                  <span class="ws-detail-label">Research</span>
                  <div class="ws-research-links">
                    <For each={props.block.research_urls}>
                      {(url) => (
                        <a href={url} target="_blank" rel="noopener noreferrer" class="ws-research-link">
                          {new URL(url).hostname}
                        </a>
                      )}
                    </For>
                  </div>
                </div>
              </Show>
              <Show when={!props.block.is_manual && props.block.edit_count > 0}>
                <div class="ws-detail-row">
                  <span class="ws-detail-label">Activity</span>
                  <span class="ws-detail-value">
                    {props.block.edit_count} edits
                    {props.block.browse_count > 0 && `, ${props.block.browse_count} pages`}
                  </span>
                </div>
              </Show>
            </div>

            <div class="ws-block-actions">
              <button class="ws-btn ws-btn-small" onClick={props.onStartEdit}>
                Edit
              </button>
              <button
                class={`ws-btn ws-btn-small ${props.block.is_pinned ? "active" : ""}`}
                onClick={(e) => {
                  e.stopPropagation();
                  props.onTogglePin();
                }}
              >
                {props.block.is_pinned ? "Unpin" : "Pin"}
              </button>
              <Show when={!props.block.is_manual && !props.block.user_summary}>
                <button
                  class="ws-btn ws-btn-small ws-btn-enhance"
                  onClick={(e) => {
                    e.stopPropagation();
                    props.onEnhance();
                  }}
                  disabled={props.isEnhancing}
                >
                  {props.isEnhancing ? "..." : "AI Enhance"}
                </button>
              </Show>
              <button
                class="ws-btn ws-btn-small ws-btn-danger"
                onClick={(e) => {
                  e.stopPropagation();
                  props.onDelete();
                }}
              >
                Delete
              </button>
            </div>
          </Show>

          <Show when={props.isEditing}>
            {/* Edit Mode */}
            <div class="ws-edit-form">
              <div class="form-field">
                <label>Summary</label>
                <input
                  type="text"
                  value={props.editSummary()}
                  onInput={(e) => props.setEditSummary(e.currentTarget.value)}
                  placeholder={props.block.auto_summary || "Enter summary..."}
                />
              </div>
              <div class="form-field">
                <label>Notes</label>
                <textarea
                  value={props.editNotes()}
                  onInput={(e) => props.setEditNotes(e.currentTarget.value)}
                  placeholder="Add notes..."
                  rows={3}
                />
              </div>
              <div class="form-field">
                <label>Tags (comma separated)</label>
                <input
                  type="text"
                  value={props.editTags()}
                  onInput={(e) => props.setEditTags(e.currentTarget.value)}
                  placeholder="tag1, tag2"
                />
              </div>
              <div class="ws-edit-actions">
                <button class="ws-btn ws-btn-small" onClick={props.onCancelEdit}>
                  Cancel
                </button>
                <button class="ws-btn ws-btn-small ws-btn-primary" onClick={props.onSaveEdit}>
                  Save
                </button>
              </div>
            </div>
          </Show>
        </div>
      </Show>
    </div>
  );
};

export default WorkStreamPanel;
