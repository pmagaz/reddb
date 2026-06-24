use serde::{Deserialize, Serialize};

/// The operation recorded in a WAL entry.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) enum WalOp {
    Insert,
    Update,
    Delete,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wal_op_copy_and_eq() {
        let op = WalOp::Insert;
        let copy = op; // Copy
        assert_eq!(op, copy);
        assert_ne!(WalOp::Insert, WalOp::Delete);
        assert_ne!(WalOp::Update, WalOp::Delete);
    }

    #[test]
    fn all_variants_are_distinct() {
        assert_ne!(WalOp::Insert, WalOp::Update);
        assert_ne!(WalOp::Insert, WalOp::Delete);
        assert_ne!(WalOp::Update, WalOp::Delete);
    }
}
