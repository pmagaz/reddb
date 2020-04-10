use super::operation::Operation;
use super::record::Record;
use super::serializer::{Serializer, Serializers};
use super::storage::Storage;
use core::fmt::Display;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::io::BufRead;
use std::io::{Error, ErrorKind};
use std::result;
use std::sync::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
use uuid::Uuid;

type ByteString = Vec<u8>;
type WriteOperation<T> = Vec<(Uuid, T, Operation)>;
pub type Result<T> = result::Result<T, std::io::Error>;
pub type StoreHM = HashMap<Uuid, Mutex<ByteString>>;

#[derive(Debug)]
pub struct Store<T, SE> {
  pub store: RwLock<StoreHM>,
  pub storage: Storage,
  pub serializer: SE,
  pub record: T,
}

impl<'a, T, SE> Store<T, SE>
where
  for<'de> T: Serialize + Deserialize<'de> + Debug + Clone + Display + Default + PartialEq,
  for<'de> SE: Serializer<'de> + Debug + Clone,
{
  pub fn new() -> Self {
    let serializer = SE::default();
    let store_name = match serializer.format() {
      Serializers::Json(st) => st,
      Serializers::Yaml(st) => st,
      Serializers::Ron(st) => st,
    };

    let storage = Storage::new(store_name).unwrap();
    let data: StoreHM = Store::<T, SE>::load_data(&storage, &serializer);
    Store::<T, SE>::rebuild(&storage, &serializer, &data).unwrap();

    Self {
      store: RwLock::new(data),
      storage: storage,
      serializer: serializer,
      record: T::default(),
    }
  }

  pub fn rebuild(storage: &Storage, serializer: &SE, data: &StoreHM) -> Result<()> {
    let docs: ByteString = data
      .iter()
      .map(|(id, value)| (id, value.lock().unwrap()))
      .map(|(id, value)| {
        let data: T = serializer.deserialize(&*value);
        Record::new(*id, data, Operation::default())
      })
      .flat_map(|record| serializer.serialize(&record))
      .collect();
    storage.rebuild_storage(&docs);
    Ok(())
  }

  pub fn load_data(storage: &Storage, serializer: &SE) -> StoreHM {
    let mut map: StoreHM = HashMap::new();
    let mut buf = Vec::new();
    storage.read_content(&mut buf);

    for (_index, content) in buf.lines().enumerate() {
      let line = content.unwrap();
      let byte_str = &line.into_bytes();
      let record: Record<T> = serializer.deserialize(byte_str);
      let id = record._id;
      let data = record.data;
      match record.operation {
        Operation::Insert => {
          let serialized = serializer.serialize(&data);
          map.insert(id, Mutex::new(serialized));
        }
        Operation::Update => {
          match map.get_mut(&id) {
            Some(value) => {
              let mut guard = value.lock().unwrap();
              *guard = serializer.serialize(&data);
            }
            None => {}
          };
        }
        Operation::Delete => {}
      }
    }
    println!("{:?}", map.len());
    map
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
    let mut store = self.to_write();
    let result = store.remove(id).unwrap();
    let guard = result.lock().unwrap();
    self.serializer.deserialize(&guard)
  }

  pub fn insert(&self, values: Vec<T>) -> WriteOperation<T> {
    let docs: WriteOperation<T> = values
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

  pub fn update(&self, search: &T, new_value: &T) -> WriteOperation<T> {
    let mut store = self.to_write();
    let serialized = self.serializer.serialize(search);

    let docs: WriteOperation<T> = store
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

  pub fn delete(&self, search: &T) -> WriteOperation<T> {
    let keys = self.find_keys(search);
    let docs: WriteOperation<T> = keys
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
      .flat_map(|record| self.serializer.serialize(&record))
      .collect();
    self.storage.append_data(&serialized);
  }

  pub fn persist_one(&self, id: Uuid, data: T, operation: Operation) {
    let record = Record::new(id, data, operation);
    let serialized = self.serializer.serialize(&record);
    self.storage.append_data(&serialized);
  }
}
