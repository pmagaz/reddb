use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
pub use uuid::Uuid;

mod config;
mod document;
mod error;
mod index;
#[cfg(feature = "migrate")]
pub mod migrate;
mod query;
pub mod serializer;
mod storage;
mod transaction;
mod update;
mod wal;

pub use config::{DbConfig, WriteOrder};
pub use document::Document;
use error::{RedDbError, Result};
use index::IndexRegistry;
pub use query::QueryBuilder;
use serde::{Deserialize, Serialize};
use serializer::Serializer;
pub use storage::FileStorage;
pub use storage::MemStorage;
use storage::Storage;
pub use transaction::Transaction;
pub use update::UpdateWhereBuilder;
use wal::WalOp;

type RedDbHM = HashMap<Uuid, Vec<u8>>;

#[cfg(feature = "bin_ser")]
pub type BinDb = RedDb<serializer::Bin, FileStorage<serializer::Bin>>;
#[cfg(feature = "json_ser")]
pub type JsonDb = RedDb<serializer::Json, FileStorage<serializer::Json>>;
#[cfg(feature = "yaml_ser")]
pub type YamlDb = RedDb<serializer::Yaml, FileStorage<serializer::Yaml>>;
#[cfg(feature = "ron_ser")]
pub type RonDb = RedDb<serializer::Ron, FileStorage<serializer::Ron>>;

/// All-in-memory database with no file persistence. Uses the Bin serializer
/// for the internal byte representation; the format does not affect behaviour.
#[cfg(feature = "bin_ser")]
pub type MemDb = RedDb<serializer::Bin, MemStorage>;

/// Snapshot of storage metrics returned by [`RedDb::stats`].
#[derive(Debug, Clone)]
pub struct StorageStats {
    /// Current on-disk file size in bytes (0 for [`MemStorage`]).
    pub file_size_bytes: u64,
    /// Number of live documents in the in-memory store.
    pub live_document_count: usize,
    /// Configured compaction ratio (compact when file ≥ live × ratio).
    pub compaction_ratio: f64,
}

pub struct RedDb<SE, ST> {
    storage: ST,
    serializer: SE,
    data: Arc<RwLock<RedDbHM>>,
    pub(crate) write_order: WriteOrder,
    compaction_ratio: f64,
    pub(crate) indexes: Arc<RwLock<IndexRegistry>>,
    has_indexes: AtomicBool,
}

impl<SE: Debug, ST: Debug> Debug for RedDb<SE, ST> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedDb")
            .field("storage", &self.storage)
            .field("serializer", &self.serializer)
            .field("write_order", &self.write_order)
            .field("compaction_ratio", &self.compaction_ratio)
            .finish_non_exhaustive()
    }
}

#[allow(private_bounds)]
impl<SE, ST: 'static> RedDb<SE, ST>
where
    SE: Serializer + Debug,
    for<'de> ST: Storage + Debug + Send + Sync,
{
    /// Open or create a database using a [`DbConfig`].
    pub async fn open<T>(config: DbConfig) -> Result<Self>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq + Send + Sync,
    {
        let stem = config.file_stem().to_string_lossy().into_owned();
        let storage = ST::new(&stem, config.compaction_ratio).await?;
        let data = storage.load::<T>().await?;
        Ok(Self {
            storage,
            data: Arc::new(RwLock::new(data)),
            serializer: SE::default(),
            write_order: config.write_order,
            compaction_ratio: config.compaction_ratio,
            indexes: Arc::new(RwLock::new(IndexRegistry::new())),
            has_indexes: AtomicBool::new(false),
        })
    }

    /// Convenience constructor — equivalent to `open(DbConfig::new(name)).await`.
    pub async fn new<T>(db_name: &str) -> Result<Self>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq + Send + Sync,
    {
        Self::open::<T>(DbConfig::new(db_name)).await
    }

    // ── lock helpers ──────────────────────────────────────────────────────────

    pub(crate) async fn read_lock(&self) -> Result<RwLockReadGuard<'_, RedDbHM>> {
        Ok(self.data.read().await)
    }

    pub(crate) async fn write_lock(&self) -> Result<RwLockWriteGuard<'_, RedDbHM>> {
        Ok(self.data.write().await)
    }

    // ── serialization helpers ─────────────────────────────────────────────────

    pub(crate) fn deserialize_raw<T>(&self, raw: &[u8]) -> Result<T>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
    {
        self.deserialize(raw)
    }

    pub(crate) fn serialize_raw<T>(&self, value: &T) -> Result<Vec<u8>>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
    {
        self.serialize(value)
    }

    // ── storage helpers ───────────────────────────────────────────────────────

    pub(crate) async fn storage_persist<T>(&self, docs: &[Document<T>], op: WalOp) -> Result<()>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + Clone + Send + Sync,
    {
        self.storage.persist(docs, op).await
    }

    pub(crate) async fn storage_persist_raw(&self, ops: &[(WalOp, Uuid, Vec<u8>)]) -> Result<()> {
        self.storage.persist_raw(ops).await
    }

    // ── index helpers ─────────────────────────────────────────────────────────

    pub(crate) async fn index_on_insert(&self, id: Uuid, raw: &[u8]) {
        if !self.has_indexes.load(Ordering::Acquire) {
            return;
        }
        self.indexes.write().await.on_insert(id, raw);
    }

    pub(crate) async fn index_on_delete(&self, id: Uuid, raw: &[u8]) {
        if !self.has_indexes.load(Ordering::Acquire) {
            return;
        }
        self.indexes.write().await.on_delete(id, raw);
    }

    pub(crate) async fn index_on_update(&self, id: Uuid, old_raw: &[u8], new_raw: &[u8]) {
        if !self.has_indexes.load(Ordering::Acquire) {
            return;
        }
        self.indexes.write().await.on_update(id, old_raw, new_raw);
    }

    // ── public API ────────────────────────────────────────────────────────────

    /// Return a [`QueryBuilder`] for closure-based queries over this database.
    pub fn query<T>(&self) -> QueryBuilder<'_, T, SE, ST>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
    {
        QueryBuilder::new(self)
    }

    /// Return an [`UpdateWhereBuilder`] targeting documents that satisfy `predicate`.
    pub fn update_where<T, F>(&self, predicate: F) -> UpdateWhereBuilder<'_, T, F, SE, ST>
    where
        F: Fn(&T) -> bool + Send + Sync + 'static,
        for<'de> T: Serialize + Deserialize<'de> + Debug + Clone + PartialEq + Send + Sync,
    {
        UpdateWhereBuilder::new(self, predicate)
    }

    /// Start a new [`Transaction`]. Stage operations on the returned value, then
    /// call `commit()` to apply them atomically or `rollback()` to discard them.
    pub fn begin(&self) -> Transaction<'_, SE, ST> {
        Transaction::new(self)
    }

    /// Register a hash index named `name` over the string key extracted by `key_fn`.
    ///
    /// Scans all existing documents to build the initial index. After this call,
    /// every write operation keeps the index up to date automatically.
    pub async fn add_index<T, F>(&self, name: impl Into<String>, key_fn: F) -> Result<()>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
        F: Fn(&T) -> String + Send + Sync + 'static,
    {
        use index::IndexEntry;

        let extractor: index::ExtractorFn = Box::new(move |raw| {
            let ser = SE::default();
            ser.deserialize::<T>(raw).ok().map(|v| key_fn(&v))
        });

        let initial_keys = {
            let data = self.read_lock().await?;
            let mut keys = std::collections::HashMap::new();
            for (id, raw) in data.iter() {
                if let Some(key) = extractor(raw) {
                    keys.entry(key).or_insert_with(Vec::new).push(*id);
                }
            }
            keys
        };

        self.indexes.write().await.entries.insert(
            name.into(),
            IndexEntry {
                extractor,
                keys: initial_keys,
            },
        );
        self.has_indexes.store(true, Ordering::Release);
        Ok(())
    }

    /// Look up documents by the value of a registered index.
    ///
    /// Returns an error if no index with the given name exists.
    pub async fn using_index<T>(&self, index_name: &str, key: &str) -> Result<Vec<Document<T>>>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
    {
        let ids: Vec<Uuid> = {
            let reg = self.indexes.read().await;
            let entry = reg
                .entries
                .get(index_name)
                .ok_or_else(|| RedDbError::IndexNotFound(index_name.to_string()))?;
            entry.keys.get(key).cloned().unwrap_or_default()
        };

        let data = self.read_lock().await?;
        let docs = ids
            .iter()
            .filter_map(|id| {
                data.get(id).and_then(|raw| {
                    self.deserialize::<T>(raw)
                        .ok()
                        .map(|v| Document::new(*id, v))
                })
            })
            .collect();
        Ok(docs)
    }

    /// Compact the backing store, rewriting it with exactly one Insert record
    /// per live document. No-op for [`MemStorage`].
    pub async fn compact(&self) -> Result<()> {
        let data = self.data.read().await;
        self.storage.compact(&*data).await
    }

    /// Return a snapshot of storage statistics.
    pub async fn stats(&self) -> Result<StorageStats> {
        let live_document_count = self.data.read().await.len();
        let file_size_bytes = self.storage.file_size().await?;
        Ok(StorageStats {
            file_size_bytes,
            live_document_count,
            compaction_ratio: self.compaction_ratio,
        })
    }

    // ── write operations ──────────────────────────────────────────────────────

    /// Delete all documents whose data satisfies `predicate`.
    ///
    /// Returns the count of deleted documents.
    pub async fn delete_where<T, F>(&self, predicate: F) -> Result<usize>
    where
        F: Fn(&T) -> bool + Send + Sync,
        for<'de> T: Serialize + Deserialize<'de> + Debug + Clone + PartialEq + Send + Sync,
    {
        if self.write_order == WriteOrder::FileFirst {
            let mut data = self.write_lock().await?;
            let matches: Vec<(Uuid, Vec<u8>, T)> = data
                .iter()
                .filter_map(|(id, raw)| {
                    self.deserialize::<T>(raw)
                        .ok()
                        .filter(|v| predicate(v))
                        .map(|v| (*id, raw.clone(), v))
                })
                .collect();
            if matches.is_empty() {
                return Ok(0);
            }
            let docs: Vec<Document<T>> = matches
                .iter()
                .map(|(id, _, v)| Document::new(*id, v.clone()))
                .collect();
            self.storage.persist(&docs, WalOp::Delete).await?;
            for (id, _, _) in &matches {
                data.remove(id);
            }
            let count = matches.len();
            drop(data);
            for (id, raw, _) in &matches {
                self.index_on_delete(*id, raw).await;
            }
            return Ok(count);
        }

        // MemoryFirst path
        let deleted: Vec<(Uuid, Vec<u8>, Document<T>)> = {
            let mut data = self.write_lock().await?;
            let matches: Vec<(Uuid, Vec<u8>)> = data
                .iter()
                .filter_map(|(id, raw)| {
                    self.deserialize::<T>(raw)
                        .ok()
                        .filter(|v| predicate(v))
                        .map(|_| (*id, raw.clone()))
                })
                .collect();
            matches
                .into_iter()
                .filter_map(|(id, raw)| {
                    data.remove(&id).map(|removed| {
                        let v = self.deserialize::<T>(&removed).unwrap();
                        (id, raw, Document::new(id, v))
                    })
                })
                .collect()
        };

        let count = deleted.len();
        if count == 0 {
            return Ok(0);
        }
        let docs: Vec<Document<T>> = deleted.iter().map(|(_, _, d)| d.clone()).collect();
        self.storage.persist(&docs, WalOp::Delete).await?;
        for (id, raw, _) in &deleted {
            self.index_on_delete(*id, raw).await;
        }
        Ok(count)
    }

    async fn find_uuids<T>(&self, search: &T) -> Result<Vec<Uuid>>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
    {
        let data = self.read_lock().await?;
        let serialized = self.serialize(search)?;
        Ok(data
            .iter()
            .filter(|(_, value)| **value == serialized)
            .map(|(id, _)| *id)
            .collect())
    }

    pub async fn insert_one<T>(&self, value: T) -> Result<Document<T>>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + Clone + PartialEq + Send + Sync,
    {
        let id = Uuid::new_v4();
        let raw = self.serialize(&value)?;
        let doc = Document::new(id, value);

        if self.write_order == WriteOrder::FileFirst {
            self.storage.persist(&[doc.clone()], WalOp::Insert).await?;
        }
        self.write_lock().await?.insert(id, raw.clone());
        if self.write_order == WriteOrder::MemoryFirst {
            self.storage.persist(&[doc.clone()], WalOp::Insert).await?;
        }
        self.index_on_insert(id, &raw).await;
        Ok(doc)
    }

    pub async fn insert<T>(&self, values: Vec<T>) -> Result<Vec<Document<T>>>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + Clone + PartialEq + Send + Sync,
    {
        let prepared: Vec<(Uuid, Vec<u8>, T)> = values
            .into_iter()
            .map(|v| -> Result<(Uuid, Vec<u8>, T)> {
                let id = Uuid::new_v4();
                let raw = self.serialize(&v)?;
                Ok((id, raw, v))
            })
            .collect::<Result<_>>()?;

        let docs: Vec<Document<T>> = prepared
            .iter()
            .map(|(id, _, v)| Document::new(*id, v.clone()))
            .collect();

        if self.write_order == WriteOrder::FileFirst {
            self.storage.persist(&docs, WalOp::Insert).await?;
        }
        {
            let mut data = self.write_lock().await?;
            for (id, raw, _) in &prepared {
                data.insert(*id, raw.clone());
            }
        }
        if self.write_order == WriteOrder::MemoryFirst {
            self.storage.persist(&docs, WalOp::Insert).await?;
        }
        for (id, raw, _) in &prepared {
            self.index_on_insert(*id, raw).await;
        }
        Ok(docs)
    }

    pub async fn get<T>(&self, id: &Uuid) -> Result<Option<Document<T>>>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
    {
        let data = self.read_lock().await?;
        match data.get(id) {
            Some(raw) => Ok(Some(Document::new(*id, self.deserialize(raw)?))),
            None => Ok(None),
        }
    }

    /// Find by id — returns error if not found.
    pub async fn find_one<T>(&self, id: &Uuid) -> Result<Document<T>>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
    {
        self.get(id).await?.ok_or(RedDbError::NotFound(*id))
    }

    pub async fn update_one<T>(&self, id: &Uuid, new_value: T) -> Result<bool>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + Clone + PartialEq + Send + Sync,
    {
        let new_raw = self.serialize(&new_value)?;
        let doc = Document::new(*id, new_value);

        if self.write_order == WriteOrder::FileFirst {
            let mut data = self.write_lock().await?;
            if let Some(entry) = data.get_mut(id) {
                let old_raw = entry.clone();
                self.storage.persist(&[doc.clone()], WalOp::Update).await?;
                *entry = new_raw.clone();
                drop(data);
                self.index_on_update(*id, &old_raw, &new_raw).await;
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            let old_raw = {
                let mut data = self.write_lock().await?;
                if let Some(entry) = data.get_mut(id) {
                    let old = entry.clone();
                    *entry = new_raw.clone();
                    Some(old)
                } else {
                    None
                }
            };
            if let Some(old) = old_raw {
                self.storage.persist(&[doc], WalOp::Update).await?;
                self.index_on_update(*id, &old, &new_raw).await;
                Ok(true)
            } else {
                Ok(false)
            }
        }
    }

    pub async fn delete_one<T>(&self, id: &Uuid) -> Result<Document<T>>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + Clone + PartialEq + Send + Sync,
    {
        if self.write_order == WriteOrder::FileFirst {
            let mut data = self.write_lock().await?;
            let raw = data.get(id).ok_or(RedDbError::NotFound(*id))?.clone();
            let value: T = self.deserialize(&raw)?;
            let doc = Document::new(*id, value);
            self.storage.persist(&[doc.clone()], WalOp::Delete).await?;
            data.remove(id);
            drop(data);
            self.index_on_delete(*id, &raw).await;
            Ok(doc)
        } else {
            let (doc, raw) = {
                let mut data = self.write_lock().await?;
                let raw = data.remove(id).ok_or(RedDbError::NotFound(*id))?;
                let value = self.deserialize(&raw)?;
                (Document::new(*id, value), raw)
            };
            self.storage.persist(&[doc.clone()], WalOp::Delete).await?;
            self.index_on_delete(*id, &raw).await;
            Ok(doc)
        }
    }

    pub async fn find_all<T>(&self) -> Result<Vec<Document<T>>>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
    {
        let data = self.read_lock().await?;
        Ok(data
            .iter()
            .map(|(id, raw)| Document::new(*id, self.deserialize(raw).unwrap()))
            .collect())
    }

    pub async fn find<T>(&self, search: &T) -> Result<Vec<Document<T>>>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
    {
        let data = self.read_lock().await?;
        let serialized = self.serialize(search)?;
        Ok(data
            .iter()
            .filter(|(_, raw)| **raw == serialized)
            .map(|(id, raw)| Document::new(*id, self.deserialize(raw).unwrap()))
            .collect())
    }

    pub async fn update<T>(&self, search: &T, new_value: &T) -> Result<usize>
    where
        for<'de> T: Serialize + Deserialize<'de> + Clone + Debug + PartialEq + Send + Sync,
    {
        let serialized_search = self.serialize(search)?;
        let new_raw = self.serialize(new_value)?;

        if self.write_order == WriteOrder::FileFirst {
            let mut data = self.write_lock().await?;
            let matching_ids: Vec<Uuid> = data
                .iter()
                .filter(|(_, raw)| **raw == serialized_search)
                .map(|(id, _)| *id)
                .collect();
            if matching_ids.is_empty() {
                return Ok(0);
            }
            let docs: Vec<Document<T>> = matching_ids
                .iter()
                .map(|id| Document::new(*id, new_value.clone()))
                .collect();
            self.storage.persist(&docs, WalOp::Update).await?;
            for id in &matching_ids {
                if let Some(entry) = data.get_mut(id) {
                    *entry = new_raw.clone();
                }
            }
            drop(data);
            for id in &matching_ids {
                self.index_on_update(*id, &serialized_search, &new_raw)
                    .await;
            }
            Ok(matching_ids.len())
        } else {
            let updated: Vec<(Uuid, Vec<u8>)> = {
                let mut data = self.write_lock().await?;
                data.iter_mut()
                    .filter(|(_, raw)| **raw == serialized_search)
                    .map(|(id, raw)| {
                        let old = raw.clone();
                        *raw = new_raw.clone();
                        (*id, old)
                    })
                    .collect()
            };
            let count = updated.len();
            if count > 0 {
                let docs: Vec<Document<T>> = updated
                    .iter()
                    .map(|(id, _)| Document::new(*id, new_value.clone()))
                    .collect();
                self.storage.persist(&docs, WalOp::Update).await?;
                for (id, old_raw) in &updated {
                    self.index_on_update(*id, old_raw, &new_raw).await;
                }
            }
            Ok(count)
        }
    }

    pub async fn delete<T>(&self, search: &T) -> Result<usize>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + Clone + PartialEq + Send + Sync,
    {
        let uuids = self.find_uuids(search).await?;
        if uuids.is_empty() {
            return Ok(0);
        }

        if self.write_order == WriteOrder::FileFirst {
            let mut data = self.write_lock().await?;
            let (docs, raws): (Vec<Document<T>>, Vec<Vec<u8>>) = uuids
                .iter()
                .filter_map(|id| {
                    data.get(id).and_then(|raw| {
                        self.deserialize::<T>(raw)
                            .ok()
                            .map(|v| (Document::new(*id, v), raw.clone()))
                    })
                })
                .unzip();
            if docs.is_empty() {
                return Ok(0);
            }
            self.storage.persist(&docs, WalOp::Delete).await?;
            for id in &uuids {
                data.remove(id);
            }
            drop(data);
            for (id, raw) in uuids.iter().zip(&raws) {
                self.index_on_delete(*id, raw).await;
            }
            Ok(docs.len())
        } else {
            let (docs, raws): (Vec<Document<T>>, Vec<Vec<u8>>) = {
                let mut data = self.write_lock().await?;
                uuids
                    .into_iter()
                    .filter_map(|id| {
                        data.remove(&id).and_then(|raw| {
                            self.deserialize::<T>(&raw)
                                .ok()
                                .map(|v| (Document::new(id, v), raw))
                        })
                    })
                    .unzip()
            };
            if !docs.is_empty() {
                self.storage.persist(&docs, WalOp::Delete).await?;
                for (id, raw) in docs.iter().map(|d| d.id).zip(&raws) {
                    self.index_on_delete(id, raw).await;
                }
            }
            Ok(docs.len())
        }
    }

    // ── serialization (private) ───────────────────────────────────────────────

    fn serialize<T>(&self, value: &T) -> Result<Vec<u8>>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
    {
        self.serializer
            .serialize(value)
            .map_err(|e| RedDbError::Serialize(e.to_string()))
    }

    fn deserialize<T>(&self, value: &[u8]) -> Result<T>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
    {
        self.serializer
            .deserialize(value)
            .map_err(|e| RedDbError::Deserialize(e.to_string()))
    }
}

// ── unit tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
#[cfg(feature = "bin_ser")]
mod delete_where_tests {
    use super::*;
    use crate::MemDb;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct Item {
        tag: String,
    }

    #[tokio::test]
    async fn deletes_matching_documents() {
        let db = MemDb::new::<Item>("_").await.unwrap();
        db.insert(vec![
            Item {
                tag: "remove".into(),
            },
            Item {
                tag: "remove".into(),
            },
            Item { tag: "keep".into() },
        ])
        .await
        .unwrap();

        let n = db
            .delete_where::<Item, _>(|i| i.tag == "remove")
            .await
            .unwrap();
        assert_eq!(n, 2);

        let all = db.find_all::<Item>().await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].data.tag, "keep");
    }

    #[tokio::test]
    async fn returns_zero_when_no_match() {
        let db = MemDb::new::<Item>("_").await.unwrap();
        db.insert_one(Item { tag: "keep".into() }).await.unwrap();
        let n = db
            .delete_where::<Item, _>(|i| i.tag == "gone")
            .await
            .unwrap();
        assert_eq!(n, 0);
        assert_eq!(db.find_all::<Item>().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn delete_all_when_predicate_always_true() {
        let db = MemDb::new::<Item>("_").await.unwrap();
        db.insert(vec![Item { tag: "a".into() }, Item { tag: "b".into() }])
            .await
            .unwrap();
        let n = db.delete_where::<Item, _>(|_| true).await.unwrap();
        assert_eq!(n, 2);
        assert!(db.find_all::<Item>().await.unwrap().is_empty());
    }
}

#[cfg(test)]
#[cfg(feature = "bin_ser")]
mod compact_stats_tests {
    use super::*;
    use crate::MemDb;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct Item {
        v: u32,
    }

    #[tokio::test]
    async fn compact_is_noop_for_memdb() {
        let db = MemDb::new::<Item>("_").await.unwrap();
        db.insert(vec![Item { v: 1 }, Item { v: 2 }]).await.unwrap();
        assert!(db.compact().await.is_ok());
        assert_eq!(db.find_all::<Item>().await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn stats_reflect_live_count_and_zero_size() {
        let db = MemDb::new::<Item>("_").await.unwrap();
        let s0 = db.stats().await.unwrap();
        assert_eq!(s0.live_document_count, 0);
        assert_eq!(s0.file_size_bytes, 0);
        assert_eq!(s0.compaction_ratio, 2.0);

        db.insert(vec![Item { v: 1 }, Item { v: 2 }]).await.unwrap();
        let s1 = db.stats().await.unwrap();
        assert_eq!(s1.live_document_count, 2);
        assert_eq!(s1.file_size_bytes, 0);
    }

    #[tokio::test]
    async fn stats_custom_compaction_ratio() {
        let db: MemDb = RedDb::open::<Item>(DbConfig::new("_").compaction_ratio(5.0))
            .await
            .unwrap();
        assert_eq!(db.stats().await.unwrap().compaction_ratio, 5.0);
    }
}

#[cfg(test)]
#[cfg(feature = "bin_ser")]
mod write_order_tests {
    use super::*;
    use crate::MemDb;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct Item {
        v: u32,
    }

    async fn file_first_db() -> MemDb {
        RedDb::open::<Item>(DbConfig::new("_").write_order(WriteOrder::FileFirst))
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn file_first_insert_one_round_trips() {
        let db = file_first_db().await;
        let doc = db.insert_one(Item { v: 42 }).await.unwrap();
        let found: Document<Item> = db.find_one(&doc.id).await.unwrap();
        assert_eq!(found.data.v, 42);
    }

    #[tokio::test]
    async fn file_first_insert_many_round_trips() {
        let db = file_first_db().await;
        let docs = db.insert(vec![Item { v: 1 }, Item { v: 2 }]).await.unwrap();
        assert_eq!(db.find_all::<Item>().await.unwrap().len(), 2);
        assert!(db.get::<Item>(&docs[0].id).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn file_first_update_one_round_trips() {
        let db = file_first_db().await;
        let doc = db.insert_one(Item { v: 1 }).await.unwrap();
        assert!(db.update_one(&doc.id, Item { v: 99 }).await.unwrap());
        assert_eq!(db.find_one::<Item>(&doc.id).await.unwrap().data.v, 99);
    }

    #[tokio::test]
    async fn file_first_update_returns_false_for_missing() {
        let db = file_first_db().await;
        let result = db
            .update_one::<Item>(&Uuid::new_v4(), Item { v: 1 })
            .await
            .unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn file_first_delete_one_round_trips() {
        let db = file_first_db().await;
        let doc = db.insert_one(Item { v: 5 }).await.unwrap();
        let deleted: Document<Item> = db.delete_one(&doc.id).await.unwrap();
        assert_eq!(deleted.id, doc.id);
        assert!(db.get::<Item>(&doc.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn file_first_delete_where_round_trips() {
        let db = file_first_db().await;
        db.insert(vec![Item { v: 1 }, Item { v: 2 }, Item { v: 1 }])
            .await
            .unwrap();
        let n = db.delete_where::<Item, _>(|i| i.v == 1).await.unwrap();
        assert_eq!(n, 2);
        assert_eq!(db.find_all::<Item>().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn file_first_update_bulk_round_trips() {
        let db = file_first_db().await;
        db.insert(vec![Item { v: 1 }, Item { v: 1 }, Item { v: 2 }])
            .await
            .unwrap();
        let n = db.update(&Item { v: 1 }, &Item { v: 99 }).await.unwrap();
        assert_eq!(n, 2);
        assert_eq!(db.find(&Item { v: 99 }).await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn file_first_delete_bulk_round_trips() {
        let db = file_first_db().await;
        db.insert(vec![Item { v: 1 }, Item { v: 1 }, Item { v: 2 }])
            .await
            .unwrap();
        let n = db.delete(&Item { v: 1 }).await.unwrap();
        assert_eq!(n, 2);
        assert_eq!(db.find_all::<Item>().await.unwrap().len(), 1);
    }
}

#[cfg(test)]
#[cfg(feature = "bin_ser")]
mod index_tests {
    use super::*;
    use crate::MemDb;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct User {
        name: String,
        role: String,
    }

    fn user(name: &str, role: &str) -> User {
        User {
            name: name.into(),
            role: role.into(),
        }
    }

    async fn seeded() -> MemDb {
        let db = MemDb::new::<User>("_").await.unwrap();
        db.insert(vec![
            user("alice", "admin"),
            user("bob", "user"),
            user("carol", "admin"),
        ])
        .await
        .unwrap();
        db
    }

    #[tokio::test]
    async fn add_index_and_lookup() {
        let db = seeded().await;
        db.add_index::<User, _>("by_role", |u| u.role.clone())
            .await
            .unwrap();

        let admins = db.using_index::<User>("by_role", "admin").await.unwrap();
        assert_eq!(admins.len(), 2);
        let names: Vec<&str> = admins.iter().map(|d| d.data.name.as_str()).collect();
        assert!(names.contains(&"alice"));
        assert!(names.contains(&"carol"));

        let users = db.using_index::<User>("by_role", "user").await.unwrap();
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].data.name, "bob");
    }

    #[tokio::test]
    async fn index_updated_on_insert() {
        let db = seeded().await;
        db.add_index::<User, _>("by_role", |u| u.role.clone())
            .await
            .unwrap();

        db.insert_one(user("dave", "admin")).await.unwrap();

        let admins = db.using_index::<User>("by_role", "admin").await.unwrap();
        assert_eq!(admins.len(), 3);
    }

    #[tokio::test]
    async fn index_updated_on_delete() {
        let db = seeded().await;
        db.add_index::<User, _>("by_role", |u| u.role.clone())
            .await
            .unwrap();

        let alice = db
            .using_index::<User>("by_role", "admin")
            .await
            .unwrap()
            .into_iter()
            .find(|d| d.data.name == "alice")
            .unwrap();
        db.delete_one::<User>(&alice.id).await.unwrap();

        let admins = db.using_index::<User>("by_role", "admin").await.unwrap();
        assert_eq!(admins.len(), 1);
        assert_eq!(admins[0].data.name, "carol");
    }

    #[tokio::test]
    async fn index_updated_on_update_one() {
        let db = seeded().await;
        db.add_index::<User, _>("by_role", |u| u.role.clone())
            .await
            .unwrap();

        let bob = db
            .using_index::<User>("by_role", "user")
            .await
            .unwrap()
            .pop()
            .unwrap();
        db.update_one(&bob.id, user("bob", "admin")).await.unwrap();

        let admins = db.using_index::<User>("by_role", "admin").await.unwrap();
        assert_eq!(admins.len(), 3);
        let users = db.using_index::<User>("by_role", "user").await.unwrap();
        assert!(users.is_empty());
    }

    #[tokio::test]
    async fn unknown_index_returns_error() {
        let db = MemDb::new::<User>("_").await.unwrap();
        let result = db.using_index::<User>("no_such_index", "x").await;
        assert!(matches!(result, Err(RedDbError::IndexNotFound(_))));
    }

    #[tokio::test]
    async fn index_key_with_no_matches_returns_empty() {
        let db = seeded().await;
        db.add_index::<User, _>("by_role", |u| u.role.clone())
            .await
            .unwrap();
        let result = db
            .using_index::<User>("by_role", "superuser")
            .await
            .unwrap();
        assert!(result.is_empty());
    }
}

#[cfg(test)]
#[cfg(feature = "ron_ser")]
mod tests {
    use super::*;
    use crate::RonDb;
    use std::fs;

    #[derive(Clone, Debug, Serialize, PartialEq, Deserialize)]
    struct TestStruct {
        foo: String,
    }

    #[tokio::test]
    async fn find_uuids() {
        let db = RonDb::new::<TestStruct>(".test2.db").await.unwrap();
        let doc = db
            .insert_one(TestStruct {
                foo: "test".to_owned(),
            })
            .await
            .unwrap();
        let doc2 = db
            .insert_one(TestStruct {
                foo: "test2".to_owned(),
            })
            .await
            .unwrap();
        let doc3 = db
            .insert_one(TestStruct {
                foo: "test".to_owned(),
            })
            .await
            .unwrap();

        let uuids: Vec<Uuid> = db
            .find_uuids(&TestStruct {
                foo: "test".to_owned(),
            })
            .await
            .unwrap();

        assert!(uuids.contains(&doc.id));
        assert!(!uuids.contains(&doc2.id));
        assert!(uuids.contains(&doc3.id));

        fs::remove_file(".test2.db.ron").unwrap();
    }

    #[tokio::test]
    async fn insert_and_find_one() {
        let db = RonDb::new::<TestStruct>(".insert_and_find_one.db")
            .await
            .unwrap();
        let doc = db
            .insert_one(TestStruct {
                foo: "test".to_owned(),
            })
            .await
            .unwrap();
        let find: Document<TestStruct> = db.find_one(&doc.id).await.unwrap();
        assert_eq!(find.id, doc.id);
        assert_eq!(find.data, doc.data);
        fs::remove_file(".insert_and_find_one.db.ron").unwrap();
    }

    #[tokio::test]
    async fn get_returns_some_for_existing() {
        let db = RonDb::new::<TestStruct>(".get_existing.db").await.unwrap();
        let doc = db
            .insert_one(TestStruct {
                foo: "hello".to_owned(),
            })
            .await
            .unwrap();
        let found: Option<Document<TestStruct>> = db.get(&doc.id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().data.foo, "hello");
        fs::remove_file(".get_existing.db.ron").unwrap();
    }

    #[tokio::test]
    async fn get_returns_none_for_missing() {
        let db = RonDb::new::<TestStruct>(".get_missing.db").await.unwrap();
        let result: Option<Document<TestStruct>> = db.get(&Uuid::new_v4()).await.unwrap();
        assert!(result.is_none());
        fs::remove_file(".get_missing.db.ron").unwrap();
    }

    #[tokio::test]
    async fn find() {
        let db = RonDb::new::<TestStruct>(".find.db").await.unwrap();
        let one = TestStruct {
            foo: String::from("one"),
        };
        let two = TestStruct {
            foo: String::from("two"),
        };
        db.insert(vec![one.clone(), one.clone(), two.clone()])
            .await
            .unwrap();
        let result = db.find(&one).await.unwrap();
        assert_eq!(result.len(), 2);
        fs::remove_file(".find.db.ron").unwrap();
    }

    #[tokio::test]
    async fn update_one() {
        let db = RonDb::new::<TestStruct>(".update_one.db").await.unwrap();
        let original = TestStruct {
            foo: "hi".to_owned(),
        };
        let updated = TestStruct {
            foo: "bye".to_owned(),
        };
        let doc = db.insert_one(original.clone()).await.unwrap();
        db.update_one(&doc.id, updated.clone()).await.unwrap();
        let result: Document<TestStruct> = db.find_one(&doc.id).await.unwrap();
        assert_eq!(result.data, updated);
        fs::remove_file(".update_one.db.ron").unwrap();
    }

    #[tokio::test]
    async fn update() {
        let db = RonDb::new::<TestStruct>(".update.db").await.unwrap();
        let one = TestStruct {
            foo: String::from("one"),
        };
        let two = TestStruct {
            foo: String::from("two"),
        };
        db.insert(vec![one.clone(), one.clone(), two.clone()])
            .await
            .unwrap();
        let updated = db.update(&one, &two).await.unwrap();
        assert_eq!(updated, 2);
        let result = db.find(&two).await.unwrap();
        assert_eq!(result.len(), 3);
        fs::remove_file(".update.db.ron").unwrap();
    }

    #[tokio::test]
    async fn delete_one_removes_document() {
        let db = RonDb::new::<TestStruct>(".delete_one.db").await.unwrap();
        let doc = db
            .insert_one(TestStruct {
                foo: "test".to_owned(),
            })
            .await
            .unwrap();
        let deleted: Document<TestStruct> = db.delete_one(&doc.id).await.unwrap();
        assert_eq!(deleted.id, doc.id);
        assert_eq!(deleted.data, doc.data);
        let after: Option<Document<TestStruct>> = db.get(&doc.id).await.unwrap();
        assert!(after.is_none());
        fs::remove_file(".delete_one.db.ron").unwrap();
    }

    #[tokio::test]
    async fn delete() {
        let db = RonDb::new::<TestStruct>(".delete.db").await.unwrap();
        let one = TestStruct {
            foo: "one".to_owned(),
        };
        let two = TestStruct {
            foo: "two".to_owned(),
        };
        db.insert(vec![one.clone(), one.clone(), two.clone()])
            .await
            .unwrap();
        let deleted = db.delete(&one).await.unwrap();
        assert_eq!(deleted, 2);
        let not_deleted = db.delete(&one).await.unwrap();
        assert_eq!(not_deleted, 0);
        fs::remove_file(".delete.db.ron").unwrap();
    }

    #[tokio::test]
    async fn serialie_deserialize() {
        let db = RonDb::new::<TestStruct>(".serialize.db").await.unwrap();
        let test = TestStruct {
            foo: "one".to_owned(),
        };
        let serialized = db.serializer.serialize(&test).unwrap();
        let deserialized: TestStruct = db.serializer.deserialize(&serialized).unwrap();
        assert_eq!(deserialized, test);
        fs::remove_file(".serialize.db.ron").unwrap();
    }
}
