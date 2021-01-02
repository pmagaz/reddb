use failure::ResultExt;
use futures::stream::{self, StreamExt};
use futures::TryStreamExt;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use std::thread;
use std::time::Instant;
use tokio::runtime::Runtime;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
pub use uuid::Uuid;

mod document;
mod error;
pub mod serializer;
mod status;
mod storage;

pub use document::Document;
use error::{RedDbErrorKind, Result};
use serde::{Deserialize, Serialize};
use serializer::Serializer;
use status::Status;
pub use storage::FileStorage;
use storage::Storage;

type RedDbHM = HashMap<Uuid, Vec<u8>>;

#[cfg(feature = "bin_ser")]
pub type BinDb = RedDb<serializer::Bin, FileStorage<serializer::Bin>>;
#[cfg(feature = "json_ser")]
pub type JsonDb = RedDb<serializer::Json, FileStorage<serializer::Json>>;
#[cfg(feature = "yaml_ser")]
pub type YamlDb = RedDb<serializer::Yaml, FileStorage<serializer::Yaml>>;
#[cfg(feature = "ron_ser")]
pub type RonDb = RedDb<serializer::Ron, FileStorage<serializer::Ron>>;

#[derive(Debug)]
pub struct RedDb<SE, ST> {
    storage: ST,
    serializer: SE,
    data: Arc<RwLock<RedDbHM>>,
}

impl<'a, SE, ST: 'static> RedDb<SE, ST>
where
    for<'de> SE: Serializer<'de> + Debug,
    for<'de> ST: Storage + Debug + Send + Sync,
{
    pub fn new<T>(db_name: &'static str) -> Result<Self>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq + Send + Sync,
    {
        let mut rt = Runtime::new().unwrap();

        let (data, storage) = thread::spawn(move || {
            let storage = rt.block_on(async { ST::new(db_name).await.unwrap() });
            let data = rt.block_on(async { storage.load::<T>().await.unwrap() });
            (data, storage)
        })
        .join()
        .map_err(|_| RedDbErrorKind::Datapersist)?;

        Ok(Self {
            storage,
            data: Arc::new(RwLock::new(data)),
            serializer: SE::default(),
        })
    }

    async fn read(&'a self) -> Result<RwLockReadGuard<'a, RedDbHM>> {
        let lock = self.data.read().await;
        Ok(lock)
    }

    async fn write(&'a self) -> Result<RwLockWriteGuard<'a, RedDbHM>> {
        let lock = self.data.write().await;
        Ok(lock)
    }

    fn create_doc<T>(&self, id: &Uuid, value: T, status: Status) -> Document<T>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
    {
        Document::new(*id, value, status)
    }

    async fn find_uuids<T>(&self, search: &T) -> Result<Vec<Uuid>>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
    {
        let data = self
            .read()
            .await
            .map_err(|_| RedDbErrorKind::PoisonedValue)
            .unwrap();

        let serialized = self.serialize(search)?;

        let docs: Vec<Uuid> = data
            .iter()
            .filter(|(uuid, value)| **value == serialized)
            .map(|(id, _value)| *id)
            .collect();

        Ok(docs)
    }

    async fn insert_document<T>(&self, value: T) -> Result<Document<T>>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
    {
        let mut data = self.write().await?;
        let id = Uuid::new_v4();
        let serialized = self.serialize(&value)?;
        data.insert(id, serialized);
        let result = self.create_doc(&id, value, Status::default());

        Ok(result)
    }

    pub async fn insert_one<T>(&self, value: T) -> Result<Document<T>>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + Clone + PartialEq + Send + Sync,
    {
        let doc = self.insert_document(value).await?;
        self.storage
            .persist(&[doc.to_owned()])
            .await
            .context(RedDbErrorKind::Datapersist)?;
        Ok(doc)
    }

    pub async fn insert<T>(&self, values: Vec<T>) -> Result<Vec<Document<T>>>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq + Send + Sync,
    {
        let docs: Vec<Document<T>> = stream::iter(values)
            .then(|data| self.insert_document(data))
            .try_collect()
            .await?;

        self.storage
            .persist(&docs)
            .await
            .context(RedDbErrorKind::Datapersist)?;

        Ok(docs)
    }

    pub async fn find_one<T>(&self, id: &Uuid) -> Result<Document<T>>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
    {
        let data = self
            .read()
            .await
            .map_err(|_| RedDbErrorKind::PoisonedValue)?;

        let data = data
            .get(&id)
            .ok_or(RedDbErrorKind::NotFound { uuid: *id })?;

        let data = self.deserialize(&*data)?;
        let doc = self.create_doc(id, data, Status::In);
        Ok(doc)
    }

    pub async fn update_one<T>(&'a self, id: &Uuid, new_value: T) -> Result<bool>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq + Send + Sync,
    {
        let mut data = self
            .write()
            .await
            .map_err(|_| RedDbErrorKind::PoisonedValue)?;

        if data.contains_key(id) {
            let data = data
                .get_mut(&id)
                .ok_or(RedDbErrorKind::NotFound { uuid: *id })?;

            *data = self.serialize(&new_value)?;
            let doc = self.create_doc(id, new_value, Status::Up);

            self.storage
                .persist(&[doc])
                .await
                .context(RedDbErrorKind::Datapersist)?;

            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn remove_document<T>(&self, id: Uuid) -> Result<Document<T>>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq + Send + Sync,
    {
        let mut data = self.write().await?;
        let value = data.remove(&id).unwrap();
        let data = self.deserialize(&value)?;
        let doc = self.create_doc(&id, data, Status::De);
        Ok(doc)
    }

    pub async fn delete_one<T>(&self, id: &Uuid) -> Result<Document<T>>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq + Send + Sync,
    {
        let result = self.remove_document(*id).await?;
        Ok(result)
    }

    pub async fn find_all<T>(&self) -> Result<Vec<Document<T>>>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
    {
        let data = self
            .read()
            .await
            .map_err(|_| RedDbErrorKind::PoisonedValue)?;

        let docs: Vec<Document<T>> = data
            .iter()
            .map(|(id, data)| {
                let data = self.deserialize(&*data).unwrap();
                self.create_doc(id, data, Status::In)
            })
            .collect();

        Ok(docs)
    }

    pub async fn find<T>(&self, search: &T) -> Result<Vec<Document<T>>>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
    {
        let data = self
            .read()
            .await
            .map_err(|_| RedDbErrorKind::PoisonedValue)?;

        let serialized = self.serialize(search)?;

        let docs: Vec<Document<T>> = data
            .iter()
            .filter(|(uuid, data)| **data == serialized)
            .map(|(id, data)| {
                let data = self.deserialize(&*data).unwrap();
                self.create_doc(id, data, Status::In)
            })
            .collect();

        Ok(docs)
    }

    pub async fn update<T>(&self, search: &T, new_value: &T) -> Result<usize>
    where
        for<'de> T: Serialize + Deserialize<'de> + Clone + Debug + PartialEq + Send + Sync,
    {
        let mut data = self
            .write()
            .await
            .map_err(|_| RedDbErrorKind::PoisonedValue)?;

        let query = self.serialize(search)?;

        let docs: Vec<Document<T>> = data
            .iter_mut()
            .filter(|(uuid, data)| **data == query)
            .map(|(id, data)| {
                *data = self.serialize(new_value).unwrap();
                self.create_doc(id, new_value.to_owned(), Status::Up)
            })
            .collect();

        let result = docs.len();

        self.storage
            .persist(&docs)
            .await
            .context(RedDbErrorKind::Datapersist)?;

        Ok(result)
    }

    pub async fn delete<T>(&self, search: &T) -> Result<usize>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq + Send + Sync,
    {
        let ids = self.find_uuids(search).await?;

        let docs: Vec<Document<T>> = stream::iter(ids)
            .then(|id| self.remove_document(id))
            .try_collect()
            .await?;

        self.storage
            .persist(&docs)
            .await
            .context(RedDbErrorKind::Datapersist)?;

        Ok(docs.len())
    }

    fn serialize<T>(&self, value: &T) -> Result<Vec<u8>>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
    {
        Ok(self
            .serializer
            .serialize(value)
            .context(RedDbErrorKind::Serialization)?)
    }

    fn deserialize<T>(&self, value: &[u8]) -> Result<T>
    where
        for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
    {
        Ok(self
            .serializer
            .deserialize(value)
            .context(RedDbErrorKind::Deserialization)?)
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
        let db = RonDb::new::<TestStruct>(".test.db").unwrap();
        let uuid = &Uuid::new_v4();
        let data = TestStruct {
            foo: "test".to_owned(),
        };
        let doc: Document<TestStruct> = db.insert_document(data).await.unwrap();
        let find: Document<TestStruct> = db.find_one(&doc.uuid).await.unwrap();
        assert_eq!(find.data, doc.data);
    }
    #[tokio::test]
    async fn find_uuids() {
        let db = RonDb::new::<TestStruct>(".test.db").unwrap();
        let doc: Document<TestStruct> = db
            .insert_document(TestStruct {
                foo: "test".to_owned(),
            })
            .await
            .unwrap();

        let doc2: Document<TestStruct> = db
            .insert_document(TestStruct {
                foo: "test2".to_owned(),
            })
            .await
            .unwrap();

        let doc3: Document<TestStruct> = db
            .insert_document(TestStruct {
                foo: "test".to_owned(),
            })
            .await
            .unwrap();
        let ids: Vec<Uuid> = db
            .find_uuids(&TestStruct {
                foo: "test".to_owned(),
            })
            .await
            .unwrap();

        assert_eq!(ids.contains(&doc.uuid), true);
        assert_eq!(ids.contains(&doc2.uuid), false);
        assert_eq!(ids.contains(&doc3.uuid), true);

        fs::remove_file(".test.db.ron").unwrap();
    }
    #[tokio::test]
    async fn insert_and_find_one() {
        let db = RonDb::new::<TestStruct>(".insert_and_find_one.db").unwrap();
        let doc: Document<TestStruct> = db
            .insert_one(TestStruct {
                foo: "test".to_owned(),
            })
            .await
            .unwrap();

        let find: Document<TestStruct> = db.find_one(&doc.uuid).await.unwrap();
        assert_eq!(find.uuid, doc.uuid);
        assert_eq!(find.data, doc.data);

        fs::remove_file(".insert_and_find_one.db.ron").unwrap();
    }
    #[tokio::test]
    async fn find() {
        let db = RonDb::new::<TestStruct>(".find.db").unwrap();

        let one = TestStruct {
            foo: String::from("one"),
        };

        let two = TestStruct {
            foo: String::from("two"),
        };

        let many = vec![one.clone(), one.clone(), two.clone()];
        db.insert(many).await.unwrap();
        let result = db.find(&one).await.unwrap();
        assert_eq!(result.len(), 2);
        fs::remove_file(".find.db.ron").unwrap();
    }
    #[tokio::test]
    async fn update_one() {
        let db = RonDb::new::<TestStruct>(".update_one.db").unwrap();
        let original = TestStruct {
            foo: "hi".to_owned(),
        };

        let updated = TestStruct {
            foo: "bye".to_owned(),
        };

        let doc = db.insert_one(original.clone()).await.unwrap();
        db.update_one(&doc.uuid, updated.clone()).await.unwrap();
        let result: Document<TestStruct> = db.find_one(&doc.uuid).await.unwrap();
        assert_eq!(result.data, updated);
        fs::remove_file(".update_one.db.ron").unwrap();
    }

    #[tokio::test]
    async fn update() {
        let db = RonDb::new::<TestStruct>(".update.db").unwrap();
        let one = TestStruct {
            foo: String::from("one"),
        };
        let two = TestStruct {
            foo: String::from("two"),
        };

        let many = vec![one.clone(), one.clone(), two.clone()];
        db.insert(many).await.unwrap();
        let updated = db.update(&one, &two).await.unwrap();
        assert_eq!(updated, 2);
        let result = db.find(&two).await.unwrap();
        assert_eq!(result.len(), 3);
        fs::remove_file(".update.db.ron").unwrap();
    }

    #[tokio::test]
    async fn delete_and_find_one() {
        let db = RonDb::new::<TestStruct>(".delete_one.db").unwrap();
        let search = TestStruct {
            foo: "test".to_owned(),
        };

        let doc = db.insert_one(search.clone()).await.unwrap();
        let deleted = db.delete_one(&doc.uuid).await.unwrap();
        assert_eq!(
            deleted,
            Document {
                uuid: doc.uuid,
                data: doc.data,
                _st: Status::De
            }
        );
        fs::remove_file(".delete_one.db.ron").unwrap();
    }

    async fn delete() {
        let db = RonDb::new::<TestStruct>(".delete.db").unwrap();
        let one = TestStruct {
            foo: "one".to_owned(),
        };

        let two = TestStruct {
            foo: "two".to_owned(),
        };

        let many = vec![one.clone(), one.clone(), two.clone()];
        db.insert(many).await.unwrap();
        let deleted = db.delete(&one).await.unwrap();
        assert_eq!(deleted, 2);

        let not_deleted = db.delete(&one).await.unwrap();
        assert_eq!(not_deleted, 0);
        fs::remove_file(".delete.db.ron").unwrap();
    }
    #[tokio::test]
    async fn serialie_deserialize() {
        let db = RonDb::new::<TestStruct>(".serialize.db").unwrap();
        let test = TestStruct {
            foo: "one".to_owned(),
        };
        let byte_str = [40, 102, 111, 111, 58, 34, 111, 110, 101, 34, 41, 10];
        let serialized = db.serializer.serialize(&test).unwrap();
        assert_eq!(serialized, byte_str);
        let deserialized: TestStruct = db.serializer.deserialize(&byte_str).unwrap();
        assert_eq!(deserialized, test);
        fs::remove_file(".serialize.db.ron").unwrap();
    }
}
