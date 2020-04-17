use core::fmt::Display;
use failure::ResultExt;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use uuid::Uuid;
mod serializer;

mod error;
mod kv;
mod operation;
mod storage;

use error::{RdStoreErrorKind, Result};
use kv::KeyValue;
use serializer::Serializer;
pub use serializer::{JsonSerializer, RonSerializer, YamlSerializer};
use std::collections::HashMap;
use std::sync::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
use storage::FileStorage;
use storage::Storage;

pub type ByteString = Vec<u8>;
pub type StoreHM = HashMap<Uuid, Mutex<ByteString>>;

//#[cfg(feature = "json_ser")]
pub type JsonStore = RdStore<JsonSerializer, FileStorage<JsonSerializer>>;
//#[cfg(feature = "yaml_ser")]
pub type YamlStore = RdStore<YamlSerializer, FileStorage<YamlSerializer>>;
//#[cfg(feature = "ron_ser")]
pub type RonStore = RdStore<RonSerializer, FileStorage<RonSerializer>>;

#[derive(Debug)]
pub struct RdStore<SE, ST> {
  pub store: RwLock<StoreHM>,
  pub storage: ST,
  pub serializer: SE,
}

impl<'a, SE, ST> RdStore<SE, ST>
where
  for<'de> SE: Serializer<'de> + Debug,
  for<'de> ST: Storage + Debug,
{
  pub fn new<T>() -> Result<Self>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq + Clone,
  {
    let storage = ST::new::<T>()?;
    let data: StoreHM = storage
      .load_content::<T>()
      .context(RdStoreErrorKind::ContentLoad)?;

    Ok(Self {
      store: RwLock::new(data),
      storage: storage,
      serializer: SE::default(),
    })
  }
  pub fn read(&'a self) -> Result<RwLockReadGuard<'a, StoreHM>> {
    let lock = self.store.read().map_err(|_| RdStoreErrorKind::Poisoned)?;
    Ok(lock)
  }

  fn write(&'a self) -> Result<RwLockWriteGuard<'a, StoreHM>> {
    let lock = self.store.write().map_err(|_| RdStoreErrorKind::Poisoned)?;
    Ok(lock)
  }

  fn serialize<T>(&self, value: &T) -> Result<ByteString>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq,
  {
    Ok(
      self
        .serializer
        .serialize(value)
        .context(RdStoreErrorKind::Serialization)?,
    )
  }

  fn deserialize<T>(&self, value: &Vec<u8>) -> Result<T>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq,
  {
    Ok(
      self
        .serializer
        .deserialize(value)
        .context(RdStoreErrorKind::Deserialization)?,
    )
  }

  fn insert_key_value(&self, key: Uuid, data: ByteString) -> Option<Mutex<ByteString>> {
    let mut store = self.store.write().unwrap();
    store.insert(key, Mutex::new(data))
  }

  pub fn find_keys<T>(&self, search: &T) -> Result<Vec<Uuid>>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq,
  {
    let store = self.read()?;
    let serialized = self.serialize(search)?;
    let kv_pairs: Vec<Uuid> = store
      .iter()
      .map(|(_key, value)| {
        (
          _key,
          value
            .lock()
            .map_err(|_| RdStoreErrorKind::PoisonedValue)
            .unwrap(),
        )
      })
      .filter(|(_key, value)| **value == serialized)
      .map(|(key, _value)| *key)
      .collect();
    Ok(kv_pairs)
  }

  pub fn insert<T>(&self, value: T) -> Result<Uuid>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq,
  {
    let key = Uuid::new_v4();
    let data = self.serialize(&value)?;
    let _result = self.insert_key_value(key, data);
    self
      .storage
      .save_one(KeyValue::new(key, value))
      .context(RdStoreErrorKind::DataSave)?;
    Ok(key)
  }

  pub fn find<T>(&self, key: &Uuid) -> Result<T>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq,
  {
    let store = self.read()?;
    let value = store
      .get(&key)
      .ok_or(RdStoreErrorKind::NotFound { key: *key })?;
    let guard = value.lock().map_err(|_| RdStoreErrorKind::PoisonedValue)?;
    let result = self.deserialize(&*guard)?;
    Ok(result)
  }

  pub fn update<T>(&'a self, key: &Uuid, new_value: T) -> Result<bool>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq,
  {
    let mut store = self.write()?;
    if store.contains_key(key) {
      let value = store
        .get_mut(&key)
        .ok_or(RdStoreErrorKind::NotFound { key: *key })?;

      let mut guard = value.lock().map_err(|_| RdStoreErrorKind::PoisonedValue)?;
      *guard = self.serialize(&new_value)?;

      self
        .storage
        .save_one(KeyValue::new(*key, new_value))
        .context(RdStoreErrorKind::DataSave)?;
      Ok(true)
    } else {
      Ok(false)
    }
  }

  pub fn delete(&self, key: &Uuid) -> Result<bool> {
    let mut store = self.store.write().unwrap();
    if store.contains_key(key) {
      store.remove(key).unwrap();
      Ok(true)
    } else {
      Ok(false)
    }
  }

  pub fn insert_many<T>(&self, values: Vec<T>) -> Result<usize>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq,
  {
    let kv_pairs: Vec<KeyValue<T>> = values
      .into_iter()
      .map(|value| {
        let key = Uuid::new_v4();
        let serialized = self.serialize(&value).unwrap();
        let _result = self.insert_key_value(key, serialized);
        KeyValue::new(key, value)
      })
      .collect();

    let result = kv_pairs.len();
    println!("{:?}", result);
    self
      .storage
      .save(kv_pairs)
      .context(RdStoreErrorKind::DataSave)?;

    Ok(result)
  }

  pub fn find_many<T>(&self, search: &T) -> Result<Vec<T>>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq,
  {
    let store = self.read()?;
    let serialized = self.serialize(search)?;
    let kv_pairs: Vec<T> = store
      .iter()
      .map(|(_key, value)| {
        value
          .lock()
          .map_err(|_| RdStoreErrorKind::PoisonedValue)
          .unwrap()
      })
      .filter(|value| **value == serialized)
      .map(|value| self.deserialize(&*value).unwrap())
      .collect();
    Ok(kv_pairs)
  }

  pub fn update_many<T>(&self, search: &T, new_value: &T) -> Result<usize>
  where
    for<'de> T: Serialize + Deserialize<'de> + Clone + Debug + Display + PartialEq,
  {
    let mut store = self.write()?;
    let serialized = self.serialize(search)?;

    let kv_pairs: Vec<KeyValue<T>> = store
      .iter_mut()
      .map(|(_key, value)| {
        (
          _key,
          value
            .lock()
            .map_err(|_| RdStoreErrorKind::PoisonedValue)
            .unwrap(),
        )
      })
      .filter(|(_key, value)| **value == serialized)
      .map(|(key, mut value)| {
        *value = self.serialize(new_value).unwrap();
        KeyValue::new(*key, new_value.clone())
      })
      .collect();

    let result = kv_pairs.len();
    self
      .storage
      .save(kv_pairs)
      .context(RdStoreErrorKind::DataSave)?;

    Ok(result)
  }

  pub fn delete_many<T>(&self, search: &T) -> Result<usize>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq,
  {
    let keys = self.find_keys(search)?;
    let kv_pairs: Vec<bool> = keys.iter().map(|key| (self.delete(key).unwrap())).collect();
    Ok(kv_pairs.len())
  }
}
