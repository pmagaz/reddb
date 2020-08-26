use failure::ResultExt;
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
use std::sync::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
use storage::FileStorage;
use storage::Storage;

pub type RedDbHM = HashMap<Uuid, Mutex<Vec<u8>>>;

//#[cfg(feature = "json_ser")]
pub type JsonDb = RedDb<JsonSerializer, FileStorage<JsonSerializer>>;
//#[cfg(feature = "yaml_ser")]
pub type YamlDb = RedDb<YamlSerializer, FileStorage<YamlSerializer>>;
//#[cfg(feature = "ron_ser")]
pub type RonDb = RedDb<RonSerializer, FileStorage<RonSerializer>>;

#[derive(Debug)]
pub struct RedDb<SE, ST> {
  pub db: RwLock<RedDbHM>,
  pub storage: ST,
  pub serializer: SE,
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
    let data: RedDbHM = storage
      .load_content::<T>()
      .context(RedDbErrorKind::ContentLoad)?;

    Ok(Self {
      db: RwLock::new(data),
      storage,
      serializer: SE::default(),
    })
  }
  pub fn read(&'a self) -> Result<RwLockReadGuard<'a, RedDbHM>> {
    let lock = self.db.read().map_err(|_| RedDbErrorKind::Poisoned)?;
    Ok(lock)
  }

  fn write(&'a self) -> Result<RwLockWriteGuard<'a, RedDbHM>> {
    let lock = self.db.write().map_err(|_| RedDbErrorKind::Poisoned)?;
    Ok(lock)
  }

  pub fn create_doc<T>(&self, _id: &Uuid, data: T) -> Document<T>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
  {
    Document::new(*_id, data)
  }

  fn insert_doc<T>(&self, data: T) -> Result<Document<T>>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
  {
    let mut db = self.db.write().unwrap();
    let _id = Uuid::new_v4();
    let serialized = self.serialize(&data).unwrap();
    db.insert(_id, Mutex::new(serialized));
    let doc = self.create_doc(&_id, data);
    Ok(doc)
  }

  pub fn find_ids<T>(&self, search: &T) -> Result<Vec<Uuid>>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
  {
    let db = self.read()?;
    let serialized = self.serialize(search)?;
    let docs: Vec<Uuid> = db
      .iter()
      .map(|(_id, data)| {
        (
          _id,
          data
            .lock()
            .map_err(|_| RedDbErrorKind::PoisonedValue)
            .unwrap(),
        )
      })
      .filter(|(_id, data)| **data == serialized)
      .map(|(_id, _value)| *_id)
      .collect();
    Ok(docs)
  }

  pub fn insert_one<T>(&self, data: T) -> Result<Document<T>>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Clone + PartialEq,
  {
    let doc = self.insert_doc(data).unwrap();
    self
      .storage
      .persist(&[doc.to_owned()])
      .context(RedDbErrorKind::Datapersist)?;
    Ok(doc)
  }

  pub fn find_one<T>(&self, _id: &Uuid) -> Result<Document<T>>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
  {
    let db = self.read()?;
    let data = db
      .get(&_id)
      .ok_or(RedDbErrorKind::NotFound { uuid: *_id })?;

    let guard = data.lock().map_err(|_| RedDbErrorKind::PoisonedValue)?;
    let data = self.deserialize(&*guard)?;
    let doc = self.create_doc(_id, data);
    Ok(doc)
  }

  pub fn update_one<T>(&'a self, _id: &Uuid, new_value: T) -> Result<bool>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
  {
    let mut db = self.write()?;
    if db.contains_key(_id) {
      let data = db
        .get_mut(&_id)
        .ok_or(RedDbErrorKind::NotFound { uuid: *_id })?;

      let mut guard = data.lock().map_err(|_| RedDbErrorKind::PoisonedValue)?;
      *guard = self.serialize(&new_value)?;
      let doc = self.create_doc(_id, new_value);
      self
        .storage
        .persist(&[doc])
        .context(RedDbErrorKind::Datapersist)?;
      Ok(true)
    } else {
      Ok(false)
    }
  }

  pub fn delete_one(&self, _id: &Uuid) -> Result<bool> {
    let mut db = self.db.write().unwrap();
    if db.contains_key(_id) {
      db.remove(_id).unwrap();
      Ok(true)
    } else {
      Ok(false)
    }
  }

  pub fn insert<T>(&self, values: Vec<T>) -> Result<Vec<Document<T>>>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
  {
    let docs: Vec<Document<T>> = values
      .into_iter()
      .map(|data| self.insert_doc(data).unwrap())
      .collect();

    self
      .storage
      .persist(&docs)
      .context(RedDbErrorKind::Datapersist)?;

    Ok(docs)
  }

  pub fn find<T>(&self, search: &T) -> Result<Vec<Document<T>>>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
  {
    let db = self.read()?;
    let serialized = self.serialize(search)?;
    let docs: Vec<Document<T>> = db
      .iter()
      .map(|(_id, data)| {
        (
          _id,
          data
            .lock()
            .map_err(|_| RedDbErrorKind::PoisonedValue)
            .unwrap(),
        )
      })
      .filter(|(_id, data)| **data == serialized)
      .map(|(_id, data)| {
        let data = self.deserialize(&*data).unwrap();
        self.create_doc(_id, data)
      })
      .collect();
    Ok(docs)
  }

  pub fn update<T>(&self, search: &T, new_value: &T) -> Result<usize>
  where
    for<'de> T: Serialize + Deserialize<'de> + Clone + Debug + PartialEq,
  {
    let mut db = self.write()?;
    let query = self.serialize(search)?;

    let docs: Vec<Document<T>> = db
      .iter_mut()
      .map(|(_id, data)| {
        (
          _id,
          data
            .lock()
            .map_err(|_| RedDbErrorKind::PoisonedValue)
            .unwrap(),
        )
      })
      .filter(|(_id, data)| **data == query)
      .map(|(_id, mut data)| {
        *data = self.serialize(new_value).unwrap();
        self.create_doc(_id, new_value.to_owned())
      })
      .collect();

    let result = docs.len();
    self
      .storage
      .persist(&docs)
      .context(RedDbErrorKind::Datapersist)?;

    Ok(result)
  }

  pub fn delete<T>(&self, search: &T) -> Result<usize>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
  {
    let ids = self.find_ids(search)?;
    let docs: Vec<bool> = ids
      .iter()
      .map(|_id| (self.delete_one(_id).unwrap()))
      .collect();
    Ok(docs.len())
  }

  fn serialize<T>(&self, data: &T) -> Result<Vec<u8>>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
  {
    Ok(
      self
        .serializer
        .serialize(data)
        .context(RedDbErrorKind::Serialization)?,
    )
  }

  fn deserialize<T>(&self, data: &[u8]) -> Result<T>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + PartialEq,
  {
    Ok(
      self
        .serializer
        .deserialize(data)
        .context(RedDbErrorKind::Deserialization)?,
    )
  }
}

#[cfg(test)]
mod tests {
  #[test]
  fn it_works() {
    assert_eq!(2 + 2, 4);
  }
}
