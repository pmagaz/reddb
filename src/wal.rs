use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The operation recorded in a WAL entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum WalOp {
    Insert,
    Update,
    Delete,
}

/// One record in the write-ahead log.
/// `payload` holds the serialized user value (`T`); it is empty for Delete.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct WalEntry {
    pub op: WalOp,
    pub id: Uuid,
    /// Serialized `T` bytes. Empty for `WalOp::Delete`.
    pub payload: Vec<u8>,
}

impl WalEntry {
    pub fn insert(id: Uuid, payload: Vec<u8>) -> Self {
        WalEntry { op: WalOp::Insert, id, payload }
    }

    pub fn update(id: Uuid, payload: Vec<u8>) -> Self {
        WalEntry { op: WalOp::Update, id, payload }
    }

    pub fn delete(id: Uuid) -> Self {
        WalEntry { op: WalOp::Delete, id, payload: Vec::new() }
    }

    pub fn is_delete(&self) -> bool {
        self.op == WalOp::Delete
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_entry_has_correct_op_and_payload() {
        let id = Uuid::new_v4();
        let payload = vec![1, 2, 3];
        let entry = WalEntry::insert(id, payload.clone());
        assert_eq!(entry.op, WalOp::Insert);
        assert_eq!(entry.id, id);
        assert_eq!(entry.payload, payload);
        assert!(!entry.is_delete());
    }

    #[test]
    fn update_entry_has_correct_op() {
        let id = Uuid::new_v4();
        let entry = WalEntry::update(id, vec![9]);
        assert_eq!(entry.op, WalOp::Update);
        assert!(!entry.is_delete());
    }

    #[test]
    fn delete_entry_has_empty_payload() {
        let id = Uuid::new_v4();
        let entry = WalEntry::delete(id);
        assert_eq!(entry.op, WalOp::Delete);
        assert!(entry.payload.is_empty());
        assert!(entry.is_delete());
    }

    #[test]
    fn wal_op_clone_and_eq() {
        assert_eq!(WalOp::Insert.clone(), WalOp::Insert);
        assert_ne!(WalOp::Insert, WalOp::Delete);
    }
}
