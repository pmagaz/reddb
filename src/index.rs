use std::collections::HashMap;
use uuid::Uuid;

/// A boxed function that extracts a string key from raw document bytes.
/// Returns `None` if the bytes cannot be decoded (e.g., wrong document type).
pub(crate) type ExtractorFn = Box<dyn Fn(&[u8]) -> Option<String> + Send + Sync>;

pub(crate) struct IndexEntry {
    pub(crate) extractor: ExtractorFn,
    /// key → list of document IDs that have that key value
    pub(crate) keys: HashMap<String, Vec<Uuid>>,
}

pub(crate) struct IndexRegistry {
    pub(crate) entries: HashMap<String, IndexEntry>,
}

impl Default for IndexRegistry {
    fn default() -> Self {
        IndexRegistry { entries: HashMap::new() }
    }
}

impl IndexRegistry {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn on_insert(&mut self, id: Uuid, raw: &[u8]) {
        for entry in self.entries.values_mut() {
            if let Some(key) = (entry.extractor)(raw) {
                entry.keys.entry(key).or_default().push(id);
            }
        }
    }

    pub(crate) fn on_delete(&mut self, id: Uuid, raw: &[u8]) {
        for entry in self.entries.values_mut() {
            if let Some(key) = (entry.extractor)(raw) {
                if let Some(ids) = entry.keys.get_mut(&key) {
                    ids.retain(|&eid| eid != id);
                    if ids.is_empty() {
                        entry.keys.remove(&key);
                    }
                }
            }
        }
    }

    pub(crate) fn on_update(&mut self, id: Uuid, old_raw: &[u8], new_raw: &[u8]) {
        for entry in self.entries.values_mut() {
            // Remove from old key bucket
            if let Some(old_key) = (entry.extractor)(old_raw) {
                if let Some(ids) = entry.keys.get_mut(&old_key) {
                    ids.retain(|&eid| eid != id);
                    if ids.is_empty() {
                        entry.keys.remove(&old_key);
                    }
                }
            }
            // Add to new key bucket
            if let Some(new_key) = (entry.extractor)(new_raw) {
                entry.keys.entry(new_key).or_default().push(id);
            }
        }
    }
}
