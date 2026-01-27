import { Component, Show, createEffect, onMount, onCleanup, createSignal } from "solid-js";
import { createTiptapEditor } from "solid-tiptap";
import StarterKit from "@tiptap/starter-kit";
import Placeholder from "@tiptap/extension-placeholder";
import Underline from "@tiptap/extension-underline";
import { useDocs, Document } from "../stores/docs";
import { useTraces } from "../stores/traces";
import { createDocumentTracker } from "../stores/workstream";
import { exportToDocx, importFromDocx } from "../lib/docx-utils";
import "./DocEditor.css";

interface DocEditorProps {
  docId: string | null;
  onClose?: () => void;
  onDocumentChange?: (doc: Document | null) => void;
}

const DocEditor: Component<DocEditorProps> = (props) => {
  const {
    activeDoc,
    isDirty,
    isLoading,
    error,
    openDocument,
    saveDocument,
    updateContent,
    updateTitle,
    closeDocument,
    createDocument,
    setActiveDoc,
  } = useDocs();

  const { logTrace, loadTraces } = useTraces();

  // WorkStream activity tracking
  const { trackEdit, trackFocus, trackBlur, trackSave } = createDocumentTracker(
    () => activeDoc()?.id || null,
    () => activeDoc()?.title || null
  );

  const [isExporting, setIsExporting] = createSignal(false);
  const [isImporting, setIsImporting] = createSignal(false);
  const [currentLoadedDocId, setCurrentLoadedDocId] = createSignal<string | null>(null);
  let editorContainer: HTMLDivElement | undefined;
  let fileInputRef: HTMLInputElement | undefined;
  let lastContentLength = 0;
  let editTraceTimeout: number | null = null;
  let isUpdatingFromEditor = false;

  // Debounced trace logging for edits
  const logEditTrace = (newLength: number, snippet?: string) => {
    const doc = activeDoc();
    if (!doc) return;

    // Clear existing timeout
    if (editTraceTimeout) {
      clearTimeout(editTraceTimeout);
    }

    // Debounce trace logging (log after 1 second of inactivity)
    editTraceTimeout = window.setTimeout(() => {
      const delta = newLength - lastContentLength;
      lastContentLength = newLength;

      // Log to trace system
      logTrace({
        doc_id: doc.id,
        event_type: "edit",
        delta,
        payload: {
          action: delta > 0 ? "insert" : delta < 0 ? "delete" : "modify",
          chars_changed: Math.abs(delta),
        },
      });

      // Log to activity system
      trackEdit(delta, snippet);
    }, 1000);
  };

  const editor = createTiptapEditor(() => ({
    element: editorContainer!,
    extensions: [
      StarterKit.configure({
        heading: {
          levels: [1, 2, 3],
        },
      }),
      Placeholder.configure({
        placeholder: "Start writing...",
      }),
      Underline.configure({}),
    ],
    content: "",
    editorProps: {
      attributes: {
        class: "prose-editor",
      },
    },
    onUpdate: ({ editor }) => {
      isUpdatingFromEditor = true;
      const html = editor.getHTML();
      updateContent(html);
      // Log edit trace (debounced) with snippet
      const text = editor.getText().slice(0, 100);
      logEditTrace(html.length, text);
      isUpdatingFromEditor = false;
    },
    onFocus: () => {
      trackFocus();
    },
    onBlur: () => {
      trackBlur();
    },
  }));

  // Load document when docId changes
  createEffect(() => {
    const docId = props.docId;
    if (docId) {
      openDocument(docId);
      // Load traces for this document
      loadTraces(docId, 100);
    } else {
      closeDocument();
      setCurrentLoadedDocId(null);
    }
  });

  // Sync editor content when document loads or editor becomes ready
  createEffect(() => {
    const doc = activeDoc();
    const ed = editor();
    const targetDocId = props.docId;
    const loadedId = currentLoadedDocId();

    // Sync content when:
    // 1. We have editor and document
    // 2. The document matches the requested docId
    // 3. We haven't loaded this doc yet
    if (ed && doc && targetDocId && doc.id === targetDocId && loadedId !== doc.id) {
      setCurrentLoadedDocId(doc.id);
      ed.commands.setContent(doc.content || "", false);
      // Initialize content length for delta tracking
      lastContentLength = doc.content?.length || 0;
    }
    props.onDocumentChange?.(doc);
  });

  // Keyboard shortcuts
  const handleKeyDown = (e: KeyboardEvent) => {
    if ((e.metaKey || e.ctrlKey) && e.key === "s") {
      e.preventDefault();
      handleSave();
    }
  };

  onMount(() => {
    document.addEventListener("keydown", handleKeyDown);
  });

  onCleanup(() => {
    document.removeEventListener("keydown", handleKeyDown);
    // Clean up debounce timeout
    if (editTraceTimeout) {
      clearTimeout(editTraceTimeout);
    }
  });

  const handleSave = async () => {
    if (!isDirty()) return;
    const doc = activeDoc();
    await saveDocument();
    if (doc) {
      logTrace({
        doc_id: doc.id,
        event_type: "edit",
        payload: { action: "save", title: doc.title },
      });
      // Log to activity system
      trackSave();
    }
  };

  const handleTitleChange = (e: Event) => {
    const target = e.target as HTMLInputElement;
    updateTitle(target.value);
  };

  const handleExport = async () => {
    const doc = activeDoc();
    if (!doc) return;

    setIsExporting(true);
    try {
      await exportToDocx(doc.title, doc.content);
      logTrace({
        doc_id: doc.id,
        event_type: "edit",
        payload: { action: "export", format: "docx", title: doc.title },
      });
    } catch (err) {
      console.error("Export failed:", err);
    } finally {
      setIsExporting(false);
    }
  };

  const handleImportClick = () => {
    fileInputRef?.click();
  };

  const handleImport = async (e: Event) => {
    const target = e.target as HTMLInputElement;
    const file = target.files?.[0];
    if (!file) return;

    setIsImporting(true);
    try {
      const { title, content } = await importFromDocx(file);

      // Create a new document with imported content
      const newDoc = await createDocument({ title, content });
      if (newDoc) {
        setActiveDoc(newDoc);
        // Load traces for the new document
        loadTraces(newDoc.id, 100);
        // Log import trace
        logTrace({
          doc_id: newDoc.id,
          event_type: "edit",
          payload: { action: "import", format: "docx", filename: file.name, title },
        });
      }
    } catch (err) {
      console.error("Import failed:", err);
    } finally {
      setIsImporting(false);
      // Reset file input
      target.value = "";
    }
  };

  const handleClose = () => {
    closeDocument();
    props.onClose?.();
  };

  // Toolbar actions
  const toggleBold = () => editor()?.chain().focus().toggleBold().run();
  const toggleItalic = () => editor()?.chain().focus().toggleItalic().run();
  const toggleUnderline = () => editor()?.chain().focus().toggleUnderline().run();
  const toggleStrike = () => editor()?.chain().focus().toggleStrike().run();
  const toggleH1 = () => editor()?.chain().focus().toggleHeading({ level: 1 }).run();
  const toggleH2 = () => editor()?.chain().focus().toggleHeading({ level: 2 }).run();
  const toggleH3 = () => editor()?.chain().focus().toggleHeading({ level: 3 }).run();
  const toggleBulletList = () => editor()?.chain().focus().toggleBulletList().run();
  const toggleOrderedList = () => editor()?.chain().focus().toggleOrderedList().run();
  const toggleBlockquote = () => editor()?.chain().focus().toggleBlockquote().run();
  const toggleCode = () => editor()?.chain().focus().toggleCode().run();
  const toggleCodeBlock = () => editor()?.chain().focus().toggleCodeBlock().run();
  const undo = () => editor()?.chain().focus().undo().run();
  const redo = () => editor()?.chain().focus().redo().run();

  return (
    <div class="doc-editor">
      <div class="doc-editor-header">
        <div class="doc-editor-title-row">
          <Show
            when={activeDoc()}
            fallback={
              <span class="doc-editor-placeholder">No document open</span>
            }
          >
            <input
              type="text"
              class="doc-editor-title"
              value={activeDoc()?.title || ""}
              onInput={handleTitleChange}
              placeholder="Untitled Document"
            />
          </Show>
          <div class="doc-editor-actions">
            <Show when={isDirty()}>
              <span class="doc-editor-dirty">Unsaved</span>
            </Show>
            <button
              class="doc-editor-btn"
              onClick={handleSave}
              disabled={!isDirty() || isLoading()}
              title="Save (Cmd+S)"
            >
              Save
            </button>
            <button
              class="doc-editor-btn"
              onClick={handleExport}
              disabled={!activeDoc() || isExporting()}
              title="Export to DOCX"
            >
              {isExporting() ? "..." : "Export"}
            </button>
            <button
              class="doc-editor-btn"
              onClick={handleImportClick}
              disabled={isImporting()}
              title="Import DOCX"
            >
              {isImporting() ? "..." : "Import"}
            </button>
            <input
              ref={fileInputRef}
              type="file"
              accept=".docx"
              style="display: none"
              onChange={handleImport}
            />
            {props.onClose && (
              <button
                class="doc-editor-btn doc-editor-close"
                onClick={handleClose}
                title="Close"
              >
                ×
              </button>
            )}
          </div>
        </div>

        {/* Toolbar */}
        <Show when={activeDoc()}>
          <div class="doc-editor-toolbar">
            <button
              class={`toolbar-btn ${editor()?.isActive("bold") ? "active" : ""}`}
              onClick={toggleBold}
              title="Bold (Cmd+B)"
            >
              B
            </button>
            <button
              class={`toolbar-btn ${editor()?.isActive("italic") ? "active" : ""}`}
              onClick={toggleItalic}
              title="Italic (Cmd+I)"
            >
              I
            </button>
            <button
              class={`toolbar-btn ${editor()?.isActive("underline") ? "active" : ""}`}
              onClick={toggleUnderline}
              title="Underline (Cmd+U)"
            >
              U
            </button>
            <button
              class={`toolbar-btn ${editor()?.isActive("strike") ? "active" : ""}`}
              onClick={toggleStrike}
              title="Strikethrough"
            >
              S
            </button>
            <div class="toolbar-divider" />
            <button
              class={`toolbar-btn ${editor()?.isActive("heading", { level: 1 }) ? "active" : ""}`}
              onClick={toggleH1}
              title="Heading 1"
            >
              H1
            </button>
            <button
              class={`toolbar-btn ${editor()?.isActive("heading", { level: 2 }) ? "active" : ""}`}
              onClick={toggleH2}
              title="Heading 2"
            >
              H2
            </button>
            <button
              class={`toolbar-btn ${editor()?.isActive("heading", { level: 3 }) ? "active" : ""}`}
              onClick={toggleH3}
              title="Heading 3"
            >
              H3
            </button>
            <div class="toolbar-divider" />
            <button
              class={`toolbar-btn ${editor()?.isActive("bulletList") ? "active" : ""}`}
              onClick={toggleBulletList}
              title="Bullet List"
            >
              •
            </button>
            <button
              class={`toolbar-btn ${editor()?.isActive("orderedList") ? "active" : ""}`}
              onClick={toggleOrderedList}
              title="Numbered List"
            >
              1.
            </button>
            <button
              class={`toolbar-btn ${editor()?.isActive("blockquote") ? "active" : ""}`}
              onClick={toggleBlockquote}
              title="Quote"
            >
              "
            </button>
            <div class="toolbar-divider" />
            <button
              class={`toolbar-btn ${editor()?.isActive("code") ? "active" : ""}`}
              onClick={toggleCode}
              title="Inline Code"
            >
              {"<>"}
            </button>
            <button
              class={`toolbar-btn ${editor()?.isActive("codeBlock") ? "active" : ""}`}
              onClick={toggleCodeBlock}
              title="Code Block"
            >
              {"{ }"}
            </button>
            <div class="toolbar-divider" />
            <button
              class="toolbar-btn"
              onClick={undo}
              title="Undo (Cmd+Z)"
            >
              ↩
            </button>
            <button
              class="toolbar-btn"
              onClick={redo}
              title="Redo (Cmd+Shift+Z)"
            >
              ↪
            </button>
          </div>
        </Show>
      </div>

      <Show when={error()}>
        <div class="doc-editor-error">{error()}</div>
      </Show>

      <Show when={isLoading()}>
        <div class="doc-editor-loading">Loading...</div>
      </Show>

      <Show when={!activeDoc() && !isLoading()}>
        <div class="doc-editor-empty">
          <p>Select a document or create a new one</p>
        </div>
      </Show>

      {/* Editor container - always rendered but hidden when no doc */}
      <div
        class="doc-editor-content"
        ref={editorContainer}
        style={{ display: activeDoc() && !isLoading() ? "block" : "none" }}
      />
    </div>
  );
};

export default DocEditor;
