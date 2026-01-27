import { invoke } from "@tauri-apps/api/core";
import { isTauri } from "./tauri-api";

// ==================== WorkStream Types ====================

export type ContextType = "document" | "task" | "browser" | "manual" | "mixed";

export interface WorkBlock {
  id: string;
  session_id: string | null;
  context_type: ContextType;
  context_id: string | null;
  context_title: string | null;
  started_at: number;
  ended_at: number;
  duration_secs: number;

  // Auto-generated
  auto_summary: string | null;
  edit_count: number;
  browse_count: number;
  research_urls: string[];

  // User-editable
  user_summary: string | null;
  notes: string | null;
  tags: string[];
  is_pinned: boolean;
  is_manual: boolean;

  created_at: number;
  updated_at: number;
}

export interface CreateBlockInput {
  context_type: string;
  context_id?: string;
  context_title?: string;
  started_at: number;
  ended_at: number;
  auto_summary?: string;
  edit_count: number;
  browse_count: number;
  research_urls: string[];
}

export interface ManualBlockInput {
  context_type?: string;
  context_id?: string;
  context_title?: string;
  started_at: number;
  ended_at: number;
  user_summary: string;
  notes?: string;
  tags?: string[];
}

export interface UpdateBlockInput {
  user_summary?: string;
  notes?: string;
  tags?: string[];
  is_pinned?: boolean;
}

export interface WorkBlockQuery {
  context_type?: string;
  context_id?: string;
  from_timestamp?: number;
  to_timestamp?: number;
  include_pinned_only?: boolean;
  limit?: number;
}

export interface Timeline {
  session: Session | null;
  blocks: WorkBlock[];
  total_duration_secs: number;
}

export interface Session {
  id: string;
  started_at: number;
  ended_at: number | null;
  summary: string | null;
  block_count: number;
  total_duration_secs: number;
}

export interface Milestone {
  id: string;
  context_type: ContextType;
  context_id: string;
  milestone_type: string;
  timestamp: number;
  note: string | null;
}

export interface MilestoneInput {
  context_type: string;
  context_id: string;
  milestone_type: string;
  note?: string;
}

export interface CleanupResult {
  blocks_deleted: number;
  sessions_deleted: number;
}

// ==================== Local Storage Fallback ====================

const STORAGE_KEY = "kuse-workstream-blocks";

function getLocalBlocks(): WorkBlock[] {
  const stored = localStorage.getItem(STORAGE_KEY);
  return stored ? JSON.parse(stored) : [];
}

function saveLocalBlocks(blocks: WorkBlock[]) {
  // Keep only last 50 blocks
  localStorage.setItem(STORAGE_KEY, JSON.stringify(blocks.slice(0, 50)));
}

// ==================== API Functions ====================

/**
 * Create a work block from buffer data
 */
export async function createBlock(input: CreateBlockInput): Promise<WorkBlock> {
  if (!isTauri()) {
    const block: WorkBlock = {
      id: crypto.randomUUID(),
      session_id: null,
      context_type: (input.context_type as ContextType) || "mixed",
      context_id: input.context_id || null,
      context_title: input.context_title || null,
      started_at: input.started_at,
      ended_at: input.ended_at,
      duration_secs: Math.floor((input.ended_at - input.started_at) / 1000),
      auto_summary: input.auto_summary || null,
      edit_count: input.edit_count,
      browse_count: input.browse_count,
      research_urls: input.research_urls,
      user_summary: null,
      notes: null,
      tags: [],
      is_pinned: false,
      is_manual: false,
      created_at: Date.now(),
      updated_at: Date.now(),
    };
    const blocks = getLocalBlocks();
    blocks.unshift(block);
    saveLocalBlocks(blocks);
    return block;
  }
  return invoke<WorkBlock>("ws_create_block", { input });
}

/**
 * Create a manual work block (user-entered)
 */
export async function createManualBlock(input: ManualBlockInput): Promise<WorkBlock> {
  if (!isTauri()) {
    const block: WorkBlock = {
      id: crypto.randomUUID(),
      session_id: null,
      context_type: (input.context_type as ContextType) || "manual",
      context_id: input.context_id || null,
      context_title: input.context_title || null,
      started_at: input.started_at,
      ended_at: input.ended_at,
      duration_secs: Math.floor((input.ended_at - input.started_at) / 1000),
      auto_summary: null,
      edit_count: 0,
      browse_count: 0,
      research_urls: [],
      user_summary: input.user_summary,
      notes: input.notes || null,
      tags: input.tags || [],
      is_pinned: false,
      is_manual: true,
      created_at: Date.now(),
      updated_at: Date.now(),
    };
    const blocks = getLocalBlocks();
    blocks.unshift(block);
    saveLocalBlocks(blocks);
    return block;
  }
  return invoke<WorkBlock>("ws_create_manual_block", { input });
}

/**
 * Update a work block (user edits)
 */
export async function updateBlock(id: string, input: UpdateBlockInput): Promise<WorkBlock | null> {
  if (!isTauri()) {
    const blocks = getLocalBlocks();
    const idx = blocks.findIndex((b) => b.id === id);
    if (idx < 0) return null;

    if (input.user_summary !== undefined) blocks[idx].user_summary = input.user_summary;
    if (input.notes !== undefined) blocks[idx].notes = input.notes;
    if (input.tags !== undefined) blocks[idx].tags = input.tags;
    if (input.is_pinned !== undefined) blocks[idx].is_pinned = input.is_pinned;
    blocks[idx].updated_at = Date.now();

    saveLocalBlocks(blocks);
    return blocks[idx];
  }
  return invoke<WorkBlock | null>("ws_update_block", { id, input });
}

/**
 * Get a work block by ID
 */
export async function getBlock(id: string): Promise<WorkBlock | null> {
  if (!isTauri()) {
    const blocks = getLocalBlocks();
    return blocks.find((b) => b.id === id) || null;
  }
  return invoke<WorkBlock | null>("ws_get_block", { id });
}

/**
 * List work blocks with filtering
 */
export async function listBlocks(query: WorkBlockQuery = {}): Promise<WorkBlock[]> {
  if (!isTauri()) {
    let blocks = getLocalBlocks();

    if (query.context_type) {
      blocks = blocks.filter((b) => b.context_type === query.context_type);
    }
    if (query.context_id) {
      blocks = blocks.filter((b) => b.context_id === query.context_id);
    }
    if (query.from_timestamp) {
      blocks = blocks.filter((b) => b.started_at >= query.from_timestamp!);
    }
    if (query.to_timestamp) {
      blocks = blocks.filter((b) => b.started_at <= query.to_timestamp!);
    }
    if (query.include_pinned_only) {
      blocks = blocks.filter((b) => b.is_pinned);
    }
    if (query.limit) {
      blocks = blocks.slice(0, query.limit);
    }

    return blocks;
  }
  return invoke<WorkBlock[]>("ws_list_blocks", { query });
}

/**
 * Delete a work block
 */
export async function deleteBlock(id: string): Promise<boolean> {
  if (!isTauri()) {
    const blocks = getLocalBlocks();
    const filtered = blocks.filter((b) => b.id !== id);
    saveLocalBlocks(filtered);
    return blocks.length !== filtered.length;
  }
  return invoke<boolean>("ws_delete_block", { id });
}

/**
 * Get timeline (recent work blocks)
 */
export async function getTimeline(limit?: number): Promise<Timeline> {
  if (!isTauri()) {
    const blocks = getLocalBlocks().slice(0, limit || 50);
    const total_duration_secs = blocks.reduce((sum, b) => sum + b.duration_secs, 0);
    return {
      session: null,
      blocks,
      total_duration_secs,
    };
  }
  return invoke<Timeline>("ws_get_timeline", { limit });
}

/**
 * AI-enhance a work block summary
 */
export async function enhanceSummary(id: string): Promise<string> {
  if (!isTauri()) {
    throw new Error("AI enhancement requires the desktop app");
  }
  return invoke<string>("ws_enhance_summary", { id });
}

/**
 * Create a milestone
 */
export async function createMilestone(input: MilestoneInput): Promise<Milestone> {
  if (!isTauri()) {
    throw new Error("Milestones require the desktop app");
  }
  return invoke<Milestone>("ws_create_milestone", { input });
}

/**
 * List milestones
 */
export async function listMilestones(contextType?: string, contextId?: string): Promise<Milestone[]> {
  if (!isTauri()) {
    return [];
  }
  return invoke<Milestone[]>("ws_list_milestones", { contextType, contextId });
}

/**
 * Cleanup old workstream data
 */
export async function cleanup(): Promise<CleanupResult> {
  if (!isTauri()) {
    return { blocks_deleted: 0, sessions_deleted: 0 };
  }
  return invoke<CleanupResult>("ws_cleanup");
}

// ==================== Helper Functions ====================

/**
 * Get display summary (user override wins)
 */
export function getDisplaySummary(block: WorkBlock): string {
  return block.user_summary || block.auto_summary || "Untitled work block";
}

/**
 * Format duration in human-readable form
 */
export function formatDuration(secs: number): string {
  if (secs < 60) return `${secs}s`;
  const mins = Math.floor(secs / 60);
  if (mins < 60) return `${mins}m`;
  const hours = Math.floor(mins / 60);
  const remainingMins = mins % 60;
  return remainingMins > 0 ? `${hours}h ${remainingMins}m` : `${hours}h`;
}

/**
 * Format relative time
 */
export function formatRelativeTime(timestamp: number): string {
  const now = Date.now();
  const diff = now - timestamp;
  const mins = Math.floor(diff / 60000);
  const hours = Math.floor(diff / 3600000);
  const days = Math.floor(diff / 86400000);

  if (mins < 1) return "just now";
  if (mins < 60) return `${mins}m ago`;
  if (hours < 24) return `${hours}h ago`;
  if (days < 7) return `${days}d ago`;
  return new Date(timestamp).toLocaleDateString();
}
