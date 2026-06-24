use async_trait::async_trait;
use core::fmt::Debug;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::Storage;
use crate::document::Document;
use crate::error::Result;
use crate::wal::WalOp;
use crate::RedDbHM;

/// No-persistence storage backend. All data lives in the in-memory store
/// inside `RedDb`; nothing is written to disk.
#[derive(Debug, Default)]
pub struct MemStorage;

#[async_trait]
impl Storage for MemStorage {
    async fn new(_db_name: &str, _compaction_ratio: f64) -> Result<Self> {
        Ok(MemStorage)
    }

    #[allow(clippy::extra_unused_type_parameters)]
    async fn load<T>(&self) -> Result<RedDbHM>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq + Send + Sync,
    {
        Ok(HashMap::new())
    }

    async fn persist<T>(&self, _data: &[Document<T>], _op: WalOp) -> Result<()>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + Sync + Clone,
    {
        Ok(())
    }

    async fn compact(&self, _data: &RedDbHM) -> Result<()> {
        Ok(())
    }

    async fn file_size(&self) -> Result<u64> {
        Ok(0)
    }

    async fn persist_raw(&self, _records: &[(WalOp, Uuid, Vec<u8>)]) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct S {
        x: u32,
    }

    #[tokio::test]
    async fn load_returns_empty_map() {
        let storage = MemStorage;
        let map = storage.load::<S>().await.unwrap();
        assert!(map.is_empty());
    }

    #[tokio::test]
    async fn persist_is_noop() {
        let storage = MemStorage;
        let doc = Document::new(Uuid::new_v4(), S { x: 1 });
        assert!(storage.persist(&[doc], WalOp::Insert).await.is_ok());
    }

    #[tokio::test]
    async fn new_ignores_db_name() {
        let storage = MemStorage::new("any_path", 2.0).await.unwrap();
        let map = storage.load::<S>().await.unwrap();
        assert!(map.is_empty());
    }

    #[tokio::test]
    async fn file_size_is_zero() {
        let storage = MemStorage;
        assert_eq!(storage.file_size().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn compact_is_noop() {
        use std::collections::HashMap;
        let storage = MemStorage;
        let data: RedDbHM = HashMap::new();
        assert!(storage.compact(&data).await.is_ok());
    }
}
