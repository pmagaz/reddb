use core::fmt::Display;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::io::{Error, ErrorKind};
use std::result;
use std::sync::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
use uuid::Uuid;

use super::operation::Operation;
use super::serializer::Serializer;

pub type ByteString = Vec<u8>;
pub type WriteOperation<T> = (Uuid, T, Operation);
pub type WriteOperations<T> = Vec<WriteOperation<T>>;
pub type Result<T> = result::Result<T, std::io::Error>;
pub type StoreHM = HashMap<Uuid, Mutex<ByteString>>;

#[derive(Debug)]
pub struct Store<T, SE> {
  pub store: RwLock<StoreHM>,
  pub serializer: SE,
  pub record: T,
}

impl<'a, T, SE> Store<T, SE>
where
  for<'de> T: Serialize + Deserialize<'de> + Debug + Display + Clone + Default + PartialEq,
  for<'de> SE: Serializer<'de> + Debug,
{
  pub fn new(data: StoreHM) -> Self {
    Self {
      store: RwLock::new(data),
      serializer: SE::default(),
      record: T::default(),
    }
  }

  pub fn to_read(&'a self) -> RwLockReadGuard<'a, StoreHM> {
    self.store.read().unwrap()
  }

  pub fn to_write(&'a self) -> RwLockWriteGuard<'a, StoreHM> {
    self.store.write().unwrap()
  }

  pub fn insert_key(&self, id: Uuid, data: ByteString) -> Option<Mutex<ByteString>> {
    let mut store = self.to_write();
    store.insert(id, Mutex::new(data))
  }

  pub fn delete_key(&self, id: &Uuid) -> Mutex<ByteString> {
    let mut store = self.to_write();
    store.remove(id).unwrap()
  }

  pub fn find_keys(&self, search: &T) -> Vec<Uuid> {
    let store = self.to_read();
    let serialized = self.serializer.serialize(search);
    let docs: Vec<Uuid> = store
      .iter()
      .map(|(_id, value)| (_id, value.lock().unwrap()))
      .filter(|(_id, value)| **value == serialized)
      .map(|(id, _value)| *id)
      .collect();
    docs
  }

  pub fn insert_one(&self, value: &T) -> Uuid {
    let id = Uuid::new_v4();
    let data = self.serializer.serialize(value);
    let _result = self.insert_key(id, data);
    id
  }

  pub fn find_one(&self, id: &Uuid) -> T {
    let store = self.to_read();
    let value = store.get(&id).unwrap();
    let guard = value.lock().unwrap();
    self.serializer.deserialize(&*guard)
  }

  pub fn update_one(&'a self, id: &Uuid, new_value: &T) -> Uuid {
    let mut store = self.to_write();
    let value = store.get_mut(&id).unwrap();
    let mut guard = value.lock().unwrap();
    *guard = self.serializer.serialize(new_value);
    id.to_owned()
  }

  pub fn delete_one(&self, id: &Uuid) -> T {
    let deleted = self.delete_key(id);
    let guard = deleted.lock().unwrap();
    self.serializer.deserialize(&guard)
  }

  pub fn insert(&self, values: Vec<T>) -> WriteOperations<T> {
    let docs: WriteOperations<T> = values
      .into_iter()
      .map(|value| {
        let id = Uuid::new_v4();
        let serialized = self.serializer.serialize(&value);
        let _result = self.insert_key(id, serialized);
        (id, value, Operation::Insert)
      })
      .collect();
    docs
  }

  pub fn find(&self, search: &T) -> Vec<T> {
    let store = self.to_read();
    let serialized = self.serializer.serialize(search);
    let docs: Vec<T> = store
      .iter()
      .map(|(_id, value)| value.lock().unwrap())
      .filter(|value| **value == serialized)
      .map(|value| self.serializer.deserialize(&*value))
      .collect();
    docs
  }

  pub fn update(&self, search: &T, new_value: &T) -> WriteOperations<T> {
    let mut store = self.to_write();
    let serialized = self.serializer.serialize(search);

    let docs: WriteOperations<T> = store
      .iter_mut()
      .map(|(_id, value)| (_id, value.lock().unwrap()))
      .filter(|(_id, value)| **value == serialized)
      .map(|(_id, mut value)| {
        *value = self.serializer.serialize(new_value);
        (*_id, new_value.clone(), Operation::Update)
      })
      .collect();
    docs
  }

  pub fn delete(&self, search: &T) -> WriteOperations<T> {
    let keys = self.find_keys(search);
    let docs: WriteOperations<T> = keys
      .iter()
      .map(|id| {
        let deleted = self.delete_key(id);
        let guard = deleted.lock().unwrap();
        let value = self.serializer.deserialize(&guard);
        (*id, value, Operation::Delete)
      })
      .collect();
    docs
  }
}
