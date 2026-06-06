use futures::stream::{self, StreamExt};
use futures::TryStreamExt;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
pub use uuid::Uuid;

mod config;
mod document;
mod error;
mod query;
pub mod serializer;
mod storage;
mod update;
mod wal;

pub use config::DbConfig;
pub use document::Document;
use error::{RedDbError, Result};
pub use query::QueryBuilder;
pub use update::UpdateWhereBuilder;
use serde::{Deserialize, Serialize};
use serializer::Serializer;
pub use storage::FileStorage;
pub use storage::MemStorage;
use storage::Storage;
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

#[derive(Debug)]
pub struct RedDb<SE, ST> {
    storage: ST,
    serializer: SE,
    data: Arc<RwLock<RedDbHM>>,
}

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
        let storage = ST::new(&stem).await?;
        let data = storage.load::<T>().await?;
        Ok(Self {
            storage,
            data: Arc::new(RwLock::new(data)),
            serializer: SE::default(),
        })
    }

    /// Convenience constructor — equivalent to `open(DbConfig::new(name)).await`.
    pub async fn new<T>(db_name: &str) -> Result<Self>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq + Send + Sync,
    {
        Self::open::<T>(DbConfig::new(db_name)).await
    }

    /// Acquire a shared read lock on the in-memory store.
    pub(crate) async fn read_lock(&self) -> Result<RwLockReadGuard<'_, RedDbHM>> {
        Ok(self.data.read().await)
    }

    /// Acquire an exclusive write lock on the in-memory store.
    pub(crate) async fn write_lock(&self) -> Result<RwLockWriteGuard<'_, RedDbHM>> {
        Ok(self.data.write().await)
    }

    async fn write(&self) -> Result<RwLockWriteGuard<'_, RedDbHM>> {
        Ok(self.data.write().await)
    }

    /// Deserialize raw bytes stored in the in-memory map back into `T`.
    pub(crate) fn deserialize_raw<T>(&self, raw: &[u8]) -> Result<T>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
    {
        self.deserialize(raw)
    }

    /// Serialize `value` into the raw bytes format used by the in-memory map.
    pub(crate) fn serialize_raw<T>(&self, value: &T) -> Result<Vec<u8>>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
    {
        self.serialize(value)
    }

    /// Persist a batch of documents via the storage backend.
    pub(crate) async fn storage_persist<T>(&self, docs: &[Document<T>], op: WalOp) -> Result<()>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + Clone + Send + Sync,
    {
        self.storage.persist(docs, op).await
    }

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

    async fn find_uuids<T>(&self, search: &T) -> Result<Vec<Uuid>>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
    {
        let data = self.read_lock().await?;
        let serialized = self.serialize(search)?;

        let uuids = data
            .iter()
            .filter(|(_, value)| **value == serialized)
            .map(|(id, _)| *id)
            .collect();

        Ok(uuids)
    }

    async fn insert_document<T>(&self, value: T) -> Result<Document<T>>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
    {
        let mut data = self.write().await?;
        let id = Uuid::new_v4();
        let serialized = self.serialize(&value)?;
        data.insert(id, serialized);
        Ok(Document::new(id, value))
    }

    pub async fn insert_one<T>(&self, value: T) -> Result<Document<T>>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + Clone + PartialEq + Send + Sync,
    {
        let doc = self.insert_document(value).await?;
        self.storage.persist(&[doc.clone()], WalOp::Insert).await?;
        Ok(doc)
    }

    pub async fn insert<T>(&self, values: Vec<T>) -> Result<Vec<Document<T>>>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + Clone + PartialEq + Send + Sync,
    {
        let docs: Vec<Document<T>> = stream::iter(values)
            .then(|data| self.insert_document(data))
            .try_collect()
            .await?;

        self.storage.persist(&docs, WalOp::Insert).await?;
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
        let mut data = self.write().await?;

        if data.contains_key(id) {
            let entry = data.get_mut(id).ok_or(RedDbError::NotFound(*id))?;
            *entry = self.serialize(&new_value)?;
            let doc = Document::new(*id, new_value);
            self.storage.persist(&[doc], WalOp::Update).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn remove_document<T>(&self, id: Uuid) -> Result<Document<T>>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq + Send + Sync,
    {
        let mut data = self.write().await?;
        let raw = data.remove(&id).ok_or(RedDbError::NotFound(id))?;
        let value = self.deserialize(&raw)?;
        Ok(Document::new(id, value))
    }

    pub async fn delete_one<T>(&self, id: &Uuid) -> Result<Document<T>>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + Clone + PartialEq + Send + Sync,
    {
        let doc = self.remove_document(*id).await?;
        self.storage.persist(&[doc.clone()], WalOp::Delete).await?;
        Ok(doc)
    }

    pub async fn find_all<T>(&self) -> Result<Vec<Document<T>>>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
    {
        let data = self.read_lock().await?;

        let docs = data
            .iter()
            .map(|(id, raw)| Document::new(*id, self.deserialize(raw).unwrap()))
            .collect();

        Ok(docs)
    }

    pub async fn find<T>(&self, search: &T) -> Result<Vec<Document<T>>>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
    {
        let data = self.read_lock().await?;
        let serialized = self.serialize(search)?;

        let docs = data
            .iter()
            .filter(|(_, raw)| **raw == serialized)
            .map(|(id, raw)| Document::new(*id, self.deserialize(raw).unwrap()))
            .collect();

        Ok(docs)
    }

    pub async fn update<T>(&self, search: &T, new_value: &T) -> Result<usize>
    where
        for<'de> T: Serialize + Deserialize<'de> + Clone + Debug + PartialEq + Send + Sync,
    {
        let mut data = self.write().await?;
        let query = self.serialize(search)?;

        let docs: Vec<Document<T>> = data
            .iter_mut()
            .filter(|(_, raw)| **raw == query)
            .map(|(id, raw)| {
                *raw = self.serialize(new_value).unwrap();
                Document::new(*id, new_value.clone())
            })
            .collect();

        let count = docs.len();
        self.storage.persist(&docs, WalOp::Update).await?;
        Ok(count)
    }

    pub async fn delete<T>(&self, search: &T) -> Result<usize>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + Clone + PartialEq + Send + Sync,
    {
        let uuids = self.find_uuids(search).await?;

        let docs: Vec<Document<T>> = stream::iter(uuids)
            .then(|id| self.remove_document(id))
            .try_collect()
            .await?;

        self.storage.persist(&docs, WalOp::Delete).await?;
        Ok(docs.len())
    }

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

#[cfg(test)]
#[cfg_attr(not(feature = "ron_ser"), ignore)]
mod tests {
    use super::*;
    use crate::RonDb;
    use std::fs;

    #[derive(Clone, Debug, Serialize, PartialEq, Deserialize)]
    struct TestStruct {
        foo: String,
    }

    #[tokio::test]
    async fn insert_document() {
        let db = RonDb::new::<TestStruct>(".test.db").await.unwrap();
        let doc: Document<TestStruct> = db.insert_document(TestStruct { foo: "test".to_owned() }).await.unwrap();
        let find: Document<TestStruct> = db.find_one(&doc.id).await.unwrap();
        assert_eq!(find.data, doc.data);
    }

    #[tokio::test]
    async fn find_uuids() {
        let db = RonDb::new::<TestStruct>(".test2.db").await.unwrap();
        let doc = db.insert_document(TestStruct { foo: "test".to_owned() }).await.unwrap();
        let doc2 = db.insert_document(TestStruct { foo: "test2".to_owned() }).await.unwrap();
        let doc3 = db.insert_document(TestStruct { foo: "test".to_owned() }).await.unwrap();

        let uuids: Vec<Uuid> = db.find_uuids(&TestStruct { foo: "test".to_owned() }).await.unwrap();

        assert!(uuids.contains(&doc.id));
        assert!(!uuids.contains(&doc2.id));
        assert!(uuids.contains(&doc3.id));

        fs::remove_file(".test2.db.ron").unwrap();
    }

    #[tokio::test]
    async fn insert_and_find_one() {
        let db = RonDb::new::<TestStruct>(".insert_and_find_one.db").await.unwrap();
        let doc = db.insert_one(TestStruct { foo: "test".to_owned() }).await.unwrap();
        let find: Document<TestStruct> = db.find_one(&doc.id).await.unwrap();
        assert_eq!(find.id, doc.id);
        assert_eq!(find.data, doc.data);
        fs::remove_file(".insert_and_find_one.db.ron").unwrap();
    }

    #[tokio::test]
    async fn get_returns_some_for_existing() {
        let db = RonDb::new::<TestStruct>(".get_existing.db").await.unwrap();
        let doc = db.insert_one(TestStruct { foo: "hello".to_owned() }).await.unwrap();
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
        let one = TestStruct { foo: String::from("one") };
        let two = TestStruct { foo: String::from("two") };
        db.insert(vec![one.clone(), one.clone(), two.clone()]).await.unwrap();
        let result = db.find(&one).await.unwrap();
        assert_eq!(result.len(), 2);
        fs::remove_file(".find.db.ron").unwrap();
    }

    #[tokio::test]
    async fn update_one() {
        let db = RonDb::new::<TestStruct>(".update_one.db").await.unwrap();
        let original = TestStruct { foo: "hi".to_owned() };
        let updated = TestStruct { foo: "bye".to_owned() };
        let doc = db.insert_one(original.clone()).await.unwrap();
        db.update_one(&doc.id, updated.clone()).await.unwrap();
        let result: Document<TestStruct> = db.find_one(&doc.id).await.unwrap();
        assert_eq!(result.data, updated);
        fs::remove_file(".update_one.db.ron").unwrap();
    }

    #[tokio::test]
    async fn update() {
        let db = RonDb::new::<TestStruct>(".update.db").await.unwrap();
        let one = TestStruct { foo: String::from("one") };
        let two = TestStruct { foo: String::from("two") };
        db.insert(vec![one.clone(), one.clone(), two.clone()]).await.unwrap();
        let updated = db.update(&one, &two).await.unwrap();
        assert_eq!(updated, 2);
        let result = db.find(&two).await.unwrap();
        assert_eq!(result.len(), 3);
        fs::remove_file(".update.db.ron").unwrap();
    }

    #[tokio::test]
    async fn delete_one_removes_document() {
        let db = RonDb::new::<TestStruct>(".delete_one.db").await.unwrap();
        let doc = db.insert_one(TestStruct { foo: "test".to_owned() }).await.unwrap();
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
        let one = TestStruct { foo: "one".to_owned() };
        let two = TestStruct { foo: "two".to_owned() };
        db.insert(vec![one.clone(), one.clone(), two.clone()]).await.unwrap();
        let deleted = db.delete(&one).await.unwrap();
        assert_eq!(deleted, 2);
        let not_deleted = db.delete(&one).await.unwrap();
        assert_eq!(not_deleted, 0);
        fs::remove_file(".delete.db.ron").unwrap();
    }

    #[tokio::test]
    async fn serialie_deserialize() {
        let db = RonDb::new::<TestStruct>(".serialize.db").await.unwrap();
        let test = TestStruct { foo: "one".to_owned() };
        let serialized = db.serializer.serialize(&test).unwrap();
        let deserialized: TestStruct = db.serializer.deserialize(&serialized).unwrap();
        assert_eq!(deserialized, test);
        fs::remove_file(".serialize.db.ron").unwrap();
    }
}
