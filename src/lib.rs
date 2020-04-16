use core::fmt::Display;
use failure::ResultExt;
use operation::Operation;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use uuid::Uuid;
mod record;
mod serializer;
//mod store;
//use store::Store;
mod error;
mod operation;
mod storage;

use error::{RdStoreError, RdStoreErrorKind, Result};
use serializer::Serializer;
use serializer::{JsonSerializer, RonSerializer, YamlSerializer};
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use storage::FileStorage;
use storage::Storage;
//use store::StoreHM;

pub type ByteString = Vec<u8>;
pub type WriteOperation<T> = (Uuid, T);
pub type StoreHM = HashMap<Uuid, Mutex<ByteString>>;

pub type JsonStore<T> = RdStore<T, JsonSerializer, FileStorage<JsonSerializer>>;
pub type YamlStore<T> = RdStore<T, YamlSerializer, FileStorage<YamlSerializer>>;
pub type RonStore<T> = RdStore<T, RonSerializer, FileStorage<RonSerializer>>;

#[derive(Debug)]
pub struct RdStore<T, SE, ST> {
  pub store: RwLock<StoreHM>,
  pub storage: ST,
  pub serializer: SE,
  pub record: T,
}

impl<'a, T, SE, ST> RdStore<T, SE, ST>
where
  for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq + Default + Clone,
  for<'de> SE: Serializer<'de> + Debug,
  for<'de> ST: Storage + Debug,
{
  pub fn new() -> Result<Self>
// where
  //   for<'de> V: Serialize + Deserialize<'de> + Debug + Display + PartialEq + Default + Clone,
  {
    let storage = ST::new::<T>()?;
    let data: StoreHM = storage
      .load_content::<T>()
      .context(RdStoreErrorKind::ContentLoad)?;

    Ok(Self {
      store: RwLock::new(data),
      storage: storage,
      serializer: SE::default(),
      record: T::default(),
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

  fn serialize(&self, value: &T) -> Result<ByteString> {
    Ok(
      self
        .serializer
        .serialize(value)
        .context(RdStoreErrorKind::Serialization)?,
    )
  }

  fn deserialize(&self, value: &Vec<u8>) -> Result<T> {
    Ok(
      self
        .serializer
        .deserialize(value)
        .context(RdStoreErrorKind::Deserialization)?,
    )
  }

  fn insert_key_value(&self, id: Uuid, data: ByteString) -> Option<Mutex<ByteString>> {
    let mut store = self.store.write().unwrap();
    store.insert(id, Mutex::new(data))
  }

  pub fn find_keys(&self, search: &T) -> Result<Vec<Uuid>> {
    let store = self.read()?;
    let serialized = self.serialize(search)?;
    let docs: Vec<Uuid> = store
      .iter()
      .map(|(_id, value)| {
        (
          _id,
          value
            .lock()
            .map_err(|_| RdStoreErrorKind::PoisonedValue)
            .unwrap(),
        )
      })
      .filter(|(_id, value)| **value == serialized)
      .map(|(id, _value)| *id)
      .collect();
    Ok(docs)
  }

  pub fn insert(&self, value: T) -> Result<Uuid> {
    let id = Uuid::new_v4();
    let data = self.serialize(&value)?;
    let _result = self.insert_key_value(id, data);
    self
      .storage
      .save_one((id, value))
      .context(RdStoreErrorKind::DataSave)?;
    Ok(id)
  }

  pub fn find(&self, id: &Uuid) -> Result<T> {
    let store = self.read()?;
    let value = store
      .get(&id)
      .ok_or(RdStoreErrorKind::NotFound { key: *id })?;
    let guard = value.lock().map_err(|_| RdStoreErrorKind::PoisonedValue)?;
    let result = self.deserialize(&*guard)?;
    Ok(result)
  }

  pub fn update(&'a self, id: &Uuid, new_value: T) -> Result<bool> {
    let mut store = self.write()?;
    if store.contains_key(id) {
      let value = store
        .get_mut(&id)
        .ok_or(RdStoreErrorKind::NotFound { key: *id })?;

      let mut guard = value.lock().map_err(|_| RdStoreErrorKind::PoisonedValue)?;
      *guard = self.serialize(&new_value)?;

      self
        .storage
        .save_one((*id, new_value))
        .context(RdStoreErrorKind::DataSave)?;
      Ok(true)
    } else {
      Ok(false)
    }
  }

  pub fn delete(&self, id: &Uuid) -> Result<bool> {
    let mut store = self.store.write().unwrap();
    if store.contains_key(id) {
      store.remove(id).unwrap();
      Ok(true)
    } else {
      Ok(false)
    }
  }

  pub fn insert_many(&self, values: Vec<T>) -> Result<usize> {
    let docs: Vec<WriteOperation<T>> = values
      .into_iter()
      .map(|value| {
        let id = Uuid::new_v4();
        let serialized = self.serialize(&value).unwrap();
        let _result = self.insert_key_value(id, serialized).unwrap();
        (id, value)
      })
      .collect();

    let result = docs.len();
    self
      .storage
      .save(docs)
      .context(RdStoreErrorKind::DataSave)?;

    Ok(result)
  }

  pub fn find_many(&self, search: &T) -> Result<Vec<T>> {
    let store = self.read()?;
    let serialized = self.serialize(search)?;
    let docs: Vec<T> = store
      .iter()
      .map(|(_id, value)| {
        value
          .lock()
          .map_err(|_| RdStoreErrorKind::PoisonedValue)
          .unwrap()
      })
      .filter(|value| **value == serialized)
      .map(|value| self.deserialize(&*value).unwrap())
      .collect();
    Ok(docs)
  }

  pub fn update_many(&self, search: &T, new_value: &T) -> Result<usize> {
    let mut store = self.write()?;
    let serialized = self.serialize(search)?;

    let docs: Vec<WriteOperation<T>> = store
      .iter_mut()
      .map(|(_id, value)| {
        (
          _id,
          value
            .lock()
            .map_err(|_| RdStoreErrorKind::PoisonedValue)
            .unwrap(),
        )
      })
      .filter(|(_id, value)| **value == serialized)
      .map(|(_id, mut value)| {
        *value = self.serialize(new_value).unwrap();
        (*_id, new_value.clone())
      })
      .collect();
    let result = docs.len();
    self
      .storage
      .save(docs)
      .context(RdStoreErrorKind::DataSave)?;

    Ok(result)
  }

  pub fn delete_many(&self, search: &T) -> Result<usize> {
    let keys = self.find_keys(search)?;
    let docs: Vec<bool> = keys.iter().map(|id| (self.delete(id).unwrap())).collect();
    Ok(docs.len())
  }
}
