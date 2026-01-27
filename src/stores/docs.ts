import { createSignal } from "solid-js";
import {
  Document,
  CreateDocumentInput,
  UpdateDocumentInput,
  createDocument as createDocumentApi,
  getDocument as getDocumentApi,
  updateDocument as updateDocumentApi,
  listDocuments as listDocumentsApi,
  deleteDocument as deleteDocumentApi,
} from "../lib/tauri-api";

export type { Document, CreateDocumentInput, UpdateDocumentInput };

// Document state
const [documents, setDocuments] = createSignal<Document[]>([]);
const [activeDoc, setActiveDoc] = createSignal<Document | null>(null);
const [isDirty, setIsDirty] = createSignal(false);
const [isLoading, setIsLoading] = createSignal(false);
const [error, setError] = createSignal<string | null>(null);

export function useDocs() {
  /**
   * Load all documents from the backend
   */
  const loadDocuments = async () => {
    setIsLoading(true);
    setError(null);
    try {
      const docs = await listDocumentsApi();
      setDocuments(docs);
    } catch (e) {
      console.error("Failed to load documents:", e);
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setIsLoading(false);
    }
  };

  /**
   * Create a new document
   */
  const createDocument = async (input: CreateDocumentInput): Promise<Document | null> => {
    setError(null);
    try {
      const doc = await createDocumentApi(input);
      setDocuments((prev) => [doc, ...prev]);
      return doc;
    } catch (e) {
      console.error("Failed to create document:", e);
      setError(e instanceof Error ? e.message : String(e));
      return null;
    }
  };

  /**
   * Open a document by ID and set it as active
   */
  const openDocument = async (id: string): Promise<Document | null> => {
    setIsLoading(true);
    setError(null);
    try {
      const doc = await getDocumentApi(id);
      if (doc) {
        setActiveDoc(doc);
        setIsDirty(false);
      }
      return doc;
    } catch (e) {
      console.error("Failed to open document:", e);
      setError(e instanceof Error ? e.message : String(e));
      return null;
    } finally {
      setIsLoading(false);
    }
  };

  /**
   * Save the current active document
   */
  const saveDocument = async (): Promise<boolean> => {
    const doc = activeDoc();
    if (!doc) return false;

    setError(null);
    try {
      const updated = await updateDocumentApi(doc.id, {
        title: doc.title,
        content: doc.content,
      });
      if (updated) {
        setActiveDoc(updated);
        setDocuments((prev) =>
          prev.map((d) => (d.id === updated.id ? updated : d))
        );
        setIsDirty(false);
        return true;
      }
      return false;
    } catch (e) {
      console.error("Failed to save document:", e);
      setError(e instanceof Error ? e.message : String(e));
      return false;
    }
  };

  /**
   * Update document content locally (marks as dirty)
   */
  const updateContent = (content: string) => {
    const doc = activeDoc();
    if (doc) {
      setActiveDoc({ ...doc, content });
      setIsDirty(true);
    }
  };

  /**
   * Update document title locally (marks as dirty)
   */
  const updateTitle = (title: string) => {
    const doc = activeDoc();
    if (doc) {
      setActiveDoc({ ...doc, title });
      setIsDirty(true);
    }
  };

  /**
   * Delete a document
   */
  const deleteDocument = async (id: string): Promise<boolean> => {
    setError(null);
    try {
      const deleted = await deleteDocumentApi(id);
      if (deleted) {
        setDocuments((prev) => prev.filter((d) => d.id !== id));
        // Clear active doc if it was deleted
        if (activeDoc()?.id === id) {
          setActiveDoc(null);
          setIsDirty(false);
        }
      }
      return deleted;
    } catch (e) {
      console.error("Failed to delete document:", e);
      setError(e instanceof Error ? e.message : String(e));
      return false;
    }
  };

  /**
   * Close the active document
   */
  const closeDocument = () => {
    setActiveDoc(null);
    setIsDirty(false);
  };

  return {
    // State
    documents,
    activeDoc,
    isDirty,
    isLoading,
    error,
    // Actions
    loadDocuments,
    createDocument,
    openDocument,
    saveDocument,
    updateContent,
    updateTitle,
    deleteDocument,
    closeDocument,
    // Direct setters for advanced use cases
    setActiveDoc,
    setIsDirty,
  };
}
