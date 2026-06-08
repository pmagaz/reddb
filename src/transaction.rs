use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use crate::document::Document;
use crate::error::Result;
use crate::serializer::Serializer;
use crate::storage::Storage;
use crate::wal::WalOp;
use crate::{RedDb, Uuid};

/// A buffered sequence of write operations applied atomically on [`commit`](Transaction::commit).
///
/// Obtain a transaction via [`RedDb::begin`]. Stage operations with
/// [`insert_one`](Transaction::insert_one), [`update_one`](Transaction::update_one), and
/// [`delete_one`](Transaction::delete_one). Finalise with `commit()` or discard with
/// [`rollback`](Transaction::rollback).
///
/// The live in-memory store is not modified until `commit` is called.
pub struct Transaction<'db, SE, ST> {
    db: &'db RedDb<SE, ST>,
    ops: Vec<(WalOp, Uuid, Vec<u8>)>,
}

impl<'db, SE, ST: 'static> Transaction<'db, SE, ST>
where
    SE: Serializer + Debug,
    for<'de> ST: Storage + Debug + Send + Sync,
{
    pub(crate) fn new(db: &'db RedDb<SE, ST>) -> Self {
        Self { db, ops: Vec::new() }
    }

    /// Stage an insert — assigns a UUID and serializes `value`.
    /// The document is not visible to other readers until `commit`.
    pub fn insert_one<T>(&mut self, value: T) -> Result<Document<T>>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + Clone + PartialEq,
    {
        let id = Uuid::new_v4();
        let raw = self.db.serialize_raw(&value)?;
        self.ops.push((WalOp::Insert, id, raw));
        Ok(Document::new(id, value))
    }

    /// Stage an update for an existing document.
    /// The change is not applied until `commit`.
    pub fn update_one<T>(&mut self, id: &Uuid, new_value: T) -> Result<()>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + Clone + PartialEq,
    {
        let raw = self.db.serialize_raw(&new_value)?;
        self.ops.push((WalOp::Update, *id, raw));
        Ok(())
    }

    /// Stage a delete.
    /// The document remains visible to other readers until `commit`.
    pub fn delete_one(&mut self, id: &Uuid) {
        self.ops.push((WalOp::Delete, *id, Vec::new()));
    }

    /// Apply all staged operations atomically: update the in-memory store,
    /// update any registered indexes, then append to the WAL.
    pub async fn commit(self) -> Result<()> {
        if self.ops.is_empty() {
            return Ok(());
        }

        // Collect index changes while holding the write lock, then apply after.
        let index_changes = {
            let mut data = self.db.write_lock().await?;

            let mut changes = Vec::with_capacity(self.ops.len());
            for (op, id, new_raw) in &self.ops {
                match op {
                    WalOp::Insert => {
                        data.insert(*id, new_raw.clone());
                        changes.push(IndexChange::Insert { id: *id, raw: new_raw.clone() });
                    }
                    WalOp::Update => {
                        let old_raw = data.get(id).cloned().unwrap_or_default();
                        data.insert(*id, new_raw.clone());
                        changes.push(IndexChange::Update {
                            id: *id,
                            old_raw,
                            new_raw: new_raw.clone(),
                        });
                    }
                    WalOp::Delete => {
                        let old_raw = data.remove(id).unwrap_or_default();
                        changes.push(IndexChange::Delete { id: *id, raw: old_raw });
                    }
                }
            }
            changes
        }; // write lock released

        // Update indexes outside the data lock to avoid deadlock.
        if !index_changes.is_empty() {
            let mut registry = self.db.indexes.write().await;
            for change in &index_changes {
                match change {
                    IndexChange::Insert { id, raw } => registry.on_insert(*id, raw),
                    IndexChange::Update { id, old_raw, new_raw } => {
                        registry.on_update(*id, old_raw, new_raw)
                    }
                    IndexChange::Delete { id, raw } => registry.on_delete(*id, raw),
                }
            }
        }

        // Persist raw ops to WAL.
        self.db.storage_persist_raw(&self.ops).await
    }

    /// Discard all staged operations. The live store is unchanged.
    pub fn rollback(self) {}
}

enum IndexChange {
    Insert { id: Uuid, raw: Vec<u8> },
    Update { id: Uuid, old_raw: Vec<u8>, new_raw: Vec<u8> },
    Delete { id: Uuid, raw: Vec<u8> },
}

#[cfg(test)]
#[cfg(feature = "bin_ser")]
mod tests {
    use super::*;
    use crate::MemDb;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct Item {
        v: u32,
    }

    #[tokio::test]
    async fn commit_applies_insert() {
        let db = MemDb::new::<Item>("_").await.unwrap();
        let mut tx = db.begin();
        let doc = tx.insert_one(Item { v: 1 }).unwrap();

        // Not yet visible
        assert!(db.get::<Item>(&doc.id).await.unwrap().is_none());

        tx.commit().await.unwrap();

        // Now visible
        let found: crate::Document<Item> = db.find_one(&doc.id).await.unwrap();
        assert_eq!(found.data.v, 1);
    }

    #[tokio::test]
    async fn commit_applies_update() {
        let db = MemDb::new::<Item>("_").await.unwrap();
        let inserted = db.insert_one(Item { v: 10 }).await.unwrap();

        let mut tx = db.begin();
        tx.update_one(&inserted.id, Item { v: 99 }).unwrap();
        tx.commit().await.unwrap();

        let found: crate::Document<Item> = db.find_one(&inserted.id).await.unwrap();
        assert_eq!(found.data.v, 99);
    }

    #[tokio::test]
    async fn commit_applies_delete() {
        let db = MemDb::new::<Item>("_").await.unwrap();
        let inserted = db.insert_one(Item { v: 5 }).await.unwrap();

        let mut tx = db.begin();
        tx.delete_one(&inserted.id);
        tx.commit().await.unwrap();

        assert!(db.get::<Item>(&inserted.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn rollback_discards_all_changes() {
        let db = MemDb::new::<Item>("_").await.unwrap();
        let inserted = db.insert_one(Item { v: 1 }).await.unwrap();

        let mut tx = db.begin();
        tx.insert_one(Item { v: 99 }).unwrap();
        tx.update_one(&inserted.id, Item { v: 42 }).unwrap();
        tx.rollback();

        // Original document unchanged, new document not inserted
        assert_eq!(db.find_all::<Item>().await.unwrap().len(), 1);
        let still_one: crate::Document<Item> = db.find_one(&inserted.id).await.unwrap();
        assert_eq!(still_one.data.v, 1);
    }

    #[tokio::test]
    async fn empty_commit_is_noop() {
        let db = MemDb::new::<Item>("_").await.unwrap();
        db.insert_one(Item { v: 7 }).await.unwrap();
        let tx = db.begin();
        tx.commit().await.unwrap();
        assert_eq!(db.find_all::<Item>().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn commit_is_atomic_after_mix_of_ops() {
        let db = MemDb::new::<Item>("_").await.unwrap();
        let d1 = db.insert_one(Item { v: 1 }).await.unwrap();
        let d2 = db.insert_one(Item { v: 2 }).await.unwrap();

        let mut tx = db.begin();
        tx.delete_one(&d1.id);
        tx.update_one(&d2.id, Item { v: 20 }).unwrap();
        let d3 = tx.insert_one(Item { v: 3 }).unwrap();
        tx.commit().await.unwrap();

        assert!(db.get::<Item>(&d1.id).await.unwrap().is_none());
        assert_eq!(db.find_one::<Item>(&d2.id).await.unwrap().data.v, 20);
        assert_eq!(db.find_one::<Item>(&d3.id).await.unwrap().data.v, 3);
        assert_eq!(db.find_all::<Item>().await.unwrap().len(), 2);
    }
}
