use super::deserializer::DeSerializer;
use super::operation::Operation;
use super::record::Record;
use super::storage::Storage;
use core::fmt::Display;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::io::BufRead;
use std::io::{Error, ErrorKind};
use std::path::Path;
use std::result;
use std::sync::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
use uuid::Uuid;

type ByteString = Vec<u8>;
type WriteOps<T> = Vec<(Uuid, T, Operation)>;
pub type Result<T> = result::Result<T, std::io::Error>;

pub type RDHM = HashMap<Uuid, Mutex<ByteString>>;

#[derive(Debug)]
pub struct Store<T, DS> {
  pub store: RwLock<RDHM>,
  pub storage: Storage,
  pub serializer: DS,
  pub record: T,
}

impl<'a, T, DS> Store<T, DS>
where
  for<'de> T: Serialize + Deserialize<'de> + Debug + Clone + Display + Default + PartialEq,
  for<'de> DS: DeSerializer<'de> + Debug + Clone,
{
  pub fn new<P>(path: P) -> Self
  where
    P: AsRef<Path>,
  {
    let deser = DS::default();
    let mut map: RDHM = HashMap::new();
    let storage = Storage::new(path).unwrap();
    let mut buf = Vec::new();
    storage.read_content(&mut buf);

    for (_index, content) in buf.lines().enumerate() {
      let line = content.unwrap();
      let leches = &line.into_bytes();
      let record: Record<T> = deser.deserializer(leches);
      let serialized = deser.serializer(&record.data);
      map.insert(record._id, Mutex::new(serialized));
    }

    Self {
      store: RwLock::new(map),
      storage: storage,
      serializer: DS::default(),
      record: T::default(),
    }
  }

  pub fn to_read(&'a self) -> RwLockReadGuard<'a, RDHM> {
    let read = self.store.read().unwrap();
    read
  }

  pub fn to_write(&'a self) -> RwLockWriteGuard<'a, RDHM> {
    let write = self.store.write().unwrap();
    write
  }

  pub fn insert_key(&self, id: Uuid, data: ByteString) -> Option<Mutex<ByteString>> {
    let mut store = self.to_write();
    store.insert(id, Mutex::new(data))
  }

  pub fn delete_key(&self, id: &Uuid) -> Mutex<ByteString> {
    let mut store = self.to_write();
    let result = store.remove(id).unwrap();
    result
  }

  pub fn find_keys(&self, search: &T) -> Vec<Uuid> {
    let store = self.to_read();
    let serialized = self.serializer.serializer(search);
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
    let data = self.serializer.serializer(value);
    let _result = self.insert_key(id, data);
    id
  }

  pub fn find_one(&self, id: &Uuid) -> T {
    let store = self.to_read();
    let value = store.get(&id).unwrap();
    let guard = value.lock().unwrap();
    self.serializer.deserializer(&*guard)
  }

  pub fn update_one(&'a self, id: &Uuid, new_value: &T) -> Uuid {
    let mut store = self.to_write();
    let value = store.get_mut(&id).unwrap();
    let mut guard = value.lock().unwrap();
    *guard = self.serializer.serializer(new_value);
    id.to_owned()
  }

  pub fn delete_one(&self, id: &Uuid) -> T {
    let mut store = self.to_write();
    let result = store.remove(id).unwrap();
    let guard = result.lock().unwrap();
    self.serializer.deserializer(&guard)
  }

  pub fn insert(&self, values: Vec<T>) -> WriteOps<T> {
    let docs: WriteOps<T> = values
      .into_iter()
      .map(|value| {
        let id = Uuid::new_v4();
        let serialized = self.serializer.serializer(&value);
        let _result = self.insert_key(id, serialized);
        (id, value, Operation::Insert)
      })
      .collect();
    docs
  }

  pub fn find(&self, search: &T) -> Vec<T> {
    let store = self.to_read();
    let serialized = self.serializer.serializer(search);
    let docs: Vec<T> = store
      .iter()
      .map(|(_id, value)| value.lock().unwrap())
      .filter(|value| **value == serialized)
      .map(|value| self.serializer.deserializer(&*value))
      .collect();
    docs
  }

  pub fn update(&self, search: &T, new_value: &T) -> WriteOps<T> {
    let mut store = self.to_write();
    let serialized = self.serializer.serializer(search);

    let docs: WriteOps<T> = store
      .iter_mut()
      .map(|(_id, value)| (_id, value.lock().unwrap()))
      .filter(|(_id, value)| **value == serialized)
      .map(|(_id, mut value)| {
        *value = self.serializer.serializer(new_value);
        (*_id, new_value.clone(), Operation::Update)
      })
      .collect();
    docs
  }

  pub fn delete(&self, search: &T) -> WriteOps<T> {
    let keys = self.find_keys(search);
    let docs: WriteOps<T> = keys
      .iter()
      .map(|id| {
        let value = self.delete_one(id);
        (*id, value, Operation::Delete)
      })
      .collect();
    docs
  }

  pub fn persist(&self, docs: Vec<(Uuid, T, Operation)>) {
    let serialized: ByteString = docs
      .into_iter()
      .map(|(id, value, operation)| Record::new(id, value, operation))
      .flat_map(|record| self.serializer.serializer(&record))
      .collect();
    &self.storage.write(&serialized);
  }

  pub fn persist_one(&self, id: Uuid, data: T, operation: Operation) {
    let record = Record::new(id, data, operation);
    let serialized = self.serializer.serializer(&record);
    &self.storage.write(&serialized);
  }
}
