use core::fmt::Display;
use operation::Operation;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::result;
use std::sync::Mutex;
use std::sync::{MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use storage::Storage;
use uuid::Uuid;

mod record;
mod store;
use record::{Empty, Record};
use store::Store;
mod deserializer;
mod json;
mod operation;
mod storage;
pub use deserializer::DeSerializer;
pub use json::JsonSerializer;

type ByteString = Vec<u8>;
type WriteOps<T> = Vec<(Uuid, T, Operation)>;

/*
 TODO
 - Change to references in search
 - Add Ron and Yaml encoders
 - Unwraps and error handing
 - Rebuild Db
 - Configuration
 - Test
 - Benches
*/

#[derive(Debug)]
pub struct RedDb<DS> {
  pub store: Store,
  pub serializer: DS,
  pub storage: Storage,
}

impl<'a, DS> RedDb<DS>
where
  for<'de> DS: DeSerializer<'de> + Debug + Clone,
{
  pub fn new() -> Self {
    Self {
      store: Store::new(),
      serializer: DS::default(),
      storage: Storage::new(".db").unwrap(),
    }
  }

  pub fn insert_key(&self, id: Uuid, data: ByteString) -> Option<Mutex<ByteString>> {
    let mut store = self.store.to_write();
    store.insert(id, Mutex::new(data))
  }

  pub fn delete_key(&self, id: &Uuid) -> Mutex<ByteString> {
    let mut store = self.store.to_write();
    let result = store.remove(id).unwrap();
    result
  }

  pub fn find_keys<T>(&self, search: &T) -> Vec<Uuid>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq + Clone,
  {
    let store = self.store.to_read();
    let serialized = self.serializer.serializer(search);
    let docs: Vec<Uuid> = store
      .iter()
      .map(|(_id, value)| (_id, value.lock().unwrap()))
      .filter(|(_id, value)| **value == serialized)
      .map(|(id, _value)| *id)
      .collect();
    docs
  }

  pub fn insert_one<T>(&self, value: T) -> Uuid
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq,
  {
    let id = Uuid::new_v4();
    let data = self.serializer.serializer(&value);
    let _result = self.insert_key(id, data);
    self.persist_one(id, value, Operation::default());
    id
  }

  pub fn find_one<T>(&self, id: &Uuid) -> T
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq,
  {
    let store = self.store.to_read();
    let value = store.get(&id).unwrap();
    let guard = value.lock().unwrap();
    self.serializer.deserializer(&*guard)
  }

  pub fn update_one<T>(&'a self, id: &Uuid, new_value: T) -> Uuid
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq + Clone,
  {
    let mut store = self.store.to_write();
    let value = store.get_mut(&id).unwrap();
    let mut guard = value.lock().unwrap();
    *guard = self.serializer.serializer(&*guard);
    self.persist_one(*id, new_value, Operation::Update);
    id.to_owned()
  }

  pub fn delete_one(&self, id: &Uuid) -> Uuid {
    let mut store = self.store.to_write();
    let _result = store.remove(id).unwrap();
    self.persist_one(*id, Empty, Operation::Delete);
    id.to_owned()
  }

  pub fn find_all<T>(&self, search: &T) -> Vec<T>
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq + Clone,
  {
    let store = self.store.to_read();
    let serialized = self.serializer.serializer(search);
    let docs: Vec<T> = store
      .iter()
      .map(|(_id, value)| value.lock().unwrap())
      .filter(|value| **value == serialized)
      .map(|value| self.serializer.deserializer(&*value))
      .collect();
    docs
  }

  pub fn update_all<T>(&self, search: &T, new_value: T) -> usize
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq + Clone,
  {
    let mut store = self.store.to_write();
    let serialized = self.serializer.serializer(search);

    let docs: WriteOps<T> = store
      .iter_mut()
      .map(|(_id, value)| (_id, value.lock().unwrap()))
      .filter(|(_id, value)| **value == serialized)
      .map(|(_id, mut value)| {
        *value = self.serializer.serializer(&new_value);
        //FIXME
        (*_id, new_value.clone(), Operation::Update)
      })
      .collect();
    let result = docs.len();
    self.persist_all(docs);
    result
  }

  pub fn delete_all<T>(&self, search: &T) -> usize
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display + PartialEq + Clone,
  {
    let keys = self.find_keys(search);
    let docs: WriteOps<Empty> = keys
      .iter()
      .map(|id| {
        self.delete_key(id);
        (*id, Empty, Operation::Delete)
      })
      .collect();
    let result = docs.len();
    self.persist_all(docs);
    result
  }

  pub fn persist_all<T>(&self, docs: Vec<(Uuid, T, Operation)>)
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display,
  {
    let serialized: Vec<u8> = docs
      .into_iter()
      .map(|(id, value, status)| Record::new(id, value, status))
      .flat_map(|record| self.serializer.serializer(&record))
      .collect();
    &self.storage.write(&serialized);
  }

  pub fn persist_one<T>(&self, id: Uuid, data: T, status: Operation)
  where
    for<'de> T: Serialize + Deserialize<'de> + Debug + Display,
  {
    let record = Record::new(id, data, status);
    let serialized = self.serializer.serializer(&record);
    &self.storage.write(&serialized);
  }
}
