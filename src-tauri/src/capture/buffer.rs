use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use crate::capture::types::{
    ClipboardCapture, BrowseCapture, SearchCapture, AIExchangeCapture, DocEditCapture,
};

// ==================== Buffer Capacities ====================
// Memory Budget: ~100KB total
// Clipboard: 50 entries × ~500 bytes = ~25KB
// Browse: 100 entries × ~200 bytes = ~20KB
// Search: 50 entries × ~200 bytes = ~10KB
// AI Exchange: 20 entries × ~2KB = ~40KB
// Active Sources: 30 entries × ~200 bytes = ~6KB

const CLIPBOARD_CAPACITY: usize = 50;
const BROWSE_CAPACITY: usize = 100;
const SEARCH_CAPACITY: usize = 50;
const AI_EXCHANGE_CAPACITY: usize = 20;
const DOC_EDIT_CAPACITY: usize = 50;

/// Generic ring buffer that drops oldest items when full
#[derive(Debug)]
pub struct RingBuffer<T> {
    buffer: VecDeque<T>,
    capacity: usize,
}

impl<T> RingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// Push an item, dropping the oldest if at capacity
    pub fn push(&mut self, item: T) {
        if self.buffer.len() >= self.capacity {
            self.buffer.pop_front();
        }
        self.buffer.push_back(item);
    }

    /// Get current number of items
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Drain all items from the buffer
    pub fn drain_all(&mut self) -> Vec<T> {
        self.buffer.drain(..).collect()
    }

    /// Peek at the most recent item
    pub fn last(&self) -> Option<&T> {
        self.buffer.back()
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    /// Get capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

impl<T: Clone> RingBuffer<T> {
    /// Get all items without removing them
    pub fn get_all(&self) -> Vec<T> {
        self.buffer.iter().cloned().collect()
    }
}

/// Coordinated capture buffer managing all ring buffers
pub struct CaptureBuffer {
    pub clipboard: Arc<Mutex<RingBuffer<ClipboardCapture>>>,
    pub browse: Arc<Mutex<RingBuffer<BrowseCapture>>>,
    pub search: Arc<Mutex<RingBuffer<SearchCapture>>>,
    pub ai_exchange: Arc<Mutex<RingBuffer<AIExchangeCapture>>>,
    pub doc_edit: Arc<Mutex<RingBuffer<DocEditCapture>>>,

    /// Last clipboard content hash to detect duplicates
    last_clipboard_hash: Arc<Mutex<Option<String>>>,
}

impl CaptureBuffer {
    pub fn new() -> Self {
        Self {
            clipboard: Arc::new(Mutex::new(RingBuffer::new(CLIPBOARD_CAPACITY))),
            browse: Arc::new(Mutex::new(RingBuffer::new(BROWSE_CAPACITY))),
            search: Arc::new(Mutex::new(RingBuffer::new(SEARCH_CAPACITY))),
            ai_exchange: Arc::new(Mutex::new(RingBuffer::new(AI_EXCHANGE_CAPACITY))),
            doc_edit: Arc::new(Mutex::new(RingBuffer::new(DOC_EDIT_CAPACITY))),
            last_clipboard_hash: Arc::new(Mutex::new(None)),
        }
    }

    /// Push a clipboard capture, skipping if duplicate
    pub fn push_clipboard(&self, capture: ClipboardCapture) -> bool {
        let mut last_hash = self.last_clipboard_hash.lock().unwrap();

        // Skip if same as last capture
        if last_hash.as_ref() == Some(&capture.content_hash) {
            return false;
        }

        *last_hash = Some(capture.content_hash.clone());
        drop(last_hash);

        let mut buffer = self.clipboard.lock().unwrap();
        buffer.push(capture);
        true
    }

    /// Push a browse capture
    pub fn push_browse(&self, capture: BrowseCapture) {
        let mut buffer = self.browse.lock().unwrap();
        buffer.push(capture);
    }

    /// Update a browse capture with exit info
    pub fn update_browse(&self, browse_id: &str, left_at: i64, scroll_depth: Option<u8>) -> bool {
        let mut buffer = self.browse.lock().unwrap();

        // Find the browse entry and update it
        for capture in buffer.buffer.iter_mut() {
            if capture.id == browse_id {
                capture.left_at = Some(left_at);
                capture.scroll_depth_percent = scroll_depth;
                return true;
            }
        }
        false
    }

    /// Push a search capture
    pub fn push_search(&self, capture: SearchCapture) {
        let mut buffer = self.search.lock().unwrap();
        buffer.push(capture);
    }

    /// Update a search capture with clicked result
    pub fn update_search_click(&self, search_id: &str, clicked_url: &str) -> bool {
        let mut buffer = self.search.lock().unwrap();

        for capture in buffer.buffer.iter_mut() {
            if capture.id == search_id {
                capture.result_clicked = Some(clicked_url.to_string());
                return true;
            }
        }
        false
    }

    /// Push an AI exchange capture
    pub fn push_ai_exchange(&self, capture: AIExchangeCapture) {
        let mut buffer = self.ai_exchange.lock().unwrap();
        buffer.push(capture);
    }

    /// Push a document edit capture
    pub fn push_doc_edit(&self, capture: DocEditCapture) {
        let mut buffer = self.doc_edit.lock().unwrap();
        buffer.push(capture);
    }

    /// Drain all buffers for batch write
    pub fn drain_all(&self) -> CaptureBufferDrain {
        CaptureBufferDrain {
            clipboard: self.clipboard.lock().unwrap().drain_all(),
            browse: self.browse.lock().unwrap().drain_all(),
            search: self.search.lock().unwrap().drain_all(),
            ai_exchange: self.ai_exchange.lock().unwrap().drain_all(),
            doc_edit: self.doc_edit.lock().unwrap().drain_all(),
        }
    }

    /// Get current buffer status
    pub fn status(&self) -> CaptureBufferStatus {
        CaptureBufferStatus {
            clipboard_count: self.clipboard.lock().unwrap().len(),
            browse_count: self.browse.lock().unwrap().len(),
            search_count: self.search.lock().unwrap().len(),
            ai_exchange_count: self.ai_exchange.lock().unwrap().len(),
            doc_edit_count: self.doc_edit.lock().unwrap().len(),
        }
    }

    /// Check if any buffer has data to flush
    pub fn has_data(&self) -> bool {
        !self.clipboard.lock().unwrap().is_empty()
            || !self.browse.lock().unwrap().is_empty()
            || !self.search.lock().unwrap().is_empty()
            || !self.ai_exchange.lock().unwrap().is_empty()
            || !self.doc_edit.lock().unwrap().is_empty()
    }

    /// Clear all buffers
    pub fn clear(&self) {
        self.clipboard.lock().unwrap().clear();
        self.browse.lock().unwrap().clear();
        self.search.lock().unwrap().clear();
        self.ai_exchange.lock().unwrap().clear();
        self.doc_edit.lock().unwrap().clear();
        *self.last_clipboard_hash.lock().unwrap() = None;
    }
}

impl Default for CaptureBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of draining all capture buffers
#[derive(Debug)]
pub struct CaptureBufferDrain {
    pub clipboard: Vec<ClipboardCapture>,
    pub browse: Vec<BrowseCapture>,
    pub search: Vec<SearchCapture>,
    pub ai_exchange: Vec<AIExchangeCapture>,
    pub doc_edit: Vec<DocEditCapture>,
}

impl CaptureBufferDrain {
    pub fn is_empty(&self) -> bool {
        self.clipboard.is_empty()
            && self.browse.is_empty()
            && self.search.is_empty()
            && self.ai_exchange.is_empty()
            && self.doc_edit.is_empty()
    }

    pub fn total_count(&self) -> usize {
        self.clipboard.len() + self.browse.len() + self.search.len() + self.ai_exchange.len() + self.doc_edit.len()
    }
}

/// Buffer status for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureBufferStatus {
    pub clipboard_count: usize,
    pub browse_count: usize,
    pub search_count: usize,
    pub ai_exchange_count: usize,
    pub doc_edit_count: usize,
}

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer_basic() {
        let mut buffer: RingBuffer<i32> = RingBuffer::new(3);
        buffer.push(1);
        buffer.push(2);
        buffer.push(3);
        assert_eq!(buffer.len(), 3);

        // Push one more, should drop oldest
        buffer.push(4);
        assert_eq!(buffer.len(), 3);

        let items = buffer.drain_all();
        assert_eq!(items, vec![2, 3, 4]);
    }

    #[test]
    fn test_capture_buffer_duplicate_clipboard() {
        let buffer = CaptureBuffer::new();

        let capture1 = ClipboardCapture {
            id: "1".to_string(),
            content_hash: "hash1".to_string(),
            content_preview: "preview".to_string(),
            source_url: None,
            source_title: None,
            captured_at: 0,
        };

        let capture2 = ClipboardCapture {
            id: "2".to_string(),
            content_hash: "hash1".to_string(), // Same hash
            content_preview: "preview".to_string(),
            source_url: None,
            source_title: None,
            captured_at: 1,
        };

        assert!(buffer.push_clipboard(capture1));
        assert!(!buffer.push_clipboard(capture2)); // Should be skipped

        assert_eq!(buffer.clipboard.lock().unwrap().len(), 1);
    }

    #[test]
    fn test_capture_buffer_drain() {
        let buffer = CaptureBuffer::new();

        buffer.push_browse(BrowseCapture {
            id: "1".to_string(),
            url: "https://example.com".to_string(),
            page_title: Some("Example".to_string()),
            entered_at: 0,
            left_at: None,
            scroll_depth_percent: None,
        });

        let drain = buffer.drain_all();
        assert_eq!(drain.browse.len(), 1);
        assert!(buffer.browse.lock().unwrap().is_empty());
    }
}
