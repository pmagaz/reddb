use core::fmt::Display;
use failure::ResultExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use uuid::Uuid;

use super::error::{RdStoreErrorKind, Result};
use super::operation::Operation;
use super::serializer::Serializer;

pub type ByteString = Vec<u8>;
pub type WriteOperation<T> = (Uuid, T);
pub type WriteOperations<T> = Vec<WriteOperation<T>>;
pub type StoreHM = HashMap<Uuid, Mutex<ByteString>>;

#[derive(Debug)]
pub struct Store<T> {
  pub store: RwLock<StoreHM>,
  //pub serializer: SE,
  pub record: T,
}

impl<'a, T> Store<T>
where
  for<'de> T: Serialize + Deserialize<'de> + Debug + Display + Clone + Default + PartialEq,
  // for<'de> SE: Serializer<'de> + Debug,
{
  pub fn new(data: StoreHM) -> Self {
    Self {
      store: RwLock::new(data),
      // serializer: SE::default(),
      record: T::default(),
    }
  }

  pub fn read(&'a self) -> Result<RwLockReadGuard<'a, StoreHM>> {
    let lock = self.store.read().map_err(|_| RdStoreErrorKind::Poisoned)?;
    Ok(lock)
  }

  fn write(&'a self) -> Result<RwLockWriteGuard<'a, StoreHM>> {
    let lock = self.store.write().map_err(|_| RdStoreErrorKind::Poisoned)?;
    Ok(lock)
  }

  // fn serialize(&self, value: &T) -> Result<ByteString> {
  //   let serialized = self
  //     .serializer
  //     .serialize(value)
  //     .context(RdStoreErrorKind::Serialization)?;
  //   Ok(serialized)
  // }

  // fn deserialize(&self, value: &Vec<u8>) -> Result<T> {
  //   let deserialized = self
  //     .serializer
  //     .deserialize(value)
  //     .context(RdStoreErrorKind::Deserialization)?;
  //   Ok(deserialized)
  // }

  fn insert_key_value(&self, id: Uuid, data: ByteString) -> Option<Mutex<ByteString>> {
    let mut store = self.write().unwrap();
    store.insert(id, Mutex::new(data))
  }

  fn delete_key(&self, id: &Uuid) -> Option<Mutex<ByteString>> {
    let mut store = self.write().unwrap();
    store.remove(id)
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

  pub fn insert(&self, value: &T) -> Result<Uuid> {
    let id = Uuid::new_v4();
    let data = self.serialize(value)?;
    let _result = self.insert_key_value(id, data);
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

  pub fn update(&'a self, id: &Uuid, new_value: &T) -> Result<Uuid> {
    let mut store = self.write()?;
    let value = store
      .get_mut(&id)
      .ok_or(RdStoreErrorKind::NotFound { key: *id })?;

    let mut guard = value.lock().map_err(|_| RdStoreErrorKind::PoisonedValue)?;
    *guard = self.serialize(new_value)?;
    Ok(id.to_owned())
  }

  pub fn delete(&self, id: &Uuid) -> Result<MutexGuard<ByteString>> {
    let deleted = self.delete_key(id).ok_or(RdStoreErrorKind::Deletekey)?;
    let guard = deleted
      .lock()
      .map_err(|_| RdStoreErrorKind::PoisonedValue)?;
    //let result = self.deserialize(&guard)?;
    Ok(guard)
  }

  pub fn insert_many(&self, values: Vec<T>) -> Result<Vec<WriteOperation<T>>> {
    let docs: WriteOperations<T> = values
      .into_iter()
      .map(|value| {
        let id = Uuid::new_v4();
        let serialized = self.serialize(&value).unwrap();
        let _result = self.insert_key_value(id, serialized).unwrap();
        (id, value, Operation::Insert)
      })
      .collect();
    Ok(docs)
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

  pub fn update_many(&self, search: &T, new_value: &T) -> Result<WriteOperations<T>> {
    let mut store = self.write()?;
    let serialized = self.serialize(search)?;

    let docs: WriteOperations<T> = store
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
        (*_id, new_value.clone(), Operation::Update)
      })
      .collect();
    Ok(docs)
  }

  pub fn delete_many(&self, search: &T) -> Result<Vec<WriteOperation<T>>> {
    let keys = self.find_keys(search)?;
    let docs: WriteOperations<T> = keys
      .iter()
      .map(|id| {
        let deleted = self.delete(id).unwrap();
        (*id, deleted, Operation::Delete)
      })
      .collect();
    Ok(docs)
  }
}
