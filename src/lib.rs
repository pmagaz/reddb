use failure::ResultExt;
use futures::stream::{self, StreamExt};
use futures::TryStreamExt;

use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use uuid::Uuid;
mod document;
mod error;
mod serializer;
mod storage;

pub use document::Document;
use error::{RedDbErrorKind, Result};
pub use serializer::{JsonSerializer, RonSerializer, Serializer, YamlSerializer};
use std::collections::HashMap;

use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use storage::FileStorage;
use storage::Storage;

pub type RedDbHM = HashMap<Uuid, Vec<u8>>;

//#[cfg(feature = "json_ser")]
pub type JsonDb = RedDb<JsonSerializer, FileStorage<JsonSerializer>>;
//#[cfg(feature = "yaml_ser")]
pub type YamlDb = RedDb<YamlSerializer, FileStorage<YamlSerializer>>;
//#[cfg(feature = "ron_ser")]
pub type RonDb = RedDb<RonSerializer, FileStorage<RonSerializer>>;

#[derive(Debug)]
pub struct RedDb<SE, ST> {
  storage: ST,
  serializer: SE,
  data: RwLock<RedDbHM>,
}

impl<'a, SE, ST> RedDb<SE, ST>
where
  for<'de> SE: Serializer<'de> + Debug,
  for<'de> ST: Storage + Debug,
{
  pub fn new<T>(db_name: &str) -> Result<Self>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
  {
    let storage = ST::new(db_name)?;
    let data: RedDbHM = storage.load::<T>().context(RedDbErrorKind::ContentLoad)?;

    Ok(Self {
      storage,
      data: RwLock::new(data),
      serializer: SE::default(),
    })
  }

  async fn read(&'a self) -> Result<RwLockReadGuard<'a, RedDbHM>> {
    let lock = self.data.read().map_err(|_| RedDbErrorKind::Poisoned)?;
    Ok(lock)
  }

  async fn write(&'a self) -> Result<RwLockWriteGuard<'a, RedDbHM>> {
    let lock = self.data.write().map_err(|_| RedDbErrorKind::Poisoned)?;
    Ok(lock)
  }

  fn create_doc<T>(&self, id: &Uuid, value: T) -> Document<T>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
  {
    Document::new(*id, value)
  }

  async fn insert_document<T>(&self, value: T) -> Result<Document<T>>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
  {
    let mut data = self.write().await?;
    let id = Uuid::new_v4();
    let serialized = self.serialize(&value)?;
    data.insert(id, serialized);
    let result = self.create_doc(&id, value);
    Ok(result)
  }

  async fn find_ids<T>(&self, search: &T) -> Result<Vec<Uuid>>
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
      .map(|(id, value)| (id, value))
      .filter(|(_id, value)| **value == serialized)
      .map(|(id, _value)| *id)
      .collect();

    Ok(docs)
  }

  pub async fn insert_one<T>(&self, value: T) -> Result<Document<T>>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Clone + PartialEq + Sync,
  {
    let doc = self.insert_document(value).await?;
    self
      .storage
      .persist(&[doc.to_owned()])
      .await
      .context(RedDbErrorKind::Datapersist)?;

    Ok(doc)
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
    let doc = self.create_doc(id, data);
    Ok(doc)
  }

  pub async fn update_one<T>(&'a self, id: &Uuid, new_value: T) -> Result<bool>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq + Sync,
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
      let doc = self.create_doc(id, new_value);
      self
        .storage
        .persist(&[doc])
        .await
        .context(RedDbErrorKind::Datapersist)?;
      Ok(true)
    } else {
      Ok(false)
    }
  }

  pub fn delete_one(&self, id: &Uuid) -> Result<bool> {
    let mut data = self.data.write().unwrap();
    if data.contains_key(id) {
      data.remove(id).unwrap();
      Ok(true)
    } else {
      Ok(false)
    }
  }
  pub async fn insert<T>(&self, values: Vec<T>) -> Result<Vec<Document<T>>>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
  {
    let docs: Vec<Document<T>> = stream::iter(values)
      .then(|data| self.insert_document(data))
      .try_collect()
      .await?;

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
      .map(|(id, data)| (id, data))
      .filter(|(_id, data)| **data == serialized)
      .map(|(id, data)| {
        let data = self.deserialize(&*data).unwrap();
        self.create_doc(id, data)
      })
      .collect();
    Ok(docs)
  }

  pub async fn update<T>(&self, search: &T, new_value: &T) -> Result<usize>
  where
    for<'de> T: Serialize + Deserialize<'de> + Clone + Debug + PartialEq + Sync,
  {
    let mut data = self
      .write()
      .await
      .map_err(|_| RedDbErrorKind::PoisonedValue)?;

    let query = self.serialize(search)?;

    let docs: Vec<Document<T>> = data
      .iter_mut()
      .map(|(id, data)| (id, data))
      .filter(|(_id, data)| **data == query)
      .map(|(id, data)| {
        *data = self.serialize(new_value).unwrap();
        self.create_doc(id, new_value.to_owned())
      })
      .collect();

    let result = docs.len();
    self
      .storage
      .persist(&docs)
      .await
      .context(RedDbErrorKind::Datapersist)?;

    Ok(result)
  }

  pub async fn delete<T>(&self, search: &T) -> Result<usize>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
  {
    let ids = self.find_ids(search).await?;
    let docs: Vec<bool> = ids
      .iter()
      .map(|id| (self.delete_one(id).unwrap()))
      .collect();
    Ok(docs.len())
  }

  fn serialize<T>(&self, value: &T) -> Result<Vec<u8>>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
  {
    Ok(
      self
        .serializer
        .serialize(value)
        .context(RedDbErrorKind::Serialization)?,
    )
  }

  fn deserialize<T>(&self, value: &[u8]) -> Result<T>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
  {
    Ok(
      self
        .serializer
        .deserialize(value)
        .context(RedDbErrorKind::Deserialization)?,
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;

  #[derive(Clone, Debug, Serialize, PartialEq, Deserialize)]
  struct TestStruct {
    foo: String,
  }

  #[tokio::test]
  async fn insert_document() {
    let db = RonDb::new::<TestStruct>(".test.db").unwrap();
    let _id = &Uuid::new_v4();
    let data = TestStruct {
      foo: "test".to_owned(),
    };
    let doc: Document<TestStruct> = db.insert_document(data).await.unwrap();
    let find: Document<TestStruct> = db.find_one(&doc.id).await.unwrap();
    assert_eq!(find.data, doc.data);
  }
  #[tokio::test]
  async fn find_ids() {
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
      .find_ids(&TestStruct {
        foo: "test".to_owned(),
      })
      .await
      .unwrap();

    assert_eq!(ids.contains(&doc.id), true);
    assert_eq!(ids.contains(&doc2.id), false);
    assert_eq!(ids.contains(&doc3.id), true);

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

    let find: Document<TestStruct> = db.find_one(&doc.id).await.unwrap();
    assert_eq!(find.id, doc.id);
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
    db.update_one(&doc.id, updated.clone()).await.unwrap();
    let result: Document<TestStruct> = db.find_one(&doc.id).await.unwrap();
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
    let deleted = db.delete_one(&doc.id).unwrap();
    assert_eq!(deleted, true);

    let not_deleted = db.delete_one(&doc.id).unwrap();
    assert_eq!(not_deleted, false);
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
  #[test]
  fn serialie_deserialize() {
    let db = RonDb::new::<TestStruct>(".test.db").unwrap();
    let test = TestStruct {
      foo: "one".to_owned(),
    };
    let byte_str = [40, 102, 111, 111, 58, 34, 111, 110, 101, 34, 41, 10];
    let serialized = db.serializer.serialize(&test).unwrap();
    assert_eq!(serialized, byte_str);
    let deserialized: TestStruct = db.serializer.deserialize(&byte_str).unwrap();
    assert_eq!(deserialized, test);
    fs::remove_file(".test.db.ron").unwrap();
  }
}
